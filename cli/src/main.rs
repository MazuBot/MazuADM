use anyhow::Result;
use clap::{Parser, Subcommand};
use mazuadm_core::*;
use tabled::{Table, Tabled};

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
    /// Add a new exploit
    Add { #[arg(long)] name: String, #[arg(long)] challenge: i32, #[arg(long)] image: String, #[arg(long)] entrypoint: Option<String>, #[arg(long)] priority: Option<i32>, #[arg(long)] max_per_container: Option<i32>, #[arg(long)] timeout: Option<i32> },
    /// List exploits
    List { #[arg(long)] challenge: Option<i32> },
    /// Update an exploit
    Update { id: i32, #[arg(long)] name: Option<String>, #[arg(long)] image: Option<String>, #[arg(long)] entrypoint: Option<String>, #[arg(long)] priority: Option<i32>, #[arg(long)] max_per_container: Option<i32>, #[arg(long)] timeout: Option<i32> },
    /// Delete an exploit
    Delete { id: i32 },
    /// Enable an exploit
    Enable { id: i32 },
    /// Disable an exploit
    Disable { id: i32 },
    /// Run exploit immediately against a team
    Run { #[arg(long)] id: i32, #[arg(long)] team: i32 },
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
            ExploitCmd::Add { name, challenge, image, priority, max_per_container, timeout, entrypoint } => {
                let e = db.create_exploit(CreateExploit { name, challenge_id: challenge, docker_image: image, entrypoint, enabled: Some(true), priority, max_per_container, timeout_secs: timeout, default_counter: None, auto_add: None, insert_into_rounds: None }).await?;
                println!("Created exploit {}", e.id);
            }
            ExploitCmd::List { challenge } => {
                let rows: Vec<_> = db.list_exploits(challenge).await?.into_iter().map(|e| ExploitRow { id: e.id, name: e.name, enabled: e.enabled, challenge: e.challenge_id, image: e.docker_image, priority: e.priority }).collect();
                println!("{}", Table::new(rows));
            }
            ExploitCmd::Update { id, name, image, entrypoint, priority, max_per_container, timeout } => {
                let e = db.get_exploit(id).await?;
                db.update_exploit(id, UpdateExploit { name: name.unwrap_or(e.name), docker_image: image.unwrap_or(e.docker_image), entrypoint: entrypoint.or(e.entrypoint), enabled: Some(e.enabled), priority: priority.or(Some(e.priority)), max_per_container: max_per_container.or(Some(e.max_per_container)), timeout_secs: timeout.or(Some(e.timeout_secs)), default_counter: Some(e.default_counter) }).await?;
                println!("Updated exploit {}", id);
            }
            ExploitCmd::Delete { id } => { db.delete_exploit(id).await?; println!("Deleted exploit {}", id); }
            ExploitCmd::Enable { id } => {
                db.set_exploit_enabled(id, true).await?;
                println!("Enabled");
            }
            ExploitCmd::Disable { id } => {
                db.set_exploit_enabled(id, false).await?;
                println!("Disabled");
            }
            ExploitCmd::Run { id, team } => {
                let exploit = db.get_exploit(id).await?;
                let challenge = db.get_challenge(exploit.challenge_id).await?;
                let team_obj = db.get_team(team).await?;
                let run = db.create_exploit_run(CreateExploitRun { exploit_id: id, challenge_id: exploit.challenge_id, team_id: team, priority: None, sequence: None }).await?;
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
