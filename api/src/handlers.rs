use crate::events::WsMessage;
use crate::AppState;
use axum::{extract::{Path, Query, State, ws::{WebSocket, WebSocketUpgrade}}, Json, response::Response};
use futures_util::StreamExt;
use mazuadm_core::*;
use mazuadm_core::scheduler::Scheduler;
use mazuadm_core::executor::Executor;
use mazuadm_core::container_manager::ContainerManager;
use std::sync::Arc;
use serde::Deserialize;

type S = State<Arc<AppState>>;
type R<T> = Result<Json<T>, String>;

fn err<E: std::fmt::Display>(e: E) -> String { e.to_string() }

fn broadcast<T: serde::Serialize>(s: &AppState, msg_type: &str, data: &T) {
    let _ = s.tx.send(WsMessage::new(msg_type, data));
}

fn should_continue_ws(err: tokio::sync::broadcast::error::RecvError) -> bool {
    matches!(err, tokio::sync::broadcast::error::RecvError::Lagged(_))
}

fn spawn_round_runner(executor: Executor, round_id: i32) {
    tokio::spawn(async move {
        if let Err(e) = executor.run_round(round_id).await {
            tracing::error!("Round {} failed: {}", round_id, e);
        }
    });
}

fn spawn_job_runner(db: Database, tx: tokio::sync::broadcast::Sender<WsMessage>, job_id: i32, exploit_run_id: i32, team_id: i32) {
    tokio::spawn(async move {
        macro_rules! fail {
            ($msg:expr) => {{
                let _ = db.finish_job(job_id, "error", None, Some($msg), 0).await;
                if let Ok(j) = db.get_job(job_id).await { let _ = tx.send(WsMessage::new("job_updated", &j)); }
                return;
            }};
        }
        let executor = match Executor::new(db.clone(), tx.clone()) { Ok(e) => e, Err(e) => fail!(&e.to_string()) };
        let run = match db.get_exploit_run(exploit_run_id).await { Ok(r) => r, Err(e) => fail!(&e.to_string()) };
        let exploit = match db.get_exploit(run.exploit_id).await { Ok(e) => e, Err(e) => fail!(&e.to_string()) };
        let challenge = match db.get_challenge(run.challenge_id).await { Ok(c) => c, Err(e) => fail!(&e.to_string()) };
        let team = match db.get_team(team_id).await { Ok(t) => t, Err(e) => fail!(&e.to_string()) };
        let relations = match db.list_relations(challenge.id).await { Ok(r) => r, Err(e) => fail!(&e.to_string()) };
        let rel = relations.iter().find(|r| r.team_id == team.id);
        let conn = match rel.and_then(|r| r.connection_info(&challenge, &team)) { Some(c) => c, None => fail!("No connection info") };
        let job = match db.get_job(job_id).await { Ok(j) => j, Err(e) => fail!(&e.to_string()) };
        let round_id = job.round_id;
        let settings = load_job_settings(&db).await;
        let timeout = compute_timeout(exploit.timeout_secs, settings.worker_timeout);
        match executor.execute_job(&job, &run, &exploit, &conn, challenge.flag_regex.as_deref(), timeout, settings.max_flags).await {
            Ok(result) => {
                for flag in result.flags {
                    let f = if let Some(rid) = round_id {
                        db.create_flag(job_id, rid, challenge.id, team.id, &flag).await
                    } else {
                        db.create_adhoc_flag(job_id, challenge.id, team.id, &flag).await
                    };
                    if let Ok(f) = f {
                        let _ = tx.send(WsMessage::new("flag_created", &f));
                    }
                }
            }
            Err(e) => {
                let _ = db.finish_job(job_id, "error", None, Some(&e.to_string()), 0).await;
            }
        }
    });
}

// WebSocket handler
pub async fn ws_handler(ws: WebSocketUpgrade, State(s): S) -> Response {
    ws.on_upgrade(move |socket| handle_ws(socket, s))
}

