from __future__ import annotations

import json
from pathlib import Path
from typing import cast

from evaluate.multiformat_reference_routing import load_reference_routing
from evaluate.multiformat_schema import JsonValue, read_object, string_list
from evaluate.tests.multiformat_gate_fixture import (
    CONTRACT_PATH,
    PROJECT_ROOT,
    MultiFormatGateFixture,
)


class PerFormatGateFixture(MultiFormatGateFixture):
    def _write_per_format_locks(self, reports: Path) -> dict[str, Path]:
        root = reports.parent
        artifacts = root / "portable-lock-artifacts"
        artifacts.mkdir()
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
            "browser-lock",
            "public-key",
            "executor",
            "contract",
            "candidate-public-key",
            "openssl",
            "receipt-signer",
            "sandbox-exec",
            "sandbox-profile",
            "attestation",
        )
        paths: dict[str, Path] = {}
        for name in names:
            path = artifacts / name
            path.write_bytes((name * 3).encode())
            paths[name] = path
        paths["public-key"].write_bytes(b"p" * 32)
        paths["contract"].write_bytes(CONTRACT_PATH.read_bytes())
        paths["attestation"].write_text(
            json.dumps(
                {
                    "schema_version": 1,
                    "os": "Darwin",
                    "architecture": "arm64",
                    "locale": "en-US",
                    "timezone": "UTC",
                    "rendering_dpi": 144,
                    "network_isolation": True,
                    "sandbox_executable": self._binding(root, paths["sandbox-exec"]),
                    "sandbox_host_artifact": self._binding(root, paths["sandbox-exec"]),
                    "sandbox_profile": self._binding(root, paths["sandbox-profile"]),
                },
                sort_keys=True,
            ),
            encoding="utf-8",
        )
        binding = {name: self._binding(root, path) for name, path in paths.items()}
        evaluator = root / "evidence" / "evaluator-manifest.json"
        routing = load_reference_routing(
            PROJECT_ROOT / "evaluate" / "multiformat" / "reference-routing.v1.json"
        )
        lock_dir = root / "locks"
        lock_dir.mkdir()
        result: dict[str, Path] = {}
        required = string_list(read_object(CONTRACT_PATH), "required_formats")
        for document_format in required:
            corpus = root / "evidence" / "corpora" / document_format / "manifest.json"
            lock = cast(
                dict[str, JsonValue],
                {
                    "schema_version": 2,
                    "status": "locked",
                    "reference_profile": "libreoffice-poppler",
                    "platform": {"os": "Darwin", "architecture": "arm64"},
                    "tools": {
                        "libreoffice": {"version": "test", **binding["soffice"]},
                        "poppler_render": {"version": "test", **binding["pdftoppm"]},
                        "poppler_text": {"version": "test", **binding["pdftotext"]},
                        "poppler_metadata": {"version": "test", **binding["pdfinfo"]},
                    },
                    "routing_table_sha256": routing.sha256,
                    "canonicalizer": {"version": "1", **binding["canonicalizer"]},
                    "font_bundle": {"version": "test", **binding["fonts"]},
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
                        "receipt_schema_version": 1,
                        "executor": binding["executor"],
                    },
                    "scope": {
                        "format": document_format,
                        "contract": binding["contract"],
                        "evaluator": self._binding(root, evaluator),
                        "corpus": self._binding(root, corpus),
                        "project_revision": "b" * 40,
                    },
                    "runtime": {
                        "locale": "en-US",
                        "timezone": "UTC",
                        "rendering_dpi": 144,
                        "network_isolation": True,
                        "attestation": binding["attestation"],
                    },
                },
            )
            path = lock_dir / f"{document_format}.json"
            path.write_text(json.dumps(lock, sort_keys=True), encoding="utf-8")
            report_path = reports / f"{document_format}.json"
            report = read_object(report_path)
            report["oracle_lock_sha256"] = self._sha256(path)
            report_path.write_text(json.dumps(report, sort_keys=True), encoding="utf-8")
            result[document_format] = path
        return result
