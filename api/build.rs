use std::process::Command;

fn cmd(args: &[&str]) -> String {
    Command::new(args[0]).args(&args[1..]).output().ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string()).unwrap_or_default()
}

fn main() {
    println!("cargo:rerun-if-changed=../.git/HEAD");
    println!("cargo:rerun-if-changed=../.git/index");

    let hash = cmd(&["git", "rev-parse", "HEAD"]);
    let dirty = !cmd(&["git", "status", "--porcelain"]).is_empty();
    let hash_display = if dirty { format!("{} (Changed)", hash) } else { hash };

    let tag = cmd(&["git", "describe", "--tags", "--exact-match"]);
    let branch = cmd(&["git", "rev-parse", "--abbrev-ref", "HEAD"]);
    let git_ref = if !tag.is_empty() { tag } else { branch };

    let build_time = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string();
    let rustc = cmd(&["rustc", "--version"]);

    println!("cargo:rustc-env=BUILD_GIT_HASH={}", hash_display);
    println!("cargo:rustc-env=BUILD_GIT_REF={}", git_ref);
    println!("cargo:rustc-env=BUILD_TIME={}", build_time);
    println!("cargo:rustc-env=BUILD_RUSTC={}", rustc);
}
