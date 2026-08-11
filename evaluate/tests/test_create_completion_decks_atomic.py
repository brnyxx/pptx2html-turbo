import hashlib
import io
import logging
import shutil
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate import create_completion_decks
from evaluate.completion_deck_manifest import ContractError
from evaluate.tests.completion_deck_test_support import (
    CANONICAL_MANIFEST,
    run_generator,
)


class CompletionDeckAtomicTests(unittest.TestCase):
    def test_output_policy_is_stable_and_non_destructive(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            file_output = root / "file"
            file_output.write_text("sentinel", encoding="utf-8")
            nonempty = root / "nonempty"
            nonempty.mkdir()
            sentinel = nonempty / "sentinel.txt"
            sentinel.write_text("keep", encoding="utf-8")
            for output, code in (
                (file_output, "OUTPUT_DIR_NOT_DIRECTORY"),
                (nonempty, "OUTPUT_DIR_NOT_EMPTY"),
            ):
                result = run_generator(output)
                self.assertEqual(result.returncode, 2)
                self.assertIn(code, result.stderr)
                self.assertNotIn("Traceback", result.stderr)
            self.assertEqual(file_output.read_text(), "sentinel")
            self.assertEqual(
                {path.name for path in nonempty.iterdir()}, {"sentinel.txt"}
            )
            canonical_digest = hashlib.sha256(
                CANONICAL_MANIFEST.read_bytes()
            ).hexdigest()
            source_output = run_generator(CANONICAL_MANIFEST.parent)
            self.assertEqual(source_output.returncode, 2)
            self.assertIn("OUTPUT_DIR_NOT_EMPTY", source_output.stderr)
            self.assertEqual(
                hashlib.sha256(CANONICAL_MANIFEST.read_bytes()).hexdigest(),
                canonical_digest,
            )
            symlink_target = root / "symlink-target"
            symlink_target.mkdir()
            symlink_output = root / "symlink-output"
            symlink_output.symlink_to(symlink_target, target_is_directory=True)
            symlink_result = run_generator(symlink_output)
            self.assertEqual(symlink_result.returncode, 2)
            self.assertIn("OUTPUT_DIR_SYMLINK", symlink_result.stderr)
            self.assertFalse(any(symlink_target.iterdir()))
            empty = root / "empty"
            empty.mkdir()
            result = run_generator(empty)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(len(tuple(empty.iterdir())), 11)

    def test_mid_write_failure_never_publishes_partial_output(self) -> None:
        original_write = Path.write_bytes

        for initially_exists in (False, True):
            with self.subTest(initially_exists=initially_exists):
                with tempfile.TemporaryDirectory() as tmp:
                    output = Path(tmp) / "out"
                    if initially_exists:
                        output.mkdir()
                    writes = 0

                    def fail_second_write(path: Path, payload: bytes) -> int:
                        nonlocal writes
                        writes += 1
                        if writes == 2:
                            raise OSError("injected write failure")
                        return original_write(path, payload)

                    with mock.patch.object(Path, "write_bytes", fail_second_write):
                        with self.assertRaisesRegex(
                            ContractError, "OUTPUT_WRITE_ERROR"
                        ):
                            create_completion_decks.generate(output, CANONICAL_MANIFEST)
                    self.assertTrue(not output.exists() or not any(output.iterdir()))
                    self.assertFalse(
                        any(
                            path.name.startswith(".out.stage-")
                            for path in Path(tmp).iterdir()
                        )
                    )

        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "out"
            writes = 0

            def fail_with_cleanup_error(path: Path, payload: bytes) -> int:
                nonlocal writes
                writes += 1
                if writes == 2:
                    raise OSError("original write failure")
                return original_write(path, payload)

            stderr = io.StringIO()
            handler = logging.StreamHandler(stderr)
            logger = create_completion_decks.logger
            previous_propagate = logger.propagate
            logger.addHandler(handler)
            logger.propagate = False
            try:
                with (
                    mock.patch.object(Path, "write_bytes", fail_with_cleanup_error),
                    mock.patch.object(
                        create_completion_decks.shutil,
                        "rmtree",
                        side_effect=OSError("cleanup failure"),
                    ),
                    mock.patch.object(
                        sys,
                        "argv",
                        ["create_completion_decks.py", "--output-dir", str(output)],
                    ),
                ):
                    self.assertEqual(create_completion_decks.main(), 2)
            finally:
                logger.removeHandler(handler)
                logger.propagate = previous_propagate
            logged = stderr.getvalue()
            leftovers = tuple(Path(tmp).glob(".out.stage-*"))
            try:
                self.assertIn("OUTPUT_WRITE_ERROR", logged)
                self.assertIn("OUTPUT_CLEANUP_ERROR", logged)
                self.assertIn("cleanup failure", logged)
                self.assertEqual(len(leftovers), 1)
                self.assertIn(str(leftovers[0].absolute()), logged)
                self.assertNotIn("Traceback", logged)
                self.assertFalse(output.exists())
            finally:
                for leftover in leftovers:
                    shutil.rmtree(leftover)

    def test_cleanup_report_preserves_primary_exception_chain(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / "out"
            original_write = Path.write_bytes
            writes = 0

            def fail_second_write(path: Path, payload: bytes) -> int:
                nonlocal writes
                writes += 1
                if writes == 2:
                    raise OSError("primary failure")
                return original_write(path, payload)

            with (
                mock.patch.object(Path, "write_bytes", fail_second_write),
                mock.patch.object(
                    create_completion_decks.shutil,
                    "rmtree",
                    side_effect=OSError("cleanup failure"),
                ),
                self.assertRaises(ContractError) as raised,
            ):
                create_completion_decks._publish(
                    output, {"first.pptx": b"first", "manifest.json": b"second"}
                )
            leftovers = tuple(Path(tmp).glob(".out.stage-*"))
            try:
                self.assertEqual(str(raised.exception.__cause__), "primary failure")
                self.assertIn("OUTPUT_WRITE_ERROR", str(raised.exception))
                self.assertIn("OUTPUT_CLEANUP_ERROR", str(raised.exception))
                self.assertEqual(len(leftovers), 1)
            finally:
                for leftover in leftovers:
                    shutil.rmtree(leftover)


if __name__ == "__main__":
    unittest.main()
