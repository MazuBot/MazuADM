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

The installer copies `config.toml` from your current directory to `/opt/mazuadm/config.toml` if it exists.

### Database
```bash
DATABASE_URL=postgres://localhost/mazuadm
sqlx database create
sqlx migrate run --source core/migrations
```

Notes:
- Always apply all migrations from `core/migrations` to keep schema current.
 - Connection pool settings can be tuned in `config.toml` under `[db_pool]` (see `config.example.toml`).

#### SQLX offline metadata (_sqlx db)
If `cargo sqlx prepare` fails due to missing migrations or schema drift, recreate the
scratch database and rerun migrations:

```bash
DATABASE_URL=postgres://localhost/mazuadm_sqlx
sqlx database drop -y
sqlx database create
sqlx migrate run --source core/migrations
cargo sqlx prepare --workspace
```

### Build
```bash
cargo build --release
```

### Run API
```bash
DATABASE_URL=postgres://localhost/mazuadm ./target/release/mazuadm-api [config_dir]
# example: ./target/release/mazuadm-api /opt/mazuadm
# logs: <config_dir>/mazuadm-api.log (or ./mazuadm-api.log when omitted)
```

### Tokio console (debug)
Enable the console subscriber only in debug builds:
```bash
RUSTFLAGS="--cfg tokio_unstable" MAZUADM_CONSOLE=1 cargo run -p mazuadm-api [config_dir]
# in another terminal
cargo install --locked tokio-console
tokio-console
```
Notes:
- The console server binds to `127.0.0.1:6669` by default.
- `RUST_LOG` controls stdout log filtering when console logging is enabled.
- `./scripts/install.sh --debug` installs a debug API binary with tokio-console support; set `MAZUADM_CONSOLE=1` when running it.

### Run CLI
```bash
./target/release/mazuadm-cli --help
# or with custom API URL:
./target/release/mazuadm-cli --api http://localhost:3000 --help
```

### Web UI
```bash
cd web && npm install && npm run dev
```

## CLI Usage

See [docs/CLI.md](docs/CLI.md) for detailed CLI documentation.

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
