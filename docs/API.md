# MazuADM API Documentation

Base URL: `http://localhost:3000`

## WebSocket

### `GET /ws`
Real-time event stream for UI updates.

Events: `challenge_created`, `challenge_updated`, `challenge_deleted`, `team_created`, `team_updated`, `team_deleted`, `exploit_created`, `exploit_updated`, `exploit_deleted`, `exploit_run_created`, `exploit_run_updated`, `exploit_run_deleted`, `exploit_runs_reordered`, `round_created`, `round_updated`, `job_created`, `job_updated`, `flag_created`, `setting_updated`, `relation_updated`

---

## Challenges

### `GET /api/challenges`
List all challenges.

**Response:** `Challenge[]`

### `POST /api/challenges`
Create a challenge.

**Body:**
```json
{
  "name": "string",
  "enabled": "boolean?",
  "default_port": "int?",
  "priority": "int? (0-99)",
  "flag_regex": "string?"
}
```

**Response:** `Challenge`

### `PUT /api/challenges/{id}`
Update a challenge.

**Body:** Same as POST

**Response:** `Challenge`

### `DELETE /api/challenges/{id}`
Delete a challenge.

### `PUT /api/challenges/{id}/enabled/{enabled}`
Set challenge enabled status.

---

## Teams

### `GET /api/teams`
List all teams.

**Response:** `Team[]`

### `POST /api/teams`
Create a team.

**Body:**
```json
{
  "team_id": "string",
  "team_name": "string",
  "default_ip": "string?",
  "priority": "int? (0-99)",
  "enabled": "boolean?"
}
```

**Response:** `Team`

### `PUT /api/teams/{id}`
Update a team.

**Body:** Same as POST

**Response:** `Team`

### `DELETE /api/teams/{id}`
Delete a team.

---

## Exploits

### `GET /api/exploits`
List exploits.

**Query:** `challenge_id?: int`

**Response:** `Exploit[]`

### `POST /api/exploits`
Create an exploit.

**Body:**
```json
{
  "name": "string",
  "challenge_id": "int",
  "docker_image": "string",
  "entrypoint": "string?",
  "enabled": "boolean?",
  "priority": "int?",
  "max_per_container": "int?",
  "max_containers": "int? (0 = unlimited)",
  "timeout_secs": "int?",
  "default_counter": "int?",
  "auto_add": "string? (start|end)",
  "insert_into_rounds": "boolean?"
}
```

**Response:** `Exploit`

### `PUT /api/exploits/{id}`
Update an exploit.

**Body:**
```json
{
  "name": "string",
  "docker_image": "string",
  "entrypoint": "string?",
  "enabled": "boolean?",
  "priority": "int?",
  "max_per_container": "int?",
  "max_containers": "int? (0 = unlimited)",
  "timeout_secs": "int?",
  "default_counter": "int?"
}
```

**Response:** `Exploit`

### `DELETE /api/exploits/{id}`
Delete an exploit.

---

## Exploit Runs

### `GET /api/exploit-runs`
List exploit runs.

**Query:** `challenge_id?: int`, `team_id?: int`

**Response:** `ExploitRun[]`

### `POST /api/exploit-runs`
Create an exploit run.

**Body:**
```json
{
  "exploit_id": "int",
  "challenge_id": "int",
  "team_id": "int",
  "priority": "int?",
  "sequence": "int?"
}
```

**Response:** `ExploitRun`

### `PUT /api/exploit-runs/{id}`
Update an exploit run.

**Body:**
```json
{
  "priority": "int?",
  "sequence": "int?",
  "enabled": "boolean?"
}
```

**Response:** `ExploitRun`

### `DELETE /api/exploit-runs/{id}`
Delete an exploit run.

### `POST /api/exploit-runs/reorder`
Reorder exploit runs.

**Body:**
```json
[{ "id": "int", "sequence": "int" }]
```

---

## Rounds

### `GET /api/rounds`
List all rounds.

**Response:** `Round[]`

### `POST /api/rounds`
Create a new round (generates jobs from exploit runs).

**Response:** `int` (round_id)

### `POST /api/rounds/{id}/run`
Start executing a round.

### `POST /api/rounds/{id}/rerun`
Rerun a round (resets this round and all subsequent rounds, then executes).

### `POST /api/rounds/{id}/rerun-unflagged`
Clone scheduled, non-flag, non-skipped, non-pending jobs for the **running** round (skips challenge/team pairs with a flag in the round), then execute.

---

## Jobs

### `GET /api/jobs`
List jobs.

**Query:** `round_id?: int`

**Response:** `ExploitJob[]` (stdout/stderr omitted)

### `GET /api/jobs/{id}`
Get job detail, including stdout/stderr logs.

**Response:** `ExploitJob`

### `POST /api/jobs/reorder`
Reorder jobs.

**Body:**
```json
[{ "id": "int", "priority": "int" }]
```

