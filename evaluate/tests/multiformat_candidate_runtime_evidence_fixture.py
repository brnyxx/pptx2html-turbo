from __future__ import annotations

import hashlib
import json
import shutil
from pathlib import Path

from evaluate.multiformat_candidate_attestation import (
    attestation_scope_from_hashes,
)
from evaluate.multiformat_candidate_fonts import prepare_font_environment
from evaluate.multiformat_schema import JsonValue
from evaluate.tests.multiformat_attestation_fixture import (
    create_test_verifier,
    write_receipt_signer,
    write_signed_attestation,
)
from evaluate.tests.multiformat_metric_artifact_fixture import binding, sha256


def runtime_evidence(
    root: Path,
    document_format: str,
    role: str,
    project_revision: str,
    contract_hash: str,
    corpus_hash: str,
    evaluator_hash: str,
    oracle_hash: str,
) -> tuple[dict[str, JsonValue], dict[str, JsonValue]]:
    if role != "candidate":
        return {}, {}
    verifier = create_test_verifier(root)
    receipt_signer = write_receipt_signer(root, verifier)
    font_bundle = _font_bundle(root)
    prepared_fonts = prepare_font_environment(
        font_bundle,
        root / "test-font-runtime",
    )
    run_nonce = hashlib.sha256(f"{document_format}:{oracle_hash}".encode()).hexdigest()
    attestation = root / f"{document_format}-candidate-attestation.json"
    write_signed_attestation(
        attestation,
        verifier,
        {
            "schema_version": 1,
            "status": "PASS",
            "network_isolation": "disabled",
            "golden_access": "denied",
            "project_revision": project_revision,
            "scope_sha256": attestation_scope_from_hashes(
                contract_hash,
                corpus_hash,
                evaluator_hash,
                oracle_hash,
            ),
            "font_environment_sha256": prepared_fonts.environment_sha256,
            "font_isolation": "locked-bundle-only",
            "run_nonce": run_nonce,
            "verifier_id": "test-verifier",
        },
    )
    artifact_paths = _runtime_artifact_paths(
        root,
        verifier.openssl,
        receipt_signer,
        attestation,
        font_bundle,
        prepared_fonts.config_path,
    )
    tools: dict[str, JsonValue] = {
        **{
            f"{name}_sha256": sha256(artifact_paths[f"{name}_binary"])
            for name in [
                "converter",
                "soffice",
                "pdftohtml",
                "pdfinfo",
                "receipt_signer",
            ]
        },
        "chromium_sha256": sha256(artifact_paths["chromium_binary"]),
        "sandbox_attestation_sha256": sha256(attestation),
        "font_bundle_sha256": sha256(font_bundle),
        "font_environment_sha256": prepared_fonts.environment_sha256,
        "font_config_sha256": sha256(prepared_fonts.config_path),
        "sandbox_public_key_sha256": sha256(verifier.public_key),
        "openssl_sha256": sha256(artifact_paths["openssl_binary"]),
        "runtime_package_sha256": sha256(artifact_paths["runtime_package_manifest"]),
        "playwright": "1.62.0",
        "browser_version": "test-revision",
        **{
            f"{name}_version": f"{name}-test"
            for name in [
                "converter",
                "soffice",
                "pdftohtml",
                "pdfinfo",
                "receipt_signer",
            ]
        },
        "build_revision": project_revision,
        "sandbox_verifier_id": "test-verifier",
        "run_nonce": run_nonce,
    }
    return tools, {name: binding(root, path) for name, path in artifact_paths.items()}


def _font_bundle(root: Path) -> Path:
    font_bundle = root / "test-font_bundle.bin"
    if font_bundle.exists():
        return font_bundle
    font_file = root / "TestFont.ttf"
    font_file.write_bytes(b"test-font")
    font_bundle.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "fonts": [
                    {
                        "path": font_file.name,
                        "sha256": sha256(font_file),
                    }
                ],
            },
            sort_keys=True,
        ),
        encoding="utf-8",
    )
    return font_bundle


def _runtime_artifact_paths(
    root: Path,
    openssl: Path,
    receipt_signer: Path,
    attestation: Path,
    font_bundle: Path,
    font_config: Path,
) -> dict[str, Path]:
    paths = {
        "sandbox_attestation": attestation,
        "sandbox_public_key": root / "test-sandbox-public.pem",
        "font_bundle": font_bundle,
        "font_config": font_config,
        "receipt_signer_binary": receipt_signer,
    }
    for name in [
        "converter_binary",
        "soffice_binary",
        "pdftohtml_binary",
        "pdfinfo_binary",
        "chromium_binary",
    ]:
        path = root / f"test-{name}.bin"
        if not path.exists():
            path.write_bytes(name.encode())
        paths[name] = path
    openssl_binary = root / "test-openssl_binary.bin"
    if not openssl_binary.exists():
        shutil.copy2(openssl, openssl_binary)
    paths["openssl_binary"] = openssl_binary
    package_manifest = root / "test-runtime-package-manifest.json"
    if not package_manifest.exists():
        package_manifest.write_text(
            json.dumps({"schema_version": 1, "entries": []}),
            encoding="utf-8",
        )
    paths["runtime_package_manifest"] = package_manifest
    return paths
