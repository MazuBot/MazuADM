use crate::{ContainerInfo, Database, Exploit};
use anyhow::Result;
use bollard::Docker;
use bollard::query_parameters::{
    CreateContainerOptions,
    StartContainerOptions,
    RemoveContainerOptions,
    InspectContainerOptions,
    ListContainersOptions,
    RestartContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecOptions};
use bollard::secret::{ContainerCreateBody, HostConfig};
use chrono::{DateTime, TimeZone, Utc};
use futures::{Stream, StreamExt};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, atomic::{AtomicI32, AtomicUsize, Ordering}};
use std::time::Duration;
use tokio::sync::{oneshot, Semaphore, Mutex};
use tracing::{info, warn, error};
use dashmap::DashSet;

pub struct ContainerManager {
    pub db: Database,
    pub docker: Docker,
    spawn_gate: Arc<Semaphore>,
    spawn_limit: Arc<AtomicUsize>,
    registry: Arc<Mutex<ContainerRegistry>>,
    restart_in_flight: DashSet<String>,
}

const MAX_OUTPUT: usize = 256 * 1024; // 256KB limit
const LABEL_MANAGED: &str = "mazuadm.managed";
const LABEL_EXPLOIT_ID: &str = "mazuadm.exploit_id";
const LABEL_EXPLOIT_NAME: &str = "mazuadm.exploit_name";
const LABEL_COUNTER: &str = "mazuadm.counter";
const LABEL_AFFINITY_LIST: &str = "mazuadm.affinity";

struct ExecOutput {
    stdout: String,
    stderr: String,
    ole: bool,
    timed_out: bool,
}

struct ManagedContainer {
    container_id: String,
    exploit_id: i32,
    counter: AtomicI32,
    exec_sem: Arc<Semaphore>,
    max_execs: usize,
    created_at: DateTime<Utc>,
}

#[derive(Default)]
struct ContainerRegistry {
    by_id: HashMap<String, Arc<ManagedContainer>>,
    pools: HashMap<i32, Vec<Arc<ManagedContainer>>>,
    affinity: HashMap<i32, String>,
    reverse_affinity: HashMap<String, HashSet<i32>>,
}

pub struct ContainerLease {
    manager: Arc<ContainerManager>,
    container: Arc<ManagedContainer>,
    permit: tokio::sync::OwnedSemaphorePermit,
}

impl ContainerLease {
    pub fn container_id(&self) -> &str {
        &self.container.container_id
    }

    pub fn exploit_id(&self) -> i32 {
        self.container.exploit_id
    }

    pub fn remaining(&self) -> i32 {
        self.container.counter.load(Ordering::SeqCst)
    }

    pub async fn finish(self) {
        self.manager.release_container(self.container, self.permit).await;
    }
}

