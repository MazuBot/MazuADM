# MazuADM Architecture

MazuADM is a CTF Attack/Defense exploit management system with a Trello-like interface for orchestrating exploit runs against multiple teams.

## System Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                              Web Browser                                 │
│                         (SvelteKit Frontend)                            │
└─────────────────────────────────────────────────────────────────────────┘
                    │ HTTP/REST              │ WebSocket
                    ▼                        ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                           mazuadm-api                                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌─────────────┐ │
│  │   Handlers   │  │  Scheduler   │  │   Executor   │  │  WebSocket  │ │
│  │   (Axum)     │  │   Runner     │  │              │  │   Broker    │ │
│  └──────────────┘  └──────────────┘  └──────────────┘  └─────────────┘ │
│         │                 │                 │                 │         │
│         └─────────────────┴─────────────────┴─────────────────┘         │
│                                   │                                      │
│                    ┌──────────────┴──────────────┐                       │
│                    │      Container Manager      │                       │
│                    └──────────────┬──────────────┘                       │
└────────────────────────────────────┼────────────────────────────────────┘
                    │                │                │
                    ▼                ▼                ▼
            ┌───────────┐    ┌───────────┐    ┌───────────────┐
            │ PostgreSQL│    │  Docker   │    │ mazuadm-cli   │
            │           │    │  Engine   │    │               │
            └───────────┘    └───────────┘    └───────────────┘
```

## Crate Structure

```
mazuadm/
├── core/           # Shared library (mazuadm-core)
│   ├── models.rs       # Data structures
│   ├── db.rs           # Database operations
│   ├── scheduler.rs    # Job scheduling & round management
│   ├── executor.rs     # Job execution
│   ├── container_manager.rs  # Docker container lifecycle
│   ├── config.rs       # Configuration loading
│   └── settings.rs     # Runtime settings
├── api/            # HTTP API server (mazuadm-api)
│   ├── main.rs         # Server startup
│   ├── routes.rs       # Route definitions
│   ├── handlers.rs     # Request handlers
│   └── events.rs       # WebSocket events
├── cli/            # Command-line tool (mazuadm-cli)
│   ├── main.rs         # CLI commands
│   ├── api.rs          # API client
│   └── models.rs       # CLI-specific models
└── web/            # Frontend (SvelteKit)
    └── src/
        ├── lib/
        │   ├── data/       # API clients & stores
        │   ├── features/   # Page components
        │   └── websocket.js
        └── routes/         # Page routes
```

---

## Core Components

### Data Models (`core/models.rs`)

| Model | Description |
|-------|-------------|
| `Challenge` | CTF challenge with default port and flag regex |
| `Team` | Competing team with default IP |
| `ChallengeTeamRelation` | Per-team connection overrides (addr/port) |
| `Exploit` | Docker image configuration for an exploit |
| `ExploitRun` | Scheduled exploit-team assignment (Trello card) |
| `Round` | Execution round grouping jobs |
| `ExploitJob` | Individual job execution with status/output |
| `ExploitContainer` | Persistent Docker container record |
| `ExploitRunner` | Affinity binding (exploit_run → container) |
| `Flag` | Captured flag with submission status |
| `Setting` | Runtime configuration key-value |

### Database (`core/db.rs`)

PostgreSQL database with connection pooling via SQLx.

Key operations:
- CRUD for all entities
- `ensure_relations(challenge_id)` - Auto-create relations for all teams
- `create_round_jobs(round_id)` - Generate jobs from enabled exploit_runs
- `clone_unflagged_jobs_for_round(round_id)` - Clone jobs that didn't capture flags

### Scheduler (`core/scheduler.rs`)

Manages round lifecycle and job execution ordering.

```rust
pub struct Scheduler {
    db: Database,
    tx: broadcast::Sender<WsMessage>,
}

pub struct SchedulerRunner {
    scheduler: Scheduler,
    rx: mpsc::UnboundedReceiver<SchedulerCommand>,
}

