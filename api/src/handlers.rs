use crate::AppState;
use axum::{extract::{Path, Query, State}, Json};
use mazuadm_core::*;
use mazuadm_core::scheduler::Scheduler;
use mazuadm_core::executor::Executor;
use mazuadm_core::container_manager::ContainerManager;
use std::sync::Arc;
use serde::Deserialize;

type S = State<Arc<AppState>>;
type R<T> = Result<Json<T>, String>;

fn err<E: std::fmt::Display>(e: E) -> String { e.to_string() }

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
    Ok(Json(challenge))
}

pub async fn update_challenge(State(s): S, Path(id): Path<i32>, Json(c): Json<CreateChallenge>) -> R<Challenge> {
    s.db.update_challenge(id, c).await.map(Json).map_err(err)
}

pub async fn delete_challenge(State(s): S, Path(id): Path<i32>) -> R<String> {
    s.db.delete_challenge(id).await.map_err(err)?;
    Ok(Json("ok".to_string()))
}

pub async fn set_challenge_enabled(State(s): S, Path((id, enabled)): Path<(i32, bool)>) -> R<String> {
    s.db.set_challenge_enabled(id, enabled).await.map_err(err)?;
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
    Ok(Json(team))
}

pub async fn update_team(State(s): S, Path(id): Path<i32>, Json(t): Json<CreateTeam>) -> R<Team> {
    s.db.update_team(id, t).await.map(Json).map_err(err)
}

pub async fn delete_team(State(s): S, Path(id): Path<i32>) -> R<String> {
    s.db.delete_team(id).await.map_err(err)?;
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
                let _ = s.db.create_exploit_run(CreateExploitRun {
                    exploit_id: exploit.id,
                    challenge_id: exploit.challenge_id,
                    team_id: team.id,
                    priority: None,
                    sequence: Some(seq),
                }).await;
            }
        }
    }
    
    // Pre-warm containers for enabled exploit
    if exploit.enabled {
        let cm = ContainerManager::new(s.db.clone()).map_err(err)?;
        let _ = cm.ensure_containers(exploit.id).await;
    }
    
    Ok(Json(exploit))
}

pub async fn update_exploit(State(s): S, Path(id): Path<i32>, Json(e): Json<CreateExploit>) -> R<Exploit> {
    let was_enabled = s.db.get_exploit(id).await.map(|e| e.enabled).unwrap_or(false);
    let exploit = s.db.update_exploit(id, e).await.map_err(err)?;
    
    let cm = ContainerManager::new(s.db.clone()).map_err(err)?;
    if exploit.enabled && !was_enabled {
        // Just enabled - spawn containers
        let _ = cm.ensure_containers(id).await;
    } else if !exploit.enabled && was_enabled {
        // Just disabled - destroy containers
        let _ = cm.destroy_exploit_containers(id).await;
    }
    
    Ok(Json(exploit))
}

pub async fn delete_exploit(State(s): S, Path(id): Path<i32>) -> R<String> {
    // Destroy containers first
    let cm = ContainerManager::new(s.db.clone()).map_err(err)?;
    let _ = cm.destroy_exploit_containers(id).await;
    
    s.db.delete_exploit(id).await.map_err(err)?;
    Ok(Json("ok".to_string()))
}

// Exploit Runs
pub async fn list_exploit_runs(State(s): S, Query(q): Query<ListQuery>) -> R<Vec<ExploitRun>> {
    s.db.list_exploit_runs(q.challenge_id, q.team_id).await.map(Json).map_err(err)
}

pub async fn create_exploit_run(State(s): S, Json(r): Json<CreateExploitRun>) -> R<ExploitRun> {
    s.db.create_exploit_run(r).await.map(Json).map_err(err)
}

#[derive(Deserialize)]
pub struct UpdateExploitRun {
    pub priority: Option<i32>,
    pub sequence: Option<i32>,
    pub enabled: Option<bool>,
}

pub async fn update_exploit_run(State(s): S, Path(id): Path<i32>, Json(u): Json<UpdateExploitRun>) -> R<ExploitRun> {
    s.db.update_exploit_run(id, u.priority, u.sequence, u.enabled).await.map(Json).map_err(err)
}

pub async fn delete_exploit_run(State(s): S, Path(id): Path<i32>) -> R<String> {
    s.db.delete_exploit_run(id).await.map_err(err)?;
    Ok(Json("ok".to_string()))
}

#[derive(Deserialize)]
pub struct ReorderItem {
    pub id: i32,
    pub sequence: i32,
}

pub async fn reorder_exploit_runs(State(s): S, Json(items): Json<Vec<ReorderItem>>) -> R<String> {
    s.db.reorder_exploit_runs(&items.iter().map(|i| (i.id, i.sequence)).collect::<Vec<_>>()).await.map_err(err)?;
    Ok(Json("ok".to_string()))
}

// Rounds
pub async fn list_rounds(State(s): S) -> R<Vec<Round>> {
    s.db.list_rounds().await.map(Json).map_err(err)
}

pub async fn create_round(State(s): S) -> R<i32> {
    let scheduler = Scheduler::new(s.db.clone());
    scheduler.generate_round().await.map(Json).map_err(err)
}

pub async fn run_round(State(s): S, Path(id): Path<i32>) -> R<String> {
    let executor = Executor::new(s.db.clone()).map_err(err)?;
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

#[derive(Deserialize)]
pub struct UpdateSetting {
    pub key: String,
    pub value: String,
}

pub async fn update_setting(State(s): S, Json(u): Json<UpdateSetting>) -> R<String> {
    s.db.set_setting(&u.key, &u.value).await.map_err(err)?;
    Ok(Json("ok".to_string()))
}

// Containers
pub async fn list_containers(State(s): S, Query(q): Query<ListQuery>) -> R<Vec<ExploitContainer>> {
    match q.challenge_id {
        Some(eid) => s.db.get_exploit_containers(eid).await.map(Json).map_err(err),
        None => s.db.list_all_containers().await.map(Json).map_err(err),
    }
}

pub async fn health_check_containers(State(s): S) -> R<String> {
    let cm = ContainerManager::new(s.db.clone()).map_err(err)?;
    cm.health_check().await.map_err(err)?;
    Ok(Json("ok".to_string()))
}

pub async fn ensure_all_containers(State(s): S) -> R<String> {
    let cm = ContainerManager::new(s.db.clone()).map_err(err)?;
    cm.ensure_all_containers().await.map_err(err)?;
    Ok(Json("ok".to_string()))
}
