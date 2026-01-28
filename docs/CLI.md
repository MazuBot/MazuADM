# MazuADM CLI Usage

## Overview

The MazuADM CLI provides command-line access to all features of the CTF Attack/Defense Manager.

## Install

```bash
./scripts/install.sh
```

This installs `mazuadm-cli` and `mazuadm-api` to `/usr/local/bin`, copies the exploit template to `/etc/mazuadm/exp-template`, and copies `config.toml` from your current directory to `/etc/mazuadm/config.toml` if it exists.

```bash
mazuadm-cli [OPTIONS] <COMMAND>
```

## Global Options

- `--config <PATH>` - Path to TOML config (overrides `MAZUADM_CONFIG` and default search)
- `--db <URL>` - Database connection URL (default: `postgres://localhost/mazuadm`, env: `DATABASE_URL`)

## Commands

### Challenge Management

```bash
# Add a new challenge
mazuadm-cli challenge add --name "pwn1" --port 9001 --priority 100 --flag-regex "[A-Z0-9]{32}"

# List all challenges
mazuadm-cli challenge list

# Update a challenge
mazuadm-cli challenge update 1 --name "pwn1-renamed" --port 9002

# Delete a challenge
mazuadm-cli challenge delete 1

# Enable/disable a challenge
mazuadm-cli challenge enable 1
mazuadm-cli challenge disable 1
```

### Team Management

```bash
# Add a new team
mazuadm-cli team add --id "team1" --name "Team 1" --ip "10.0.1.1" --priority 50

# List all teams
mazuadm-cli team list

# Update a team
mazuadm-cli team update 1 --name "Team One" --ip "10.0.1.2"

# Delete a team
mazuadm-cli team delete 1

# Enable/disable a team
mazuadm-cli team enable 1
mazuadm-cli team disable 1
```

### Exploit Management

```bash
# Create a new exploit from template config
mazuadm-cli exploit create pwn1-exp --challenge pwn1 --config config.toml
mazuadm-cli exploit create . --challenge pwn1

# List exploits
mazuadm-cli exploit list
mazuadm-cli exploit list --challenge pwn1

# Update an exploit
mazuadm-cli exploit update pwn1-exp --challenge pwn1 --timeout 60
mazuadm-cli exploit update pwn1-exp --challenge pwn1 --config config.toml

# Delete an exploit
mazuadm-cli exploit delete pwn1-exp --challenge pwn1

# Enable/disable an exploit
mazuadm-cli exploit enable pwn1-exp --challenge pwn1
mazuadm-cli exploit disable pwn1-exp --challenge pwn1

# Run exploit immediately against a team (ad-hoc execution)
mazuadm-cli exploit run pwn1-exp --challenge pwn1 --team 1
```

### Exploit Run Management

Exploit runs define which exploit runs against which team for a challenge.

```bash
# Add a new exploit run
mazuadm-cli run add --exploit 1 --challenge 1 --team 1 --priority 100 --sequence 0

# List exploit runs
mazuadm-cli run list
mazuadm-cli run list --challenge 1
mazuadm-cli run list --team 1

# Update an exploit run
mazuadm-cli run update 1 --priority 200 --sequence 1

# Delete an exploit run
mazuadm-cli run delete 1
```

### Round Management

```bash
# Create a new round (generates jobs from exploit runs)
mazuadm-cli round new

# List all rounds
mazuadm-cli round list

# Run a round (execute all pending jobs)
mazuadm-cli round run 1

# Clean all round data (truncates rounds, jobs, flags)
mazuadm-cli round clean
```

### Job Management

```bash
# List jobs for a round
mazuadm-cli job list --round 1

# Run a specific job immediately
mazuadm-cli job run 42

# Set job priority
mazuadm-cli job set-priority 42 100
```

### Flag Management

```bash
# List all flags
mazuadm-cli flag list

# List flags for a specific round
mazuadm-cli flag list --round 1
```

### Settings Management

```bash
# List all settings
mazuadm-cli setting list

# Set a setting value
mazuadm-cli setting set concurrent_limit 10
mazuadm-cli setting set worker_timeout 60
```

### Container Management

```bash
# List all containers
mazuadm-cli container list

# Show runners for a container
mazuadm-cli container runners 1

# Delete a container
mazuadm-cli container delete 1

# Restart a container (not implemented - delete and let it recreate)
mazuadm-cli container restart 1
```

### Relation Management

Relations define per-team connection overrides for challenges.

```bash
# List relations for a challenge
mazuadm-cli relation list 1

# Get a specific relation
mazuadm-cli relation get 1 1

# Update a relation (set IP/port override)
mazuadm-cli relation update 1 1 --ip "10.0.1.100" --port 9999
```

## Examples

### Quick Start

```bash
# 1. Add a challenge
mazuadm-cli challenge add --name "pwn1" --port 9001

# 2. Add teams
mazuadm-cli team add --id "team1" --name "Team 1" --ip "10.0.1.1"
mazuadm-cli team add --id "team2" --name "Team 2" --ip "10.0.2.1"

# 3. Add an exploit
mazuadm-cli exploit create pwn1-exp --challenge pwn1 --config config.toml

# 4. Add exploit runs (cards)
mazuadm-cli run add --exploit 1 --challenge 1 --team 1
mazuadm-cli run add --exploit 1 --challenge 1 --team 2

# 5. Create and run a round
mazuadm-cli round new
mazuadm-cli round run 1

# 6. View flags
mazuadm-cli flag list --round 1
```

### Ad-hoc Exploit Execution

Run an exploit immediately without creating a round:

```bash
# Run exploit against team 2 immediately
mazuadm-cli exploit run pwn1-exp --challenge pwn1 --team 2
```

### Re-run a Failed Job

```bash
# Find the job ID
mazuadm-cli job list --round 1

# Re-run it
mazuadm-cli job run 42
```

## Environment Variables

- `DATABASE_URL` - PostgreSQL connection string (e.g., `postgres://user:pass@host:port/dbname`)
