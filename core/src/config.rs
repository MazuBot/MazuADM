use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Default, Clone)]
pub struct AppConfig {
    pub database_url: Option<String>,
    pub listen_addr: Option<String>,
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
    candidates.push(PathBuf::from("/etc/mazuadm/config.toml"));

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
}
