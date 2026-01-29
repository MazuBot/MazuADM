use crate::{Database, ExploitJob, Round, WsMessage};
use crate::executor::{
    build_job_context,
    finish_job_and_broadcast,
    get_target_lock,
    log_job_error,
    should_skip_job,
    stagger_delay_ms,
    JobContextError,
};
use crate::executor::Executor;
use crate::settings::{compute_timeout, load_executor_settings, load_job_settings};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex, Notify, Semaphore};
use tokio::task::JoinSet;
use std::time::Duration;

fn broadcast<T: serde::Serialize>(tx: &broadcast::Sender<WsMessage>, msg_type: &str, data: &T) {
    let _ = tx.send(WsMessage::new(msg_type, data));
}

#[derive(Debug, Clone)]
pub enum SchedulerCommand {
    RunRound(i32),
    RerunRound(i32),
    ScheduleUnflagged(i32),
    RunPending(i32),
}

pub struct SchedulerRunner {
    scheduler: Scheduler,
    notify: Arc<Notify>,
    rx: mpsc::UnboundedReceiver<SchedulerCommand>,
}

#[derive(Clone)]
pub struct SchedulerHandle {
    tx: mpsc::UnboundedSender<SchedulerCommand>,
    notify: Arc<Notify>,
}

impl SchedulerRunner {
    pub fn new(scheduler: Scheduler) -> (Self, SchedulerHandle) {
        let (tx, rx) = mpsc::unbounded_channel();
        let notify = Arc::new(Notify::new());
        let runner = Self {
            scheduler,
            notify: notify.clone(),
            rx,
        };
        let handle = SchedulerHandle { tx, notify };
        (runner, handle)
    }

    pub async fn run(mut self) {
        loop {
            let mut did_work = false;

            while let Ok(cmd) = self.rx.try_recv() {
                did_work = true;
                if let Err(e) = self.handle_command(cmd).await {
                    tracing::error!("Scheduler command failed: {}", e);
                }
            }

            if !did_work {
                match self.run_pending_running_round().await {
                    Ok(ran) => {
                        did_work = ran;
                    }
                    Err(e) => {
                        tracing::error!("Scheduler pending run failed: {}", e);
                        did_work = true;
                    }
                }
            }

            if !did_work {
                self.notify.notified().await;
            }
        }
    }

    async fn handle_command(&self, cmd: SchedulerCommand) -> Result<()> {
        match cmd {
            SchedulerCommand::RunRound(id) => self.scheduler.run_round(id).await,
            SchedulerCommand::RerunRound(id) => self.scheduler.rerun_round(id).await,
            SchedulerCommand::ScheduleUnflagged(id) => self.scheduler.schedule_unflagged_round(id).await,
            SchedulerCommand::RunPending(id) => self.scheduler.run_pending_round(id).await,
        }
    }

    async fn run_pending_running_round(&self) -> Result<bool> {
        let rounds = self.scheduler.db.get_active_rounds().await?;
        let Some(round_id) = select_running_round_id(&rounds) else {
            return Ok(false);
        };
        let pending = self.scheduler.db.get_pending_jobs(round_id).await?;
        if pending.is_empty() {
            return Ok(false);
        }
        self.scheduler.run_pending_round(round_id).await?;
        Ok(true)
    }
}

impl SchedulerHandle {
    pub fn send(&self, cmd: SchedulerCommand) -> Result<(), mpsc::error::SendError<SchedulerCommand>> {
        let res = self.tx.send(cmd);
        self.notify.notify_one();
        res
    }

    pub fn notify(&self) {
        self.notify.notify_one();
    }
}

pub struct Scheduler {
    db: Database,
    executor: Arc<Executor>,
    tx: broadcast::Sender<WsMessage>,
}

impl Scheduler {
    pub fn new(db: Database, executor: Arc<Executor>, tx: broadcast::Sender<WsMessage>) -> Self {
        Self { db, executor, tx }
    }

