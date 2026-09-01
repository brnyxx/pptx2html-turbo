from __future__ import annotations

import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.multiformat_capture_types import (
    ArtifactIdentity,
    CaptureManifest,
    CaptureUnit,
)
from evaluate.multiformat_evaluator_files import EVALUATOR_FILES
from evaluate.multiformat_review_materialize import (
    ReviewMaterializeError,
    load_review_packet,
)
from evaluate.multiformat_review_packet import materialize_review_packet
from evaluate.multiformat_review_registry import (
    REGISTRY_PATH,
    ReviewRegistryError,
    load_reviewer_registry,
)
from evaluate.multiformat_schema import JsonValue, read_object, sha256_file
from evaluate.sign_multiformat_review_decision import sign_review_decision
from evaluate.tests.multiformat_review_registry_fixture import write_test_registry
from evaluate.validate_multiformat_review_decision import validate_completed_review

PAIRS = frozenset({"pair-1"})


def _bindings() -> dict[str, JsonValue]:
    return {
        "project_revision": "r" * 40,
        "contract_sha256": "1" * 64,
        "corpus_manifest_sha256": "2" * 64,
        "evaluator_manifest_sha256": "3" * 64,
        "oracle_lock_sha256": "4" * 64,
        "oracle_capture": {"path": "oracle.json", "sha256": "5" * 64},
        "candidate_capture": {"path": "candidate.json", "sha256": "6" * 64},
    }


def _capture(png: str, inventory: str) -> CaptureManifest:
    unit = CaptureUnit(
        "pair-1",
        "source",
        "e" * 64,
        1,
        ArtifactIdentity("png", png * 64),
        ArtifactIdentity("inventory", inventory * 64),
    )
    return CaptureManifest({"pair-1": unit}, {}, None)


def _objects(values: dict[str, JsonValue], field: str) -> list[dict[str, JsonValue]]:
    value = values[field]
    if not isinstance(value, list):
        raise TypeError(field)
    result: list[dict[str, JsonValue]] = []
    for item in value:
        if not isinstance(item, dict):
            raise TypeError(field)
        result.append(item)
    return result


def _strings(values: dict[str, JsonValue], field: str) -> list[str]:
    value = values[field]
    if not isinstance(value, list):
        raise TypeError(field)
    result: list[str] = []
    for item in value:
        if not isinstance(item, str):
            raise TypeError(field)
        result.append(item)
    return result


