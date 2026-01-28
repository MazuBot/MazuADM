mod events;
mod handlers;
mod routes;

use crate::events::WsMessage;
use anyhow::Result;
use mazuadm_core::executor::Executor;
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
    
    let (tx, _) = broadcast::channel::<WsMessage>(256);
    let executor = Executor::new(db.clone(), tx.clone())?;
    // Reset any jobs stuck in "running" state from previous run
    let reset = db.reset_stale_jobs().await?;
    if reset > 0 {
        tracing::warn!("Reset {} stale running jobs to stopped status", reset);
    }
    
    let state = Arc::new(AppState { db, tx, executor });

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