enum AffinityAcquire {
    Lease(ContainerLease),
    Exhausted,
    None,
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
        Ok(Self {
            db,
            docker,
            spawn_gate: Arc::new(Semaphore::new(1)),
            spawn_limit: Arc::new(AtomicUsize::new(1)),
            registry: Arc::new(Mutex::new(ContainerRegistry::default())),
            restart_in_flight: DashSet::new(),
        })
    }

    pub fn set_concurrent_create_limit(&self, limit: usize) {
        let limit = limit.max(1);
        let current = self.spawn_limit.load(Ordering::SeqCst);
        if limit > current {
            self.spawn_gate.add_permits(limit - current);
        }
        self.spawn_limit.store(limit, Ordering::SeqCst);
    }

    pub async fn restore_from_docker(&self) -> Result<()> {
        let mut registry = ContainerRegistry::default();
        let containers = self.list_managed_containers().await?;

        for container in containers {
            let Some(container_id) = container.id else { continue; };
            let labels = container.labels.unwrap_or_default();
            let Some(exploit_id) = parse_label_i32(&labels, LABEL_EXPLOIT_ID) else { continue; };
            let exploit = match self.db.get_exploit(exploit_id).await {
                Ok(exploit) => exploit,
                Err(_) => continue,
            };
            if !exploit.enabled {
                continue;
            }
            let max_execs = exploit.max_per_container.max(1) as usize;
            let counter = parse_label_i32(&labels, LABEL_COUNTER).unwrap_or(exploit.default_counter);
            let created_at = container.created.map(timestamp_to_utc).unwrap_or_else(Utc::now);
            let managed = Arc::new(ManagedContainer {
                container_id: container_id.clone(),
                exploit_id,
                counter: AtomicI32::new(counter),
                exec_sem: Arc::new(Semaphore::new(max_execs)),
                max_execs,
                created_at,
            });
            registry.by_id.insert(container_id.clone(), managed.clone());
            registry.pools.entry(exploit_id).or_default().push(managed);

            if let Some(list) = labels.get(LABEL_AFFINITY_LIST) {
                let runs = parse_affinity_list(list);
                if !runs.is_empty() {
                    let mut set = HashSet::new();
                    for run_id in runs {
                        if registry.affinity.insert(run_id, container_id.clone()).is_some() {
                            warn!("Affinity run {} assigned to multiple containers; latest wins", run_id);
                        }
                        set.insert(run_id);
                    }
                    registry.reverse_affinity.insert(container_id.clone(), set);
                }
            }
        }

        let mut guard = self.registry.lock().await;
        *guard = registry;
        Ok(())
    }

    pub async fn list_containers(&self) -> Result<Vec<ContainerInfo>> {
        let (containers, affinity_map) = {
            let guard = self.registry.lock().await;
            (
                guard.by_id.values().cloned().collect::<Vec<_>>(),
                guard.reverse_affinity.clone(),
            )
        };

        let mut infos = Vec::new();
        for container in containers {
            let status = match self.docker.inspect_container(&container.container_id, None::<InspectContainerOptions>).await {
                Ok(info) => info.state.and_then(|s| s.running).map(|r| if r { "running" } else { "dead" }).unwrap_or("dead"),
                Err(_) => "dead",
            };
            let running_execs = container.max_execs.saturating_sub(container.exec_sem.available_permits());
            let affinity_runs = affinity_map
                .get(&container.container_id)
                .map(sorted_run_ids)
                .unwrap_or_default();
            infos.push(ContainerInfo {
                id: container.container_id.clone(),
                exploit_id: container.exploit_id,
                status: status.to_string(),
                counter: container.counter.load(Ordering::SeqCst),
                running_execs,
                max_execs: container.max_execs,
                created_at: container.created_at,
                affinity_runs,
            });
        }
        Ok(infos)
    }

    pub async fn lease_container(self: &Arc<Self>, exploit: &Exploit, exploit_run_id: i32) -> Result<ContainerLease> {
        let max_execs = exploit.max_per_container.max(1) as usize;
        let exploit_id = exploit.id;
        let max_containers = exploit.max_containers;

        let affinity_result = self.try_acquire_affinity(exploit_id, exploit_run_id).await?;
        match affinity_result {
            AffinityAcquire::Lease(lease) => return Ok(lease),
            AffinityAcquire::Exhausted => {}
            AffinityAcquire::None => {
                if let Some(lease) = self.try_acquire_best_available(exploit_id, exploit_run_id).await? {
                    return Ok(lease);
                }
            }
        }

        let pool_len = {
            let guard = self.registry.lock().await;
            guard
                .pools
                .get(&exploit_id)
                .map(|p| p.iter().filter(|c| c.counter.load(Ordering::SeqCst) > 0).count())
                .unwrap_or(0)
        };
        if max_containers > 0 && (pool_len as i32) >= max_containers {
            return Err(anyhow::anyhow!("Max containers reached for exploit {}", exploit_id));
        }

        let run_ids = self.db.get_exploit_runs_for_exploit(exploit_id).await?
            .into_iter()
            .map(|r| r.id)
            .collect::<Vec<_>>();
        let assigned = self.select_affinity_for_new_container(exploit_run_id, &run_ids, max_execs).await;
        let affinity_runs = if assigned.is_empty() { None } else { Some(assigned.clone()) };
        let container = self.spawn_container(exploit, max_execs, affinity_runs).await?;
        let permit = container.exec_sem.clone().acquire_owned().await?;
        if !try_decrement_counter(&container) {
            drop(permit);
            self.handle_exhausted_container(&container).await;
            return Err(anyhow::anyhow!("Container counter exhausted for exploit {}", exploit_id));
        }
        Ok(ContainerLease { manager: self.clone(), container, permit })
    }

    async fn release_container(&self, container: Arc<ManagedContainer>, permit: tokio::sync::OwnedSemaphorePermit) {
        drop(permit);
        if container.counter.load(Ordering::SeqCst) <= 0
            && container.exec_sem.available_permits() == container.max_execs
        {
            let _ = self.destroy_container_by_id(&container.container_id).await;
        }
    }

    async fn register_container(&self, container: Arc<ManagedContainer>) {
        let mut guard = self.registry.lock().await;
        guard.by_id.insert(container.container_id.clone(), container.clone());
        guard.pools.entry(container.exploit_id).or_default().push(container);
    }

    async fn register_affinity(&self, container_id: &str, runs: &[i32]) {
        if runs.is_empty() {
            return;
        }
        let mut guard = self.registry.lock().await;
        for run_id in runs {
            let registry = &mut *guard;
            drop_affinity_for_run(&mut registry.affinity, &mut registry.reverse_affinity, *run_id);
            registry.affinity.insert(*run_id, container_id.to_string());
            registry
                .reverse_affinity
                .entry(container_id.to_string())
                .or_default()
                .insert(*run_id);
        }
    }

    async fn try_acquire_affinity(self: &Arc<Self>, exploit_id: i32, run_id: i32) -> Result<AffinityAcquire> {
        let container = {
            let mut guard = self.registry.lock().await;
            let Some(container_id) = guard.affinity.get(&run_id).cloned() else {
                return Ok(AffinityAcquire::None);
            };
            let Some(container) = guard.by_id.get(&container_id).cloned() else {
                let registry = &mut *guard;
                drop_affinity_for_run(&mut registry.affinity, &mut registry.reverse_affinity, run_id);
                return Ok(AffinityAcquire::None);
            };
            let has_run = guard
                .reverse_affinity
                .get(&container_id)
                .map(|set| set.contains(&run_id))
                .unwrap_or(false);
            if container.exploit_id != exploit_id || !has_run {
                let registry = &mut *guard;
                drop_affinity_for_run(&mut registry.affinity, &mut registry.reverse_affinity, run_id);
                return Ok(AffinityAcquire::None);
            }
            container
        };

        if container.counter.load(Ordering::SeqCst) <= 0 {
            self.handle_exhausted_container(&container).await;
            return Ok(AffinityAcquire::Exhausted);
        }

        let permit = container.exec_sem.clone().acquire_owned().await?;
        if !try_decrement_counter(&container) {
            drop(permit);
            self.handle_exhausted_container(&container).await;
            return Ok(AffinityAcquire::Exhausted);
        }
        Ok(AffinityAcquire::Lease(ContainerLease { manager: self.clone(), container, permit }))
    }

    async fn try_acquire_best_available(self: &Arc<Self>, exploit_id: i32, run_id: i32) -> Result<Option<ContainerLease>> {
        let containers = {
            let guard = self.registry.lock().await;
            guard.pools.get(&exploit_id).cloned().unwrap_or_default()
        };

        let Some(container) = select_best_container(containers.into_iter()) else {
            return Ok(None);
        };

        let permit = container.exec_sem.clone().acquire_owned().await?;
        if !try_decrement_counter(&container) {
            drop(permit);
            self.handle_exhausted_container(&container).await;
            return Ok(None);
        }

        self.register_affinity(&container.container_id, &[run_id]).await;
        Ok(Some(ContainerLease { manager: self.clone(), container, permit }))
    }

    async fn select_affinity_for_new_container(&self, run_id: i32, run_ids: &[i32], max_execs: usize) -> Vec<i32> {
        if max_execs == 0 {
            return Vec::new();
        }

        let mut ids = run_ids.to_vec();
        ids.sort_unstable();
        ids.dedup();

        let mut guard = self.registry.lock().await;
        let mut mapped = HashSet::new();
        let mut to_unmap = Vec::new();
        for id in &ids {
            if let Some(container_id) = guard.affinity.get(id) {
                let is_active = guard
                    .by_id
                    .get(container_id)
                    .map(|c| c.counter.load(Ordering::SeqCst) > 0)
                    .unwrap_or(false);
                if is_active {
                    mapped.insert(*id);
                    continue;
                }
                to_unmap.push(*id);
            }
        }
        for id in to_unmap {
            let registry = &mut *guard;
            drop_affinity_for_run(&mut registry.affinity, &mut registry.reverse_affinity, id);
        }
        build_affinity_for_new_container(run_id, &ids, &mapped, max_execs)
    }

    async fn handle_exhausted_container(&self, container: &Arc<ManagedContainer>) {
        if container.counter.load(Ordering::SeqCst) > 0 {
            return;
        }
        self.unmap_exhausted_container(&container.container_id).await;
        if container.exec_sem.available_permits() == container.max_execs {
            let _ = self.destroy_container_by_id(&container.container_id).await;
        }
    }

    async fn unmap_exhausted_container(&self, container_id: &str) {
        let mut guard = self.registry.lock().await;
        let mut reverse = std::mem::take(&mut guard.reverse_affinity);
        drop_affinity_for_container(&mut guard.affinity, &mut reverse, container_id);
        guard.reverse_affinity = reverse;
    }

    async fn detach_container(&self, container_id: &str) -> Option<Arc<ManagedContainer>> {
        let mut guard = self.registry.lock().await;
        let mut reverse = std::mem::take(&mut guard.reverse_affinity);
        drop_affinity_for_container(&mut guard.affinity, &mut reverse, container_id);
        guard.reverse_affinity = reverse;
        let container = guard.by_id.remove(container_id)?;
        if let Some(pool) = guard.pools.get_mut(&container.exploit_id) {
            pool.retain(|c| c.container_id != container_id);
            if pool.is_empty() {
                guard.pools.remove(&container.exploit_id);
            }
        }
        Some(container)
    }

    async fn begin_restart(&self, container_id: &str) -> bool {
        if self.restart_in_flight.contains(container_id) {
            return false;
        }
        self.restart_in_flight.insert(container_id.to_string());
        true
    }

    async fn end_restart(&self, container_id: &str) {
        self.restart_in_flight.remove(container_id);
    }

    async fn list_managed_containers(&self) -> Result<Vec<bollard::models::ContainerSummary>> {
        let mut filters = HashMap::new();
        filters.insert("label".to_string(), vec![format!("{}=true", LABEL_MANAGED)]);
        let options = ListContainersOptions {
            all: true,
            filters: Some(filters),
            ..Default::default()
        };
        Ok(self.docker.list_containers(Some(options)).await?)
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

        let existing = {
            let guard = self.registry.lock().await;
            guard.pools.get(&exploit_id).map(|p| p.len()).unwrap_or(0)
        };
        if existing == 0 {
            let run_ids = self
                .db
                .get_exploit_runs_for_exploit(exploit_id)
                .await?
                .into_iter()
                .map(|r| r.id)
                .collect::<Vec<_>>();
            let max_execs = exploit.max_per_container.max(1) as usize;
            let affinity = if run_ids.is_empty() {
                None
            } else {
                let primary = *run_ids.iter().min().unwrap();
                let assigned = self.select_affinity_for_new_container(primary, &run_ids, max_execs).await;
                if assigned.is_empty() { None } else { Some(assigned) }
            };
            let _ = self.spawn_container(&exploit, max_execs, affinity).await?;
        }
        Ok(())
    }

    /// Spawn a new persistent container for an exploit
    async fn spawn_container(&self, exploit: &Exploit, max_execs: usize, affinity_runs: Option<Vec<i32>>) -> Result<Arc<ManagedContainer>> {
        let _permit = self.spawn_gate.acquire().await?;
        let normalized: String = exploit.name.chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .take(20)
            .collect();
        let rand_id: String = (0..8).map(|_| format!("{:x}", rand::random::<u8>() % 16)).collect();
        let container_name = format!("mazuadm-{}-{}", normalized, rand_id);

        let mut labels = HashMap::new();
        labels.insert(LABEL_MANAGED.to_string(), "true".to_string());
        labels.insert(LABEL_EXPLOIT_ID.to_string(), exploit.id.to_string());
        labels.insert(LABEL_EXPLOIT_NAME.to_string(), exploit.name.clone());
        labels.insert(LABEL_COUNTER.to_string(), exploit.default_counter.to_string());
        if let Some(runs) = &affinity_runs {
            labels.insert(LABEL_AFFINITY_LIST.to_string(), format_affinity_list(runs));
        }

        let config = ContainerCreateBody {
            image: Some(exploit.docker_image.clone()),
            entrypoint: Some(vec!["sleep".to_string(), "infinity".to_string()]),
            labels: Some(labels),
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

        let managed = Arc::new(ManagedContainer {
            container_id: resp.id.clone(),
            exploit_id: exploit.id,
            counter: AtomicI32::new(exploit.default_counter),
            exec_sem: Arc::new(Semaphore::new(max_execs)),
            max_execs,
            created_at: Utc::now(),
        });
        self.register_container(managed.clone()).await;
        if let Some(runs) = affinity_runs {
            self.register_affinity(&managed.container_id, &runs).await;
        }

        info!("Spawned container {} for exploit {}", resp.id, exploit.id);
        Ok(managed)
    }

    /// Execute command in a persistent container with timeout support
    pub async fn execute_in_container_with_timeout(&self, container_id: &str, cmd: Vec<String>, env: Vec<String>, timeout: Duration, pid_notify: Option<oneshot::Sender<i64>>) -> Result<ExecResult> {
        let exec = self.docker.create_exec(container_id, CreateExecOptions {
            cmd: Some(cmd),
            env: Some(env.into_iter().chain(std::iter::once("TERM=xterm".to_string())).collect()),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            tty: Some(false),
            ..Default::default()
        }).await?;

        let exec_id = exec.id.clone();

        let pid_future = {
            let docker = self.docker.clone();
            let eid = exec_id.clone();
            let pid_notify = pid_notify;
            async move {
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
    pub async fn execute_in_container(&self, container_id: &str, cmd: Vec<String>, env: Vec<String>) -> Result<ExecResult> {
        let exec = self.docker.create_exec(container_id, CreateExecOptions {
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
        let nspid_output = tokio::fs::read_to_string(format!("/proc/{}/status", host_pid)).await;

        let container_pid = match nspid_output {
            Ok(status) => {
                status.lines()
                    .find(|line| line.starts_with("NSpid:"))
                    .and_then(|line| line.split_whitespace().last())
                    .and_then(|pid| pid.parse::<i64>().ok())
            }
            Err(_) => None,
        };

        let pid_to_kill = container_pid.unwrap_or(host_pid);

        let exec = self.docker.create_exec(container_id, CreateExecOptions {
            cmd: Some(vec!["/bin/sh".to_string(), "-c".to_string(), format!("kill -9 {}", pid_to_kill)]),
            user: Some("root".to_string()),
            ..Default::default()
        }).await?;

        let _ = self.docker.start_exec(&exec.id, Some(StartExecOptions { detach: true, ..Default::default() })).await;
        info!("Killed PID {} (host: {}) in container {}", pid_to_kill, host_pid, container_id);
        Ok(())
    }

    /// Health check all containers, remove dead ones
    pub async fn health_check(&self) -> Result<()> {
        let containers: Vec<Arc<ManagedContainer>> = {
            let guard = self.registry.lock().await;
            guard.by_id.values().cloned().collect()
        };

        for container in containers {
            let alive = match self.docker.inspect_container(&container.container_id, None::<InspectContainerOptions>).await {
                Ok(info) => info.state.and_then(|s| s.running).unwrap_or(false),
                Err(_) => false,
            };

            if !alive {
                warn!("Container {} is dead, removing", container.container_id);
                let _ = self.destroy_container_by_id(&container.container_id).await;
            }
        }
        Ok(())
    }

    /// Destroy a container and clean up
    pub async fn destroy_container_by_id(&self, container_id: &str) -> Result<()> {
        let _ = self.detach_container(container_id).await;
        let _ = self.docker.remove_container(container_id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await;
        info!("Destroyed container {}", container_id);
        Ok(())
    }

    /// Restart a container by ID
    pub async fn restart_container_by_id(&self, container_id: &str) -> Result<()> {
        if !self.begin_restart(container_id).await {
            return Ok(());
        }

        let result = self
            .docker
            .restart_container(container_id, Some(RestartContainerOptions { t: Some(5), signal: None }))
            .await
            .map_err(Into::into);

        self.end_restart(container_id).await;
        result
    }

    /// Destroy all containers for an exploit
    pub async fn destroy_exploit_containers(&self, exploit_id: i32) -> Result<()> {
        let containers: Vec<String> = {
            let guard = self.registry.lock().await;
            guard.pools.get(&exploit_id).map(|p| p.iter().map(|c| c.container_id.clone()).collect()).unwrap_or_default()
        };
        for cid in containers {
            let _ = self.destroy_container_by_id(&cid).await;
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

            let run_ids = runs.iter().map(|r| r.id).collect::<Vec<_>>();
            let max_execs = exploit.max_per_container.max(1) as usize;

            let active_runs = runs.len().min(concurrent_limit);
            let mut needed = needed_containers(active_runs, exploit.max_per_container);
            if exploit.max_containers > 0 {
                needed = needed.min(exploit.max_containers as usize);
            }

            let existing = {
                let guard = self.registry.lock().await;
                guard.pools.get(&exploit.id)
                    .map(|p| p.iter().filter(|c| c.counter.load(Ordering::SeqCst) > 0).count())
                    .unwrap_or(0)
            };

            let to_spawn = needed.saturating_sub(existing);
            if to_spawn > 0 {
                info!("Pre-warming {} containers for exploit {} (need {}, have {})", to_spawn, exploit.name, needed, existing);
                let mut ids = run_ids.clone();
                ids.sort_unstable();
                ids.dedup();
                let unmapped = {
                    let mut guard = self.registry.lock().await;
                    let mut unmapped = Vec::new();
                    let mut to_unmap = Vec::new();
                    for run_id in &ids {
                        if let Some(container_id) = guard.affinity.get(run_id) {
                            let is_active = guard
                                .by_id
                                .get(container_id)
                                .map(|c| c.counter.load(Ordering::SeqCst) > 0)
                                .unwrap_or(false);
                            if is_active {
                                continue;
                            }
                            to_unmap.push(*run_id);
                        }
                        unmapped.push(*run_id);
                    }
                    for run_id in to_unmap {
                        let registry = &mut *guard;
                        drop_affinity_for_run(&mut registry.affinity, &mut registry.reverse_affinity, run_id);
                    }
                    unmapped
                };

                let mut spawned = 0usize;
                for chunk in unmapped.chunks(max_execs) {
                    if spawned >= to_spawn {
                        break;
                    }
                    if chunk.is_empty() {
                        continue;
                    }
                    if let Err(e) = self.spawn_container(&exploit, max_execs, Some(chunk.to_vec())).await {
                        error!("Failed to spawn container for {}: {}", exploit.name, e);
                    } else {
                        spawned += 1;
                    }
                }
            }
        }
        Ok(())
    }
}

fn needed_containers(active_runs: usize, max_per_container: i32) -> usize {
    let per_container = max_per_container.max(1) as usize;
    (active_runs + per_container - 1) / per_container
}

fn try_decrement_counter(container: &ManagedContainer) -> bool {
    loop {
        let current = container.counter.load(Ordering::SeqCst);
        if current <= 0 {
            return false;
        }
        if container
            .counter
            .compare_exchange(current, current - 1, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return true;
        }
    }
}

fn select_best_container<I>(containers: I) -> Option<Arc<ManagedContainer>>
where
    I: IntoIterator<Item = Arc<ManagedContainer>>,
{
    let mut best: Option<(usize, DateTime<Utc>, Arc<ManagedContainer>)> = None;
    for container in containers {
        if container.counter.load(Ordering::SeqCst) <= 0 {
            continue;
        }
        let available = container.exec_sem.available_permits();
        if available == 0 {
            continue;
        }
        let running = container.max_execs.saturating_sub(available);
        match &best {
            None => best = Some((running, container.created_at, container)),
            Some((best_running, best_created, _)) => {
                if running < *best_running || (running == *best_running && container.created_at < *best_created) {
                    best = Some((running, container.created_at, container));
                }
            }
        }
    }
    best.map(|(_, _, container)| container)
}

fn build_affinity_for_new_container(run_id: i32, run_ids: &[i32], mapped: &HashSet<i32>, max_per_container: usize) -> Vec<i32> {
    let mut result = Vec::new();
    if max_per_container == 0 {
        return result;
    }
    let mut seen = HashSet::new();
    result.push(run_id);
    seen.insert(run_id);

    for id in run_ids {
        if result.len() >= max_per_container {
            break;
        }
        if mapped.contains(id) || seen.contains(id) {
            continue;
        }
        result.push(*id);
        seen.insert(*id);
    }
    result
}

fn parse_affinity_list(value: &str) -> Vec<i32> {
    value
        .split(',')
        .filter_map(|v| v.trim().parse::<i32>().ok())
        .collect()
}

fn format_affinity_list(runs: &[i32]) -> String {
    let mut ids = runs.to_vec();
    ids.sort_unstable();
    ids.dedup();
    ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",")
}

fn sorted_run_ids(set: &HashSet<i32>) -> Vec<i32> {
    let mut ids: Vec<i32> = set.iter().copied().collect();
    ids.sort_unstable();
    ids
}

fn drop_affinity_for_run(
    affinity: &mut HashMap<i32, String>,
    reverse_affinity: &mut HashMap<String, HashSet<i32>>,
    run_id: i32,
) {
    if let Some(container_id) = affinity.remove(&run_id) {
        if let Some(set) = reverse_affinity.get_mut(&container_id) {
            set.remove(&run_id);
            if set.is_empty() {
                reverse_affinity.remove(&container_id);
            }
        }
    }
}

fn drop_affinity_for_container(
    affinity: &mut HashMap<i32, String>,
    reverse_affinity: &mut HashMap<String, HashSet<i32>>,
    container_id: &str,
) {
    if let Some(runs) = reverse_affinity.remove(container_id) {
        for run_id in runs {
            if affinity.get(&run_id).map(|cid| cid == container_id).unwrap_or(false) {
                affinity.remove(&run_id);
            }
        }
    }
}

fn parse_label_i32(labels: &HashMap<String, String>, key: &str) -> Option<i32> {
    labels.get(key).and_then(|v| v.parse::<i32>().ok())
}

fn timestamp_to_utc(ts: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(ts, 0).single().unwrap_or_else(Utc::now)
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
    use super::{
        build_affinity_for_new_container,
        collect_exec_output,
        drop_affinity_for_container,
        drop_affinity_for_run,
        needed_containers,
        parse_affinity_list,
        format_affinity_list,
        select_best_container,
        try_decrement_counter,
        ManagedContainer,
        MAX_OUTPUT,
    };
    use bollard::container::LogOutput;
    use chrono::{TimeZone, Utc};
    use futures::stream;
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, atomic::{AtomicI32, Ordering}};
    use std::time::Duration;
    use tokio::sync::{OwnedSemaphorePermit, Semaphore};

    fn make_container(
        counter: i32,
        max_execs: usize,
        running: usize,
        created_at: chrono::DateTime<Utc>,
    ) -> (Arc<ManagedContainer>, Vec<OwnedSemaphorePermit>) {
        let exec_sem = Arc::new(Semaphore::new(max_execs));
        let mut permits = Vec::new();
        for _ in 0..running {
            permits.push(exec_sem.clone().try_acquire_owned().expect("permit"));
        }
        let container = Arc::new(ManagedContainer {
            container_id: format!("c-{}-{}", counter, running),
            exploit_id: 1,
            counter: AtomicI32::new(counter),
            exec_sem,
            max_execs,
            created_at,
        });
        (container, permits)
    }

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

    #[test]
    fn needed_containers_rounds_up() {
        assert_eq!(needed_containers(1, 1), 1);
        assert_eq!(needed_containers(2, 1), 2);
        assert_eq!(needed_containers(3, 2), 2);
    }

    #[test]
    fn parse_affinity_list_handles_csv() {
        assert_eq!(parse_affinity_list("1, 2,3"), vec![1, 2, 3]);
    }

    #[test]
    fn format_affinity_list_sorts_and_dedups() {
        assert_eq!(format_affinity_list(&[3, 1, 1, 2]), "1,2,3");
    }

    #[test]
    fn drop_affinity_for_container_removes_mappings() {
        let mut affinity = HashMap::new();
        let mut reverse = HashMap::new();
        affinity.insert(1, "c1".to_string());
        affinity.insert(2, "c1".to_string());
        affinity.insert(3, "c2".to_string());
        reverse.insert("c1".to_string(), HashSet::from([1, 2]));
        reverse.insert("c2".to_string(), HashSet::from([3]));

        drop_affinity_for_container(&mut affinity, &mut reverse, "c1");

        assert!(!affinity.contains_key(&1));
        assert!(!affinity.contains_key(&2));
        assert!(affinity.contains_key(&3));
        assert!(!reverse.contains_key("c1"));
    }

    #[test]
    fn drop_affinity_for_run_removes_reverse_entry() {
        let mut affinity = HashMap::new();
        let mut reverse = HashMap::new();
        affinity.insert(1, "c1".to_string());
        affinity.insert(2, "c1".to_string());
        reverse.insert("c1".to_string(), HashSet::from([1, 2]));

        drop_affinity_for_run(&mut affinity, &mut reverse, 2);

        assert!(!affinity.contains_key(&2));
        assert!(affinity.contains_key(&1));
        assert_eq!(reverse.get("c1").unwrap(), &HashSet::from([1]));
    }

    #[test]
    fn try_decrement_counter_decrements_once() {
        let (container, _permits) = make_container(2, 1, 0, Utc::now());
        assert!(try_decrement_counter(&container));
        assert_eq!(container.counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn try_decrement_counter_refuses_at_zero() {
        let (container, _permits) = make_container(0, 1, 0, Utc::now());
        assert!(!try_decrement_counter(&container));
        assert_eq!(container.counter.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn build_affinity_for_new_container_skips_mapped_and_caps() {
        let run_ids = vec![1, 2, 3, 4];
        let mapped = HashSet::from([2, 4]);
        let result = build_affinity_for_new_container(3, &run_ids, &mapped, 2);
        assert_eq!(result, vec![3, 1]);
    }

    #[test]
    fn select_best_container_prefers_lowest_running_and_oldest() {
        let t_old = Utc.timestamp_opt(100, 0).single().unwrap();
        let t_new = Utc.timestamp_opt(200, 0).single().unwrap();
        let (c1, _p1) = make_container(5, 2, 0, t_new);
        let (c2, _p2) = make_container(5, 2, 1, t_old);
        let (c3, _p3) = make_container(0, 2, 0, t_old);

        let selected = select_best_container(vec![c2.clone(), c3.clone(), c1.clone()]).unwrap();
        assert_eq!(selected.container_id, c1.container_id);

        let (c4, _p4) = make_container(5, 2, 0, t_old);
        let tie_selected = select_best_container(vec![c1.clone(), c4.clone()]).unwrap();
        assert_eq!(tie_selected.container_id, c4.container_id);
    }
}
