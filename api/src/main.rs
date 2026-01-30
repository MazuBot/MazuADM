mod events;
mod handlers;
mod routes;

use crate::events::WsMessage;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use mazuadm_core::executor::Executor;
use mazuadm_core::scheduler::{Scheduler, SchedulerCommand, SchedulerHandle, SchedulerRunner};
use mazuadm_core::settings::load_executor_settings;
use mazuadm_core::Database;
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs::OpenOptions;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::compression::CompressionLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Layer;
use tracing_subscriber::util::SubscriberInitExt;
use uuid::Uuid;

pub struct WsConnection {
    pub client_ip: String,
    pub client_name: String,
    pub user: String,
    pub subscribed_events: HashSet<String>,
    pub connected_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub tx: broadcast::Sender<WsMessage>,
    pub scheduler: SchedulerHandle,
    pub ws_connections: Arc<DashMap<Uuid, WsConnection>>,
}

#[cfg(debug_assertions)]
const CONSOLE_ENV: &str = "MAZUADM_CONSOLE";

#[cfg(debug_assertions)]
fn console_log_enabled() -> bool {
    std::env::var(CONSOLE_ENV).ok().as_deref() == Some("1")
}

#[cfg(not(debug_assertions))]
fn console_log_enabled() -> bool {
    false
}

fn default_log_level() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "info"
    }
}

#[cfg(debug_assertions)]
fn ensure_default_rust_log() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", default_log_level());
    }
}

#[cfg(not(debug_assertions))]
fn ensure_default_rust_log() {}

fn should_resume_running_round(pending_count: usize) -> bool {
    pending_count > 0
}

fn init_logging(log_dir: &Path) -> Result<Option<WorkerGuard>> {
    ensure_default_rust_log();
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_log_level()));
    let console_layer = if cfg!(debug_assertions) {
        Some(
            console_subscriber::ConsoleLayer::builder()
                .with_default_env()
                .spawn(),
        )
    } else {
        None
    };

    if console_log_enabled() {
        let registry = tracing_subscriber::registry()
            .with(console_layer)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(std::io::stdout)
                    .with_filter(env_filter),
            );
        registry.init();
        return Ok(None);
    }
    let log_path = log_dir.join("mazuadm-api.log");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("failed to open log {}", log_path.display()))?;
    let (writer, guard) = tracing_appender::non_blocking(file);
    let registry = tracing_subscriber::registry()
        .with(console_layer)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(writer)
                .with_ansi(false)
                .with_filter(env_filter),
        );
    registry.init();
    Ok(Some(guard))
}

fn parse_config_dir<I, T>(args: I) -> Result<Option<PathBuf>>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let mut iter = args.into_iter();
    let _ = iter.next();
    let mut config_dir = None;

    while let Some(arg) = iter.next() {
        let arg_os: OsString = arg.into();
        let arg_s = arg_os.to_string_lossy();

        if arg_s.starts_with('-') {
            bail!("unexpected argument: {}", arg_s);
        }

        if config_dir.is_some() {
            bail!("unexpected argument: {}", arg_s);
        }
        config_dir = Some(PathBuf::from(arg_os));
    }

    Ok(config_dir)
}

fn resolve_config_from_dir(config_dir: Option<PathBuf>) -> Result<Option<PathBuf>> {
    let (dir, explicit) = match config_dir {
        Some(dir) => (dir, true),
        None => (std::env::current_dir()?, false),
    };
    let path = dir.join("config.toml");
    if path.exists() {
        Ok(Some(path))
    } else if explicit {
        bail!("config file not found: {}", path.display());
    } else {
        Ok(None)
    }
}

