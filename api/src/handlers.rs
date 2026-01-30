use crate::events::WsMessage;
use crate::{AppState, WsConnection};
use axum::{extract::{Path, Query, State, ws::{WebSocket, WebSocketUpgrade}, ConnectInfo}, http::HeaderMap, Json, response::Response};
use chrono::Utc;
use futures_util::{stream, SinkExt, StreamExt};
use mazuadm_core::*;
use mazuadm_core::scheduler::{select_running_round_id, SchedulerCommand};
use mazuadm_core::settings::parse_setting_usize;
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use serde::Deserialize;
use uuid::Uuid;

type S = State<Arc<AppState>>;
type R<T> = Result<Json<T>, String>;

fn err<E: std::fmt::Display>(e: E) -> String { e.to_string() }

fn broadcast<T: serde::Serialize>(s: &AppState, msg_type: &str, data: &T) {
    let _ = s.tx.send(WsMessage::new(msg_type, data));
}

fn broadcast_job(s: &AppState, msg_type: &str, job: &ExploitJob) {
    broadcast(s, msg_type, &job.without_logs());
}

fn should_continue_ws(err: tokio::sync::broadcast::error::RecvError) -> bool {
    matches!(err, tokio::sync::broadcast::error::RecvError::Lagged(_))
}

fn should_refresh_scheduler(round_status: &str) -> bool {
    round_status == "running"
}

// WebSocket handler
#[derive(Deserialize)]
pub struct WsQuery {
    pub events: Option<String>,
    pub client: Option<String>,
    pub user: Option<String>,
}

fn parse_events(events: Option<String>) -> Option<HashSet<String>> {
    events.map(|s| s.split(',').filter(|p| !p.is_empty()).map(|p| p.to_string()).collect())
}

fn matches_subscription(event_type: &str, subs: &Option<HashSet<String>>) -> bool {
    match subs {
        None => true,
        Some(set) => {
            // Extract category: everything before the last underscore (e.g., "exploit_run" from "exploit_run_created")
            let category = event_type.rsplit_once('_').map(|(cat, _)| cat).unwrap_or(event_type);
            set.iter().any(|sub| category == sub || category.starts_with(&format!("{}_", sub)))
        }
    }
}

#[derive(Deserialize)]
struct WsClientMsg {
    action: String,
    events: Vec<String>,
}

fn is_valid_name(s: &str) -> bool {
    s.len() >= 3 && s.len() <= 16 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

pub async fn ws_handler(ws: WebSocketUpgrade, Query(q): Query<WsQuery>, headers: HeaderMap, ConnectInfo(addr): ConnectInfo<SocketAddr>, State(s): S) -> Response {
    let user = q.user.filter(|u| !u.is_empty());
    let client = q.client.filter(|c| !c.is_empty());
    let error = match (&user, &client) {
        (None, _) => Some("Missing required 'user' parameter"),
        (Some(u), _) if !is_valid_name(u) => Some("User must be 3-16 alphanumeric or hyphen characters"),
        (_, Some(c)) if !is_valid_name(c) => Some("Client must be 3-16 alphanumeric or hyphen characters"),
        _ => None,
    };
    if let Some(err_msg) = error {
        return ws.on_upgrade(move |mut socket| async move {
            let msg = WsMessage::new("error", &serde_json::json!({"message": err_msg}));
            let _ = socket.send(axum::extract::ws::Message::Text(serde_json::to_string(&msg).unwrap().into())).await;
            let _ = socket.close().await;
        });
    }

    let client_ip = get_client_ip(&s, &headers, addr).await;
    let subs = parse_events(q.events);
    let client_name = client.unwrap_or_else(|| "unknown".to_string());
    ws.on_upgrade(move |socket| handle_ws(socket, s, subs, client_ip, client_name, user.unwrap()))
}

async fn get_client_ip(state: &AppState, headers: &HeaderMap, addr: SocketAddr) -> String {
    let ip_headers = state.db.get_setting("ip_headers").await.unwrap_or_default();
    for header_name in ip_headers.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        if let Some(value) = headers.get(header_name).and_then(|v| v.to_str().ok()) {
            let ip = value.split(',').next().unwrap_or("").trim();
            if !ip.is_empty() {
                return ip.to_string();
            }
        }
    }
    addr.ip().to_string()
}

