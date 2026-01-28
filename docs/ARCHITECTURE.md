# MazuADM Rust Architecture

## Crate Structure

```
mazuadm/
├── core/       # Shared library (mazuadm-core)
├── api/        # HTTP API server (mazuadm-api)
└── cli/        # Command-line tool (mazuadm-cli)
```

---

## Core Library (`mazuadm-core`)

### Models (`models.rs`)

Data structures mapping to database tables.

| Struct | Purpose |
|--------|---------|
| `Challenge` | CTF challenge definition |
| `CreateChallenge` | DTO for creating challenges |
| `Team` | Competing team |
| `CreateTeam` | DTO for creating teams |
| `ChallengeTeamRelation` | Per-team connection overrides |
| `Exploit` | Exploit container definition |
| `CreateExploit` | DTO for creating exploits |
| `ExploitRun` | Scheduled exploit-team assignment (Trello card) |
| `CreateExploitRun` | DTO for creating runs |
| `Round` | Execution round |
| `ExploitJob` | Individual job execution |
| `Flag` | Captured flag |
| `ConnectionInfo` | Resolved target addr/port |

Key method:
```rust
impl ChallengeTeamRelation {
    // Resolves connection info with fallbacks:
    // addr: relation.addr -> team.default_ip
    // port: relation.port -> challenge.default_port
    fn connection_info(&self, challenge: &Challenge, team: &Team) -> Option<ConnectionInfo>
}
```

### Database (`db.rs`)

```rust
pub struct Database {
    pub pool: PgPool,
}
```

CRUD operations for all entities. Key methods:

| Method | Description |
|--------|-------------|
| `connect(url)` | Create connection pool |
| `create_challenge/team/exploit/...` | Insert entities |
| `list_challenges/teams/...` | Query entities |
| `ensure_relations(challenge_id)` | Auto-create relations for all teams |
| `create_round()` | Start new round |
| `create_job(...)` | Create job for round |
| `get_pending_jobs(round_id)` | Get jobs to execute |
| `update_job_status(id, status)` | Mark job running |
| `finish_job(id, status, stdout, stderr, duration)` | Complete job |
| `create_flag(...)` | Store captured flag |

### Scheduler (`scheduler.rs`)

```rust
pub struct Scheduler {
    db: Database,
}
```

| Method | Description |
|--------|-------------|
| `calculate_priority(challenge_priority, team_priority, sequence, override)` | Compute job priority |
| `generate_round()` | Create round and generate all jobs from enabled exploit_runs |

Priority formula (when no override):
```
priority = challenge_priority + team_priority * 100 - sequence * 10000
```

### Executor (`executor.rs`)

```rust
pub struct Executor {
    db: Database,
    docker: Docker,  // bollard client
}

pub struct JobResult {
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: i32,
    pub exit_code: i64,
    pub flags: Vec<String>,
}
```

| Method | Description |
|--------|-------------|
| `execute_job(job, exploit, conn)` | Run single container, capture output |
| `extract_flags(output, pattern)` | Extract flags using regex |
| `run_round(round_id)` | Execute all pending jobs in priority order |

Container environment:
- `TARGET_HOST` - Target IP
- `TARGET_PORT` - Target port

---

## API Server (`mazuadm-api`)

### State

```rust
pub struct AppState {
    pub db: Database,
}
```

### Routes (`routes.rs`)

| Endpoint | Method | Handler |
|----------|--------|---------|
| `/api/challenges` | GET | `list_challenges` |
| `/api/challenges` | POST | `create_challenge` |
| `/api/challenges/{id}/enabled/{enabled}` | PUT | `set_challenge_enabled` |
| `/api/teams` | GET | `list_teams` |
| `/api/teams` | POST | `create_team` |
| `/api/exploits` | GET | `list_exploits` |
| `/api/exploits` | POST | `create_exploit` |
| `/api/exploit-runs` | GET | `list_exploit_runs` |
| `/api/exploit-runs` | POST | `create_exploit_run` |
| `/api/rounds` | GET | `list_rounds` |
| `/api/rounds` | POST | `create_round` |
| `/api/rounds/{id}/run` | POST | `run_round` |
| `/api/jobs` | GET | `list_jobs` |
| `/api/flags` | GET | `list_flags` |

Query parameters: `challenge_id`, `team_id`, `round_id`

---

## CLI (`mazuadm-cli`)

### Commands

```
mazuadm-cli [--db DATABASE_URL] <command>

challenge add --name <NAME> [--port <PORT>] [--priority <N>]
challenge list
challenge enable <ID>
challenge disable <ID>

team add --id <ID> --name <NAME> [--ip <IP>] [--priority <N>]
team list

exploit create <NAME|.> [--challenge <NAME>] [--config <PATH>]
exploit list [--challenge <ID>]

run add --exploit <ID> --challenge <ID> --team <ID> [--priority <N>] [--sequence <N>]
run list [--challenge <ID>] [--team <ID>]

round new
round list
round run <ID>
round jobs <ID>

flag list [--round <ID>]
```

---

## Execution Flow

```
1. Setup
   challenge add → team add → exploit create → run add

2. Round Execution
   round new
     └─→ Scheduler.generate_round()
           └─→ Creates ExploitJobs from enabled ExploitRuns
   
   round run <id>
     └─→ Executor.run_round()
           └─→ For each pending job (by priority):
                 └─→ execute_job()
                       ├─→ Create container with TARGET_HOST/PORT
                       ├─→ Wait for completion
                       ├─→ extract_flags() from stdout
                       └─→ Store flags in DB

3. Results
   flag list --round <id>
```
