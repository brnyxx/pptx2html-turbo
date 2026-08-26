from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_evidence import oracle_lock_ready
from evaluate.multiformat_portable_lock import (
    PortableLockError,
    PortableLockIncompleteError,
    portable_lock_template,
    validate_reference_lock,
)
from evaluate.multiformat_portable_package_inventory import (
    bind_package_executable_with_inventory,
    package_binding,
)
from evaluate.multiformat_reference_profile import ReferenceProfile
from evaluate.multiformat_reference_routing import load_reference_routing
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.tests.multiformat_portable_lock_fixture import PortableLockFixture

ROUTING_TABLE = (
    Path(__file__).resolve().parents[1] / "multiformat/reference-routing.v1.json"
)


class MultiFormatPortableLockTests(PortableLockFixture, unittest.TestCase):
    def test_template_artifact_bindings_are_independent(self) -> None:
        template = portable_lock_template()
        tools = self._mapping(template, "tools")
        libreoffice = self._mapping(tools, "libreoffice")
        poppler = self._mapping(tools, "poppler_render")

        libreoffice["path"] = "artifacts/soffice"

        self.assertEqual(poppler["path"], "")

    def test_missing_lock_is_not_ready(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)

            ready = oracle_lock_ready(root / "missing.json", root)

            self.assertFalse(ready)

    def test_malformed_lock_is_not_ready(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            path = root / "malformed.json"
            path.write_text("{", encoding="utf-8")

            ready = oracle_lock_ready(path, root)

            self.assertFalse(ready)

    def test_schema_two_lock_without_evidence_root_is_not_ready(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            _root, path, _lock = self._portable_lock(Path(temp_dir))

            ready = oracle_lock_ready(path)

            self.assertFalse(ready)

    def test_schema_two_lock_with_wrong_evidence_root_is_not_ready(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, path, _lock = self._portable_lock(Path(temp_dir) / "evidence")
            wrong_root = root.parent / "wrong-evidence"
            wrong_root.mkdir()

            ready = oracle_lock_ready(path, wrong_root)

            self.assertFalse(ready)

    def test_complete_lock_returns_typed_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, path, _lock = self._portable_lock(Path(temp_dir))

            identity = validate_reference_lock(path, root)

            self.assertEqual(identity.schema_version, 2)
            self.assertIs(identity.profile, ReferenceProfile.LIBREOFFICE_POPPLER)
            self.assertEqual(identity.sha256, sha256_file(path))
            self.assertEqual(identity.routing, load_reference_routing(ROUTING_TABLE))
            self.assertTrue(oracle_lock_ready(path, root))

    def test_routing_digest_substitution_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, path, lock = self._portable_lock(Path(temp_dir))
            lock["routing_table_sha256"] = "0" * 64
            self._write(path, lock)

            with self.assertRaisesRegex(
                PortableLockError,
                "routing table digest mismatch",
            ):
                validate_reference_lock(path, root)

    def test_linux_lock_and_attestation_return_typed_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, path, lock = self._portable_lock(Path(temp_dir))
            platform = self._mapping(lock, "platform")
            platform["os"] = "Linux"
            platform["architecture"] = "x86_64"
            runtime = self._mapping(lock, "runtime")
            binding = self._mapping(runtime, "attestation")
            attestation = root / self._string(binding, "path")
            values = json.loads(attestation.read_text(encoding="utf-8"))
            values["os"] = "Linux"
            values["architecture"] = "x86_64"
            self._write(attestation, values)
            binding["sha256"] = sha256_file(attestation)
            self._write(path, lock)

            identity = validate_reference_lock(path, root)

            self.assertIs(identity.profile, ReferenceProfile.LIBREOFFICE_POPPLER)
            self.assertEqual(identity.sha256, sha256_file(path))
            self.assertTrue(oracle_lock_ready(path, root))

    def test_incomplete_lock_reports_incomplete_before_ready(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            path = root / "oracle-lock.json"
            self._write(path, {"schema_version": 2, "status": "INCOMPLETE"})

            with self.assertRaises(PortableLockIncompleteError):
                validate_reference_lock(path, root)
            self.assertFalse(oracle_lock_ready(path))

    def test_invalid_identity_or_hash_fails_closed(self) -> None:
        mutations: tuple[tuple[str, JsonValue], ...] = (
            ("platform.os", "Windows"),
            ("platform.architecture", "sparc"),
            ("reference_profile", "microsoft-office"),
            ("schema_version", 1),
            ("routing_table_sha256", "not-a-hash"),
            ("canonicalizer.version", ""),
            ("runtime.locale", ""),
            ("runtime.timezone", ""),
            ("runtime.rendering_dpi", 96),
            ("runtime.network_isolation", False),
        )
        for field, value in mutations:
            with self.subTest(field=field), tempfile.TemporaryDirectory() as temp_dir:
                root, path, lock = self._portable_lock(Path(temp_dir))
                self._set(lock, field, value)
                self._write(path, lock)

                with self.assertRaises(PortableLockError):
                    validate_reference_lock(path, root)

    def test_missing_tampered_or_escaping_artifact_fails_closed(self) -> None:
        for attack in ("missing", "tampered", "traversal", "symlink"):
            with self.subTest(attack=attack), tempfile.TemporaryDirectory() as temp_dir:
                root, path, lock = self._portable_lock(Path(temp_dir))
                tool = self._mapping(self._mapping(lock, "tools"), "libreoffice")
                artifact = root / self._string(tool, "path")
                if attack == "missing":
                    artifact.unlink()
                elif attack == "tampered":
                    artifact.write_bytes(b"substituted")
                elif attack == "traversal":
                    tool["path"] = "../outside"
                else:
                    outside = root.parent / f"{root.name}-outside"
                    outside.write_bytes(b"outside")
                    link = root / "artifacts" / "escaped"
                    link.symlink_to(outside)
                    tool["path"] = "artifacts/escaped"
                    tool["sha256"] = sha256_file(outside)
                self._write(path, lock)

                with self.assertRaises(PortableLockError):
                    validate_reference_lock(path, root)
                self.assertFalse(oracle_lock_ready(path))

    def test_candidate_and_reference_sandbox_substitution_fails_closed(self) -> None:
        mutations = (
            ("candidate_sandbox.public_key.sha256", "0" * 64),
            ("candidate_sandbox.openssl.sha256", "1" * 64),
            ("candidate_sandbox.receipt_signer.sha256", "2" * 64),
            ("sandbox.executable.sha256", "3" * 64),
            ("sandbox.profile.sha256", "4" * 64),
        )
        for field, value in mutations:
            with self.subTest(field=field), tempfile.TemporaryDirectory() as temp_dir:
                root, path, lock = self._portable_lock(Path(temp_dir))
                self._set(lock, field, value)
                self._write(path, lock)

                with self.assertRaises(PortableLockError):
                    validate_reference_lock(path, root)

    def test_locked_app_sibling_and_symlink_tampering_fails_closed(self) -> None:
        for tool_name, attack in (
            ("libreoffice", "sibling"),
            ("chromium", "symlink"),
        ):
            with (
                self.subTest(tool=tool_name, attack=attack),
                tempfile.TemporaryDirectory() as temp_dir,
            ):
                root, path, lock = self._portable_lock(Path(temp_dir))
                source_app = root.parent / f"{root.name}-{tool_name}.app"
                executable = source_app / "Contents/MacOS/tool"
                resource = source_app / "Contents/Resources/data"
                executable.parent.mkdir(parents=True)
                resource.parent.mkdir(parents=True)
                executable.write_bytes(b"tool")
                resource.write_bytes(b"resource")
                (resource.parent / "alias").symlink_to("data")
                bound, inventory = bind_package_executable_with_inventory(
                    executable, root, root / "artifacts" / f"{tool_name}-package"
                )
                self.assertIsNotNone(inventory)
                tool = (
                    self._mapping(self._mapping(lock, "tools"), "libreoffice")
                    if tool_name == "libreoffice"
                    else self._mapping(self._mapping(lock, "browser"), "chromium")
                )
                tool.clear()
                tool.update(package_binding(root, bound, "test", inventory))
                self._write(path, lock)
                validate_reference_lock(path, root)

                copied_resource = bound.parents[1] / "Resources/data"
                if attack == "sibling":
                    copied_resource.write_bytes(b"tampered")
                else:
                    alias = copied_resource.with_name("alias")
                    alias.unlink()
                    outside = root.parent / f"{root.name}-outside"
                    outside.write_bytes(b"outside")
                    alias.symlink_to(outside)

                with self.assertRaises(PortableLockError):
                    validate_reference_lock(path, root)

    def test_runtime_attestation_drift_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, path, lock = self._portable_lock(Path(temp_dir))
            runtime = self._mapping(lock, "runtime")
            binding = self._mapping(runtime, "attestation")
            attestation = root / self._string(binding, "path")
            values = json.loads(attestation.read_text(encoding="utf-8"))
            values["timezone"] = "Europe/London"
            self._write(attestation, values)
            binding["sha256"] = sha256_file(attestation)
            self._write(path, lock)

            with self.assertRaises(PortableLockError):
                validate_reference_lock(path, root)

    def test_trust_substitution_fails_closed(self) -> None:
        mutations = (
            ("signer.signer_id", "substituted-signer"),
            ("signer.public_key.sha256", "0" * 64),
            ("signer.executor.sha256", "1" * 64),
        )
        for field, value in mutations:
            with self.subTest(field=field), tempfile.TemporaryDirectory() as temp_dir:
                root, path, lock = self._portable_lock(Path(temp_dir))
                self._set(lock, field, value)
                self._write(path, lock)

                with self.assertRaises(PortableLockError):
                    validate_reference_lock(path, root)


if __name__ == "__main__":
    unittest.main()
