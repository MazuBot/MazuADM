# Database Structure

## Tables

### challenges
Stores CTF challenge definitions.

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | SERIAL | NO | Primary key |
| name | VARCHAR(255) | NO | Challenge name |
| enabled | BOOLEAN | NO | Whether challenge is active (default: true) |
| default_port | INTEGER | YES | Default service port |
| priority | INTEGER | NO | Execution priority (higher = first, default: 0) |
| flag_regex | VARCHAR(512) | YES | Custom regex for flag extraction |
| created_at | TIMESTAMPTZ | NO | Creation timestamp |

### teams
Target teams in the competition.

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | SERIAL | NO | Primary key |
| team_id | VARCHAR(100) | NO | External team identifier (unique) |
| team_name | VARCHAR(255) | NO | Display name |
| default_ip | VARCHAR(255) | YES | Default target IP |
| priority | INTEGER | NO | Execution priority (default: 0) |
| enabled | BOOLEAN | NO | Skip if disabled (default: true) |
| created_at | TIMESTAMPTZ | NO | Creation timestamp |

### challenge_team_relations
Per-team connection overrides for challenges.

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | SERIAL | NO | Primary key |
| challenge_id | INTEGER | NO | FK → challenges (CASCADE) |
| team_id | INTEGER | NO | FK → teams (CASCADE) |
| addr | VARCHAR(255) | YES | Override IP/hostname |
| port | INTEGER | YES | Override port |
| created_at | TIMESTAMPTZ | NO | Creation timestamp |

Unique constraint: (challenge_id, team_id)

### exploits
Docker-based exploit definitions.

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | SERIAL | NO | Primary key |
| name | VARCHAR(255) | NO | Exploit name |
| challenge_id | INTEGER | NO | FK → challenges (CASCADE) |
| enabled | BOOLEAN | NO | Whether exploit is active (default: true) |
| docker_image | VARCHAR(512) | NO | Docker image to run |
| entrypoint | VARCHAR(512) | YES | Custom entrypoint |
| timeout_secs | INTEGER | NO | Container timeout (default: 30) |
| max_per_container | INTEGER | NO | Max affinity teams per container (default: 1) |
| max_containers | INTEGER | NO | Max active containers for exploit (default: 0 = unlimited) |
| default_counter | INTEGER | NO | Container lifetime (default: 999) |
| created_at | TIMESTAMPTZ | NO | Creation timestamp |

### exploit_runs
Cards in the board view - which exploits run against which teams.

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | SERIAL | NO | Primary key |
| exploit_id | INTEGER | NO | FK → exploits (CASCADE) |
| challenge_id | INTEGER | NO | FK → challenges (CASCADE) |
| team_id | INTEGER | NO | FK → teams (CASCADE) |
| priority | INTEGER | YES | Override priority |
| sequence | INTEGER | NO | Order within team column (default: 0) |
| enabled | BOOLEAN | NO | Skip if disabled (default: true) |
| created_at | TIMESTAMPTZ | NO | Creation timestamp |

Unique constraint: (exploit_id, challenge_id, team_id)

### container_runtime
Managed containers are tracked in Docker labels and restored on startup.

### rounds
Execution rounds grouping jobs.

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | SERIAL | NO | Primary key |
| started_at | TIMESTAMPTZ | NO | Round start time |
| finished_at | TIMESTAMPTZ | YES | Round completion time |
| status | VARCHAR(50) | NO | pending/running/finished (default: pending) |

