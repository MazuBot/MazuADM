use crate::{Database, Exploit, ExploitJob, ExploitRun, ConnectionInfo, WsMessage, Challenge, Team};
use crate::container_manager::{ContainerManager, ContainerRegistry, ContainerRegistryHandle, ExecResult};
use anyhow::Result;
use std::future::Future;
use std::time::{Instant, Duration};
use std::sync::Arc;
use tokio::sync::{broadcast, oneshot, Mutex};
use tokio::time::Instant as TokioInstant;
use futures::future::{AbortHandle, Abortable};
use regex::Regex;
use crate::settings::{compute_timeout, load_job_settings};
use dashmap::DashMap;

pub struct Executor {
    pub db: Database,
    pub container_manager: ContainerManager,
    pub(crate) container_registry: ContainerRegistryHandle,
    pub tx: broadcast::Sender<WsMessage>,
    stop_tx: broadcast::Sender<StopSignal>,
    exploit_executors: DashMap<i32, ExploitExecutor>,
}

pub(crate) struct JobContext {
    pub(crate) job: ExploitJob,
    pub(crate) run: ExploitRun,
    pub(crate) exploit: Exploit,
    pub(crate) challenge: Challenge,
    pub(crate) team: Team,
    pub(crate) conn: ConnectionInfo,
}

#[derive(Clone, Debug)]
struct StopSignal {
    job_id: i32,
}

#[derive(Clone)]
struct ExploitExecutor {
    max_containers: i32,
    max_per_container: i32,
    default_counter: i32,
    gate: Arc<Mutex<()>>,
}

pub(crate) enum JobContextError {
    NotPending,
    MissingExploitRunId,
    MissingConnectionInfo,
    Db(anyhow::Error),
}

fn broadcast<T: serde::Serialize>(tx: &broadcast::Sender<WsMessage>, msg_type: &str, data: &T) {
    let _ = tx.send(WsMessage::new(msg_type, data));
}

impl Executor {
    pub fn new(db: Database, tx: broadcast::Sender<WsMessage>) -> Result<Self> {
        let container_manager = ContainerManager::new(db.clone(), tx.clone())?;
        let container_registry = Arc::new(Mutex::new(ContainerRegistry::default()));
        let (stop_tx, _stop_rx) = broadcast::channel::<StopSignal>(128);
        Ok(Self {
            db,
            container_manager,
            container_registry,
            tx,
            stop_tx,
            exploit_executors: DashMap::new(),
        })
    }

    pub async fn restore_from_docker(&self) -> Result<()> {
        self.container_manager
            .restore_from_docker(&self.container_registry)
            .await
    }

    async fn get_exploit_executor(&self, exploit: &Exploit) -> ExploitExecutor {
        let mut entry = self.exploit_executors.entry(exploit.id).or_insert_with(|| ExploitExecutor {
            max_containers: exploit.max_containers,
            max_per_container: exploit.max_per_container,
            default_counter: exploit.default_counter,
            gate: Arc::new(Mutex::new(())),
        });
        if entry.max_containers != exploit.max_containers {
            entry.max_containers = exploit.max_containers;
        }
        if entry.max_per_container != exploit.max_per_container {
            entry.max_per_container = exploit.max_per_container;
        }
        if entry.default_counter != exploit.default_counter {
            entry.default_counter = exploit.default_counter;
        }
        entry.clone()
    }

