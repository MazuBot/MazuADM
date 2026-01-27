# MazuADM - CTF Attack Manager

A Trello-like interface for managing CTF A/D exploits with persistent container support.

## Features

- Trello-style board for managing exploit runs per team
- Persistent Docker containers with configurable lifetime counters
- Parallel exploit execution with concurrency limits
- ANSI color support in job output logs
- Container management UI (view status, runners, restart/remove)

## Setup

### Database
```bash
createdb mazuadm
psql mazuadm < core/migrations/001_initial.sql
```

### Build
```bash
cargo build --release
```

### Run API
```bash
DATABASE_URL=postgres://localhost/mazuadm ./target/release/mazuadm-api
```

### Run CLI
```bash
DATABASE_URL=postgres://localhost/mazuadm ./target/release/mazuadm-cli --help
```

### Web UI
```bash
cd web && npm install && npm run dev
```

## CLI Usage

```bash
# Add challenges
mazuadm-cli challenge add --name "pwn1" --port 9001 --priority 100

# Add teams
mazuadm-cli team add --id "team1" --name "Team 1" --ip "10.0.1.1" --priority 50

# Add exploits
mazuadm-cli exploit add --name "pwn1-exp" --challenge 1 --image "myexploit:latest"

# Add exploit runs (cards)
mazuadm-cli run add --exploit 1 --challenge 1 --team 1

# Create and run a round
mazuadm-cli round new
mazuadm-cli round run 1
mazuadm-cli round jobs 1

# View flags
mazuadm-cli flag list --round 1
```

## Exploit Container Interface

Exploits run in persistent Docker containers. Each container has a lifetime counter that decrements per execution.

Environment variables provided:
- `TARGET_HOST` - Target IP/hostname
- `TARGET_PORT` - Target port
- `TARGET_TEAM_ID` - Target team identifier

Container settings per exploit:
- `max_per_container` - Max concurrent runners per container (default: 1)
- `default_counter` - Lifetime counter for new containers (default: 999)

Output flags to stdout. Default regex: `[A-Za-z0-9]{31}=`
