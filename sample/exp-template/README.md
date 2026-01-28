# Exploit Template

This folder is a starter exploit container template for MazuADM.

## Build

```bash
docker build -t my-exploit:latest .
```

## Register

```bash
mazuadm-cli exploit add --name "example-exp" --challenge 1 --image "my-exploit:latest"
```

## Required Environment

The runner injects:

- `TARGET_HOST`
- `TARGET_PORT`
- `TARGET_TEAM_ID`

Your exploit should print any flags to stdout. Update the HTTP request path and regex in `exploit.py` to match your service.