    pub async fn execute_job(&self, job: &ExploitJob, run: &ExploitRun, exploit: &Exploit, conn: &ConnectionInfo, flag_regex: Option<&str>, timeout_secs: u64, max_flags: usize) -> Result<JobResult> {
        let start = Instant::now();
        let stop_rx = self.stop_tx.subscribe();
        self.db.mark_job_running(job.id).await?;

        tracing::info!(
            "Executing job {} exploit {} run {} team {} challenge {} timeout {}s",
            job.id,
            exploit.id,
            run.id,
            job.team_id,
            run.challenge_id,
            timeout_secs
        );
        
        // Broadcast job running
        if let Ok(updated_job) = self.db.get_job(job.id).await {
            broadcast(&self.tx, "job_updated", &updated_job.without_logs());
        }

        let team = self.db.get_team(job.team_id).await?;
        
        let exploit_exec = self.get_exploit_executor(exploit).await;
        let _guard = exploit_exec.gate.lock().await;
        let lease = self
            .container_manager
            .lease_container(&self.container_registry, exploit, run.id)
            .await?;
        drop(_guard);
        self.db.set_job_container(job.id, lease.container_id()).await?;
        tracing::info!("Job {} using container {}", job.id, lease.container_id());

        // Build command - use entrypoint or docker image default cmd
        let args = vec![conn.addr.clone(), conn.port.to_string(), team.team_id.clone()];
        let cmd = match &exploit.entrypoint {
            Some(ep) => [vec![ep.clone()], args].concat(),
            None => {
                let image_cmd = self.container_manager.get_image_cmd(&exploit.docker_image).await.unwrap_or_default();
                [image_cmd, args].concat()
            }
        };

        let mut env = vec![
            format!("TARGET_HOST={}", conn.addr),
            format!("TARGET_PORT={}", conn.port),
            format!("TARGET_TEAM_ID={}", team.team_id),
        ];
        if let Some(envs_json) = job.envs.as_ref() {
            if let Ok(envs_map) = serde_json::from_str::<std::collections::HashMap<String, String>>(envs_json) {
                for (k, v) in envs_map {
                    env.push(format!("{}={}", k, v));
                }
            }
        }

        let (pid_tx, pid_rx) = oneshot::channel::<i64>();
        let container_id = lease.container_id().to_string();

        // Execute with timeout and PID tracking
        let manager = &self.container_manager;
        let kill_container_id = container_id.clone();
        let kill = move |pid: i64| {
            let container_id = kill_container_id.clone();
            async move {
                let _ = manager.kill_process_in_container(&container_id, pid).await;
            }
        };
        let exec_container_id = container_id.clone();
        let exec_future = self.container_manager.execute_in_container_with_timeout(
            &exec_container_id,
            cmd,
            env,
            Duration::from_secs(timeout_secs),
            Some(pid_tx),
        );
        let exec_result = run_exec_with_stop(
            exec_future,
            pid_rx,
            stop_rx,
            job.id,
            Duration::from_millis(500),
            kill,
        )
        .await;
        let result = match exec_result {
            Ok(result) => result,
            Err(e) => {
                lease.finish().await;
                return Err(e);
            }
        };

        // Kill process on timeout or OLE
        if result.timed_out || result.ole {
            if let Some(p) = result.pid {
                let _ = self.container_manager.kill_process_in_container(&container_id, p).await;
            }
        }

        let (stdout, stderr, exit_code, timed_out, ole) = 
            (result.stdout, result.stderr, result.exit_code, result.timed_out, result.ole);

        let duration_ms = start.elapsed().as_millis() as i32;
        let combined_output = if stderr.is_empty() {
            stdout.clone()
        } else if stdout.is_empty() {
            stderr.clone()
        } else {
            format!("{}\n{}", stdout, stderr)
        };
        let flags = Self::extract_flags(&combined_output, flag_regex, max_flags);
        
        let status = derive_job_status(!flags.is_empty(), timed_out, ole, exit_code);
        let mut final_status = status;
        let mut final_stderr = stderr.clone();
        if let Ok(current) = self.db.get_job(job.id).await {
            if current.status == "stopped" && status != "flag" {
                final_status = "stopped";
                if let Some(existing) = current.stderr {
                    if !existing.is_empty() {
                        final_stderr = if final_stderr.is_empty() {
                            existing
                        } else {
                            format!("{}\n{}", existing, final_stderr)
                        };
                    }
                }
            }
        }
        
        self.db.finish_job(job.id, final_status, Some(&stdout), Some(&final_stderr), duration_ms).await?;
        
        // Broadcast job finished
        if let Ok(updated_job) = self.db.get_job(job.id).await {
            broadcast(&self.tx, "job_updated", &updated_job.without_logs());
        }
        lease.finish().await;

        tracing::info!(
            "Job {} completed status={} duration_ms={} flags={}",
            job.id,
            final_status,
            duration_ms,
            flags.len()
        );
        Ok(JobResult { stdout, stderr, duration_ms, exit_code, flags })
    }

