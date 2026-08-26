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
DECOY_BYTES = b'{"decoy": "what a path validator would inspect"}\n'
METRICS_VALUE = {"schema_version": 2, "status": "READY"}


def _identity(path: Path) -> tuple[int, int]:
    value = path.lstat()
    return value.st_dev, value.st_ino


class MetricFilePublishTests(unittest.TestCase):
    def test_published_file_is_a_validated_regular_single_link_inode(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            destination = Path(temporary) / "metrics.json"
            seen: list[bytes] = []

            publish_created_file(destination, PAYLOAD, seen.append)

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

            def substitute(_: bytes) -> None:
                # Deterministic race: the validated pending inode is unlinked and
                # its name is re-pointed at the victim before publication.
                pending.unlink()
                pending.symlink_to(victim.name)

            with self.assertRaises(MetricFilePublishError):
                publish_created_file(destination, PAYLOAD, substitute)

            self.assertEqual(victim.read_bytes(), VICTIM_BYTES)
            self.assertEqual(_identity(victim), victim_identity)
            self.assertFalse(os.path.lexists(destination))

    def test_swap_for_validation_then_restore_cannot_smuggle_payload(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            destination = root / "metrics.json"
            pending = root / ".metrics.json.pending"
            aside = root / ".aside"
            observed: list[bytes] = []

            def swap_then_restore(source: bytes) -> None:
                # The attack an inode post-check alone cannot see: move the
                # pinned inode aside, expose a decoy at the pending name for the
                # validator, then restore the pinned inode so every identity
                # check still matches. A path-based validator would inspect the
                # decoy while the original, unvalidated bytes get published.
                observed.append(source)
                os.rename(pending, aside)
                pending.write_bytes(DECOY_BYTES)
                pending.unlink()
                os.rename(aside, pending)

            publish_created_file(destination, PAYLOAD, swap_then_restore)

            self.assertEqual(observed, [PAYLOAD])
            self.assertEqual(destination.read_bytes(), PAYLOAD)
            self.assertEqual(destination.lstat().st_nlink, 1)

    def test_decoy_left_at_the_pending_name_is_never_published(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            destination = root / "metrics.json"
            pending = root / ".metrics.json.pending"
            aside = root / ".aside"

            def swap_without_restore(_: bytes) -> None:
                # Same swap, but the decoy is left in place. The pinned identity
                # no longer matches the pending name, so nothing is published.
                os.rename(pending, aside)
                pending.write_bytes(DECOY_BYTES)

            with self.assertRaises(MetricFilePublishError):
                publish_created_file(destination, PAYLOAD, swap_without_restore)

            self.assertEqual(pending.read_bytes(), DECOY_BYTES)
            self.assertFalse(os.path.lexists(destination))

    def test_payload_rewritten_through_the_pinned_inode_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            destination = root / "metrics.json"
            pending = root / ".metrics.json.pending"

            def rewrite_in_place(_: bytes) -> None:
                # The inode identity is untouched, only its bytes change, so the
                # descriptor read-back is the only thing that can catch this.
                with pending.open("wb") as stream:
                    stream.write(DECOY_BYTES)

            with self.assertRaises(MetricFilePublishError):
                publish_created_file(destination, PAYLOAD, rewrite_in_place)

            self.assertFalse(os.path.lexists(destination))

    def test_pending_hardlink_substitution_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            victim = root / "victim.json"
            victim.write_bytes(VICTIM_BYTES)
            destination = root / "metrics.json"

            def link(_: bytes) -> None:
                # A second link to the pending inode means the published evidence
                # would stay writable through the attacker's name.
                os.link(root / ".metrics.json.pending", root / "alias.json")

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

            def reject(_: bytes) -> None:
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
        self.assertIn(
            "evaluate/multiformat_metrics_bindings.py", PORTABLE_WAVE_ENGINE_FILES
        )
        for path in (
            producer,
            regression,
            "evaluate/multiformat_metric_manifest.py",
            "evaluate/multiformat_metrics_bindings.py",
        ):
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

            def substitute(*_: object, **__: object) -> None:
                # Deterministic race at the validation seam: the pending name is
                # re-pointed at the victim while the validator runs.
                pending.unlink()
                pending.symlink_to(victim.name)

            with (
                mock.patch.object(
                    metric_manifest, "validate_metrics_bytes", substitute
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
            validated: list[bytes] = []

            def record(source: bytes, *_: object, **__: object) -> None:
                validated.append(source)

            with mock.patch.object(metric_manifest, "validate_metrics_bytes", record):
                self._publish(output)

            value = output.lstat()
            self.assertEqual(json.loads(output.read_bytes()), METRICS_VALUE)
            self.assertEqual(validated, [output.read_bytes()])
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

    def test_validator_receives_the_exact_published_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            output = root / "metrics.json"
            pending = root / ".metrics.json.pending"
            aside = root / ".aside"
            validated: list[bytes] = []

            def swap_then_restore(source: bytes, *_: object, **__: object) -> None:
                # Swap-for-validation-then-restore through the real assembly
                # entry point: the validator must still see published bytes.
                validated.append(source)
                os.rename(pending, aside)
                pending.write_bytes(DECOY_BYTES)
                pending.unlink()
                os.rename(aside, pending)

            with mock.patch.object(
                metric_manifest, "validate_metrics_bytes", swap_then_restore
            ):
                self._publish(output)

            self.assertEqual(validated, [output.read_bytes()])
            self.assertEqual(json.loads(output.read_bytes()), METRICS_VALUE)
            self.assertNotIn(b"decoy", output.read_bytes())


if __name__ == "__main__":
    unittest.main()
