use axum::{Router, routing::{get, post, put}};
use crate::{AppState, handlers::*};
use std::sync::Arc;

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/challenges", get(list_challenges).post(create_challenge))
        .route("/api/challenges/{id}", put(update_challenge).delete(delete_challenge))
        .route("/api/challenges/{id}/enabled/{enabled}", put(set_challenge_enabled))
        .route("/api/teams", get(list_teams).post(create_team))
        .route("/api/teams/{id}", put(update_team).delete(delete_team))
        .route("/api/exploits", get(list_exploits).post(create_exploit))
        .route("/api/exploits/{id}", put(update_exploit).delete(delete_exploit))
        .route("/api/exploit-runs", get(list_exploit_runs).post(create_exploit_run))
        .route("/api/exploit-runs/reorder", post(reorder_exploit_runs))
        .route("/api/exploit-runs/{id}", put(update_exploit_run).delete(delete_exploit_run))
        .route("/api/rounds", get(list_rounds).post(create_round))
        .route("/api/rounds/{id}/run", post(run_round))
        .route("/api/jobs", get(list_jobs))
        .route("/api/flags", get(list_flags))
        .route("/api/settings", get(list_settings).post(update_setting))
        .route("/api/containers", get(list_containers))
        .route("/api/containers/health", post(health_check_containers))
        .route("/api/containers/ensure", post(ensure_all_containers))
}
