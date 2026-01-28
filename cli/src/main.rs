use anyhow::Result;
use clap::{Parser, Subcommand};
use mazuadm_core::*;
use tabled::{Table, Tabled};

#[derive(Parser)]
#[command(name = "mazuadm", about = "CTF Attack Manager")]
struct Cli {
    #[arg(long, env = "DATABASE_URL", default_value = "postgres://localhost/mazuadm")]
    db: String,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    Challenge { #[command(subcommand)] cmd: ChallengeCmd },
    Team { #[command(subcommand)] cmd: TeamCmd },
    Exploit { #[command(subcommand)] cmd: ExploitCmd },
    Run { #[command(subcommand)] cmd: RunCmd },
    Round { #[command(subcommand)] cmd: RoundCmd },
    Flag { #[command(subcommand)] cmd: FlagCmd },
}

#[derive(Subcommand)]
enum ChallengeCmd {
    Add { #[arg(long)] name: String, #[arg(long)] port: Option<i32>, #[arg(long)] priority: Option<i32>, #[arg(long)] flag_regex: Option<String> },
    List,
    Enable { id: i32 },
    Disable { id: i32 },
}

#[derive(Subcommand)]
enum TeamCmd {
    Add { #[arg(long)] id: String, #[arg(long)] name: String, #[arg(long)] ip: Option<String>, #[arg(long)] priority: Option<i32> },
    List,
}

#[derive(Subcommand)]
enum ExploitCmd {
    Add { #[arg(long)] name: String, #[arg(long)] challenge: i32, #[arg(long)] image: String, #[arg(long)] entrypoint: Option<String>, #[arg(long)] priority: Option<i32>, #[arg(long)] max_per_container: Option<i32>, #[arg(long)] timeout: Option<i32> },
    List { #[arg(long)] challenge: Option<i32> },
}

#[derive(Subcommand)]
enum RunCmd {
    Add { #[arg(long)] exploit: i32, #[arg(long)] challenge: i32, #[arg(long)] team: i32, #[arg(long)] priority: Option<i32>, #[arg(long)] sequence: Option<i32> },
    List { #[arg(long)] challenge: Option<i32>, #[arg(long)] team: Option<i32> },
}

#[derive(Subcommand)]
enum RoundCmd {
    New,
    List,
    Run { id: i32 },
    Jobs { id: i32 },
    Clean,
}

#[derive(Subcommand)]
enum FlagCmd {
    List { #[arg(long)] round: Option<i32> },
}

#[derive(Tabled)]
struct ChallengeRow { id: i32, name: String, enabled: bool, port: String, priority: i32 }
#[derive(Tabled)]
struct TeamRow { id: i32, team_id: String, name: String, ip: String, priority: i32 }
#[derive(Tabled)]
struct ExploitRow { id: i32, name: String, challenge: i32, image: String, priority: i32 }
#[derive(Tabled)]
struct RunRow { id: i32, exploit: i32, challenge: i32, team: i32, priority: String, seq: i32 }
#[derive(Tabled)]
struct RoundRow { id: i32, status: String, started: String }
#[derive(Tabled)]
struct JobRow { id: i32, run: i32, team: i32, priority: i32, status: String }
#[derive(Tabled)]
struct FlagRow { id: i32, round: i32, challenge: i32, team: i32, flag: String, status: String }

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let db = Database::connect(&cli.db).await?;

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
                let rows: Vec<_> = db.list_teams().await?.into_iter().map(|t| TeamRow { id: t.id, team_id: t.team_id, name: t.team_name, ip: t.default_ip.unwrap_or_default(), priority: t.priority }).collect();
                println!("{}", Table::new(rows));
            }
        },
        Cmd::Exploit { cmd } => match cmd {
            ExploitCmd::Add { name, challenge, image, priority, max_per_container, timeout, entrypoint } => {
                let e = db.create_exploit(CreateExploit { name, challenge_id: challenge, docker_image: image, entrypoint, enabled: Some(true), priority, max_per_container, timeout_secs: timeout, default_counter: None, auto_add: None }).await?;
                println!("Created exploit {}", e.id);
            }
            ExploitCmd::List { challenge } => {
                let rows: Vec<_> = db.list_exploits(challenge).await?.into_iter().map(|e| ExploitRow { id: e.id, name: e.name, challenge: e.challenge_id, image: e.docker_image, priority: e.priority }).collect();
                println!("{}", Table::new(rows));
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
            RoundCmd::Jobs { id } => {
                let rows: Vec<_> = db.list_jobs(id).await?.into_iter().map(|j| JobRow { id: j.id, run: j.exploit_run_id.unwrap_or(-1), team: j.team_id, priority: j.priority, status: j.status }).collect();
                println!("{}", Table::new(rows));
            }
            RoundCmd::Clean => {
                db.clean_rounds().await?;
                println!("Cleaned all round data");
            }
        },
        Cmd::Flag { cmd } => match cmd {
            FlagCmd::List { round } => {
                let rows: Vec<_> = db.list_flags(round).await?.into_iter().map(|f| FlagRow { id: f.id, round: f.round_id, challenge: f.challenge_id, team: f.team_id, flag: f.flag_value, status: f.status }).collect();
                println!("{}", Table::new(rows));
            }
        },
    }
    Ok(())
}
