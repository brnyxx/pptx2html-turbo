from __future__ import annotations

import os
import socket
import subprocess
import sys
import tempfile
import threading
import unittest
from pathlib import Path
from unittest.mock import patch

from evaluate.multiformat_candidate_process import (
    CandidateProcessError,
    CandidateProcessFailure,
    run_bounded_process,
)


class MultiFormatNativeUnitDescendantTimeoutTests(unittest.TestCase):
    def test_timeout_kills_ready_descendant_group(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            child_pid = root / "child.pid"
            child_script = root / "child.py"
            _ = child_script.write_text(
                "import os, signal, sys\n"
                + f"from pathlib import Path\nPath({str(child_pid)!r}).write_text(str(os.getpid()))\n"
                + "os.write(int(sys.argv[1]), b'ready')\n"
                + "os.close(int(sys.argv[1]))\n"
                + "signal.pause()\n",
                encoding="utf-8",
            )
            parent_script = root / "parent.py"
            ready_socket = root / "ready.sock"
            _ = parent_script.write_text(
                "import os, signal, socket, subprocess, sys\n"
                + f"read_fd, write_fd = os.pipe()\nsubprocess.Popen([sys.executable, {str(child_script)!r}, str(write_fd)], pass_fds=(write_fd,))\n"
                + "os.close(write_fd)\nos.read(read_fd, 64)\nos.close(read_fd)\n"
                + f"with socket.socket(socket.AF_UNIX) as ready:\n    ready.connect({str(ready_socket)!r})\n    ready.sendall(b'ready')\n"
                + "signal.pause()\n",
                encoding="utf-8",
            )
            listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
            listener.bind(ready_socket.as_posix())
            listener.listen(1)
            listener.settimeout(5.0)
            ready = threading.Event()
            received: list[bytes] = []
            listener_errors: list[OSError] = []

            def accept_ready() -> None:
                try:
                    connection = listener.accept()[0]
                    with connection:
                        received.append(connection.recv(64))
                except OSError as error:
                    listener_errors.append(error)
                finally:
                    ready.set()

            wait_calls = 0
            patch_active = True

            def timeout_after_ready(
                process: subprocess.Popen[bytes],
                timeout: float | None = None,
            ) -> int:
                nonlocal patch_active, wait_calls
                wait_calls += 1
                if wait_calls == 1:
                    _ = ready.wait(timeout=5.0)
                    wait_patcher.stop()
                    patch_active = False
                    raise subprocess.TimeoutExpired(
                        process.args,
                        timeout if timeout is not None else 0.0,
                    )
                raise AssertionError("wait interceptor ran after restoration")

            listener_worker = threading.Thread(target=accept_ready, daemon=True)
            listener_worker.start()
            wait_patcher = patch.object(
                subprocess.Popen,
                "wait",
                timeout_after_ready,
            )
            _ = wait_patcher.start()
            try:
                with self.assertRaises(CandidateProcessError) as raised:
                    _ = run_bounded_process(
                        (sys.executable, parent_script.as_posix()),
                        root,
                        {"PATH": os.defpath},
                        root / "stdout",
                        root / "stderr",
                        timeout_seconds=0.0,
                        max_log_bytes=1024,
                    )
            finally:
                if patch_active:
                    wait_patcher.stop()
                listener.close()
                listener_worker.join(timeout=5.0)

            self.assertFalse(listener_worker.is_alive())
            self.assertEqual(listener_errors, [])
            self.assertEqual(received, [b"ready"])
            self.assertIs(raised.exception.failure, CandidateProcessFailure.TIMEOUT)
            child = int(child_pid.read_text(encoding="utf-8"))
            with self.assertRaises(ProcessLookupError):
                os.kill(child, 0)


if __name__ == "__main__":
    _ = unittest.main()
