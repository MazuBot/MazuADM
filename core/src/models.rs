use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Challenge {
    pub id: i32,
    pub name: String,
    pub enabled: bool,
    pub default_port: Option<i32>,
    pub priority: i32,
    pub flag_regex: Option<String>,
    pub created_at: DateTime<Utc>,
}


#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateChallenge {
    pub name: String,
    pub enabled: Option<bool>,
    pub default_port: Option<i32>,
    pub priority: Option<i32>,
    pub flag_regex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Team {
    pub id: i32,
    pub team_id: String,
    pub team_name: String,
    pub default_ip: Option<String>,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateTeam {
    pub team_id: String,
    pub team_name: String,
    pub default_ip: Option<String>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct ChallengeTeamRelation {
    pub id: i32,
    pub challenge_id: i32,
    pub team_id: i32,
    pub addr: Option<String>,
    pub port: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Exploit {
    pub id: i32,
    pub name: String,
    pub challenge_id: i32,
    pub enabled: bool,
    pub max_per_container: i32,
    pub max_containers: i32,
    pub docker_image: String,
    pub entrypoint: Option<String>,
    pub timeout_secs: i32,
    pub default_counter: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateExploit {
    pub name: String,
    pub challenge_id: i32,
    pub docker_image: String,
    pub entrypoint: Option<String>,
    pub enabled: Option<bool>,
    pub max_per_container: Option<i32>,
    pub max_containers: Option<i32>,
    pub timeout_secs: Option<i32>,
    pub default_counter: Option<i32>,
    pub auto_add: Option<String>,
    pub insert_into_rounds: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateExploit {
    pub name: String,
    pub docker_image: String,
    pub entrypoint: Option<String>,
    pub enabled: Option<bool>,
    pub max_per_container: Option<i32>,
    pub max_containers: Option<i32>,
    pub timeout_secs: Option<i32>,
    pub default_counter: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct ExploitRun {
    pub id: i32,
    pub exploit_id: i32,
    pub challenge_id: i32,
    pub team_id: i32,
    pub priority: Option<i32>,
    pub sequence: i32,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateExploitRun {
    pub exploit_id: i32,
    pub challenge_id: i32,
    pub team_id: i32,
    pub priority: Option<i32>,
    pub sequence: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Round {
    pub id: i32,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct ExploitJob {
    pub id: i32,
    pub round_id: i32,
    pub exploit_run_id: Option<i32>,
    pub team_id: i32,
    pub priority: i32,
    pub status: String,
    pub container_id: Option<String>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub create_reason: Option<String>,
    pub duration_ms: Option<i32>,
    pub schedule_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl ExploitJob {
    pub fn without_logs(&self) -> Self {
        let mut job = self.clone();
        job.stdout = None;
        job.stderr = None;
        job
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Flag {
    pub id: i32,
    pub job_id: Option<i32>,
    pub round_id: i32,
    pub challenge_id: i32,
    pub team_id: i32,
    pub flag_value: String,
    pub status: String,
    pub submitted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub addr: String,
    pub port: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, ToSchema)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContainerInfo {
    pub id: String,
    pub exploit_id: i32,
    pub status: String,
    pub counter: i32,
    pub running_execs: usize,
    pub max_execs: usize,
    pub created_at: DateTime<Utc>,
    pub affinity_runs: Vec<i32>,
}

impl ChallengeTeamRelation {
    pub fn connection_info(&self, challenge: &Challenge, team: &Team) -> Option<ConnectionInfo> {
        let addr = self.addr.clone().or_else(|| team.default_ip.clone())?;
        let port = self.port.or(challenge.default_port)?;
        Some(ConnectionInfo { addr, port })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_challenge(default_port: Option<i32>) -> Challenge {
        Challenge {
            id: 1,
            name: "test".into(),
            enabled: true,
            default_port,
            priority: 0,
            flag_regex: None,
            created_at: Utc::now(),
        }
    }

    fn make_team(default_ip: Option<&str>) -> Team {
        Team {
            id: 1,
            team_id: "t1".into(),
            team_name: "Team1".into(),
            default_ip: default_ip.map(String::from),
            priority: 0,
            created_at: Utc::now(),
            enabled: true,
        }
    }

    fn make_relation(addr: Option<&str>, port: Option<i32>) -> ChallengeTeamRelation {
        ChallengeTeamRelation { id: 1, challenge_id: 1, team_id: 1, addr: addr.map(String::from), port, created_at: Utc::now() }
    }

    #[test]
    fn exploit_job_without_logs_clears_stdout_stderr() {
        let now = Utc::now();
        let job = ExploitJob {
            id: 1,
            round_id: 2,
            exploit_run_id: Some(3),
            team_id: 4,
            priority: 5,
            status: "running".to_string(),
            container_id: Some("container".to_string()),
            stdout: Some("stdout".to_string()),
            stderr: Some("stderr".to_string()),
            create_reason: None,
            duration_ms: Some(123),
            schedule_at: None,
            started_at: Some(now),
            finished_at: None,
            created_at: now,
        };

        let trimmed = job.without_logs();
        assert!(trimmed.stdout.is_none());
        assert!(trimmed.stderr.is_none());
        assert_eq!(trimmed.id, job.id);
        assert_eq!(trimmed.status, job.status);
    }

    #[test]
    fn test_connection_info_from_relation() {
        let rel = make_relation(Some("10.0.0.1"), Some(8080));
        let conn = rel.connection_info(&make_challenge(None), &make_team(None)).unwrap();
        assert_eq!(conn.addr, "10.0.0.1");
        assert_eq!(conn.port, 8080);
    }

    #[test]
    fn test_connection_info_fallback_to_defaults() {
        let rel = make_relation(None, None);
        let conn = rel.connection_info(&make_challenge(Some(9000)), &make_team(Some("192.168.1.1"))).unwrap();
        assert_eq!(conn.addr, "192.168.1.1");
        assert_eq!(conn.port, 9000);
    }

    #[test]
    fn test_connection_info_missing_addr() {
        let rel = make_relation(None, Some(8080));
        assert!(rel.connection_info(&make_challenge(None), &make_team(None)).is_none());
    }

    #[test]
    fn test_connection_info_missing_port() {
        let rel = make_relation(Some("10.0.0.1"), None);
        assert!(rel.connection_info(&make_challenge(None), &make_team(None)).is_none());
    }
}
