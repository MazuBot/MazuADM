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
    executor: Executor,
    tx: broadcast::Sender<WsMessage>,
}

pub struct SchedulerRunner {
    scheduler: Scheduler,
    notify: Arc<Notify>,
    rx: mpsc::UnboundedReceiver<SchedulerCommand>,
}

#[derive(Clone)]
pub struct SchedulerHandle {
    tx: mpsc::UnboundedSender<SchedulerCommand>,
    notify: Arc<Notify>,
}
```

| Method | Description |
|--------|-------------|
| `calculate_priority(challenge_priority, team_priority, sequence, override)` | Compute job priority |
| `generate_round()` | Create round and generate all jobs from enabled exploit_runs |
| `create_round()` | Create round and pre-warm containers |
| `run_round(round_id)` | Execute all pending jobs in priority order |
| `rerun_round(round_id)` | Reset round state and re-run |
| `rerun_unflagged_round(round_id)` | Reset non-flag jobs for running round and execute |

Priority formula (when no override):
```
priority = challenge_priority + team_priority * 100 - sequence * 10000
```

SchedulerRunner is started once at API startup; handlers enqueue `SchedulerCommand` values
via `SchedulerHandle` and notify the runner.

### Executor (`executor.rs`)

```rust
pub struct Executor {
    db: Database,
    container_manager: ContainerManager,
    tx: broadcast::Sender<WsMessage>,
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
| `run_job_immediately(job_id)` | Execute a pending job immediately (internal) |
| `stop_job(job_id, reason)` | Stop a running job (kills exec PID only) |

Container environment:
- `TARGET_HOST` - Target IP
- `TARGET_PORT` - Target port
- `TARGET_TEAM_ID` - Target team ID

---

## API Server (`mazuadm-api`)

### State

```rust
pub struct AppState {
    pub db: Database,
    pub tx: broadcast::Sender<WsMessage>,
    pub executor: Executor,
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
| `/api/exploits/{id}` | PUT | `update_exploit` |
| `/api/exploits/{id}` | DELETE | `delete_exploit` |
| `/api/exploit-runs` | GET | `list_exploit_runs` |
| `/api/exploit-runs` | POST | `create_exploit_run` |
| `/api/exploit-runs/reorder` | POST | `reorder_exploit_runs` |
| `/api/exploit-runs/{id}` | PUT | `update_exploit_run` |
| `/api/exploit-runs/{id}` | DELETE | `delete_exploit_run` |
| `/api/rounds` | GET | `list_rounds` |
| `/api/rounds` | POST | `create_round` |
| `/api/rounds/{id}/run` | POST | `run_round` |
| `/api/rounds/{id}/rerun` | POST | `rerun_round` |
| `/api/rounds/{id}/rerun-unflagged` | POST | `rerun_unflagged_round` |
| `/api/jobs` | GET | `list_jobs` |
| `/api/jobs/{id}` | GET | `get_job` |
| `/api/jobs/reorder` | POST | `reorder_jobs` |
| `/api/jobs/enqueue` | POST | `enqueue_single_job` |
| `/api/jobs/{id}/enqueue` | POST | `enqueue_existing_job` |
| `/api/jobs/{id}/stop` | POST | `stop_job` |
| `/api/flags` | GET | `list_flags` |
| `/api/settings` | GET | `list_settings` |
| `/api/settings` | POST | `update_setting` |
| `/api/containers` | GET | `list_containers` |
| `/api/containers/restart-all` | POST | `restart_all_containers` |
| `/api/containers/remove-all` | POST | `remove_all_containers` |
| `/api/containers/{id}` | DELETE | `delete_container` |
| `/api/containers/{id}/runners` | GET | `get_container_runners` |
| `/api/containers/{id}/restart` | POST | `restart_container` |
| `/api/relations/{challenge_id}` | GET | `list_relations` |
| `/api/relations/{challenge_id}/{team_id}` | GET/PUT | `get_relation` / `update_relation` |

Query parameters: `challenge_id`, `team_id`, `round_id`

---

## Container Lifecycle (Current)

1. Containers are pre-warmed when a round is created.
2. Each container has a `counter` that decrements when a job lease is acquired.
3. Each container enforces `max_per_container` concurrent execs.
4. Containers carry `mazuadm.affinity` with a CSV list of dynamically assigned `exploit_run_id` values.
5. When `counter` reaches 0 and no execs remain, the container is destroyed.
6. Dead containers are removed and recreated on demand.
7. `max_containers` caps active containers per exploit (0 = unlimited).
8. Containers and affinities are restored from Docker labels on restart.

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
round rerun-unflagged <ID>
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
     └─→ Scheduler.run_round()
           ├─→ Stops all running jobs immediately (any round)
           └─→ For each pending job (by priority):
                 └─→ execute_job()
                       ├─→ Lease container (per-exploit pool)
                       ├─→ Wait for completion
                       ├─→ extract_flags() from stdout
                       └─→ Store flags in DB

3. Results
   flag list --round <id>
```
