from __future__ import annotations

import json
import subprocess
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

ROUTING_TABLE = (
    Path(__file__).resolve().parents[1] / "multiformat/reference-routing.v1.json"
)
CARGO = Path(
    subprocess.run(
        ["rustup", "which", "cargo"], check=True, capture_output=True, text=True
    ).stdout.strip()
).resolve(strict=True)
RUSTC = Path(
    subprocess.run(
        ["rustup", "which", "rustc"], check=True, capture_output=True, text=True
    ).stdout.strip()
).resolve(strict=True)


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
            self.assertEqual(identity.routing, load_reference_routing(ROUTING_TABLE))
            self.assertTrue(oracle_lock_ready(path))

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

    @classmethod
    def _portable_lock(cls, root: Path) -> tuple[Path, Path, dict[str, JsonValue]]:
        names = (
            ("soffice", "pdftoppm", "pdftotext", "pdfinfo", "canonicalizer")
            + ("fonts", "configuration", "chromium", "candidate-runtime-lock")
            + ("public-key", "executor", "contract", "evaluator", "corpus")
            + ("candidate-public-key", "openssl", "receipt-signer")
            + ("sandbox-exec", "sandbox-profile", "sandbox-host")
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
                "sandbox_executable": cls._binding(root, artifacts["sandbox-exec"]),
                "sandbox_host_artifact": cls._binding(root, artifacts["sandbox-host"]),
                "sandbox_profile": cls._binding(root, artifacts["sandbox-profile"]),
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
            "rust_toolchain": {
                "cargo": {
                    "path": CARGO.as_posix(),
                    "sha256": sha256_file(CARGO),
                },
                "rustc": {
                    "path": RUSTC.as_posix(),
                    "sha256": sha256_file(RUSTC),
                },
            },
            "tools": {
                "libreoffice": {"version": "test", **bindings["soffice"]},
                "poppler_render": {"version": "test", **bindings["pdftoppm"]},
                "poppler_text": {"version": "test", **bindings["pdftotext"]},
                "poppler_metadata": {"version": "test", **bindings["pdfinfo"]},
            },
            "routing_table_sha256": load_reference_routing(ROUTING_TABLE).sha256,
            "canonicalizer": {"version": "1", **bindings["canonicalizer"]},
            "font_bundle": {"version": "test", **bindings["fonts"]},
            "configuration": {"version": "test", **bindings["configuration"]},
            "browser": {
                "chromium": {"version": "test", **bindings["chromium"]},
                "lock": bindings["configuration"],
            },
            "candidate_runtime_lock": bindings["candidate-runtime-lock"],
            "candidate_sandbox": {
                "public_key": bindings["candidate-public-key"],
                "openssl": bindings["openssl"],
                "receipt_signer": bindings["receipt-signer"],
            },
            "sandbox": {
                "executable": bindings["sandbox-exec"],
                "profile": bindings["sandbox-profile"],
            },
            "signer": {
                "algorithm": "ed25519",
                "signer_id": "multiformat-portable-reference-v1",
                "public_key": bindings["public-key"],
                "receipt_schema_version": 2,
                "executor": bindings["executor"],
            },
            "scope": {
                "format": "docx",
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
