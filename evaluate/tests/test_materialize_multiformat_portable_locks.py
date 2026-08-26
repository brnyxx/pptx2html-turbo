from __future__ import annotations

import subprocess
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path

from evaluate.materialize_multiformat_portable_locks import (
    PortableLockInputs,
    materialize_portable_locks,
)
from evaluate.materialize_multiformat_portable_locks_cli import parse_args
from evaluate.multiformat_portable_reference_artifacts import load_raw_private_key
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture

PROJECT = Path(__file__).resolve().parents[2]


class PortableLockMaterializerTests(unittest.TestCase):
    def test_fixed_inputs_produce_deterministic_valid_lock_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            base = Path(temp_dir)
            first = self._fixture(base / "first")
            second = self._fixture(base / "second")
            first_lock = materialize_portable_locks(first)[0]
            second_lock = materialize_portable_locks(second)[0]
            self.assertEqual(first_lock.read_bytes(), second_lock.read_bytes())
            self.assertEqual(first.private_key.stat().st_mode & 0o777, 0o600)
            self.assertEqual(
                (first.output_dir / "keys/public.raw").stat().st_mode & 0o777, 0o644
            )
            sandbox = first.output_dir / "generated/sandbox-exec"
            local = subprocess.run(
                [
                    sandbox.as_posix(),
                    "-f",
                    (first.output_dir / "generated/portable-reference.sb").as_posix(),
                    "/bin/echo",
                    "ok",
                ],
                capture_output=True,
                check=False,
            )
            self.assertEqual(local.returncode, 0)

    def test_bad_private_permissions_and_overwrite_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            inputs = self._fixture(Path(temp_dir))
            inputs.private_key.chmod(0o644)
            with self.assertRaisesRegex(ValueError, "permissions"):
                materialize_portable_locks(inputs)
            inputs.private_key.chmod(0o600)
            inputs.output_dir.mkdir()
            with self.assertRaisesRegex(ValueError, "already exists"):
                materialize_portable_locks(inputs)

    def test_bad_tool_probe_and_private_scope_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir) / "evidence"
            inputs = self._fixture(root)
            inputs.libreoffice.write_text("#!/bin/sh\nexit 1\n")
            with self.assertRaisesRegex(ValueError, "version probe"):
                materialize_portable_locks(inputs)

            scoped = self._fixture(Path(temp_dir) / "scoped")
            scoped.private_key.unlink()
            scoped_key = scoped.evidence_root / "private.raw"
            scoped_key.write_bytes(b"1" * 32)
            scoped_key.chmod(0o600)
            with self.assertRaisesRegex(ValueError, "outside"):
                materialize_portable_locks(replace(scoped, private_key=scoped_key))

    def test_cli_help_and_bad_input(self) -> None:
        with self.assertRaises(SystemExit) as help_exit:
            parse_args(["--help"])
        self.assertEqual(help_exit.exception.code, 0)
        with self.assertRaises(SystemExit) as bad_exit:
            parse_args([])
        self.assertEqual(bad_exit.exception.code, 2)

    def _fixture(self, root: Path) -> PortableLockInputs:
        root.mkdir(parents=True, exist_ok=True)
        contract, corpus = ready_fixture(root)
        evaluator = root / "evaluator.json"
        evaluator.write_text("{}")
        tools = root / "tools"
        tools.mkdir()
        names = ["soffice", "pdftoppm", "pdftotext", "pdfinfo", "chromium"]
        paths = {}
        for name in names:
            path = tools / name
            path.write_text(f"#!/bin/sh\necho '{name} 1.0'\n")
            path.chmod(0o755)
            paths[name] = path
        plain = {}
        for name in ["canonicalizer", "fonts", "configuration", "executor"]:
            path = tools / name
            path.write_bytes(name.encode())
            plain[name] = path
        key = root.parent / f"{root.name}.private.raw"
        key.write_bytes(b"1" * 32)
        key.chmod(0o600)
        load_raw_private_key(key)
        return PortableLockInputs(
            PROJECT,
            root,
            root / "out",
            contract,
            evaluator,
            (corpus,),
            paths["soffice"],
            paths["pdftoppm"],
            paths["pdftotext"],
            paths["pdfinfo"],
            plain["canonicalizer"],
            plain["fonts"],
            plain["configuration"],
            paths["chromium"],
            plain["executor"],
            Path("/usr/bin/sandbox-exec"),
            key,
        )


if __name__ == "__main__":
    unittest.main()
