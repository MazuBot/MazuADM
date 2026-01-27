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
                    Err(_) => break,
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
    let tx = s.tx.clone();
    let executor = Executor::new(s.db.clone(), tx).map_err(err)?;
    tokio::spawn(async move { executor.run_round(id).await });
    Ok(Json("started".to_string()))
}

// Jobs
pub async fn list_jobs(State(s): S, Query(q): Query<ListQuery>) -> R<Vec<ExploitJob>> {
    let round_id = q.round_id.unwrap_or(0);
    s.db.list_jobs(round_id).await.map(Json).map_err(err)
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
