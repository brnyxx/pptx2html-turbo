from __future__ import annotations

import os
import socket
import tempfile
from collections.abc import Callable
from pathlib import Path
from typing import Final

NETWORK_ENDPOINT: Final = "1.1.1.1:443"
NETWORK_SCRIPT: Final = (
    "import socket; s=socket.create_connection(('1.1.1.1',443),2); s.close()"
)
ORACLE_SCRIPT: Final = (
    "import os,pathlib; pathlib.Path(os.environ['ORACLE_SENTINEL']).read_bytes()"
)
UNIX_SOCKET_SCRIPT: Final = """\
import socket
import tempfile
from pathlib import Path
with tempfile.TemporaryDirectory(prefix="candidate-unix-probe-") as temporary:
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
        listener.bind((Path(temporary) / "listener.sock").as_posix())
"""
_PROBE_TIMEOUT_SECONDS: Final = 2.0


class ActiveSandboxProbeError(ValueError):
    """The current process did not prove the required sandbox denials."""


def require_current_process_isolation(
    oracle_root: Path, sentinel: Path, endpoint: str
) -> None:
    """Probe sandbox restrictions without delegating them to child processes."""
    require_oracle_denied(oracle_root, sentinel)
    require_network_denied(endpoint)
    require_unix_socket_denied()


def require_oracle_denied(oracle_root: Path, sentinel: Path) -> None:
    _require_permission_denied(
        lambda: _open_directory(oracle_root), "candidate oracle root"
    )
    _require_permission_denied(
        lambda: _read_one_byte(sentinel), "candidate oracle sentinel"
    )


def require_network_denied(endpoint: str) -> None:
    host, port = _parse_endpoint(endpoint)
    connection = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    try:
        connection.settimeout(_PROBE_TIMEOUT_SECONDS)
        _require_permission_denied(
            lambda: connection.connect((host, port)),
            f"candidate endpoint {endpoint}",
        )
    finally:
        connection.close()


def require_unix_socket_denied() -> None:
    with tempfile.TemporaryDirectory(prefix="candidate-unix-probe-") as temporary:
        path = Path(temporary) / "listener.sock"
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as listener:
            _require_permission_denied(
                lambda: listener.bind(path.as_posix()),
                "candidate Unix socket creation",
            )


def _open_directory(path: Path) -> None:
    with os.scandir(path):
        pass


def _read_one_byte(path: Path) -> None:
    with path.open("rb") as stream:
        _ = stream.read(1)


def _require_permission_denied(action: Callable[[], None], label: str) -> None:
    try:
        action()
    except PermissionError:
        return
    except OSError as error:
        raise ActiveSandboxProbeError(f"{label} was not sandbox-denied") from error
    raise ActiveSandboxProbeError(f"{label} is readable or reachable")


def _parse_endpoint(endpoint: str) -> tuple[str, int]:
    try:
        host, raw_port = endpoint.rsplit(":", 1)
        port = int(raw_port)
    except (ValueError, TypeError) as error:
        raise ActiveSandboxProbeError("candidate endpoint is invalid") from error
    if not host or not 1 <= port <= 65535:
        raise ActiveSandboxProbeError("candidate endpoint is invalid")
    return host, port


__all__ = [
    "ActiveSandboxProbeError",
    "NETWORK_ENDPOINT",
    "NETWORK_SCRIPT",
    "ORACLE_SCRIPT",
    "UNIX_SOCKET_SCRIPT",
    "require_current_process_isolation",
    "require_network_denied",
    "require_oracle_denied",
    "require_unix_socket_denied",
]