class TrackedReviewerRegistryTests(unittest.TestCase):
    def test_tracked_registry_binds_two_distinct_roles_and_public_keys(self) -> None:
        registry = load_reviewer_registry()

        self.assertEqual(len(registry.reviewers), 2)
        self.assertEqual(len({item.reviewer_id for item in registry.reviewers}), 2)
        self.assertEqual(len({item.reviewer_role for item in registry.reviewers}), 2)
        self.assertEqual(len({item.public_key for item in registry.reviewers}), 2)
        for reviewer in registry.reviewers:
            self.assertEqual(len(reviewer.public_key), 32)
            self.assertEqual(reviewer.reviewer_id, reviewer.reviewer_id.casefold())

    def test_tracked_registry_never_carries_private_key_material(self) -> None:
        # The registry and its key files must be publishable as-is.
        registry = load_reviewer_registry()
        raw = REGISTRY_PATH.read_bytes()
        for reviewer in registry.reviewers:
            self.assertNotIn(reviewer.public_key, raw)
        keys_root = REGISTRY_PATH.parent / "reviewer-keys"
        for path in sorted(keys_root.iterdir()):
            self.assertEqual(len(path.read_bytes()), 32)
            self.assertEqual(os.stat(path).st_mode & 0o777, 0o644)

    def test_registry_and_loader_and_tests_are_evaluator_bound(self) -> None:
        bound = set(EVALUATOR_FILES)
        expected = {
            "evaluate/multiformat/reviewer-registry.v1.json",
            "evaluate/multiformat/reviewer-keys/visual.ed25519.public",
            "evaluate/multiformat/reviewer-keys/semantic.ed25519.public",
            "evaluate/multiformat_review_registry.py",
            "evaluate/tests/multiformat_review_registry_fixture.py",
            "evaluate/tests/test_multiformat_review_registry.py",
        }

        self.assertEqual(expected - bound, set())

    def test_duplicate_registry_key_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            fixture = write_test_registry(Path(temporary), duplicate_key=True)

            with self.assertRaises(ReviewRegistryError):
                fixture.load()

    def test_registry_with_one_role_or_one_reviewer_is_rejected(self) -> None:
        cases = (
            (
                ("visual", "test-visual-reviewer", "visual-fidelity"),
                ("semantic", "test-semantic-reviewer", "visual-fidelity"),
            ),
            (("visual", "test-visual-reviewer", "visual-fidelity"),),
        )
        for reviewers in cases:
            with (
                self.subTest(count=len(reviewers)),
                tempfile.TemporaryDirectory() as temporary,
            ):
                fixture = write_test_registry(Path(temporary), reviewers=reviewers)
                with self.assertRaises(ReviewRegistryError):
                    fixture.load()

    def test_edited_registry_digest_or_public_key_bytes_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture = write_test_registry(root)
            self.assertEqual(len(fixture.load().reviewers), 2)

            value = read_object(fixture.path)
            _objects(value, "reviewers")[0]["public_key_sha256"] = "0" * 64
            fixture.path.write_text(json.dumps(value), encoding="utf-8")
            with self.assertRaises(ReviewRegistryError):
                fixture.load()

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture = write_test_registry(root)
            swapped = Ed25519PrivateKey.generate().public_key().public_bytes_raw()
            (root / "reviewer-keys" / "visual.public").write_bytes(swapped)
            with self.assertRaises(ReviewRegistryError):
                fixture.load()

    def test_registry_public_key_path_cannot_escape_the_registry_directory(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture = write_test_registry(root)
            value = read_object(fixture.path)
            _objects(value, "reviewers")[0]["public_key_path"] = "../escape.public"
            fixture.path.write_text(json.dumps(value), encoding="utf-8")
            (root.parent / "escape.public").write_bytes(b"\x01" * 32)

            with self.assertRaises(ReviewRegistryError):
                fixture.load()


class RegistryBoundPacketTests(unittest.TestCase):
    def _packet(self, root: Path, fixture_root: Path):
        fixture = write_test_registry(fixture_root)
        registry = fixture.load()
        with mock.patch(
            "evaluate.multiformat_review_packet.load_reviewer_registry",
            return_value=registry,
        ):
            summary = materialize_review_packet(
                root / "review",
                _capture("a", "b"),
                _capture("c", "d"),
                PAIRS,
                bindings=_bindings(),
            )
        return fixture, registry, summary

    def test_packet_materializes_exactly_the_registry_identities(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture, registry, summary = self._packet(root, root / "registry")
            packet = Path(str(summary["review_packet"]))

            value = read_object(packet)
            reviewers = _objects(value, "reviewers")

            self.assertEqual(
                {str(item["reviewer_id"]) for item in reviewers},
                {item.reviewer_id for item in registry.reviewers},
            )
            self.assertEqual(
                {str(item["reviewer_role"]) for item in reviewers},
                {item.reviewer_role for item in registry.reviewers},
            )
            self.assertEqual(
                {str(item["public_key_sha256"]) for item in reviewers},
                {item.public_key_sha256 for item in registry.reviewers},
            )
            self.assertEqual(len(fixture.private_keys), 2)

    def test_packet_validation_reloads_the_registry_and_rejects_wrong_keys(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture, registry, summary = self._packet(root, root / "registry")
            packet = Path(str(summary["review_packet"]))

            with mock.patch(
                "evaluate.multiformat_review_packet_trust.load_reviewer_registry",
                return_value=registry,
            ):
                trusts, packet_hash = load_review_packet(
                    packet, PAIRS, _capture("a", "b"), _capture("c", "d"), _bindings()
                )
                self.assertEqual(
                    set(trusts), {i.reviewer_id for i in registry.reviewers}
                )
                self.assertEqual(packet_hash, sha256_file(packet))

            # A different registry must not accept this packet's keys.
            other = write_test_registry(root / "other").load()
            with (
                mock.patch(
                    "evaluate.multiformat_review_packet_trust.load_reviewer_registry",
                    return_value=other,
                ),
                self.assertRaises(ReviewMaterializeError),
            ):
                load_review_packet(
                    packet, PAIRS, _capture("a", "b"), _capture("c", "d"), _bindings()
                )
            self.assertEqual(len(fixture.private_keys), 2)

    def test_producer_substituted_public_key_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _fixture, registry, summary = self._packet(root, root / "registry")
            packet = Path(str(summary["review_packet"]))

            # The producer self-authors a second identity it controls.
            forged = Ed25519PrivateKey.generate()
            public = forged.public_key().public_bytes_raw()
            value = read_object(packet)
            reviewers = _objects(value, "reviewers")
            reviewers[0]["public_key"] = public.hex()
            reviewers[0]["public_key_sha256"] = hashlib.sha256(public).hexdigest()
            packet.write_text(json.dumps(value), encoding="utf-8")

            with (
                mock.patch(
                    "evaluate.multiformat_review_packet_trust.load_reviewer_registry",
                    return_value=registry,
                ),
                self.assertRaises(ReviewMaterializeError),
            ):
                load_review_packet(
                    packet, PAIRS, _capture("a", "b"), _capture("c", "d"), _bindings()
                )

    def test_swapped_registry_roles_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            _fixture, registry, summary = self._packet(root, root / "registry")
            packet = Path(str(summary["review_packet"]))

            value = read_object(packet)
            reviewers = _objects(value, "reviewers")
            first, second = reviewers[0], reviewers[1]
            first["reviewer_role"], second["reviewer_role"] = (
                second["reviewer_role"],
                first["reviewer_role"],
            )
            packet.write_text(json.dumps(value), encoding="utf-8")

            with (
                mock.patch(
                    "evaluate.multiformat_review_packet_trust.load_reviewer_registry",
                    return_value=registry,
                ),
                self.assertRaises(ReviewMaterializeError),
            ):
                load_review_packet(
                    packet, PAIRS, _capture("a", "b"), _capture("c", "d"), _bindings()
                )

    def test_signed_decision_from_the_registry_key_validates(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            fixture, registry, summary = self._packet(root, root / "registry")
            packet = Path(str(summary["review_packet"]))
            first = Path(_strings(summary, "decision_templates")[0])
            value = read_object(first)
            _objects(value, "pairs")[0].update(
                {"decision": "PASS", "critical_defect": False}
            )
            first.write_text(json.dumps(value), encoding="utf-8")
            reviewer_id = str(value["reviewer_id"])
            signed = root / "signed.json"
            sign_review_decision(first, fixture.private_key(reviewer_id), signed)

            with mock.patch(
                "evaluate.multiformat_review_packet_trust.load_reviewer_registry",
                return_value=registry,
            ):
                summary_value = validate_completed_review(packet, signed)

            self.assertEqual(summary_value["status"], "VALID")
            self.assertEqual(summary_value["reviewer_id"], reviewer_id)


if __name__ == "__main__":
    unittest.main()
