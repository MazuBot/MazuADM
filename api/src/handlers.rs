use crate::events::WsMessage;
use crate::AppState;
use axum::{extract::{Path, Query, State, ws::{WebSocket, WebSocketUpgrade}}, Json, response::Response};
use futures_util::StreamExt;
use mazuadm_core::*;
use mazuadm_core::scheduler::{select_running_round_id, SchedulerCommand};
use mazuadm_core::executor::Executor;
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

fn spawn_job_runner(executor: Executor, job_id: i32) {
    tokio::spawn(async move {
        if let Err(e) = executor.run_job_immediately(job_id).await {
            tracing::error!("Job {} failed: {}", job_id, e);
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

// settings helpers live in mazuadm_core::settings

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
    let mut inserted_jobs = false;
    if insert_into_rounds.unwrap_or(false) {
        if let Ok(rounds) = s.db.get_active_rounds().await {
            let runs = s.db.list_exploit_runs(Some(exploit.challenge_id), None).await.unwrap_or_default();
            let exploit_runs: Vec<_> = runs.iter().filter(|r| r.exploit_id == exploit.id).collect();
            for round in rounds {
                for run in &exploit_runs {
                    if let Ok(job) = s.db.create_job(round.id, run.id, run.team_id, 0).await {
                        inserted_jobs = true;
                        broadcast(&s, "job_created", &job);
                    }
                }
            }
        }
    }
    if inserted_jobs {
        s.scheduler.notify();
    }
    
    // Pre-warm containers for enabled exploit
    if exploit.enabled {
        let cm = s.executor.container_manager.clone();
        let _ = cm.ensure_containers(exploit.id).await;
    }
    
    broadcast(&s, "exploit_created", &exploit);
    Ok(Json(exploit))
}

pub async fn update_exploit(State(s): S, Path(id): Path<i32>, Json(e): Json<UpdateExploit>) -> R<Exploit> {
    let was_enabled = s.db.get_exploit(id).await.map(|e| e.enabled).unwrap_or(false);
    let exploit = s.db.update_exploit(id, e).await.map_err(err)?;
    
    let cm = s.executor.container_manager.clone();
    if exploit.enabled && !was_enabled {
        let _ = cm.ensure_containers(id).await;
    } else if !exploit.enabled && was_enabled {
        let _ = cm.destroy_exploit_containers(id).await;
    }
    
    broadcast(&s, "exploit_updated", &exploit);
    Ok(Json(exploit))
}

pub async fn delete_exploit(State(s): S, Path(id): Path<i32>) -> R<String> {
    let cm = s.executor.container_manager.clone();
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
    let scheduler = mazuadm_core::scheduler::Scheduler::new(s.db.clone(), s.executor.clone(), s.tx.clone());
    let round_id = scheduler.create_round().await.map_err(err)?;
    Ok(Json(round_id))
}

pub async fn run_round(State(s): S, Path(id): Path<i32>) -> R<String> {
    s.scheduler.send(SchedulerCommand::RunRound(id)).map_err(err)?;
    Ok(Json("started".to_string()))
}

pub async fn rerun_round(State(s): S, Path(id): Path<i32>) -> R<String> {
    s.scheduler.send(SchedulerCommand::RerunRound(id)).map_err(err)?;
    Ok(Json("restarted".to_string()))
}

pub async fn schedule_unflagged_round(State(s): S, Path(id): Path<i32>) -> R<String> {
    s.scheduler.send(SchedulerCommand::ScheduleUnflagged(id)).map_err(err)?;
    Ok(Json("scheduled".to_string()))
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
    
    spawn_job_runner(s.executor.clone(), job.id);
    
    Ok(Json(job))
}

pub async fn run_existing_job(State(s): S, Path(job_id): Path<i32>) -> R<ExploitJob> {
    let job = s.db.get_job(job_id).await.map_err(err)?;
    let _ = job.exploit_run_id.ok_or("Job has no exploit_run_id".to_string())?;
    
    // Reset job status to pending
    s.db.update_job_status(job_id, "pending", false).await.map_err(err)?;
    let job = s.db.get_job(job_id).await.map_err(err)?;
    broadcast(&s, "job_updated", &job);
    
    spawn_job_runner(s.executor.clone(), job_id);
    
    Ok(Json(job))
}

pub async fn stop_job(State(s): S, Path(job_id): Path<i32>) -> R<ExploitJob> {
    let job = s.executor.stop_job(job_id, "stopped by user").await.map_err(err)?;
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
pub async fn list_containers(State(s): S, Query(q): Query<ListQuery>) -> R<Vec<ContainerInfo>> {
    let mut containers = s.executor.container_manager.list_containers().await.map_err(err)?;
    if let Some(exploit_id) = q.challenge_id {
        containers.retain(|c| c.exploit_id == exploit_id);
    }
    Ok(Json(containers))
}

pub async fn get_container_runners(State(s): S, Path(id): Path<String>) -> R<Vec<ExploitJob>> {
    s.db.get_running_jobs_by_container(&id).await.map(Json).map_err(err)
}

pub async fn delete_container(State(s): S, Path(id): Path<String>) -> R<String> {
    let cm = s.executor.container_manager.clone();
    cm.destroy_container_by_id(&id).await.map_err(err)?;
    Ok(Json("ok".to_string()))
}

pub async fn restart_container(State(s): S, Path(id): Path<String>) -> R<String> {
    let cm = s.executor.container_manager.clone();
    cm.restart_container_by_id(&id).await.map_err(err)?;
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
    use super::should_continue_ws;

    #[test]
    fn should_continue_ws_only_on_lagged() {
        assert!(should_continue_ws(tokio::sync::broadcast::error::RecvError::Lagged(1)));
        assert!(!should_continue_ws(tokio::sync::broadcast::error::RecvError::Closed));
    }
}
