from __future__ import annotations

import os
import socket
import sys
import threading
from typing import Callable


def _env(name: str) -> str | None:
    value = os.environ.get(name)
    if value:
        return value
    return None


def _parse_target_args(argv: list[str]) -> tuple[str, int, str]:
    if len(argv) < 4:
        raise RuntimeError("missing target args: <host> <port> <team_id>")
    host = argv[1]
    port = int(argv[2])
    team_id = argv[3]
    return host, port, team_id


def get_target(argv: list[str]) -> tuple[str, int, str]:
    host = _env("TARGET_HOST")
    port = _env("TARGET_PORT")
    team_id = _env("TARGET_TEAM_ID")
    if host and port and team_id:
        return host, int(port), team_id
    return _parse_target_args(argv)


ConnectionHandler = Callable[[socket.socket], int]


class ListenerThread(threading.Thread):
    def __init__(
        self,
        listener: socket.socket,
        handler: ConnectionHandler,
        timeout_s: float = 8.0,
        timeout_message: str = "target did not connect back (is the attacker reachable?)",
    ) -> None:
        super().__init__()
        self._listener = listener
        self._handler = handler
        self._timeout_s = timeout_s
        self._timeout_message = timeout_message
        self.exit_code = 1

    def run(self) -> None:
        self._listener.settimeout(self._timeout_s)
        try:
            conn, _addr = self._listener.accept()
        except OSError:
            return
        except socket.timeout:
            print(f"[!] {self._timeout_message}", file=sys.stderr)
            return
        finally:
            try:
                self._listener.close()
            except Exception:
                pass

        try:
            self.exit_code = self._handler(conn)
        except Exception as exc:
            print(f"[!] listener failed: {exc}", file=sys.stderr)
        finally:
            try:
                conn.close()
            except Exception:
                pass

    def stop(self) -> None:
        try:
            self._listener.close()
        except Exception:
            pass


def start_listener(
    handler: ConnectionHandler,
    timeout_s: float = 8.0,
    timeout_message: str = "target did not connect back (is the attacker reachable?)",
) -> tuple[int, ListenerThread]:
    listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    listener.bind(("0.0.0.0", 0))
    listener.listen(1)
    callback_port = listener.getsockname()[1]
    thread = ListenerThread(listener, handler, timeout_s, timeout_message)
    thread.start()
    return callback_port, thread
