mod handlers;
mod routes;

use anyhow::Result;
use mazuadm_core::Database;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/mazuadm".to_string());
    let db = Database::connect(&db_url).await?;
    
    // Reset any jobs stuck in "running" state from previous run
    let reset = db.reset_stale_jobs().await?;
    if reset > 0 {
        tracing::warn!("Reset {} stale running jobs to error status", reset);
    }
    
    let state = Arc::new(AppState { db });

    let app = routes::routes()
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await?;
    Ok(())
}
