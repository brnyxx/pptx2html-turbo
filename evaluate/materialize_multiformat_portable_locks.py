from __future__ import annotations

import platform
from dataclasses import dataclass, fields
from pathlib import Path

from evaluate import multiformat_portable_lock_io as lock_io
from evaluate import multiformat_portable_package_inventory as package_io
from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_portable_lock import validate_reference_lock
from evaluate.multiformat_portable_lock_io import (
    PortableLockIncompleteError,
    PortableLockMaterializeError,
)
from evaluate.multiformat_portable_lock_keys import prepare_key_material
from evaluate.multiformat_portable_receipt_trust import load_portable_receipt_trust
from evaluate.multiformat_reference_routing import load_reference_routing
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_rust_toolchain import (
    evaluator_rust_toolchain_identity,
    rust_toolchain_value,
)
from evaluate.multiformat_schema import JsonValue, sha256_file, string_value
from evaluate.multiformat_strict_json import read_strict_object


@dataclass(frozen=True, slots=True)
class PortableLockInputs:
    project_root: Path
    evidence_root: Path
    output_dir: Path
    contract: Path
    evaluator: Path
    corpora: tuple[Path, ...]
    cargo: Path
    rustc: Path
    libreoffice: Path
    pdftoppm: Path
    pdftotext: Path
    pdfinfo: Path
    canonicalizer: Path
    font_bundle: Path
    configuration: Path
    chromium: Path
    executor: Path
    sandbox_exec: Path
    browser_lock: Path
    candidate_runtime_lock: Path
    converter: Path
    pdftohtml: Path
    openssl: Path
    receipt_signer: Path
    candidate_sandbox_public_key: Path
    private_key: Path
    generate_keys: bool = False