    pub async fn run_job_immediately(&self, job_id: i32) -> Result<JobResult> {
        let ctx = match build_job_context(&self.db, job_id).await {
            Ok(ctx) => ctx,
            Err(JobContextError::NotPending) => {
                return Err(anyhow::anyhow!("Job {} is not pending", job_id));
            }
            Err(JobContextError::MissingExploitRunId) => {
                finish_job_and_broadcast(&self.db, &self.tx, job_id, "error", None, Some("Job missing exploit_run_id"), 0).await;
                return Err(anyhow::anyhow!("Job {} missing exploit_run_id", job_id));
            }
            Err(JobContextError::MissingConnectionInfo) => {
                if let Err(e) = self.db.mark_job_scheduled(job_id).await {
                    tracing::warn!("Failed to mark job {} scheduled before missing connection info: {}", job_id, e);
                }
                finish_job_and_broadcast(&self.db, &self.tx, job_id, "error", None, Some("No connection info (missing IP or port)"), 0).await;
                return Err(anyhow::anyhow!("Job {} missing connection info", job_id));
            }
            Err(JobContextError::Db(e)) => {
                return Err(e);
            }
        };

        let settings = load_job_settings(&self.db).await;
        let timeout = compute_timeout(ctx.exploit.timeout_secs, settings.worker_timeout);

        let result = self.execute_job(&ctx.job, &ctx.run, &ctx.exploit, &ctx.conn, ctx.challenge.flag_regex.as_deref(), timeout, settings.max_flags).await;

        match result {
            Ok(result) => {
                for flag in &result.flags {
                    let f = self.db.create_flag(ctx.job.id, ctx.job.round_id, ctx.challenge.id, ctx.team.id, flag).await;
                    if let Ok(f) = f {
                        broadcast(&self.tx, "flag_created", &f);
                    }
                }
                Ok(result)
            }
            Err(e) => {
                if let Ok(current) = self.db.get_job(ctx.job.id).await {
                    if current.status == "stopped" {
                        broadcast(&self.tx, "job_updated", &current.without_logs());
                        return Err(e);
                    }
                }
                let _ = self.db.finish_job(ctx.job.id, "error", None, Some(&e.to_string()), 0).await;
                if let Ok(updated) = self.db.get_job(ctx.job.id).await {
                    broadcast(&self.tx, "job_updated", &updated.without_logs());
                }
                Err(e)
            }
        }
    }

    pub async fn stop_job(&self, job_id: i32, reason: &str) -> Result<ExploitJob> {
        self.stop_job_with_flags(job_id, false, reason).await
    }

    pub async fn stop_job_with_flags(&self, job_id: i32, has_flag: bool, reason: &str) -> Result<ExploitJob> {
        let job = self.db.get_job(job_id).await?;
        if job.status != "running" {
            return Ok(job);
        }

        let _ = self.stop_tx.send(StopSignal { job_id });

        self.db.mark_job_stopped_with_reason(job_id, has_flag, reason).await?;
        let job = self.db.get_job(job_id).await?;
        broadcast(&self.tx, "job_updated", &job.without_logs());
        Ok(job)
    }

    pub fn extract_flags(output: &str, pattern: Option<&str>, max_flags: usize) -> Vec<String> {
        let pattern = pattern.unwrap_or(r"[A-Za-z0-9]{31}=");
        let Ok(re) = Regex::new(pattern) else { return vec![] };
        let mut seen = std::collections::HashSet::new();
        re.find_iter(output)
            .filter_map(|m| {
                let s = m.as_str().to_string();
                if seen.insert(s.clone()) { Some(s) } else { None }
            })
            .take(max_flags)
            .collect()
    }

}

