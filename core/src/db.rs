use crate::models::*;
use sqlx::{PgPool, postgres::PgPoolOptions};
use anyhow::Result;

#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
}

impl Database {
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new().max_connections(10).connect(url).await?;
        Ok(Self { pool })
    }

    // Challenges
    pub async fn create_challenge(&self, c: CreateChallenge) -> Result<Challenge> {
        Ok(sqlx::query_as!(Challenge,
            "INSERT INTO challenges (name, enabled, default_port, priority, flag_regex) VALUES ($1, $2, $3, $4, $5) RETURNING *",
            c.name, c.enabled.unwrap_or(true), c.default_port, c.priority.unwrap_or(0), c.flag_regex
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
        Ok(sqlx::query_as!(Challenge,
            "UPDATE challenges SET name = $2, enabled = COALESCE($3, enabled), default_port = $4, priority = COALESCE($5, priority), flag_regex = $6 WHERE id = $1 RETURNING *",
            id, c.name, c.enabled, c.default_port, c.priority, c.flag_regex
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
            t.team_id, t.team_name, t.default_ip, t.priority.unwrap_or(0), t.enabled.unwrap_or(true)
        ).fetch_one(&self.pool).await?)
    }

    pub async fn list_teams(&self) -> Result<Vec<Team>> {
        Ok(sqlx::query_as!(Team, "SELECT * FROM teams ORDER BY priority DESC, id").fetch_all(&self.pool).await?)
    }

    pub async fn get_team(&self, id: i32) -> Result<Team> {
        Ok(sqlx::query_as!(Team, "SELECT * FROM teams WHERE id = $1", id).fetch_one(&self.pool).await?)
    }

    pub async fn update_team(&self, id: i32, t: CreateTeam) -> Result<Team> {
        Ok(sqlx::query_as!(Team,
            "UPDATE teams SET team_id = $2, team_name = $3, default_ip = $4, priority = COALESCE($5, priority), enabled = COALESCE($6, enabled) WHERE id = $1 RETURNING *",
            id, t.team_id, t.team_name, t.default_ip, t.priority, t.enabled
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

    pub async fn update_relation(&self, challenge_id: i32, team_id: i32, addr: Option<String>, port: Option<i32>) -> Result<ChallengeTeamRelation> {
        Ok(sqlx::query_as!(ChallengeTeamRelation,
            "INSERT INTO challenge_team_relations (challenge_id, team_id, addr, port) VALUES ($1, $2, $3, $4) ON CONFLICT (challenge_id, team_id) DO UPDATE SET addr = $3, port = $4 RETURNING *",
            challenge_id, team_id, addr, port
        ).fetch_one(&self.pool).await?)
    }

    pub async fn ensure_relations(&self, challenge_id: i32) -> Result<()> {
        sqlx::query!("INSERT INTO challenge_team_relations (challenge_id, team_id) SELECT $1, id FROM teams ON CONFLICT DO NOTHING", challenge_id)
            .execute(&self.pool).await?;
        Ok(())
    }

    // Exploits
    pub async fn create_exploit(&self, e: CreateExploit) -> Result<Exploit> {
        Ok(sqlx::query_as!(Exploit,
            "INSERT INTO exploits (name, challenge_id, docker_image, entrypoint, enabled, priority, max_per_container, timeout_secs, default_counter) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *",
            e.name, e.challenge_id, e.docker_image, e.entrypoint, e.enabled.unwrap_or(true), e.priority.unwrap_or(0), e.max_per_container.unwrap_or(1), e.timeout_secs.unwrap_or(30), e.default_counter.unwrap_or(999)
        ).fetch_one(&self.pool).await?)
    }

    pub async fn list_exploits(&self, challenge_id: Option<i32>) -> Result<Vec<Exploit>> {
        match challenge_id {
            Some(cid) => Ok(sqlx::query_as!(Exploit, "SELECT * FROM exploits WHERE challenge_id = $1 ORDER BY priority DESC", cid).fetch_all(&self.pool).await?),
            None => Ok(sqlx::query_as!(Exploit, "SELECT * FROM exploits ORDER BY priority DESC").fetch_all(&self.pool).await?),
        }
    }

    pub async fn get_exploit(&self, id: i32) -> Result<Exploit> {
        Ok(sqlx::query_as!(Exploit, "SELECT * FROM exploits WHERE id = $1", id).fetch_one(&self.pool).await?)
    }

    pub async fn list_enabled_exploits(&self) -> Result<Vec<Exploit>> {
        Ok(sqlx::query_as!(Exploit, "SELECT * FROM exploits WHERE enabled = true ORDER BY priority DESC").fetch_all(&self.pool).await?)
    }

    pub async fn update_exploit(&self, id: i32, e: UpdateExploit) -> Result<Exploit> {
        Ok(sqlx::query_as!(Exploit,
            "UPDATE exploits SET name = $2, docker_image = $3, entrypoint = $4, enabled = COALESCE($5, enabled), priority = COALESCE($6, priority), max_per_container = COALESCE($7, max_per_container), timeout_secs = COALESCE($8, timeout_secs), default_counter = COALESCE($9, default_counter) WHERE id = $1 RETURNING *",
            id, e.name, e.docker_image, e.entrypoint, e.enabled, e.priority, e.max_per_container, e.timeout_secs, e.default_counter
        ).fetch_one(&self.pool).await?)
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
            "UPDATE exploit_runs SET priority = COALESCE($2, priority), sequence = COALESCE($3, sequence), enabled = COALESCE($4, enabled) WHERE id = $1 RETURNING *",
            id, priority, sequence, enabled
        ).fetch_one(&self.pool).await?)
    }

    pub async fn delete_exploit_run(&self, id: i32) -> Result<()> {
        sqlx::query!("DELETE FROM exploit_runs WHERE id = $1", id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn reorder_exploit_runs(&self, items: &[(i32, i32)]) -> Result<()> {
        for (id, seq) in items {
            sqlx::query!("UPDATE exploit_runs SET sequence = $2 WHERE id = $1", id, seq).execute(&self.pool).await?;
        }
        Ok(())
    }

    pub async fn get_exploit_run(&self, id: i32) -> Result<ExploitRun> {
        Ok(sqlx::query_as!(ExploitRun, "SELECT * FROM exploit_runs WHERE id = $1", id).fetch_one(&self.pool).await?)
    }

    // Rounds
    pub async fn create_round(&self) -> Result<Round> {
        Ok(sqlx::query_as!(Round, "INSERT INTO rounds DEFAULT VALUES RETURNING *").fetch_one(&self.pool).await?)
    }

    pub async fn get_round(&self, id: i32) -> Result<Round> {
        Ok(sqlx::query_as!(Round, "SELECT * FROM rounds WHERE id = $1", id).fetch_one(&self.pool).await?)
    }

    pub async fn finish_round(&self, id: i32) -> Result<()> {
        sqlx::query!("UPDATE rounds SET finished_at = NOW(), status = 'finished' WHERE id = $1", id).execute(&self.pool).await?;
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

    pub async fn clean_rounds(&self) -> Result<()> {
        sqlx::query!("TRUNCATE flags, exploit_jobs, rounds RESTART IDENTITY CASCADE").execute(&self.pool).await?;
        Ok(())
    }

    // Jobs
    pub async fn create_job(&self, round_id: i32, exploit_run_id: i32, team_id: i32, priority: i32) -> Result<ExploitJob> {
        Ok(sqlx::query_as!(ExploitJob,
            "INSERT INTO exploit_jobs (round_id, exploit_run_id, team_id, priority) VALUES ($1, $2, $3, $4) RETURNING *",
            round_id, exploit_run_id, team_id, priority
        ).fetch_one(&self.pool).await?)
    }

    pub async fn create_adhoc_job(&self, exploit_run_id: i32, team_id: i32) -> Result<ExploitJob> {
        Ok(sqlx::query_as!(ExploitJob,
            "INSERT INTO exploit_jobs (round_id, exploit_run_id, team_id, priority) VALUES (NULL, $1, $2, 0) RETURNING *",
            exploit_run_id, team_id
        ).fetch_one(&self.pool).await?)
    }

    pub async fn list_jobs(&self, round_id: i32) -> Result<Vec<ExploitJob>> {
        Ok(sqlx::query_as!(ExploitJob, "SELECT * FROM exploit_jobs WHERE round_id = $1 ORDER BY priority DESC", round_id).fetch_all(&self.pool).await?)
    }

    pub async fn get_job(&self, id: i32) -> Result<ExploitJob> {
        Ok(sqlx::query_as!(ExploitJob, "SELECT * FROM exploit_jobs WHERE id = $1", id).fetch_one(&self.pool).await?)
    }

    pub async fn get_pending_jobs(&self, round_id: i32) -> Result<Vec<ExploitJob>> {
        Ok(sqlx::query_as!(ExploitJob,
            "SELECT * FROM exploit_jobs WHERE round_id = $1 AND status = 'pending' ORDER BY priority DESC", round_id
        ).fetch_all(&self.pool).await?)
    }

    pub async fn count_running_jobs_for_container(&self, container_id: &str) -> Result<i64> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM exploit_jobs WHERE container_id = $1 AND status = 'running'")
            .bind(container_id)
            .fetch_one(&self.pool).await?;
        Ok(count)
    }

    pub async fn update_job_status(&self, id: i32, status: &str, set_started: bool) -> Result<()> {
        if set_started {
            sqlx::query!("UPDATE exploit_jobs SET status = $2, started_at = NOW() WHERE id = $1", id, status).execute(&self.pool).await?;
        } else {
            sqlx::query!("UPDATE exploit_jobs SET status = $2 WHERE id = $1", id, status).execute(&self.pool).await?;
        }
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
        for (id, priority) in items {
            sqlx::query!("UPDATE exploit_jobs SET priority = $2 WHERE id = $1 AND status = 'pending'", id, priority).execute(&self.pool).await?;
        }
        Ok(())
    }

    // Flags
    pub async fn create_flag(&self, job_id: i32, round_id: i32, challenge_id: i32, team_id: i32, flag_value: &str) -> Result<Flag> {
        Ok(sqlx::query_as!(Flag,
            "INSERT INTO flags (job_id, round_id, challenge_id, team_id, flag_value) VALUES ($1, $2, $3, $4, $5) RETURNING *",
            job_id, round_id, challenge_id, team_id, flag_value
        ).fetch_one(&self.pool).await?)
    }

    pub async fn create_adhoc_flag(&self, job_id: i32, challenge_id: i32, team_id: i32, flag_value: &str) -> Result<Flag> {
        Ok(sqlx::query_as!(Flag,
            "INSERT INTO flags (job_id, round_id, challenge_id, team_id, flag_value) VALUES ($1, NULL, $2, $3, $4) RETURNING *",
            job_id, challenge_id, team_id, flag_value
        ).fetch_one(&self.pool).await?)
    }

    pub async fn has_flag_for(&self, round_id: i32, challenge_id: i32, team_id: i32) -> Result<bool> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM flags WHERE round_id = $1 AND challenge_id = $2 AND team_id = $3")
            .bind(round_id).bind(challenge_id).bind(team_id)
            .fetch_one(&self.pool).await?;
        Ok(count > 0)
    }

    pub async fn list_flags(&self, round_id: Option<i32>) -> Result<Vec<Flag>> {
        match round_id {
            Some(r) => Ok(sqlx::query_as!(Flag, "SELECT * FROM flags WHERE round_id = $1 ORDER BY id DESC", r).fetch_all(&self.pool).await?),
            None => Ok(sqlx::query_as!(Flag, "SELECT * FROM flags ORDER BY id DESC").fetch_all(&self.pool).await?),
        }
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

    // Exploit Containers
    pub async fn create_exploit_container(&self, exploit_id: i32, container_id: &str, counter: i32) -> Result<ExploitContainer> {
        Ok(sqlx::query_as!(ExploitContainer,
            "INSERT INTO exploit_containers (exploit_id, container_id, counter) VALUES ($1, $2, $3) RETURNING *",
            exploit_id, container_id, counter
        ).fetch_one(&self.pool).await?)
    }

    pub async fn get_exploit_containers(&self, exploit_id: i32) -> Result<Vec<ExploitContainer>> {
        Ok(sqlx::query_as!(ExploitContainer,
            "SELECT * FROM exploit_containers WHERE exploit_id = $1 AND status = 'running' ORDER BY id",
            exploit_id
        ).fetch_all(&self.pool).await?)
    }

    pub async fn list_all_containers(&self) -> Result<Vec<ExploitContainer>> {
        Ok(sqlx::query_as!(ExploitContainer, "SELECT * FROM exploit_containers ORDER BY id").fetch_all(&self.pool).await?)
    }

    pub async fn get_container(&self, id: i32) -> Result<ExploitContainer> {
        Ok(sqlx::query_as!(ExploitContainer, "SELECT * FROM exploit_containers WHERE id = $1", id).fetch_one(&self.pool).await?)
    }

    pub async fn get_available_container(&self, exploit_id: i32, max_per_container: i32) -> Result<Option<ExploitContainer>> {
        Ok(sqlx::query_as!(ExploitContainer,
            r#"SELECT c.* FROM exploit_containers c
               LEFT JOIN exploit_runners r ON r.exploit_container_id = c.id
               WHERE c.exploit_id = $1 AND c.status = 'running' AND c.counter > 0
               GROUP BY c.id HAVING COUNT(r.id) < $2
               ORDER BY c.id LIMIT 1"#,
            exploit_id, max_per_container as i64
        ).fetch_optional(&self.pool).await?)
    }

    pub async fn decrement_container_counter(&self, id: i32) -> Result<i32> {
        let rec = sqlx::query!("UPDATE exploit_containers SET counter = counter - 1 WHERE id = $1 RETURNING counter", id)
            .fetch_one(&self.pool).await?;
        Ok(rec.counter)
    }

    pub async fn set_container_status(&self, id: i32, status: &str) -> Result<()> {
        sqlx::query!("UPDATE exploit_containers SET status = $2 WHERE id = $1", id, status).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn delete_exploit_container(&self, id: i32) -> Result<()> {
        sqlx::query!("DELETE FROM exploit_containers WHERE id = $1", id).execute(&self.pool).await?;
        Ok(())
    }

    // Exploit Runners
    pub async fn create_exploit_runner(&self, container_id: i32, run_id: i32, team_id: i32) -> Result<ExploitRunner> {
        Ok(sqlx::query_as!(ExploitRunner,
            "INSERT INTO exploit_runners (exploit_container_id, exploit_run_id, team_id) VALUES ($1, $2, $3) RETURNING *",
            container_id, run_id, team_id
        ).fetch_one(&self.pool).await?)
    }

    pub async fn get_runner_for_run(&self, exploit_run_id: i32) -> Result<Option<ExploitRunner>> {
        Ok(sqlx::query_as!(ExploitRunner,
            "SELECT * FROM exploit_runners WHERE exploit_run_id = $1", exploit_run_id
        ).fetch_optional(&self.pool).await?)
    }

    pub async fn get_runners_for_container(&self, container_id: i32) -> Result<Vec<ExploitRunner>> {
        Ok(sqlx::query_as!(ExploitRunner,
            "SELECT * FROM exploit_runners WHERE exploit_container_id = $1", container_id
        ).fetch_all(&self.pool).await?)
    }

    pub async fn delete_runners_for_container(&self, container_id: i32) -> Result<()> {
        sqlx::query!("DELETE FROM exploit_runners WHERE exploit_container_id = $1", container_id).execute(&self.pool).await?;
        Ok(())
    }

    // Reset stale running jobs on startup
    pub async fn reset_stale_jobs(&self) -> Result<u64> {
        let result = sqlx::query("UPDATE exploit_jobs SET status = 'error', stdout = COALESCE(stdout, '') || '\n[interrupted by server restart]' WHERE status = 'running'")
            .execute(&self.pool).await?;
        Ok(result.rows_affected())
    }
}