async fn handle_ws(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();
    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(m) => {
                        let text = serde_json::to_string(&m).unwrap_or_default();
                        if socket.send(axum::extract::ws::Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(err) => {
                        if !should_continue_ws(err) {
                            break;
                        }
                    }
                }
            }
            msg = socket.next() => {
                match msg {
                    Some(Ok(_)) => {} // ignore client messages
                    _ => break,
                }
            }
        }
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub challenge_id: Option<i32>,
    pub team_id: Option<i32>,
    pub round_id: Option<i32>,
}

struct RoundFinalizePlan {
    skip_pending_ids: Vec<i32>,
    finish_running_ids: Vec<i32>,
}

struct JobSettings {
    worker_timeout: u64,
    max_flags: usize,
}

fn parse_setting_u64(value: Option<String>, default: u64) -> u64 {
    value.and_then(|v| v.parse().ok()).unwrap_or(default)
}

fn parse_setting_usize(value: Option<String>, default: usize) -> usize {
    value.and_then(|v| v.parse().ok()).unwrap_or(default)
}

async fn load_job_settings(db: &Database) -> JobSettings {
    let worker_timeout = parse_setting_u64(db.get_setting("worker_timeout").await.ok(), 60);
    let max_flags = parse_setting_usize(db.get_setting("max_flags_per_job").await.ok(), 50);
    JobSettings { worker_timeout, max_flags }
}

