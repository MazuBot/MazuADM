use crate::{ContainerInfo, Database, ExploitJob, Round, WsMessage};
use crate::executor::{
    build_job_context_or_finish,
    finish_job_and_broadcast,
    get_target_lock,
    log_job_error,
    should_skip_job,
    stagger_delay_ms,
    JobContext,
};
use crate::executor::Executor;
use crate::settings::{compute_timeout, load_executor_settings, load_job_settings};
use anyhow::Result;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use dashmap::DashMap;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex, Notify, OwnedMutexGuard, Semaphore};
use tokio::task::JoinSet;
use std::time::Duration;

fn broadcast<T: serde::Serialize>(tx: &broadcast::Sender<WsMessage>, msg_type: &str, data: &T) {
    let _ = tx.send(WsMessage::new(msg_type, data));
}

pub enum SchedulerCommand {
    RunRound(i32),
    RerunRound(i32),
    RerunUnflagged(i32),
    RunPending(i32),
    RefreshJob(i32),
    CreateRound { resp: oneshot::Sender<Result<i32>> },
    RunJobImmediately(i32),
    StopJob { job_id: i32, reason: String, resp: oneshot::Sender<Result<ExploitJob>> },
    EnsureContainers { exploit_id: i32, resp: oneshot::Sender<Result<()>> },
    DestroyExploitContainers { exploit_id: i32, resp: oneshot::Sender<Result<()>> },
    ListContainers { exploit_id: Option<i32>, resp: oneshot::Sender<Result<Vec<ContainerInfo>>> },
    RestartContainer { id: String, resp: oneshot::Sender<Result<()>> },
    DestroyContainer { id: String, resp: oneshot::Sender<Result<()>> },
}

#[derive(Clone, Debug)]
struct ScheduleSettings {
    worker_timeout: u64,
    max_flags: usize,
    skip_on_flag: bool,
    sequential_per_target: bool,
}

#[derive(Clone, Debug)]
struct PendingEntry {
    priority: i32,
    job_id: i32,
}

#[derive(serde::Serialize)]
struct JobsChangedPayload {
    round_id: i32,
    created: u64,
}

impl Ord for PendingEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => other.job_id.cmp(&self.job_id),
            ord => ord,
        }
    }
}

impl PartialOrd for PendingEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PendingEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.job_id == other.job_id
    }
}

impl Eq for PendingEntry {}

#[derive(Default)]
struct PendingQueue {
    round_id: Option<i32>,
    heap: BinaryHeap<PendingEntry>,
    jobs: HashMap<i32, ExploitJob>,
}

impl PendingQueue {
    fn reset(&mut self, round_id: i32, jobs: Vec<ExploitJob>) {
        self.round_id = Some(round_id);
        self.heap.clear();
        self.jobs.clear();
        for job in jobs.into_iter().filter(|j| j.status == "pending" && j.round_id == round_id) {
            self.heap.push(PendingEntry { priority: job.priority, job_id: job.id });
            self.jobs.insert(job.id, job);
        }
    }

    fn upsert(&mut self, job: ExploitJob) {
        if Some(job.round_id) != self.round_id {
            return;
        }
        if job.status == "pending" {
            self.heap.push(PendingEntry { priority: job.priority, job_id: job.id });
            self.jobs.insert(job.id, job);
        } else {
            self.jobs.remove(&job.id);
        }
    }

    fn pop_next(&mut self) -> Option<ExploitJob> {
        while let Some(entry) = self.heap.pop() {
            if let Some(job) = self.jobs.get(&entry.job_id) {
                if job.priority == entry.priority && job.status == "pending" {
                    return self.jobs.remove(&entry.job_id);
                }
            }
        }
        None
    }

    fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    fn len(&self) -> usize {
        self.jobs.len()
    }
}

enum TargetLockOutcome {
    NoLock,
    Acquired(OwnedMutexGuard<()>),
    Busy,
}

fn try_acquire_target_guard(
    target_locks: &DashMap<(i32, i32), Arc<Mutex<()>>>,
    sequential_per_target: bool,
    key: (i32, i32),
) -> TargetLockOutcome {
    if !sequential_per_target {
        return TargetLockOutcome::NoLock;
    }
    let Some(lock) = get_target_lock(target_locks, sequential_per_target, key) else {
        return TargetLockOutcome::NoLock;
    };
    match lock.try_lock_owned() {
        Ok(guard) => TargetLockOutcome::Acquired(guard),
        Err(_) => TargetLockOutcome::Busy,
    }
}

