from __future__ import annotations

import socket
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_portable_lock_io import write_sandbox_profile


class PortableSandboxTests(unittest.TestCase):
    def test_real_profile_denies_arbitrary_local_unix_socket_client(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            profile = root / "profile.sb"
            write_sandbox_profile(profile)
            oracle_root = root / "reference"
            oracle_root.mkdir()
            socket_path = root / "host-relay.sock"
            client_script = "".join(
                (
                    "import socket;client=socket.socket(socket.AF_UNIX);",
                    f"client.connect({socket_path.as_posix()!r});",
                    "client.sendall(b'candidate relay');client.close()",
                )
            )
            with socket.socket(socket.AF_UNIX) as server:
                server.bind(socket_path.as_posix())
                server.listen(1)
                client = subprocess.run(
                    [
                        "/usr/bin/sandbox-exec",
                        "-D",
                        f"ORACLE_ROOT={oracle_root}",
                        "-f",
                        profile.as_posix(),
                        "/usr/bin/python3",
                        "-c",
                        client_script,
                    ],
                    capture_output=True,
                    check=False,
                )
                if client.returncode == 0:
                    connection, _ = server.accept()
                    with connection:
                        self.assertEqual(connection.recv(64), b"candidate relay")
            self.assertNotEqual(client.returncode, 0)

    def test_real_profile_allows_local_execution_and_denies_socket(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            profile = Path(temp_dir) / "profile.sb"
            write_sandbox_profile(profile)
            oracle_root = Path(temp_dir) / "reference"
            oracle_root.mkdir()
            sentinel = oracle_root / ".candidate-denial-sentinel"
            sentinel.write_text("readable before sandbox", encoding="utf-8")
            reference = oracle_root / "rendered-reference.png"
            reference.write_bytes(b"reference bytes")
            prefix = [
                "/usr/bin/sandbox-exec",
                "-D",
                f"ORACLE_ROOT={oracle_root.resolve(strict=True)}",
                "-D",
                f"ORACLE_SENTINEL={sentinel.resolve(strict=True)}",
                "-f",
                str(profile),
            ]
            local = subprocess.run(
                [*prefix, "/bin/echo", "ok"],
                capture_output=True,
                check=False,
            )
            denied = subprocess.run(
                [
                    *prefix,
                    "/usr/bin/python3",
                    "-c",
                    "import socket;socket.create_connection(('127.0.0.1',9),.1)",
                ],
                capture_output=True,
                check=False,
            )
            golden = subprocess.run(
                [*prefix, "/bin/cat", str(sentinel)],
                capture_output=True,
                check=False,
            )
            reference_read = subprocess.run(
                [*prefix, "/bin/cat", str(reference)],
                capture_output=True,
                check=False,
            )
            direct = subprocess.run(
                [
                    *prefix,
                    sys.executable,
                    "-c",
                    "from pathlib import Path; "
                    "from evaluate.multiformat_candidate_sandbox_probe import "
                    "require_current_process_isolation; "
                    f"require_current_process_isolation(Path({str(oracle_root)!r}), "
                    f"Path({str(sentinel)!r}), '1.1.1.1:443')",
                ],
                capture_output=True,
                check=False,
            )
            self.assertEqual(local.returncode, 0)
            self.assertNotEqual(denied.returncode, 0)
            self.assertNotEqual(golden.returncode, 0)
            self.assertNotEqual(reference_read.returncode, 0)
            self.assertEqual(direct.returncode, 0, direct.stderr.decode())


if __name__ == "__main__":
    unittest.main()
