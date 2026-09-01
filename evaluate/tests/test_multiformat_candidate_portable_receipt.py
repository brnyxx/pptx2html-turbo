from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate.multiformat_candidate_portable_receipt import (
    _execute,
    write_portable_candidate_receipt,
)
from evaluate.multiformat_portable_receipt import (
    PortableReceiptInput,
    sign_portable_receipt,
)
from evaluate.tests.multiformat_portable_receipt_fixture import ReceiptFixture


class CandidatePortableReceiptTests(unittest.TestCase):
    def test_host_executor_boundary_must_return_a_real_portable_signature(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = ReceiptFixture(Path(temp_dir))
            output = fixture.root / "candidate-receipt"
            output.mkdir()

            def execute(_executor: Path, request: Path, receipt: Path) -> None:
                value = json.loads(request.read_text(encoding="utf-8"))
                self.assertEqual(value["schema_version"], 2)
                self.assertEqual(value["scope_sha256"], fixture.trust.scope_sha256)
                self.assertNotIn("nonce", value)
                sign_portable_receipt(
                    receipt,
                    PortableReceiptInput(
                        trust=fixture.trust,
                        batch_id=value["batch_id"],
                        artifacts=value["artifacts"],
                    ),
                    fixture.private_key,
                )

            receipt = write_portable_candidate_receipt(
                fixture.root,
                output,
                fixture.lock,
                fixture.root / "locked/executor",
                batch_id="candidate-docx",
                artifacts={fixture.artifact: "candidate-output"},
                execute=execute,
            )

            self.assertTrue(receipt.is_file())

    def test_executor_uses_bounded_process_group_logs(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            executor = root / "executor"
            executor.write_text("#!/bin/sh\n", encoding="utf-8")
            executor.chmod(0o700)
            request = root / "request.json"
            request.write_text("{}", encoding="utf-8")
            with mock.patch(
                "evaluate.multiformat_candidate_portable_receipt.run_bounded_process",
                return_value=0,
            ) as bounded:
                _execute(executor, request, root / "receipt.json")
            self.assertEqual(bounded.call_args.kwargs["max_log_bytes"], 1024 * 1024)


if __name__ == "__main__":
    unittest.main()
