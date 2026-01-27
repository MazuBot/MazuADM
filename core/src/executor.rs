use crate::{Database, Exploit, ExploitJob, ExploitRun, ConnectionInfo};
use crate::container_manager::ContainerManager;
use anyhow::Result;
use std::time::{Instant, Duration};
use std::sync::Arc;
use tokio::sync::Semaphore;
use regex::Regex;

pub struct Executor {
    pub db: Database,
    pub container_manager: ContainerManager,
}

impl Executor {
    pub fn new(db: Database) -> Result<Self> {
        let container_manager = ContainerManager::new(db.clone())?;
        Ok(Self { db, container_manager })
    }

    pub async fn execute_job(&self, job: &ExploitJob, run: &ExploitRun, exploit: &Exploit, conn: &ConnectionInfo, flag_regex: Option<&str>, timeout_secs: u64) -> Result<JobResult> {
        let start = Instant::now();
        self.db.update_job_status(job.id, "running", true).await?;

        let team = self.db.get_team(job.team_id).await?;
        
        // Get or assign persistent container
        let container = self.container_manager.get_or_assign_container(run).await?;

        // Build command - use entrypoint or default script
        let cmd = match &exploit.entrypoint {
            Some(ep) => vec![ep.clone(), conn.addr.clone(), conn.port.to_string(), team.team_id.clone()],
            None => vec!["/exploit".to_string(), conn.addr.clone(), conn.port.to_string(), team.team_id.clone()],
        };

        let env = vec![
            format!("TARGET_HOST={}", conn.addr),
            format!("TARGET_PORT={}", conn.port),
            format!("TARGET_TEAM_ID={}", team.team_id),
        ];

        // Execute with timeout
        let exec_result = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            self.container_manager.execute_in_container(&container, cmd, env)
        ).await;

        let (stdout, stderr, exit_code, timed_out) = match exec_result {
            Ok(Ok(r)) => (r.stdout, r.stderr, r.exit_code, false),
            Ok(Err(e)) => (String::new(), e.to_string(), -1, false),
            Err(_) => (String::new(), "Timeout".to_string(), -1, true),
        };

        // Decrement counter and destroy if exhausted
        let new_counter = self.db.decrement_container_counter(container.id).await?;
        if new_counter <= 0 {
            let _ = self.container_manager.destroy_container(container.id).await;
        }

        let duration_ms = start.elapsed().as_millis() as i32;
        let status = if timed_out { "timeout" } else if exit_code == 0 { "success" } else { "failed" };
        
        let flags = Self::extract_flags(&stdout, flag_regex);
        
        self.db.finish_job(job.id, status, Some(&stdout), Some(&stderr), duration_ms).await?;

        Ok(JobResult { stdout, stderr, duration_ms, exit_code, flags })
    }

    pub fn extract_flags(output: &str, pattern: Option<&str>) -> Vec<String> {
        let pattern = pattern.unwrap_or(r"[A-Za-z0-9]{31}=");
        let Ok(re) = Regex::new(pattern) else { return vec![] };
        re.find_iter(output).map(|m| m.as_str().to_string()).collect()
    }

    pub async fn run_round(&self, round_id: i32) -> Result<()> {
        // Health check containers before round
        self.container_manager.health_check().await?;

        let concurrent_limit: usize = self.db.get_setting("concurrent_limit").await
            .ok().and_then(|v| v.parse().ok()).unwrap_or(10);
        let worker_timeout: u64 = self.db.get_setting("worker_timeout").await
            .ok().and_then(|v| v.parse().ok()).unwrap_or(60);

        let jobs = self.db.get_pending_jobs(round_id).await?;
        let semaphore = Arc::new(Semaphore::new(concurrent_limit));
        let mut handles = Vec::new();

        for job in jobs {
            let permit = semaphore.clone().acquire_owned().await?;
            let db = self.db.clone();
            let executor = Executor::new(db.clone())?;

            let handle = tokio::spawn(async move {
                let _permit = permit;
                
                let run: ExploitRun = match sqlx::query_as("SELECT * FROM exploit_runs WHERE id = $1")
                    .bind(job.exploit_run_id).fetch_one(&db.pool).await {
                    Ok(r) => r,
                    Err(e) => { tracing::error!("Job {} failed: {}", job.id, e); return; }
                };
                
                let exploit = match db.get_exploit(run.exploit_id).await {
                    Ok(e) => e,
                    Err(e) => { tracing::error!("Job {} failed: {}", job.id, e); return; }
                };
                
                let challenge = match db.get_challenge(run.challenge_id).await {
                    Ok(c) => c,
                    Err(e) => { tracing::error!("Job {} failed: {}", job.id, e); return; }
                };
                
                let team = match db.get_team(job.team_id).await {
                    Ok(t) => t,
                    Err(e) => { tracing::error!("Job {} failed: {}", job.id, e); return; }
                };

                if !team.enabled {
                    let _ = db.finish_job(job.id, "skipped", None, Some("Team disabled"), 0).await;
                    return;
                }

                let relations = match db.list_relations(challenge.id).await {
                    Ok(r) => r,
                    Err(e) => { tracing::error!("Job {} failed: {}", job.id, e); return; }
                };
                
                let rel = relations.iter().find(|r| r.team_id == team.id);
                
                if let Some(rel) = rel {
                    if let Some(conn) = rel.connection_info(&challenge, &team) {
                        match executor.execute_job(&job, &run, &exploit, &conn, challenge.flag_regex.as_deref(), worker_timeout).await {
                            Ok(result) => {
                                for flag in result.flags {
                                    let _ = db.create_flag(job.id, round_id, challenge.id, team.id, &flag).await;
                                }
                            }
                            Err(e) => {
                                tracing::error!("Job {} failed: {}", job.id, e);
                                let _ = db.finish_job(job.id, "error", None, Some(&e.to_string()), 0).await;
                            }
                        }
                    }
                }
            });
            
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }
        
        self.db.finish_round(round_id).await?;
        Ok(())
    }
}

pub struct JobResult {
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: i32,
    pub exit_code: i64,
    pub flags: Vec<String>,
}