### `POST /api/jobs/enqueue`
Enqueue a job into the current running round (fails if no round is running).

**Body:**
```json
{
  "exploit_run_id": "int",
  "team_id": "int"
}
```

**Response:** `ExploitJob`

### `POST /api/jobs/{id}/enqueue`
Enqueue an existing job into the current running round (fails if no round is running).
If the job is pending in the running round, its priority is bumped; otherwise a new
job is created from the same exploit_run/team.

**Response:** `ExploitJob`

### `POST /api/jobs/{id}/stop`
Stop a running job.

**Response:** `ExploitJob`

Stops the current exec process for that job (containers are not destroyed).

---

## Flags

### `GET /api/flags`
List flags.

**Query:** `round_id?: int`

**Response:** `Flag[]`

---

## Settings

### `GET /api/settings`
List all settings.

**Response:** `Setting[]`

### `POST /api/settings`
Update a setting.

**Body:**
```json
{
  "key": "string",
  "value": "string"
}
```

---

## Containers

### `GET /api/containers`
List containers.

**Query:** `challenge_id?: int` (filters by exploit_id)

**Response:** `ContainerInfo[]`

### `DELETE /api/containers/{id}`
Destroy a container.
`id` is the Docker container ID.

### `POST /api/containers/restart-all`
Restart all managed containers.

### `POST /api/containers/remove-all`
Remove all managed containers.

### `GET /api/containers/{id}/runners`
Get running jobs for a container.
`id` is the Docker container ID.

**Response:** `ExploitJob[]`

### `POST /api/containers/{id}/restart`
Restart a container (Docker restart; preserves container ID and labels). Repeated requests while a restart is in-flight are ignored.
`id` is the Docker container ID.

---

## Relations

### `GET /api/relations/{challenge_id}`
List challenge-team relations.

**Response:** `ChallengeTeamRelation[]`

### `GET /api/relations/{challenge_id}/{team_id}`
Get a specific relation.

**Response:** `ChallengeTeamRelation?`

### `PUT /api/relations/{challenge_id}/{team_id}`
Update a relation.

**Body:**
```json
{
  "addr": "string?",
  "port": "int?"
}
```

**Response:** `ChallengeTeamRelation`

---

## Models

### Challenge
```json
{
  "id": "int",
  "name": "string",
  "enabled": "boolean",
  "default_port": "int?",
  "priority": "int",
  "flag_regex": "string?",
  "created_at": "datetime"
}
```

### Team
```json
{
  "id": "int",
  "team_id": "string",
  "team_name": "string",
  "default_ip": "string?",
  "priority": "int",
  "enabled": "boolean",
  "created_at": "datetime"
}
```

### Exploit
```json
{
  "id": "int",
  "name": "string",
  "challenge_id": "int",
  "enabled": "boolean",
  "priority": "int",
  "max_per_container": "int",
  "max_containers": "int",
  "docker_image": "string",
  "entrypoint": "string?",
  "timeout_secs": "int",
  "default_counter": "int",
  "created_at": "datetime"
}
```

### ExploitRun
```json
{
  "id": "int",
  "exploit_id": "int",
  "challenge_id": "int",
  "team_id": "int",
  "priority": "int?",
  "sequence": "int",
  "enabled": "boolean",
  "created_at": "datetime"
}
```

### Round
```json
{
  "id": "int",
  "started_at": "datetime",
  "finished_at": "datetime?",
  "status": "string (pending|running|finished|skipped)"
}
```

### ExploitJob
```json
{
  "id": "int",
  "round_id": "int?",
  "exploit_run_id": "int?",
  "team_id": "int",
  "priority": "int",
  "status": "string (pending|running|success|failed|timeout|ole|error|skipped|flag|stopped)",
  "container_id": "string?",
  "stdout": "string?",
  "stderr": "string?",
  "create_reason": "string?",
  "duration_ms": "int?",
  "schedule_at": "datetime?",
  "started_at": "datetime?",
  "finished_at": "datetime?",
  "created_at": "datetime"
}
```

### Flag
```json
{
  "id": "int",
  "job_id": "int?",
  "round_id": "int?",
  "challenge_id": "int",
  "team_id": "int",
  "flag_value": "string",
  "status": "string",
  "submitted_at": "datetime?",
  "created_at": "datetime"
}
```

### ContainerInfo
```json
{
  "id": "string",
  "exploit_id": "int",
  "counter": "int",
  "status": "string",
  "running_execs": "int",
  "max_execs": "int",
  "affinity_runs": "int[]",
  "created_at": "datetime"
}
```

### ChallengeTeamRelation
```json
{
  "id": "int",
  "challenge_id": "int",
  "team_id": "int",
  "addr": "string?",
  "port": "int?",
  "created_at": "datetime"
}
```

### Setting
```json
{
  "key": "string",
  "value": "string"
}
```
