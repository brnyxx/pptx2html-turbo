from __future__ import annotations

import json
import tempfile
import unittest
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from unittest.mock import patch

from evaluate.materialize_multiformat_candidate_runtime_locks_cli import (
    main,
    parse_args,
)
from evaluate.tests.multiformat_candidate_runtime_lock_fixture import (
    candidate_runtime_lock_inputs,
)


class CandidateRuntimeLockMaterializerCliTests(unittest.TestCase):
    def test_cli_help_and_bad_input(self) -> None:
        with self.assertRaises(SystemExit) as help_exit:
            parse_args(["--help"])
        self.assertEqual(help_exit.exception.code, 0)
        with self.assertRaises(SystemExit) as bad_exit:
            parse_args([])
        self.assertEqual(bad_exit.exception.code, 2)

    def test_cli_dirty_worktree_returns_machine_readable_failure(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            # Given: complete materializer inputs in a dirty project worktree.
            inputs = candidate_runtime_lock_inputs(Path(temporary))
            (inputs.project_root / "dirty").write_text("dirty")
            output = StringIO()
            argv = [
                "--project-root",
                inputs.project_root.as_posix(),
                "--evidence-root",
                inputs.evidence_root.as_posix(),
                "--output-dir",
                inputs.output_dir.as_posix(),
                "--converter",
                inputs.converter.as_posix(),
                "--soffice",
                inputs.soffice.as_posix(),
                "--pdftohtml",
                inputs.pdftohtml.as_posix(),
                "--pdfinfo",
                inputs.pdfinfo.as_posix(),
                "--receipt-signer",
                inputs.receipt_signer.as_posix(),
                "--chromium",
                inputs.chromium.as_posix(),
                "--font-bundle",
                inputs.font_bundle.as_posix(),
                "--sandbox-public-key",
                inputs.sandbox_public_key.as_posix(),
                "--openssl",
                inputs.openssl.as_posix(),
                "--verifier-id",
                inputs.verifier_id,
            ]

            # When: the CLI evaluates the dirty worktree.
            with (
                patch("importlib.metadata.version", return_value="1.62.0"),
                redirect_stdout(output),
            ):
                result = main(argv)

            # Then: it returns the stable machine-readable failure without output.
            self.assertEqual(result, 1)
            self.assertEqual(
                json.loads(output.getvalue()),
                {
                    "status": "FAIL",
                    "reason": "candidate capture requires a clean worktree",
                },
            )
            self.assertFalse(inputs.output_dir.exists())


if __name__ == "__main__":
    unittest.main()
