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
from evaluate.multiformat_reference_profile import ReferenceProfile
from evaluate.multiformat_schema import JsonValue, sha256_file


class MultiFormatPortableLockTests(unittest.TestCase):
    def test_template_artifact_bindings_are_independent(self) -> None:
        template = portable_lock_template()
        tools = self._mapping(template, "tools")
        libreoffice = self._mapping(tools, "libreoffice")
        poppler = self._mapping(tools, "poppler_render")

        libreoffice["path"] = "artifacts/soffice"

        self.assertEqual(poppler["path"], "")

    def test_complete_lock_returns_typed_identity(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root, path, _lock = self._portable_lock(Path(temp_dir))

            identity = validate_reference_lock(path, root)

            self.assertEqual(identity.schema_version, 2)
            self.assertIs(identity.profile, ReferenceProfile.LIBREOFFICE_POPPLER)
            self.assertEqual(identity.sha256, sha256_file(path))
            self.assertTrue(oracle_lock_ready(path))

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
            self.assertTrue(oracle_lock_ready(path))

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

    @classmethod
    def _portable_lock(cls, root: Path) -> tuple[Path, Path, dict[str, JsonValue]]:
        names = (
            "soffice",
            "pdftoppm",
            "pdftotext",
            "pdfinfo",
            "canonicalizer",
            "fonts",
            "configuration",
            "chromium",
            "candidate-runtime-lock",
            "public-key",
            "executor",
            "contract",
            "evaluator",
            "corpus",
        )
        artifacts = {name: cls._artifact(root, name, name.encode()) for name in names}
        attestation = cls._artifact(root, "attestation", b"")
        cls._write(
            attestation,
            {
                "schema_version": 1,
                "os": "Darwin",
                "architecture": "arm64",
                "locale": "en-US",
                "timezone": "UTC",
                "rendering_dpi": 144,
                "network_isolation": True,
            },
        )
        bindings = {
            name: cls._binding(root, artifact) for name, artifact in artifacts.items()
        }
        lock: dict[str, JsonValue] = {
            "schema_version": 2,
            "status": "locked",
            "reference_profile": "libreoffice-poppler",
            "platform": {"os": "Darwin", "architecture": "arm64"},
            "tools": {
                "libreoffice": {"version": "test", **bindings["soffice"]},
                "poppler_render": {"version": "test", **bindings["pdftoppm"]},
                "poppler_text": {"version": "test", **bindings["pdftotext"]},
                "poppler_metadata": {"version": "test", **bindings["pdfinfo"]},
            },
            "routing_table_sha256": "a" * 64,
            "canonicalizer": {"version": "1", **bindings["canonicalizer"]},
            "font_bundle": bindings["fonts"],
            "configuration": bindings["configuration"],
            "browser": {
                "chromium": {"version": "test", **bindings["chromium"]},
                "lock": bindings["configuration"],
            },
            "candidate_runtime_lock": bindings["candidate-runtime-lock"],
            "signer": {
                "algorithm": "ed25519",
                "signer_id": "multiformat-portable-reference-v1",
                "public_key": bindings["public-key"],
                "receipt_schema_version": 1,
                "executor": bindings["executor"],
            },
            "scope": {
                "contract": bindings["contract"],
                "evaluator": bindings["evaluator"],
                "corpus": bindings["corpus"],
                "project_revision": "b" * 40,
            },
            "runtime": {
                "locale": "en-US",
                "timezone": "UTC",
                "rendering_dpi": 144,
                "network_isolation": True,
                "attestation": cls._binding(root, attestation),
            },
        }
        path = root / "oracle-lock.json"
        cls._write(path, lock)
        return root, path, lock

    @staticmethod
    def _artifact(root: Path, name: str, content: bytes) -> Path:
        path = root / "artifacts" / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        return path

    @staticmethod
    def _binding(root: Path, path: Path) -> dict[str, JsonValue]:
        return {"path": path.relative_to(root).as_posix(), "sha256": sha256_file(path)}

    @staticmethod
    def _write(path: Path, value: JsonValue) -> None:
        path.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")

    @staticmethod
    def _mapping(values: dict[str, JsonValue], field: str) -> dict[str, JsonValue]:
        value = values[field]
        if not isinstance(value, dict):
            raise TypeError(field)
        return value

    @staticmethod
    def _string(values: dict[str, JsonValue], field: str) -> str:
        value = values[field]
        if not isinstance(value, str):
            raise TypeError(field)
        return value

    @classmethod
    def _set(cls, values: dict[str, JsonValue], path: str, value: JsonValue) -> None:
        parts = path.split(".")
        target = values
        for part in parts[:-1]:
            target = cls._mapping(target, part)
        target[parts[-1]] = value


if __name__ == "__main__":
    unittest.main()