pub struct SchedulerHandle {
    tx: mpsc::UnboundedSender<SchedulerCommand>,
}
```

Commands:
- `CreateRound` - Create new round and spawn background job creation
- `RunRound(id)` - Execute pending jobs for a round
- `RerunRound(id)` - Reset and re-execute a round
- `RerunUnflagged(id)` - Clone and run jobs that didn't get flags
- `RunJobImmediately(id)` - Execute single job immediately
- `StopJob { job_id, reason }` - Stop a running job

Priority formula:
```
priority = challenge_priority + team_priority * 100 - sequence * 10000
```

### Executor (`core/executor.rs`)

Executes jobs in Docker containers.

```rust
pub struct Executor {
    db: Database,
    container_manager: ContainerManager,
    container_registry: ContainerRegistryHandle,
    tx: broadcast::Sender<WsMessage>,
}
```

Job execution flow:
1. Acquire container lease (with affinity)
2. Create Docker exec with environment variables
3. Stream stdout/stderr with timeout
4. Extract flags using regex
5. Store results and release container

Environment variables provided to containers:
- `TARGET_HOST` - Target IP/hostname
- `TARGET_PORT` - Target port
- `TARGET_TEAM_ID` - Target team identifier

### Container Manager (`core/container_manager.rs`)

Manages persistent Docker containers with affinity-based routing.

```rust
pub struct ContainerManager {
    db: Database,
    docker: Docker,
    tx: broadcast::Sender<WsMessage>,
}
```

Container lifecycle:
1. Pre-warm containers when round starts
2. Assign affinity (exploit_run → container) on first job
3. Decrement counter on each job lease
4. Destroy when counter reaches 0 and no running execs
5. Restore state from Docker labels on API restart

Key settings per exploit:
- `max_per_container` - Max affinity assignments per container
- `max_containers` - Max active containers (0 = unlimited)
- `default_counter` - Initial lifetime counter

---

## API Server (`mazuadm-api`)

### Application State

```rust
pub struct AppState {
    pub db: Database,
    pub tx: broadcast::Sender<WsMessage>,
    pub scheduler: SchedulerHandle,
    pub ws_connections: Arc<DashMap<Uuid, WsConnection>>,
}
```

### REST Endpoints

| Endpoint | Methods | Description |
|----------|---------|-------------|
| `/api/challenges` | GET, POST | List/create challenges |
| `/api/challenges/{id}` | PUT, DELETE | Update/delete challenge |
| `/api/teams` | GET, POST | List/create teams |
| `/api/teams/{id}` | PUT, DELETE | Update/delete team |
| `/api/exploits` | GET, POST | List/create exploits |
| `/api/exploits/{id}` | PUT, DELETE | Update/delete exploit |
| `/api/exploit-runs` | GET, POST | List/create exploit runs |
| `/api/exploit-runs/reorder` | POST | Reorder runs (sequence) |
| `/api/rounds` | GET, POST | List/create rounds |
| `/api/rounds/{id}/run` | POST | Execute round |
| `/api/rounds/{id}/rerun` | POST | Re-execute round |
| `/api/rounds/{id}/rerun-unflagged` | POST | Rerun unflagged jobs |
| `/api/jobs` | GET | List jobs (with filters) |
| `/api/jobs/{id}` | GET | Get job details |
| `/api/jobs/{id}/stop` | POST | Stop running job |
| `/api/jobs/enqueue` | POST | Create and run ad-hoc job |
| `/api/flags` | GET, POST, PATCH | List/submit/update flags |
| `/api/settings` | GET, POST | List/update settings |
| `/api/containers` | GET | List containers |
| `/api/containers/{id}` | DELETE | Remove container |
| `/api/containers/{id}/restart` | POST | Restart container |
| `/api/relations/{challenge_id}` | GET | List team relations |
| `/api/relations/{challenge_id}/{team_id}` | GET, PUT | Get/update relation |
| `/ws` | WebSocket | Real-time events |

### WebSocket Events

Connection: `GET /ws?user=<name>&client=<client>&events=<prefixes>`

Event categories:
- `challenge_*` - Challenge CRUD
- `team_*` - Team CRUD
- `exploit_*` - Exploit CRUD
- `exploit_run_*` - Exploit run CRUD
- `round_*` - Round lifecycle
- `job_*` - Job status changes
- `flag_*` - Flag captures
- `container_*` - Container lifecycle
- `setting_updated` - Settings changes

---

## Frontend (`web/`)

SvelteKit application with reactive stores.

### Structure

```
web/src/
├── lib/
│   ├── data/
│   │   ├── api/          # REST API clients
│   │   └── stores/       # Svelte stores
│   │       ├── app.js        # WebSocket message handler
│   │       ├── entities.js   # challenges, teams, exploits, runs
│   │       ├── selections.js # UI selections
│   │       └── rounds.js     # Round-specific state
│   ├── features/
│   │   ├── board/        # Trello-like board
│   │   ├── rounds/       # Round management
│   │   ├── flags/        # Flag viewer
│   │   ├── containers/   # Container management
│   │   ├── challenges/   # Challenge CRUD
│   │   ├── teams/        # Team CRUD
│   │   └── settings/     # Settings page
│   └── websocket.js      # WebSocket connection
└── routes/               # SvelteKit routes
```

### Data Flow

1. Initial load fetches data via REST API
2. WebSocket connects for real-time updates
3. Store updates trigger reactive UI changes
4. User actions call REST API → broadcast WebSocket events

---

## CLI (`mazuadm-cli`)

Command-line interface for automation and scripting.

```
mazuadm-cli [--api URL] <command>

