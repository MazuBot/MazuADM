use crate::models::*;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, postgres::PgPoolOptions};
use anyhow::Result;

#[derive(Clone)]
pub struct Database {
    pub pool: PgPool,
}

#[derive(Debug)]
pub struct JobContextData {
    pub job: ExploitJob,
    pub run: Option<ExploitRun>,
    pub exploit: Option<Exploit>,
    pub challenge: Option<Challenge>,
    pub team: Option<Team>,
    pub relation: Option<ChallengeTeamRelation>,
}

#[derive(sqlx::FromRow)]
struct JobContextRow {
    job_id: i32,
    job_round_id: i32,
    job_exploit_run_id: Option<i32>,
    job_team_id: i32,
    job_priority: i32,
    job_status: String,
    job_container_id: Option<String>,
    job_stdout: Option<String>,
    job_stderr: Option<String>,
    job_create_reason: Option<String>,
    job_duration_ms: Option<i32>,
    job_schedule_at: Option<DateTime<Utc>>,
    job_started_at: Option<DateTime<Utc>>,
    job_finished_at: Option<DateTime<Utc>>,
    job_created_at: DateTime<Utc>,
    run_id: Option<i32>,
    run_exploit_id: Option<i32>,
    run_challenge_id: Option<i32>,
    run_team_id: Option<i32>,
    run_priority: Option<i32>,
    run_sequence: Option<i32>,
    run_enabled: Option<bool>,
    run_created_at: Option<DateTime<Utc>>,
    exploit_id: Option<i32>,
    exploit_name: Option<String>,
    exploit_challenge_id: Option<i32>,
    exploit_enabled: Option<bool>,
    exploit_max_per_container: Option<i32>,
    exploit_max_containers: Option<i32>,
    exploit_docker_image: Option<String>,
    exploit_entrypoint: Option<String>,
    exploit_timeout_secs: Option<i32>,
    exploit_default_counter: Option<i32>,
    exploit_max_concurrent_jobs: Option<i32>,
    exploit_created_at: Option<DateTime<Utc>>,
    challenge_id: Option<i32>,
    challenge_name: Option<String>,
    challenge_enabled: Option<bool>,
    challenge_default_port: Option<i32>,
    challenge_priority: Option<i32>,
    challenge_flag_regex: Option<String>,
    challenge_created_at: Option<DateTime<Utc>>,
    team_id: Option<i32>,
    team_team_id: Option<String>,
    team_team_name: Option<String>,
    team_default_ip: Option<String>,
    team_priority: Option<i32>,
    team_created_at: Option<DateTime<Utc>>,
    team_enabled: Option<bool>,
    rel_id: Option<i32>,
    rel_challenge_id: Option<i32>,
    rel_team_id: Option<i32>,
    rel_addr: Option<String>,
    rel_port: Option<i32>,
    rel_created_at: Option<DateTime<Utc>>,
}

