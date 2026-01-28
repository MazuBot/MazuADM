# Exploit Template

This folder is a starter exploit container template for MazuADM.

## Build

```bash
docker build -t my-exploit:latest .
```

## Register

```bash
mazuadm-cli exploit add --config config.toml
```

Update an existing exploit:

```bash
mazuadm-cli exploit update 1 --config config.toml
```

## Required Environment

The runner injects:

- `TARGET_HOST`
- `TARGET_PORT`
- `TARGET_TEAM_ID`

If those are not set, the template accepts args: `exploit.py <host> <port> <team_id>`.

Optional overrides:

- `TARGET_PORT_OVERRIDE` - Replace the resolved port if set.

Your exploit should print any flags to stdout. Update the HTTP request path and regex in `exploit.py` to match your service.
