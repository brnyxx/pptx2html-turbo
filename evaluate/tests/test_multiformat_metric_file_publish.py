from __future__ import annotations

import json
import os
import stat
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate import multiformat_metric_manifest as metric_manifest
from evaluate.multiformat_evaluator_files import EVALUATOR_FILES
from evaluate.multiformat_evaluator_portable_wave_files import (
    PORTABLE_WAVE_ENGINE_FILES,
    PORTABLE_WAVE_TEST_FILES,
)
from evaluate.multiformat_metric_file_publish import (
    MetricFilePublishError,
    publish_created_file,
)
from evaluate.multiformat_metric_manifest import (
    MetricsAssemblyError,
    publish_validated_metrics,
)

VICTIM_BYTES = b'{"victim": true}\n'
PAYLOAD = b'{"schema_version": 2}\n'
METRICS_VALUE = {"schema_version": 2, "status": "READY"}


def _identity(path: Path) -> tuple[int, int]:
    value = path.lstat()
    return value.st_dev, value.st_ino


class MetricFilePublishTests(unittest.TestCase):
    def test_published_file_is_a_validated_regular_single_link_inode(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "metrics.json"
            seen: list[bytes] = []

            publish_created_file(
                destination, PAYLOAD, lambda p: seen.append(p.read_bytes())
            )

            value = destination.lstat()
            self.assertEqual(seen, [PAYLOAD])
            self.assertEqual(destination.read_bytes(), PAYLOAD)
            self.assertTrue(stat.S_ISREG(value.st_mode))
            self.assertEqual(value.st_nlink, 1)
            self.assertFalse(destination.is_symlink())

    def test_pending_file_is_removed_after_publication(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            destination = root / "metrics.json"

            publish_created_file(destination, PAYLOAD, lambda _: None)

            self.assertEqual(
                sorted(entry.name for entry in root.iterdir()),
                ["metrics.json"],
            )

    def test_preexisting_regular_output_is_refused_and_left_intact(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "metrics.json"
            destination.write_bytes(VICTIM_BYTES)
            identity = _identity(destination)

            with self.assertRaises(MetricFilePublishError):
                publish_created_file(destination, PAYLOAD, lambda _: None)

            self.assertEqual(destination.read_bytes(), VICTIM_BYTES)
            self.assertEqual(_identity(destination), identity)

    def test_preexisting_output_symlink_is_refused_without_following(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            victim = root / "victim.json"
            victim.write_bytes(VICTIM_BYTES)
            destination = root / "metrics.json"
            destination.symlink_to(victim.name)

            with self.assertRaises(MetricFilePublishError):
                publish_created_file(destination, PAYLOAD, lambda _: None)

            self.assertEqual(victim.read_bytes(), VICTIM_BYTES)
            self.assertTrue(destination.is_symlink())

    def test_preexisting_pending_file_is_refused_and_left_intact(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            destination = root / "metrics.json"
            pending = root / ".metrics.json.pending"
            pending.write_bytes(VICTIM_BYTES)
            identity = _identity(pending)

            with self.assertRaises(MetricFilePublishError):
                publish_created_file(destination, PAYLOAD, lambda _: None)

            self.assertEqual(pending.read_bytes(), VICTIM_BYTES)
            self.assertEqual(_identity(pending), identity)
            self.assertFalse(os.path.lexists(destination))

    def test_pending_symlink_planted_before_the_write_is_never_followed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            victim = root / "victim.json"
            victim.write_bytes(VICTIM_BYTES)
            destination = root / "metrics.json"
            pending = root / ".metrics.json.pending"
            pending.symlink_to(victim.name)

            with self.assertRaises(MetricFilePublishError):
                publish_created_file(destination, PAYLOAD, lambda _: None)

            self.assertEqual(victim.read_bytes(), VICTIM_BYTES)
            self.assertFalse(os.path.lexists(destination))

    def test_pending_substituted_during_validation_cannot_be_published(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            victim = root / "victim.json"
            victim.write_bytes(VICTIM_BYTES)
            destination = root / "metrics.json"
            pending = root / ".metrics.json.pending"
            victim_identity = _identity(victim)

            def substitute(path: Path) -> None:
                # Deterministic race: the validated pending inode is unlinked and
                # its name is re-pointed at the victim before publication.
                self.assertEqual(path, pending)
                path.unlink()
                path.symlink_to(victim.name)

            with self.assertRaises(MetricFilePublishError):
                publish_created_file(destination, PAYLOAD, substitute)

            self.assertEqual(victim.read_bytes(), VICTIM_BYTES)
            self.assertEqual(_identity(victim), victim_identity)
            self.assertFalse(os.path.lexists(destination))

    def test_pending_hardlink_substitution_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            victim = root / "victim.json"
            victim.write_bytes(VICTIM_BYTES)
            destination = root / "metrics.json"

            def link(path: Path) -> None:
                # A second link to the pending inode means the published evidence
                # would stay writable through the attacker's name.
                os.link(path, root / "attacker-alias.json")

            with self.assertRaises(MetricFilePublishError):
                publish_created_file(destination, PAYLOAD, link)

            self.assertEqual(victim.read_bytes(), VICTIM_BYTES)
            self.assertFalse(os.path.lexists(destination))

    def test_validation_failure_removes_only_the_pending_inode(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            destination = root / "metrics.json"
            keeper = root / "keep.json"
            keeper.write_bytes(VICTIM_BYTES)

            def reject(_: Path) -> None:
                raise ValueError("invalid evidence")

            with self.assertRaises(ValueError):
                publish_created_file(destination, PAYLOAD, reject)

            self.assertEqual(
                sorted(entry.name for entry in root.iterdir()),
                ["keep.json"],
            )
            self.assertEqual(keeper.read_bytes(), VICTIM_BYTES)


class MetricFilePublishBindingTests(unittest.TestCase):
    def test_publisher_and_regression_are_digest_bound(self) -> None:
        producer = "evaluate/multiformat_metric_file_publish.py"
        regression = "evaluate/tests/test_multiformat_metric_file_publish.py"
        self.assertIn(producer, PORTABLE_WAVE_ENGINE_FILES)
        self.assertIn(regression, PORTABLE_WAVE_TEST_FILES)
        for path in (producer, regression, "evaluate/multiformat_metric_manifest.py"):
            with self.subTest(path=path):
                self.assertIn(path, EVALUATOR_FILES)


class PublishValidatedMetricsRaceTests(unittest.TestCase):
    def _publish(self, output: Path) -> None:
        publish_validated_metrics(
            METRICS_VALUE,
            output,
            mock.MagicMock(),
            output.parent / "contract.json",
            output.parent / "corpus.json",
            output.parent,
            output.parent / "lock.json",
        )

    def test_pending_substituted_in_the_race_window_is_never_published(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            victim = root / "victim.json"
            victim.write_bytes(VICTIM_BYTES)
            output = root / "metrics.json"
            pending = root / ".metrics.json.pending"
            victim_identity = _identity(victim)

            def substitute(*args: object, **_: object) -> None:
                # Deterministic race injected at the validation seam both the
                # pre-fix and the descriptor-pinned publisher pass through: the
                # validated pending name is re-pointed at the victim.
                validated = Path(str(args[2]))
                self.assertEqual(validated, pending)
                validated.unlink()
                validated.symlink_to(victim.name)

            with (
                mock.patch.object(
                    metric_manifest, "validate_metrics_evidence", substitute
                ),
                self.assertRaises(MetricsAssemblyError),
            ):
                self._publish(output)

            self.assertEqual(victim.read_bytes(), VICTIM_BYTES)
            self.assertEqual(_identity(victim), victim_identity)
            self.assertFalse(os.path.lexists(output))

    def test_validated_metrics_are_published_as_a_pinned_inode(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            output = root / "metrics.json"
            validated: list[Path] = []

            def record(*args: object, **_: object) -> None:
                validated.append(Path(str(args[2])))

            with mock.patch.object(
                metric_manifest, "validate_metrics_evidence", record
            ):
                self._publish(output)

            value = output.lstat()
            self.assertEqual(json.loads(output.read_bytes()), METRICS_VALUE)
            self.assertEqual(validated, [root / ".metrics.json.pending"])
            self.assertEqual(value.st_nlink, 1)
            self.assertTrue(stat.S_ISREG(value.st_mode))
            self.assertFalse(output.is_symlink())

    def test_existing_output_is_refused_before_any_pending_write(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            output = root / "metrics.json"
            output.write_bytes(VICTIM_BYTES)

            with self.assertRaises(MetricsAssemblyError):
                self._publish(output)

            self.assertEqual(output.read_bytes(), VICTIM_BYTES)
            self.assertFalse(os.path.lexists(root / ".metrics.json.pending"))


if __name__ == "__main__":
    unittest.main()