fn compute_timeout(exploit_timeout_secs: i32, worker_timeout: u64) -> u64 {
    if exploit_timeout_secs > 0 {
        exploit_timeout_secs as u64
    } else {
        worker_timeout
    }
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

fn select_running_round_id(rounds: &[Round]) -> Option<i32> {
    rounds.iter().find(|r| r.status == "running").map(|r| r.id)
}

// Challenges
pub async fn list_challenges(State(s): S) -> R<Vec<Challenge>> {
    s.db.list_challenges().await.map(Json).map_err(err)
}

pub async fn create_challenge(State(s): S, Json(c): Json<CreateChallenge>) -> R<Challenge> {
    let challenge = s.db.create_challenge(c).await.map_err(err)?;
    s.db.ensure_relations(challenge.id).await.map_err(err)?;
    broadcast(&s, "challenge_created", &challenge);
    Ok(Json(challenge))
}

pub async fn update_challenge(State(s): S, Path(id): Path<i32>, Json(c): Json<CreateChallenge>) -> R<Challenge> {
    let challenge = s.db.update_challenge(id, c).await.map_err(err)?;
    broadcast(&s, "challenge_updated", &challenge);
    Ok(Json(challenge))
}

pub async fn delete_challenge(State(s): S, Path(id): Path<i32>) -> R<String> {
    s.db.delete_challenge(id).await.map_err(err)?;
    broadcast(&s, "challenge_deleted", &id);
    Ok(Json("ok".to_string()))
}

pub async fn set_challenge_enabled(State(s): S, Path((id, enabled)): Path<(i32, bool)>) -> R<String> {
    s.db.set_challenge_enabled(id, enabled).await.map_err(err)?;
    let challenge = s.db.get_challenge(id).await.map_err(err)?;
    broadcast(&s, "challenge_updated", &challenge);
    Ok(Json("ok".to_string()))
}

// Teams
pub async fn list_teams(State(s): S) -> R<Vec<Team>> {
    s.db.list_teams().await.map(Json).map_err(err)
}

pub async fn create_team(State(s): S, Json(t): Json<CreateTeam>) -> R<Team> {
    let team = s.db.create_team(t).await.map_err(err)?;
    for c in s.db.list_challenges().await.map_err(err)? {
        let _ = s.db.create_relation(c.id, team.id, None, None).await;
    }
    broadcast(&s, "team_created", &team);
    Ok(Json(team))
}

pub async fn update_team(State(s): S, Path(id): Path<i32>, Json(t): Json<CreateTeam>) -> R<Team> {
    let team = s.db.update_team(id, t).await.map_err(err)?;
    broadcast(&s, "team_updated", &team);
    Ok(Json(team))
}

pub async fn delete_team(State(s): S, Path(id): Path<i32>) -> R<String> {
    s.db.delete_team(id).await.map_err(err)?;
    broadcast(&s, "team_deleted", &id);
    Ok(Json("ok".to_string()))
}

// Exploits
pub async fn list_exploits(State(s): S, Query(q): Query<ListQuery>) -> R<Vec<Exploit>> {
    s.db.list_exploits(q.challenge_id).await.map(Json).map_err(err)
}

pub async fn create_exploit(State(s): S, Json(e): Json<CreateExploit>) -> R<Exploit> {
    let auto_add = e.auto_add.clone();
    let insert_into_rounds = e.insert_into_rounds;
    let exploit = s.db.create_exploit(e).await.map_err(err)?;
    
    if let Some(mode) = auto_add {
        if mode == "start" || mode == "end" {
            let teams = s.db.list_teams().await.map_err(err)?;
            for team in teams {
                let runs = s.db.list_exploit_runs(Some(exploit.challenge_id), Some(team.id)).await.map_err(err)?;
                let seq = if mode == "start" {
                    runs.iter().map(|r| r.sequence).min().unwrap_or(0) - 1
                } else {
                    runs.iter().map(|r| r.sequence).max().unwrap_or(-1) + 1
                };
                if let Ok(run) = s.db.create_exploit_run(CreateExploitRun {
                    exploit_id: exploit.id,
                    challenge_id: exploit.challenge_id,
                    team_id: team.id,
                    priority: None,
                    sequence: Some(seq),
                }).await {
                    broadcast(&s, "exploit_run_created", &run);
                }
            }
        }
    }

    // Insert jobs into active rounds if requested
    if insert_into_rounds.unwrap_or(false) {
        if let Ok(rounds) = s.db.get_active_rounds().await {
            let runs = s.db.list_exploit_runs(Some(exploit.challenge_id), None).await.unwrap_or_default();
            let exploit_runs: Vec<_> = runs.iter().filter(|r| r.exploit_id == exploit.id).collect();
            for round in rounds {
                for run in &exploit_runs {
                    if let Ok(job) = s.db.create_job(round.id, run.id, run.team_id, 0).await {
                        broadcast(&s, "job_created", &job);
                    }
                }
            }
        }
    }
    
    // Pre-warm containers for enabled exploit
    if exploit.enabled {
        let cm = ContainerManager::new(s.db.clone()).map_err(err)?;
        let _ = cm.ensure_containers(exploit.id).await;
    }
    
    broadcast(&s, "exploit_created", &exploit);
    Ok(Json(exploit))
}

pub async fn update_exploit(State(s): S, Path(id): Path<i32>, Json(e): Json<UpdateExploit>) -> R<Exploit> {
    let was_enabled = s.db.get_exploit(id).await.map(|e| e.enabled).unwrap_or(false);
    let exploit = s.db.update_exploit(id, e).await.map_err(err)?;
    
    let cm = ContainerManager::new(s.db.clone()).map_err(err)?;
    if exploit.enabled && !was_enabled {
        let _ = cm.ensure_containers(id).await;
    } else if !exploit.enabled && was_enabled {
        let _ = cm.destroy_exploit_containers(id).await;
    }
    
    broadcast(&s, "exploit_updated", &exploit);
    Ok(Json(exploit))
}

pub async fn delete_exploit(State(s): S, Path(id): Path<i32>) -> R<String> {
    let cm = ContainerManager::new(s.db.clone()).map_err(err)?;
    let _ = cm.destroy_exploit_containers(id).await;
    
    s.db.delete_exploit(id).await.map_err(err)?;
    broadcast(&s, "exploit_deleted", &id);
    Ok(Json("ok".to_string()))
}

// Exploit Runs
pub async fn list_exploit_runs(State(s): S, Query(q): Query<ListQuery>) -> R<Vec<ExploitRun>> {
    s.db.list_exploit_runs(q.challenge_id, q.team_id).await.map(Json).map_err(err)
}

pub async fn create_exploit_run(State(s): S, Json(r): Json<CreateExploitRun>) -> R<ExploitRun> {
    let run = s.db.create_exploit_run(r).await.map_err(err)?;
    broadcast(&s, "exploit_run_created", &run);
    Ok(Json(run))
}

#[derive(Deserialize)]
pub struct UpdateExploitRun {
    pub priority: Option<i32>,
    pub sequence: Option<i32>,
    pub enabled: Option<bool>,
}

pub async fn update_exploit_run(State(s): S, Path(id): Path<i32>, Json(u): Json<UpdateExploitRun>) -> R<ExploitRun> {
    let run = s.db.update_exploit_run(id, u.priority, u.sequence, u.enabled).await.map_err(err)?;
    broadcast(&s, "exploit_run_updated", &run);
    Ok(Json(run))
}

pub async fn delete_exploit_run(State(s): S, Path(id): Path<i32>) -> R<String> {
    s.db.delete_exploit_run(id).await.map_err(err)?;
    broadcast(&s, "exploit_run_deleted", &id);
    Ok(Json("ok".to_string()))
}

#[derive(Deserialize, serde::Serialize)]
pub struct ReorderItem {
    pub id: i32,
    pub sequence: i32,
}

pub async fn reorder_exploit_runs(State(s): S, Json(items): Json<Vec<ReorderItem>>) -> R<String> {
    s.db.reorder_exploit_runs(&items.iter().map(|i| (i.id, i.sequence)).collect::<Vec<_>>()).await.map_err(err)?;
    broadcast(&s, "exploit_runs_reordered", &items);
    Ok(Json("ok".to_string()))
}

// Rounds
pub async fn list_rounds(State(s): S) -> R<Vec<Round>> {
    s.db.list_rounds().await.map(Json).map_err(err)
}

pub async fn create_round(State(s): S) -> R<i32> {
    let scheduler = Scheduler::new(s.db.clone());
    let round_id = scheduler.generate_round().await.map_err(err)?;
    let round = s.db.get_round(round_id).await.map_err(err)?;
    broadcast(&s, "round_created", &round);
    Ok(Json(round_id))
}

pub async fn run_round(State(s): S, Path(id): Path<i32>) -> R<String> {
    // Stop running jobs from older rounds and check for flags
    stop_running_jobs_with_flag_check(&s).await;
    
    // Skip older pending rounds and finish older running rounds
    if let Ok(rounds) = s.db.get_active_rounds().await {
        let plan = rounds_to_finalize(&rounds, id);
        for round_id in plan.skip_pending_ids {
            let _ = s.db.skip_pending_jobs_for_round(round_id).await;
            let _ = s.db.skip_round(round_id).await;
            if let Ok(r) = s.db.get_round(round_id).await {
                broadcast(&s, "round_updated", &r);
            }
        }
        for round_id in plan.finish_running_ids {
            let _ = s.db.skip_pending_jobs_for_round(round_id).await;
            let _ = s.db.finish_round(round_id).await;
            if let Ok(r) = s.db.get_round(round_id).await {
                broadcast(&s, "round_updated", &r);
            }
        }
    }
    
    let tx = s.tx.clone();
    let executor = Executor::new(s.db.clone(), tx).map_err(err)?;
    spawn_round_runner(executor, id);
    Ok(Json("started".to_string()))
}

async fn stop_running_jobs_with_flag_check(s: &AppState) {
    let settings = load_job_settings(&s.db).await;
    if let Ok(jobs) = s.db.kill_running_jobs().await {
        for job in jobs {
            let stdout = job.stdout.as_deref().unwrap_or("");
            let flags = Executor::extract_flags(stdout, None, settings.max_flags);
            let has_flag = !flags.is_empty();
            let _ = s.db.mark_job_stopped(job.id, has_flag).await;
            if let Ok(j) = s.db.get_job(job.id).await {
                broadcast(s, "job_updated", &j);
            }
        }
    }
}

pub async fn rerun_round(State(s): S, Path(id): Path<i32>) -> R<String> {
    // Stop running jobs and check for flags
    stop_running_jobs_with_flag_check(&s).await;
    
    // Reset all rounds after this one to pending
    if let Ok(rounds) = s.db.list_rounds().await {
        for round_id in rounds_to_reset_after(&rounds, id) {
            let _ = s.db.reset_jobs_for_round(round_id).await;
            let _ = s.db.reset_round(round_id).await;
            if let Ok(r) = s.db.get_round(round_id).await {
                broadcast(&s, "round_updated", &r);
            }
        }
    }
    
    // Reset this round
    let _ = s.db.reset_jobs_for_round(id).await;
    let _ = s.db.reset_round(id).await;
    if let Ok(r) = s.db.get_round(id).await {
        broadcast(&s, "round_updated", &r);
    }
    
    // Run the round
    let tx = s.tx.clone();
    let executor = Executor::new(s.db.clone(), tx).map_err(err)?;
    spawn_round_runner(executor, id);
    Ok(Json("restarted".to_string()))
}

// Jobs
pub async fn list_jobs(State(s): S, Query(q): Query<ListQuery>) -> R<Vec<ExploitJob>> {
    let round_id = q.round_id.unwrap_or(0);
    s.db.list_jobs(round_id).await.map(Json).map_err(err)
}

#[derive(Deserialize, serde::Serialize)]
pub struct ReorderJobItem {
    pub id: i32,
    pub priority: i32,
}

pub async fn reorder_jobs(State(s): S, Json(items): Json<Vec<ReorderJobItem>>) -> R<String> {
    s.db.reorder_jobs(&items.iter().map(|i| (i.id, i.priority)).collect::<Vec<_>>()).await.map_err(err)?;
    for item in &items {
        if let Ok(job) = s.db.get_job(item.id).await {
            broadcast(&s, "job_updated", &job);
        }
    }
    Ok(Json("ok".to_string()))
}

#[derive(Deserialize)]
pub struct RunSingleJobRequest {
    pub exploit_run_id: i32,
    pub team_id: i32,
}

pub async fn run_single_job(State(s): S, Json(req): Json<RunSingleJobRequest>) -> R<ExploitJob> {
    let run = s.db.get_exploit_run(req.exploit_run_id).await.map_err(err)?;
    
    // Find current running round, or create ad-hoc job if none
    let job = if let Ok(rounds) = s.db.get_active_rounds().await {
        if let Some(round_id) = select_running_round_id(&rounds) {
            s.db.create_job(round_id, run.id, req.team_id, 0).await.map_err(err)?
        } else {
            s.db.create_adhoc_job(run.id, req.team_id).await.map_err(err)?
        }
    } else {
        s.db.create_adhoc_job(run.id, req.team_id).await.map_err(err)?
    };
    broadcast(&s, "job_created", &job);
    
    spawn_job_runner(s.db.clone(), s.tx.clone(), job.id, req.exploit_run_id, req.team_id);
    
    Ok(Json(job))
}

pub async fn run_existing_job(State(s): S, Path(job_id): Path<i32>) -> R<ExploitJob> {
    let job = s.db.get_job(job_id).await.map_err(err)?;
    let exploit_run_id = job.exploit_run_id.ok_or("Job has no exploit_run_id".to_string())?;
    
    // Reset job status to pending
    s.db.update_job_status(job_id, "pending", false).await.map_err(err)?;
    let job = s.db.get_job(job_id).await.map_err(err)?;
    broadcast(&s, "job_updated", &job);
    
    spawn_job_runner(s.db.clone(), s.tx.clone(), job_id, exploit_run_id, job.team_id);
    
    Ok(Json(job))
}

// Flags
pub async fn list_flags(State(s): S, Query(q): Query<ListQuery>) -> R<Vec<Flag>> {
    s.db.list_flags(q.round_id).await.map(Json).map_err(err)
}

// Settings
pub async fn list_settings(State(s): S) -> R<Vec<Setting>> {
    s.db.list_settings().await.map(Json).map_err(err)
}

#[derive(Deserialize, serde::Serialize)]
pub struct UpdateSetting {
    pub key: String,
    pub value: String,
}

pub async fn update_setting(State(s): S, Json(u): Json<UpdateSetting>) -> R<String> {
    s.db.set_setting(&u.key, &u.value).await.map_err(err)?;
    broadcast(&s, "setting_updated", &u);
    Ok(Json("ok".to_string()))
}

// Containers
pub async fn list_containers(State(s): S, Query(q): Query<ListQuery>) -> R<Vec<ExploitContainer>> {
    match q.challenge_id {
        Some(eid) => s.db.get_exploit_containers(eid).await.map(Json).map_err(err),
        None => s.db.list_all_containers().await.map(Json).map_err(err),
    }
}

pub async fn get_container_runners(State(s): S, Path(id): Path<i32>) -> R<Vec<ExploitRunner>> {
    s.db.get_runners_for_container(id).await.map(Json).map_err(err)
}

pub async fn delete_container(State(s): S, Path(id): Path<i32>) -> R<String> {
    let cm = ContainerManager::new(s.db.clone()).map_err(err)?;
    cm.destroy_container(id).await.map_err(err)?;
    Ok(Json("ok".to_string()))
}

pub async fn restart_container(State(s): S, Path(id): Path<i32>) -> R<String> {
    let cm = ContainerManager::new(s.db.clone()).map_err(err)?;
    let container = s.db.get_container(id).await.map_err(err)?;
    let exploit = s.db.get_exploit(container.exploit_id).await.map_err(err)?;
    let runners = s.db.get_runners_for_container(id).await.map_err(err)?;
    cm.destroy_container(id).await.map_err(err)?;
    let new_container = cm.spawn_container(&exploit).await.map_err(err)?;
    for r in runners {
        let _ = s.db.create_exploit_runner(new_container.id, r.exploit_run_id, r.team_id).await;
    }
    Ok(Json("ok".to_string()))
}

// Relations
pub async fn list_relations(State(s): S, Path(challenge_id): Path<i32>) -> R<Vec<ChallengeTeamRelation>> {
    s.db.list_relations(challenge_id).await.map(Json).map_err(err)
}

pub async fn get_relation(State(s): S, Path((challenge_id, team_id)): Path<(i32, i32)>) -> R<Option<ChallengeTeamRelation>> {
    s.db.get_relation(challenge_id, team_id).await.map(Json).map_err(err)
}

#[derive(Deserialize)]
pub struct UpdateRelation {
    pub addr: Option<String>,
    pub port: Option<i32>,
}

pub async fn update_relation(State(s): S, Path((challenge_id, team_id)): Path<(i32, i32)>, Json(u): Json<UpdateRelation>) -> R<ChallengeTeamRelation> {
    let rel = s.db.update_relation(challenge_id, team_id, u.addr, u.port).await.map_err(err)?;
    broadcast(&s, "relation_updated", &rel);
    Ok(Json(rel))
}

#[cfg(test)]
mod tests {
    use super::{
        compute_timeout,
        parse_setting_u64,
        parse_setting_usize,
        rounds_to_finalize,
        rounds_to_reset_after,
        select_running_round_id,
        should_continue_ws,
    };
    use mazuadm_core::Round;
    use chrono::{TimeZone, Utc};

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

    #[test]
    fn should_continue_ws_only_on_lagged() {
        assert!(should_continue_ws(tokio::sync::broadcast::error::RecvError::Lagged(1)));
        assert!(!should_continue_ws(tokio::sync::broadcast::error::RecvError::Closed));
    }

    #[test]
    fn parse_setting_u64_falls_back() {
        assert_eq!(parse_setting_u64(None, 60), 60);
        assert_eq!(parse_setting_u64(Some("bad".to_string()), 60), 60);
        assert_eq!(parse_setting_u64(Some("30".to_string()), 60), 30);
    }

    #[test]
    fn parse_setting_usize_falls_back() {
        assert_eq!(parse_setting_usize(None, 50), 50);
        assert_eq!(parse_setting_usize(Some("bad".to_string()), 50), 50);
        assert_eq!(parse_setting_usize(Some("25".to_string()), 50), 25);
    }

    #[test]
    fn compute_timeout_prefers_exploit() {
        assert_eq!(compute_timeout(10, 60), 10);
    }

    #[test]
    fn compute_timeout_falls_back_to_worker() {
        assert_eq!(compute_timeout(0, 60), 60);
    }
}
