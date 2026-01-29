use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub id: i32,
    pub name: String,
    pub enabled: bool,
    pub default_port: Option<i32>,
    pub priority: i32,
    pub flag_regex: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateChallenge {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_port: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag_regex: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: i32,
    pub team_id: String,
    pub team_name: String,
    pub default_ip: Option<String>,
    pub priority: i32,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateTeam {
    pub team_id: String,
    pub team_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateExploit {
    pub name: String,
    pub challenge_id: i32,
    pub docker_image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entrypoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_per_container: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_containers: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_counter: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_add: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_into_rounds: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateExploit {
    pub name: String,
    pub docker_image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entrypoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_per_container: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_containers: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_counter: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateExploitRun {
    pub exploit_id: i32,
    pub challenge_id: i32,
    pub team_id: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateExploitRun {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sequence: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Round {
    pub id: i32,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub duration_ms: Option<i32>,
    pub schedule_at: Option<DateTime<Utc>>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChallengeTeamRelation {
    pub id: i32,
    pub challenge_id: i32,
    pub team_id: i32,
    pub addr: Option<String>,
    pub port: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateRelation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReorderJobItem {
    pub id: i32,
    pub priority: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnqueueSingleJobRequest {
    pub exploit_run_id: i32,
    pub team_id: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateSetting {
    pub key: String,
    pub value: String,
}
