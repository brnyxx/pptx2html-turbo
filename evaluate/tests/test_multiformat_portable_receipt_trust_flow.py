from __future__ import annotations

import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from evaluate.multiformat_portable_receipt import PortableReceiptError
from evaluate.multiformat_portable_receipt_trust import (
    PortableReceiptTrustError,
    load_portable_receipt_trust,
)
from evaluate.tests.multiformat_portable_receipt_fixture import ReceiptFixture


class _CloseMutation:
    """Mutable callable that injects one deterministic close-boundary write."""

    __slots__ = ("count", "fixture", "original_close", "target", "target_close")

    def __init__(
        self,
        fixture: ReceiptFixture,
        target: os.stat_result,
        target_close: int,
    ) -> None:
        self.fixture = fixture
        self.target = target
        self.target_close = target_close
        self.original_close = os.close
        self.count = 0

    def __call__(self, descriptor: int) -> None:
        descriptor_stat = os.fstat(descriptor)
        if (descriptor_stat.st_dev, descriptor_stat.st_ino) == (
            self.target.st_dev,
            self.target.st_ino,
        ):
            self.count += 1
            if self.count == self.target_close:
                self.fixture.artifact.write_bytes(b"X" * self.target.st_size)
                os.utime(
                    self.fixture.artifact,
                    ns=(self.target.st_atime_ns, self.target.st_mtime_ns),
                )
        self.original_close(descriptor)


class MultiFormatPortableReceiptTrustFlowTests(unittest.TestCase):
    def test_equal_length_write_at_either_output_close_is_rejected(self) -> None:
        for target_close in (1, 2):
            with (
                self.subTest(target_close=target_close),
                tempfile.TemporaryDirectory() as temp_dir,
            ):
                fixture = ReceiptFixture(Path(temp_dir))
                fixture.sign()
                target = fixture.artifact.stat()
                mutation = _CloseMutation(fixture, target, target_close)

                with (
                    mock.patch(
                        "evaluate.multiformat_portable_receipt_validation.os.close",
                        side_effect=mutation,
                    ),
                    self.assertRaisesRegex(PortableReceiptError, "changed"),
                ):
                    fixture.verify()
                self.assertEqual(mutation.count, target_close)

    def test_each_lock_bound_artifact_class_is_revalidated_after_context(self) -> None:
        attacks = (
            ("contract", "changed"),
            ("evaluator", "replaced"),
            ("corpus-manifest", "changed"),
            ("executor", "hardlinked"),
            ("public-key", "symlinked"),
            ("tool:libreoffice", "changed"),
            ("configuration", "replaced"),
            ("attestation", "changed"),
        )
        for role, attack in attacks:
            with (
                self.subTest(role=role, attack=attack),
                tempfile.TemporaryDirectory() as temp_dir,
            ):
                fixture = ReceiptFixture(Path(temp_dir))
                fixture.sign()
                identity = next(
                    item for item in fixture.trust.lock_artifacts if item.role == role
                )
                path = fixture.root / identity.path
                original = path.read_bytes()
                if attack == "changed":
                    path.write_bytes(b"tampered")
                elif attack == "replaced":
                    replacement = path.with_name(path.name + ".replacement")
                    replacement.write_bytes(original)
                    replacement.replace(path)
                elif attack == "hardlinked":
                    replacement = path.with_name(path.name + ".hardlink")
                    replacement.write_bytes(original)
                    path.unlink()
                    os.link(replacement, path)
                else:
                    replacement = path.with_name(path.name + ".target")
                    replacement.write_bytes(original)
                    path.unlink()
                    path.symlink_to(replacement.name)

                with self.assertRaises(PortableReceiptError):
                    fixture.verify()

    def test_source_exact_path_or_hardlink_cannot_be_signed_output(self) -> None:
        for attack in ("exact", "hardlink"):
            with self.subTest(attack=attack), tempfile.TemporaryDirectory() as temp_dir:
                fixture = ReceiptFixture(Path(temp_dir))
                source = fixture.root / fixture.trust.sources[0].path
                output = source
                if attack == "hardlink":
                    output = fixture.root / "outputs/source-alias.pdf"
                    output.parent.mkdir(parents=True, exist_ok=True)
                    os.link(source, output)
                fixture.artifacts = [fixture._artifact_record(path=output)]
                fixture.sign()

                with self.assertRaises(PortableReceiptError):
                    fixture.verify()

    def test_output_cannot_hardlink_to_tool_and_trust_roles_cannot_alias(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = ReceiptFixture(Path(temp_dir))
            tool = next(
                item
                for item in fixture.trust.lock_artifacts
                if item.role == "tool:libreoffice"
            )
            alias = fixture.root / "outputs/tool-alias.pdf"
            alias.parent.mkdir(parents=True, exist_ok=True)
            os.link(fixture.root / tool.path, alias)
            fixture.artifacts = [fixture._artifact_record(path=alias)]
            fixture.sign()
            with self.assertRaises(PortableReceiptError):
                fixture.verify()

            lock = json.loads(fixture.lock.read_text(encoding="utf-8"))
            lock["signer"]["executor"] = lock["scope"]["contract"]
            fixture.lock.write_text(json.dumps(lock, sort_keys=True), encoding="utf-8")
            with self.assertRaisesRegex(PortableReceiptTrustError, "alias"):
                load_portable_receipt_trust(fixture.lock, fixture.root)