// settings helpers live in crate::settings

fn skip_reason(exploit_enabled: bool, team_enabled: bool, skip_on_flag: bool, has_flag: bool) -> Option<&'static str> {
    if !exploit_enabled {
        Some("Exploit disabled")
    } else if !team_enabled {
        Some("Team disabled")
    } else if skip_on_flag && has_flag {
        Some("Flag already found")
    } else {
        None
    }
}

pub(crate) async fn should_skip_job(
    db: &Database,
    tx: &broadcast::Sender<WsMessage>,
    ctx: &JobContext,
    skip_on_flag: bool,
    round_id: i32,
) -> bool {
    if ctx.job.create_reason.as_ref().map(|r| r.starts_with("rerun_job:")).unwrap_or(false) {
        return false;
    }
    let has_flag = if skip_on_flag {
        db.has_flag_for(round_id, ctx.challenge.id, ctx.team.id).await.ok().unwrap_or(false)
    } else {
        false
    };

    if let Some(reason) = skip_reason(ctx.exploit.enabled, ctx.team.enabled, skip_on_flag, has_flag) {
        finish_job_and_broadcast(db, tx, ctx.job.id, "skipped", None, Some(reason), 0).await;
        return true;
    }
    false
}

pub(crate) async fn build_job_context(db: &Database, job_id: i32) -> std::result::Result<JobContext, JobContextError> {
    let job = db.get_job(job_id).await.map_err(JobContextError::Db)?;
    ensure_pending(&job)?;
    let exploit_run_id = require_exploit_run_id(&job)?;

    let run = db.get_exploit_run(exploit_run_id).await.map_err(JobContextError::Db)?;
    let exploit = db.get_exploit(run.exploit_id).await.map_err(JobContextError::Db)?;
    let challenge = db.get_challenge(run.challenge_id).await.map_err(JobContextError::Db)?;
    let team = db.get_team(job.team_id).await.map_err(JobContextError::Db)?;

    let rel = db.get_relation(challenge.id, team.id).await.map_err(JobContextError::Db)?;
    let conn = if exploit.ignore_connection_info {
        ConnectionInfo { addr: String::new(), port: 0 }
    } else {
        rel.and_then(|r| r.connection_info(&challenge, &team)).ok_or(JobContextError::MissingConnectionInfo)?
    };

    Ok(JobContext { job, run, exploit, challenge, team, conn })
}

pub(crate) async fn build_job_context_or_finish(
    db: &Database,
    tx: &broadcast::Sender<WsMessage>,
    job_id: i32,
) -> Option<JobContext> {
    match build_job_context(db, job_id).await {
        Ok(ctx) => Some(ctx),
        Err(JobContextError::NotPending) => None,
        Err(JobContextError::MissingExploitRunId) => {
            finish_job_and_broadcast(db, tx, job_id, "error", None, Some("Job missing exploit_run_id"), 0).await;
            None
        }
        Err(JobContextError::MissingConnectionInfo) => {
            if let Err(e) = db.mark_job_scheduled(job_id).await {
                tracing::warn!("Failed to mark job {} scheduled before missing connection info: {}", job_id, e);
            }
            finish_job_and_broadcast(db, tx, job_id, "error", None, Some("No connection info (missing IP or port)"), 0).await;
            None
        }
        Err(JobContextError::Db(e)) => {
            log_job_error(job_id, &e);
            None
        }
    }
}

fn ensure_pending(job: &ExploitJob) -> std::result::Result<(), JobContextError> {
    if job.status == "pending" {
        Ok(())
    } else {
        Err(JobContextError::NotPending)
    }
}

fn require_exploit_run_id(job: &ExploitJob) -> std::result::Result<i32, JobContextError> {
    job.exploit_run_id.ok_or(JobContextError::MissingExploitRunId)
}