async fn resume_running_round_if_needed(state: &AppState) -> Result<()> {
    let rounds = state.db.get_active_rounds().await?;
    let running_round_id = rounds.iter().find(|r| r.status == "running").map(|r| r.id);
    if let Some(round_id) = running_round_id {
        let pending = state.db.get_pending_jobs(round_id).await?;
        if should_resume_running_round(pending.len()) {
            if let Err(e) = state.scheduler.send(SchedulerCommand::RunPending(round_id)) {
                tracing::error!("Round {} resume failed: {}", round_id, e);
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let config_dir = parse_config_dir(std::env::args_os())?;
    let log_dir = match &config_dir {
        Some(dir) => dir.clone(),
        None => std::env::current_dir()?,
    };
    let _log_guard = init_logging(&log_dir)?;
    let config_path = resolve_config_from_dir(config_dir)?;
    let config = match config_path {
        Some(path) => mazuadm_core::config::load_toml_config(&path)?,
        None => mazuadm_core::AppConfig::default(),
    };

    let db_url = std::env::var("DATABASE_URL")
        .ok()
        .or_else(|| config.database_url.clone())
        .unwrap_or_else(|| "postgres://localhost/mazuadm".to_string());
    let db = Database::connect(&db_url, &config).await?;

    let settings = load_executor_settings(&db).await;
    let (tx, _) = broadcast::channel::<WsMessage>(256);
    let executor = Executor::new(db.clone(), tx.clone())?;
    executor.container_manager.set_concurrent_create_limit(settings.concurrent_create_limit);
    executor.restore_from_docker().await?;

    let scheduler = Scheduler::new(db.clone(), tx.clone());
    let (runner, scheduler_handle) = SchedulerRunner::new(scheduler, executor);
    tokio::spawn(runner.run());

    if let Err(e) = scheduler_handle.restart_all_containers(Some(0), true).await {
        tracing::warn!("Failed to restart containers on startup: {}", e);
    }

    let running_round_id = db
        .get_active_rounds()
        .await?
        .iter()
        .find(|r| r.status == "running")
        .map(|r| r.id);
    let cleanup = db.cleanup_stale_scheduled_jobs(running_round_id).await?;
    if cleanup.marked > 0 || cleanup.requeued > 0 {
        tracing::warn!(
            "Marked {} stale scheduled jobs; requeued {}",
            cleanup.marked,
            cleanup.requeued
        );
    }

    // Reset any jobs stuck in "running" state from previous run
    let reset = db.reset_stale_jobs().await?;
    if reset > 0 {
        tracing::warn!("Reset {} stale running jobs to stopped status", reset);
    }

    let state = Arc::new(AppState { db, tx, scheduler: scheduler_handle, ws_connections: Arc::new(DashMap::new()) });
    resume_running_round_if_needed(state.as_ref()).await?;

    let app = routes::routes()
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new());

    let addr = std::env::var("LISTEN_ADDR")
        .ok()
        .or_else(|| config.listen_addr)
        .unwrap_or_else(|| "0.0.0.0:3000".to_string());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_config_dir, resolve_config_from_dir, should_resume_running_round};
    use std::fs;
    use std::path::PathBuf;

    struct CwdGuard {
        prev: PathBuf,
    }

    impl Drop for CwdGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.prev);
        }
    }

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "mazuadm-api-test-{}",
            uuid::Uuid::new_v4()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn set_cwd(path: &PathBuf) -> CwdGuard {
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(path).unwrap();
        CwdGuard { prev }
    }

    #[test]
    fn should_resume_when_pending_jobs_exist() {
        assert!(should_resume_running_round(1));
    }

    #[test]
    fn should_not_resume_when_no_pending_jobs() {
        assert!(!should_resume_running_round(0));
    }

    #[test]
    fn parse_config_dir_accepts_optional_dir() {
        let args = vec!["bin", "/opt/mazuadm"];
        let config_dir = parse_config_dir(args).unwrap();
        assert_eq!(config_dir, Some(PathBuf::from("/opt/mazuadm")));
    }

    #[test]
    fn parse_config_dir_rejects_extra_args() {
        let args = vec!["bin", "one", "two"];
        assert!(parse_config_dir(args).is_err());
    }

    #[test]
    fn resolve_config_from_dir_uses_cwd_when_missing_dir_arg() {
        let dir = temp_dir();
        fs::write(dir.join("config.toml"), "listen_addr = \"127.0.0.1:3000\"\n").unwrap();
        {
            let _guard = set_cwd(&dir);
            let path = resolve_config_from_dir(None).unwrap();
            assert_eq!(path, Some(dir.join("config.toml")));
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_config_from_dir_errors_on_missing_explicit_dir() {
        let dir = temp_dir();
        let err = resolve_config_from_dir(Some(dir.clone())).unwrap_err();
        assert!(err.to_string().contains("config.toml"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolve_config_from_dir_returns_none_when_missing_cwd() {
        let dir = temp_dir();
        {
            let _guard = set_cwd(&dir);
            let path = resolve_config_from_dir(None).unwrap();
            assert!(path.is_none());
        }
        let _ = fs::remove_dir_all(&dir);
    }
}