### exploit_jobs
Individual exploit executions within a round.

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | SERIAL | NO | Primary key |
| round_id | INTEGER | NO | FK → rounds (CASCADE) |
| exploit_run_id | INTEGER | YES | FK → exploit_runs (SET NULL) |
| team_id | INTEGER | NO | FK → teams (CASCADE) |
| priority | INTEGER | NO | Computed priority |
| status | VARCHAR(50) | NO | pending/running/success/failed/timeout/ole/error/skipped/flag/stopped |
| container_id | VARCHAR(100) | YES | Docker container ID used |
| stdout | TEXT | YES | Container stdout |
| stderr | TEXT | YES | Container stderr |
| create_reason | VARCHAR(20) | YES | Reason the job was created (e.g., new_round, enqueue_exploit, rerun_job:<id>, rerun_unflag:<id>) |
| duration_ms | INTEGER | YES | Execution time in milliseconds |
| schedule_at | TIMESTAMPTZ | YES | Job scheduled time (set when picked by scheduler) |
| started_at | TIMESTAMPTZ | YES | Job start time |
| finished_at | TIMESTAMPTZ | YES | Job completion time |
| created_at | TIMESTAMPTZ | NO | Creation timestamp |

### flags
Captured flags extracted from job output or submitted manually.

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| id | SERIAL | NO | Primary key |
| job_id | INTEGER | YES | FK → exploit_jobs (SET NULL) |
| round_id | INTEGER | NO | FK → rounds (CASCADE) |
| challenge_id | INTEGER | NO | FK → challenges (CASCADE) |
| team_id | INTEGER | NO | FK → teams (CASCADE) |
| flag_value | VARCHAR(512) | NO | The captured flag |
| status | VARCHAR(50) | NO | captured/submitted/accepted/rejected (default: captured) |
| submitted_at | TIMESTAMPTZ | YES | Submission time |
| created_at | TIMESTAMPTZ | NO | Creation timestamp |

### settings
Runtime configuration key-value store.

| Column | Type | Nullable | Description |
|--------|------|----------|-------------|
| key | VARCHAR(100) | NO | Setting name (PK) |
| value | TEXT | NO | Setting value |

Known settings:
- `concurrent_limit` - Max parallel container executions (default: 10)
- `concurrent_create_limit` - Max concurrent container creations (default: 1)
- `worker_timeout` - Container timeout override in seconds (default: 60)
- `max_flags_per_job` - Max flags to extract per job (default: 50)
- `sequential_per_target` - Run one job per target at a time (default: false)
- `skip_on_flag` - Skip remaining jobs for target after flag (default: false)

## Indexes

| Table | Index | Columns |
|-------|-------|---------|
| exploit_jobs | idx_exploit_jobs_round | round_id |
| exploit_jobs | idx_exploit_jobs_status | status |
| flags | idx_flags_round | round_id |
| flags | idx_flags_status | status |

## Relationships

```
challenges ─┬─< challenge_team_relations >─┬─ teams
            │                              │
            ├─< exploit_runs >─────────────┤
            └─< exploits ────────────────┐
                                         v
                                   exploit_jobs ──> flags
                                         │
                                         v
                                       rounds
```

## Job Status Values

| Status | Description |
|--------|-------------|
| pending | Waiting to execute |
| running | Currently executing |
| success | Completed with exit code 0, no flags |
| failed | Completed with non-zero exit code |
| timeout | Execution timed out |
| ole | Output limit exceeded |
| error | Internal error during execution |
| skipped | Skipped (exploit/team disabled, or skip_on_flag) |
| flag | Completed and captured flag(s) |

## Container Lifecycle

1. Containers are pre-warmed when a round is created
2. Each container has a `counter` (default 999) that decrements when a job lease is acquired
3. Each container caps affinity assignments at `max_per_container`
4. Containers store `mazuadm.affinity` with a CSV list of dynamically assigned `exploit_run_id` values
5. When counter reaches 0 and no execs remain, the container is destroyed
6. Dead containers are removed and recreated on demand
7. `max_containers` caps active containers per exploit (0 = unlimited)
8. Containers and affinities are restored from Docker labels on restart

## Enqueued Jobs

"Run now" actions enqueue jobs into the current running round. These jobs follow the same
scheduler rules (sequential_per_target, skip_on_flag, concurrent_limit).

## Round Lifecycle

1. `round new` - Creates round with status 'pending', generates jobs from enabled exploit_runs
2. `round run` - Sets status to 'running', executes pending jobs
3. Round stays 'running' until next round starts
4. Starting a new round sets previous running rounds to 'finished'