pub(crate) fn get_target_lock(
    target_locks: &DashMap<(i32, i32), Arc<Mutex<()>>>,
    sequential_per_target: bool,
    key: (i32, i32),
) -> Option<Arc<Mutex<()>>> {
    if !sequential_per_target {
        return None;
    }
    let entry = target_locks.entry(key).or_insert_with(|| Arc::new(Mutex::new(())));
    Some(entry.clone())
}

pub(crate) async fn finish_job_and_broadcast(
    db: &Database,
    tx: &broadcast::Sender<WsMessage>,
    job_id: i32,
    status: &str,
    stdout: Option<&str>,
    stderr: Option<&str>,
    duration_ms: i32,
) {
    let _ = db.finish_job(job_id, status, stdout, stderr, duration_ms).await;
    if let Ok(j) = db.get_job(job_id).await {
        broadcast(tx, "job_updated", &j.without_logs());
    }
}

pub(crate) fn log_job_error<E: std::fmt::Display>(job_id: i32, err: &E) {
    tracing::error!("Job {} failed: {}", job_id, err);
}

fn derive_job_status(flags_found: bool, timed_out: bool, ole: bool, exit_code: i64) -> &'static str {
    if flags_found {
        "flag"
    } else if timed_out {
        "timeout"
    } else if ole {
        "ole"
    } else if exit_code == 0 {
        "success"
    } else {
        "failed"
    }
}

fn stopped_exec_result(pid: Option<i64>) -> ExecResult {
    ExecResult {
        stdout: String::new(),
        stderr: "stopped".to_string(),
        exit_code: -1,
        ole: false,
        timed_out: false,
        pid,
    }
}