fn broadcast_ws_connections(state: &AppState) {
    let now = Utc::now();
    let conns: Vec<_> = state.ws_connections.iter().map(|entry| {
        let conn = entry.value();
        WsConnectionInfo {
            id: entry.key().to_string(),
            client_ip: conn.client_ip.clone(),
            client_name: conn.client_name.clone(),
            user: conn.user.clone(),
            subscribed_events: conn.subscribed_events.iter().cloned().collect(),
            connected_at: conn.connected_at,
            duration_secs: (now - conn.connected_at).num_seconds(),
        }
    }).collect();
    broadcast(state, "ws_connections", &conns);
}

async fn handle_ws(mut socket: WebSocket, state: Arc<AppState>, mut subs: Option<HashSet<String>>, client_ip: String, client_name: String, user: String) {
    let conn_id = Uuid::new_v4();
    let mut rx = state.tx.subscribe();
    
    state.ws_connections.insert(conn_id, WsConnection {
        client_ip,
        client_name,
        user,
        subscribed_events: subs.clone().unwrap_or_default(),
        connected_at: Utc::now(),
    });
    broadcast_ws_connections(&state);

    loop {
        tokio::select! {
            msg = rx.recv() => {
                match msg {
                    Ok(m) => {
                        if matches_subscription(&m.msg_type, &subs) {
                            let text = serde_json::to_string(&m).unwrap_or_default();
                            if socket.send(axum::extract::ws::Message::Text(text.into())).await.is_err() {
                                break;
                            }
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
                    Some(Ok(axum::extract::ws::Message::Text(text))) => {
                        if let Ok(cmd) = serde_json::from_str::<WsClientMsg>(&text) {
                            let set = subs.get_or_insert_with(HashSet::new);
                            match cmd.action.as_str() {
                                "subscribe" => set.extend(cmd.events.clone()),
                                "unsubscribe" => { for e in &cmd.events { set.remove(e); } }
                                _ => {}
                            }
                            if let Some(mut conn) = state.ws_connections.get_mut(&conn_id) {
                                conn.subscribed_events = set.clone();
                            }
                        }
                    }
                    Some(Ok(_)) => {}
                    _ => break,
                }
            }
        }
    }
    state.ws_connections.remove(&conn_id);
    broadcast_ws_connections(&state);
}

// Version
#[derive(serde::Serialize)]
pub struct VersionInfo {
    pub version: &'static str,
    pub git_hash: &'static str,
    pub git_ref: &'static str,
    pub build_time: &'static str,
    pub rustc: &'static str,
}

pub async fn version() -> Json<VersionInfo> {
    Json(VersionInfo {
        version: env!("CARGO_PKG_VERSION"),
        git_hash: env!("BUILD_GIT_HASH"),
        git_ref: env!("BUILD_GIT_REF"),
        build_time: env!("BUILD_TIME"),
        rustc: env!("BUILD_RUSTC"),
    })
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

pub async fn create_exploit(State(s): S, Json(mut e): Json<CreateExploit>) -> R<Exploit> {
    // Apply default_ignore_connection_info setting if not specified
    if e.ignore_connection_info.is_none() {
        if let Ok(val) = s.db.get_setting("default_ignore_connection_info").await {
            e.ignore_connection_info = Some(val == "true");
        }
    }
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
    let mut refresh_job_ids = Vec::new();
    if insert_into_rounds.unwrap_or(false) {
        if let Ok(rounds) = s.db.get_active_rounds().await {
            let runs = s.db.list_exploit_runs(Some(exploit.challenge_id), None).await.unwrap_or_default();
            let exploit_runs: Vec<_> = runs.iter().filter(|r| r.exploit_id == exploit.id).collect();
            for round in rounds {
                for run in &exploit_runs {
                    if let Ok(job) = s.db.create_job(round.id, run.id, run.team_id, 0, Some("new_exploit")).await {
                        broadcast_job(&s, "job_created", &job);
                        if should_refresh_scheduler(&round.status) {
                            refresh_job_ids.push(job.id);
                        }
                    }
                }
            }
        }
    }
    for job_id in refresh_job_ids {
        if let Err(e) = s.scheduler.send(SchedulerCommand::RefreshJob(job_id)) {
            tracing::error!("Failed to refresh scheduler for job {}: {}", job_id, e);
        }
    }
    
    // Pre-warm containers for enabled exploit
    if exploit.enabled {
        let _ = s.scheduler.ensure_containers(exploit.id).await;
    }
    
    broadcast(&s, "exploit_created", &exploit);
    Ok(Json(exploit))
}

pub async fn update_exploit(State(s): S, Path(id): Path<i32>, Json(e): Json<UpdateExploit>) -> R<Exploit> {
    let was_enabled = s.db.get_exploit(id).await.map(|e| e.enabled).unwrap_or(false);
    let exploit = s.db.update_exploit(id, e).await.map_err(err)?;
    
    if exploit.enabled && !was_enabled {
        let _ = s.scheduler.ensure_containers(id).await;
    } else if !exploit.enabled && was_enabled {
        let _ = s.scheduler.destroy_exploit_containers(id).await;
    }
    
    broadcast(&s, "exploit_updated", &exploit);
    Ok(Json(exploit))
}

pub async fn delete_exploit(State(s): S, Path(id): Path<i32>) -> R<String> {
    let _ = s.scheduler.destroy_exploit_containers(id).await;
    
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

pub async fn get_current_round(State(s): S) -> R<Option<Round>> {
    let rounds = s.db.get_active_rounds().await.map_err(err)?;
    Ok(Json(rounds.into_iter().find(|r| r.status == "running")))
}

pub async fn create_round(State(s): S) -> R<i32> {
    let round_id = s.scheduler.create_round().await.map_err(err)?;
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

pub async fn rerun_unflagged(State(s): S, Path(id): Path<i32>) -> R<String> {
    let round = s.db.get_round(id).await.map_err(err)?;
    if round.status != "running" {
        return Err(format!("Round {} is not running", id));
    }
    s.scheduler.send(SchedulerCommand::RerunUnflagged(id)).map_err(err)?;
    Ok(Json("rerun".to_string()))
}

// Jobs
pub async fn list_jobs(State(s): S, Query(q): Query<ListQuery>) -> R<Vec<ExploitJob>> {
    let round_id = q.round_id.unwrap_or(0);
    s.db.list_jobs(round_id).await.map(Json).map_err(err)
}

pub async fn get_job(State(s): S, Path(id): Path<i32>) -> R<ExploitJob> {
    s.db.get_job(id).await.map(Json).map_err(err)
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
            broadcast_job(&s, "job_updated", &job);
            s.scheduler.send(SchedulerCommand::RefreshJob(job.id)).map_err(err)?;
        }
    }
    Ok(Json("ok".to_string()))
}

#[derive(Deserialize)]
pub struct EnqueueSingleJobRequest {
    pub exploit_run_id: i32,
    pub team_id: i32,
}

async fn require_running_round_id(s: &AppState) -> Result<i32, String> {
    let rounds = s.db.get_active_rounds().await.map_err(err)?;
    select_running_round_id(&rounds).ok_or_else(|| "No running round".to_string())
}

fn min_allowed_round_id(running_round_id: i32, past_rounds: usize) -> i32 {
    let past_rounds = past_rounds.min(i32::MAX as usize) as i32;
    running_round_id.saturating_sub(past_rounds).max(0)
}

fn round_within_history(target_round_id: i32, running_round_id: i32, past_rounds: usize) -> bool {
    let min_allowed = min_allowed_round_id(running_round_id, past_rounds);
    target_round_id >= min_allowed && target_round_id <= running_round_id
}

async fn run_job_immediately(s: &AppState, job_id: i32) {
    if let Err(e) = s.db.mark_job_scheduled(job_id).await {
        tracing::error!("Failed to mark job {} scheduled: {}", job_id, e);
    }
    if let Err(e) = s.scheduler.run_job_immediately(job_id) {
        tracing::error!("Immediate job {} failed to enqueue: {}", job_id, e);
    }
}

pub async fn enqueue_single_job(State(s): S, Json(req): Json<EnqueueSingleJobRequest>) -> R<ExploitJob> {
    let run = s.db.get_exploit_run(req.exploit_run_id).await.map_err(err)?;
    let round_id = require_running_round_id(&s).await?;
    let max_priority = s.db.get_max_priority_for_round(round_id).await.map_err(err)?;
    let job = s.db.create_job(round_id, run.id, req.team_id, max_priority + 1, Some("enqueue_exploit")).await.map_err(err)?;
    broadcast_job(&s, "job_created", &job);
    run_job_immediately(&s, job.id).await;
    Ok(Json(job))
}

pub async fn enqueue_existing_job(State(s): S, Path(job_id): Path<i32>) -> R<ExploitJob> {
    let job = s.db.get_job(job_id).await.map_err(err)?;
    let round_id = require_running_round_id(&s).await?;
    let max_priority = s.db.get_max_priority_for_round(round_id).await.map_err(err)?;

    if job.status == "pending" && job.round_id == round_id {
        run_job_immediately(&s, job_id).await;
        return Ok(Json(job));
    }

    let run_id = job.exploit_run_id.ok_or_else(|| "Job has no exploit_run_id".to_string())?;
    let create_reason = format!("rerun_job:{}", job_id);
    let new_job = s.db.create_job(round_id, run_id, job.team_id, max_priority + 1, Some(&create_reason)).await.map_err(err)?;
    broadcast_job(&s, "job_created", &new_job);
    s.scheduler.send(SchedulerCommand::RefreshJob(new_job.id)).map_err(err)?;
    Ok(Json(new_job))
}

pub async fn stop_job(State(s): S, Path(job_id): Path<i32>) -> R<ExploitJob> {
    let job = s.scheduler.stop_job(job_id, "stopped by user").await.map_err(err)?;
    s.scheduler.send(SchedulerCommand::RefreshJob(job.id)).map_err(err)?;
    Ok(Json(job))
}

// Flags
#[derive(Deserialize, Clone)]
pub struct SubmitFlagRequest {
    pub round_id: Option<i32>,
    pub challenge_id: i32,
    pub team_id: i32,
    pub flag_value: String,
    pub status: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum SubmitFlagsBody {
    Single(SubmitFlagRequest),
    Multiple(Vec<SubmitFlagRequest>),
}

async fn create_one_flag(s: &AppState, req: &SubmitFlagRequest, running_round_id: i32, past_rounds: usize) -> Result<Flag, String> {
    let flag_value = req.flag_value.trim();
    if flag_value.is_empty() {
        return Err("Flag value cannot be empty".to_string());
    }
    if flag_value.len() > 512 {
        return Err("Flag value exceeds 512 characters".to_string());
    }
    let round_id = req.round_id.unwrap_or(running_round_id);
    if !round_within_history(round_id, running_round_id, past_rounds) {
        let min_allowed = min_allowed_round_id(running_round_id, past_rounds);
        return Err(format!(
            "Round {} is outside allowed range ({}..={})",
            round_id, min_allowed, running_round_id
        ));
    }
    s.db.get_round(round_id).await.map_err(err)?;
    s.db.get_challenge(req.challenge_id).await.map_err(err)?;
    s.db.get_team(req.team_id).await.map_err(err)?;
    let status = req.status.as_deref().unwrap_or("captured");
    let flag = s.db.create_manual_flag(round_id, req.challenge_id, req.team_id, flag_value, status).await.map_err(err)?;
    broadcast(s, "flag_created", &flag);
    Ok(flag)
}

pub async fn submit_flag(State(s): S, Json(body): Json<SubmitFlagsBody>) -> R<Vec<Flag>> {
    let running_round_id = require_running_round_id(&s).await?;
    let past_rounds = parse_setting_usize(s.db.get_setting("past_flag_rounds").await.ok(), 5);
    let reqs = match body {
        SubmitFlagsBody::Single(r) => vec![r],
        SubmitFlagsBody::Multiple(r) => r,
    };
    let mut flags = Vec::new();
    for req in &reqs {
        flags.push(create_one_flag(&s, req, running_round_id, past_rounds).await?);
    }
    Ok(Json(flags))
}

pub async fn list_flags(State(s): S, Query(q): Query<ListFlagsQuery>) -> Result<Json<Vec<serde_json::Value>>, String> {
    let statuses = q.status.as_ref().map(|s| s.split(',').map(|x| x.to_string()).collect::<Vec<_>>());
    let desc = match q.sort.as_deref() {
        Some("asc") => false,
        Some("desc") | None => true,
        Some(_) => return Err("sort must be 'asc' or 'desc'".to_string()),
    };
    let flags = s.db.list_flags(q.round_id, statuses, desc).await.map_err(err)?;
    let fields: Option<Vec<&str>> = q.fields.as_ref().map(|f| f.split(',').collect());
    let result: Vec<serde_json::Value> = flags.into_iter().map(|flag| {
        let mut obj = serde_json::to_value(&flag).unwrap_or_default();
        if let (Some(fields), Some(map)) = (&fields, obj.as_object_mut()) {
            map.retain(|k, _| fields.contains(&k.as_str()));
        }
        obj
    }).collect();
    Ok(Json(result))
}

#[derive(Deserialize)]
pub struct ListFlagsQuery {
    pub round_id: Option<i32>,
    pub status: Option<String>,
    pub sort: Option<String>,
    pub fields: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct UpdateFlagRequest {
    pub id: i32,
    pub status: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum UpdateFlagsBody {
    Single(UpdateFlagRequest),
    Multiple(Vec<UpdateFlagRequest>),
}

#[derive(Deserialize)]
pub struct UpdateFlagsQuery {
    pub force: Option<bool>,
}

pub async fn update_flags(State(s): S, Query(q): Query<UpdateFlagsQuery>, Json(body): Json<UpdateFlagsBody>) -> R<Vec<bool>> {
    let force = q.force.unwrap_or(false);
    let reqs = match body {
        UpdateFlagsBody::Single(r) => vec![r],
        UpdateFlagsBody::Multiple(r) => r,
    };
    let mut results = Vec::new();
    for req in &reqs {
        let updated = s.db.update_flag_status(req.id, &req.status, force).await.map_err(err)?;
        if updated {
            if let Ok(flag) = s.db.get_flag(req.id).await {
                broadcast(&s, "flag_updated", &flag);
            }
        }
        results.push(updated);
    }
    Ok(Json(results))
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

#[derive(Deserialize, Default)]
pub struct RestartContainerRequest {
    pub timeout: Option<u64>,
    pub force: Option<bool>,
}

#[derive(Deserialize, Default)]
pub struct ContainerBulkQuery {
    pub challenge_id: Option<i32>,
}

#[derive(serde::Serialize)]
pub struct ContainerBulkOpResult {
    pub total: usize,
    pub success: usize,
    pub failed: usize,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<String>,
}

fn summarize_container_ops<E: std::fmt::Display>(results: Vec<(String, std::result::Result<(), E>)>) -> ContainerBulkOpResult {
    let total = results.len();
    let failures: Vec<String> = results
        .into_iter()
        .filter_map(|(id, res)| res.err().map(|e| format!("{}: {}", id, e)))
        .collect();
    let failed = failures.len();
    let success = total - failed;
    ContainerBulkOpResult { total, success, failed, failures }
}

async fn list_containers_for_challenge(s: &AppState, challenge_id: Option<i32>) -> Result<Vec<ContainerInfo>, String> {
    let containers = s.scheduler.list_containers(None).await.map_err(err)?;
    let Some(challenge_id) = challenge_id else {
        return Ok(containers);
    };
    let exploits = s.db.list_exploits(Some(challenge_id)).await.map_err(err)?;
    let exploit_ids: HashSet<i32> = exploits.into_iter().map(|exploit| exploit.id).collect();
    Ok(containers
        .into_iter()
        .filter(|container| exploit_ids.contains(&container.exploit_id))
        .collect())
}

pub async fn update_setting(State(s): S, Json(u): Json<UpdateSetting>) -> R<String> {
    s.db.set_setting(&u.key, &u.value).await.map_err(err)?;
    broadcast(&s, "setting_updated", &u);
    Ok(Json("ok".to_string()))
}

// Containers
pub async fn list_containers(State(s): S, Query(q): Query<ListQuery>) -> R<Vec<ContainerInfo>> {
    let containers = list_containers_for_challenge(&s, q.challenge_id).await?;
    Ok(Json(containers))
}

pub async fn get_container_runners(State(s): S, Path(id): Path<String>) -> R<Vec<ExploitRunner>> {
    let Some(container) = s
        .db
        .get_exploit_container_by_container_id(&id)
        .await
        .map_err(err)? else {
        return Ok(Json(Vec::new()));
    };
    s.db.list_exploit_runners_by_container(container.id).await.map(Json).map_err(err)
}

pub async fn delete_container(State(s): S, Path(id): Path<String>) -> R<String> {
    s.scheduler.destroy_container(id).await.map_err(err)?;
    Ok(Json("ok".to_string()))
}

pub async fn restart_container(State(s): S, Path(id): Path<String>, body: Option<Json<RestartContainerRequest>>) -> R<String> {
    let req = body.map(|Json(r)| r).unwrap_or_default();
    s.scheduler
        .restart_container(id, req.timeout, req.force.unwrap_or(false))
        .await
        .map_err(err)?;
    Ok(Json("ok".to_string()))
}

pub async fn restart_all_containers(
    State(s): S,
    Query(q): Query<ContainerBulkQuery>,
    body: Option<Json<RestartContainerRequest>>,
) -> R<ContainerBulkOpResult> {
    let req = body.map(|Json(r)| r).unwrap_or_default();
    let force = req.force.unwrap_or(false);
    let timeout = req.timeout;
    let containers = list_containers_for_challenge(&s, q.challenge_id).await?;
    let ids: Vec<String> = containers.into_iter().map(|c| c.id).collect();
    let scheduler = s.scheduler.clone();
    let results = stream::iter(ids)
        .map(|id| {
            let scheduler = scheduler.clone();
            async move { (id.clone(), scheduler.restart_container(id, timeout, force).await) }
        })
        .buffer_unordered(10)
        .collect::<Vec<_>>()
        .await;
    Ok(Json(summarize_container_ops(results)))
}

pub async fn remove_all_containers(State(s): S, Query(q): Query<ContainerBulkQuery>) -> R<ContainerBulkOpResult> {
    let containers = list_containers_for_challenge(&s, q.challenge_id).await?;
    let ids: Vec<String> = containers.into_iter().map(|c| c.id).collect();
    let scheduler = s.scheduler.clone();
    let results = stream::iter(ids)
        .map(|id| {
            let scheduler = scheduler.clone();
            async move { (id.clone(), scheduler.destroy_container(id).await) }
        })
        .buffer_unordered(10)
        .collect::<Vec<_>>()
        .await;
    Ok(Json(summarize_container_ops(results)))
}

// Relations
pub async fn list_relations(State(s): S, Path(challenge_id): Path<i32>) -> R<Vec<ChallengeTeamRelation>> {
    s.db.list_relations(challenge_id).await.map(Json).map_err(err)
}

pub async fn get_relation(State(s): S, Path((challenge_id, team_id)): Path<(i32, i32)>) -> R<Option<ChallengeTeamRelation>> {
    s.db.get_relation(challenge_id, team_id).await.map(Json).map_err(err)
}

#[derive(Deserialize)]
pub struct UpdateConnectionInfo {
    pub addr: Option<String>,
    pub port: Option<i32>,
    pub enabled: Option<bool>,
}

pub async fn update_connection_info(State(s): S, Path((challenge_id, team_id)): Path<(i32, i32)>, Json(u): Json<UpdateConnectionInfo>) -> R<ChallengeTeamRelation> {
    let rel = s.db.update_connection_info(challenge_id, team_id, u.addr, u.port, u.enabled).await.map_err(err)?;
    broadcast(&s, "connection_info_updated", &rel);
    Ok(Json(rel))
}

// WebSocket connections list
#[derive(serde::Serialize)]
pub struct WsConnectionInfo {
    pub id: String,
    pub client_ip: String,
    pub client_name: String,
    pub user: String,
    pub subscribed_events: Vec<String>,
    pub connected_at: chrono::DateTime<Utc>,
    pub duration_secs: i64,
}

pub async fn list_ws_connections(State(s): S) -> Json<Vec<WsConnectionInfo>> {
    let now = Utc::now();
    let conns: Vec<_> = s.ws_connections.iter().map(|entry| {
        let conn = entry.value();
        WsConnectionInfo {
            id: entry.key().to_string(),
            client_ip: conn.client_ip.clone(),
            client_name: conn.client_name.clone(),
            user: conn.user.clone(),
            subscribed_events: conn.subscribed_events.iter().cloned().collect(),
            connected_at: conn.connected_at,
            duration_secs: (now - conn.connected_at).num_seconds(),
        }
    }).collect();
    Json(conns)
}

#[cfg(test)]
mod tests {
    use super::{min_allowed_round_id, round_within_history, should_continue_ws, should_refresh_scheduler};

    #[test]
    fn should_continue_ws_only_on_lagged() {
        assert!(should_continue_ws(tokio::sync::broadcast::error::RecvError::Lagged(1)));
        assert!(!should_continue_ws(tokio::sync::broadcast::error::RecvError::Closed));
    }

    #[test]
    fn should_refresh_scheduler_only_for_running_rounds() {
        assert!(should_refresh_scheduler("running"));
        assert!(!should_refresh_scheduler("pending"));
        assert!(!should_refresh_scheduler("finished"));
    }

    #[test]
    fn round_within_history_allows_running_and_past() {
        assert!(round_within_history(10, 10, 0));
        assert!(round_within_history(9, 10, 1));
        assert!(round_within_history(8, 10, 2));
    }

    #[test]
    fn round_within_history_blocks_future_and_too_old() {
        assert!(!round_within_history(11, 10, 2));
        assert!(!round_within_history(7, 10, 2));
    }

    #[test]
    fn min_allowed_round_id_saturates() {
        assert_eq!(min_allowed_round_id(2, 5), 0);
    }
}
