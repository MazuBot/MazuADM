use crate::models::*;
use crate::config::resolve_db_pool_settings;
use sqlx::{PgPool, postgres::PgPoolOptions};
use anyhow::Result;
use std::time::Duration;
use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Serialize, Deserialize};

const REDIS_FLAG_PREFIX: &str = "mazuadm:flag:";

#[derive(Clone, Serialize, Deserialize)]
pub struct CachedFlag {
    pub id: i32,
    pub job_id: Option<i32>,
    pub round_id: i32,
    pub challenge_id: i32,
    pub team_id: i32,
}

#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
    redis: Option<redis::aio::MultiplexedConnection>,
}

pub struct CleanupStaleScheduledReport {
    pub marked: usize,
    pub requeued: usize,
}

fn stale_schedule_error_suffix() -> &'static str {
    "stale schedule_at while pending/running"
}

fn cleanup_requeue_reason(job_id: i32) -> String {
    format!("cleanup_requeue:{}", job_id)
}

impl Database {
    pub async fn connect(url: &str, cfg: &crate::AppConfig) -> Result<Self> {
        let settings = resolve_db_pool_settings(cfg);
        let pool = PgPoolOptions::new()
            .max_connections(settings.max_connections)
            .min_connections(settings.min_connections)
            .acquire_timeout(Duration::from_secs(settings.acquire_timeout_secs))
            .idle_timeout(Duration::from_secs(settings.idle_timeout_secs))
            .max_lifetime(Duration::from_secs(settings.max_lifetime_secs))
            .connect(url)
            .await?;
        let redis = if let Some(redis_url) = &cfg.redis_url {
            let client = redis::Client::open(redis_url.as_str())?;
            Some(client.get_multiplexed_async_connection().await?)
        } else {
            None
        };
        Ok(Self { pool, redis })
    }

    pub async fn init_flag_cache(&self) -> Result<()> {
        if let Some(mut conn) = self.redis.clone() {
            let _: () = redis::cmd("FLUSHDB").query_async(&mut conn).await?;
            let flags: Vec<Flag> = sqlx::query_as("SELECT DISTINCT ON (flag_value) * FROM flags ORDER BY flag_value, id")
                .fetch_all(&self.pool).await?;
            for flag in &flags {
                let key = format!("{}{}", REDIS_FLAG_PREFIX, flag.flag_value);
                let cached = CachedFlag {
                    id: flag.id,
                    job_id: flag.job_id,
                    round_id: flag.round_id,
                    challenge_id: flag.challenge_id,
                    team_id: flag.team_id,
                };
                let _: () = conn.set(&key, serde_json::to_string(&cached)?).await?;
            }
            tracing::info!("Loaded {} unique flags into Redis cache", flags.len());
        }
        Ok(())
    }

    async fn is_flag_duplicate(&self, flag_value: &str) -> bool {
        if let Some(mut conn) = self.redis.clone() {
            let key = format!("{}{}", REDIS_FLAG_PREFIX, flag_value);
            conn.exists(&key).await.unwrap_or(false)
        } else {
            false
        }
    }

    async fn add_flag_to_cache(&self, flag: &Flag) {
        if let Some(mut conn) = self.redis.clone() {
            let key = format!("{}{}", REDIS_FLAG_PREFIX, flag.flag_value);
            let cached = CachedFlag {
                id: flag.id,
                job_id: flag.job_id,
                round_id: flag.round_id,
                challenge_id: flag.challenge_id,
                team_id: flag.team_id,
            };
            if let Ok(json) = serde_json::to_string(&cached) {
                let _: Result<(), _> = conn.set(&key, json).await;
            }
        }
    }

    fn clamp_priority(p: Option<i32>) -> i32 {
        p.unwrap_or(0).clamp(0, 99)
    }

    // Challenges
    pub async fn create_challenge(&self, c: CreateChallenge) -> Result<Challenge> {
        Ok(sqlx::query_as!(Challenge,
            "INSERT INTO challenges (name, enabled, default_port, priority, flag_regex) VALUES ($1, $2, $3, $4, $5) RETURNING *",
            c.name, c.enabled.unwrap_or(true), c.default_port, Self::clamp_priority(c.priority), c.flag_regex
        ).fetch_one(&self.pool).await?)
    }

    pub async fn list_challenges(&self) -> Result<Vec<Challenge>> {
        Ok(sqlx::query_as!(Challenge, "SELECT * FROM challenges ORDER BY priority DESC, id").fetch_all(&self.pool).await?)
    }

    pub async fn get_challenge(&self, id: i32) -> Result<Challenge> {
        Ok(sqlx::query_as!(Challenge, "SELECT * FROM challenges WHERE id = $1", id).fetch_one(&self.pool).await?)
    }

