# Database Structure

## Tables

### challenges
Stores CTF challenge definitions.

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL | Primary key |
| name | VARCHAR(255) | Challenge name |
| enabled | BOOLEAN | Whether challenge is active |
| default_port | INTEGER | Default service port |
| priority | INTEGER | Execution priority (higher = first) |
| flag_regex | VARCHAR(512) | Custom regex for flag extraction |
| created_at | TIMESTAMPTZ | Creation timestamp |

### teams
Target teams in the competition.

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL | Primary key |
| team_id | VARCHAR(100) | External team identifier (unique) |
| team_name | VARCHAR(255) | Display name |
| default_ip | VARCHAR(255) | Default target IP |
| priority | INTEGER | Execution priority |
| enabled | BOOLEAN | Skip if disabled |
| created_at | TIMESTAMPTZ | Creation timestamp |

### challenge_team_relations
Per-team connection overrides for challenges.

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL | Primary key |
| challenge_id | INTEGER | FK → challenges |
| team_id | INTEGER | FK → teams |
| addr | VARCHAR(255) | Override IP/hostname |
| port | INTEGER | Override port |

### exploits
Docker-based exploit definitions.

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL | Primary key |
| name | VARCHAR(255) | Exploit name |
| challenge_id | INTEGER | FK → challenges |
| enabled | BOOLEAN | Whether exploit is active |
| priority | INTEGER | Execution priority |
| docker_image | VARCHAR(512) | Docker image to run |
| entrypoint | VARCHAR(512) | Custom entrypoint |
| timeout_secs | INTEGER | Container timeout (default: 30) |
| max_per_container | INTEGER | Max runners per container |
| default_counter | INTEGER | Container lifetime (default: 999) |

### exploit_containers
Persistent Docker containers for exploits.

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL | Primary key |
| exploit_id | INTEGER | FK → exploits |
| container_id | VARCHAR(100) | Docker container ID |
| counter | INTEGER | Remaining uses before destroy |
| status | VARCHAR(50) | running/dead/destroyed |
| created_at | TIMESTAMPTZ | Creation timestamp |

### exploit_runners
Pins exploit_runs to specific containers.

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL | Primary key |
| exploit_container_id | INTEGER | FK → exploit_containers |
| exploit_run_id | INTEGER | FK → exploit_runs (unique) |
| team_id | INTEGER | FK → teams |
| created_at | TIMESTAMPTZ | Creation timestamp |

### exploit_runs
Cards in the board view - which exploits run against which teams.

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL | Primary key |
| exploit_id | INTEGER | FK → exploits |
| challenge_id | INTEGER | FK → challenges |
| team_id | INTEGER | FK → teams |
| priority | INTEGER | Override priority |
| sequence | INTEGER | Order within team column |
| enabled | BOOLEAN | Skip if disabled |

### rounds
Execution rounds grouping jobs.

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL | Primary key |
| started_at | TIMESTAMPTZ | Round start time |
| finished_at | TIMESTAMPTZ | Round completion time |
| status | VARCHAR(50) | pending/running/finished |

### exploit_jobs
Individual exploit executions within a round.

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL | Primary key |
| round_id | INTEGER | FK → rounds |
| exploit_run_id | INTEGER | FK → exploit_runs |
| team_id | INTEGER | FK → teams |
| priority | INTEGER | Computed priority |
| status | VARCHAR(50) | pending/running/success/failed/timeout/error/skipped |
| stdout | TEXT | Container stdout |
| stderr | TEXT | Container stderr |
| duration_ms | INTEGER | Execution time |
| started_at | TIMESTAMPTZ | Job start |
| finished_at | TIMESTAMPTZ | Job completion |

### flags
Captured flags extracted from job output.

| Column | Type | Description |
|--------|------|-------------|
| id | SERIAL | Primary key |
| job_id | INTEGER | FK → exploit_jobs |
| round_id | INTEGER | FK → rounds |
| challenge_id | INTEGER | FK → challenges |
| team_id | INTEGER | FK → teams |
| flag_value | VARCHAR(512) | The captured flag |
| status | VARCHAR(50) | captured/submitted/accepted/rejected |
| submitted_at | TIMESTAMPTZ | Submission time |

### settings
Runtime configuration key-value store.

| Column | Type | Description |
|--------|------|-------------|
| key | VARCHAR(100) | Setting name (PK) |
| value | TEXT | Setting value |

Known settings:
- `concurrent_limit` - Max parallel container executions (default: 10)
- `worker_timeout` - Container timeout override in seconds (default: 60)

## Relationships

```
challenges ─┬─< exploit_runs >─┬─ teams
            │                  │
            └─< exploits ──────┘
                    │
                    ├─< exploit_containers ─< exploit_runners
                    │                              │
                    v                              v
              exploit_jobs ──> flags         exploit_runs
                    │
                    v
                  rounds
```

## Container Lifecycle

1. When an exploit is created/enabled, containers are pre-warmed
2. Each container has a `counter` (default 999) that decrements on each use
3. Runners (exploit_run + team) are pinned to containers
4. When counter reaches 0, container is destroyed and runners reassigned
5. Dead containers are auto-detected and recreated with runners reassigned
6. Containers stay running between rounds
