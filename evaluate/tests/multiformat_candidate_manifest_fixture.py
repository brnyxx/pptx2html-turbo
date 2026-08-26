from __future__ import annotations

import json
import platform
import shutil
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_attestation import (
    attestation_scope_from_hashes,
)
from evaluate.multiformat_candidate_fonts import (
    prepare_font_environment,
    validate_font_bundle,
)
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.tests.multiformat_attestation_fixture import (
    TestVerifier,
    create_test_verifier,
    verifier_lock,
    write_receipt_signer,
    write_signed_attestation,
)


@dataclass(frozen=True, slots=True)
class ManifestRuntime:
    oracle_lock: Path
    artifacts: dict[str, Path]
    tools: dict[str, str]
    font_bundle_sha256: str


def prepare_manifest_runtime(
    root: Path,
    contract: Path,
    corpus: Path,
    evaluator: Path,
) -> ManifestRuntime:
    verifier = create_test_verifier(root)
    office_verifier = create_test_verifier(root, name="office-oracle")
    receipt_signer = write_receipt_signer(root, verifier)
    binaries: dict[str, Path] = {}
    for name in [
        "converter_binary",
        "soffice_binary",
        "pdftohtml_binary",
        "pdfinfo_binary",
        "chromium_binary",
    ]:
        path = root / name
        path.write_bytes(name.encode())
        binaries[name] = path
    openssl_binary = root / "openssl_binary"
    shutil.copy2(verifier.openssl, openssl_binary)
    binaries["openssl_binary"] = openssl_binary
    binaries["receipt_signer_binary"] = receipt_signer
    package_manifest = root / "runtime-package-manifest.json"
    package_manifest.write_text(
        json.dumps({"schema_version": 1, "entries": []}),
        encoding="utf-8",
    )
    binaries["runtime_package_manifest"] = package_manifest
    font_file = root / "TestFont.ttf"
    font_file.write_bytes(b"test-font")
    font_bundle = root / "font_bundle.bin"
    font_bundle.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "fonts": [
                    {
                        "path": font_file.name,
                        "sha256": sha256_file(font_file),
                    }
                ],
            },
            sort_keys=True,
        ),
        encoding="utf-8",
    )
    font_environment = validate_font_bundle(font_bundle)
    font_config = prepare_font_environment(
        font_bundle,
        root / "font-runtime",
    ).config_path
    lock = root / "oracle-lock.json"
    lock.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "status": "locked",
                "office": {
                    "os": "Windows 11",
                    "channel": "test",
                    "word": "test",
                    "excel": "test",
                    "powerpoint": "test",
                },
                "pdf": {
                    "primary": "test",
                    "secondary": "test",
                    "text": "test",
                },
                "browser": {
                    "chromium": "test-chromium",
                    "executable_sha256": sha256_file(binaries["chromium_binary"]),
                    "playwright": "1.62.0",
                    "viewport_width": 1920,
                    "viewport_height": 2400,
                    "device_scale_factor": 1,
                    "locale": "en-US",
                    "timezone": "UTC",
                    "color_profile": "srgb",
                    "reduced_motion": "reduce",
                    "animations": "disabled",
                    "os": platform.system(),
                    "architecture": platform.machine(),
                    "font_environment_sha256": font_environment,
                },
                "candidate_runtime": _candidate_runtime_lock(binaries),
                "sandbox_verifier": verifier_lock(verifier),
                "office_oracle_verifier": verifier_lock(
                    office_verifier,
                    verifier_id="test-office-oracle",
                ),
                "font_bundle_sha256": sha256_file(font_bundle),
            },
            sort_keys=True,
        ),
        encoding="utf-8",
    )
    attestation = root / "sandbox_attestation.json"
    write_signed_attestation(
        attestation,
        verifier,
        {
            "schema_version": 1,
            "status": "PASS",
            "network_isolation": "disabled",
            "golden_access": "denied",
            "project_revision": "a" * 40,
            "scope_sha256": attestation_scope_from_hashes(
                sha256_file(contract),
                sha256_file(corpus),
                sha256_file(evaluator),
                sha256_file(lock),
            ),
            "font_environment_sha256": font_environment,
            "font_isolation": "locked-bundle-only",
            "run_nonce": "e" * 64,
            "verifier_id": "test-verifier",
        },
    )
    artifacts = {
        "sandbox_attestation": attestation,
        "sandbox_public_key": verifier.public_key,
        "font_bundle": font_bundle,
        "font_config": font_config,
        **binaries,
    }
    return ManifestRuntime(
        lock,
        artifacts,
        _runtime_tools(artifacts, verifier),
        sha256_file(font_bundle),
    )


def _runtime_tools(
    artifacts: dict[str, Path],
    verifier: TestVerifier,
) -> dict[str, str]:
    return {
        **{
            f"{name}_sha256": sha256_file(artifacts[f"{name}_binary"])
            for name in [
                "converter",
                "soffice",
                "pdftohtml",
                "pdfinfo",
                "receipt_signer",
            ]
        },
        "chromium_sha256": sha256_file(artifacts["chromium_binary"]),
        "playwright": "1.62.0",
        "sandbox_attestation_sha256": sha256_file(artifacts["sandbox_attestation"]),
        "font_bundle_sha256": sha256_file(artifacts["font_bundle"]),
        "font_environment_sha256": validate_font_bundle(artifacts["font_bundle"]),
        "font_config_sha256": sha256_file(artifacts["font_config"]),
        "sandbox_public_key_sha256": sha256_file(artifacts["sandbox_public_key"]),
        "openssl_sha256": sha256_file(artifacts["openssl_binary"]),
        "runtime_package_sha256": sha256_file(artifacts["runtime_package_manifest"]),
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
        "browser_version": "test-chromium",
        "build_revision": "a" * 40,
        "sandbox_verifier_id": "test-verifier",
        "run_nonce": "e" * 64,
    }


def _candidate_runtime_lock(
    artifacts: dict[str, Path],
) -> dict[str, JsonValue]:
    return {
        "build_revision": "a" * 40,
        **{
            f"{name}_sha256": sha256_file(artifacts[f"{name}_binary"])
            for name in [
                "converter",
                "soffice",
                "pdftohtml",
                "pdfinfo",
                "receipt_signer",
            ]
        },
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
    }