async fn run_exec_with_stop<F, K, Fut>(
    exec_future: F,
    mut pid_rx: oneshot::Receiver<i64>,
    mut stop_rx: broadcast::Receiver<StopSignal>,
    job_id: i32,
    stop_grace: Duration,
    mut kill: K,
) -> Result<ExecResult>
where
    F: Future<Output = Result<ExecResult>> + Send,
    K: FnMut(i64) -> Fut + Send,
    Fut: Future<Output = ()> + Send,
{
    let (abort_handle, abort_reg) = AbortHandle::new_pair();
    let exec_future = Abortable::new(exec_future, abort_reg);
    tokio::pin!(exec_future);
    let mut exec_done = false;
    let mut pid: Option<i64> = None;
    let mut pid_pending = true;
    let mut stop_requested = false;
    let mut stop_rx_closed = false;
    let mut stop_deadline: Option<TokioInstant> = None;
    let mut exec_cancelled = false;

    loop {
        tokio::select! {
            res = &mut exec_future, if !exec_done => {
                exec_done = true;
                match res {
                    Ok(result) => return result,
                    Err(_) if stop_requested => {
                        exec_cancelled = true;
                        if !pid_pending {
                            return Ok(stopped_exec_result(pid));
                        }
                    }
                    Err(err) => return Err(anyhow::Error::new(err)),
                }
            }
            recv = &mut pid_rx, if pid_pending => {
                pid_pending = false;
                if let Ok(found_pid) = recv {
                    pid = Some(found_pid);
                    if stop_requested {
                        kill(found_pid).await;
                    }
                }
                if exec_cancelled && stop_requested {
                    return Ok(stopped_exec_result(pid));
                }
            }
            msg = stop_rx.recv(), if !stop_rx_closed => {
                match msg {
                    Ok(signal) => {
                        if signal.job_id == job_id {
                            stop_requested = true;
                            if let Some(found_pid) = pid {
                                kill(found_pid).await;
                            }
                            if stop_deadline.is_none() {
                                stop_deadline = Some(TokioInstant::now() + stop_grace);
                            }
                            if !exec_done {
                                abort_handle.abort();
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => {
                        stop_rx_closed = true;
                    }
                }
            }
            _ = async {
                if let Some(deadline) = stop_deadline {
                    tokio::time::sleep_until(deadline).await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                if !exec_done {
                    abort_handle.abort();
                }
                return Ok(stopped_exec_result(pid));
            }
        }
    }
}

pub(crate) fn stagger_delay_ms(job_id: i32) -> u64 {
    (job_id.unsigned_abs() as u64) % 500
}

pub struct JobResult {
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: i32,
    pub exit_code: i64,
    pub flags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::{
        derive_job_status,
        ensure_pending,
        ExecResult,
        get_target_lock,
        require_exploit_run_id,
        run_exec_with_stop,
        skip_reason,
        StopSignal,
        stagger_delay_ms,
    };
    use crate::ExploitJob;
    use chrono::{TimeZone, Utc};
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    use std::time::Duration;
    use tokio::sync::{broadcast, oneshot, Mutex};
    use tokio::time::{sleep, timeout};
    use dashmap::DashMap;

    fn make_job(status: &str, exploit_run_id: Option<i32>) -> ExploitJob {
        ExploitJob {
            id: 1,
            round_id: 1,
            exploit_run_id,
            team_id: 1,
            priority: 0,
            status: status.to_string(),
            container_id: None,
            stdout: None,
            stderr: None,
            create_reason: None,
            duration_ms: None,
            schedule_at: None,
            started_at: None,
            finished_at: None,
            created_at: Utc.timestamp_opt(0, 0).single().unwrap(),
        }
    }

    #[test]
    fn derive_job_status_flag() {
        assert_eq!(derive_job_status(true, false, false, 1), "flag");
    }

    #[test]
    fn derive_job_status_timeout() {
        assert_eq!(derive_job_status(false, true, false, 0), "timeout");
    }

    #[test]
    fn derive_job_status_ole() {
        assert_eq!(derive_job_status(false, false, true, 0), "ole");
    }

    #[test]
    fn derive_job_status_success() {
        assert_eq!(derive_job_status(false, false, false, 0), "success");
    }

    #[test]
    fn derive_job_status_failed() {
        assert_eq!(derive_job_status(false, false, false, 1), "failed");
    }

    #[test]
    fn stagger_delay_is_deterministic() {
        assert_eq!(stagger_delay_ms(123), (123_u64 % 500));
    }

    #[test]
    fn ensure_pending_rejects_non_pending() {
        let job = make_job("running", Some(1));
        assert!(ensure_pending(&job).is_err());
    }

    #[test]
    fn require_exploit_run_id_errors() {
        let job = make_job("pending", None);
        assert!(require_exploit_run_id(&job).is_err());
    }

    #[test]
    fn skip_reason_precedence() {
        assert_eq!(skip_reason(false, true, true, true), Some("Exploit disabled"));
        assert_eq!(skip_reason(true, false, true, true), Some("Team disabled"));
        assert_eq!(skip_reason(true, true, true, true), Some("Flag already found"));
        assert_eq!(skip_reason(true, true, true, false), None);
    }

    #[test]
    fn get_target_lock_none_when_disabled() {
        let locks: DashMap<(i32, i32), Arc<Mutex<()>>> = DashMap::new();
        let lock = get_target_lock(&locks, false, (1, 1));
        assert!(lock.is_none());
    }

    #[test]
    fn get_target_lock_reuses_arc() {
        let locks: DashMap<(i32, i32), Arc<Mutex<()>>> = DashMap::new();
        let first = get_target_lock(&locks, true, (1, 1)).unwrap();
        let second = get_target_lock(&locks, true, (1, 1)).unwrap();
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[tokio::test]
    async fn stop_without_pid_finishes() {
        let (_pid_tx, pid_rx) = oneshot::channel::<i64>();
        let (stop_tx, stop_rx) = broadcast::channel(4);
        let exec_future = async {
            sleep(Duration::from_secs(5)).await;
            Ok(ExecResult {
                stdout: "late".to_string(),
                stderr: String::new(),
                exit_code: 0,
                ole: false,
                timed_out: false,
                pid: None,
            })
        };
        let kill_count = Arc::new(AtomicUsize::new(0));
        let kill = {
            let kill_count = kill_count.clone();
            move |_pid| {
                let kill_count = kill_count.clone();
                async move {
                    kill_count.fetch_add(1, Ordering::SeqCst);
                }
            }
        };

        let handle = tokio::spawn(run_exec_with_stop(
            exec_future,
            pid_rx,
            stop_rx,
            42,
            Duration::from_millis(50),
            kill,
        ));
        let _ = stop_tx.send(StopSignal { job_id: 42 });

        let result = timeout(Duration::from_millis(200), handle)
            .await
            .expect("timeout")
            .expect("join")
            .expect("exec");
        assert_eq!(result.stderr, "stopped");
        assert_eq!(kill_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn stop_uses_output_if_exec_completes() {
        let (_pid_tx, pid_rx) = oneshot::channel::<i64>();
        let (stop_tx, stop_rx) = broadcast::channel(4);
        let exec_future = async {
            sleep(Duration::from_millis(10)).await;
            Ok(ExecResult {
                stdout: "ok".to_string(),
                stderr: String::new(),
                exit_code: 0,
                ole: false,
                timed_out: false,
                pid: None,
            })
        };
        let kill = |_pid| async {};
        let stop_sender = tokio::spawn(async move {
            sleep(Duration::from_millis(20)).await;
            let _ = stop_tx.send(StopSignal { job_id: 7 });
        });

        let result = run_exec_with_stop(
            exec_future,
            pid_rx,
            stop_rx,
            7,
            Duration::from_millis(50),
            kill,
        )
        .await
        .expect("exec");
        let _ = stop_sender.await;

        assert_eq!(result.stdout, "ok");
    }

    #[tokio::test]
    async fn stop_kills_pid_when_arrives_after_stop() {
        let (pid_tx, pid_rx) = oneshot::channel::<i64>();
        let (stop_tx, stop_rx) = broadcast::channel(4);
        let exec_future = async {
            sleep(Duration::from_secs(5)).await;
            Ok(ExecResult {
                stdout: String::new(),
                stderr: String::new(),
                exit_code: 0,
                ole: false,
                timed_out: false,
                pid: None,
            })
        };
        let kill_count = Arc::new(AtomicUsize::new(0));
        let kill = {
            let kill_count = kill_count.clone();
            move |_pid| {
                let kill_count = kill_count.clone();
                async move {
                    kill_count.fetch_add(1, Ordering::SeqCst);
                }
            }
        };

        let handle = tokio::spawn(run_exec_with_stop(
            exec_future,
            pid_rx,
            stop_rx,
            9,
            Duration::from_millis(100),
            kill,
        ));
        let _ = stop_tx.send(StopSignal { job_id: 9 });
        sleep(Duration::from_millis(10)).await;
        let _ = pid_tx.send(123);

        let result = handle.await.expect("join").expect("exec");
        assert_eq!(result.stderr, "stopped");
        assert_eq!(kill_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn run_exec_with_stop_aborts_task_on_deadline() {
        use std::sync::atomic::{AtomicBool, Ordering};

        struct DropFlag(Arc<AtomicBool>);
        impl Drop for DropFlag {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        let dropped = Arc::new(AtomicBool::new(false));
        let drop_flag = DropFlag(dropped.clone());

        let exec_future = async move {
            let _guard = drop_flag;
            std::future::pending::<anyhow::Result<ExecResult>>().await
        };

        let (pid_tx, pid_rx) = oneshot::channel();
        drop(pid_tx);

        let (stop_tx, stop_rx) = broadcast::channel(1);
        let _ = stop_tx.send(StopSignal { job_id: 42 });

        let result = run_exec_with_stop(
            exec_future,
            pid_rx,
            stop_rx,
            42,
            Duration::from_millis(10),
            |_| async {},
        )
        .await
        .unwrap();

        assert_eq!(result.stderr, "stopped");
        assert!(dropped.load(Ordering::SeqCst));
    }
}
