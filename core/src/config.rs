use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Default, Clone)]
pub struct AppConfig {
    pub database_url: Option<String>,
    pub listen_addr: Option<String>,
    pub db_pool: Option<DbPoolConfig>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct DbPoolConfig {
    pub max_connections: Option<u32>,
    pub min_connections: Option<u32>,
    pub acquire_timeout_secs: Option<u64>,
    pub idle_timeout_secs: Option<u64>,
    pub max_lifetime_secs: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DbPoolSettings {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub max_lifetime_secs: u64,
}

const DEFAULT_DB_MAX_CONNECTIONS: u32 = 75;
const DEFAULT_DB_MIN_CONNECTIONS: u32 = 1;
const DEFAULT_DB_ACQUIRE_TIMEOUT_SECS: u64 = 5;
const DEFAULT_DB_IDLE_TIMEOUT_SECS: u64 = 300;
const DEFAULT_DB_MAX_LIFETIME_SECS: u64 = 1800;

pub fn resolve_db_pool_settings(cfg: &AppConfig) -> DbPoolSettings {
    let pool = cfg.db_pool.as_ref();
    DbPoolSettings {
        max_connections: pool.and_then(|p| p.max_connections).unwrap_or(DEFAULT_DB_MAX_CONNECTIONS),
        min_connections: pool.and_then(|p| p.min_connections).unwrap_or(DEFAULT_DB_MIN_CONNECTIONS),
        acquire_timeout_secs: pool.and_then(|p| p.acquire_timeout_secs).unwrap_or(DEFAULT_DB_ACQUIRE_TIMEOUT_SECS),
        idle_timeout_secs: pool.and_then(|p| p.idle_timeout_secs).unwrap_or(DEFAULT_DB_IDLE_TIMEOUT_SECS),
        max_lifetime_secs: pool.and_then(|p| p.max_lifetime_secs).unwrap_or(DEFAULT_DB_MAX_LIFETIME_SECS),
    }
}

pub fn find_config_arg<I, T>(args: I) -> Result<Option<PathBuf>>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString>,
{
    let mut iter = args.into_iter();
    let _ = iter.next();
    let mut last = None;

    while let Some(arg) = iter.next() {
        let arg_os: OsString = arg.into();
        let arg_s = arg_os.to_string_lossy();

        if arg_s == "--config" {
            if let Some(value) = iter.next() {
                let value_os: OsString = value.into();
                last = Some(PathBuf::from(value_os));
            } else {
                bail!("--config requires a value");
            }
            continue;
        }

        if let Some(rest) = arg_s.strip_prefix("--config=") {
            if rest.is_empty() {
                bail!("--config requires a value");
            }
            last = Some(PathBuf::from(rest));
        }
    }

    Ok(last)
}

pub fn resolve_config_path(explicit: Option<PathBuf>) -> Result<Option<PathBuf>> {
    if let Some(path) = explicit {
        if path.exists() {
            return Ok(Some(path));
        }
        bail!("config file not found: {}", path.display());
    }

    if let Ok(path) = std::env::var("MAZUADM_CONFIG") {
        if !path.is_empty() {
            let pb = PathBuf::from(path);
            if pb.exists() {
                return Ok(Some(pb));
            }
            bail!("config file not found: {}", pb.display());
        }
    }

    let mut candidates = Vec::new();
    candidates.push(PathBuf::from("/opt/mazuadm/config.toml"));

    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        candidates.push(PathBuf::from(xdg).join("mazuadm/config.toml"));
    } else if let Ok(home) = std::env::var("HOME") {
        candidates.push(PathBuf::from(home).join(".config/mazuadm/config.toml"));
    }

    for path in candidates {
        if path.exists() {
            return Ok(Some(path));
        }
    }

    Ok(None)
}

pub fn load_toml_config(path: &Path) -> Result<AppConfig> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config {}", path.display()))?;
    let config = toml::from_str(&contents)
        .with_context(|| format!("failed to parse config {}", path.display()))?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use tempfile::TempDir;

