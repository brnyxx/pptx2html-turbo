from __future__ import annotations

import json
import subprocess
from pathlib import Path

from evaluate.materialize_multiformat_portable_locks import PortableLockInputs
from evaluate.multiformat_portable_package_inventory import write_package_inventory
from evaluate.multiformat_portable_reference_artifacts import load_raw_private_key
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import sha256_file
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture

_PROJECT = Path(__file__).resolve().parents[2]
_CARGO = Path(
    subprocess.run(
        ["rustup", "which", "cargo"], check=True, capture_output=True, text=True
    ).stdout.strip()
)
_RUSTC = Path(
    subprocess.run(
        ["rustup", "which", "rustc"], check=True, capture_output=True, text=True
    ).stdout.strip()
)


def portable_lock_inputs(root: Path) -> PortableLockInputs:
    """Create deterministic materializer inputs with inventoried native packages."""
    root.mkdir(parents=True, exist_ok=True)
    contract, corpus = ready_fixture(root)
    evaluator = root / "evaluator.json"
    evaluator.write_text("{}")
    tools = root / "tools"
    tools.mkdir()
    names = (
        "soffice",
        "pdftoppm",
        "pdftotext",
        "pdfinfo",
        "chromium",
        "converter",
        "pdftohtml",
        "openssl",
        "receipt-signer",
    )
    paths: dict[str, Path] = {}
    for name in names:
        path = tools / name
        path.write_text(f"#!/bin/sh\necho '{name} 1.0'\n")
        path.chmod(0o755)
        paths[name] = path
    for package_name, members in (
        ("poppler", ("pdftoppm", "pdftotext", "pdfinfo", "pdftohtml")),
        ("openssl", ("openssl",)),
    ):
        package = tools / f"{package_name}-package/root"
        package_bin = package / "bin"
        package_bin.mkdir(parents=True)
        for name in members:
            destination = package_bin / name
            paths[name].replace(destination)
            paths[name] = destination
        write_package_inventory(package.parent / "inventory.json", package, root)
    plain: dict[str, Path] = {}
    for name in (
        "canonicalizer",
        "fonts",
        "configuration",
        "executor",
        "candidate-public-key",
    ):
        path = tools / name
        path.write_bytes(name.encode())
        plain[name] = path
    browser_lock = root / "browser-lock.json"
    browser_lock.write_text(
        json.dumps(
            {
                "chromium": "chromium 1.0",
                "executable_sha256": sha256_file(paths["chromium"]),
                "playwright": "1.62.0",
                "os": "Darwin",
                "architecture": "arm64",
                "font_environment_sha256": "a" * 64,
                "viewport_width": 1920,
                "viewport_height": 2400,
                "device_scale_factor": 1,
                "locale": "en-US",
                "timezone": "UTC",
                "color_profile": "srgb",
                "reduced_motion": "reduce",
                "animations": "disabled",
            },
            sort_keys=True,
        )
    )
    candidate_lock = root / "candidate-runtime-lock.json"
    candidate_lock.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "status": "locked",
                "browser": json.loads(browser_lock.read_text()),
                "candidate_runtime": {
                    "build_revision": current_project_revision(_PROJECT),
                    "converter_sha256": sha256_file(paths["converter"]),
                    "converter_version": "converter 1.0",
                    "soffice_sha256": sha256_file(paths["soffice"]),
                    "soffice_version": "soffice 1.0",
                    "pdftohtml_sha256": sha256_file(paths["pdftohtml"]),
                    "pdftohtml_version": "pdftohtml 1.0",
                    "pdfinfo_sha256": sha256_file(paths["pdfinfo"]),
                    "pdfinfo_version": "pdfinfo 1.0",
                    "receipt_signer_sha256": sha256_file(paths["receipt-signer"]),
                    "receipt_signer_version": "receipt-signer 1.0",
                },
                "sandbox_verifier": {
                    "algorithm": "ed25519",
                    "verifier_id": "candidate-sandbox-v1",
                    "public_key_sha256": sha256_file(plain["candidate-public-key"]),
                    "openssl_sha256": sha256_file(paths["openssl"]),
                },
                "font_bundle_sha256": sha256_file(plain["fonts"]),
            },
            sort_keys=True,
        )
    )
    key = root.parent / f"{root.name}.private.raw"
    key.write_bytes(b"1" * 32)
    key.chmod(0o600)
    load_raw_private_key(key)
    return PortableLockInputs(
        _PROJECT,
        root,
        root / "out",
        contract,
        evaluator,
        (corpus,),
        _CARGO,
        _RUSTC,
        paths["soffice"],
        paths["pdftoppm"],
        paths["pdftotext"],
        paths["pdfinfo"],
        plain["canonicalizer"],
        plain["fonts"],
        plain["configuration"],
        paths["chromium"],
        plain["executor"],
        Path("/usr/bin/sandbox-exec"),
        browser_lock,
        candidate_lock,
        paths["converter"],
        paths["pdftohtml"],
        paths["openssl"],
        paths["receipt-signer"],
        plain["candidate-public-key"],
        key,
    )
