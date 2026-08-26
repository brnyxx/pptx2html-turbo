from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from typing import NamedTuple
from unittest import mock

from evaluate.materialize_multiformat_command_plan import materialize_command_plan
from evaluate.multiformat_command_plan import (
    CommandEvidenceError,
    CommandPlan,
    load_command_plan,
)
from evaluate.multiformat_command_runtime import run_performance_command
from evaluate.multiformat_schema import JsonValue, sha256_file

ProcessArgument = tuple[str, ...] | Path | dict[str, str] | float | int | None


class ToolchainFixture(NamedTuple):
    plan: CommandPlan
    cargo: Path


class RustToolchainRuntimeTrustTests(unittest.TestCase):
    def test_rejects_cargo_changed_before_execution(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture = self._fixture(root)
            fixture.cargo.write_text("#!/bin/sh\nexit 7\n", encoding="utf-8")

            with (
                mock.patch(
                    "evaluate.multiformat_command_runtime.process.run_bounded_process"
                ) as run,
                self.assertRaises(CommandEvidenceError),
            ):
                self._run_performance(fixture.plan, root)

            run.assert_not_called()

    def test_rejects_cargo_changed_during_execution(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture = self._fixture(root)

            def replace_cargo(
                *_args: ProcessArgument, **_kwargs: ProcessArgument
            ) -> int:
                fixture.cargo.write_text("#!/bin/sh\nexit 7\n", encoding="utf-8")
                return 0

            with (
                mock.patch(
                    "evaluate.multiformat_command_runtime.process.run_bounded_process",
                    side_effect=replace_cargo,
                ) as run,
                self.assertRaises(CommandEvidenceError),
            ):
                self._run_performance(fixture.plan, root)

            run.assert_called_once()

    @staticmethod
    def _run_performance(plan: CommandPlan, root: Path) -> dict[str, JsonValue]:
        return run_performance_command(
            plan,
            root,
            root / "execution",
            bindings={
                "project_revision": "r" * 40,
                "evaluator_hash": "e" * 64,
                "corpus_hash": "c" * 64,
            },
            working_directory=root,
            timeout_seconds=10,
        )

    @staticmethod
    def _fixture(root: Path) -> ToolchainFixture:
        root = root.resolve(strict=True)
        cargo = root / "cargo"
        rustc = root / "rustc"
        for name, destination in (("cargo", cargo), ("rustc", rustc)):
            source = Path(
                subprocess.run(
                    ["rustup", "which", name],
                    check=True,
                    capture_output=True,
                    text=True,
                ).stdout.strip()
            )
            shutil.copy2(source, destination)
        outer_lock = root / "outer-lock.json"
        outer_lock.write_text(
            json.dumps(
                {
                    "rust_toolchain": {
                        "cargo": {
                            "path": cargo.as_posix(),
                            "sha256": sha256_file(cargo),
                        },
                        "rustc": {
                            "path": rustc.as_posix(),
                            "sha256": sha256_file(rustc),
                        },
                    }
                }
            ),
            encoding="utf-8",
        )
        python = Path(sys.executable).resolve().as_posix()
        env = Path("/usr/bin/env").resolve().as_posix()
        path_arg = f"PATH={root.as_posix()}:/usr/bin:/bin"
        quality = {
            "tests": (
                env,
                path_arg,
                cargo.as_posix(),
                "test",
                "-p",
                "document2html-core",
                "-p",
                "document2html-native",
            ),
            "builds": (
                env,
                path_arg,
                cargo.as_posix(),
                "build",
                "--release",
                "-p",
                "pptx2html-cli",
                "--bin",
                "document2html",
            ),
            "diagnostics": (
                env,
                path_arg,
                cargo.as_posix(),
                "clippy",
                "-p",
                "document2html-core",
                "-p",
                "document2html-native",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ),
            "contract_checks": (
                python,
                "-m",
                "evaluate.check_exactness_contract",
                "--repo-root",
                root.as_posix(),
            ),
        }
        commands = root / "commands.json"
        materialize_command_plan(
            commands,
            (python, "-m", "evaluate.run_multiformat_security_case"),
            quality,
            (
                env,
                path_arg,
                cargo.as_posix(),
                "test",
                "--release",
                "-p",
                "document2html-native",
            ),
            outer_lock=outer_lock,
        )
        return ToolchainFixture(load_command_plan(commands), cargo)


if __name__ == "__main__":
    unittest.main()
