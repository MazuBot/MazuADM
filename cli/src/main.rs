use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use tabled::{Table, Tabled};

mod api;
mod exploit_config;
mod models;

use api::ApiClient;
use models::*;

#[derive(Parser)]
#[command(name = "mazuadm", about = "MazuADM CLI")]
struct Cli {
    #[arg(long, env = "MAZUADM_API_URL", default_value = "http://localhost:3000")]
    api: String,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Version,
    Challenge { #[command(subcommand)] cmd: ChallengeCmd },
    Team { #[command(subcommand)] cmd: TeamCmd },
    Exploit { #[command(subcommand)] cmd: ExploitCmd },
    Run { #[command(subcommand)] cmd: RunCmd },
    Round { #[command(subcommand)] cmd: RoundCmd },
    Job { #[command(subcommand)] cmd: JobCmd },
    Flag { #[command(subcommand)] cmd: FlagCmd },
    Setting { #[command(subcommand)] cmd: SettingCmd },
    Container { #[command(subcommand)] cmd: ContainerCmd },
    Relation { #[command(subcommand)] cmd: RelationCmd },
    Ws { #[command(subcommand)] cmd: WsCmd },
}

#[derive(Subcommand)]
enum ChallengeCmd {
    Add { #[arg(long)] name: String, #[arg(long)] port: Option<i32>, #[arg(long, value_parser = clap::value_parser!(i32).range(0..=99))] priority: Option<i32>, #[arg(long)] flag_regex: Option<String> },
    List,
    Update { challenge: String, #[arg(long)] name: Option<String>, #[arg(long)] port: Option<i32>, #[arg(long, value_parser = clap::value_parser!(i32).range(0..=99))] priority: Option<i32>, #[arg(long)] flag_regex: Option<String> },
    Delete { challenge: String },
    Enable { challenge: String },
    Disable { challenge: String },
}

#[derive(Subcommand)]
enum TeamCmd {
    Add { #[arg(long)] id: String, #[arg(long)] name: String, #[arg(long)] ip: Option<String>, #[arg(long, value_parser = clap::value_parser!(i32).range(0..=99))] priority: Option<i32> },
    List,
    Update { team: String, #[arg(long)] team_id: Option<String>, #[arg(long)] name: Option<String>, #[arg(long)] ip: Option<String>, #[arg(long, value_parser = clap::value_parser!(i32).range(0..=99))] priority: Option<i32> },
    Delete { team: String },
    Enable { team: String },
    Disable { team: String },
}

#[derive(Subcommand)]
enum ExploitCmd {
    Init { #[arg(default_value = ".")] name: String, #[arg(long)] challenge: Option<String> },
    Create { name: String, #[arg(long)] challenge: Option<String>, #[arg(long)] config: Option<std::path::PathBuf>, #[arg(long)] image: Option<String>, #[arg(long)] entrypoint: Option<String>, #[arg(long)] max_per_container: Option<i32>, #[arg(long)] max_containers: Option<i32>, #[arg(long)] max_concurrent_jobs: Option<i32>, #[arg(long)] timeout: Option<i32>, #[arg(long)] default_counter: Option<i32>, #[arg(long)] ignore_connection_info: Option<bool>, #[arg(long)] auto_add: Option<String>, #[arg(long)] insert_into_rounds: Option<bool> },
    Pack { #[arg(default_value = ".")] name: String, #[arg(long)] challenge: Option<String>, #[arg(long, default_missing_value = "config.toml")] config: Option<std::path::PathBuf>, #[arg(long)] dir: Option<std::path::PathBuf> },
    List { #[arg(long)] challenge: Option<String> },
    Update { name: String, #[arg(long)] challenge: Option<String>, #[arg(long, default_missing_value = "config.toml")] config: Option<std::path::PathBuf>, #[arg(long)] image: Option<String>, #[arg(long)] entrypoint: Option<String>, #[arg(long)] max_per_container: Option<i32>, #[arg(long)] max_containers: Option<i32>, #[arg(long)] max_concurrent_jobs: Option<i32>, #[arg(long)] timeout: Option<i32>, #[arg(long)] default_counter: Option<i32> },
    Delete { name: String, #[arg(long)] challenge: Option<String> },
    Enable { name: String, #[arg(long)] challenge: Option<String> },
    Disable { name: String, #[arg(long)] challenge: Option<String> },
    Run { name: String, #[arg(long)] challenge: Option<String>, #[arg(long)] team: String },
}

#[derive(Subcommand)]
enum RunCmd {
    Add { #[arg(long)] exploit: String, #[arg(long)] challenge: String, #[arg(long)] team: String, #[arg(long)] priority: Option<i32>, #[arg(long)] sequence: Option<i32> },
    List { #[arg(long)] challenge: Option<String>, #[arg(long)] team: Option<String> },
    Update { id: i32, #[arg(long)] priority: Option<i32>, #[arg(long)] sequence: Option<i32> },
    Delete { id: i32 },
    Enable { id: i32 },
    Disable { id: i32 },
    Reorder { #[arg(value_parser = parse_reorder_run)] items: Vec<(i32, i32)> },
    AppendAll { #[arg(long)] exploit: String, #[arg(long)] challenge: String, #[arg(long)] priority: Option<i32> },
    PrependAll { #[arg(long)] exploit: String, #[arg(long)] challenge: String, #[arg(long)] priority: Option<i32> },
}

fn parse_reorder_run(s: &str) -> Result<(i32, i32), String> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 { return Err("format: id:sequence".into()); }
    Ok((parts[0].parse().map_err(|_| "invalid id")?, parts[1].parse().map_err(|_| "invalid sequence")?))
}

#[derive(Subcommand)]
enum RoundCmd {
    New,
    List,
    Current,
    Run { id: i32 },
    Rerun { id: i32 },
    RerunUnflagged { id: i32 },
    Clean { #[arg(long, env = "DATABASE_URL")] db: String },
}

#[derive(Subcommand)]
enum JobCmd { List { #[arg(long)] round: i32 }, Get { id: i32 }, Run { id: i32 }, Stop { id: i32 }, SetPriority { id: i32, priority: i32 } }

#[derive(Subcommand)]
enum FlagCmd {
    List { #[arg(long)] round: Option<i32> },
    Submit {
        #[arg(long)] round: Option<i32>,
        #[arg(long)] challenge: String,
        #[arg(long)] team: String,
        flag: String,
    },
    Update { #[arg(long)] force: bool, #[arg(value_parser = parse_flag_update)] items: Vec<(i32, String)> },
}

fn parse_flag_update(s: &str) -> Result<(i32, String), String> {
    let parts: Vec<&str> = s.splitn(2, ':').collect();
    if parts.len() != 2 { return Err("format: id:status".into()); }
    Ok((parts[0].parse().map_err(|_| "invalid id")?, parts[1].to_string()))
}

#[derive(Subcommand)]
enum SettingCmd { List, Set { key: String, value: String } }

#[derive(Subcommand)]
enum ContainerCmd { List, Runners { id: String }, Delete { id: String }, Restart { id: String }, RestartAll, RemoveAll }

#[derive(Subcommand)]
enum WsCmd { List }

#[derive(Subcommand)]
enum RelationCmd { List { challenge: String }, Get { challenge: String, team: String }, Update { challenge: String, team: String, #[arg(long)] ip: Option<String>, #[arg(long)] port: Option<i32> }, Enable { challenge: String, team: String }, Disable { challenge: String, team: String } }

#[derive(Tabled)] struct ChallengeRow { id: i32, name: String, enabled: bool, port: String, priority: i32 }
#[derive(Tabled)] struct TeamRow { id: i32, team_id: String, name: String, enabled: bool, ip: String, priority: i32 }
#[derive(Tabled)] struct ExploitRow { id: i32, name: String, enabled: bool, challenge: i32, image: String }
#[derive(Tabled)] struct RunRow { id: i32, exploit: i32, challenge: i32, team_id: String, team_name: String, priority: String, seq: i32, enabled: bool }
#[derive(Tabled)] struct RoundRow { id: i32, status: String, started: String }
#[derive(Tabled)] struct JobRow { id: i32, run: String, team_id: String, team_name: String, priority: i32, status: String }
#[derive(Tabled)] struct FlagRow { id: i32, round: String, challenge: i32, team_id: String, team_name: String, flag: String, status: String }
#[derive(Tabled)] struct SettingRow { key: String, value: String }
#[derive(Tabled)] struct ContainerRow { id: String, exploit: i32, status: String, counter: i32, execs: String, affinity: String }
#[derive(Tabled)] struct RunnerRow { id: i32, run: String, team_id: String, team_name: String, status: String }
#[derive(Tabled)] struct RelationRow { challenge: i32, team_id: String, team_name: String, enabled: bool, addr: String, port: String }
#[derive(Tabled)] struct WsRow { id: String, client_ip: String, client_name: String, user: String, duration: String }

struct Ctx { api: ApiClient, challenges: Option<Vec<Challenge>>, teams: Option<Vec<Team>>, exploits: Option<Vec<Exploit>> }

impl Ctx {
    fn new(api: ApiClient) -> Self { Self { api, challenges: None, teams: None, exploits: None } }
    async fn challenges(&mut self) -> Result<&Vec<Challenge>> {
        if self.challenges.is_none() { self.challenges = Some(self.api.list_challenges().await?); }
        Ok(self.challenges.as_ref().unwrap())
    }
    async fn teams(&mut self) -> Result<&Vec<Team>> {
        if self.teams.is_none() { self.teams = Some(self.api.list_teams().await?); }
        Ok(self.teams.as_ref().unwrap())
    }
    async fn exploits(&mut self, challenge_id: Option<i32>) -> Result<&Vec<Exploit>> {
        if self.exploits.is_none() { self.exploits = Some(self.api.list_exploits(challenge_id).await?); }
        Ok(self.exploits.as_ref().unwrap())
    }
    async fn find_challenge(&mut self, s: &str) -> Result<Challenge> {
        let challenges = self.challenges().await?;
        if let Some(c) = challenges.iter().find(|c| c.name == s) { return Ok(c.clone()); }
        if let Ok(id) = s.parse::<i32>() { if let Some(c) = challenges.iter().find(|c| c.id == id) { return Ok(c.clone()); } }
        Err(anyhow!("challenge not found: {}", s))
    }
    async fn find_team(&mut self, s: &str) -> Result<Team> {
        let teams = self.teams().await?;
        if let Some(t) = teams.iter().find(|t| t.team_id == s) { return Ok(t.clone()); }
        if let Ok(id) = s.parse::<i32>() { if let Some(t) = teams.iter().find(|t| t.id == id) { return Ok(t.clone()); } }
        Err(anyhow!("team not found: {}", s))
    }
    async fn find_exploit(&mut self, challenge_id: i32, name: &str) -> Result<Exploit> {
        let exploits = self.exploits(Some(challenge_id)).await?;
        exploits.iter().find(|e| e.name == name && e.challenge_id == challenge_id).cloned().ok_or_else(|| anyhow!("exploit not found: {}", name))
    }
    fn team_label(&self, id: i32) -> (String, String) {
        self.teams.as_ref().and_then(|ts| ts.iter().find(|t| t.id == id)).map(|t| (t.team_id.clone(), t.team_name.clone())).unwrap_or_else(|| (id.to_string(), "-".into()))
    }
}

fn cwd_basename() -> Result<String> {
    std::env::current_dir()?.file_name().and_then(|v| v.to_str()).map(|s| s.to_string()).ok_or_else(|| anyhow!("failed to get cwd name"))
}

async fn prompt_challenge(ctx: &mut Ctx) -> Result<Challenge> {
    let challenges = ctx.challenges().await?;
    if challenges.is_empty() { return Err(anyhow!("no challenges")); }
    println!("Select challenge:");
    for (i, c) in challenges.iter().enumerate() { println!("  {}) {} (id: {})", i + 1, c.name, c.id); }
    print!("Enter number: ");
    use std::io::Write; std::io::stdout().flush()?;
    let mut input = String::new(); std::io::stdin().read_line(&mut input)?;
    let choice: usize = input.trim().parse()?;
    challenges.get(choice - 1).cloned().ok_or_else(|| anyhow!("invalid choice"))
}

async fn resolve_challenge(ctx: &mut Ctx, name: Option<String>, cfg: Option<&exploit_config::ChallengeRef>) -> Result<Challenge> {
    if let Some(n) = name.filter(|s| !s.trim().is_empty()) { return ctx.find_challenge(&n).await; }
    if let Some(c) = cfg { if let Some(n) = c.as_name() { return ctx.find_challenge(n).await; } }
    prompt_challenge(ctx).await
}


#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let mut ctx = Ctx::new(ApiClient::new(&cli.api));

    match cli.cmd {
        Cmd::Version => {
            println!("CLI:");
            println!("  Version:    {}", env!("CARGO_PKG_VERSION"));
            println!("  Git Ref:    {}", env!("BUILD_GIT_REF"));
            println!("  Git Hash:   {}", env!("BUILD_GIT_HASH"));
            println!("  Build Time: {}", env!("BUILD_TIME"));
            println!("  Rustc:      {}", env!("BUILD_RUSTC"));
            println!();
            println!("API:");
            match ctx.api.get_version().await {
                Ok(v) => {
                    println!("  Version:    {}", v.version);
                    println!("  Git Ref:    {}", v.git_ref);
                    println!("  Git Hash:   {}", v.git_hash);
                    println!("  Build Time: {}", v.build_time);
                    println!("  Rustc:      {}", v.rustc);
                }
                Err(e) => println!("  Error: {}", e),
            }
        }
        Cmd::Challenge { cmd } => match cmd {
            ChallengeCmd::Add { name, port, priority, flag_regex } => {
                let c = ctx.api.create_challenge(CreateChallenge { name, enabled: Some(true), default_port: port, priority, flag_regex }).await?;
                println!("Created challenge {}", c.id);
            }
            ChallengeCmd::List => {
                let rows: Vec<_> = ctx.api.list_challenges().await?.into_iter().map(|c| ChallengeRow { id: c.id, name: c.name, enabled: c.enabled, port: c.default_port.map(|p| p.to_string()).unwrap_or_default(), priority: c.priority }).collect();
                println!("{}", Table::new(rows));
            }
            ChallengeCmd::Update { challenge, name, port, priority, flag_regex } => {
                let c = ctx.find_challenge(&challenge).await?;
                ctx.api.update_challenge(c.id, CreateChallenge { name: name.unwrap_or(c.name), enabled: Some(c.enabled), default_port: port.or(c.default_port), priority: priority.or(Some(c.priority)), flag_regex: flag_regex.or(c.flag_regex) }).await?;
                println!("Updated");
            }
            ChallengeCmd::Delete { challenge } => { let c = ctx.find_challenge(&challenge).await?; ctx.api.delete_challenge(c.id).await?; println!("Deleted"); }
            ChallengeCmd::Enable { challenge } => { let c = ctx.find_challenge(&challenge).await?; ctx.api.set_challenge_enabled(c.id, true).await?; println!("Enabled"); }
            ChallengeCmd::Disable { challenge } => { let c = ctx.find_challenge(&challenge).await?; ctx.api.set_challenge_enabled(c.id, false).await?; println!("Disabled"); }
        },
        Cmd::Team { cmd } => match cmd {
            TeamCmd::Add { id, name, ip, priority } => {
                let t = ctx.api.create_team(CreateTeam { team_id: id, team_name: name, default_ip: ip, priority, enabled: Some(true) }).await?;
                println!("Created team {}", t.id);
            }
            TeamCmd::List => {
                let rows: Vec<_> = ctx.api.list_teams().await?.into_iter().map(|t| TeamRow { id: t.id, team_id: t.team_id, name: t.team_name, enabled: t.enabled, ip: t.default_ip.unwrap_or_default(), priority: t.priority }).collect();
                println!("{}", Table::new(rows));
            }
            TeamCmd::Update { team, team_id, name, ip, priority } => {
                let t = ctx.find_team(&team).await?;
                ctx.api.update_team(t.id, CreateTeam { team_id: team_id.unwrap_or(t.team_id), team_name: name.unwrap_or(t.team_name), default_ip: ip.or(t.default_ip), priority: priority.or(Some(t.priority)), enabled: Some(t.enabled) }).await?;
                println!("Updated");
            }
            TeamCmd::Delete { team } => { let t = ctx.find_team(&team).await?; ctx.api.delete_team(t.id).await?; println!("Deleted"); }
            TeamCmd::Enable { team } => {
                let t = ctx.find_team(&team).await?;
                ctx.api.update_team(t.id, CreateTeam { team_id: t.team_id, team_name: t.team_name, default_ip: t.default_ip, priority: Some(t.priority), enabled: Some(true) }).await?;
                println!("Enabled");
            }
            TeamCmd::Disable { team } => {
                let t = ctx.find_team(&team).await?;
                ctx.api.update_team(t.id, CreateTeam { team_id: t.team_id, team_name: t.team_name, default_ip: t.default_ip, priority: Some(t.priority), enabled: Some(false) }).await?;
                println!("Disabled");
            }
        },
        Cmd::Exploit { cmd } => match cmd {
            ExploitCmd::Init { name, challenge } => {
                let challenge = match challenge {
                    Some(c) => ctx.find_challenge(&c).await?,
                    None => prompt_challenge(&mut ctx).await?,
                };
                let target_dir = if name == "." { std::env::current_dir()? } else {
                    let dir = std::env::current_dir()?.join(&name);
                    std::fs::create_dir_all(&dir)?;
                    dir
                };
                let template_dir = std::path::Path::new("/opt/mazuadm/exp-template");
                if !template_dir.exists() { return Err(anyhow!("template not found at {}", template_dir.display())); }
                for entry in std::fs::read_dir(template_dir)? {
                    let entry = entry?;
                    let dest = target_dir.join(entry.file_name());
                    if entry.file_name() == "config.toml" {
                        let content = std::fs::read_to_string(entry.path())?;
                        std::fs::write(&dest, content.replace("challenge-name", &challenge.name))?;
                    } else {
                        std::fs::copy(entry.path(), &dest)?;
                    }
                }
                println!("Initialized exploit in {} for challenge {}", target_dir.display(), challenge.name);
            }
            ExploitCmd::Create { name, challenge, config, image, entrypoint, max_per_container, max_containers, max_concurrent_jobs, timeout, default_counter, ignore_connection_info, auto_add, insert_into_rounds } => {
                let cfg = match config { Some(p) => exploit_config::load_exploit_config(&p)?, None => exploit_config::ExploitConfig::default() };
                let challenge = resolve_challenge(&mut ctx, challenge, cfg.challenge.as_ref()).await?;
                let image = image.or(cfg.docker_image).ok_or_else(|| anyhow!("--image or config docker_image required"))?;
                let e = ctx.api.create_exploit(CreateExploit { name, challenge_id: challenge.id, docker_image: image, entrypoint: entrypoint.or(cfg.entrypoint), enabled: cfg.enabled.or(Some(true)), max_per_container: max_per_container.or(cfg.max_per_container), max_containers: max_containers.or(cfg.max_containers), max_concurrent_jobs: max_concurrent_jobs.or(cfg.max_concurrent_jobs), timeout_secs: timeout.or(cfg.timeout_secs), default_counter: default_counter.or(cfg.default_counter), ignore_connection_info: ignore_connection_info.or(cfg.ignore_connection_info), auto_add: auto_add.or(cfg.auto_add), insert_into_rounds: insert_into_rounds.or(cfg.insert_into_rounds), envs: None }).await?;
                println!("Created exploit {}", e.id);
            }
            ExploitCmd::Pack { name, challenge, config, dir } => {
                let original_dir = if let Some(d) = &dir { let orig = std::env::current_dir()?; std::env::set_current_dir(d)?; Some(orig) } else { None };
                let cfg = match config { Some(p) => exploit_config::load_exploit_config(&p)?, None => exploit_config::load_default_exploit_config()? };
                let name = if name != "." { name } else if let Some(n) = cfg.name.clone() { n } else { cwd_basename()? };
                let challenge = resolve_challenge(&mut ctx, challenge, cfg.challenge.as_ref()).await?;
                let image = cfg.docker_image.unwrap_or_else(|| name.to_lowercase().replace(|c: char| !c.is_alphanumeric(), "-"));
                println!("Building docker image: {}", image);
                let status = std::process::Command::new("docker").args(["build", "-t", &image, "."]).status()?;
                if !status.success() { return Err(anyhow!("docker build failed")); }
                let e = ctx.api.create_exploit(CreateExploit { name, challenge_id: challenge.id, docker_image: image, entrypoint: cfg.entrypoint, enabled: cfg.enabled, max_per_container: cfg.max_per_container, max_containers: cfg.max_containers, max_concurrent_jobs: cfg.max_concurrent_jobs, timeout_secs: cfg.timeout_secs, default_counter: cfg.default_counter, ignore_connection_info: cfg.ignore_connection_info, auto_add: cfg.auto_add, insert_into_rounds: cfg.insert_into_rounds, envs: None }).await?;
                println!("Created exploit {}", e.id);
                if let Some(orig) = original_dir { std::env::set_current_dir(orig)?; }
            }
            ExploitCmd::List { challenge } => {
                let cid = match challenge { Some(c) => Some(ctx.find_challenge(&c).await?.id), None => None };
                let rows: Vec<_> = ctx.api.list_exploits(cid).await?.into_iter().map(|e| ExploitRow { id: e.id, name: e.name, enabled: e.enabled, challenge: e.challenge_id, image: e.docker_image }).collect();
                println!("{}", Table::new(rows));
            }
            ExploitCmd::Update { name, challenge, config, image, entrypoint, max_per_container, max_containers, max_concurrent_jobs, timeout, default_counter } => {
                let cfg = match config { Some(p) => exploit_config::load_exploit_config(&p)?, None => exploit_config::load_default_exploit_config()? };
                let challenge = resolve_challenge(&mut ctx, challenge, cfg.challenge.as_ref()).await?;
                let e = ctx.find_exploit(challenge.id, &name).await?;
                ctx.api.update_exploit(e.id, UpdateExploit { name: e.name, docker_image: image.or(cfg.docker_image).unwrap_or(e.docker_image), entrypoint: entrypoint.or(cfg.entrypoint).or(e.entrypoint), enabled: cfg.enabled.or(Some(e.enabled)), max_per_container: max_per_container.or(cfg.max_per_container).or(Some(e.max_per_container)), max_containers: max_containers.or(cfg.max_containers), max_concurrent_jobs: max_concurrent_jobs.or(cfg.max_concurrent_jobs).or(Some(e.max_concurrent_jobs)), timeout_secs: timeout.or(cfg.timeout_secs).or(Some(e.timeout_secs)), default_counter: default_counter.or(cfg.default_counter).or(Some(e.default_counter)), ignore_connection_info: cfg.ignore_connection_info.or(Some(e.ignore_connection_info)), envs: e.envs }).await?;
                println!("Updated");
            }
            ExploitCmd::Delete { name, challenge } => {
                let challenge = resolve_challenge(&mut ctx, challenge, None).await?;
                let e = ctx.find_exploit(challenge.id, &name).await?;
                ctx.api.delete_exploit(e.id).await?;
                println!("Deleted");
            }
            ExploitCmd::Enable { name, challenge } => {
                let challenge = resolve_challenge(&mut ctx, challenge, None).await?;
                let e = ctx.find_exploit(challenge.id, &name).await?;
                ctx.api.update_exploit(e.id, UpdateExploit { name: e.name, docker_image: e.docker_image, entrypoint: e.entrypoint, enabled: Some(true), max_per_container: Some(e.max_per_container), max_containers: None, max_concurrent_jobs: Some(e.max_concurrent_jobs), timeout_secs: Some(e.timeout_secs), default_counter: Some(e.default_counter), ignore_connection_info: Some(e.ignore_connection_info), envs: e.envs }).await?;
                println!("Enabled");
            }
            ExploitCmd::Disable { name, challenge } => {
                let challenge = resolve_challenge(&mut ctx, challenge, None).await?;
                let e = ctx.find_exploit(challenge.id, &name).await?;
                ctx.api.update_exploit(e.id, UpdateExploit { name: e.name, docker_image: e.docker_image, entrypoint: e.entrypoint, enabled: Some(false), max_per_container: Some(e.max_per_container), max_containers: None, max_concurrent_jobs: Some(e.max_concurrent_jobs), timeout_secs: Some(e.timeout_secs), default_counter: Some(e.default_counter), ignore_connection_info: Some(e.ignore_connection_info), envs: e.envs }).await?;
                println!("Disabled");
            }
            ExploitCmd::Run { name, challenge, team } => {
                let challenge = resolve_challenge(&mut ctx, challenge, None).await?;
                let e = ctx.find_exploit(challenge.id, &name).await?;
                let t = ctx.find_team(&team).await?;
                let runs = ctx.api.list_exploit_runs(Some(challenge.id), Some(t.id)).await?;
                let run = runs.iter().find(|r| r.exploit_id == e.id).ok_or_else(|| anyhow!("no exploit run found"))?;
                let job = ctx.api.enqueue_single_job(EnqueueSingleJobRequest { exploit_run_id: run.id, team_id: t.id }).await?;
                println!("Enqueued job {}", job.id);
            }
        },
        Cmd::Run { cmd } => match cmd {
            RunCmd::Add { exploit, challenge, team, priority, sequence } => {
                let c = ctx.find_challenge(&challenge).await?;
                let e = ctx.find_exploit(c.id, &exploit).await?;
                let t = ctx.find_team(&team).await?;
                let r = ctx.api.create_exploit_run(CreateExploitRun { exploit_id: e.id, challenge_id: c.id, team_id: t.id, priority, sequence }).await?;
                println!("Created run {}", r.id);
            }
            RunCmd::List { challenge, team } => {
                let cid = match challenge { Some(c) => Some(ctx.find_challenge(&c).await?.id), None => None };
                let tid = match team { Some(t) => Some(ctx.find_team(&t).await?.id), None => None };
                ctx.teams().await?;
                let rows: Vec<_> = ctx.api.list_exploit_runs(cid, tid).await?.into_iter().map(|r| { let (tid, tn) = ctx.team_label(r.team_id); RunRow { id: r.id, exploit: r.exploit_id, challenge: r.challenge_id, team_id: tid, team_name: tn, priority: r.priority.map(|p| p.to_string()).unwrap_or("-".into()), seq: r.sequence, enabled: r.enabled } }).collect();
                println!("{}", Table::new(rows));
            }
            RunCmd::Update { id, priority, sequence } => { ctx.api.update_exploit_run(id, UpdateExploitRun { priority, sequence, enabled: None }).await?; println!("Updated"); }
            RunCmd::Delete { id } => { ctx.api.delete_exploit_run(id).await?; println!("Deleted"); }
            RunCmd::Enable { id } => { ctx.api.update_exploit_run(id, UpdateExploitRun { priority: None, sequence: None, enabled: Some(true) }).await?; println!("Enabled"); }
            RunCmd::Disable { id } => { ctx.api.update_exploit_run(id, UpdateExploitRun { priority: None, sequence: None, enabled: Some(false) }).await?; println!("Disabled"); }
            RunCmd::Reorder { items } => {
                ctx.api.reorder_exploit_runs(items.into_iter().map(|(id, sequence)| ReorderExploitRunItem { id, sequence }).collect()).await?;
                println!("Reordered");
            }
            RunCmd::AppendAll { exploit, challenge, priority } => {
                let c = ctx.find_challenge(&challenge).await?;
                let e = ctx.find_exploit(c.id, &exploit).await?;
                let teams = ctx.api.list_teams().await?;
                for t in &teams {
                    let runs = ctx.api.list_exploit_runs(Some(c.id), Some(t.id)).await?;
                    let seq = runs.iter().map(|r| r.sequence).max().unwrap_or(-1) + 1;
                    let r = ctx.api.create_exploit_run(CreateExploitRun { exploit_id: e.id, challenge_id: c.id, team_id: t.id, priority, sequence: Some(seq) }).await?;
                    println!("Team {}: run {} seq {}", t.team_id, r.id, r.sequence);
                }
            }
            RunCmd::PrependAll { exploit, challenge, priority } => {
                let c = ctx.find_challenge(&challenge).await?;
                let e = ctx.find_exploit(c.id, &exploit).await?;
                let teams = ctx.api.list_teams().await?;
                for t in &teams {
                    let runs = ctx.api.list_exploit_runs(Some(c.id), Some(t.id)).await?;
                    let seq = runs.iter().map(|r| r.sequence).min().unwrap_or(0) - 1;
                    let r = ctx.api.create_exploit_run(CreateExploitRun { exploit_id: e.id, challenge_id: c.id, team_id: t.id, priority, sequence: Some(seq) }).await?;
                    println!("Team {}: run {} seq {}", t.team_id, r.id, r.sequence);
                }
            }
        },
        Cmd::Round { cmd } => match cmd {
            RoundCmd::New => { let id = ctx.api.create_round().await?; println!("Created round {}", id); }
            RoundCmd::List => {
                let rows: Vec<_> = ctx.api.list_rounds().await?.into_iter().map(|r| RoundRow { id: r.id, status: r.status, started: r.started_at.format("%H:%M:%S").to_string() }).collect();
                println!("{}", Table::new(rows));
            }
            RoundCmd::Current => {
                match ctx.api.get_current_round().await? {
                    Some(r) => println!("Round {} ({})", r.id, r.status),
                    None => println!("No running round"),
                }
            }
            RoundCmd::Run { id } => { ctx.api.run_round(id).await?; println!("Started round {}", id); }
            RoundCmd::Rerun { id } => { ctx.api.rerun_round(id).await?; println!("Rerunning round {}", id); }
            RoundCmd::RerunUnflagged { id } => { ctx.api.rerun_unflagged_round(id).await?; println!("Reran unflagged jobs for round {}", id); }
            RoundCmd::Clean { db } => {
                let pool = sqlx::PgPool::connect(&db).await?;
                sqlx::query!("TRUNCATE flags, exploit_jobs, rounds RESTART IDENTITY CASCADE")
                    .execute(&pool)
                    .await?;
                println!("Cleaned all round data");
            }
        },
        Cmd::Job { cmd } => match cmd {
            JobCmd::List { round } => {
                ctx.teams().await?;
                let rows: Vec<_> = ctx.api.list_jobs(round).await?.into_iter().map(|j| { let (tid, tn) = ctx.team_label(j.team_id); JobRow { id: j.id, run: j.exploit_run_id.map(|r| r.to_string()).unwrap_or("-".into()), team_id: tid, team_name: tn, priority: j.priority, status: j.status } }).collect();
                println!("{}", Table::new(rows));
            }
            JobCmd::Get { id } => {
                let j = ctx.api.get_job(id).await?;
                println!("Job {} ({})", j.id, j.status);
                if let Some(out) = j.stdout { if !out.is_empty() { println!("--- stdout ---\n{}", out); } }
                if let Some(err) = j.stderr { if !err.is_empty() { println!("--- stderr ---\n{}", err); } }
            }
            JobCmd::Run { id } => { let j = ctx.api.enqueue_existing_job(id).await?; println!("Enqueued job {}", j.id); }
            JobCmd::Stop { id } => { ctx.api.stop_job(id).await?; println!("Stopped job {}", id); }
            JobCmd::SetPriority { id, priority } => { ctx.api.reorder_jobs(vec![ReorderJobItem { id, priority }]).await?; println!("Set priority"); }
        },
        Cmd::Flag { cmd } => match cmd {
            FlagCmd::List { round } => {
                ctx.teams().await?;
                let rows: Vec<_> = ctx.api.list_flags(round).await?.into_iter().map(|f| { let (tid, tn) = ctx.team_label(f.team_id); FlagRow { id: f.id, round: f.round_id.to_string(), challenge: f.challenge_id, team_id: tid, team_name: tn, flag: f.flag_value, status: f.status } }).collect();
                println!("{}", Table::new(rows));
            }
            FlagCmd::Submit { round, challenge, team, flag } => {
                let c = ctx.find_challenge(&challenge).await?;
                let t = ctx.find_team(&team).await?;
                let f = ctx.api.submit_flag(SubmitFlagRequest { round_id: round, challenge_id: c.id, team_id: t.id, flag_value: flag }).await?;
                println!("Submitted flag {} for round {}", f.id, f.round_id);
            }
            FlagCmd::Update { force, items } => {
                let reqs: Vec<_> = items.into_iter().map(|(id, status)| UpdateFlagRequest { id, status }).collect();
                let results = ctx.api.update_flags(reqs, force).await?;
                println!("Updated {} flags", results.iter().filter(|&&r| r).count());
            }
        },
        Cmd::Setting { cmd } => match cmd {
            SettingCmd::List => {
                let rows: Vec<_> = ctx.api.list_settings().await?.into_iter().map(|s| SettingRow { key: s.key, value: s.value }).collect();
                println!("{}", Table::new(rows));
            }
            SettingCmd::Set { key, value } => { ctx.api.update_setting(UpdateSetting { key: key.clone(), value: value.clone() }).await?; println!("Set {} = {}", key, value); }
        },
        Cmd::Container { cmd } => match cmd {
            ContainerCmd::List => {
                let rows: Vec<_> = ctx.api.list_containers().await?.into_iter().map(|c| ContainerRow { id: c.id[..12.min(c.id.len())].to_string(), exploit: c.exploit_id, status: c.status, counter: c.counter, execs: format!("{}/{}", c.running_execs, c.max_execs), affinity: if c.affinity_runs.is_empty() { "-".into() } else { c.affinity_runs.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",") } }).collect();
                println!("{}", Table::new(rows));
            }
            ContainerCmd::Runners { id } => {
                ctx.teams().await?;
                let rows: Vec<_> = ctx.api.get_container_runners(&id).await?.into_iter().map(|r| { let (tid, tn) = ctx.team_label(r.team_id); RunnerRow { id: r.id, run: r.exploit_run_id.map(|id| id.to_string()).unwrap_or("-".into()), team_id: tid, team_name: tn, status: r.status } }).collect();
                println!("{}", Table::new(rows));
            }
            ContainerCmd::Delete { id } => { ctx.api.delete_container(&id).await?; println!("Deleted"); }
            ContainerCmd::Restart { id } => { ctx.api.restart_container(&id).await?; println!("Restarted"); }
            ContainerCmd::RestartAll => {
                let r = ctx.api.restart_all_containers().await?;
                println!("Restarted {}/{} containers", r.success, r.total);
                if !r.failures.is_empty() { for f in r.failures { eprintln!("  {}", f); } }
            }
            ContainerCmd::RemoveAll => {
                let r = ctx.api.remove_all_containers().await?;
                println!("Removed {}/{} containers", r.success, r.total);
                if !r.failures.is_empty() { for f in r.failures { eprintln!("  {}", f); } }
            }
        },
        Cmd::Ws { cmd } => match cmd {
            WsCmd::List => {
                let rows: Vec<_> = ctx.api.list_ws_connections().await?.into_iter().map(|w| WsRow { id: w.id, client_ip: w.client_ip, client_name: w.client_name, user: w.user, duration: format!("{}s", w.duration_secs) }).collect();
                println!("{}", Table::new(rows));
            }
        },
        Cmd::Relation { cmd } => match cmd {
            RelationCmd::List { challenge } => {
                let c = ctx.find_challenge(&challenge).await?;
                ctx.teams().await?;
                let rows: Vec<_> = ctx.api.list_relations(c.id).await?.into_iter().map(|r| { let (tid, tn) = ctx.team_label(r.team_id); RelationRow { challenge: r.challenge_id, team_id: tid, team_name: tn, enabled: r.enabled, addr: r.addr.unwrap_or_default(), port: r.port.map(|p| p.to_string()).unwrap_or_default() } }).collect();
                println!("{}", Table::new(rows));
            }
            RelationCmd::Get { challenge, team } => {
                let c = ctx.find_challenge(&challenge).await?;
                let t = ctx.find_team(&team).await?;
                if let Some(r) = ctx.api.get_relation(c.id, t.id).await? {
                    println!("Challenge: {}, Team: {} ({}), Enabled: {}, Addr: {}, Port: {}", r.challenge_id, t.team_id, t.team_name, r.enabled, r.addr.unwrap_or_default(), r.port.map(|p| p.to_string()).unwrap_or_default());
                } else { println!("Not found"); }
            }
            RelationCmd::Update { challenge, team, ip, port } => {
                let c = ctx.find_challenge(&challenge).await?;
                let t = ctx.find_team(&team).await?;
                ctx.api.update_connection_info(c.id, t.id, UpdateConnectionInfo { addr: ip, port, enabled: None }).await?;
                println!("Updated");
            }
            RelationCmd::Enable { challenge, team } => {
                let c = ctx.find_challenge(&challenge).await?;
                let t = ctx.find_team(&team).await?;
                ctx.api.update_connection_info(c.id, t.id, UpdateConnectionInfo { addr: None, port: None, enabled: Some(true) }).await?;
                println!("Enabled");
            }
            RelationCmd::Disable { challenge, team } => {
                let c = ctx.find_challenge(&challenge).await?;
                let t = ctx.find_team(&team).await?;
                ctx.api.update_connection_info(c.id, t.id, UpdateConnectionInfo { addr: None, port: None, enabled: Some(false) }).await?;
                println!("Disabled");
            }
        },
    }
    Ok(())
}