pub struct SchedulerRunner {
    scheduler: Scheduler,
    notify: Arc<Notify>,
    rx: mpsc::UnboundedReceiver<SchedulerCommand>,
    queue: PendingQueue,
    join_set: JoinSet<()>,
    immediate_join_set: JoinSet<()>,
    semaphore: Arc<Semaphore>,
    concurrent_limit: usize,
    running_jobs: usize,
    settings: Option<ScheduleSettings>,
    round_id: Option<i32>,
    target_locks: DashMap<(i32, i32), Arc<Mutex<()>>>,
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
            queue: PendingQueue::default(),
            join_set: JoinSet::new(),
            immediate_join_set: JoinSet::new(),
            semaphore: Arc::new(Semaphore::new(1)),
            concurrent_limit: 1,
            running_jobs: 0,
            settings: None,
            round_id: None,
            target_locks: DashMap::new(),
        };
        let handle = SchedulerHandle { tx, notify };
        (runner, handle)
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
                cmd = self.rx.recv() => {
                    match cmd {
                        Some(cmd) => {
                            if let Err(e) = self.handle_command(cmd).await {
                                tracing::error!("Scheduler command failed: {}", e);
                            }
                        }
                        None => break,
                    }
                }
                Some(result) = self.join_set.join_next(), if self.running_jobs > 0 => {
                    self.running_jobs = self.running_jobs.saturating_sub(1);
                    if let Err(e) = result {
                        tracing::error!("Job task failed: {}", e);
                    }
                    self.schedule_more().await;
                }
                Some(result) = self.immediate_join_set.join_next(), if !self.immediate_join_set.is_empty() => {
                    if let Err(e) = result {
                        tracing::error!("Immediate job task failed: {}", e);
                    }
                }
                _ = self.notify.notified() => {
                    self.schedule_more().await;
                }
            }
        }

        self.immediate_join_set.shutdown().await;
        self.join_set.shutdown().await;
    }

    async fn handle_command(&mut self, cmd: SchedulerCommand) -> Result<()> {
        match cmd {
            SchedulerCommand::RunRound(id) => {
                self.scheduler.run_round(id).await?;
                self.reset_queue(id).await?;
            }
            SchedulerCommand::RerunRound(id) => {
                self.scheduler.rerun_round(id).await?;
                self.reset_queue(id).await?;
            }
            SchedulerCommand::RerunUnflagged(id) => {
                self.scheduler.rerun_unflagged(id).await?;
                self.reset_queue(id).await?;
            }
            SchedulerCommand::RunPending(id) => {
                self.scheduler.run_pending_round(id).await?;
                self.reset_queue(id).await?;
            }
            SchedulerCommand::RefreshJob(id) => {
                self.refresh_job(id).await?;
            }
            SchedulerCommand::CreateRound { resp } => {
                let res = self.scheduler.create_round().await;
                let _ = resp.send(res);
            }
            SchedulerCommand::RunJobImmediately(job_id) => {
                let executor = self.executor_static();
                self.immediate_join_set.spawn(async move {
                    if let Err(e) = executor.run_job_immediately(job_id).await {
                        tracing::error!("Immediate job {} failed: {}", job_id, e);
                    }
                });
            }
            SchedulerCommand::StopJob { job_id, reason, resp } => {
                let res = self.scheduler.executor.stop_job(job_id, &reason).await;
                let _ = resp.send(res);
            }
            SchedulerCommand::EnsureContainers { exploit_id, resp } => {
                let cm = self.scheduler.executor.container_manager.clone();
                tokio::spawn(async move {
                    let res = cm.ensure_containers(exploit_id).await;
                    let _ = resp.send(res);
                });
            }
            SchedulerCommand::DestroyExploitContainers { exploit_id, resp } => {
                let cm = self.scheduler.executor.container_manager.clone();
                tokio::spawn(async move {
                    let res = cm.destroy_exploit_containers(exploit_id).await;
                    let _ = resp.send(res);
                });
            }
            SchedulerCommand::ListContainers { exploit_id, resp } => {
                let res = self.scheduler.executor.container_manager.list_containers().await
                    .map(|mut containers| {
                        if let Some(id) = exploit_id {
                            containers.retain(|c| c.exploit_id == id);
                        }
                        containers
                    });
                let _ = resp.send(res);
            }
            SchedulerCommand::RestartContainer { id, resp } => {
                let cm = self.scheduler.executor.container_manager.clone();
                tokio::spawn(async move {
                    let res = cm.restart_container_by_id(&id).await;
                    let _ = resp.send(res);
                });
            }
            SchedulerCommand::DestroyContainer { id, resp } => {
                let cm = self.scheduler.executor.container_manager.clone();
                tokio::spawn(async move {
                    let res = cm.destroy_container_by_id(&id).await;
                    let _ = resp.send(res);
                });
            }
        }
        self.schedule_more().await;
        Ok(())
    }

    fn executor_static(&self) -> &'static Executor {
        // Safety: SchedulerRunner owns Scheduler for the lifetime of the runner task.
        // We drain join sets before drop, so spawned tasks cannot outlive executor.
        unsafe { std::mem::transmute::<&Executor, &'static Executor>(&self.scheduler.executor) }
    }

    async fn reset_queue(&mut self, round_id: i32) -> Result<()> {
        let settings = load_executor_settings(&self.scheduler.db).await;
        self.scheduler.executor.container_manager.set_concurrent_create_limit(settings.concurrent_create_limit);
        self.scheduler.executor.container_manager.health_check().await?;
        let jobs = self.scheduler.db.get_pending_jobs(round_id).await?;
        self.queue.reset(round_id, jobs);
        self.round_id = Some(round_id);
        self.target_locks = DashMap::new();
        self.update_settings(settings);
        Ok(())
    }

    fn sync_semaphore(&self) {
        let desired_available = self.concurrent_limit.saturating_sub(self.running_jobs);
        let current_available = self.semaphore.available_permits();
        if current_available < desired_available {
            self.semaphore.add_permits(desired_available - current_available);
        } else if current_available > desired_available {
            let to_forget = current_available - desired_available;
            let _ = self.semaphore.forget_permits(to_forget);
        }
    }

    async fn refresh_job(&mut self, job_id: i32) -> Result<()> {
        let job = self.scheduler.db.get_job(job_id).await?;
        if let Some(round_id) = self.round_id {
            if job.round_id != round_id {
                return Ok(());
            }
            self.queue.upsert(job);
            return Ok(());
        }

        if job.status != "pending" {
            return Ok(());
        }
        let round = self.scheduler.db.get_round(job.round_id).await?;
        if round.status != "running" {
            return Ok(());
        }
        self.reset_queue(job.round_id).await?;
        Ok(())
    }

    fn update_settings(&mut self, settings: crate::settings::ExecutorSettings) {
        let concurrent_limit = settings.concurrent_limit.max(1);
        self.concurrent_limit = concurrent_limit;
        self.sync_semaphore();
        self.settings = Some(ScheduleSettings {
            worker_timeout: settings.worker_timeout,
            max_flags: settings.max_flags,
            skip_on_flag: settings.skip_on_flag,
            sequential_per_target: settings.sequential_per_target,
        });
    }

    async fn schedule_more(&mut self) {
        self.sync_semaphore();
        let Some(settings) = self.settings.clone() else { return; };
        let Some(round_id) = self.round_id else { return; };
        let mut deferred = Vec::new();

        while self.semaphore.available_permits() > 0 {
            let mut scanned = 0;
            let max_scan = self.queue.len();
            let mut selected: Option<(JobContext, Option<OwnedMutexGuard<()>>)> = None;

            while scanned < max_scan {
                let Some(job) = self.queue.pop_next() else { break; };
                scanned += 1;

                let db = self.scheduler.db.clone();
                let tx = self.scheduler.tx.clone();
                let ctx = match build_job_context_or_finish(&db, &tx, job.id).await {
                    Some(ctx) => ctx,
                    None => continue,
                };

                let target_guard = match try_acquire_target_guard(
                    &self.target_locks,
                    settings.sequential_per_target,
                    (ctx.challenge.id, ctx.team.id),
                ) {
                    TargetLockOutcome::NoLock => None,
                    TargetLockOutcome::Acquired(guard) => Some(guard),
                    TargetLockOutcome::Busy => {
                        deferred.push(ctx.job);
                        continue;
                    }
                };

                selected = Some((ctx, target_guard));
                break;
            }

            let Some((ctx, target_guard)) = selected else { break; };

            tracing::info!(
                "Scheduling job {} for round {} (challenge {}, team {}, priority {})",
                ctx.job.id,
                round_id,
                ctx.challenge.id,
                ctx.team.id,
                ctx.job.priority
            );
            if let Err(e) = self.scheduler.db.mark_job_scheduled(ctx.job.id).await {
                tracing::error!("Failed to mark job {} scheduled: {}", ctx.job.id, e);
            }

            let permit = match self.semaphore.clone().acquire_owned().await {
                Ok(permit) => permit,
                Err(_) => break,
            };

            self.running_jobs += 1;
            let db = self.scheduler.db.clone();
            let tx = self.scheduler.tx.clone();
            let executor = self.executor_static();
            let settings = settings.clone();

            self.join_set.spawn(async move {
                let _permit = permit;
                let _target_guard = target_guard;
                execute_one_job(db, tx, executor, _target_guard, ctx, round_id, settings).await;
            });
        }

        for job in deferred {
            self.queue.upsert(job);
        }

        if self.running_jobs == 0 && self.queue.is_empty() {
            self.round_id = None;
        }
    }
}

