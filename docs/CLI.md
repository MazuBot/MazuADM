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

### Version

```bash
mazuadm-cli version    # show CLI and API version info
```

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
mazuadm-cli exploit init                                     # init template in current dir, prompt for challenge
mazuadm-cli exploit init myexp                               # create myexp/ dir with template
mazuadm-cli exploit init --challenge pwn1                    # init with challenge pre-selected
mazuadm-cli exploit create pwn1-exp --challenge pwn1 --image pwn1-exp:latest
mazuadm-cli exploit create pwn1-exp --challenge pwn1 --image pwn1-exp:latest --timeout 60
mazuadm-cli exploit create pwn1-exp --config config.toml      # uses challenge/image from config
mazuadm-cli exploit create pwn1-exp --config config.toml --timeout 60  # override config timeout
mazuadm-cli exploit pack --challenge pwn1                    # builds image, uses cwd name and config.toml
mazuadm-cli exploit pack pwn1-exp --challenge pwn1 --config config.toml
mazuadm-cli exploit list
mazuadm-cli exploit list --challenge pwn1
mazuadm-cli exploit update pwn1-exp --challenge pwn1 --timeout 60
mazuadm-cli exploit update pwn1-exp --challenge pwn1 --max-concurrent-jobs 3
mazuadm-cli exploit delete pwn1-exp --challenge pwn1
mazuadm-cli exploit enable pwn1-exp --challenge pwn1
mazuadm-cli exploit disable pwn1-exp --challenge pwn1
mazuadm-cli exploit run pwn1-exp --challenge pwn1 --team team1 # enqueue into running round
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
mazuadm-cli run reorder 1:0 2:1 3:2              # reorder runs by id:sequence
mazuadm-cli run append-all --exploit pwn1-exp --challenge pwn1
mazuadm-cli run prepend-all --exploit pwn1-exp --challenge pwn1
```

### Round Management

```bash
mazuadm-cli round new
mazuadm-cli round list
mazuadm-cli round current                        # show current running round
mazuadm-cli round run 1
mazuadm-cli round rerun 1
mazuadm-cli round rerun-unflagged 1              # only for running rounds
mazuadm-cli round clean --db postgres://localhost/mazuadm --confirm
mazuadm-cli round purge 5 --db postgres://localhost/mazuadm --confirm  # purge to round 5
```

### Job Management

```bash
mazuadm-cli job list --round 1
mazuadm-cli job get 42                           # show job details with stdout/stderr
mazuadm-cli job run 42 # enqueue into running round
mazuadm-cli job stop 42
mazuadm-cli job set-priority 42 100
```

### Flag Management

```bash
mazuadm-cli flag list
mazuadm-cli flag list --round 1
mazuadm-cli flag submit --round 1 --challenge pwn1 --team team01 "FLAG{...}"
mazuadm-cli flag submit --challenge pwn1 --team team01 "FLAG{...}" # uses running round
mazuadm-cli flag update 1:submitted 2:submitted  # update flag status (id:status)
mazuadm-cli flag update --force 1:pending        # force update even if already submitted
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
mazuadm-cli container restart-all
mazuadm-cli container remove-all
```

### WebSocket Connections

```bash
mazuadm-cli ws list
```

### Relation Management

```bash
mazuadm-cli relation list pwn1
mazuadm-cli relation get pwn1 team1
mazuadm-cli relation update pwn1 team1 --ip "10.0.1.100" --port 9999
mazuadm-cli relation enable pwn1 team1
mazuadm-cli relation disable pwn1 team1
```

## Environment Variables

- `MAZUADM_API_URL` - API server URL (default: `http://localhost:3000`)
