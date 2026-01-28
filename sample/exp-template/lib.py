from __future__ import annotations

from dataclasses import dataclass
import os


@dataclass(frozen=True)
class Target:
    host: str
    port: int
    team_id: str


def _env(name: str) -> str | None:
    value = os.environ.get(name)
    if value:
        return value
    return None


def _parse_target_args(argv: list[str]) -> Target:
    if len(argv) < 4:
        raise RuntimeError("missing target args: <host> <port> <team_id>")
    host = argv[1]
    port = int(argv[2])
    team_id = argv[3]
    return Target(host=host, port=port, team_id=team_id)


def get_target(argv: list[str]) -> Target:
    host = _env("TARGET_HOST")
    port = _env("TARGET_PORT")
    team_id = _env("TARGET_TEAM_ID")
    if host and port and team_id:
        return Target(host=host, port=int(port), team_id=team_id)
    return _parse_target_args(argv)


def alloc_port(port: int | None) -> int | None:
    if port is None:
        return None
    value = os.environ.get("TARGET_PORT_OVERRIDE")
    if value:
        return int(value)
    return port
