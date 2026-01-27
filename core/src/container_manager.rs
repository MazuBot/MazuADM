use crate::{Database, Exploit, ExploitContainer, ExploitRun};
use anyhow::Result;
use bollard::Docker;
use bollard::query_parameters::{CreateContainerOptions, StartContainerOptions, RemoveContainerOptions, InspectContainerOptions};
use bollard::exec::{CreateExecOptions, StartExecOptions};
use bollard::secret::{ContainerCreateBody, HostConfig};
use futures::StreamExt;
use tracing::{info, warn, error};

pub struct ContainerManager {
    pub db: Database,
    pub docker: Docker,
}

impl ContainerManager {
    pub fn new(db: Database) -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self { db, docker })
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

    /// Execute command in a persistent container
    pub async fn execute_in_container(&self, container: &ExploitContainer, cmd: Vec<String>, env: Vec<String>) -> Result<ExecResult> {
        const MAX_OUTPUT: usize = 256 * 1024; // 256KB limit
        
        let exec = self.docker.create_exec(&container.container_id, CreateExecOptions {
            cmd: Some(cmd),
            env: Some(env.into_iter().chain(std::iter::once("TERM=xterm".to_string())).collect()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(true),
            ..Default::default()
        }).await?;

        let output = self.docker.start_exec(&exec.id, Some(StartExecOptions { detach: false, tty: true, ..Default::default() })).await?;
        
        let mut stdout = String::new();
        let mut ole = false;
        
        // With TTY, all output comes as Raw bytes (stdout and stderr combined)
        if let bollard::exec::StartExecResults::Attached { output: mut stream, .. } = output {
            while let Some(Ok(log)) = stream.next().await {
                if stdout.len() >= MAX_OUTPUT {
                    ole = true;
                    break;
                }
                let msg = match log {
                    bollard::container::LogOutput::StdOut { message } => Some(message),
                    bollard::container::LogOutput::StdErr { message } => Some(message),
                    bollard::container::LogOutput::Console { message } => Some(message),
                    _ => None,
                };
                if let Some(m) = msg {
                    let remaining = MAX_OUTPUT.saturating_sub(stdout.len());
                    let slice = &m[..m.len().min(remaining)];
                    stdout.push_str(&String::from_utf8_lossy(slice));
                }
            }
        }

        let inspect = self.docker.inspect_exec(&exec.id).await?;
        let exit_code = if ole { -2 } else { inspect.exit_code.unwrap_or(-1) };

        Ok(ExecResult { stdout, stderr: String::new(), exit_code, ole })
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
}

pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i64,
    pub ole: bool,
}