impl Database {
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new().max_connections(10).connect(url).await?;
        Ok(Self { pool })
    }

    fn clamp_priority(p: Option<i32>) -> i32 {
        p.unwrap_or(0).clamp(0, 99)
    }

    pub async fn get_job_context_data(&self, job_id: i32) -> Result<JobContextData> {
        let row = sqlx::query_as::<_, JobContextRow>(
            r#"
SELECT
    ej.id AS job_id,
    ej.round_id AS job_round_id,
    ej.exploit_run_id AS job_exploit_run_id,
    ej.team_id AS job_team_id,
    ej.priority AS job_priority,
    ej.status AS job_status,
    ej.container_id AS job_container_id,
    ej.stdout AS job_stdout,
    ej.stderr AS job_stderr,
    ej.create_reason AS job_create_reason,
    ej.duration_ms AS job_duration_ms,
    ej.schedule_at AS job_schedule_at,
    ej.started_at AS job_started_at,
    ej.finished_at AS job_finished_at,
    ej.created_at AS job_created_at,
    er.id AS run_id,
    er.exploit_id AS run_exploit_id,
    er.challenge_id AS run_challenge_id,
    er.team_id AS run_team_id,
    er.priority AS run_priority,
    er.sequence AS run_sequence,
    er.enabled AS run_enabled,
    er.created_at AS run_created_at,
    e.id AS exploit_id,
    e.name AS exploit_name,
    e.challenge_id AS exploit_challenge_id,
    e.enabled AS exploit_enabled,
    e.max_per_container AS exploit_max_per_container,
    e.max_containers AS exploit_max_containers,
    e.docker_image AS exploit_docker_image,
    e.entrypoint AS exploit_entrypoint,
    e.timeout_secs AS exploit_timeout_secs,
    e.default_counter AS exploit_default_counter,
    e.max_concurrent_jobs AS exploit_max_concurrent_jobs,
    e.created_at AS exploit_created_at,
    c.id AS challenge_id,
    c.name AS challenge_name,
    c.enabled AS challenge_enabled,
    c.default_port AS challenge_default_port,
    c.priority AS challenge_priority,
    c.flag_regex AS challenge_flag_regex,
    c.created_at AS challenge_created_at,
    t.id AS team_id,
    t.team_id AS team_team_id,
    t.team_name AS team_team_name,
    t.default_ip AS team_default_ip,
    t.priority AS team_priority,
    t.created_at AS team_created_at,
    t.enabled AS team_enabled,
    ctr.id AS rel_id,
    ctr.challenge_id AS rel_challenge_id,
    ctr.team_id AS rel_team_id,
    ctr.addr AS rel_addr,
    ctr.port AS rel_port,
    ctr.created_at AS rel_created_at
FROM exploit_jobs ej
LEFT JOIN exploit_runs er ON er.id = ej.exploit_run_id
LEFT JOIN exploits e ON e.id = er.exploit_id
LEFT JOIN challenges c ON c.id = er.challenge_id
LEFT JOIN teams t ON t.id = ej.team_id
LEFT JOIN challenge_team_relations ctr ON ctr.challenge_id = c.id AND ctr.team_id = t.id
WHERE ej.id = $1
"#,
        )
        .bind(job_id)
        .fetch_one(&self.pool)
        .await?;

        let JobContextRow {
            job_id,
            job_round_id,
            job_exploit_run_id,
            job_team_id,
            job_priority,
            job_status,
            job_container_id,
            job_stdout,
            job_stderr,
            job_create_reason,
            job_duration_ms,
            job_schedule_at,
            job_started_at,
            job_finished_at,
            job_created_at,
            run_id,
            run_exploit_id,
            run_challenge_id,
            run_team_id,
            run_priority,
            run_sequence,
            run_enabled,
            run_created_at,
            exploit_id,
            exploit_name,
            exploit_challenge_id,
            exploit_enabled,
            exploit_max_per_container,
            exploit_max_containers,
            exploit_docker_image,
            exploit_entrypoint,
            exploit_timeout_secs,
            exploit_default_counter,
            exploit_max_concurrent_jobs,
            exploit_created_at,
            challenge_id,
            challenge_name,
            challenge_enabled,
            challenge_default_port,
            challenge_priority,
            challenge_flag_regex,
            challenge_created_at,
            team_id,
            team_team_id,
            team_team_name,
            team_default_ip,
            team_priority,
            team_created_at,
            team_enabled,
            rel_id,
            rel_challenge_id,
            rel_team_id,
            rel_addr,
            rel_port,
            rel_created_at,
        } = row;

        let job = ExploitJob {
            id: job_id,
            round_id: job_round_id,
            exploit_run_id: job_exploit_run_id,
            team_id: job_team_id,
            priority: job_priority,
            status: job_status,
            container_id: job_container_id,
            stdout: job_stdout,
            stderr: job_stderr,
            create_reason: job_create_reason,
            duration_ms: job_duration_ms,
            schedule_at: job_schedule_at,
            started_at: job_started_at,
            finished_at: job_finished_at,
            created_at: job_created_at,
        };

        let run = match run_id {
            Some(run_id) => {
                let exploit_id = run_exploit_id.ok_or_else(|| anyhow::anyhow!("missing exploit_id for run {}", run_id))?;
                let challenge_id = run_challenge_id.ok_or_else(|| anyhow::anyhow!("missing challenge_id for run {}", run_id))?;
                let team_id = run_team_id.ok_or_else(|| anyhow::anyhow!("missing team_id for run {}", run_id))?;
                let sequence = run_sequence.ok_or_else(|| anyhow::anyhow!("missing sequence for run {}", run_id))?;
                let enabled = run_enabled.ok_or_else(|| anyhow::anyhow!("missing enabled for run {}", run_id))?;
                let created_at = run_created_at.ok_or_else(|| anyhow::anyhow!("missing created_at for run {}", run_id))?;
                Some(ExploitRun {
                    id: run_id,
                    exploit_id,
                    challenge_id,
                    team_id,
                    priority: run_priority,
                    sequence,
                    enabled,
                    created_at,
                })
            }
            None => None,
        };

        let exploit = match exploit_id {
            Some(exploit_id) => {
                let name = exploit_name.ok_or_else(|| anyhow::anyhow!("missing name for exploit {}", exploit_id))?;
                let challenge_id = exploit_challenge_id.ok_or_else(|| anyhow::anyhow!("missing challenge_id for exploit {}", exploit_id))?;
                let enabled = exploit_enabled.ok_or_else(|| anyhow::anyhow!("missing enabled for exploit {}", exploit_id))?;
                let max_per_container = exploit_max_per_container.ok_or_else(|| anyhow::anyhow!("missing max_per_container for exploit {}", exploit_id))?;
                let max_containers = exploit_max_containers.ok_or_else(|| anyhow::anyhow!("missing max_containers for exploit {}", exploit_id))?;
                let docker_image = exploit_docker_image.ok_or_else(|| anyhow::anyhow!("missing docker_image for exploit {}", exploit_id))?;
                let timeout_secs = exploit_timeout_secs.ok_or_else(|| anyhow::anyhow!("missing timeout_secs for exploit {}", exploit_id))?;
                let default_counter = exploit_default_counter.ok_or_else(|| anyhow::anyhow!("missing default_counter for exploit {}", exploit_id))?;
                let max_concurrent_jobs = exploit_max_concurrent_jobs.ok_or_else(|| anyhow::anyhow!("missing max_concurrent_jobs for exploit {}", exploit_id))?;
                let created_at = exploit_created_at.ok_or_else(|| anyhow::anyhow!("missing created_at for exploit {}", exploit_id))?;
                Some(Exploit {
                    id: exploit_id,
                    name,
                    challenge_id,
                    enabled,
                    max_per_container,
                    max_containers,
                    max_concurrent_jobs,
                    docker_image,
                    entrypoint: exploit_entrypoint,
                    timeout_secs,
                    default_counter,
                    created_at,
                })
            }
            None => None,
        };

        let challenge = match challenge_id {
            Some(challenge_id) => {
                let name = challenge_name.ok_or_else(|| anyhow::anyhow!("missing name for challenge {}", challenge_id))?;
                let enabled = challenge_enabled.ok_or_else(|| anyhow::anyhow!("missing enabled for challenge {}", challenge_id))?;
                let priority = challenge_priority.ok_or_else(|| anyhow::anyhow!("missing priority for challenge {}", challenge_id))?;
                let created_at = challenge_created_at.ok_or_else(|| anyhow::anyhow!("missing created_at for challenge {}", challenge_id))?;
                Some(Challenge {
                    id: challenge_id,
                    name,
                    enabled,
                    default_port: challenge_default_port,
                    priority,
                    flag_regex: challenge_flag_regex,
                    created_at,
                })
            }
            None => None,
        };

        let team = match team_id {
            Some(team_id) => {
                let team_code = team_team_id.ok_or_else(|| anyhow::anyhow!("missing team_id for team {}", team_id))?;
                let team_name = team_team_name.ok_or_else(|| anyhow::anyhow!("missing team_name for team {}", team_id))?;
                let priority = team_priority.ok_or_else(|| anyhow::anyhow!("missing priority for team {}", team_id))?;
                let created_at = team_created_at.ok_or_else(|| anyhow::anyhow!("missing created_at for team {}", team_id))?;
                let enabled = team_enabled.ok_or_else(|| anyhow::anyhow!("missing enabled for team {}", team_id))?;
                Some(Team {
                    id: team_id,
                    team_id: team_code,
                    team_name,
                    default_ip: team_default_ip,
                    priority,
                    created_at,
                    enabled,
                })
            }
            None => None,
        };

        let relation = match rel_id {
            Some(rel_id) => {
                let challenge_id = rel_challenge_id.ok_or_else(|| anyhow::anyhow!("missing challenge_id for relation {}", rel_id))?;
                let team_id = rel_team_id.ok_or_else(|| anyhow::anyhow!("missing team_id for relation {}", rel_id))?;
                let created_at = rel_created_at.ok_or_else(|| anyhow::anyhow!("missing created_at for relation {}", rel_id))?;
                Some(ChallengeTeamRelation {
                    id: rel_id,
                    challenge_id,
                    team_id,
                    addr: rel_addr,
                    port: rel_port,
                    created_at,
                })
            }
            None => None,
        };

        Ok(JobContextData {
            job,
            run,
            exploit,
            challenge,
            team,
            relation,
        })
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

    pub async fn update_connection_info(&self, challenge_id: i32, team_id: i32, addr: Option<String>, port: Option<i32>) -> Result<ChallengeTeamRelation> {
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
        let exploit = sqlx::query_as!(
            Exploit,
            "INSERT INTO exploits (name, challenge_id, docker_image, entrypoint, enabled, max_per_container, max_containers, max_concurrent_jobs, timeout_secs, default_counter) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *",
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
            "UPDATE exploits SET name = $2, docker_image = $3, entrypoint = $4, enabled = COALESCE($5, enabled), max_per_container = COALESCE($6, max_per_container), max_containers = COALESCE($7, max_containers), max_concurrent_jobs = COALESCE($8, max_concurrent_jobs), timeout_secs = COALESCE($9, timeout_secs), default_counter = COALESCE($10, default_counter) WHERE id = $1 RETURNING *",
            id,
            e.name,
            e.docker_image,
            e.entrypoint,
            e.enabled,
            e.max_per_container,
            e.max_containers,
            e.max_concurrent_jobs,
            e.timeout_secs,
            e.default_counter
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

    pub async fn clone_unflagged_jobs_for_round(&self, round_id: i32) -> Result<u64> {
        let result = sqlx::query!(
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
               )",
            round_id
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected())
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
    pub async fn create_job(&self, round_id: i32, exploit_run_id: i32, team_id: i32, priority: i32, create_reason: Option<&str>) -> Result<ExploitJob> {
        Ok(sqlx::query_as!(ExploitJob,
            "INSERT INTO exploit_jobs (round_id, exploit_run_id, team_id, priority, create_reason) VALUES ($1, $2, $3, $4, $5) RETURNING *",
            round_id, exploit_run_id, team_id, priority, create_reason
        ).fetch_one(&self.pool).await?)
    }

    pub async fn list_jobs(&self, round_id: i32) -> Result<Vec<ExploitJob>> {
        Ok(sqlx::query_as!(ExploitJob,
            "SELECT id, round_id, exploit_run_id, team_id, priority, status, container_id, NULL::TEXT AS stdout, NULL::TEXT AS stderr, create_reason, duration_ms, schedule_at, started_at, finished_at, created_at FROM exploit_jobs WHERE round_id = $1 ORDER BY priority DESC",
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

    pub async fn mark_job_running(&self, id: i32) -> Result<ExploitJob> {
        let job = sqlx::query_as::<_, ExploitJob>(
            "UPDATE exploit_jobs SET status = 'running', started_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(job)
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

    pub async fn finish_job(&self, id: i32, status: &str, stdout: Option<&str>, stderr: Option<&str>, duration_ms: i32) -> Result<ExploitJob> {
        let job = sqlx::query_as::<_, ExploitJob>(
            "UPDATE exploit_jobs SET status = $2, stdout = $3, stderr = $4, duration_ms = $5, finished_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(status)
        .bind(stdout)
        .bind(stderr)
        .bind(duration_ms)
        .fetch_one(&self.pool)
        .await?;
        Ok(job)
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
        Ok(sqlx::query_as!(Flag,
            "INSERT INTO flags (job_id, round_id, challenge_id, team_id, flag_value, status) VALUES ($1, $2, $3, $4, $5, 'captured') RETURNING *",
            job_id, round_id, challenge_id, team_id, flag_value
        ).fetch_one(&self.pool).await?)
    }

    pub async fn create_manual_flag(&self, round_id: i32, challenge_id: i32, team_id: i32, flag_value: &str, status: &str) -> Result<Flag> {
        Ok(sqlx::query_as!(Flag,
            "INSERT INTO flags (job_id, round_id, challenge_id, team_id, flag_value, status, submitted_at) VALUES (NULL, $1, $2, $3, $4, $5, NOW()) RETURNING *",
            round_id, challenge_id, team_id, flag_value, status
        ).fetch_one(&self.pool).await?)
    }

    pub async fn has_flag_for(&self, round_id: i32, challenge_id: i32, team_id: i32) -> Result<bool> {
        let count = sqlx::query_scalar!("SELECT COUNT(*) FROM flags WHERE round_id = $1 AND challenge_id = $2 AND team_id = $3", round_id, challenge_id, team_id)
            .fetch_one(&self.pool).await?;
        Ok(count.unwrap_or(0) > 0)
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

    // Reset stale running jobs on startup
    pub async fn reset_stale_jobs(&self) -> Result<u64> {
        let result = sqlx::query!(r#"UPDATE exploit_jobs SET status = 'stopped', stderr = COALESCE(stderr, '') || E'\n[stopped by server restart]' WHERE status = 'running'"#)
            .execute(&self.pool).await?;
        Ok(result.rows_affected())
    }
}
