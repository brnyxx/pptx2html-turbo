from __future__ import annotations

import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from cryptography.exceptions import InvalidSignature
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.jcs import canonicalize
from evaluate.multiformat_portable_receipt import (
    PortableReceiptError,
    PortableReceiptVerification,
)
from evaluate.multiformat_schema import JsonValue
from evaluate.tests.multiformat_portable_receipt_fixture import ReceiptFixture


class MultiFormatPortableReceiptTests(unittest.TestCase):
    def test_sign_and_verify_use_digest_bytes_and_return_stable_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = ReceiptFixture(Path(temp_dir))
            fixture.sign()

            identity = fixture.verify()
            receipt = fixture.read_receipt()
            payload = {"runtime": receipt["runtime"], "artifacts": receipt["artifacts"]}
            payload_bytes = canonicalize(payload)
            digest = hashlib.sha256(payload_bytes).digest()
            signature = bytes.fromhex(_string(receipt, "signature"))

            self.assertEqual(identity.payload_sha256, digest.hex())
            self.assertEqual(
                identity.artifacts[0].inode, fixture.artifact.stat().st_ino
            )
            fixture.private_key.public_key().verify(signature, digest)
            with self.assertRaises(InvalidSignature):
                fixture.private_key.public_key().verify(signature, payload_bytes)

    def test_self_signed_substituted_key_and_payload_transplant_fail(self) -> None:
        with (
            tempfile.TemporaryDirectory() as trusted_dir,
            tempfile.TemporaryDirectory() as attacker_dir,
        ):
            trusted = ReceiptFixture(Path(trusted_dir))
            attacker = ReceiptFixture(Path(attacker_dir), nonce=trusted.nonce)
            attacker.sign()
            trusted.receipt.write_bytes(attacker.receipt.read_bytes())

            with self.assertRaises(PortableReceiptError):
                trusted.verify()

    def test_envelope_key_or_signature_swapping_fails(self) -> None:
        with (
            tempfile.TemporaryDirectory() as first_dir,
            tempfile.TemporaryDirectory() as second_dir,
        ):
            first = ReceiptFixture(Path(first_dir))
            second = ReceiptFixture(Path(second_dir))
            first.sign()
            second.sign()
            receipt = first.read_receipt()
            other = second.read_receipt()
            for field in ("public_key", "public_key_sha256", "signature"):
                receipt[field] = other[field]
            first.receipt.write_bytes(canonicalize(receipt))

            with self.assertRaises(PortableReceiptError):
                first.verify()

    def test_missing_extra_or_tampered_source_fails(self) -> None:
        for attack in ("missing", "extra", "tampered"):
            with self.subTest(attack=attack), tempfile.TemporaryDirectory() as temp_dir:
                fixture = ReceiptFixture(Path(temp_dir))
                fixture.sign()
                if attack == "tampered":
                    (fixture.root / "corpus/source.docx").write_bytes(b"changed")
                else:
                    receipt = fixture.read_receipt()
                    runtime = _mapping(receipt, "runtime")
                    sources = _objects(runtime, "sources")
                    if attack == "missing":
                        sources.clear()
                    else:
                        sources.append(
                            {"path": "corpus/extra.docx", "sha256": "0" * 64, "size": 1}
                        )
                    fixture.resign(receipt)

                with self.assertRaises(PortableReceiptError):
                    fixture.verify()

    def test_hardlink_alias_across_artifact_roles_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = ReceiptFixture(Path(temp_dir))
            alias = fixture.root / "outputs/alias.pdf"
            os.link(fixture.artifact, alias)
            fixture.artifacts.append(
                fixture._artifact_record(path=alias, role="layout")
            )
            fixture.artifacts.sort(key=lambda item: str(item["path"]))
            fixture.sign()

            with self.assertRaisesRegex(PortableReceiptError, "alias"):
                fixture.verify()

    def test_post_hash_path_replacement_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = ReceiptFixture(Path(temp_dir))
            fixture.sign()

            def replace_after_hash(path: Path) -> None:
                if path.name == fixture.artifact.name:
                    replacement = path.with_suffix(".replacement")
                    replacement.write_bytes(fixture.artifact.read_bytes())
                    replacement.replace(path)

            with (
                mock.patch(
                    "evaluate.multiformat_portable_receipt_validation._after_file_hash",
                    side_effect=replace_after_hash,
                ),
                self.assertRaisesRegex(PortableReceiptError, "changed"),
            ):
                fixture.verify()

    def test_unsigned_prior_receipt_cannot_influence_replay(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = ReceiptFixture(Path(temp_dir))
            fixture.sign()
            unsigned = fixture.root / "unsigned.json"
            unsigned.write_text(json.dumps({"nonce": fixture.nonce}), encoding="utf-8")
            verification = PortableReceiptVerification(
                trust=fixture.trust,
                prior_receipts=(unsigned,),
            )

            with self.assertRaisesRegex(PortableReceiptError, "identity"):
                fixture.verify(verification)

    def test_cross_scope_identity_does_not_control_replay(self) -> None:
        with (
            tempfile.TemporaryDirectory() as first_dir,
            tempfile.TemporaryDirectory() as second_dir,
        ):
            first = ReceiptFixture(Path(first_dir))
            second = ReceiptFixture(Path(second_dir), nonce=first.nonce)
            first.sign()
            second.sign()
            prior = second.verify()

            identity = first.verify(first.verification((prior,)))

            self.assertEqual(identity.nonce, first.nonce)

    def test_same_scope_verified_nonce_replay_fails(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = ReceiptFixture(Path(temp_dir))
            fixture.sign()
            prior = fixture.verify()

            with self.assertRaisesRegex(PortableReceiptError, "nonce"):
                fixture.verify(fixture.verification((prior,)))

    def test_duplicate_noncanonical_malformed_and_signed_field_tamper_fail(
        self,
    ) -> None:
        attacks = (
            b'{"schema_version":1,"schema_version":1}',
            b'{"schema_version":1}',
            b'{"unterminated":',
        )
        for attack in attacks:
            with self.subTest(attack=attack), tempfile.TemporaryDirectory() as temp_dir:
                fixture = ReceiptFixture(Path(temp_dir))
                fixture.sign()
                if attack == b'{"schema_version":1}':
                    attack = json.dumps(fixture.read_receipt(), indent=2).encode()
                fixture.receipt.write_bytes(attack)
                with self.assertRaises(PortableReceiptError):
                    fixture.verify()

        for path, replacement in (
            ("runtime.nonce", "0" * 64),
            ("runtime.executor_sha256", "0" * 64),
            ("runtime.reference_lock.sha256", "0" * 64),
            ("runtime.routing_table_sha256", "0" * 64),
            ("runtime.corpus_sha256", "0" * 64),
            ("runtime.contract_sha256", "0" * 64),
            ("runtime.evaluator_sha256", "0" * 64),
            ("runtime.project_revision", "0" * 40),
            ("runtime.reference_profile", "microsoft-office"),
            ("runtime.platform.os", "Linux"),
            ("runtime.canonicalizer.sha256", "0" * 64),
            ("artifacts.0.role", "other"),
        ):
            with self.subTest(path=path), tempfile.TemporaryDirectory() as temp_dir:
                fixture = ReceiptFixture(Path(temp_dir))
                fixture.sign()
                value = fixture.read_receipt()
                _set(value, path, replacement)
                fixture.receipt.write_bytes(canonicalize(value))
                with self.assertRaises(PortableReceiptError):
                    fixture.verify()

    def test_wrong_key_and_signature_malleation_fail(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            fixture = ReceiptFixture(Path(temp_dir))
            fixture.sign()
            receipt = fixture.read_receipt()
            signature = bytearray.fromhex(_string(receipt, "signature"))
            signature[-1] ^= 1
            receipt["signature"] = signature.hex()
            fixture.receipt.write_bytes(canonicalize(receipt))
            with self.assertRaises(PortableReceiptError):
                fixture.verify()

            wrong = Ed25519PrivateKey.generate().public_key().public_bytes_raw()
            receipt = fixture.read_receipt()
            receipt["public_key"] = wrong.hex()
            receipt["public_key_sha256"] = hashlib.sha256(wrong).hexdigest()
            fixture.receipt.write_bytes(canonicalize(receipt))
            with self.assertRaises(PortableReceiptError):
                fixture.verify()


def _string(value: dict[str, JsonValue], field: str) -> str:
    result = value[field]
    if not isinstance(result, str):
        raise TypeError(field)
    return result


def _mapping(value: dict[str, JsonValue], field: str) -> dict[str, JsonValue]:
    result = value[field]
    if not isinstance(result, dict):
        raise TypeError(field)
    return result


def _objects(value: dict[str, JsonValue], field: str) -> list[dict[str, JsonValue]]:
    result = value[field]
    if not isinstance(result, list) or not all(
        isinstance(item, dict) for item in result
    ):
        raise TypeError(field)
    return result


def _set(value: dict[str, JsonValue], path: str, replacement: JsonValue) -> None:
    current: JsonValue = value
    parts = path.split(".")
    for part in parts[:-1]:
        current = current[int(part)] if isinstance(current, list) else current[part]
    if isinstance(current, list):
        current[int(parts[-1])] = replacement
    else:
        current[parts[-1]] = replacement
