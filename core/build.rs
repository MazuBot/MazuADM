use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize)]
struct BuildConfig {
    database_url: Option<String>,
}

fn main() {
    println!("cargo:rerun-if-env-changed=DATABASE_URL");
    println!("cargo:rerun-if-env-changed=MAZUADM_CONFIG");
    println!("cargo:rerun-if-changed=/etc/mazuadm/config.toml");

    if env::var_os("DATABASE_URL").is_some() {
        return;
    }

    let config_path = match env::var("MAZUADM_CONFIG") {
        Ok(path) if !path.trim().is_empty() => PathBuf::from(path),
        _ => PathBuf::from("/etc/mazuadm/config.toml"),
    };

    if !config_path.exists() {
        println!(
            "cargo:warning=DATABASE_URL not set and config not found at {}",
            config_path.display()
        );
        return;
    }

    if let Ok(path) = env::var("MAZUADM_CONFIG") {
        if !path.trim().is_empty() {
            println!("cargo:rerun-if-changed={}", path);
        }
    }

    let contents = match fs::read_to_string(&config_path) {
        Ok(contents) => contents,
        Err(err) => {
            println!(
                "cargo:warning=failed to read config {}: {}",
                config_path.display(),
                err
            );
            return;
        }
    };

    let config: BuildConfig = match toml::from_str(&contents) {
        Ok(config) => config,
        Err(err) => {
            println!(
                "cargo:warning=failed to parse config {}: {}",
                config_path.display(),
                err
            );
            return;
        }
    };

    if let Some(url) = config.database_url {
        println!("cargo:rustc-env=DATABASE_URL={}", url);
    } else {
        println!(
            "cargo:warning=config {} missing database_url",
            config_path.display()
        );
    }
}
