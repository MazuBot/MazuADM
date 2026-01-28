use anyhow::Result;
use clap::{Parser, Subcommand};
use mazuadm_core::*;
use tabled::{Table, Tabled};

mod exploit_config;

#[derive(Parser)]
#[command(name = "mazuadm", about = "MazuADM - CTF Attack/Defense Manager CLI")]
struct Cli {
    #[arg(long, global = true, value_name = "PATH", help = "Path to TOML config (overrides MAZUADM_CONFIG and default search)")]
    config: Option<std::path::PathBuf>,
    #[arg(long, env = "DATABASE_URL")]
    db: Option<String>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Manage challenges
    Challenge { #[command(subcommand)] cmd: ChallengeCmd },
    /// Manage teams
    Team { #[command(subcommand)] cmd: TeamCmd },
    /// Manage exploits
    Exploit { #[command(subcommand)] cmd: ExploitCmd },
    /// Manage exploit runs
    Run { #[command(subcommand)] cmd: RunCmd },
    /// Manage rounds
    Round { #[command(subcommand)] cmd: RoundCmd },
    /// Manage jobs
    Job { #[command(subcommand)] cmd: JobCmd },
    /// Manage flags
    Flag { #[command(subcommand)] cmd: FlagCmd },
    /// Manage settings
    Setting { #[command(subcommand)] cmd: SettingCmd },
    /// Manage containers
    Container { #[command(subcommand)] cmd: ContainerCmd },
    /// Manage challenge-team relations
    Relation { #[command(subcommand)] cmd: RelationCmd },
}

#[derive(Subcommand)]
enum ChallengeCmd {
    /// Add a new challenge
    Add { #[arg(long)] name: String, #[arg(long)] port: Option<i32>, #[arg(long)] priority: Option<i32>, #[arg(long)] flag_regex: Option<String> },
    /// List all challenges
    List,
    /// Update a challenge
    Update { id: i32, #[arg(long)] name: Option<String>, #[arg(long)] port: Option<i32>, #[arg(long)] priority: Option<i32>, #[arg(long)] flag_regex: Option<String> },
    /// Delete a challenge
    Delete { id: i32 },
    /// Enable a challenge
    Enable { id: i32 },
    /// Disable a challenge
    Disable { id: i32 },
}

#[derive(Subcommand)]
enum TeamCmd {
    /// Add a new team
    Add { #[arg(long)] id: String, #[arg(long)] name: String, #[arg(long)] ip: Option<String>, #[arg(long)] priority: Option<i32> },
    /// List all teams
    List,
    /// Update a team
    Update { id: i32, #[arg(long)] team_id: Option<String>, #[arg(long)] name: Option<String>, #[arg(long)] ip: Option<String>, #[arg(long)] priority: Option<i32> },
    /// Delete a team
    Delete { id: i32 },
    /// Enable a team
    Enable { id: i32 },
    /// Disable a team
    Disable { id: i32 },
}

#[derive(Subcommand)]
enum ExploitCmd {
    /// Create a new exploit from template config
    Create {
        #[arg(value_name = "NAME", default_value = ".", num_args = 0..=1)]
        name: String,
        #[arg(long)] challenge: Option<String>,
        #[arg(long, value_name = "PATH", num_args = 0..=1, default_missing_value = "config.toml")]
        config: Option<std::path::PathBuf>,
    },
    /// List exploits
    List { #[arg(long)] challenge: Option<String> },
    /// Update an exploit
    Update {
        name: String,
        #[arg(long)] challenge: Option<String>,
        #[arg(long, value_name = "PATH", num_args = 0..=1, default_missing_value = "config.toml")]
        config: Option<std::path::PathBuf>,
        #[arg(long)] image: Option<String>,
        #[arg(long)] entrypoint: Option<String>,
        #[arg(long)] priority: Option<i32>,
        #[arg(long)] max_per_container: Option<i32>,
        #[arg(long)] timeout: Option<i32>,
        #[arg(long)] default_counter: Option<i32>,
    },
    /// Delete an exploit
    Delete { name: String, #[arg(long)] challenge: Option<String> },
    /// Enable an exploit
    Enable { name: String, #[arg(long)] challenge: Option<String> },
    /// Disable an exploit
    Disable { name: String, #[arg(long)] challenge: Option<String> },
    /// Run exploit immediately against a team
    Run { name: String, #[arg(long)] challenge: Option<String>, #[arg(long)] team: i32 },
}

#[derive(Subcommand)]
enum RunCmd {
    /// Add a new exploit run
    Add { #[arg(long)] exploit: i32, #[arg(long)] challenge: i32, #[arg(long)] team: i32, #[arg(long)] priority: Option<i32>, #[arg(long)] sequence: Option<i32> },
    /// List exploit runs
    List { #[arg(long)] challenge: Option<i32>, #[arg(long)] team: Option<i32> },
    /// Update an exploit run
    Update { id: i32, #[arg(long)] priority: Option<i32>, #[arg(long)] sequence: Option<i32> },
    /// Delete an exploit run
    Delete { id: i32 },
}

#[derive(Subcommand)]
enum RoundCmd {
    /// Create a new round
    New,
    /// List all rounds
    List,
    /// Run a round
    Run { id: i32 },
    /// Clean all round data
    Clean,
}

#[derive(Subcommand)]
enum JobCmd {
    /// List jobs for a round
    List { #[arg(long)] round: i32 },
    /// Run a job immediately
    Run { id: i32 },
    /// Set job priority
    SetPriority { id: i32, priority: i32 },
}

#[derive(Subcommand)]
enum FlagCmd {
    /// List flags
    List { #[arg(long)] round: Option<i32> },
}

#[derive(Subcommand)]
enum SettingCmd {
    /// List all settings
    List,
    /// Set a setting value
    Set { key: String, value: String },
}

#[derive(Subcommand)]
enum ContainerCmd {
    /// List all containers
    List,
    /// Show runners for a container
    Runners { id: i32 },
    /// Delete a container
    Delete { id: i32 },
    /// Restart a container
    Restart { id: i32 },
}

#[derive(Subcommand)]
enum RelationCmd {
    /// List relations for a challenge
    List { challenge: i32 },
    /// Get a specific relation
    Get { challenge: i32, team: i32 },
    /// Update a relation
    Update { challenge: i32, team: i32, #[arg(long)] ip: Option<String>, #[arg(long)] port: Option<i32> },
}

#[derive(Tabled)] struct ChallengeRow { id: i32, name: String, enabled: bool, port: String, priority: i32 }
#[derive(Tabled)] struct TeamRow { id: i32, team_id: String, name: String, enabled: bool, ip: String, priority: i32 }
#[derive(Tabled)] struct ExploitRow { id: i32, name: String, enabled: bool, challenge: i32, image: String, priority: i32 }
#[derive(Tabled)] struct RunRow { id: i32, exploit: i32, challenge: i32, team: i32, priority: String, seq: i32 }
#[derive(Tabled)] struct RoundRow { id: i32, status: String, started: String }
#[derive(Tabled)] struct JobRow { id: i32, run: String, team: i32, priority: i32, status: String }
#[derive(Tabled)] struct FlagRow { id: i32, round: String, challenge: i32, team: i32, flag: String, status: String }
#[derive(Tabled)] struct SettingRow { key: String, value: String }
#[derive(Tabled)] struct ContainerRow { id: i32, container_id: String, exploit: i32, status: String, counter: i32 }
#[derive(Tabled)] struct RunnerRow { id: i32, container: i32, run: i32, team: i32 }
#[derive(Tabled)] struct RelationRow { challenge: i32, team: i32, addr: String, port: String }

fn normalize_name(value: Option<String>) -> Option<String> {
    value.and_then(|v| {
        let trimmed = v.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

struct TeamRefParts {
    team_id: String,
    numeric_id: Option<i32>,
}

fn parse_team_ref(value: &str) -> TeamRefParts {
    let trimmed = value.trim();
    let numeric_id = trimmed.parse::<i32>().ok();
    TeamRefParts {
        team_id: trimmed.to_string(),
        numeric_id,
    }
}

fn resolve_team_ref_sync<F, G>(team_ref: &str, mut by_team_id: F, mut by_id: G) -> Result<Team>
where
    F: FnMut(&str) -> Option<Team>,
    G: FnMut(i32) -> Option<Team>,
{
    let parts = parse_team_ref(team_ref);
    if let Some(team) = by_team_id(&parts.team_id) {
        return Ok(team);
    }
    if let Some(id) = parts.numeric_id {
        if let Some(team) = by_id(id) {
            return Ok(team);
        }
    }
    Err(anyhow::anyhow!("team not found: {}", team_ref))
}

async fn resolve_team_ref(db: &Database, team_ref: &str) -> Result<Team> {
    let parts = parse_team_ref(team_ref);
    if let Some(team) = db.get_team_by_team_id(&parts.team_id).await? {
        return Ok(team);
    }
    if let Some(id) = parts.numeric_id {
        return db.get_team(id).await;
    }
    Err(anyhow::anyhow!("team not found: {}", team_ref))
}

fn cwd_basename() -> Result<String> {
    let dir = std::env::current_dir()?;
    let name = dir
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| anyhow::anyhow!("failed to determine current directory name"))?;
    Ok(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_team(id: i32, team_id: &str) -> Team {
        Team {
            id,
            team_id: team_id.to_string(),
            team_name: "Test".to_string(),
            default_ip: None,
            priority: 0,
            created_at: Utc::now(),
            enabled: true,
        }
    }

    #[test]
    fn test_parse_team_ref_numeric() {
        let parsed = parse_team_ref("007");
        assert_eq!(parsed.team_id, "007");
        assert_eq!(parsed.numeric_id, Some(7));
    }

    #[test]
    fn test_resolve_team_ref_prefers_team_id() {
        let preferred = make_team(10, "1");
        let fallback = make_team(1, "team1");
        let found = resolve_team_ref_sync(
            "1",
            |value| if value == "1" { Some(preferred.clone()) } else { None },
            |id| if id == 1 { Some(fallback.clone()) } else { None },
        )
        .unwrap();
        assert_eq!(found.id, 10);
        assert_eq!(found.team_id, "1");
    }

    #[test]
    fn test_resolve_team_ref_fallback_numeric() {
        let fallback = make_team(42, "team42");
        let found = resolve_team_ref_sync(
            "42",
            |_| None,
            |id| if id == 42 { Some(fallback.clone()) } else { None },
        )
        .unwrap();
        assert_eq!(found.id, 42);
    }

    #[test]
    fn test_resolve_team_ref_missing() {
        let err = resolve_team_ref_sync("missing", |_| None, |_| None).unwrap_err();
        assert!(err.to_string().contains("team not found"));
    }
}

async fn prompt_challenge(db: &Database) -> Result<Challenge> {
    let challenges = db.list_challenges().await?;
    if challenges.is_empty() {
        return Err(anyhow::anyhow!("no challenges found"));
    }

    println!("Select a challenge:");
    for (idx, challenge) in challenges.iter().enumerate() {
        println!(
            "  {}) {} (id: {}, enabled: {})",
            idx + 1,
            challenge.name,
            challenge.id,
            challenge.enabled
        );
    }
    print!("Enter number: ");
    use std::io::Write;
    std::io::stdout().flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let choice: usize = input.trim().parse().map_err(|_| anyhow::anyhow!("invalid choice"))?;
    if choice == 0 || choice > challenges.len() {
        return Err(anyhow::anyhow!("choice out of range"));
    }
    Ok(challenges[choice - 1].clone())
}

async fn resolve_challenge(db: &Database, name: Option<String>, cfg: Option<&exploit_config::ChallengeRef>) -> Result<Challenge> {
    if let Some(name) = normalize_name(name) {
        return db.get_challenge_by_name(&name).await;
    }
    if let Some(cfg) = cfg {
        if let Some(id) = cfg.as_id() {
            return db.get_challenge(id).await;
        }
        if let Some(name) = cfg.as_name() {
            return db.get_challenge_by_name(name).await;
        }
    }
    prompt_challenge(db).await
}

async fn resolve_exploit(db: &Database, challenge_id: i32, name: &str) -> Result<Exploit> {
    db.get_exploit_by_name(challenge_id, name).await
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = mazuadm_core::config::resolve_config_path(cli.config)?;
    let config = match config_path {
        Some(path) => mazuadm_core::config::load_toml_config(&path)?,
        None => mazuadm_core::AppConfig::default(),
    };
    let db_url = cli
        .db
        .or_else(|| config.database_url)
        .unwrap_or_else(|| "postgres://localhost/mazuadm".to_string());
    let db = Database::connect(&db_url).await?;

    match cli.cmd {
        Cmd::Challenge { cmd } => match cmd {
            ChallengeCmd::Add { name, port, priority, flag_regex } => {
                let c = db.create_challenge(CreateChallenge { name, enabled: Some(true), default_port: port, priority, flag_regex }).await?;
                db.ensure_relations(c.id).await?;
                println!("Created challenge {}", c.id);
            }
            ChallengeCmd::List => {
                let rows: Vec<_> = db.list_challenges().await?.into_iter().map(|c| ChallengeRow { id: c.id, name: c.name, enabled: c.enabled, port: c.default_port.map(|p| p.to_string()).unwrap_or_default(), priority: c.priority }).collect();
                println!("{}", Table::new(rows));
            }
            ChallengeCmd::Update { id, name, port, priority, flag_regex } => {
                let c = db.get_challenge(id).await?;
                db.update_challenge(id, CreateChallenge { name: name.unwrap_or(c.name), enabled: Some(c.enabled), default_port: port.or(c.default_port), priority: priority.or(Some(c.priority)), flag_regex: flag_regex.or(c.flag_regex) }).await?;
                println!("Updated challenge {}", id);
            }
            ChallengeCmd::Delete { id } => { db.delete_challenge(id).await?; println!("Deleted challenge {}", id); }
            ChallengeCmd::Enable { id } => { db.set_challenge_enabled(id, true).await?; println!("Enabled"); }
            ChallengeCmd::Disable { id } => { db.set_challenge_enabled(id, false).await?; println!("Disabled"); }
        },
        Cmd::Team { cmd } => match cmd {
            TeamCmd::Add { id, name, ip, priority } => {
                let t = db.create_team(CreateTeam { team_id: id, team_name: name, default_ip: ip, priority, enabled: Some(true) }).await?;
                for c in db.list_challenges().await? { let _ = db.create_relation(c.id, t.id, None, None).await; }
                println!("Created team {}", t.id);
            }
            TeamCmd::List => {
                let rows: Vec<_> = db.list_teams().await?.into_iter().map(|t| TeamRow { id: t.id, team_id: t.team_id, name: t.team_name, enabled: t.enabled, ip: t.default_ip.unwrap_or_default(), priority: t.priority }).collect();
                println!("{}", Table::new(rows));
            }
            TeamCmd::Update { id, team_id, name, ip, priority } => {
                let t = db.get_team(id).await?;
                db.update_team(id, CreateTeam { team_id: team_id.unwrap_or(t.team_id), team_name: name.unwrap_or(t.team_name), default_ip: ip.or(t.default_ip), priority: priority.or(Some(t.priority)), enabled: Some(t.enabled) }).await?;
                println!("Updated team {}", id);
            }
            TeamCmd::Delete { id } => { db.delete_team(id).await?; println!("Deleted team {}", id); }
            TeamCmd::Enable { id } => {
                db.set_team_enabled(id, true).await?;
                println!("Enabled");
            }
            TeamCmd::Disable { id } => {
                db.set_team_enabled(id, false).await?;
                println!("Disabled");
            }
        },
        Cmd::Exploit { cmd } => match cmd {
            ExploitCmd::Create { name, challenge, config } => {
                let cfg = match config {
                    Some(path) => exploit_config::load_exploit_config(&path)?,
                    None => exploit_config::load_default_exploit_config()?,
                };
                let name = if name == "." { cwd_basename()? } else { name };
                let name = normalize_name(Some(name)).ok_or_else(|| anyhow::anyhow!("missing exploit name"))?;
                let challenge = resolve_challenge(&db, challenge, cfg.challenge.as_ref()).await?;

                let docker_image = cfg.docker_image.ok_or_else(|| anyhow::anyhow!("missing image in config"))?;
                let entrypoint = cfg.entrypoint;
                let priority = cfg.priority;
                let max_per_container = cfg.max_per_container;
                let timeout_secs = cfg.timeout_secs;
                let default_counter = cfg.default_counter;
                let enabled = cfg.enabled.unwrap_or(true);

                let e = db
                    .create_exploit(CreateExploit {
                        name,
                        challenge_id: challenge.id,
                        docker_image,
                        entrypoint,
                        enabled: Some(enabled),
                        priority,
                        max_per_container,
                        timeout_secs,
                        default_counter,
                        auto_add: None,
                        insert_into_rounds: None,
                    })
                    .await?;
                println!("Created exploit {}", e.id);
            }
            ExploitCmd::List { challenge } => {
                let challenge_id = match normalize_name(challenge) {
                    Some(name) => Some(db.get_challenge_by_name(&name).await?.id),
                    None => None,
                };
                let rows: Vec<_> = db
                    .list_exploits(challenge_id)
                    .await?
                    .into_iter()
                    .map(|e| ExploitRow {
                        id: e.id,
                        name: e.name,
                        enabled: e.enabled,
                        challenge: e.challenge_id,
                        image: e.docker_image,
                        priority: e.priority,
                    })
                    .collect();
                println!("{}", Table::new(rows));
            }
            ExploitCmd::Update { name, challenge, config, image, entrypoint, priority, max_per_container, timeout, default_counter } => {
                let cfg = match config {
                    Some(path) => exploit_config::load_exploit_config(&path)?,
                    None => exploit_config::load_default_exploit_config()?,
                };
                let challenge = resolve_challenge(&db, challenge, cfg.challenge.as_ref()).await?;
                let exploit = resolve_exploit(&db, challenge.id, &name).await?;

                let name = normalize_name(Some(name)).unwrap_or(exploit.name);
                let docker_image = image.or(cfg.docker_image).unwrap_or(exploit.docker_image);
                let entrypoint = entrypoint.or(cfg.entrypoint).or(exploit.entrypoint);
                let enabled = cfg.enabled.unwrap_or(exploit.enabled);
                let priority = priority.or(cfg.priority).or(Some(exploit.priority));
                let max_per_container = max_per_container.or(cfg.max_per_container).or(Some(exploit.max_per_container));
                let timeout_secs = timeout.or(cfg.timeout_secs).or(Some(exploit.timeout_secs));
                let default_counter = default_counter.or(cfg.default_counter).or(Some(exploit.default_counter));

                db.update_exploit(
                    exploit.id,
                    UpdateExploit {
                        name,
                        docker_image,
                        entrypoint,
                        enabled: Some(enabled),
                        priority,
                        max_per_container,
                        timeout_secs,
                        default_counter,
                    },
                )
                .await?;
                println!("Updated exploit {}", exploit.id);
            }
            ExploitCmd::Delete { name, challenge } => {
                let challenge = resolve_challenge(&db, challenge, None).await?;
                let exploit = resolve_exploit(&db, challenge.id, &name).await?;
                db.delete_exploit(exploit.id).await?;
                println!("Deleted exploit {}", exploit.id);
            }
            ExploitCmd::Enable { name, challenge } => {
                let challenge = resolve_challenge(&db, challenge, None).await?;
                let exploit = resolve_exploit(&db, challenge.id, &name).await?;
                db.set_exploit_enabled(exploit.id, true).await?;
                println!("Enabled");
            }
            ExploitCmd::Disable { name, challenge } => {
                let challenge = resolve_challenge(&db, challenge, None).await?;
                let exploit = resolve_exploit(&db, challenge.id, &name).await?;
                db.set_exploit_enabled(exploit.id, false).await?;
                println!("Disabled");
            }
            ExploitCmd::Run { name, challenge, team } => {
                let challenge = resolve_challenge(&db, challenge, None).await?;
                let exploit = resolve_exploit(&db, challenge.id, &name).await?;
                let team_obj = db.get_team(team).await?;
                let run = db.create_exploit_run(CreateExploitRun { exploit_id: exploit.id, challenge_id: exploit.challenge_id, team_id: team, priority: None, sequence: None }).await?;
                let job = db.create_adhoc_job(run.id, team).await?;
                let relations = db.list_relations(challenge.id).await?;
                let rel = relations.iter().find(|r| r.team_id == team);
                let conn = rel.and_then(|r| r.connection_info(&challenge, &team_obj)).ok_or(anyhow::anyhow!("No connection info"))?;
                let (tx, _) = tokio::sync::broadcast::channel(1);
                let executor = mazuadm_core::executor::Executor::new(db.clone(), tx)?;
                let timeout = if exploit.timeout_secs > 0 { exploit.timeout_secs as u64 } else { 60 };
                println!("Running exploit {} against team {}...", exploit.name, team_obj.team_name);
                let result = executor.execute_job(&job, &run, &exploit, &conn, challenge.flag_regex.as_deref(), timeout, 50).await?;
                println!("Completed with {} flags", result.flags.len());
                for flag in result.flags { println!("  {}", flag); }
            }
        },
        Cmd::Run { cmd } => match cmd {
            RunCmd::Add { exploit, challenge, team, priority, sequence } => {
                let r = db.create_exploit_run(CreateExploitRun { exploit_id: exploit, challenge_id: challenge, team_id: team, priority, sequence }).await?;
                println!("Created run {}", r.id);
            }
            RunCmd::List { challenge, team } => {
                let rows: Vec<_> = db.list_exploit_runs(challenge, team).await?.into_iter().map(|r| RunRow { id: r.id, exploit: r.exploit_id, challenge: r.challenge_id, team: r.team_id, priority: r.priority.map(|p| p.to_string()).unwrap_or("-".into()), seq: r.sequence }).collect();
                println!("{}", Table::new(rows));
            }
            RunCmd::Update { id, priority, sequence } => {
                db.update_exploit_run(id, priority, sequence, None).await?;
                println!("Updated run {}", id);
            }
            RunCmd::Delete { id } => { db.delete_exploit_run(id).await?; println!("Deleted run {}", id); }
        },
        Cmd::Round { cmd } => match cmd {
            RoundCmd::New => {
                let scheduler = mazuadm_core::scheduler::Scheduler::new(db);
                let id = scheduler.generate_round().await?;
                println!("Created round {}", id);
            }
            RoundCmd::List => {
                let rows: Vec<_> = db.list_rounds().await?.into_iter().map(|r| RoundRow { id: r.id, status: r.status, started: r.started_at.format("%H:%M:%S").to_string() }).collect();
                println!("{}", Table::new(rows));
            }
            RoundCmd::Run { id } => {
                let (tx, _) = tokio::sync::broadcast::channel(1);
                let executor = mazuadm_core::executor::Executor::new(db, tx)?;
                executor.run_round(id).await?;
                println!("Round {} completed", id);
            }
            RoundCmd::Clean => {
                db.clean_rounds().await?;
                println!("Cleaned all round data");
            }
        },
        Cmd::Job { cmd } => match cmd {
            JobCmd::List { round } => {
                let rows: Vec<_> = db.list_jobs(round).await?.into_iter().map(|j| JobRow { id: j.id, run: j.exploit_run_id.map(|r| r.to_string()).unwrap_or("-".into()), team: j.team_id, priority: j.priority, status: j.status }).collect();
                println!("{}", Table::new(rows));
            }
            JobCmd::Run { id } => {
                let job = db.get_job(id).await?;
                db.update_job_status(id, "pending", false).await?;
                let run_id = job.exploit_run_id.ok_or(anyhow::anyhow!("Job has no exploit_run_id"))?;
                let run = db.get_exploit_run(run_id).await?;
                let exploit = db.get_exploit(run.exploit_id).await?;
                let challenge = db.get_challenge(run.challenge_id).await?;
                let team = db.get_team(job.team_id).await?;
                let relations = db.list_relations(challenge.id).await?;
                let rel = relations.iter().find(|r| r.team_id == team.id);
                let conn = rel.and_then(|r| r.connection_info(&challenge, &team)).ok_or(anyhow::anyhow!("No connection info"))?;
                let (tx, _) = tokio::sync::broadcast::channel(1);
                let executor = mazuadm_core::executor::Executor::new(db.clone(), tx)?;
                let timeout = if exploit.timeout_secs > 0 { exploit.timeout_secs as u64 } else { 60 };
                println!("Running job {}...", id);
                let result = executor.execute_job(&job, &run, &exploit, &conn, challenge.flag_regex.as_deref(), timeout, 50).await?;
                println!("Completed with {} flags", result.flags.len());
                for flag in result.flags { println!("  {}", flag); }
            }
            JobCmd::SetPriority { id, priority } => {
                db.update_job_priority(id, priority).await?;
                println!("Set job {} priority to {}", id, priority);
            }
        },
        Cmd::Flag { cmd } => match cmd {
            FlagCmd::List { round } => {
                let rows: Vec<_> = db.list_flags(round).await?.into_iter().map(|f| FlagRow { id: f.id, round: f.round_id.map(|r| r.to_string()).unwrap_or("-".into()), challenge: f.challenge_id, team: f.team_id, flag: f.flag_value, status: f.status }).collect();
                println!("{}", Table::new(rows));
            }
        },
        Cmd::Setting { cmd } => match cmd {
            SettingCmd::List => {
                let rows: Vec<_> = db.list_settings().await?.into_iter().map(|s| SettingRow { key: s.key, value: s.value }).collect();
                println!("{}", Table::new(rows));
            }
            SettingCmd::Set { key, value } => {
                db.set_setting(&key, &value).await?;
                println!("Set {} = {}", key, value);
            }
        },
        Cmd::Container { cmd } => match cmd {
            ContainerCmd::List => {
                let rows: Vec<_> = db.list_all_containers().await?.into_iter().map(|c| ContainerRow { id: c.id, container_id: c.container_id[..12.min(c.container_id.len())].to_string(), exploit: c.exploit_id, status: c.status, counter: c.counter }).collect();
                println!("{}", Table::new(rows));
            }
            ContainerCmd::Runners { id } => {
                let rows: Vec<_> = db.get_runners_for_container(id).await?.into_iter().map(|r| RunnerRow { id: r.id, container: r.exploit_container_id, run: r.exploit_run_id, team: r.team_id }).collect();
                println!("{}", Table::new(rows));
            }
            ContainerCmd::Delete { id } => {
                let cm: mazuadm_core::ContainerManager = mazuadm_core::ContainerManager::new(db.clone())?;
                cm.destroy_container(id).await?;
                println!("Deleted container {}", id);
            }
            ContainerCmd::Restart { id: _ } => {
                println!("Restart not implemented - delete and let it recreate");
            }
        },
        Cmd::Relation { cmd } => match cmd {
            RelationCmd::List { challenge } => {
                let rows: Vec<_> = db.list_relations(challenge).await?.into_iter().map(|r| RelationRow { challenge: r.challenge_id, team: r.team_id, addr: r.addr.unwrap_or_default(), port: r.port.map(|p| p.to_string()).unwrap_or_default() }).collect();
                println!("{}", Table::new(rows));
            }
            RelationCmd::Get { challenge, team } => {
                if let Some(r) = db.get_relation(challenge, team).await? {
                    println!("Challenge: {}, Team: {}, Addr: {}, Port: {}", r.challenge_id, r.team_id, r.addr.unwrap_or_default(), r.port.map(|p| p.to_string()).unwrap_or_default());
                } else { println!("Relation not found"); }
            }
            RelationCmd::Update { challenge, team, ip, port } => {
                db.update_relation(challenge, team, ip, port).await?;
                println!("Updated relation");
            }
        },
    }
    Ok(())
}
