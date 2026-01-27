use crate::AppState;
use axum::{extract::{Path, Query, State}, Json};
use mazuadm_core::*;
use mazuadm_core::scheduler::Scheduler;
use mazuadm_core::executor::Executor;
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
    s.db.create_exploit(e).await.map(Json).map_err(err)
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
