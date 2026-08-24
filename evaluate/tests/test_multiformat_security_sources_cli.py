from __future__ import annotations

import contextlib
import io
import unittest
from pathlib import Path
from unittest import mock

from evaluate.generate_multiformat_security_sources import main as generate_main
from evaluate.multiformat_security_snapshot import (
    SecuritySnapshotError,
    SecuritySnapshotSummary,
)
from evaluate.validate_multiformat_security_sources import main as validate_main


class MultiFormatSecuritySourcesCliTests(unittest.TestCase):
    def test_generator_binds_contract_output_and_emits_canonical_success(self) -> None:
        summary = self._summary()
        stdout = io.StringIO()
        stderr = io.StringIO()
        with (
            mock.patch(
                "evaluate.generate_multiformat_security_sources."
                "generate_security_snapshot",
                return_value=summary,
            ) as generate,
            contextlib.redirect_stdout(stdout),
            contextlib.redirect_stderr(stderr),
        ):
            exit_code = generate_main(
                [
                    "--contract",
                    "contract.json",
                    "--output-dir",
                    "security",
                ]
            )

        self.assertEqual(exit_code, 0)
        generate.assert_called_once_with(
            Path("contract.json"),
            Path("security"),
        )
        self.assertEqual(stdout.getvalue(), self._success_json())
        self.assertEqual(stderr.getvalue(), "")

    def test_validator_binds_contract_manifest_and_emits_same_success(self) -> None:
        summary = self._summary()
        stdout = io.StringIO()
        stderr = io.StringIO()
        with (
            mock.patch(
                "evaluate.validate_multiformat_security_sources."
                "validate_security_snapshot",
                return_value=summary,
            ) as validate,
            contextlib.redirect_stdout(stdout),
            contextlib.redirect_stderr(stderr),
        ):
            exit_code = validate_main(
                [
                    "--contract",
                    "contract.json",
                    "--manifest",
                    "security/security-sources.json",
                ]
            )

        self.assertEqual(exit_code, 0)
        validate.assert_called_once_with(
            Path("contract.json"),
            Path("security/security-sources.json"),
        )
        self.assertEqual(stdout.getvalue(), self._success_json())
        self.assertEqual(stderr.getvalue(), "")

    def test_domain_failure_emits_canonical_error_and_exit_one(self) -> None:
        for module, main, arguments in (
            (
                (
                    "evaluate.generate_multiformat_security_sources."
                    "generate_security_snapshot"
                ),
                generate_main,
                ["--contract", "contract.json", "--output-dir", "security"],
            ),
            (
                (
                    "evaluate.validate_multiformat_security_sources."
                    "validate_security_snapshot"
                ),
                validate_main,
                [
                    "--contract",
                    "contract.json",
                    "--manifest",
                    "security/security-sources.json",
                ],
            ),
        ):
            with self.subTest(module=module):
                stdout = io.StringIO()
                stderr = io.StringIO()
                with (
                    mock.patch(
                        module,
                        side_effect=SecuritySnapshotError("broken"),
                    ),
                    contextlib.redirect_stdout(stdout),
                    contextlib.redirect_stderr(stderr),
                ):
                    exit_code = main(arguments)

                self.assertEqual(exit_code, 1)
                self.assertEqual(stdout.getvalue(), "")
                self.assertEqual(
                    stderr.getvalue(),
                    '{"error":"security-snapshot","message":"broken"}\n',
                )

    def test_usage_errors_remain_argparse_exit_two(self) -> None:
        for main in (generate_main, validate_main):
            with self.subTest(main=main), self.assertRaises(SystemExit) as raised:
                main([])

            self.assertEqual(raised.exception.code, 2)

    def _summary(self) -> SecuritySnapshotSummary:
        return SecuritySnapshotSummary(
            counts={
                "doc": 10,
                "docx": 10,
                "pdf": 10,
                "ppt": 10,
                "pptx": 10,
                "xls": 10,
                "xlsx": 10,
            },
            files=71,
            manifest_sha256="a" * 64,
        )

    def _success_json(self) -> str:
        return (
            '{"counts":{"doc":10,"docx":10,"pdf":10,"ppt":10,"pptx":10,'
            '"xls":10,"xlsx":10},"files":71,"manifest_sha256":"'
            + "a" * 64
            + '","schema_version":1,"status":"GENERATED"}\n'
        )


if __name__ == "__main__":
    unittest.main()
