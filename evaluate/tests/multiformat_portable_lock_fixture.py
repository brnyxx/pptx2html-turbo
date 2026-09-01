from __future__ import annotations

import json
import subprocess
from pathlib import Path

from evaluate.multiformat_conformance_pdf import pdf_canonicalizer_identity
from evaluate.multiformat_reference_routing import load_reference_routing
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.tests.multiformat_east_asian_font_fixture import (
    east_asian_font_binding,
)

_ROUTING_TABLE = (
    Path(__file__).resolve().parents[1] / "multiformat/reference-routing.v1.json"
)
_CARGO = Path(
    subprocess.run(
        ["rustup", "which", "cargo"], check=True, capture_output=True, text=True
    ).stdout.strip()
).resolve(strict=True)
_RUSTC = Path(
    subprocess.run(
        ["rustup", "which", "rustc"], check=True, capture_output=True, text=True
    ).stdout.strip()
).resolve(strict=True)


class PortableLockFixture:
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
                "os": "Linux",
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
            "platform": {"os": "Linux", "architecture": "arm64"},
            "rust_toolchain": {
                "cargo": {
                    "path": _CARGO.as_posix(),
                    "sha256": sha256_file(_CARGO),
                },
                "rustc": {
                    "path": _RUSTC.as_posix(),
                    "sha256": sha256_file(_RUSTC),
                },
            },
            "tools": {
                "libreoffice": {"version": "test", **bindings["soffice"]},
                "poppler_render": {"version": "test", **bindings["pdftoppm"]},
                "poppler_text": {"version": "test", **bindings["pdftotext"]},
                "poppler_metadata": {"version": "test", **bindings["pdfinfo"]},
            },
            "routing_table_sha256": load_reference_routing(_ROUTING_TABLE).sha256,
            "canonicalizer": {
                "version": pdf_canonicalizer_identity().version,
                **bindings["canonicalizer"],
            },
            "font_bundle": {"version": "test", **bindings["fonts"]},
            "east_asian_font": east_asian_font_binding(),
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