    pub async fn set_challenge_enabled(&self, id: i32, enabled: bool) -> Result<()> {
        sqlx::query!("UPDATE challenges SET enabled = $2 WHERE id = $1", id, enabled).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn update_challenge(&self, id: i32, c: CreateChallenge) -> Result<Challenge> {
        let priority = c.priority.map(|p| p.clamp(0, 99));
        Ok(sqlx::query_as!(Challenge,
            "UPDATE challenges SET name = $2, enabled = COALESCE($3, enabled), default_port = $4, priority = COALESCE($5, priority), flag_regex = $6 WHERE id = $1 RETURNING *",
            id, c.name, c.enabled, c.default_port, priority, c.flag_regex
        ).fetch_one(&self.pool).await?)
    }

    pub async fn delete_challenge(&self, id: i32) -> Result<()> {
        sqlx::query!("DELETE FROM challenges WHERE id = $1", id).execute(&self.pool).await?;
        Ok(())
    }

    // Teams
    pub async fn create_team(&self, t: CreateTeam) -> Result<Team> {
        Ok(sqlx::query_as!(Team,
            "INSERT INTO teams (team_id, team_name, default_ip, priority, enabled) VALUES ($1, $2, $3, $4, $5) RETURNING *",
            t.team_id, t.team_name, t.default_ip, Self::clamp_priority(t.priority), t.enabled.unwrap_or(true)
        ).fetch_one(&self.pool).await?)
    }

    pub async fn list_teams(&self) -> Result<Vec<Team>> {
        Ok(sqlx::query_as!(Team, "SELECT * FROM teams ORDER BY priority DESC, id").fetch_all(&self.pool).await?)
    }

    pub async fn get_team(&self, id: i32) -> Result<Team> {
        Ok(sqlx::query_as!(Team, "SELECT * FROM teams WHERE id = $1", id).fetch_one(&self.pool).await?)
    }

    pub async fn update_team(&self, id: i32, t: CreateTeam) -> Result<Team> {
        let priority = t.priority.map(|p| p.clamp(0, 99));
        Ok(sqlx::query_as!(Team,
            "UPDATE teams SET team_id = $2, team_name = $3, default_ip = $4, priority = COALESCE($5, priority), enabled = COALESCE($6, enabled) WHERE id = $1 RETURNING *",
            id, t.team_id, t.team_name, t.default_ip, priority, t.enabled
        ).fetch_one(&self.pool).await?)
    }

    pub async fn delete_team(&self, id: i32) -> Result<()> {
        sqlx::query!("DELETE FROM teams WHERE id = $1", id).execute(&self.pool).await?;
        Ok(())
    }

    // Relations
    pub async fn create_relation(&self, challenge_id: i32, team_id: i32, addr: Option<String>, port: Option<i32>) -> Result<ChallengeTeamRelation> {
        Ok(sqlx::query_as!(ChallengeTeamRelation,
            "INSERT INTO challenge_team_relations (challenge_id, team_id, addr, port) VALUES ($1, $2, $3, $4) RETURNING *",
            challenge_id, team_id, addr, port
        ).fetch_one(&self.pool).await?)
    }

    pub async fn list_relations(&self, challenge_id: i32) -> Result<Vec<ChallengeTeamRelation>> {
        Ok(sqlx::query_as!(ChallengeTeamRelation,
            "SELECT * FROM challenge_team_relations WHERE challenge_id = $1", challenge_id
        ).fetch_all(&self.pool).await?)
    }

    pub async fn get_relation(&self, challenge_id: i32, team_id: i32) -> Result<Option<ChallengeTeamRelation>> {
        Ok(sqlx::query_as!(ChallengeTeamRelation,
            "SELECT * FROM challenge_team_relations WHERE challenge_id = $1 AND team_id = $2", challenge_id, team_id
        ).fetch_optional(&self.pool).await?)
    }

    pub async fn update_connection_info(&self, challenge_id: i32, team_id: i32, addr: Option<String>, port: Option<i32>, enabled: Option<bool>) -> Result<ChallengeTeamRelation> {
        Ok(sqlx::query_as!(ChallengeTeamRelation,
            "INSERT INTO challenge_team_relations (challenge_id, team_id, addr, port, enabled) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (challenge_id, team_id) DO UPDATE SET addr = COALESCE($3, challenge_team_relations.addr), port = COALESCE($4, challenge_team_relations.port), enabled = COALESCE($5, challenge_team_relations.enabled) RETURNING *",
            challenge_id, team_id, addr, port, enabled
        ).fetch_one(&self.pool).await?)
    }

    pub async fn ensure_relations(&self, challenge_id: i32) -> Result<()> {
        sqlx::query!("INSERT INTO challenge_team_relations (challenge_id, team_id) SELECT $1, id FROM teams ON CONFLICT DO NOTHING", challenge_id)
            .execute(&self.pool).await?;
        Ok(())
    }

    // Exploits
    pub async fn create_exploit(&self, e: CreateExploit) -> Result<Exploit> {
        let exploit = sqlx::query_as!(
            Exploit,
            "INSERT INTO exploits (name, challenge_id, docker_image, entrypoint, enabled, max_per_container, max_containers, max_concurrent_jobs, timeout_secs, default_counter, ignore_connection_info, envs) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) RETURNING *",
            e.name,
            e.challenge_id,
            e.docker_image,
            e.entrypoint,
            e.enabled.unwrap_or(true),
            e.max_per_container.unwrap_or(1),
            e.max_containers.unwrap_or(0),
            e.max_concurrent_jobs.unwrap_or(0),
            e.timeout_secs.unwrap_or(30),
            e.default_counter.unwrap_or(999),
            e.ignore_connection_info.unwrap_or(false),
            e.envs,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(exploit)
    }

    pub async fn list_exploits(&self, challenge_id: Option<i32>) -> Result<Vec<Exploit>> {
        match challenge_id {
            Some(cid) => Ok(sqlx::query_as!(
                Exploit,
                "SELECT * FROM exploits WHERE challenge_id = $1 ORDER BY id DESC",
                cid
            )
            .fetch_all(&self.pool)
            .await?),
            None => Ok(sqlx::query_as!(Exploit, "SELECT * FROM exploits ORDER BY id DESC")
                .fetch_all(&self.pool)
                .await?),
        }
    }

    pub async fn get_exploit(&self, id: i32) -> Result<Exploit> {
        Ok(sqlx::query_as!(Exploit, "SELECT * FROM exploits WHERE id = $1", id)
            .fetch_one(&self.pool)
            .await?)
    }

    pub async fn list_enabled_exploits(&self) -> Result<Vec<Exploit>> {
        Ok(sqlx::query_as!(
            Exploit,
            "SELECT * FROM exploits WHERE enabled = true ORDER BY id DESC"
        )
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn update_exploit(&self, id: i32, e: UpdateExploit) -> Result<Exploit> {
        let exploit = sqlx::query_as!(
            Exploit,
            "UPDATE exploits SET name = $2, docker_image = $3, entrypoint = $4, enabled = COALESCE($5, enabled), max_per_container = COALESCE($6, max_per_container), max_containers = COALESCE($7, max_containers), max_concurrent_jobs = COALESCE($8, max_concurrent_jobs), timeout_secs = COALESCE($9, timeout_secs), default_counter = COALESCE($10, default_counter), ignore_connection_info = COALESCE($11, ignore_connection_info), envs = $12 WHERE id = $1 RETURNING *",
            id,
            e.name,
            e.docker_image,
            e.entrypoint,
            e.enabled,
            e.max_per_container,
            e.max_containers,
            e.max_concurrent_jobs,
            e.timeout_secs,
            e.default_counter,
            e.ignore_connection_info,
            e.envs
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(exploit)
    }

    pub async fn delete_exploit(&self, id: i32) -> Result<()> {
        sqlx::query!("DELETE FROM exploits WHERE id = $1", id).execute(&self.pool).await?;
        Ok(())
    }

    // Exploit Runs
    pub async fn create_exploit_run(&self, r: CreateExploitRun) -> Result<ExploitRun> {
        Ok(sqlx::query_as!(ExploitRun,
            "INSERT INTO exploit_runs (exploit_id, challenge_id, team_id, priority, sequence) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (exploit_id, challenge_id, team_id) DO UPDATE SET priority = EXCLUDED.priority, sequence = EXCLUDED.sequence RETURNING *",
            r.exploit_id, r.challenge_id, r.team_id, r.priority, r.sequence.unwrap_or(0)
        ).fetch_one(&self.pool).await?)
    }

    pub async fn list_exploit_runs(&self, challenge_id: Option<i32>, team_id: Option<i32>) -> Result<Vec<ExploitRun>> {
        match (challenge_id, team_id) {
            (Some(c), Some(t)) => Ok(sqlx::query_as!(ExploitRun, "SELECT * FROM exploit_runs WHERE challenge_id = $1 AND team_id = $2 ORDER BY sequence", c, t).fetch_all(&self.pool).await?),
            (Some(c), None) => Ok(sqlx::query_as!(ExploitRun, "SELECT * FROM exploit_runs WHERE challenge_id = $1 ORDER BY sequence", c).fetch_all(&self.pool).await?),
            (None, Some(t)) => Ok(sqlx::query_as!(ExploitRun, "SELECT * FROM exploit_runs WHERE team_id = $1 ORDER BY sequence", t).fetch_all(&self.pool).await?),
            (None, None) => Ok(sqlx::query_as!(ExploitRun, "SELECT * FROM exploit_runs ORDER BY sequence").fetch_all(&self.pool).await?),
        }
    }

    pub async fn get_exploit_runs_for_exploit(&self, exploit_id: i32) -> Result<Vec<ExploitRun>> {
        Ok(sqlx::query_as!(ExploitRun, 
            "SELECT er.* FROM exploit_runs er JOIN challenges c ON er.challenge_id = c.id WHERE er.exploit_id = $1 AND er.enabled = true AND c.enabled = true", 
            exploit_id
        ).fetch_all(&self.pool).await?)
    }

    pub async fn update_exploit_run(&self, id: i32, priority: Option<i32>, sequence: Option<i32>, enabled: Option<bool>) -> Result<ExploitRun> {
        Ok(sqlx::query_as!(ExploitRun,
            "UPDATE exploit_runs SET priority = $2, sequence = COALESCE($3, sequence), enabled = COALESCE($4, enabled) WHERE id = $1 RETURNING *",
            id, priority, sequence, enabled
        ).fetch_one(&self.pool).await?)
    }

    pub async fn delete_exploit_run(&self, id: i32) -> Result<()> {
        sqlx::query!("DELETE FROM exploit_runs WHERE id = $1", id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn reorder_exploit_runs(&self, items: &[(i32, i32)]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let (ids, seqs): (Vec<i32>, Vec<i32>) = items.iter().copied().unzip();
        sqlx::query!(
            "UPDATE exploit_runs er SET sequence = v.sequence FROM UNNEST($1::int[], $2::int[]) AS v(id, sequence) WHERE er.id = v.id",
            &ids,
            &seqs
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_exploit_run(&self, id: i32) -> Result<ExploitRun> {
        Ok(sqlx::query_as!(ExploitRun, "SELECT * FROM exploit_runs WHERE id = $1", id).fetch_one(&self.pool).await?)
    }

    // Rounds
    pub async fn create_round(&self) -> Result<Round> {
        Ok(sqlx::query_as!(Round, "INSERT INTO rounds DEFAULT VALUES RETURNING *").fetch_one(&self.pool).await?)
    }

    pub async fn get_latest_round_id(&self) -> Result<Option<i32>> {
        let row = sqlx::query!("SELECT id FROM rounds ORDER BY id DESC LIMIT 1")
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.id))
    }

    pub async fn get_latest_pending_round_id(&self) -> Result<Option<i32>> {
        let row = sqlx::query!(
            "SELECT id FROM rounds WHERE status = 'pending' ORDER BY id DESC LIMIT 1"
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| r.id))
    }

    pub async fn get_round(&self, id: i32) -> Result<Round> {
        Ok(sqlx::query_as!(Round, "SELECT * FROM rounds WHERE id = $1", id).fetch_one(&self.pool).await?)
    }

    pub async fn finish_round(&self, id: i32) -> Result<()> {
        sqlx::query!("UPDATE rounds SET finished_at = NOW(), status = 'finished' WHERE id = $1", id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn skip_round(&self, id: i32) -> Result<()> {
        sqlx::query!("UPDATE rounds SET finished_at = NOW(), status = 'skipped' WHERE id = $1", id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn reset_round(&self, id: i32) -> Result<()> {
        sqlx::query!("UPDATE rounds SET finished_at = NULL, status = 'pending' WHERE id = $1", id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn reset_jobs_for_round(&self, round_id: i32) -> Result<u64> {
        let result = sqlx::query!("UPDATE exploit_jobs SET status = 'pending', schedule_at = NULL, started_at = NULL, finished_at = NULL, stdout = NULL, stderr = NULL, duration_ms = NULL WHERE round_id = $1", round_id)
            .execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    pub async fn reset_unflagged_jobs_for_round(&self, round_id: i32) -> Result<u64> {
        let result = sqlx::query!(
            "UPDATE exploit_jobs SET status = 'pending', schedule_at = NULL, started_at = NULL, finished_at = NULL, stdout = NULL, stderr = NULL, duration_ms = NULL WHERE round_id = $1 AND status != 'flag'",
            round_id
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
    }

    pub async fn clone_unflagged_jobs_for_round(&self, round_id: i32) -> Result<Vec<ExploitJob>> {
        let jobs = sqlx::query_as!(
            ExploitJob,
            "INSERT INTO exploit_jobs (round_id, exploit_run_id, team_id, priority, create_reason)
             SELECT $1, ej.exploit_run_id, ej.team_id, ej.priority, 'rerun_unflag:' || ej.id::text
             FROM exploit_jobs ej
             JOIN exploit_runs er ON er.id = ej.exploit_run_id
             WHERE ej.round_id = $1
               AND ej.status NOT IN ('flag', 'skipped', 'pending')
               AND ej.schedule_at IS NOT NULL
               AND ej.exploit_run_id IS NOT NULL
               AND NOT EXISTS (
                 SELECT 1 FROM flags f
                 WHERE f.round_id = $1
                   AND f.challenge_id = er.challenge_id
                   AND f.team_id = ej.team_id
                 LIMIT 1
               )
             RETURNING *",
            round_id
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(jobs)
    }

    pub async fn kill_running_jobs(&self) -> Result<Vec<ExploitJob>> {
        let jobs = sqlx::query_as!(ExploitJob, "SELECT * FROM exploit_jobs WHERE status = 'running'")
            .fetch_all(&self.pool).await?;
        Ok(jobs)
    }

    pub async fn get_running_jobs_by_container(&self, container_id: &str) -> Result<Vec<ExploitJob>> {
        Ok(sqlx::query_as!(ExploitJob,
            "SELECT * FROM exploit_jobs WHERE container_id = $1 AND status = 'running'",
            container_id
        ).fetch_all(&self.pool).await?)
    }

    // Exploit containers
    pub async fn list_exploit_containers(&self) -> Result<Vec<ExploitContainer>> {
        Ok(sqlx::query_as::<_, ExploitContainer>("SELECT * FROM exploit_containers ORDER BY id")
            .fetch_all(&self.pool)
            .await?)
    }

    pub async fn get_exploit_container_by_container_id(&self, container_id: &str) -> Result<Option<ExploitContainer>> {
        Ok(sqlx::query_as::<_, ExploitContainer>(
            "SELECT * FROM exploit_containers WHERE container_id = $1",
        )
        .bind(container_id)
        .fetch_optional(&self.pool)
        .await?)
    }

    pub async fn create_exploit_container(
        &self,
        exploit_id: i32,
        container_id: &str,
        counter: i32,
        status: &str,
        created_at: DateTime<Utc>,
    ) -> Result<ExploitContainer> {
        Ok(sqlx::query_as::<_, ExploitContainer>(
            "INSERT INTO exploit_containers (exploit_id, container_id, counter, status, created_at) VALUES ($1, $2, $3, $4, $5) RETURNING *",
        )
        .bind(exploit_id)
        .bind(container_id)
        .bind(counter)
        .bind(status)
        .bind(created_at)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn update_exploit_container_metadata(&self, id: i32, exploit_id: i32, status: &str) -> Result<()> {
        sqlx::query("UPDATE exploit_containers SET exploit_id = $2, status = $3 WHERE id = $1")
            .bind(id)
            .bind(exploit_id)
            .bind(status)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_exploit_container_counter(&self, id: i32, counter: i32) -> Result<()> {
        sqlx::query("UPDATE exploit_containers SET counter = $2 WHERE id = $1")
            .bind(id)
            .bind(counter)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_exploit_container_by_container_id(&self, container_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM exploit_containers WHERE container_id = $1")
            .bind(container_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // Exploit runners (affinity)
    pub async fn list_exploit_runners(&self) -> Result<Vec<ExploitRunner>> {
        Ok(sqlx::query_as::<_, ExploitRunner>("SELECT * FROM exploit_runners ORDER BY id")
            .fetch_all(&self.pool)
            .await?)
    }

    pub async fn list_exploit_runners_by_container(&self, exploit_container_id: i32) -> Result<Vec<ExploitRunner>> {
        Ok(sqlx::query_as::<_, ExploitRunner>(
            "SELECT * FROM exploit_runners WHERE exploit_container_id = $1 ORDER BY id",
        )
        .bind(exploit_container_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn list_exploit_runners_by_exploit(&self, exploit_id: i32) -> Result<Vec<ExploitRunner>> {
        Ok(sqlx::query_as::<_, ExploitRunner>(
            "SELECT * FROM exploit_runners WHERE exploit_id = $1 ORDER BY id",
        )
        .bind(exploit_id)
        .fetch_all(&self.pool)
        .await?)
    }

    pub async fn upsert_exploit_runner(&self, exploit_container_id: i32, exploit_run_id: i32, team_id: i32, exploit_id: i32) -> Result<ExploitRunner> {
        sqlx::query(
            "DELETE FROM exploit_runners WHERE exploit_run_id = $1 OR (team_id = $2 AND exploit_id = $3)",
        )
        .bind(exploit_run_id)
        .bind(team_id)
        .bind(exploit_id)
        .execute(&self.pool)
        .await?;
        Ok(sqlx::query_as::<_, ExploitRunner>(
            "INSERT INTO exploit_runners (exploit_container_id, exploit_run_id, team_id, exploit_id) VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(exploit_container_id)
        .bind(exploit_run_id)
        .bind(team_id)
        .bind(exploit_id)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn delete_exploit_runner_by_run(&self, exploit_run_id: i32) -> Result<()> {
        sqlx::query("DELETE FROM exploit_runners WHERE exploit_run_id = $1")
            .bind(exploit_run_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_exploit_runners_by_container(&self, exploit_container_id: i32) -> Result<()> {
        sqlx::query("DELETE FROM exploit_runners WHERE exploit_container_id = $1")
            .bind(exploit_container_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn clear_exploit_runners(&self) -> Result<()> {
        sqlx::query("DELETE FROM exploit_runners")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn mark_job_stopped_with_reason(&self, id: i32, has_flag: bool, reason: &str) -> Result<()> {
        let status = if has_flag { "flag" } else { "stopped" };
        sqlx::query!(
            "UPDATE exploit_jobs SET status = $2, stderr = COALESCE(stderr, '') || E'\\n[' || $3 || ']' WHERE id = $1",
            id,
            status,
            reason
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn start_round(&self, id: i32) -> Result<()> {
        sqlx::query!("UPDATE rounds SET status = 'running' WHERE id = $1", id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn list_rounds(&self) -> Result<Vec<Round>> {
        Ok(sqlx::query_as!(Round, "SELECT * FROM rounds ORDER BY id DESC").fetch_all(&self.pool).await?)
    }

    pub async fn get_active_rounds(&self) -> Result<Vec<Round>> {
        Ok(sqlx::query_as!(Round, "SELECT * FROM rounds WHERE status IN ('pending', 'running') ORDER BY id").fetch_all(&self.pool).await?)
    }

    // Jobs
    pub async fn create_job(&self, round_id: i32, exploit_run_id: i32, team_id: i32, priority: i32, create_reason: Option<&str>, injected_envs: Option<&str>) -> Result<ExploitJob> {
        // Merge injected_envs with exploit.envs
        let merged_envs = if injected_envs.is_some() {
            let run = self.get_exploit_run(exploit_run_id).await?;
            let exploit = self.get_exploit(run.exploit_id).await?;
            let mut merged: std::collections::HashMap<String, String> = std::collections::HashMap::new();
            if let Some(envs_json) = injected_envs {
                if let Ok(envs_map) = serde_json::from_str::<std::collections::HashMap<String, String>>(envs_json) {
                    merged.extend(envs_map);
                }
            }
            if let Some(ref envs_json) = exploit.envs {
                if let Ok(envs_map) = serde_json::from_str::<std::collections::HashMap<String, String>>(envs_json) {
                    merged.extend(envs_map);
                }
            }
            if merged.is_empty() { None } else { Some(serde_json::to_string(&merged).unwrap()) }
        } else {
            None
        };
        Ok(sqlx::query_as!(ExploitJob,
            "INSERT INTO exploit_jobs (round_id, exploit_run_id, team_id, priority, create_reason, envs) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
            round_id, exploit_run_id, team_id, priority, create_reason, merged_envs
        ).fetch_one(&self.pool).await?)
    }

    pub async fn list_jobs(&self, round_id: i32) -> Result<Vec<ExploitJob>> {
        Ok(sqlx::query_as!(ExploitJob,
            "SELECT id, round_id, exploit_run_id, team_id, priority, status, container_id, NULL::TEXT AS stdout, NULL::TEXT AS stderr, create_reason, envs, duration_ms, schedule_at, started_at, finished_at, created_at FROM exploit_jobs WHERE round_id = $1 ORDER BY priority DESC",
            round_id
        ).fetch_all(&self.pool).await?)
    }

    pub async fn get_job(&self, id: i32) -> Result<ExploitJob> {
        Ok(sqlx::query_as!(ExploitJob, "SELECT * FROM exploit_jobs WHERE id = $1", id).fetch_one(&self.pool).await?)
    }

    pub async fn get_pending_jobs(&self, round_id: i32) -> Result<Vec<ExploitJob>> {
        Ok(sqlx::query_as!(ExploitJob,
            "SELECT * FROM exploit_jobs WHERE round_id = $1 AND status = 'pending' ORDER BY priority DESC, id", round_id
        ).fetch_all(&self.pool).await?)
    }

    pub async fn get_max_priority_for_round(&self, round_id: i32) -> Result<i32> {
        let row = sqlx::query_scalar!("SELECT MAX(priority) FROM exploit_jobs WHERE round_id = $1", round_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.unwrap_or(0))
    }

    pub async fn mark_job_running(&self, id: i32) -> Result<()> {
        sqlx::query!(
            "UPDATE exploit_jobs SET status = 'running', started_at = NOW() WHERE id = $1",
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_job_scheduled(&self, id: i32) -> Result<()> {
        sqlx::query!("UPDATE exploit_jobs SET schedule_at = NOW() WHERE id = $1", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn set_job_container(&self, id: i32, container_id: &str) -> Result<()> {
        sqlx::query!("UPDATE exploit_jobs SET container_id = $2 WHERE id = $1", id, container_id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn finish_job(&self, id: i32, status: &str, stdout: Option<&str>, stderr: Option<&str>, duration_ms: i32) -> Result<()> {
        sqlx::query!("UPDATE exploit_jobs SET status = $2, stdout = $3, stderr = $4, duration_ms = $5, finished_at = NOW() WHERE id = $1",
            id, status, stdout, stderr, duration_ms
        ).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn reorder_jobs(&self, items: &[(i32, i32)]) -> Result<()> {
        if items.is_empty() {
            return Ok(());
        }
        let (ids, priorities): (Vec<i32>, Vec<i32>) = items.iter().copied().unzip();
        sqlx::query!(
            "UPDATE exploit_jobs ej SET priority = v.priority FROM UNNEST($1::int[], $2::int[]) AS v(id, priority) WHERE ej.id = v.id AND ej.status = 'pending'",
            &ids,
            &priorities
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_job_priority(&self, id: i32, priority: i32) -> Result<()> {
        sqlx::query!("UPDATE exploit_jobs SET priority = $2 WHERE id = $1", id, priority).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn skip_pending_jobs_for_round(&self, round_id: i32) -> Result<u64> {
        let result = sqlx::query!("UPDATE exploit_jobs SET status = 'skipped', stderr = 'Round skipped' WHERE round_id = $1 AND status = 'pending'", round_id)
            .execute(&self.pool).await?;
        Ok(result.rows_affected())
    }

    // Flags
    pub async fn create_flag(&self, job_id: i32, round_id: i32, challenge_id: i32, team_id: i32, flag_value: &str) -> Result<Flag> {
        let status = if self.is_flag_duplicate(flag_value).await { "duplicated" } else { "captured" };
        let flag = sqlx::query_as!(Flag,
            "INSERT INTO flags (job_id, round_id, challenge_id, team_id, flag_value, status) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
            job_id, round_id, challenge_id, team_id, flag_value, status
        ).fetch_one(&self.pool).await?;
        self.add_flag_to_cache(&flag).await;
        Ok(flag)
    }

    pub async fn create_manual_flag(&self, round_id: i32, challenge_id: i32, team_id: i32, flag_value: &str, status: &str) -> Result<Flag> {
        let status = if self.is_flag_duplicate(flag_value).await { "duplicated" } else { status };
        let flag = sqlx::query_as!(Flag,
            "INSERT INTO flags (job_id, round_id, challenge_id, team_id, flag_value, status, submitted_at) VALUES (NULL, $1, $2, $3, $4, $5, NOW()) RETURNING *",
            round_id, challenge_id, team_id, flag_value, status
        ).fetch_one(&self.pool).await?;
        self.add_flag_to_cache(&flag).await;
        Ok(flag)
    }

    pub async fn has_flag_for(&self, round_id: i32, challenge_id: i32, team_id: i32) -> Result<bool> {
        let count = sqlx::query_scalar!("SELECT COUNT(*) FROM flags WHERE round_id = $1 AND challenge_id = $2 AND team_id = $3", round_id, challenge_id, team_id)
            .fetch_one(&self.pool).await?;
        Ok(count.unwrap_or(0) > 0)
    }

    pub async fn list_flags(&self, round_id: Option<i32>, statuses: Option<Vec<String>>, desc: bool) -> Result<Vec<Flag>> {
        let order = if desc { "DESC" } else { "ASC" };
        match (round_id, statuses) {
            (Some(r), Some(s)) => Ok(sqlx::query_as(&format!("SELECT * FROM flags WHERE round_id = $1 AND status = ANY($2) ORDER BY id {}", order)).bind(r).bind(&s).fetch_all(&self.pool).await?),
            (Some(r), None) => Ok(sqlx::query_as(&format!("SELECT * FROM flags WHERE round_id = $1 ORDER BY id {}", order)).bind(r).fetch_all(&self.pool).await?),
            (None, Some(s)) => Ok(sqlx::query_as(&format!("SELECT * FROM flags WHERE status = ANY($1) ORDER BY id {}", order)).bind(&s).fetch_all(&self.pool).await?),
            (None, None) => Ok(sqlx::query_as(&format!("SELECT * FROM flags ORDER BY id {}", order)).fetch_all(&self.pool).await?),
        }
    }

    pub async fn update_flag_status(&self, id: i32, status: &str, force: bool) -> Result<bool> {
        let result = if force {
            sqlx::query!("UPDATE flags SET status = $2 WHERE id = $1", id, status).execute(&self.pool).await?
        } else {
            sqlx::query!("UPDATE flags SET status = $2 WHERE id = $1 AND status != 'success'", id, status).execute(&self.pool).await?
        };
        Ok(result.rows_affected() > 0)
    }

    pub async fn get_flag(&self, id: i32) -> Result<Flag> {
        Ok(sqlx::query_as!(Flag, "SELECT * FROM flags WHERE id = $1", id).fetch_one(&self.pool).await?)
    }

    // Settings
    pub async fn get_setting(&self, key: &str) -> Result<String> {
        let s = sqlx::query_as!(Setting, "SELECT * FROM settings WHERE key = $1", key).fetch_one(&self.pool).await?;
        Ok(s.value)
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query!("INSERT INTO settings (key, value) VALUES ($1, $2) ON CONFLICT (key) DO UPDATE SET value = $2", key, value)
            .execute(&self.pool).await?;
        Ok(())
    }

    pub async fn list_settings(&self) -> Result<Vec<Setting>> {
        Ok(sqlx::query_as!(Setting, "SELECT * FROM settings ORDER BY key").fetch_all(&self.pool).await?)
    }

    // Clean up jobs left with schedule_at but pending/running status
    pub async fn cleanup_stale_scheduled_jobs(&self, running_round_id: Option<i32>) -> Result<CleanupStaleScheduledReport> {
        let mut tx = self.pool.begin().await?;
        let suffix = format!("\n[{}]", stale_schedule_error_suffix());
        let rows = sqlx::query!(
            "UPDATE exploit_jobs
             SET status = $1,
                 finished_at = NOW(),
                 stderr = COALESCE(stderr, '') || $2
             WHERE schedule_at IS NOT NULL
               AND status IN ('pending', 'running')
             RETURNING id, round_id, exploit_run_id, team_id, priority",
            "error:stale",
            suffix
        )
        .fetch_all(&mut *tx)
        .await?;

        let mut requeue_round_ids: Vec<i32> = Vec::new();
        let mut requeue_run_ids: Vec<i32> = Vec::new();
        let mut requeue_team_ids: Vec<i32> = Vec::new();
        let mut requeue_priorities: Vec<i32> = Vec::new();
        let mut requeue_reasons: Vec<String> = Vec::new();

        if let Some(running_id) = running_round_id {
            for row in rows.iter() {
                if row.round_id != running_id {
                    continue;
                }
                let Some(exploit_run_id) = row.exploit_run_id else {
                    continue;
                };
                requeue_round_ids.push(row.round_id);
                requeue_run_ids.push(exploit_run_id);
                requeue_team_ids.push(row.team_id);
                requeue_priorities.push(row.priority);
                requeue_reasons.push(cleanup_requeue_reason(row.id));
            }
        }

        let mut requeued = 0usize;
        if !requeue_round_ids.is_empty() {
            let result = sqlx::query!(
                "INSERT INTO exploit_jobs (round_id, exploit_run_id, team_id, priority, create_reason)
                 SELECT * FROM UNNEST($1::int[], $2::int[], $3::int[], $4::int[], $5::text[])",
                &requeue_round_ids,
                &requeue_run_ids,
                &requeue_team_ids,
                &requeue_priorities,
                &requeue_reasons
            )
            .execute(&mut *tx)
            .await?;
            requeued = result.rows_affected() as usize;
        }

        tx.commit().await?;
        Ok(CleanupStaleScheduledReport { marked: rows.len(), requeued })
    }

    // Reset stale running jobs on startup
    pub async fn reset_stale_jobs(&self) -> Result<u64> {
        let result = sqlx::query!(r#"UPDATE exploit_jobs SET status = 'stopped', stderr = COALESCE(stderr, '') || E'\n[stopped by server restart]' WHERE status = 'running'"#)
            .execute(&self.pool).await?;
        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod cleanup_tests {
    use super::{cleanup_requeue_reason, stale_schedule_error_suffix};

    #[test]
    fn stale_schedule_error_suffix_is_stable() {
        assert_eq!(stale_schedule_error_suffix(), "stale schedule_at while pending/running");
    }

    #[test]
    fn cleanup_requeue_reason_formats_id() {
        assert_eq!(cleanup_requeue_reason(42), "cleanup_requeue:42");
    }
}
