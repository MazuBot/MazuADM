use crate::{Database, Exploit, ExploitJob, ExploitRun, ConnectionInfo};
use anyhow::Result;
use bollard::Docker;
use bollard::query_parameters::{CreateContainerOptions, StartContainerOptions, WaitContainerOptions, LogsOptions, RemoveContainerOptions};
use bollard::secret::{ContainerCreateBody, HostConfig};
use futures::{StreamExt, TryStreamExt};
use std::time::{Instant, Duration};
use std::sync::Arc;
use tokio::sync::Semaphore;
use regex::Regex;

pub struct Executor {
    db: Database,
    docker: Docker,
}

impl Executor {
    pub fn new(db: Database) -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;
        Ok(Self { db, docker })
    }

    pub async fn execute_job(&self, job: &ExploitJob, exploit: &Exploit, conn: &ConnectionInfo, flag_regex: Option<&str>, timeout_secs: u64) -> Result<JobResult> {
        let start = Instant::now();
        self.db.update_job_status(job.id, "running", true).await?;

        let container_name = format!("mazuadm-{}-{}", job.round_id, job.id);
        let team = self.db.get_team(job.team_id).await?;
        let config = ContainerCreateBody {
            image: Some(exploit.docker_image.clone()),
            entrypoint: exploit.entrypoint.as_ref().map(|e| vec![e.clone()]),
            cmd: Some(vec![conn.addr.clone(), conn.port.to_string(), team.team_id.clone()]),
            env: Some(vec![
                format!("TARGET_HOST={}", conn.addr),
                format!("TARGET_PORT={}", conn.port),
                format!("TARGET_TEAM_ID={}", team.team_id),
            ]),
            host_config: Some(HostConfig {
                network_mode: Some("host".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let id = self.docker.create_container(Some(CreateContainerOptions { name: Some(container_name), platform: String::new() }), config).await?.id;
        self.docker.start_container(&id, None::<StartContainerOptions>).await?;

        // Wait with timeout
        let wait_result = tokio::time::timeout(
            Duration::from_secs(timeout_secs),
            async {
                let mut wait = self.docker.wait_container(&id, None::<WaitContainerOptions>);
                if let Some(Ok(res)) = wait.next().await { res.status_code } else { -1 }
            }
        ).await;

        let (exit_code, timed_out) = match wait_result {
            Ok(code) => (code, false),
            Err(_) => {
                // Timeout - kill container
                let _ = self.docker.stop_container(&id, None).await;
                (-1, true)
            }
        };

        let mut stdout = String::new();
        let mut stderr = String::new();
        if let Ok(logs) = self.docker.logs(&id, Some(LogsOptions { stdout: true, stderr: true, ..Default::default() })).try_collect::<Vec<_>>().await {
            for log in logs {
                match log {
                    bollard::container::LogOutput::StdOut { message } => stdout.push_str(&String::from_utf8_lossy(&message)),
                    bollard::container::LogOutput::StdErr { message } => stderr.push_str(&String::from_utf8_lossy(&message)),
                    _ => {}
                }
            }
        }

        // Cleanup container
        let _ = self.docker.remove_container(&id, Some(RemoveContainerOptions { force: true, ..Default::default() })).await;

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
        // Get settings
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
            let docker = Docker::connect_with_local_defaults()?;
            let executor = Executor { db: db.clone(), docker };

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

                // Skip disabled teams
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
                        match executor.execute_job(&job, &exploit, &conn, challenge.flag_regex.as_deref(), worker_timeout).await {
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

        // Wait for all jobs
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
