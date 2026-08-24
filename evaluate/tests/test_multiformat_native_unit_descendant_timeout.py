from __future__ import annotations

import os
import socket
import sys
import tempfile
import threading
import unittest
from pathlib import Path

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
            finished = threading.Event()
            result: list[CandidateProcessError] = []

            def invoke() -> None:
                try:
                    _ = run_bounded_process(
                        (sys.executable, parent_script.as_posix()),
                        root,
                        {"PATH": os.defpath},
                        root / "stdout",
                        root / "stderr",
                        timeout_seconds=0.1,
                        max_log_bytes=1024,
                    )
                except CandidateProcessError as error:
                    result.append(error)
                finally:
                    finished.set()

            worker = threading.Thread(target=invoke, daemon=True)
            worker.start()
            try:
                connection = listener.accept()[0]
                with connection:
                    self.assertEqual(connection.recv(64), b"ready")
                self.assertTrue(finished.wait(timeout=5.0))
            finally:
                listener.close()

            self.assertEqual(len(result), 1)
            self.assertIs(result[0].failure, CandidateProcessFailure.TIMEOUT)
            child = int(child_pid.read_text(encoding="utf-8"))
            with self.assertRaises(ProcessLookupError):
                os.kill(child, 0)


if __name__ == "__main__":
    _ = unittest.main()
