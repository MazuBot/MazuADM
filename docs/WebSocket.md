# WebSocket API

## Endpoint

`GET /ws`

## Query Parameters

| Parameter | Required | Description |
|-----------|----------|-------------|
| `user` | Yes | User identifier (3-16 alphanumeric characters) |
| `client` | No | Client identifier (e.g., `web-ui-dev`) |
| `events` | No | Event subscription filter |

### User Parameter

The `user` parameter is mandatory. If missing or invalid, the server will:
1. Accept the WebSocket connection
2. Send an error message: `{"type": "error", "data": {"message": "..."}}`
3. Close the connection

### Events Parameter

`?events=<prefix1>,<prefix2>,...`

- **Absent**: Subscribe to all events
- **Empty** (`?events=`): Subscribe to nothing
- **With values** (`?events=job,flag`): Subscribe to events matching prefixes

### Available Prefixes

`challenge`, `team`, `exploit`, `exploit_run`, `round`, `job`, `flag`, `setting`, `ws_connection`, `connection_info`

## Dynamic Subscription

Send JSON messages to change subscriptions:

```json
{"action": "subscribe", "events": ["job", "flag"]}
{"action": "unsubscribe", "events": ["job"]}
```

## Message Structure

```json
{
  "type": "<event_name>",
  "data": <payload>
}
```

## Events

### Challenges
| Event | Data | Description |
|-------|------|-------------|
| `challenge_created` | `Challenge` | New challenge created |
| `challenge_updated` | `Challenge` | Challenge modified |
| `challenge_deleted` | `i32` | Challenge ID deleted |

### Teams
| Event | Data | Description |
|-------|------|-------------|
| `team_created` | `Team` | New team created |
| `team_updated` | `Team` | Team modified |
| `team_deleted` | `i32` | Team ID deleted |

### Exploits
| Event | Data | Description |
|-------|------|-------------|
| `exploit_created` | `Exploit` | New exploit created |
| `exploit_updated` | `Exploit` | Exploit modified |
| `exploit_deleted` | `i32` | Exploit ID deleted |

### Exploit Runs
| Event | Data | Description |
|-------|------|-------------|
| `exploit_run_created` | `ExploitRun` | New exploit run created |
| `exploit_run_updated` | `ExploitRun` | Exploit run modified |
| `exploit_run_deleted` | `i32` | Exploit run ID deleted |
| `exploit_runs_reordered` | `Vec<ReorderItem>` | Exploit runs reordered |

### Rounds
| Event | Data | Description |
|-------|------|-------------|
| `round_created` | `Round` | New round created |
| `round_updated` | `Round` | Round status/progress changed |

### Jobs
| Event | Data | Description |
|-------|------|-------------|
| `job_created` | `ExploitJob` | New job created |
| `job_updated` | `ExploitJob` | Job status/result changed |

### Flags
| Event | Data | Description |
|-------|------|-------------|
| `flag_created` | `Flag` | New flag captured |

### Settings
| Event | Data | Description |
|-------|------|-------------|
| `setting_updated` | `UpdateSetting` | Setting value changed |

### WebSocket Connections
| Event | Data | Description |
|-------|------|-------------|
| `ws_connections` | `Vec<WsConnectionInfo>` | Connection list updated |

### Relations
| Event | Data | Description |
|-------|------|-------------|
| `connection_info_updated` | `ChallengeTeamRelation` | Connection info updated |

### Errors
| Event | Data | Description |
|-------|------|-------------|
| `error` | `{"message": string}` | Error (connection will close) |
