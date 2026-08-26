from __future__ import annotations

import importlib
import importlib.util
import io
import json
import tempfile
import unittest
from collections.abc import Callable, Sequence
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from types import ModuleType
from typing import Protocol, runtime_checkable
from unittest.mock import patch

from evaluate.multiformat_ready_assembly_types import (
    ReadyAssemblyError,
    ReadyAssemblyFailure,
    ReadyAssemblyInputs,
    ReadyAssemblySummary,
    ReadyValidationInputs,
)
from evaluate.multiformat_ready_types import ReadyInputPaths


@runtime_checkable
class _CliModule(Protocol):
    main: Callable[[Sequence[str] | None], int]


class MultiFormatReadyCliTests(unittest.TestCase):
    def test_assemble_cli_binds_inputs_and_emits_summary(self) -> None:
        main = _load_main("evaluate.assemble_multiformat_ready_corpora")
        summary = _summary()
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            output = io.StringIO()
            with (
                patch(
                    "evaluate.assemble_multiformat_ready_corpora."
                    "assemble_ready_corpora",
                    return_value=summary,
                ) as assemble,
                redirect_stdout(output),
            ):
                exit_code = main([*_source_arguments(root), "--output-dir", str(root)])

        self.assertEqual(exit_code, 0)
        self.assertEqual(
            assemble.call_args.args,
            (ReadyAssemblyInputs(_source_paths(root), root),),
        )
        self.assertEqual(json.loads(output.getvalue()), summary.to_json_value())

    def test_validate_cli_binds_inputs_and_emits_summary(self) -> None:
        main = _load_main("evaluate.validate_multiformat_ready_corpora")
        summary = _summary()
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            output = io.StringIO()
            with (
                patch(
                    "evaluate.validate_multiformat_ready_corpora."
                    "validate_ready_corpora",
                    return_value=summary,
                ) as validate,
                redirect_stdout(output),
            ):
                exit_code = main([*_source_arguments(root), "--corpus-root", str(root)])

        self.assertEqual(exit_code, 0)
        self.assertEqual(
            validate.call_args.args,
            (ReadyValidationInputs(_source_paths(root), root),),
        )
        self.assertEqual(json.loads(output.getvalue()), summary.to_json_value())

    def test_cli_emits_typed_failure_as_json(self) -> None:
        main = _load_main("evaluate.validate_multiformat_ready_corpora")
        error = ReadyAssemblyError(
            ReadyAssemblyFailure.VALIDATION_FAILED,
            "invalid READY root",
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            output = io.StringIO()
            with (
                patch(
                    "evaluate.validate_multiformat_ready_corpora."
                    "validate_ready_corpora",
                    side_effect=error,
                ),
                redirect_stderr(output),
            ):
                exit_code = main([*_source_arguments(root), "--corpus-root", str(root)])

        self.assertEqual(exit_code, 1)
        self.assertEqual(
            json.loads(output.getvalue()),
            {"error": "validation-failed", "message": "invalid READY root"},
        )


def _load_main(module_name: str) -> Callable[[Sequence[str] | None], int]:
    if importlib.util.find_spec(module_name) is None:
        raise AssertionError(f"missing CLI module: {module_name}")
    module: ModuleType = importlib.import_module(module_name)
    if not isinstance(module, _CliModule):
        raise AssertionError(f"invalid CLI module: {module_name}")
    return module.main


def _summary() -> ReadyAssemblySummary:
    return ReadyAssemblySummary(
        "VALIDATED",
        7,
        1485,
        1295,
        180,
        1484,
        134026967,
        "a" * 64,
        "b" * 64,
    )


def _source_arguments(root: Path) -> list[str]:
    options = (
        ("--contract", "contract.json"),
        ("--plan", "plan.json"),
        ("--pptx-manifest", "pptx.json"),
        ("--docx-manifest", "docx.json"),
        ("--xlsx-manifest", "xlsx.json"),
        ("--pdf-manifest", "pdf.json"),
        ("--legacy-manifest", "legacy.json"),
        ("--public-config", "public-config.json"),
        ("--public-pool-manifest", "public-pool.json"),
        ("--legacy-binary-config", "legacy-binary-config.json"),
        ("--legacy-binary-manifest", "legacy-binary.json"),
        ("--security-manifest", "security.json"),
        ("--routing", "routing.json"),
        ("--font-bundle", "fonts.json"),
        ("--soffice", "soffice"),
        ("--pdfinfo", "pdfinfo"),
        ("--native-inventory-root", "inventory"),
    )
    return [value for option, name in options for value in (option, str(root / name))]


def _source_paths(root: Path) -> ReadyInputPaths:
    return ReadyInputPaths(
        root / "contract.json",
        root / "plan.json",
        root / "pptx.json",
        root / "docx.json",
        root / "xlsx.json",
        root / "pdf.json",
        root / "legacy.json",
        root / "public-config.json",
        root / "public-pool.json",
        root / "legacy-binary-config.json",
        root / "legacy-binary.json",
        root / "security.json",
        root / "routing.json",
        root / "fonts.json",
        root / "soffice",
        root / "pdfinfo",
        root / "inventory",
    )


if __name__ == "__main__":
    unittest.main()
