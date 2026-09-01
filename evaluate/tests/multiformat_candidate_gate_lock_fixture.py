from __future__ import annotations

import json
import shutil
import subprocess
from pathlib import Path

from evaluate.multiformat_candidate_fonts import prepare_font_environment
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.tests.multiformat_attestation_fixture import (
    create_test_verifier,
    verifier_lock,
    write_receipt_signer,
)


def rust_toolchain_lock_value() -> dict[str, JsonValue]:
    cargo = Path(
        subprocess.run(
            ["rustup", "which", "cargo"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    ).resolve(strict=True)
    rustc = Path(
        subprocess.run(
            ["rustup", "which", "rustc"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
    ).resolve(strict=True)
    return {
        "cargo": {"path": cargo.as_posix(), "sha256": sha256_file(cargo)},
        "rustc": {"path": rustc.as_posix(), "sha256": sha256_file(rustc)},
    }


def write_gate_oracle_lock(root: Path, project_root: Path) -> Path:
    font_file = root / "TestFont.ttf"
    font_file.write_bytes(b"test-font")
    font_bundle = root / "test-font_bundle.bin"
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
    font_environment = prepare_font_environment(
        font_bundle,
        root / "test-font-runtime",
    ).environment_sha256
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
        path = root / f"test-{name}.bin"
        path.write_bytes(name.encode())
        binaries[name] = path
    openssl_binary = root / "test-openssl_binary.bin"
    shutil.copy2(verifier.openssl, openssl_binary)
    binaries["openssl_binary"] = openssl_binary
    binaries["receipt_signer_binary"] = receipt_signer
    lock = root / "oracle-lock.json"
    lock.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "status": "locked",
                "rust_toolchain": rust_toolchain_lock_value(),
                "office": {
                    "os": "Windows 11 23H2",
                    "channel": "test",
                    "word": "test-build",
                    "excel": "test-build",
                    "powerpoint": "test-build",
                },
                "pdf": {
                    "primary": "test-mupdf",
                    "secondary": "test-renderer",
                    "text": "test-pdftotext",
                },
                "browser": {
                    "chromium": "test-revision",
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
                    "os": "test-os",
                    "architecture": "test-architecture",
                    "font_environment_sha256": font_environment,
                },
                "candidate_runtime": {
                    "build_revision": current_project_revision(project_root),
                    **{
                        f"{name}_sha256": sha256_file(binaries[f"{name}_binary"])
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
                },
                "sandbox_verifier": {
                    **verifier_lock(verifier),
                    "openssl_sha256": sha256_file(binaries["openssl_binary"]),
                },
                "office_oracle_verifier": {
                    **verifier_lock(
                        office_verifier,
                        verifier_id="test-office-oracle",
                    ),
                    "openssl_sha256": sha256_file(binaries["openssl_binary"]),
                },
                "font_bundle_sha256": sha256_file(font_bundle),
            },
            sort_keys=True,
        ),
        encoding="utf-8",
    )
    return lock
