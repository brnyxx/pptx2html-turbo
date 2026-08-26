from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path


class PortableSandboxTests(unittest.TestCase):
    def test_real_profile_allows_local_execution_and_denies_socket(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            profile = Path(temp_dir) / "profile.sb"
            profile.write_text(
                '(version 1)\n(allow default)\n(deny network*)\n(allow network* (local unix-socket))\n(allow network* (remote unix-socket))\n(deny file-read* (subpath (param "ORACLE_ROOT")))\n'
            )
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
            self.assertEqual(local.returncode, 0)
            self.assertNotEqual(denied.returncode, 0)
            self.assertNotEqual(golden.returncode, 0)
            self.assertNotEqual(reference_read.returncode, 0)


if __name__ == "__main__":
    unittest.main()
