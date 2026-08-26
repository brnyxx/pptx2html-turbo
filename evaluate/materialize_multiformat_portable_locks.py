from __future__ import annotations

import platform
import shlex
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_portable_lock import validate_reference_lock
from evaluate.multiformat_portable_lock_io import (
    bind_corpus as _bind_corpus,
    bind_file as _bind_file,
    binding as _binding,
    exclusive_write as _exclusive_write,
    tool_version as _version,
    versioned as _versioned,
)
from evaluate.multiformat_portable_receipt_trust import load_portable_receipt_trust
from evaluate.multiformat_portable_reference_artifacts import (
    load_raw_private_key,
    write_raw_keypair,
)
from evaluate.multiformat_reference_routing import load_reference_routing
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import JsonValue, sha256_file, string_value
from evaluate.multiformat_strict_json import read_strict_object

ROUTING = Path(__file__).parent / "multiformat/reference-routing.v1.json"


class PortableLockMaterializeError(ValueError):
    pass


@dataclass(frozen=True, slots=True)
class PortableLockInputs:
    project_root: Path
    evidence_root: Path
    output_dir: Path
    contract: Path
    evaluator: Path
    corpora: tuple[Path, ...]
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
    private_key: Path
    generate_keys: bool = False


def materialize_portable_locks(inputs: PortableLockInputs) -> tuple[Path, ...]:
    """Create and immediately validate corpus-scoped schema-2 portable locks."""
    root = inputs.evidence_root.resolve(strict=True)
    output = inputs.output_dir.parent.resolve(strict=True) / inputs.output_dir.name
    if not output.is_relative_to(root):
        raise PortableLockMaterializeError(
            "portable lock output must be inside evidence root"
        )
    if output.exists():
        raise PortableLockMaterializeError("portable lock output already exists")
    private_destination = inputs.private_key.resolve(strict=False)
    if private_destination.is_relative_to(
        inputs.project_root.resolve(strict=True)
    ) or private_destination.is_relative_to(root):
        raise PortableLockMaterializeError(
            "portable private key must remain outside the project and evidence root"
        )
    if inputs.generate_keys:
        public_source = output / "keys/public.raw"
        write_raw_keypair(inputs.private_key, public_source)
        private = load_raw_private_key(inputs.private_key)
    else:
        private = load_raw_private_key(inputs.private_key)
        public_source = output / "keys/public.raw"
        public_source.parent.mkdir(parents=True)
        _exclusive_write(public_source, private.public_key().public_bytes_raw(), 0o644)
    artifacts_dir = output / "artifacts"
    artifacts_dir.mkdir(parents=True)
    artifact_inputs = {
        "libreoffice": inputs.libreoffice,
        "poppler-render": inputs.pdftoppm,
        "poppler-text": inputs.pdftotext,
        "poppler-metadata": inputs.pdfinfo,
        "canonicalizer": inputs.canonicalizer,
        "font-bundle": inputs.font_bundle,
        "configuration": inputs.configuration,
        "chromium": inputs.chromium,
        "executor": inputs.executor,
        "sandbox-exec": inputs.sandbox_exec,
        "contract": inputs.contract,
        "evaluator": inputs.evaluator,
    }
    paths = {
        name: _bind_file(path, root, artifacts_dir / name)
        for name, path in artifact_inputs.items()
    }
    public_key = public_source.resolve(strict=True)
    versions = {
        "libreoffice": _version(paths["libreoffice"]),
        "poppler-render": _version(paths["poppler-render"]),
        "poppler-text": _version(paths["poppler-text"]),
        "poppler-metadata": _version(paths["poppler-metadata"]),
        "chromium": _version(paths["chromium"]),
    }
    generated = output / "generated"
    generated.mkdir()
    browser_lock = generated / "browser.json"
    write_canonical_json(
        browser_lock,
        {
            "schema_version": 1,
            "version": versions["chromium"],
            "sha256": sha256_file(paths["chromium"]),
        },
    )
    candidate_runtime = generated / "candidate-runtime.json"
    write_canonical_json(
        candidate_runtime,
        {
            "schema_version": 1,
            "libreoffice_sha256": sha256_file(paths["libreoffice"]),
            "poppler": {
                "render": sha256_file(paths["poppler-render"]),
                "text": sha256_file(paths["poppler-text"]),
                "metadata": sha256_file(paths["poppler-metadata"]),
            },
        },
    )
    sandbox_profile = generated / "portable-reference.sb"
    sandbox_profile.write_text(
        "(version 1)\n(allow default)\n(deny network*)\n", encoding="utf-8"
    )
    sandbox_wrapper = generated / "sandbox-exec"
    wrapper = (
        "#!/bin/sh\nexec "
        + shlex.quote(inputs.sandbox_exec.resolve(strict=True).as_posix())
        + ' "$@"\n'
    ).encode()
    _exclusive_write(sandbox_wrapper, wrapper, 0o755)
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
            "sandbox_executable": _binding(root, sandbox_wrapper),
            "sandbox_host_artifact": _binding(root, paths["sandbox-exec"]),
            "sandbox_profile": _binding(root, sandbox_profile),
        },
    )
    revision = current_project_revision(inputs.project_root)
    routing = load_reference_routing(ROUTING)
    lock_dir = output / "locks"
    lock_dir.mkdir()
    results: list[Path] = []
    seen: set[str] = set()
    for corpus_source in inputs.corpora:
        corpus = _bind_corpus(corpus_source, root, output / "corpora")
        document_format = string_value(read_strict_object(corpus), "format")
        if document_format in seen:
            raise PortableLockMaterializeError("portable corpus format is duplicated")
        seen.add(document_format)
        lock = lock_dir / f"{document_format}.json"
        value: dict[str, JsonValue] = {
            "schema_version": 2,
            "status": "locked",
            "reference_profile": "libreoffice-poppler",
            "platform": {"os": platform.system(), "architecture": platform.machine()},
            "tools": {
                "libreoffice": _versioned(
                    root, paths["libreoffice"], versions["libreoffice"]
                ),
                "poppler_render": _versioned(
                    root, paths["poppler-render"], versions["poppler-render"]
                ),
                "poppler_text": _versioned(
                    root, paths["poppler-text"], versions["poppler-text"]
                ),
                "poppler_metadata": _versioned(
                    root, paths["poppler-metadata"], versions["poppler-metadata"]
                ),
            },
            "routing_table_sha256": routing.sha256,
            "canonicalizer": _versioned(
                root, paths["canonicalizer"], routing.canonicalizer_version
            ),
            "font_bundle": _versioned(
                root, paths["font-bundle"], sha256_file(paths["font-bundle"])[:16]
            ),
            "configuration": _versioned(
                root, paths["configuration"], sha256_file(paths["configuration"])[:16]
            ),
            "browser": {
                "chromium": _versioned(root, paths["chromium"], versions["chromium"]),
                "lock": _binding(root, browser_lock),
            },
            "candidate_runtime_lock": _binding(root, candidate_runtime),
            "sandbox": {
                "executable": _binding(root, sandbox_wrapper),
                "profile": _binding(root, sandbox_profile),
            },
            "signer": {
                "algorithm": "ed25519",
                "signer_id": "multiformat-portable-reference-v1",
                "public_key": _binding(root, public_key),
                "receipt_schema_version": 1,
                "executor": _binding(root, paths["executor"]),
            },
            "scope": {
                "contract": _binding(root, paths["contract"]),
                "evaluator": _binding(root, paths["evaluator"]),
                "corpus": _binding(root, corpus),
                "project_revision": revision,
            },
            "runtime": {
                "locale": "en-US",
                "timezone": "UTC",
                "rendering_dpi": 144,
                "network_isolation": True,
                "attestation": _binding(root, attestation),
            },
        }
        write_canonical_json(lock, value)
        validate_reference_lock(lock, root)
        load_portable_receipt_trust(lock, root)
        results.append(lock)
    return tuple(results)
