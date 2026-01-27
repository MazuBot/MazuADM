use crate::{Database, Exploit, ExploitJob, ExploitRun, ConnectionInfo};
use anyhow::Result;
use bollard::Docker;
use bollard::container::{Config, CreateContainerOptions, StartContainerOptions, WaitContainerOptions, LogsOptions};
use bollard::models::HostConfig;
use futures::StreamExt;
use std::time::Instant;
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

    pub async fn execute_job(&self, job: &ExploitJob, exploit: &Exploit, conn: &ConnectionInfo, flag_regex: Option<&str>) -> Result<JobResult> {
        let start = Instant::now();
        self.db.update_job_status(job.id, "running").await?;

        let container_name = format!("mazuadm-{}-{}", job.round_id, job.id);
        let team = self.db.get_team(job.team_id).await?;
        let config = Config {
            image: Some(exploit.docker_image.clone()),
            entrypoint: exploit.entrypoint.as_ref().map(|e| vec![e.clone()]),
            cmd: Some(vec![conn.addr.clone(), conn.port.to_string(), team.team_id.clone()]),
            env: Some(vec![
                format!("TARGET_HOST={}", conn.addr),
                format!("TARGET_PORT={}", conn.port),
                format!("TARGET_TEAM_ID={}", team.team_id),
            ]),
            host_config: Some(HostConfig {
                auto_remove: Some(true),
                network_mode: Some("host".to_string()),
                ..Default::default()
            }),
            ..Default::default()
        };

        let id = self.docker.create_container(Some(CreateContainerOptions { name: container_name, platform: None }), config).await?.id;
        self.docker.start_container(&id, None::<StartContainerOptions<String>>).await?;

        let mut wait = self.docker.wait_container(&id, None::<WaitContainerOptions<String>>);
        let exit_code = if let Some(Ok(res)) = wait.next().await { res.status_code } else { -1 };

        let mut stdout = String::new();
        let mut stderr = String::new();
        let mut logs = self.docker.logs(&id, Some(LogsOptions::<String> { stdout: true, stderr: true, ..Default::default() }));
        while let Some(Ok(log)) = logs.next().await {
            match log {
                bollard::container::LogOutput::StdOut { message } => stdout.push_str(&String::from_utf8_lossy(&message)),
                bollard::container::LogOutput::StdErr { message } => stderr.push_str(&String::from_utf8_lossy(&message)),
                _ => {}
            }
        }

        let duration_ms = start.elapsed().as_millis() as i32;
        let status = if exit_code == 0 { "success" } else { "failed" };
        
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
        let jobs = self.db.get_pending_jobs(round_id).await?;
        
        for job in jobs {
            let run: ExploitRun = sqlx::query_as("SELECT * FROM exploit_runs WHERE id = $1")
                .bind(job.exploit_run_id).fetch_one(&self.db.pool).await?;
            let exploit = self.db.get_exploit(run.exploit_id).await?;
            let challenge = self.db.get_challenge(run.challenge_id).await?;
            let team = self.db.get_team(job.team_id).await?;
            
            let relations = self.db.list_relations(challenge.id).await?;
            let rel = relations.iter().find(|r| r.team_id == team.id);
            
            if let Some(rel) = rel {
                if let Some(conn) = rel.connection_info(&challenge, &team) {
                    match self.execute_job(&job, &exploit, &conn, challenge.flag_regex.as_deref()).await {
                        Ok(result) => {
                            for flag in result.flags {
                                let _ = self.db.create_flag(job.id, round_id, challenge.id, team.id, &flag).await;
                            }
                        }
                        Err(e) => {
                            tracing::error!("Job {} failed: {}", job.id, e);
                            let _ = self.db.finish_job(job.id, "error", None, Some(&e.to_string()), 0).await;
                        }
                    }
                }
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_flags_default_pattern() {
        let output = "Got flag: ABCDEFGHIJKLMNOPQRSTUVWXYZabcde=";
        let flags = Executor::extract_flags(output, None);
        assert_eq!(flags, vec!["ABCDEFGHIJKLMNOPQRSTUVWXYZabcde="]);
    }

    #[test]
    fn test_extract_flags_multiple() {
        let output = "flag1: AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA= flag2: BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=";
        let flags = Executor::extract_flags(output, None);
        assert_eq!(flags.len(), 2);
    }

    #[test]
    fn test_extract_flags_custom_pattern() {
        let output = "FLAG{test123}";
        let flags = Executor::extract_flags(output, Some(r"FLAG\{[^}]+\}"));
        assert_eq!(flags, vec!["FLAG{test123}"]);
    }

    #[test]
    fn test_extract_flags_no_match() {
        let flags = Executor::extract_flags("no flags here", None);
        assert!(flags.is_empty());
    }

    #[test]
    fn test_extract_flags_invalid_regex() {
        let flags = Executor::extract_flags("test", Some("[invalid"));
        assert!(flags.is_empty());
    }
}