impl SchedulerHandle {
    pub fn send(&self, cmd: SchedulerCommand) -> Result<(), mpsc::error::SendError<SchedulerCommand>> {
        let res = self.tx.send(cmd);
        self.notify.notify_one();
        res
    }

    pub async fn create_round(&self) -> Result<i32> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.send(SchedulerCommand::CreateRound { resp: resp_tx })?;
        resp_rx.await.unwrap_or_else(|_| Err(anyhow::anyhow!("Scheduler response dropped")))
    }

    pub fn run_job_immediately(&self, job_id: i32) -> Result<(), mpsc::error::SendError<SchedulerCommand>> {
        self.send(SchedulerCommand::RunJobImmediately(job_id))
    }

    pub async fn stop_job(&self, job_id: i32, reason: &str) -> Result<ExploitJob> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.send(SchedulerCommand::StopJob { job_id, reason: reason.to_string(), resp: resp_tx })?;
        resp_rx.await.unwrap_or_else(|_| Err(anyhow::anyhow!("Scheduler response dropped")))
    }

    pub async fn ensure_containers(&self, exploit_id: i32) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.send(SchedulerCommand::EnsureContainers { exploit_id, resp: resp_tx })?;
        resp_rx.await.unwrap_or_else(|_| Err(anyhow::anyhow!("Scheduler response dropped")))
    }

    pub async fn destroy_exploit_containers(&self, exploit_id: i32) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.send(SchedulerCommand::DestroyExploitContainers { exploit_id, resp: resp_tx })?;
        resp_rx.await.unwrap_or_else(|_| Err(anyhow::anyhow!("Scheduler response dropped")))
    }

    pub async fn list_containers(&self, exploit_id: Option<i32>) -> Result<Vec<ContainerInfo>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.send(SchedulerCommand::ListContainers { exploit_id, resp: resp_tx })?;
        resp_rx.await.unwrap_or_else(|_| Err(anyhow::anyhow!("Scheduler response dropped")))
    }

    pub async fn restart_container(&self, id: String) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.send(SchedulerCommand::RestartContainer { id, resp: resp_tx })?;
        resp_rx.await.unwrap_or_else(|_| Err(anyhow::anyhow!("Scheduler response dropped")))
    }

    pub async fn destroy_container(&self, id: String) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.send(SchedulerCommand::DestroyContainer { id, resp: resp_tx })?;
        resp_rx.await.unwrap_or_else(|_| Err(anyhow::anyhow!("Scheduler response dropped")))
    }

    pub fn notify(&self) {
        self.notify.notify_one();
    }
}

