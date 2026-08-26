from __future__ import annotations

import json
import subprocess
from pathlib import Path

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.materialize_multiformat_candidate_runtime_locks import (
    CandidateRuntimeLockInputs,
)
from evaluate.multiformat_schema import sha256_file


def candidate_runtime_lock_inputs(root: Path) -> CandidateRuntimeLockInputs:
    """Create deterministic candidate runtime lock materializer inputs."""
    project = root / "project"
    project.mkdir(parents=True)
    subprocess.run(["git", "init", "-q"], cwd=project, check=True)
    subprocess.run(
        ["git", "config", "user.email", "test@example.invalid"],
        cwd=project,
        check=True,
    )
    subprocess.run(["git", "config", "user.name", "Test"], cwd=project, check=True)
    (project / "tracked").write_text("tracked")
    subprocess.run(["git", "add", "tracked"], cwd=project, check=True)
    subprocess.run(["git", "commit", "-qm", "fixture"], cwd=project, check=True)
    evidence = project / "evidence"
    evidence.mkdir()
    release = project / "target/release"
    release.mkdir(parents=True)
    tools = {
        "converter": _tool(release / "document2html", "converter 1.0"),
        "soffice": _tool(root / "soffice", "soffice 1.0"),
        "pdftohtml": _tool(root / "pdftohtml", "pdftohtml 1.0"),
        "pdfinfo": _tool(root / "pdfinfo", "pdfinfo 1.0"),
        "receipt": _tool(root / "receipt-signer", "receipt-signer 1.0"),
        "chromium": _tool(root / "chromium", "Chromium 1.0"),
        "openssl": _tool(root / "openssl", "OpenSSL 1.0"),
    }
    subprocess.run(
        ["git", "add", "target/release/document2html"], cwd=project, check=True
    )
    subprocess.run(["git", "commit", "-qm", "release fixture"], cwd=project, check=True)
    font = evidence / "font.ttf"
    font.write_bytes(b"font")
    manifest = evidence / "font-bundle.json"
    manifest.write_text(
        json.dumps(
            {
                "schema_version": 1,
                "fonts": [{"path": "font.ttf", "sha256": sha256_file(font)}],
            }
        )
    )
    key = evidence / "candidate-public.pem"
    key.write_bytes(
        Ed25519PrivateKey.generate()
        .public_key()
        .public_bytes(
            serialization.Encoding.PEM,
            serialization.PublicFormat.SubjectPublicKeyInfo,
        )
    )
    return CandidateRuntimeLockInputs(
        project,
        evidence,
        evidence / "locks",
        tools["converter"],
        tools["soffice"],
        tools["pdftohtml"],
        tools["pdfinfo"],
        tools["receipt"],
        tools["chromium"],
        manifest,
        key,
        tools["openssl"],
        "candidate-sandbox-v1",
    )


def _tool(path: Path, version: str) -> Path:
    path.write_text(f"#!/bin/sh\necho '{version}'\n")
    path.chmod(0o755)
    return path
