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
        Ok(sqlx::query_as("INSERT INTO challenges (name, enabled, default_port, priority, flag_regex) VALUES ($1, $2, $3, $4, $5) RETURNING *")
            .bind(&c.name).bind(c.enabled.unwrap_or(true)).bind(c.default_port).bind(c.priority.unwrap_or(0)).bind(&c.flag_regex)
            .fetch_one(&self.pool).await?)
    }

    pub async fn list_challenges(&self) -> Result<Vec<Challenge>> {
        Ok(sqlx::query_as("SELECT * FROM challenges ORDER BY priority DESC, id").fetch_all(&self.pool).await?)
    }

    pub async fn get_challenge(&self, id: i32) -> Result<Challenge> {
        Ok(sqlx::query_as("SELECT * FROM challenges WHERE id = $1").bind(id).fetch_one(&self.pool).await?)
    }

    pub async fn set_challenge_enabled(&self, id: i32, enabled: bool) -> Result<()> {
        sqlx::query("UPDATE challenges SET enabled = $2 WHERE id = $1").bind(id).bind(enabled).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn update_challenge(&self, id: i32, c: CreateChallenge) -> Result<Challenge> {
        Ok(sqlx::query_as("UPDATE challenges SET name = $2, enabled = COALESCE($3, enabled), default_port = $4, priority = COALESCE($5, priority) WHERE id = $1 RETURNING *")
            .bind(id).bind(&c.name).bind(c.enabled).bind(c.default_port).bind(c.priority)
            .fetch_one(&self.pool).await?)
    }

    pub async fn delete_challenge(&self, id: i32) -> Result<()> {
        sqlx::query("DELETE FROM challenges WHERE id = $1").bind(id).execute(&self.pool).await?;
        Ok(())
    }

    // Teams
    pub async fn create_team(&self, t: CreateTeam) -> Result<Team> {
        Ok(sqlx::query_as("INSERT INTO teams (team_id, team_name, default_ip, priority) VALUES ($1, $2, $3, $4) RETURNING *")
            .bind(&t.team_id).bind(&t.team_name).bind(&t.default_ip).bind(t.priority.unwrap_or(0))
            .fetch_one(&self.pool).await?)
    }

    pub async fn list_teams(&self) -> Result<Vec<Team>> {
        Ok(sqlx::query_as("SELECT * FROM teams ORDER BY priority DESC, id").fetch_all(&self.pool).await?)
    }

    pub async fn get_team(&self, id: i32) -> Result<Team> {
        Ok(sqlx::query_as("SELECT * FROM teams WHERE id = $1").bind(id).fetch_one(&self.pool).await?)
    }

    pub async fn update_team(&self, id: i32, t: CreateTeam) -> Result<Team> {
        Ok(sqlx::query_as("UPDATE teams SET team_id = $2, team_name = $3, default_ip = $4, priority = COALESCE($5, priority) WHERE id = $1 RETURNING *")
            .bind(id).bind(&t.team_id).bind(&t.team_name).bind(&t.default_ip).bind(t.priority)
            .fetch_one(&self.pool).await?)
    }

    pub async fn delete_team(&self, id: i32) -> Result<()> {
        sqlx::query("DELETE FROM teams WHERE id = $1").bind(id).execute(&self.pool).await?;
        Ok(())
    }

    // Relations
    pub async fn create_relation(&self, challenge_id: i32, team_id: i32, addr: Option<String>, port: Option<i32>) -> Result<ChallengeTeamRelation> {
        Ok(sqlx::query_as("INSERT INTO challenge_team_relations (challenge_id, team_id, addr, port) VALUES ($1, $2, $3, $4) RETURNING *")
            .bind(challenge_id).bind(team_id).bind(addr).bind(port).fetch_one(&self.pool).await?)
    }

    pub async fn list_relations(&self, challenge_id: i32) -> Result<Vec<ChallengeTeamRelation>> {
        Ok(sqlx::query_as("SELECT * FROM challenge_team_relations WHERE challenge_id = $1").bind(challenge_id).fetch_all(&self.pool).await?)
    }

    pub async fn ensure_relations(&self, challenge_id: i32) -> Result<()> {
        sqlx::query("INSERT INTO challenge_team_relations (challenge_id, team_id) SELECT $1, id FROM teams ON CONFLICT DO NOTHING")
            .bind(challenge_id).execute(&self.pool).await?;
        Ok(())
    }

    // Exploits
    pub async fn create_exploit(&self, e: CreateExploit) -> Result<Exploit> {
        Ok(sqlx::query_as("INSERT INTO exploits (name, challenge_id, docker_image, entrypoint, enabled, priority, max_per_container, timeout_secs) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *")
            .bind(&e.name).bind(e.challenge_id).bind(&e.docker_image).bind(&e.entrypoint).bind(e.enabled.unwrap_or(true))
            .bind(e.priority.unwrap_or(0)).bind(e.max_per_container.unwrap_or(1)).bind(e.timeout_secs.unwrap_or(30))
            .fetch_one(&self.pool).await?)
    }

    pub async fn list_exploits(&self, challenge_id: Option<i32>) -> Result<Vec<Exploit>> {
        match challenge_id {
            Some(cid) => Ok(sqlx::query_as("SELECT * FROM exploits WHERE challenge_id = $1 ORDER BY priority DESC").bind(cid).fetch_all(&self.pool).await?),
            None => Ok(sqlx::query_as("SELECT * FROM exploits ORDER BY priority DESC").fetch_all(&self.pool).await?),
        }
    }

    pub async fn get_exploit(&self, id: i32) -> Result<Exploit> {
        Ok(sqlx::query_as("SELECT * FROM exploits WHERE id = $1").bind(id).fetch_one(&self.pool).await?)
    }

    // Exploit Runs
    pub async fn create_exploit_run(&self, r: CreateExploitRun) -> Result<ExploitRun> {
        Ok(sqlx::query_as("INSERT INTO exploit_runs (exploit_id, challenge_id, team_id, priority, sequence) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (exploit_id, challenge_id, team_id) DO UPDATE SET priority = EXCLUDED.priority, sequence = EXCLUDED.sequence RETURNING *")
            .bind(r.exploit_id).bind(r.challenge_id).bind(r.team_id).bind(r.priority).bind(r.sequence.unwrap_or(0))
            .fetch_one(&self.pool).await?)
    }

    pub async fn update_exploit_run(&self, id: i32, priority: Option<i32>, sequence: Option<i32>, enabled: Option<bool>) -> Result<ExploitRun> {
        Ok(sqlx::query_as("UPDATE exploit_runs SET priority = COALESCE($2, priority), sequence = COALESCE($3, sequence), enabled = COALESCE($4, enabled) WHERE id = $1 RETURNING *")
            .bind(id).bind(priority).bind(sequence).bind(enabled)
            .fetch_one(&self.pool).await?)
    }

    pub async fn delete_exploit_run(&self, id: i32) -> Result<()> {
        sqlx::query("DELETE FROM exploit_runs WHERE id = $1").bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn list_exploit_runs(&self, challenge_id: Option<i32>, team_id: Option<i32>) -> Result<Vec<ExploitRun>> {
        let mut q = "SELECT * FROM exploit_runs WHERE enabled = true".to_string();
        if challenge_id.is_some() { q.push_str(" AND challenge_id = $1"); }
        if team_id.is_some() { q.push_str(if challenge_id.is_some() { " AND team_id = $2" } else { " AND team_id = $1" }); }
        q.push_str(" ORDER BY COALESCE(priority, 0) DESC, sequence");
        
        match (challenge_id, team_id) {
            (Some(c), Some(t)) => Ok(sqlx::query_as(&q).bind(c).bind(t).fetch_all(&self.pool).await?),
            (Some(c), None) => Ok(sqlx::query_as(&q).bind(c).fetch_all(&self.pool).await?),
            (None, Some(t)) => Ok(sqlx::query_as(&q).bind(t).fetch_all(&self.pool).await?),
            (None, None) => Ok(sqlx::query_as(&q).fetch_all(&self.pool).await?),
        }
    }

    // Rounds
    pub async fn create_round(&self) -> Result<Round> {
        Ok(sqlx::query_as("INSERT INTO rounds DEFAULT VALUES RETURNING *").fetch_one(&self.pool).await?)
    }

    pub async fn get_round(&self, id: i32) -> Result<Round> {
        Ok(sqlx::query_as("SELECT * FROM rounds WHERE id = $1").bind(id).fetch_one(&self.pool).await?)
    }

    pub async fn finish_round(&self, id: i32) -> Result<()> {
        sqlx::query("UPDATE rounds SET finished_at = NOW(), status = 'finished' WHERE id = $1").bind(id).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn list_rounds(&self) -> Result<Vec<Round>> {
        Ok(sqlx::query_as("SELECT * FROM rounds ORDER BY id DESC").fetch_all(&self.pool).await?)
    }

    // Jobs
    pub async fn create_job(&self, round_id: i32, exploit_run_id: i32, team_id: i32, priority: i32) -> Result<ExploitJob> {
        Ok(sqlx::query_as("INSERT INTO exploit_jobs (round_id, exploit_run_id, team_id, priority) VALUES ($1, $2, $3, $4) RETURNING *")
            .bind(round_id).bind(exploit_run_id).bind(team_id).bind(priority).fetch_one(&self.pool).await?)
    }

    pub async fn list_jobs(&self, round_id: i32) -> Result<Vec<ExploitJob>> {
        Ok(sqlx::query_as("SELECT * FROM exploit_jobs WHERE round_id = $1 ORDER BY priority DESC").bind(round_id).fetch_all(&self.pool).await?)
    }

    pub async fn get_pending_jobs(&self, round_id: i32) -> Result<Vec<ExploitJob>> {
        Ok(sqlx::query_as("SELECT * FROM exploit_jobs WHERE round_id = $1 AND status = 'pending' ORDER BY priority DESC")
            .bind(round_id).fetch_all(&self.pool).await?)
    }

    pub async fn update_job_status(&self, id: i32, status: &str) -> Result<()> {
        sqlx::query("UPDATE exploit_jobs SET status = $2, started_at = CASE WHEN $2 = 'running' THEN NOW() ELSE started_at END WHERE id = $1")
            .bind(id).bind(status).execute(&self.pool).await?;
        Ok(())
    }

    pub async fn finish_job(&self, id: i32, status: &str, stdout: Option<&str>, stderr: Option<&str>, duration_ms: i32) -> Result<()> {
        sqlx::query("UPDATE exploit_jobs SET status = $2, stdout = $3, stderr = $4, duration_ms = $5, finished_at = NOW() WHERE id = $1")
            .bind(id).bind(status).bind(stdout).bind(stderr).bind(duration_ms).execute(&self.pool).await?;
        Ok(())
    }

    // Flags
    pub async fn create_flag(&self, job_id: i32, round_id: i32, challenge_id: i32, team_id: i32, flag_value: &str) -> Result<Flag> {
        Ok(sqlx::query_as("INSERT INTO flags (job_id, round_id, challenge_id, team_id, flag_value) VALUES ($1, $2, $3, $4, $5) RETURNING *")
            .bind(job_id).bind(round_id).bind(challenge_id).bind(team_id).bind(flag_value).fetch_one(&self.pool).await?)
    }

    pub async fn list_flags(&self, round_id: Option<i32>) -> Result<Vec<Flag>> {
        match round_id {
            Some(r) => Ok(sqlx::query_as("SELECT * FROM flags WHERE round_id = $1 ORDER BY id DESC").bind(r).fetch_all(&self.pool).await?),
            None => Ok(sqlx::query_as("SELECT * FROM flags ORDER BY id DESC").fetch_all(&self.pool).await?),
        }
    }
}
