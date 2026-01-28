use crate::{Database, Exploit, ExploitContainer, ExploitRun};
use anyhow::Result;
use bollard::Docker;
use bollard::query_parameters::{CreateContainerOptions, StartContainerOptions, RemoveContainerOptions, InspectContainerOptions};
use bollard::exec::{CreateExecOptions, StartExecOptions};
use bollard::secret::{ContainerCreateBody, HostConfig};
use futures::{Stream, StreamExt};
use std::time::Duration;
use tokio::sync::oneshot;
use tracing::{info, warn, error};

#[derive(Clone)]
pub struct ContainerManager {
    pub db: Database,
    pub docker: Docker,
}

const MAX_OUTPUT: usize = 256 * 1024; // 256KB limit

struct ExecOutput {
    stdout: String,
    stderr: String,
    ole: bool,
    timed_out: bool,
}

async fn collect_exec_output<S>(stream: S, timeout: Option<Duration>) -> ExecOutput
where
    S: Stream<Item = Result<bollard::container::LogOutput, bollard::errors::Error>>,
{
    let mut stream = Box::pin(stream);
    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut ole = false;
    let mut timed_out = false;
    let deadline = timeout.map(|t| tokio::time::Instant::now() + t);

    loop {
        let msg = if let Some(deadline) = deadline {
            let mut pinned_stream = stream.as_mut();
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => {
                    timed_out = true;
                    None
                }
                msg = pinned_stream.next() => msg,
            }
        } else {
            stream.as_mut().next().await
        };

        if timed_out {
            break;
        }

        match msg {
            Some(Ok(log)) => {
                let total_len = stdout.len() + stderr.len();
                if total_len >= MAX_OUTPUT {
                    ole = true;
                    break;
                }
                let remaining = MAX_OUTPUT - total_len;
                match log {
                    bollard::container::LogOutput::StdOut { message }
                    | bollard::container::LogOutput::Console { message } => {
                        let slice = &message[..message.len().min(remaining)];
                        stdout.push_str(&String::from_utf8_lossy(slice));
                    }
                    bollard::container::LogOutput::StdErr { message } => {
                        let slice = &message[..message.len().min(remaining)];
                        stderr.push_str(&String::from_utf8_lossy(slice));
                    }
                    _ => {}
                }
                if stdout.len() + stderr.len() >= MAX_OUTPUT {
                    ole = true;
                    break;
                }
            }
            Some(Err(_)) | None => break,
        }
    }

    ExecOutput { stdout, stderr, ole, timed_out }
}