    pub fn calculate_priority(challenge_priority: i32, team_priority: i32, sequence: i32, override_priority: Option<i32>) -> i32 {
        override_priority.unwrap_or_else(|| challenge_priority + team_priority * 100 - sequence * 10000)
    }

    pub async fn generate_round(&self) -> Result<i32> {
        let round = self.db.create_round().await?;
        let challenges = self.db.list_challenges().await?;
        let teams = self.db.list_teams().await?;
        
        let mut jobs = Vec::new();
        
        for challenge in challenges.iter().filter(|c| c.enabled) {
            for team in &teams {
                let runs = self.db.list_exploit_runs(Some(challenge.id), Some(team.id)).await?;
                for run in runs {
                    let priority = Self::calculate_priority(challenge.priority, team.priority, run.sequence, run.priority);
                    jobs.push((run.id, team.id, priority));
                }
            }
        }

        jobs.sort_by(|a, b| b.2.cmp(&a.2)); // Higher priority first

        for (run_id, team_id, priority) in jobs {
            self.db.create_job(round.id, run_id, team_id, priority).await?;
        }

        Ok(round.id)
    }

    pub async fn create_round(&self) -> Result<i32> {
        let round_id = self.generate_round().await?;
        if let Ok(round) = self.db.get_round(round_id).await {
            broadcast(&self.tx, "round_created", &round);
        }
        let settings = load_executor_settings(&self.db).await;
        let cm = self.executor.container_manager.clone();
        cm.set_concurrent_create_limit(settings.concurrent_create_limit);
        tokio::spawn(async move {
            if let Err(e) = cm.prewarm_for_round(settings.concurrent_limit).await {
                tracing::error!("Prewarm failed: {}", e);
            }
        });
        Ok(round_id)
    }

    pub async fn run_round(&self, round_id: i32) -> Result<()> {
        // Stop running jobs from older rounds and check for flags
        self.stop_running_jobs_with_flag_check().await;

        // Skip older pending rounds and finish older running rounds
        if let Ok(rounds) = self.db.get_active_rounds().await {
            let plan = rounds_to_finalize(&rounds, round_id);
            for rid in plan.skip_pending_ids {
                let _ = self.db.skip_pending_jobs_for_round(rid).await;
                let _ = self.db.skip_round(rid).await;
                if let Ok(r) = self.db.get_round(rid).await {
                    broadcast(&self.tx, "round_updated", &r);
                }
            }
            for rid in plan.finish_running_ids {
                let _ = self.db.skip_pending_jobs_for_round(rid).await;
                let _ = self.db.finish_round(rid).await;
                if let Ok(r) = self.db.get_round(rid).await {
                    broadcast(&self.tx, "round_updated", &r);
                }
            }
        }

        // Mark round as running
        self.db.start_round(round_id).await?;
        if let Ok(round) = self.db.get_round(round_id).await {
            broadcast(&self.tx, "round_updated", &round);
        }

        // Health check containers before round
        self.executor.container_manager.health_check().await?;

        self.execute_pending_jobs(round_id).await
    }

    pub async fn run_pending_round(&self, round_id: i32) -> Result<()> {
        let round = self.db.get_round(round_id).await?;
        if round.status != "running" {
            return Err(anyhow::anyhow!("Round {} is not running", round_id));
        }
        self.execute_pending_jobs(round_id).await
    }

    pub async fn rerun_round(&self, round_id: i32) -> Result<()> {
        self.stop_running_jobs_with_flag_check().await;

        if let Ok(rounds) = self.db.list_rounds().await {
            for rid in rounds_to_reset_after(&rounds, round_id) {
                let _ = self.db.reset_jobs_for_round(rid).await;
                let _ = self.db.reset_round(rid).await;
                if let Ok(r) = self.db.get_round(rid).await {
                    broadcast(&self.tx, "round_updated", &r);
                }
            }
        }

        let _ = self.db.reset_jobs_for_round(round_id).await;
        let _ = self.db.reset_round(round_id).await;
        if let Ok(r) = self.db.get_round(round_id).await {
            broadcast(&self.tx, "round_updated", &r);
        }

        self.run_round(round_id).await?;
        Ok(())
    }

