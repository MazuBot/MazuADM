mod events;
mod handlers;
mod routes;

use crate::events::WsMessage;
use anyhow::Result;
use mazuadm_core::executor::Executor;
use mazuadm_core::scheduler::{Scheduler, SchedulerCommand, SchedulerHandle, SchedulerRunner};
use mazuadm_core::settings::load_executor_settings;
use mazuadm_core::Database;
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub tx: broadcast::Sender<WsMessage>,
    pub executor: Executor,
    pub scheduler: SchedulerHandle,
}

fn should_resume_running_round(pending_count: usize) -> bool {
    pending_count > 0
}

async fn resume_running_round_if_needed(state: &AppState) -> Result<()> {
    let rounds = state.db.get_active_rounds().await?;
    let running_round_id = rounds.iter().find(|r| r.status == "running").map(|r| r.id);
    if let Some(round_id) = running_round_id {
        let pending = state.db.get_pending_jobs(round_id).await?;
        if should_resume_running_round(pending.len()) {
            if let Err(e) = state.scheduler.send(SchedulerCommand::RunRound(round_id)) {
                tracing::error!("Round {} resume failed: {}", round_id, e);
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config_arg = mazuadm_core::config::find_config_arg(std::env::args_os())?;
    let config_path = mazuadm_core::config::resolve_config_path(config_arg)?;
    let config = match config_path {
        Some(path) => mazuadm_core::config::load_toml_config(&path)?,
        None => mazuadm_core::AppConfig::default(),
    };

    let db_url = std::env::var("DATABASE_URL")
        .ok()
        .or_else(|| config.database_url.clone())
        .unwrap_or_else(|| "postgres://localhost/mazuadm".to_string());
    let db = Database::connect(&db_url).await?;

    let settings = load_executor_settings(&db).await;
    let (tx, _) = broadcast::channel::<WsMessage>(256);
    let executor = Executor::new(db.clone(), tx.clone())?;
    executor.container_manager.set_concurrent_create_limit(settings.concurrent_create_limit);
    executor.container_manager.restore_from_docker().await?;
    // Reset any jobs stuck in "running" state from previous run
    let reset = db.reset_stale_jobs().await?;
    if reset > 0 {
        tracing::warn!("Reset {} stale running jobs to stopped status", reset);
    }
    
    let scheduler = Scheduler::new(db.clone(), executor.clone(), tx.clone());
    let (runner, scheduler_handle) = SchedulerRunner::new(scheduler);
    tokio::spawn(runner.run());

    let state = Arc::new(AppState { db, tx, executor, scheduler: scheduler_handle });
    resume_running_round_if_needed(state.as_ref()).await?;

    let app = routes::routes()
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = std::env::var("LISTEN_ADDR")
        .ok()
        .or_else(|| config.listen_addr)
        .unwrap_or_else(|| "0.0.0.0:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::should_resume_running_round;

    #[test]
    fn should_resume_when_pending_jobs_exist() {
        assert!(should_resume_running_round(1));
    }

    #[test]
    fn should_not_resume_when_no_pending_jobs() {
        assert!(!should_resume_running_round(0));
    }
}
