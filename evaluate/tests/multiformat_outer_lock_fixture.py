from __future__ import annotations

import json
from pathlib import Path

from evaluate.multiformat_conformance_pdf import pdf_canonicalizer_identity
from evaluate.multiformat_reference_routing import load_reference_routing
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.tests.multiformat_candidate_gate_lock_fixture import (
    rust_toolchain_lock_value,
)
from evaluate.tests.multiformat_east_asian_font_fixture import (
    east_asian_font_binding,
)

_ROUTING = Path(__file__).resolve().parents[1] / "multiformat/reference-routing.v1.json"
PORTABLE_TEST_OS = "Linux"
PORTABLE_TEST_ARCHITECTURE = "arm64"


def write_portable_receipt_lock(
    root: Path,
    raw_key: bytes,
    candidate_runtime_lock: Path | None,
) -> Path:
    """Write the portable outer lock used by receipt tests."""
    contents = {
        "soffice": b"soffice",
        "pdftoppm": b"pdftoppm",
        "pdftotext": b"pdftotext",
        "pdfinfo": b"pdfinfo",
        "canonicalizer": b"canonicalizer",
        "fonts": b"fonts",
        "configuration": b"configuration",
        "chromium": b"chromium",
        "candidate-runtime-lock": b"candidate-runtime-lock",
        "browser-lock": b"browser-lock",
        "public-key": raw_key,
        "executor": b"executor",
        "contract": b"contract",
        "evaluator": b"evaluator",
        "candidate-public-key": b"candidate-public-key",
        "openssl": b"openssl",
        "receipt-signer": b"receipt-signer",
        "sandbox-exec": b"sandbox-exec",
        "sandbox-profile": b"sandbox-profile",
        "sandbox-host": b"sandbox-host",
    }
    artifacts: dict[str, Path] = {}
    for name, content in contents.items():
        path = root / f"locked/{name}"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        artifacts[name] = path
    source = root / "corpus/source.docx"
    source.parent.mkdir(parents=True, exist_ok=True)
    source.write_bytes(b"source")
    corpus = root / "corpus/manifest.json"
    corpus.write_text(
        json.dumps(
            {"sources": [{"path": source.name, "sha256": sha256_file(source)}]},
            sort_keys=True,
        ),
        encoding="utf-8",
    )
    binding: dict[str, dict[str, JsonValue]] = {
        name: _binding(root, path) for name, path in artifacts.items()
    }
    attestation = root / "locked/attestation.json"
    attestation.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "os": PORTABLE_TEST_OS,
                "architecture": PORTABLE_TEST_ARCHITECTURE,
                "locale": "en-US",
                "timezone": "UTC",
                "rendering_dpi": 144,
                "network_isolation": True,
                "sandbox_executable": binding["sandbox-exec"],
                "sandbox_host_artifact": binding["sandbox-host"],
                "sandbox_profile": binding["sandbox-profile"],
            },
            sort_keys=True,
        ),
        encoding="utf-8",
    )
    if candidate_runtime_lock is not None:
        binding["candidate-runtime-lock"] = _binding(root, candidate_runtime_lock)
    lock: dict[str, JsonValue] = {
        "schema_version": 2,
        "status": "locked",
        "reference_profile": "libreoffice-poppler",
        "platform": {
            "os": PORTABLE_TEST_OS,
            "architecture": PORTABLE_TEST_ARCHITECTURE,
        },
        "rust_toolchain": rust_toolchain_lock_value(),
        "tools": {
            "libreoffice": {"version": "test", **binding["soffice"]},
            "poppler_render": {"version": "test", **binding["pdftoppm"]},
            "poppler_text": {"version": "test", **binding["pdftotext"]},
            "poppler_metadata": {"version": "test", **binding["pdfinfo"]},
        },
        "routing_table_sha256": load_reference_routing(_ROUTING).sha256,
        "canonicalizer": {
            "version": pdf_canonicalizer_identity().version,
            **binding["canonicalizer"],
        },
        "font_bundle": {"version": "test", **binding["fonts"]},
        "east_asian_font": east_asian_font_binding(),
        "configuration": {"version": "test", **binding["configuration"]},
        "browser": {
            "chromium": {"version": "test", **binding["chromium"]},
            "lock": binding["browser-lock"],
        },
        "candidate_runtime_lock": binding["candidate-runtime-lock"],
        "candidate_sandbox": {
            "public_key": binding["candidate-public-key"],
            "openssl": binding["openssl"],
            "receipt_signer": binding["receipt-signer"],
        },
        "sandbox": {
            "executable": binding["sandbox-exec"],
            "profile": binding["sandbox-profile"],
        },
        "signer": {
            "algorithm": "ed25519",
            "signer_id": "multiformat-portable-reference-v1",
            "public_key": binding["public-key"],
            "receipt_schema_version": 2,
            "executor": binding["executor"],
        },
        "scope": {
            "format": "docx",
            "contract": binding["contract"],
            "evaluator": binding["evaluator"],
            "corpus": _binding(root, corpus),
            "project_revision": "6" * 40,
        },
        "runtime": {
            "locale": "en-US",
            "timezone": "UTC",
            "rendering_dpi": 144,
            "network_isolation": True,
            "attestation": _binding(root, attestation),
        },
    }
    path = root / "portable-lock.json"
    path.write_text(json.dumps(lock, sort_keys=True), encoding="utf-8")
    return path


def _binding(root: Path, path: Path) -> dict[str, JsonValue]:
    return {"path": path.relative_to(root).as_posix(), "sha256": sha256_file(path)}