    pub async fn schedule_unflagged_round(&self, round_id: i32) -> Result<()> {
        self.stop_running_jobs_with_flag_check().await;

        let _ = self.db.reset_unflagged_jobs_for_round(round_id).await;
        let _ = self.db.reset_round(round_id).await;
        if let Ok(r) = self.db.get_round(round_id).await {
            broadcast(&self.tx, "round_updated", &r);
        }

        self.run_round(round_id).await?;
        Ok(())
    }

    async fn stop_running_jobs_with_flag_check(&self) {
        let settings = load_job_settings(&self.db).await;
        let executor = self.executor.clone();
        if let Ok(jobs) = self.db.kill_running_jobs().await {
            for job in jobs {
                let stdout = job.stdout.as_deref().unwrap_or("");
                let stderr = job.stderr.as_deref().unwrap_or("");
                let combined = if stderr.is_empty() {
                    stdout.to_string()
                } else if stdout.is_empty() {
                    stderr.to_string()
                } else {
                    format!("{}\n{}", stdout, stderr)
                };
                let flags = Executor::extract_flags(&combined, None, settings.max_flags);
                let has_flag = !flags.is_empty();
                let _ = executor.stop_job_with_flags(job.id, has_flag, "stopped by new round").await;
            }
        }
    }

    pub async fn get_jobs(&self, round_id: i32) -> Result<Vec<ExploitJob>> {
        self.db.list_jobs(round_id).await
    }

    async fn execute_pending_jobs(&self, round_id: i32) -> Result<()> {
        // Health check containers before scheduling
        self.executor.container_manager.health_check().await?;

        let settings = load_executor_settings(&self.db).await;
        self.executor.container_manager.set_concurrent_create_limit(settings.concurrent_create_limit);
        let concurrent_limit = settings.concurrent_limit;
        let worker_timeout = settings.worker_timeout;
        let max_flags = settings.max_flags;
        let skip_on_flag = settings.skip_on_flag;
        let sequential_per_target = settings.sequential_per_target;

        let jobs = self.db.get_pending_jobs(round_id).await?;
        let semaphore = Arc::new(Semaphore::new(concurrent_limit));
        let target_locks: Arc<Mutex<HashMap<(i32, i32), Arc<Mutex<()>>>>> = Arc::new(Mutex::new(HashMap::new()));

        let mut join_set = JoinSet::new();
        let base_executor = self.executor.clone();
        let db = self.db.clone();
        let tx = self.tx.clone();

        for job in jobs {
            let permit = semaphore.clone().acquire_owned().await?;
            let db = db.clone();
            let tx = tx.clone();
            let executor = base_executor.clone();
            let target_locks = target_locks.clone();

            join_set.spawn(async move {
                let _permit = permit;

                let ctx = match build_job_context(&db, job.id).await {
                    Ok(ctx) => ctx,
                    Err(JobContextError::NotPending) => return,
                    Err(JobContextError::MissingExploitRunId) => {
                        finish_job_and_broadcast(&db, &tx, job.id, "error", None, Some("Job missing exploit_run_id"), 0).await;
                        return;
                    }
                    Err(JobContextError::MissingConnectionInfo) => {
                        finish_job_and_broadcast(&db, &tx, job.id, "error", None, Some("No connection info (missing IP or port)"), 0).await;
                        return;
                    }
                    Err(JobContextError::Db(e)) => {
                        log_job_error(job.id, &e);
                        return;
                    }
                };

                if should_skip_job(&db, &tx, &ctx, skip_on_flag, round_id).await {
                    return;
                }

                let target_lock = get_target_lock(&target_locks, sequential_per_target, (ctx.challenge.id, ctx.team.id)).await;
                let _target_guard = match &target_lock {
                    Some(lock) => Some(lock.lock().await),
                    None => None,
                };

                if should_skip_job(&db, &tx, &ctx, skip_on_flag, round_id).await {
                    return;
                }

                let delay = stagger_delay_ms(ctx.job.id);
                tokio::time::sleep(Duration::from_millis(delay)).await;

                let timeout = compute_timeout(ctx.exploit.timeout_secs, worker_timeout);

                match executor.execute_job(&ctx.job, &ctx.run, &ctx.exploit, &ctx.conn, ctx.challenge.flag_regex.as_deref(), timeout, max_flags).await {
                    Ok(result) => {
                        for flag in result.flags {
                            if let Ok(f) = db.create_flag(ctx.job.id, round_id, ctx.challenge.id, ctx.team.id, &flag).await {
                                broadcast(&tx, "flag_created", &f);
                            }
                        }
                    }
                    Err(e) => {
                        log_job_error(ctx.job.id, &e);
                        finish_job_and_broadcast(&db, &tx, ctx.job.id, "error", None, Some(&e.to_string()), 0).await;
                    }
                }
            });
        }

        while let Some(result) = join_set.join_next().await {
            if let Err(e) = result {
                tracing::error!("Job task failed: {}", e);
            }
        }

        Ok(())
    }
}

