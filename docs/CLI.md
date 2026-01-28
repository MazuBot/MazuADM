# MazuADM CLI Usage

## Overview

The MazuADM CLI provides command-line access to all features via the API server.

## Install

```bash
./scripts/install.sh
```

## Usage

```bash
mazuadm-cli [OPTIONS] <COMMAND>
```

## Global Options

- `--api <URL>` - API server URL (default: `http://localhost:3000`, env: `MAZUADM_API_URL`)

## Commands

### Challenge Management

```bash
mazuadm-cli challenge add --name "pwn1" --port 9001 --priority 100 --flag-regex "[A-Z0-9]{32}"
mazuadm-cli challenge list
mazuadm-cli challenge update pwn1 --name "pwn1-renamed" --port 9002
mazuadm-cli challenge delete pwn1
mazuadm-cli challenge enable pwn1
mazuadm-cli challenge disable pwn1
```

### Team Management

```bash
mazuadm-cli team add --id "team1" --name "Team 1" --ip "10.0.1.1" --priority 50
mazuadm-cli team list
mazuadm-cli team update team1 --name "Team One" --ip "10.0.1.2"
mazuadm-cli team delete team1
mazuadm-cli team enable team1
mazuadm-cli team disable team1
```

### Exploit Management

```bash
mazuadm-cli exploit create pwn1-exp --challenge pwn1 --config config.toml
mazuadm-cli exploit create . --challenge pwn1
mazuadm-cli exploit list
mazuadm-cli exploit list --challenge pwn1
mazuadm-cli exploit update pwn1-exp --challenge pwn1 --timeout 60
mazuadm-cli exploit delete pwn1-exp --challenge pwn1
mazuadm-cli exploit enable pwn1-exp --challenge pwn1
mazuadm-cli exploit disable pwn1-exp --challenge pwn1
mazuadm-cli exploit run pwn1-exp --challenge pwn1 --team team1
```

### Exploit Run Management

```bash
mazuadm-cli run add --exploit pwn1-exp --challenge pwn1 --team team1 --priority 100 --sequence 0
mazuadm-cli run list
mazuadm-cli run list --challenge pwn1
mazuadm-cli run list --team team1
mazuadm-cli run update 1 --priority 200 --sequence 1
mazuadm-cli run delete 1
mazuadm-cli run enable 1
mazuadm-cli run disable 1
mazuadm-cli run append-all --exploit pwn1-exp --challenge pwn1
mazuadm-cli run prepend-all --exploit pwn1-exp --challenge pwn1
```

### Round Management

```bash
mazuadm-cli round new
mazuadm-cli round list
mazuadm-cli round run 1
mazuadm-cli round rerun 1
mazuadm-cli round schedule-unflagged 1
mazuadm-cli round clean --db postgres://localhost/mazuadm
```

### Job Management

```bash
mazuadm-cli job list --round 1
mazuadm-cli job run 42
mazuadm-cli job stop 42
mazuadm-cli job set-priority 42 100
```

### Flag Management

```bash
mazuadm-cli flag list
mazuadm-cli flag list --round 1
```

### Settings Management

```bash
mazuadm-cli setting list
mazuadm-cli setting set concurrent_limit 10
mazuadm-cli setting set concurrent_create_limit 1
```

### Container Management

```bash
mazuadm-cli container list
mazuadm-cli container runners <container_id>
mazuadm-cli container delete <container_id>
mazuadm-cli container restart <container_id>
```

### Relation Management

```bash
mazuadm-cli relation list pwn1
mazuadm-cli relation get pwn1 team1
mazuadm-cli relation update pwn1 team1 --ip "10.0.1.100" --port 9999
```

## Environment Variables

- `MAZUADM_API_URL` - API server URL (default: `http://localhost:3000`)