impl ContainerManager {
    pub fn new(db: Database) -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self { db, docker })
    }

    /// Get the default CMD from a Docker image
    pub async fn get_image_cmd(&self, image: &str) -> Option<Vec<String>> {
        let inspect = self.docker.inspect_image(image).await.ok()?;
        inspect.config?.cmd
    }

    /// Ensure at least one container exists for an exploit
    pub async fn ensure_containers(&self, exploit_id: i32) -> Result<()> {
        let exploit = self.db.get_exploit(exploit_id).await?;
        if !exploit.enabled {
            return Ok(());
        }

        let containers = self.db.get_exploit_containers(exploit_id).await?;
        if containers.is_empty() {
            self.spawn_container(&exploit).await?;
        }
        Ok(())
    }

    /// Spawn a new persistent container for an exploit
    pub async fn spawn_container(&self, exploit: &Exploit) -> Result<ExploitContainer> {
        let normalized: String = exploit.name.chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .take(20)
            .collect();
        let rand_id: String = (0..8).map(|_| format!("{:x}", rand::random::<u8>() % 16)).collect();
        let container_name = format!("mazuadm-{}-{}", normalized, rand_id);
        
        let config = ContainerCreateBody {
            image: Some(exploit.docker_image.clone()),
            entrypoint: Some(vec!["sleep".to_string(), "infinity".to_string()]),
            host_config: Some(HostConfig {
                network_mode: Some("host".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let resp = self.docker.create_container(
            Some(CreateContainerOptions { name: Some(container_name.clone()), platform: String::new() }),
            config
        ).await?;

        self.docker.start_container(&resp.id, None::<StartContainerOptions>).await?;
        
        let container = self.db.create_exploit_container(exploit.id, &resp.id, exploit.default_counter).await?;
        info!("Spawned container {} for exploit {}", resp.id, exploit.id);
        Ok(container)
    }

    /// Get or assign a container for an exploit run
    pub async fn get_or_assign_container(&self, run: &ExploitRun) -> Result<ExploitContainer> {
        // Check if runner already has a container
        if let Some(runner) = self.db.get_runner_for_run(run.id).await? {
            let container = self.db.get_container(runner.exploit_container_id).await?;
            if container.status == "running" {
                return Ok(container);
            }
            // Container died, will reassign below
        }

        let exploit = self.db.get_exploit(run.exploit_id).await?;
        
        // Find available container or spawn new one
        let container = match self.db.get_available_container(exploit.id, exploit.max_per_container).await? {
            Some(c) => c,
            None => self.spawn_container(&exploit).await?,
        };

        // Create runner assignment (ignore if already exists)
        let _ = self.db.create_exploit_runner(container.id, run.id, run.team_id).await;
        
        Ok(container)
    }

    /// Execute command in a persistent container with timeout support
    pub async fn execute_in_container_with_timeout(&self, container: &ExploitContainer, cmd: Vec<String>, env: Vec<String>, timeout: Duration, pid_notify: Option<oneshot::Sender<i64>>) -> Result<ExecResult> {
        let exec = self.docker.create_exec(&container.container_id, CreateExecOptions {
            cmd: Some(cmd),
            env: Some(env.into_iter().chain(std::iter::once("TERM=xterm".to_string())).collect()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(false),
            ..Default::default()
        }).await?;

        let exec_id = exec.id.clone();
        
        // Get PID right after creating exec (before it finishes)
        let pid_future = {
            let docker = self.docker.clone();
            let eid = exec_id.clone();
            let pid_notify = pid_notify;
            async move {
                // Poll until we get a PID
                for _ in 0..100 {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    if let Ok(inspect) = docker.inspect_exec(&eid).await {
                        if inspect.pid.is_some() {
                            if let Some(pid) = inspect.pid {
                                if let Some(tx) = pid_notify {
                                    let _ = tx.send(pid);
                                }
                                return Some(pid);
                            }
                        }
                    }
                }
                None
            }
        };
        
        let output = self.docker.start_exec(&exec_id, Some(StartExecOptions { detach: false, tty: false, ..Default::default() })).await?;

        let mut pid: Option<i64> = None;
        
        // Spawn PID fetcher
        let pid_handle = tokio::spawn(pid_future);

        let mut stdout = String::new();
        let mut stderr = String::new();
        let mut ole = false;
        let mut timed_out = false;

        if let bollard::exec::StartExecResults::Attached { output: stream, .. } = output {
            let capture = collect_exec_output(stream, Some(timeout)).await;
            stdout = capture.stdout;
            stderr = capture.stderr;
            ole = capture.ole;
            timed_out = capture.timed_out;
        }
        
        // Get PID if we need to kill
        if timed_out || ole {
            pid = pid_handle.await.ok().flatten();
        } else {
            pid_handle.abort();
        }
        
        let inspect = self.docker.inspect_exec(&exec_id).await.ok();
        let exit_code = if ole { -2 } else if timed_out { -1 } else { 
            inspect.and_then(|i| i.exit_code).unwrap_or(-1) 
        };

        Ok(ExecResult { stdout, stderr, exit_code, ole, timed_out, pid })
    }

    /// Execute command in a persistent container (legacy method for compatibility)
    pub async fn execute_in_container(&self, container: &ExploitContainer, cmd: Vec<String>, env: Vec<String>) -> Result<ExecResult> {
        let exec = self.docker.create_exec(&container.container_id, CreateExecOptions {
            cmd: Some(cmd),
            env: Some(env.into_iter().chain(std::iter::once("TERM=xterm".to_string())).collect()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(false),
            ..Default::default()
        }).await?;

        let exec_id = exec.id.clone();
        let output = self.docker.start_exec(&exec_id, Some(StartExecOptions { detach: false, tty: false, ..Default::default() })).await?;

        let mut stdout = String::new();
        let mut stderr = String::new();
        let mut ole = false;

        if let bollard::exec::StartExecResults::Attached { output: stream, .. } = output {
            let capture = collect_exec_output(stream, None).await;
            stdout = capture.stdout;
            stderr = capture.stderr;
            ole = capture.ole;
        }

        let inspect = self.docker.inspect_exec(&exec_id).await?;
        let exit_code = if ole { -2 } else { inspect.exit_code.unwrap_or(-1) };

        Ok(ExecResult { stdout, stderr, exit_code, ole, timed_out: false, pid: inspect.pid })
    }

    /// Kill a process inside a container by host PID (translates to container PID)
    pub async fn kill_process_in_container(&self, container_id: &str, host_pid: i64) -> Result<()> {
        // The PID from exec_inspect is the host PID, we need to translate it to container PID
        // by reading /proc/<host_pid>/status and getting NSpid (last column is container PID)
        let nspid_output = tokio::fs::read_to_string(format!("/proc/{}/status", host_pid)).await;
        
        let container_pid = match nspid_output {
            Ok(status) => {
                // Find NSpid line and get the last value (innermost namespace PID)
                status.lines()
                    .find(|line| line.starts_with("NSpid:"))
                    .and_then(|line| line.split_whitespace().last())
                    .and_then(|pid| pid.parse::<i64>().ok())
            }
            Err(_) => None,
        };

        let pid_to_kill = container_pid.unwrap_or(host_pid);
        
        // Use /bin/sh -c "kill" since some containers don't have kill binary
        let exec = self.docker.create_exec(container_id, CreateExecOptions {
            cmd: Some(vec!["/bin/sh".to_string(), "-c".to_string(), format!("kill -9 {}", pid_to_kill)]),
            user: Some("root".to_string()),
            ..Default::default()
        }).await?;
        
        let _ = self.docker.start_exec(&exec.id, Some(StartExecOptions { detach: true, ..Default::default() })).await;
        info!("Killed PID {} (host: {}) in container {}", pid_to_kill, host_pid, container_id);
        Ok(())
    }

    /// Health check all containers, recreate dead ones
    pub async fn health_check(&self) -> Result<()> {
        let containers = self.db.list_all_containers().await?;
        
        for container in containers {
            if container.status != "running" {
                continue;
            }

            let alive = match self.docker.inspect_container(&container.container_id, None::<InspectContainerOptions>).await {
                Ok(info) => info.state.and_then(|s| s.running).unwrap_or(false),
                Err(_) => false,
            };

            if !alive {
                warn!("Container {} is dead, recreating", container.container_id);
                self.db.set_container_status(container.id, "dead").await?;
                
                // Get runners to reassign
                let runners = self.db.get_runners_for_container(container.id).await?;
                self.db.delete_runners_for_container(container.id).await?;
                
                // Spawn new container
                let exploit = self.db.get_exploit(container.exploit_id).await?;
                if exploit.enabled {
                    let new_container = self.spawn_container(&exploit).await?;
                    
                    // Reassign runners
                    for runner in runners {
                        let _ = self.db.create_exploit_runner(new_container.id, runner.exploit_run_id, runner.team_id).await;
                    }
                }
            }
        }
        Ok(())
    }

    /// Destroy a container and clean up
    pub async fn destroy_container(&self, container_id: i32) -> Result<()> {
        let container = self.db.get_container(container_id).await?;
        
        let _ = self.docker.remove_container(&container.container_id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await;
        self.db.delete_runners_for_container(container_id).await?;
        self.db.delete_exploit_container(container_id).await?;
        
        info!("Destroyed container {}", container.container_id);
        Ok(())
    }

    /// Destroy all containers for an exploit
    pub async fn destroy_exploit_containers(&self, exploit_id: i32) -> Result<()> {
        let containers = self.db.get_exploit_containers(exploit_id).await?;
        for c in containers {
            self.destroy_container(c.id).await?;
        }
        Ok(())
    }

    /// Ensure containers for all enabled exploits
    pub async fn ensure_all_containers(&self) -> Result<()> {
        let exploits = self.db.list_enabled_exploits().await?;
        for exploit in exploits {
            if let Err(e) = self.ensure_containers(exploit.id).await {
                error!("Failed to ensure containers for exploit {}: {}", exploit.id, e);
            }
        }
        Ok(())
    }

    /// Pre-warm containers based on concurrent limit
    pub async fn prewarm_for_round(&self, concurrent_limit: usize) -> Result<()> {
        let exploits = self.db.list_enabled_exploits().await?;
        
        for exploit in exploits {
            let runs = self.db.get_exploit_runs_for_exploit(exploit.id).await?;
            if runs.is_empty() { continue; }
            
            // Calculate needed containers: min(runs, concurrent_limit) / max_per_container
            let active_runs = runs.len().min(concurrent_limit);
            let needed = (active_runs + exploit.max_per_container as usize - 1) / exploit.max_per_container as usize;
            
            let existing = self.db.get_exploit_containers(exploit.id).await?;
            let healthy: Vec<_> = existing.iter().filter(|c| c.counter > 0).collect();
            
            let to_spawn = needed.saturating_sub(healthy.len());
            if to_spawn > 0 {
                info!("Pre-warming {} containers for exploit {} (need {}, have {})", to_spawn, exploit.name, needed, healthy.len());
                for _ in 0..to_spawn {
                    if let Err(e) = self.spawn_container(&exploit).await {
                        error!("Failed to spawn container for {}: {}", exploit.name, e);
                    }
                }
            }
        }
        Ok(())
    }
}

pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i64,
    pub ole: bool,
    pub timed_out: bool,
    pub pid: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::{collect_exec_output, MAX_OUTPUT};
    use bollard::container::LogOutput;
    use futures::stream;
    use std::time::Duration;

    #[tokio::test]
    async fn collect_exec_output_separates_streams() {
        let items = vec![
            Ok(LogOutput::StdOut { message: "out".into() }),
            Ok(LogOutput::StdErr { message: "err".into() }),
            Ok(LogOutput::Console { message: "con".into() }),
        ];
        let stream = stream::iter(items);
        let out = collect_exec_output(stream, None).await;
        assert_eq!(out.stdout, "outcon");
        assert_eq!(out.stderr, "err");
        assert!(!out.ole);
        assert!(!out.timed_out);
    }

    #[tokio::test]
    async fn collect_exec_output_times_out() {
        let stream = stream::unfold((), |_| async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            Some((Ok(LogOutput::StdOut { message: "late".into() }), ()))
        });
        let out = collect_exec_output(stream, Some(Duration::from_millis(20))).await;
        assert!(out.timed_out);
        assert!(out.stdout.is_empty());
        assert!(out.stderr.is_empty());
    }

    #[tokio::test]
    async fn collect_exec_output_enforces_max_output() {
        let chunk = vec![b'a'; MAX_OUTPUT / 2];
        let items = vec![
            Ok(LogOutput::StdOut { message: chunk.clone().into() }),
            Ok(LogOutput::StdErr { message: chunk.clone().into() }),
            Ok(LogOutput::StdOut { message: chunk.clone().into() }),
        ];
        let stream = stream::iter(items);
        let out = collect_exec_output(stream, None).await;
        assert!(out.ole);
        assert_eq!(out.stdout.len() + out.stderr.len(), MAX_OUTPUT);
    }
}