struct RoundFinalizePlan {
    skip_pending_ids: Vec<i32>,
    finish_running_ids: Vec<i32>,
}

fn rounds_to_finalize(rounds: &[Round], current_id: i32) -> RoundFinalizePlan {
    let mut skip_pending_ids = Vec::new();
    let mut finish_running_ids = Vec::new();
    for round in rounds {
        if round.id < current_id {
            if round.status == "pending" {
                skip_pending_ids.push(round.id);
            } else {
                finish_running_ids.push(round.id);
            }
        }
    }
    RoundFinalizePlan { skip_pending_ids, finish_running_ids }
}

fn rounds_to_reset_after(rounds: &[Round], id: i32) -> Vec<i32> {
    rounds.iter().filter(|r| r.id > id).map(|r| r.id).collect()
}

pub fn select_running_round_id(rounds: &[Round]) -> Option<i32> {
    rounds.iter().find(|r| r.status == "running").map(|r| r.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn test_calculate_priority_default() {
        assert_eq!(Scheduler::calculate_priority(5, 3, 2, None), -19695);
    }

    #[test]
    fn test_calculate_priority_override() {
        assert_eq!(Scheduler::calculate_priority(5, 3, 2, Some(999)), 999);
    }

    fn make_round(id: i32, status: &str) -> Round {
        Round {
            id,
            started_at: Utc.timestamp_opt(0, 0).single().unwrap(),
            finished_at: None,
            status: status.to_string(),
        }
    }

    #[test]
    fn rounds_to_finalize_splits_pending_and_running() {
        let rounds = vec![
            make_round(1, "pending"),
            make_round(2, "running"),
            make_round(0, "finished"),
            make_round(3, "pending"),
        ];
        let plan = rounds_to_finalize(&rounds, 3);
        assert_eq!(plan.skip_pending_ids, vec![1]);
        assert_eq!(plan.finish_running_ids, vec![2, 0]);
    }

    #[test]
    fn rounds_to_reset_after_filters_ids() {
        let rounds = vec![
            make_round(1, "pending"),
            make_round(2, "running"),
            make_round(3, "finished"),
        ];
        let ids = rounds_to_reset_after(&rounds, 2);
        assert_eq!(ids, vec![3]);
    }

    #[test]
    fn select_running_round_id_picks_first_running() {
        let rounds = vec![
            make_round(1, "pending"),
            make_round(2, "running"),
            make_round(3, "running"),
        ];
        assert_eq!(select_running_round_id(&rounds), Some(2));
    }
}