    struct EnvGuard {
        key: &'static str,
        prev: Option<OsString>,
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.prev.take() {
                Some(val) => env::set_var(self.key, val),
                None => env::remove_var(self.key),
            }
        }
    }

    fn set_env(key: &'static str, val: Option<&str>) -> EnvGuard {
        let prev = env::var_os(key);
        match val {
            Some(v) => env::set_var(key, v),
            None => env::remove_var(key),
        }
        EnvGuard { key, prev }
    }

    #[test]
    fn find_config_arg_parses_values() {
        let args = vec!["bin", "--config=one.toml", "--config", "two.toml"];
        let path = find_config_arg(args).unwrap();
        assert_eq!(path, Some(PathBuf::from("two.toml")));
    }

    #[test]
    fn resolve_config_path_prefers_explicit() {
        let dir = TempDir::new().unwrap();
        let explicit = dir.path().join("config.toml");
        fs::write(&explicit, "database_url = \"postgres://example\"\n").unwrap();
        let _g = set_env("MAZUADM_CONFIG", None);
        let path = resolve_config_path(Some(explicit.clone())).unwrap();
        assert_eq!(path, Some(explicit));
    }

    #[test]
    fn resolve_config_path_uses_env_var() {
        let dir = TempDir::new().unwrap();
        let env_path = dir.path().join("config.toml");
        fs::write(&env_path, "database_url = \"postgres://example\"\n").unwrap();
        let _g = set_env("MAZUADM_CONFIG", Some(env_path.to_str().unwrap()));
        let path = resolve_config_path(None).unwrap();
        assert_eq!(path, Some(env_path));
    }

    #[test]
    fn load_toml_config_reads_fields() {
        let dir = TempDir::new().unwrap();
        let cfg = dir.path().join("config.toml");
        fs::write(&cfg, "database_url = \"postgres://example\"\nlisten_addr = \"0.0.0.0:4000\"\n").unwrap();
        let parsed = load_toml_config(&cfg).unwrap();
        assert_eq!(parsed.database_url.as_deref(), Some("postgres://example"));
        assert_eq!(parsed.listen_addr.as_deref(), Some("0.0.0.0:4000"));
    }

    #[test]
    fn resolve_db_pool_defaults_when_missing() {
        let cfg = AppConfig::default();
        let settings = resolve_db_pool_settings(&cfg);
        assert_eq!(settings.max_connections, DEFAULT_DB_MAX_CONNECTIONS);
        assert_eq!(settings.min_connections, DEFAULT_DB_MIN_CONNECTIONS);
        assert_eq!(settings.acquire_timeout_secs, DEFAULT_DB_ACQUIRE_TIMEOUT_SECS);
        assert_eq!(settings.idle_timeout_secs, DEFAULT_DB_IDLE_TIMEOUT_SECS);
        assert_eq!(settings.max_lifetime_secs, DEFAULT_DB_MAX_LIFETIME_SECS);
    }

    #[test]
    fn resolve_db_pool_overrides_selected_fields() {
        let cfg = AppConfig {
            db_pool: Some(DbPoolConfig {
                max_connections: Some(10),
                idle_timeout_secs: Some(60),
                ..Default::default()
            }),
            ..Default::default()
        };
        let settings = resolve_db_pool_settings(&cfg);
        assert_eq!(settings.max_connections, 10);
        assert_eq!(settings.min_connections, DEFAULT_DB_MIN_CONNECTIONS);
        assert_eq!(settings.acquire_timeout_secs, DEFAULT_DB_ACQUIRE_TIMEOUT_SECS);
        assert_eq!(settings.idle_timeout_secs, 60);
        assert_eq!(settings.max_lifetime_secs, DEFAULT_DB_MAX_LIFETIME_SECS);
    }
}
