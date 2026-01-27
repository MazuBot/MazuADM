# MazuADM CLI Usage

## Environment

```bash
export DATABASE_URL=postgres://localhost/mazuadm
```

Or pass `--db <URL>` to each command.

---

## Commands

### Challenges

```bash
# Add a challenge
mazuadm-cli challenge add --name "pwn1" --port 9001 --priority 100

# List all challenges
mazuadm-cli challenge list

# Enable/disable
mazuadm-cli challenge enable 1
mazuadm-cli challenge disable 1
```

### Teams

```bash
# Add a team
mazuadm-cli team add --id "team1" --name "Team Alpha" --ip "10.0.1.1" --priority 50

# List all teams
mazuadm-cli team list
```

### Exploits

```bash
# Add an exploit
mazuadm-cli exploit add \
  --name "pwn1-overflow" \
  --challenge 1 \
  --image "myexploit:latest" \
  --priority 10 \
  --timeout 30 \
  --flag-regex "FLAG\{[^}]+\}"

# List exploits (optionally filter by challenge)
mazuadm-cli exploit list
mazuadm-cli exploit list --challenge 1
```

### Exploit Runs (Cards)

```bash
# Schedule an exploit against a team
mazuadm-cli run add --exploit 1 --challenge 1 --team 1 --sequence 0

# List runs
mazuadm-cli run list
mazuadm-cli run list --challenge 1 --team 2
```

### Rounds

```bash
# Create a new round (generates jobs from enabled runs)
mazuadm-cli round new

# List all rounds
mazuadm-cli round list

# Execute a round
mazuadm-cli round run 1

# View jobs in a round
mazuadm-cli round jobs 1
```

### Flags

```bash
# List captured flags
mazuadm-cli flag list
mazuadm-cli flag list --round 1
```

---

## Quick Start

```bash
# 1. Add challenge and teams
mazuadm-cli challenge add --name "web1" --port 8080
mazuadm-cli team add --id "t1" --name "Team 1" --ip "10.0.1.1"
mazuadm-cli team add --id "t2" --name "Team 2" --ip "10.0.1.2"

# 2. Add exploit
mazuadm-cli exploit add --name "web1-sqli" --challenge 1 --image "web1-exp:latest"

# 3. Schedule runs for all teams
mazuadm-cli run add --exploit 1 --challenge 1 --team 1
mazuadm-cli run add --exploit 1 --challenge 1 --team 2

# 4. Execute
mazuadm-cli round new
mazuadm-cli round run 1
mazuadm-cli flag list --round 1
```
