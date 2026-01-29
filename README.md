# MazuADM - CTF Attack Manager

A Trello-like interface for managing CTF A/D exploits with persistent container support.

## Features

- Trello-style board for managing exploit runs per team
- Persistent Docker containers with configurable lifetime counters
- Parallel exploit execution
- ANSI color support in job output logs
- Container management UI (view status, runners, restart/remove)

## Setup

### Install binaries
```bash
./scripts/install.sh
```

The installer copies `config.toml` from your current directory to `/etc/mazuadm/config.toml` if it exists.

### Database
```bash
createdb mazuadm
DATABASE_URL=postgres://localhost/mazuadm sqlx migrate run --source core/migrations
```

Notes:
- Running only `001_initial.sql` leaves newer columns/tables missing (ex: settings, enabled flags).
- Always apply all migrations from `core/migrations` to keep schema current.

#### SQLX offline metadata (_sqlx db)
If `cargo sqlx prepare` fails due to missing migrations or schema drift, recreate the
scratch database and rerun migrations:

```bash
DATABASE_URL=postgres://localhost/mazuadm_sqlx sqlx database drop -y
DATABASE_URL=postgres://localhost/mazuadm_sqlx sqlx database create
DATABASE_URL=postgres://localhost/mazuadm_sqlx sqlx migrate run --source core/migrations
DATABASE_URL=postgres://localhost/mazuadm_sqlx cargo sqlx prepare --workspace
```

### Build
```bash
cargo build --release
```

### Run API
```bash
DATABASE_URL=postgres://localhost/mazuadm ./target/release/mazuadm-api [config_dir]
# example: ./target/release/mazuadm-api /etc/mazuadm
# logs: <config_dir>/mazuadm-api.log (or ./mazuadm-api.log when omitted)
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

See [CLI.md](CLI.md) for detailed CLI documentation.

## Exploit Container Interface

Exploits run in persistent Docker containers. Each container has a lifetime counter that decrements when a job lease is acquired.

Environment variables provided:
- `TARGET_HOST` - Target IP/hostname
- `TARGET_PORT` - Target port
- `TARGET_TEAM_ID` - Target team identifier

Container settings per exploit:
- `max_per_container` - Max affinity teams per container (default: 1)
- `default_counter` - Lifetime counter for new containers (default: 999)

Output flags to stdout. Default regex: `[A-Za-z0-9]{31}=`
