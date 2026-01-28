use crate::{Database, Exploit, ExploitJob, ExploitRun, ConnectionInfo, WsMessage};
use crate::container_manager::ContainerManager;
use anyhow::Result;
use std::time::{Instant, Duration};
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{broadcast, Semaphore, Mutex};
use tokio::task::JoinSet;
use regex::Regex;
use crate::settings::{compute_timeout, load_executor_settings};

#[derive(Clone)]
pub struct Executor {
    pub db: Database,
    pub container_manager: ContainerManager,
    pub tx: broadcast::Sender<WsMessage>,
}

fn broadcast<T: serde::Serialize>(tx: &broadcast::Sender<WsMessage>, msg_type: &str, data: &T) {
    let _ = tx.send(WsMessage::new(msg_type, data));
}

impl Executor {
    pub fn new(db: Database, tx: broadcast::Sender<WsMessage>) -> Result<Self> {
        let container_manager = ContainerManager::new(db.clone())?;
        Ok(Self { db, container_manager, tx })
    }

    pub async fn execute_job(&self, job: &ExploitJob, run: &ExploitRun, exploit: &Exploit, conn: &ConnectionInfo, flag_regex: Option<&str>, timeout_secs: u64, max_flags: usize) -> Result<JobResult> {
        let start = Instant::now();
        self.db.update_job_status(job.id, "running", true).await?;
        
        // Broadcast job running
        if let Ok(updated_job) = self.db.get_job(job.id).await {
            broadcast(&self.tx, "job_updated", &updated_job);
        }

        let team = self.db.get_team(job.team_id).await?;
        
        // Get or assign persistent container
        let container = self.container_manager.get_or_assign_container(run).await?;
        self.db.set_job_container(job.id, &container.container_id).await?;

        // Build command - use entrypoint or docker image default cmd
        let args = vec![conn.addr.clone(), conn.port.to_string(), team.team_id.clone()];
        let cmd = match &exploit.entrypoint {
            Some(ep) => [vec![ep.clone()], args].concat(),
            None => {
                let image_cmd = self.container_manager.get_image_cmd(&exploit.docker_image).await.unwrap_or_default();
                [image_cmd, args].concat()
            }
        };

        let env = vec![
            format!("TARGET_HOST={}", conn.addr),
            format!("TARGET_PORT={}", conn.port),
            format!("TARGET_TEAM_ID={}", team.team_id),
        ];

        // Execute with timeout and PID tracking
        let result = self.container_manager.execute_in_container_with_timeout(
            &container, cmd, env, Duration::from_secs(timeout_secs)
        ).await?;

        // Kill process on timeout or OLE
        if result.timed_out || result.ole {
            if let Some(p) = result.pid {
                let _ = self.container_manager.kill_process_in_container(&container.container_id, p).await;
            }
        }

        let (stdout, stderr, exit_code, timed_out, ole) = 
            (result.stdout, result.stderr, result.exit_code, result.timed_out, result.ole);

        // Decrement counter and destroy if exhausted
        let new_counter = self.db.decrement_container_counter(container.id).await?;
        if new_counter <= 0 {
            // Only destroy if no other jobs are using this container
            let active = self.db.count_running_jobs_for_container(&container.container_id).await.unwrap_or(1);
            if active <= 1 {
                let _ = self.container_manager.destroy_container(container.id).await;
            }
        }

        let duration_ms = start.elapsed().as_millis() as i32;
        let flags = Self::extract_flags(&stdout, flag_regex, max_flags);
        
        let status = derive_job_status(!flags.is_empty(), timed_out, ole, exit_code);
        
        self.db.finish_job(job.id, status, Some(&stdout), Some(&stderr), duration_ms).await?;
        
        // Broadcast job finished
        if let Ok(updated_job) = self.db.get_job(job.id).await {
            broadcast(&self.tx, "job_updated", &updated_job);
        }

        Ok(JobResult { stdout, stderr, duration_ms, exit_code, flags })
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

    pub async fn run_round(&self, round_id: i32) -> Result<()> {
        // Mark round as running
        self.db.start_round(round_id).await?;
        if let Ok(round) = self.db.get_round(round_id).await {
            broadcast(&self.tx, "round_updated", &round);
        }

        // Health check containers before round
        self.container_manager.health_check().await?;

        let settings = load_executor_settings(&self.db).await;
        let concurrent_limit = settings.concurrent_limit;
        let worker_timeout = settings.worker_timeout;
        let max_flags = settings.max_flags;
        let skip_on_flag = settings.skip_on_flag;
        let sequential_per_target = settings.sequential_per_target;

        // Pre-warm containers
        self.container_manager.prewarm_for_round(concurrent_limit).await?;

        let jobs = self.db.get_pending_jobs(round_id).await?;
        let semaphore = Arc::new(Semaphore::new(concurrent_limit));
        
        // Per-target locks for sequential execution
        let target_locks: Arc<Mutex<HashMap<(i32, i32), Arc<Mutex<()>>>>> = Arc::new(Mutex::new(HashMap::new()));
        
        let mut join_set = JoinSet::new();

        let base_executor = self.clone();

        for job in jobs {
            let permit = semaphore.clone().acquire_owned().await?;
            let db = self.db.clone();
            let tx = self.tx.clone();
            let executor = base_executor.clone();
            let target_locks = target_locks.clone();

            join_set.spawn(async move {
                let _permit = permit;
                
                // Re-check job status before running (may have been skipped/stopped)
                let current_job = match db.get_job(job.id).await {
                    Ok(j) => j,
                    Err(_) => return,
                };
                if current_job.status != "pending" {
                    return;
                }

                let exploit_run_id = match job.exploit_run_id {
                    Some(id) => id,
                    None => {
                        finish_job_and_broadcast(&db, &tx, job.id, "error", None, Some("Job missing exploit_run_id"), 0).await;
                        return;
                    }
                };

                let run: ExploitRun = match db.get_exploit_run(exploit_run_id).await {
                    Ok(r) => r,
                    Err(e) => { log_job_error(job.id, &e); return; }
                };
                
                let exploit = match db.get_exploit(run.exploit_id).await {
                    Ok(e) => e,
                    Err(e) => { log_job_error(job.id, &e); return; }
                };
                
                let challenge = match db.get_challenge(run.challenge_id).await {
                    Ok(c) => c,
                    Err(e) => { log_job_error(job.id, &e); return; }
                };
                
                let team = match db.get_team(job.team_id).await {
                    Ok(t) => t,
                    Err(e) => { log_job_error(job.id, &e); return; }
                };

                if !exploit.enabled {
                    finish_job_and_broadcast(&db, &tx, job.id, "skipped", None, Some("Exploit disabled"), 0).await;
                    return;
                }

                if !team.enabled {
                    finish_job_and_broadcast(&db, &tx, job.id, "skipped", None, Some("Team disabled"), 0).await;
                    return;
                }

                if skip_on_flag && has_flag_and_skip(&db, &tx, round_id, job.id, challenge.id, team.id).await {
                    return;
                }

                let rel = match db.get_relation(challenge.id, team.id).await {
                    Ok(r) => r,
                    Err(e) => { log_job_error(job.id, &e); return; }
                };
                
                let conn = match rel.and_then(|r| r.connection_info(&challenge, &team)) {
                    Some(c) => c,
                    None => {
                        finish_job_and_broadcast(&db, &tx, job.id, "error", None, Some("No connection info (missing IP or port)"), 0).await;
                        return;
                    }
                };

                // Get or create per-target lock
                let target_lock = if sequential_per_target {
                    let mut locks = target_locks.lock().await;
                    Some(locks.entry((challenge.id, team.id)).or_insert_with(|| Arc::new(Mutex::new(()))).clone())
                } else {
                    None
                };

                // Acquire target lock if sequential mode
                let _target_guard = match &target_lock {
                    Some(lock) => Some(lock.lock().await),
                    None => None,
                };

                // Re-check skip_on_flag after acquiring lock
                if skip_on_flag && has_flag_and_skip(&db, &tx, round_id, job.id, challenge.id, team.id).await {
                    return;
                }

                // Random delay 0-500ms to spread container reuse
                let delay = stagger_delay_ms(job.id);
                tokio::time::sleep(Duration::from_millis(delay)).await;

                // Use exploit timeout if set, otherwise global worker_timeout
                let timeout = compute_timeout(exploit.timeout_secs, worker_timeout);

                match executor.execute_job(&job, &run, &exploit, &conn, challenge.flag_regex.as_deref(), timeout, max_flags).await {
                    Ok(result) => {
                        for flag in result.flags {
                            if let Ok(f) = db.create_flag(job.id, round_id, challenge.id, team.id, &flag).await {
                                broadcast(&tx, "flag_created", &f);
                            }
                        }
                    }
                    Err(e) => {
                        log_job_error(job.id, &e);
                        finish_job_and_broadcast(&db, &tx, job.id, "error", None, Some(&e.to_string()), 0).await;
                    }
                }
            });
        }

        while let Some(result) = join_set.join_next().await {
            if let Err(e) = result {
                tracing::error!("Job task failed: {}", e);
            }
        }
        
        // Don't finish round here - keep it running until next round starts
        Ok(())
    }
}

// settings helpers live in crate::settings

async fn has_flag_and_skip(
    db: &Database,
    tx: &broadcast::Sender<WsMessage>,
    round_id: i32,
    job_id: i32,
    challenge_id: i32,
    team_id: i32,
) -> bool {
    if let Ok(true) = db.has_flag_for(round_id, challenge_id, team_id).await {
        finish_job_and_broadcast(db, tx, job_id, "skipped", None, Some("Flag already found"), 0).await;
        return true;
    }
    false
}

async fn finish_job_and_broadcast(
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
        broadcast(tx, "job_updated", &j);
    }
}

fn log_job_error<E: std::fmt::Display>(job_id: i32, err: &E) {
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

fn stagger_delay_ms(job_id: i32) -> u64 {
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
    use super::{derive_job_status, stagger_delay_ms, Executor};
    use crate::container_manager::ContainerManager;

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
    fn executor_and_container_manager_are_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<Executor>();
        assert_clone::<ContainerManager>();
    }
}
