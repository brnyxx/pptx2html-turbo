from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path


class PortableSandboxTests(unittest.TestCase):
    def test_real_profile_allows_local_execution_and_denies_socket(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            profile = Path(temp_dir) / "profile.sb"
            profile.write_text("(version 1)\n(allow default)\n(deny network*)\n")
            local = subprocess.run(
                ["/usr/bin/sandbox-exec", "-f", str(profile), "/bin/echo", "ok"],
                capture_output=True,
                check=False,
            )
            denied = subprocess.run(
                [
                    "/usr/bin/sandbox-exec",
                    "-f",
                    str(profile),
                    "/usr/bin/python3",
                    "-c",
                    "import socket;socket.create_connection(('127.0.0.1',9),.1)",
                ],
                capture_output=True,
                check=False,
            )
            self.assertEqual(local.returncode, 0)
            self.assertNotEqual(denied.returncode, 0)


if __name__ == "__main__":
    unittest.main()
