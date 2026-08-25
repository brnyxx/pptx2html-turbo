from __future__ import annotations

import io
import json
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from typing import cast
from unittest.mock import patch

from evaluate.capture_multiformat_native_units import main
from evaluate.multiformat_native_unit_capture import NativeUnitCaptureInputs
from evaluate.multiformat_native_unit_types import NativeUnitError, NativeUnitFailure
from evaluate.multiformat_native_unit_validation import (
    NativeUnitInventorySummary,
    NativeUnitValidationInputs,
)


class CaptureMultiFormatNativeUnitsCliTests(unittest.TestCase):
    def test_capture_binds_exact_arguments_and_emits_summary(self) -> None:
        output = io.StringIO()
        with (
            patch(
                "evaluate.capture_multiformat_native_units.capture_native_unit_inventory",
                return_value=self._summary(),
            ) as capture,
            redirect_stdout(output),
        ):
            code = main(
                [
                    "capture",
                    *self._trusted_arguments(),
                    "--output-dir",
                    "inventory",
                    "--workers",
                    "4",
                ]
            )

        self.assertEqual(code, 0)
        inputs = cast(NativeUnitCaptureInputs, capture.call_args.args[0])
        self.assertEqual(inputs.output_dir, Path("inventory"))
        self.assertEqual(inputs.workers, 4)
        self.assertEqual(
            cast(dict[str, object], json.loads(output.getvalue())),
            self._summary_value(),
        )

    def test_validate_binds_inventory_root_and_emits_summary(self) -> None:
        output = io.StringIO()
        with (
            patch(
                "evaluate.capture_multiformat_native_units.validate_native_unit_inventory",
                return_value=self._summary(),
            ) as validate,
            redirect_stdout(output),
        ):
            code = main(
                [
                    "validate",
                    *self._trusted_arguments(),
                    "--inventory-root",
                    "inventory",
                ]
            )

        self.assertEqual(code, 0)
        inputs = cast(NativeUnitValidationInputs, validate.call_args.args[0])
        self.assertEqual(inputs.inventory_root, Path("inventory"))
        self.assertEqual(
            cast(dict[str, object], json.loads(output.getvalue())),
            self._summary_value(),
        )

    def test_typed_domain_failure_returns_one_without_traceback(self) -> None:
        error = NativeUnitError(
            NativeUnitFailure.OUTPUT_INVALID,
            None,
            None,
            "fixture failure",
        )
        stderr = io.StringIO()
        with (
            patch(
                "evaluate.capture_multiformat_native_units.validate_native_unit_inventory",
                side_effect=error,
            ),
            redirect_stderr(stderr),
        ):
            code = main(
                [
                    "validate",
                    *self._trusted_arguments(),
                    "--inventory-root",
                    "inventory",
                ]
            )

        self.assertEqual(code, 1)
        value = cast(dict[str, object], json.loads(stderr.getvalue()))
        self.assertEqual(value["error"], "output-invalid")
        self.assertNotIn("Traceback", stderr.getvalue())

    def test_usage_error_exits_two(self) -> None:
        with (
            redirect_stderr(io.StringIO()),
            self.assertRaises(SystemExit) as raised,
        ):
            _ = main(["capture"])

        self.assertEqual(raised.exception.code, 2)

    @staticmethod
    def _trusted_arguments() -> list[str]:
        return [
            "--contract",
            "contract.json",
            "--public-config",
            "public-config.json",
            "--blind-manifest",
            "public-pool.json",
            "--routing",
            "routing.json",
            "--font-bundle",
            "font-bundle.json",
            "--soffice",
            "soffice",
            "--pdfinfo",
            "pdfinfo",
        ]

    @staticmethod
    def _summary() -> NativeUnitInventorySummary:
        return NativeUnitInventorySummary(3_151, 525, 1_050, 525, "a" * 64)

    @staticmethod
    def _summary_value() -> dict[str, int | str]:
        return {
            "files": 3_151,
            "manifest_sha256": "a" * 64,
            "observations": 1_050,
            "sources": 525,
            "total_units": 525,
        }


if __name__ == "__main__":
    _ = unittest.main()
