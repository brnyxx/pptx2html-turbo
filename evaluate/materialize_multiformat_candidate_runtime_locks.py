from __future__ import annotations

import importlib.metadata
import json
import platform
import shutil
import tempfile
from dataclasses import dataclass, fields
from pathlib import Path

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

from evaluate.multiformat_candidate_fonts import prepare_font_environment
from evaluate.multiformat_candidate_runtime_lock import (
    require_browser_lock,
    require_clean_worktree,
    validate_candidate_runtime,
)
from evaluate.multiformat_candidate_types import CandidateRuntimePaths
from evaluate.multiformat_portable_lock_io import (
    exclusive_write,
    tool_version,
    validate_candidate_artifacts,
    validate_candidate_locks,
)
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import JsonValue, sha256_file

PLAYWRIGHT_VERSION = "1.62.0"


class CandidateRuntimeLockMaterializeError(ValueError):
    pass


class CandidateRuntimeLockIncompleteError(CandidateRuntimeLockMaterializeError):
    pass


@dataclass(frozen=True, slots=True)
class CandidateRuntimeLockInputs:
    project_root: Path
    evidence_root: Path
    output_dir: Path
    converter: Path
    soffice: Path
    pdftohtml: Path
    pdfinfo: Path
    receipt_signer: Path
    chromium: Path
    font_bundle: Path
    sandbox_public_key: Path
    openssl: Path
    verifier_id: str