pub struct Scheduler {
    db: Database,
    executor: Executor,
    tx: broadcast::Sender<WsMessage>,
}

impl Scheduler {
    pub fn new(db: Database, executor: Executor, tx: broadcast::Sender<WsMessage>) -> Self {
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
            self.db.create_job(round.id, run_id, team_id, priority, Some("new_round")).await?;
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
        tracing::info!("Round {} started", round_id);
        if let Ok(round) = self.db.get_round(round_id).await {
            broadcast(&self.tx, "round_updated", &round);
        }

        Ok(())
    }

    pub async fn run_pending_round(&self, round_id: i32) -> Result<()> {
        let round = self.db.get_round(round_id).await?;
        if round.status != "running" {
            return Err(anyhow::anyhow!("Round {} is not running", round_id));
        }
        Ok(())
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

    pub async fn rerun_unflagged(&self, round_id: i32) -> Result<()> {
        let created = self.db.clone_unflagged_jobs_for_round(round_id).await?;
        if created > 0 {
            broadcast(&self.tx, "jobs_changed", &JobsChangedPayload { round_id, created });
        }
        if let Ok(r) = self.db.get_round(round_id).await {
            broadcast(&self.tx, "round_updated", &r);
        }

        self.run_round(round_id).await?;
        Ok(())
    }

    async fn stop_running_jobs_with_flag_check(&self) {
        let settings = load_job_settings(&self.db).await;
        let executor = &self.executor;
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

}

async fn execute_one_job(
    db: Database,
    tx: broadcast::Sender<WsMessage>,
    executor: &Executor,
    _target_guard: Option<OwnedMutexGuard<()>>,
    ctx: JobContext,
    round_id: i32,
    settings: ScheduleSettings,
) {
    if should_skip_job(&db, &tx, &ctx, settings.skip_on_flag, round_id).await {
        return;
    }

    if should_skip_job(&db, &tx, &ctx, settings.skip_on_flag, round_id).await {
        return;
    }

    let delay = stagger_delay_ms(ctx.job.id);
    tokio::time::sleep(Duration::from_millis(delay)).await;

    let timeout = compute_timeout(ctx.exploit.timeout_secs, settings.worker_timeout);

    match executor.execute_job(&ctx.job, &ctx.run, &ctx.exploit, &ctx.conn, ctx.challenge.flag_regex.as_deref(), timeout, settings.max_flags).await {
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
}

#[cfg(test)]
mod target_lock_tests {
    use super::{get_target_lock, try_acquire_target_guard, TargetLockOutcome};
    use dashmap::DashMap;

    #[test]
    fn try_acquire_target_guard_skips_locked_targets() {
        let locks = DashMap::new();
        let lock = get_target_lock(&locks, true, (1, 1)).expect("lock");
        let _held = lock.try_lock_owned().expect("lock guard");

        assert!(matches!(
            try_acquire_target_guard(&locks, true, (1, 1)),
            TargetLockOutcome::Busy
        ));
        assert!(matches!(
            try_acquire_target_guard(&locks, true, (1, 2)),
            TargetLockOutcome::Acquired(_)
        ));
        assert!(matches!(
            try_acquire_target_guard(&locks, false, (1, 1)),
            TargetLockOutcome::NoLock
        ));
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

    fn make_job(id: i32, priority: i32, status: &str, round_id: i32) -> ExploitJob {
        ExploitJob {
            id,
            round_id,
            exploit_run_id: Some(1),
            team_id: 1,
            priority,
            status: status.to_string(),
            container_id: None,
            stdout: None,
            stderr: None,
            create_reason: None,
            duration_ms: None,
            schedule_at: None,
            started_at: None,
            finished_at: None,
            created_at: Utc.timestamp_opt(0, 0).single().unwrap(),
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

    #[test]
    fn pending_queue_orders_by_priority_then_id() {
        let mut queue = PendingQueue::default();
        let jobs = vec![
            make_job(2, 10, "pending", 1),
            make_job(1, 10, "pending", 1),
            make_job(3, 5, "pending", 1),
        ];
        queue.reset(1, jobs);
        assert_eq!(queue.pop_next().map(|j| j.id), Some(1));
        assert_eq!(queue.pop_next().map(|j| j.id), Some(2));
        assert_eq!(queue.pop_next().map(|j| j.id), Some(3));
        assert!(queue.pop_next().is_none());
    }

    #[test]
    fn pending_queue_upsert_updates_priority() {
        let mut queue = PendingQueue::default();
        queue.reset(1, vec![make_job(1, 1, "pending", 1)]);
        queue.upsert(make_job(1, 99, "pending", 1));
        assert_eq!(queue.pop_next().map(|j| j.priority), Some(99));
    }

    #[test]
    fn pending_queue_remove_on_non_pending() {
        let mut queue = PendingQueue::default();
        queue.reset(1, vec![make_job(1, 1, "pending", 1)]);
        queue.upsert(make_job(1, 1, "running", 1));
        assert!(queue.pop_next().is_none());
    }

    #[test]
    fn pending_queue_ignores_wrong_round() {
        let mut queue = PendingQueue::default();
        queue.reset(1, vec![]);
        queue.upsert(make_job(1, 10, "pending", 2));
        assert!(queue.pop_next().is_none());
    }
}