def materialize_portable_locks(inputs: PortableLockInputs) -> tuple[Path, ...]:
    skip = {"project_root", "evidence_root", "output_dir", "private_key"}
    try:
        for field in fields(inputs):
            value = getattr(inputs, field.name)
            artifacts = value if isinstance(value, tuple) else (value,)
            if field.name not in skip:
                for artifact in artifacts:
                    if isinstance(artifact, Path):
                        artifact.resolve(strict=True)
    except OSError as error:
        raise PortableLockIncompleteError(
            "portable runtime artifact is unavailable"
        ) from error
    rust_toolchain = evaluator_rust_toolchain_identity(inputs.cargo, inputs.rustc)
    root, output, public_key = prepare_key_material(
        inputs.project_root,
        inputs.evidence_root,
        inputs.output_dir,
        inputs.private_key,
        inputs.generate_keys,
    )
    artifacts_dir = output / "artifacts"
    artifacts_dir.mkdir(parents=True)
    artifact_inputs = {
        "poppler-render": inputs.pdftoppm,
        "poppler-text": inputs.pdftotext,
        "poppler-metadata": inputs.pdfinfo,
        "canonicalizer": inputs.canonicalizer,
        "configuration": inputs.configuration,
        "browser-lock": inputs.browser_lock,
        "candidate-runtime-lock": inputs.candidate_runtime_lock,
        "converter": inputs.converter,
        "pdftohtml": inputs.pdftohtml,
        "openssl": inputs.openssl,
        "receipt-signer": inputs.receipt_signer,
        "candidate-sandbox-public-key": inputs.candidate_sandbox_public_key,
        "executor": inputs.executor,
        "sandbox-exec": inputs.sandbox_exec,
        "contract": inputs.contract,
        "evaluator": inputs.evaluator,
    }
    paths = {
        name: lock_io.bind_file(path, root, artifacts_dir / name)
        for name, path in artifact_inputs.items()
    }
    paths["libreoffice"], libreoffice_inventory = (
        package_io.bind_package_executable_with_inventory(
            inputs.libreoffice, root, artifacts_dir / "libreoffice-package"
        )
    )
    paths["chromium"], chromium_inventory = (
        package_io.bind_package_executable_with_inventory(
            inputs.chromium, root, artifacts_dir / "chromium-package"
        )
    )
    paths["font-bundle"] = lock_io.bind_font_bundle(
        inputs.font_bundle, root, artifacts_dir / "font-bundle"
    )
    if sha256_file(paths["candidate-sandbox-public-key"]) == sha256_file(public_key):
        raise PortableLockMaterializeError(
            "portable candidate verifier key must be distinct"
        )
    versions = {
        "libreoffice": lock_io.tool_version(paths["libreoffice"], ("--version",)),
        "poppler-render": lock_io.tool_version(paths["poppler-render"], ("-v",)),
        "poppler-text": lock_io.tool_version(paths["poppler-text"], ("-v",)),
        "poppler-metadata": lock_io.tool_version(paths["poppler-metadata"], ("-v",)),
        "chromium": lock_io.tool_version(paths["chromium"], ("--version",)),
        "converter": lock_io.tool_version(paths["converter"], ("--version",)),
        "pdftohtml": lock_io.tool_version(paths["pdftohtml"], ("-v",)),
        "openssl": lock_io.tool_version(paths["openssl"], ("version",)),
        "receipt-signer": lock_io.tool_version(paths["receipt-signer"], ("--version",)),
    }
    revision = current_project_revision(inputs.project_root)
    generated = output / "generated"
    generated.mkdir()
    browser_lock = paths["browser-lock"]
    candidate_runtime = paths["candidate-runtime-lock"]
    lock_io.validate_candidate_locks(browser_lock, candidate_runtime)
    lock_io.validate_candidate_artifacts(candidate_runtime, paths, versions, revision)
    sandbox_profile = generated / "portable-reference.sb"
    lock_io.write_sandbox_profile(sandbox_profile)
    sandbox_wrapper = generated / "sandbox-exec"
    lock_io.write_sandbox_wrapper(sandbox_wrapper, inputs.sandbox_exec)
    attestation = generated / "attestation.json"
    write_canonical_json(
        attestation,
        {
            "schema_version": 1,
            "os": platform.system(),
            "architecture": platform.machine(),
            "locale": "en-US",
            "timezone": "UTC",
            "rendering_dpi": 144,
            "network_isolation": True,
            "sandbox_executable": lock_io.binding(root, sandbox_wrapper),
            "sandbox_host_artifact": lock_io.binding(root, paths["sandbox-exec"]),
            "sandbox_profile": lock_io.binding(root, sandbox_profile),
        },
    )
    routing = load_reference_routing(
        Path(__file__).parent / "multiformat/reference-routing.v1.json"
    )
    lock_dir = output / "locks"
    lock_dir.mkdir()
    results: list[Path] = []
    seen: set[str] = set()
    for corpus_source in inputs.corpora:
        corpus = lock_io.bind_corpus(corpus_source, root, output / "corpora")
        document_format = string_value(read_strict_object(corpus), "format")
        if document_format in seen:
            raise PortableLockMaterializeError("portable corpus format is duplicated")
        seen.add(document_format)
        lock = lock_dir / f"{document_format}.json"
        libreoffice_binding = package_io.package_binding(
            root, paths["libreoffice"], versions["libreoffice"], libreoffice_inventory
        )
        chromium_binding = package_io.package_binding(
            root, paths["chromium"], versions["chromium"], chromium_inventory
        )
        value: dict[str, JsonValue] = {
            "schema_version": 2,
            "status": "locked",
            "reference_profile": "libreoffice-poppler",
            "platform": {"os": platform.system(), "architecture": platform.machine()},
            "rust_toolchain": rust_toolchain_value(rust_toolchain),
            "tools": {
                "libreoffice": libreoffice_binding,
                "poppler_render": lock_io.versioned(
                    root, paths["poppler-render"], versions["poppler-render"]
                ),
                "poppler_text": lock_io.versioned(
                    root, paths["poppler-text"], versions["poppler-text"]
                ),
                "poppler_metadata": lock_io.versioned(
                    root, paths["poppler-metadata"], versions["poppler-metadata"]
                ),
            },
            "routing_table_sha256": routing.sha256,
            "canonicalizer": lock_io.versioned(
                root, paths["canonicalizer"], routing.canonicalizer_version
            ),
            "font_bundle": lock_io.versioned(
                root, paths["font-bundle"], sha256_file(paths["font-bundle"])[:16]
            ),
            "configuration": lock_io.versioned(
                root, paths["configuration"], sha256_file(paths["configuration"])[:16]
            ),
            "browser": {
                "chromium": chromium_binding,
                "lock": lock_io.binding(root, browser_lock),
            },
            "candidate_runtime_lock": lock_io.binding(root, candidate_runtime),
            "candidate_sandbox": {
                "public_key": lock_io.binding(
                    root, paths["candidate-sandbox-public-key"]
                ),
                "openssl": lock_io.binding(root, paths["openssl"]),
                "receipt_signer": lock_io.binding(root, paths["receipt-signer"]),
            },
            "sandbox": {
                "executable": lock_io.binding(root, sandbox_wrapper),
                "profile": lock_io.binding(root, sandbox_profile),
            },
            "signer": {
                "algorithm": "ed25519",
                "signer_id": "multiformat-portable-reference-v1",
                "public_key": lock_io.binding(root, public_key),
                "receipt_schema_version": 2,
                "executor": lock_io.binding(root, paths["executor"]),
            },
            "scope": {
                "format": document_format,
                "contract": lock_io.binding(root, paths["contract"]),
                "evaluator": lock_io.binding(root, paths["evaluator"]),
                "corpus": lock_io.binding(root, corpus),
                "project_revision": revision,
            },
            "runtime": {
                "locale": "en-US",
                "timezone": "UTC",
                "rendering_dpi": 144,
                "network_isolation": True,
                "attestation": lock_io.binding(root, attestation),
            },
        }
        write_canonical_json(lock, value)
        validate_reference_lock(lock, root)
        load_portable_receipt_trust(lock, root)
        results.append(lock)
    return tuple(results)