def materialize_candidate_runtime_locks(
    inputs: CandidateRuntimeLockInputs,
) -> tuple[Path, Path]:
    try:
        paths = _resolve_inputs(inputs)
    except OSError as error:
        raise CandidateRuntimeLockIncompleteError(
            "candidate runtime artifact is unavailable"
        ) from error
    root = inputs.project_root.resolve(strict=True)
    evidence = inputs.evidence_root.resolve(strict=True)
    output = inputs.output_dir.resolve(strict=False)
    if not output.is_relative_to(evidence):
        raise CandidateRuntimeLockMaterializeError(
            "candidate lock output escapes evidence root"
        )
    if output.exists():
        raise CandidateRuntimeLockMaterializeError(
            "candidate lock output already exists"
        )
    if paths["converter"].parent.name != "release":
        raise CandidateRuntimeLockMaterializeError(
            "candidate locks require a release converter"
        )
    for name in ("font_bundle", "sandbox_public_key"):
        if not paths[name].is_relative_to(evidence):
            raise CandidateRuntimeLockMaterializeError(
                f"candidate {name} escapes evidence root"
            )
    _require_public_pem(paths["sandbox_public_key"])
    if not inputs.verifier_id.strip():
        raise CandidateRuntimeLockMaterializeError("candidate verifier ID is empty")
    require_clean_worktree(root, evidence)
    revision = current_project_revision(root)
    playwright = importlib.metadata.version("playwright")
    if playwright != PLAYWRIGHT_VERSION:
        raise CandidateRuntimeLockMaterializeError(
            f"Playwright version must be {PLAYWRIGHT_VERSION}"
        )
    versions = {
        "converter": _one_line_version(paths["converter"], ("--version",)),
        "libreoffice": _one_line_version(paths["soffice"], ("--version",)),
        "pdftohtml": _one_line_version(paths["pdftohtml"], ("-v",)),
        "poppler-metadata": _one_line_version(paths["pdfinfo"], ("-v",)),
        "receipt-signer": _one_line_version(paths["receipt_signer"], ("--version",)),
        "chromium": _one_line_version(paths["chromium"], ("--version",)),
    }
    _one_line_version(paths["openssl"], ("version",))
    output.mkdir()
    try:
        with tempfile.TemporaryDirectory(
            prefix="candidate-lock-fonts-", dir=evidence
        ) as temporary:
            fonts = prepare_font_environment(paths["font_bundle"], Path(temporary))
            browser: dict[str, JsonValue] = {
                "chromium": versions["chromium"],
                "executable_sha256": sha256_file(paths["chromium"]),
                "playwright": playwright,
                "os": platform.system(),
                "architecture": platform.machine(),
                "font_environment_sha256": fonts.environment_sha256,
                "viewport_width": 1920,
                "viewport_height": 2400,
                "device_scale_factor": 1,
                "locale": "en-US",
                "timezone": "UTC",
                "color_profile": "srgb",
                "reduced_motion": "reduce",
                "animations": "disabled",
            }
            candidate_runtime: dict[str, JsonValue] = {
                "build_revision": revision,
                "converter_sha256": sha256_file(paths["converter"]),
                "converter_version": versions["converter"],
                "soffice_sha256": sha256_file(paths["soffice"]),
                "soffice_version": versions["libreoffice"],
                "pdftohtml_sha256": sha256_file(paths["pdftohtml"]),
                "pdftohtml_version": versions["pdftohtml"],
                "pdfinfo_sha256": sha256_file(paths["pdfinfo"]),
                "pdfinfo_version": versions["poppler-metadata"],
                "receipt_signer_sha256": sha256_file(paths["receipt_signer"]),
                "receipt_signer_version": versions["receipt-signer"],
            }
            runtime_lock: dict[str, JsonValue] = {
                "schema_version": 1,
                "status": "locked",
                "browser": browser,
                "candidate_runtime": candidate_runtime,
                "sandbox_verifier": {
                    "algorithm": "ed25519",
                    "verifier_id": inputs.verifier_id,
                    "public_key_sha256": sha256_file(paths["sandbox_public_key"]),
                    "openssl_sha256": sha256_file(paths["openssl"]),
                },
                "font_bundle_sha256": fonts.manifest_sha256,
            }
            browser_path = output / "browser-lock.json"
            runtime_path = output / "candidate-runtime-lock.json"
            exclusive_write(browser_path, _canonical(browser), 0o644)
            exclusive_write(runtime_path, _canonical(runtime_lock), 0o644)
            require_browser_lock(browser)
            runtime = CandidateRuntimePaths(
                paths["converter"],
                paths["soffice"],
                paths["pdftohtml"],
                paths["pdfinfo"],
                paths["chromium"],
                paths["receipt_signer"],
                fonts.config_path,
                versions["chromium"],
                15,
            )
            validate_candidate_runtime(candidate_runtime, runtime, revision)
            validate_candidate_locks(browser_path, runtime_path)
            validate_candidate_artifacts(
                runtime_path,
                {
                    "browser-lock": browser_path,
                    "chromium": paths["chromium"],
                    "converter": paths["converter"],
                    "libreoffice": paths["soffice"],
                    "pdftohtml": paths["pdftohtml"],
                    "poppler-metadata": paths["pdfinfo"],
                    "receipt-signer": paths["receipt_signer"],
                    "candidate-sandbox-public-key": paths["sandbox_public_key"],
                    "openssl": paths["openssl"],
                    "font-bundle": paths["font_bundle"],
                },
                versions,
                revision,
            )
            return browser_path, runtime_path
    except Exception:
        shutil.rmtree(output)
        raise


def _resolve_inputs(inputs: CandidateRuntimeLockInputs) -> dict[str, Path]:
    excluded = {"project_root", "evidence_root", "output_dir", "verifier_id"}
    return {
        field.name: getattr(inputs, field.name).resolve(strict=True)
        for field in fields(inputs)
        if field.name not in excluded
    }


def _one_line_version(path: Path, arguments: tuple[str, ...]) -> str:
    value = tool_version(path, arguments)
    if len(value.splitlines()) != 1:
        raise CandidateRuntimeLockMaterializeError(
            f"candidate tool version is not one line: {path}"
        )
    return value


def _require_public_pem(path: Path) -> None:
    try:
        key = serialization.load_pem_public_key(path.read_bytes())
    except (OSError, ValueError, TypeError) as error:
        raise CandidateRuntimeLockMaterializeError(
            "candidate verifier must be an Ed25519 public PEM key"
        ) from error
    if not isinstance(key, Ed25519PublicKey):
        raise CandidateRuntimeLockMaterializeError(
            "candidate verifier must be an Ed25519 public PEM key"
        )


def _canonical(value: JsonValue) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode()
