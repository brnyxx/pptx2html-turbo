from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_evidence_path import EvidencePathError, resolve_evidence_path
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


def bound_evidence_path(
    report: dict[str, JsonValue],
    field: str,
    evidence_root: Path,
    failures: list[str],
) -> Path | None:
    try:
        binding = object_value(report, field)
        relative_path = string_value(binding, "path")
        expected_hash = sha256_value(binding, "sha256")
        evidence_path = resolve_evidence_path(evidence_root, relative_path)
        if sha256_file(evidence_path) != expected_hash:
            failures.append(field)
            return None
        return evidence_path
    except (OSError, TypeError, ValueError, EvidencePathError):
        failures.append(field)
        return None


def oracle_lock_ready(path: Path, evidence_root: Path | None = None) -> bool:
    try:
        if not path.is_file():
            return False
        lock = read_strict_object(path)
        if lock.get("schema_version") == 2:
            if evidence_root is None:
                return False
            from evaluate.multiformat_portable_lock import (
                PortableLockError,
                validate_reference_lock,
            )

            try:
                validate_reference_lock(path, evidence_root)
            except PortableLockError:
                return False
            return True
        if lock.get("schema_version") != 1 or lock.get("status") != "locked":
            return False
        office = object_value(lock, "office")
        pdf = object_value(lock, "pdf")
        browser = object_value(lock, "browser")
        candidate_runtime = object_value(lock, "candidate_runtime")
        sandbox_verifier = object_value(lock, "sandbox_verifier")
        office_oracle_verifier = object_value(lock, "office_oracle_verifier")
        for field in ["os", "channel", "word", "excel", "powerpoint"]:
            string_value(office, field)
        for field in ["primary", "secondary", "text"]:
            string_value(pdf, field)
        if not _browser_lock_ready(browser):
            return False
        if not _candidate_runtime_ready(candidate_runtime):
            return False
        if not _sandbox_verifier_ready(sandbox_verifier):
            return False
        if not _sandbox_verifier_ready(office_oracle_verifier):
            return False
        if sha256_value(
            sandbox_verifier,
            "public_key_sha256",
        ) == sha256_value(
            office_oracle_verifier,
            "public_key_sha256",
        ):
            return False
        font_hash = string_value(lock, "font_bundle_sha256")
        return len(font_hash) == 64 and all(
            character in "0123456789abcdef" for character in font_hash
        )
    except (OSError, UnicodeError, ValueError, TypeError):
        return False


def _browser_lock_ready(browser: dict[str, JsonValue]) -> bool:
    string_value(browser, "chromium")
    sha256_value(browser, "executable_sha256")
    return all(
        [
            string_value(browser, "playwright") == "1.62.0",
            integer_value(browser, "viewport_width") == 1920,
            integer_value(browser, "viewport_height") == 2400,
            integer_value(browser, "device_scale_factor") == 1,
            string_value(browser, "locale") == "en-US",
            string_value(browser, "timezone") == "UTC",
            string_value(browser, "color_profile") == "srgb",
            string_value(browser, "reduced_motion") == "reduce",
            string_value(browser, "animations") == "disabled",
            bool(string_value(browser, "os")),
            bool(string_value(browser, "architecture")),
            bool(sha256_value(browser, "font_environment_sha256")),
        ]
    )


def _candidate_runtime_ready(values: dict[str, JsonValue]) -> bool:
    revision = string_value(values, "build_revision")
    if len(revision) != 40 or any(
        character not in "0123456789abcdef" for character in revision
    ):
        return False
    for name in [
        "converter",
        "soffice",
        "pdftohtml",
        "pdfinfo",
        "receipt_signer",
    ]:
        sha256_value(values, f"{name}_sha256")
        string_value(values, f"{name}_version")
    return True


def _sandbox_verifier_ready(values: dict[str, JsonValue]) -> bool:
    return all(
        [
            string_value(values, "algorithm") == "ed25519",
            bool(string_value(values, "verifier_id")),
            bool(sha256_value(values, "public_key_sha256")),
            bool(sha256_value(values, "openssl_sha256")),
        ]
    )
