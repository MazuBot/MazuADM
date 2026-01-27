# MazuADM Database Structure

## Entity Relationship

```
┌─────────────┐       ┌─────────────┐
│  Challenge  │       │    Team     │
└──────┬──────┘       └──────┬──────┘
       │                     │
       │  ┌──────────────────┤
       │  │                  │
       ▼  ▼                  │
┌─────────────────────┐      │
│ ChallengeTeamRelation│      │
│ (addr/port override) │      │
└─────────────────────┘      │
       │                     │
       │  ┌──────────────────┘
       ▼  ▼
┌─────────────┐
│   Exploit   │──────────────┐
└──────┬──────┘              │
       │                     │
       ▼                     ▼
┌─────────────┐       ┌─────────────┐
│ ExploitRun  │◄──────│   Round     │
│  (card)     │       └──────┬──────┘
└──────┬──────┘              │
       │                     │
       └────────┬────────────┘
                ▼
         ┌─────────────┐
         │ ExploitJob  │
         │ (execution) │
         └──────┬──────┘
                │
                ▼
         ┌─────────────┐
         │    Flag     │
         └─────────────┘
```

## Tables

### challenges
CTF challenge/service definitions.
- `priority` - Higher = runs first

### teams
Competing teams.
- `team_id` - External identifier
- `default_ip` - Fallback target IP
- `priority` - Higher = targeted first

### challenge_team_relations
Per-team connection overrides for each challenge (Trello columns).
- `addr` - Override IP (falls back to team.default_ip)
- `port` - Override port (falls back to challenge.default_port)

### exploits
Exploit container definitions.
- `docker_image` - Container image to run
- `max_per_container` - Batch size per execution
- `timeout_secs` - Execution timeout
- `flag_regex` - Custom flag pattern (default: `[A-Za-z0-9]{31}=`)

### exploit_runs
Scheduled exploit-team assignments (Trello cards).
- Links: Exploit → Challenge → Team
- `sequence` - Execution order within same priority
- `priority` - Override calculated priority

### rounds
Execution rounds (one per tick).
- `status` - pending/running/finished

### exploit_jobs
Individual job executions per round.
- Created from exploit_runs when round starts
- `priority` = `challenge.priority * 10000 + team.priority * 100 + sequence`
- `status` - pending/running/success/failed/error
- Stores stdout/stderr/duration after execution

### flags
Captured flags extracted from job output.
- `status` - captured/submitted/accepted/rejected