Commands:
  challenge add|list|enable|disable
  team add|list|enable|disable
  exploit create|list|enable|disable
  run add|list
  round new|list|run|rerun-unflagged|jobs
  flag list|submit
  container list|restart|remove
  setting list|set
```

The CLI communicates with the API server via REST.

---

## Execution Flow

### Round Lifecycle

```
1. Create Round
   POST /api/rounds
   └─→ Scheduler.create_round()
         ├─→ Insert round record (status: pending)
         ├─→ Spawn background job creation task
         └─→ Broadcast round_created

2. Background Job Creation
   └─→ create_round_jobs()
         ├─→ For each enabled exploit_run:
         │     └─→ Create ExploitJob with calculated priority
         ├─→ Pre-warm containers for enabled exploits
         └─→ Broadcast round_jobs_ready

3. Run Round
   POST /api/rounds/{id}/run
   └─→ Scheduler.run_round()
         ├─→ Update round status to 'running'
         └─→ For each pending job (by priority):
               └─→ Spawn job execution task

4. Job Execution
   └─→ Executor.execute_job()
         ├─→ Acquire container lease (affinity-aware)
         ├─→ Mark job as 'running'
         ├─→ Create Docker exec with TARGET_* env vars
         ├─→ Stream output with timeout
         ├─→ Extract flags from stdout
         ├─→ Store flags in database
         ├─→ Mark job as 'flag'/'done'/'error'/'timeout'
         └─→ Release container lease

5. Round Completion
   └─→ When all jobs finish:
         ├─→ Update round status to 'finished'
         └─→ Broadcast round_updated
```

### Container Affinity

```
Job arrives for exploit_run_id=42, team_id=5
    │
    ▼
Check existing affinity (exploit_run_id → container)
    │
    ├─→ Found: Use that container
    │
    └─→ Not found:
          │
          ▼
        Find container with capacity (affinity_count < max_per_container)
          │
          ├─→ Found: Assign affinity, use container
          │
          └─→ Not found:
                │
                ▼
              Create new container (if under max_containers)
                │
                └─→ Assign affinity, use container
```

---

## Configuration

### config.toml

```toml
[server]
host = "0.0.0.0"
port = 3000

[db_pool]
max_connections = 10
min_connections = 1
acquire_timeout_secs = 30
idle_timeout_secs = 600
max_lifetime_secs = 1800

[container]
spawn_limit = 10
```

### Runtime Settings (database)

| Key | Default | Description |
|-----|---------|-------------|
| `worker_timeout` | 60 | Job execution timeout (seconds) |
| `max_flags` | 1 | Max flags to extract per job |
| `skip_on_flag` | true | Skip remaining jobs for team after flag |
| `sequential_per_target` | false | Run jobs sequentially per target |
| `stagger_delay_ms` | 0 | Delay between job starts |

---

## Database Schema

```
challenges ─────────────────┐
    │                       │
    ▼                       │
challenge_team_relations    │
    │                       │
    ▼                       │
teams ◄─────────────────────┤
    │                       │
    ▼                       │
exploit_runs ◄──────────────┤
    │         │             │
    │         ▼             │
    │     exploits ◄────────┘
    │         │
    │         ▼
    │     exploit_containers
    │         │
    │         ▼
    │     exploit_runners
    │
    ▼
exploit_jobs ──────────────► rounds
    │
    ▼
flags
```

Key relationships:
- `exploit_runs` links `exploit` to `team` for a `challenge`
- `exploit_jobs` are created per `round` from `exploit_runs`
- `exploit_runners` bind `exploit_run` to `exploit_container` (affinity)
- `flags` are extracted from `exploit_jobs`
