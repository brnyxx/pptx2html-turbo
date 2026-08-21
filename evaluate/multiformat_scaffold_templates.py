from __future__ import annotations

from evaluate.multiformat_schema import JsonValue


def office_lock_template() -> dict[str, JsonValue]:
    return {
        "schema_version": 1,
        "status": "INCOMPLETE",
        "office": {
            "os": "",
            "channel": "",
            "word": "",
            "excel": "",
            "powerpoint": "",
        },
        "pdf": {"primary": "", "secondary": "", "text": ""},
        "browser": {
            "chromium": "",
            "executable_sha256": "",
            "playwright": "1.62.0",
            "viewport_width": 1920,
            "viewport_height": 2400,
            "device_scale_factor": 1,
            "locale": "en-US",
            "timezone": "UTC",
            "color_profile": "srgb",
            "reduced_motion": "reduce",
            "animations": "disabled",
            "os": "",
            "architecture": "",
            "font_environment_sha256": "",
        },
        "candidate_runtime": {
            "build_revision": "",
            "converter_sha256": "",
            "converter_version": "",
            "soffice_sha256": "",
            "soffice_version": "",
            "pdftohtml_sha256": "",
            "pdftohtml_version": "",
            "pdfinfo_sha256": "",
            "pdfinfo_version": "",
            "receipt_signer_sha256": "",
            "receipt_signer_version": "",
        },
        "sandbox_verifier": _verifier_template(),
        "office_oracle_verifier": _verifier_template(),
        "font_bundle_sha256": "",
    }


def candidate_sandbox_attestation_template() -> dict[str, JsonValue]:
    return {
        "schema_version": 1,
        "status": "INCOMPLETE",
        "network_isolation": "",
        "golden_access": "",
        "project_revision": "",
        "scope_sha256": "",
        "font_environment_sha256": "",
        "font_isolation": "",
        "run_nonce": "",
        "verifier_id": "",
        "signature": "",
    }


def _verifier_template() -> dict[str, JsonValue]:
    return {
        "algorithm": "ed25519",
        "verifier_id": "",
        "public_key_sha256": "",
        "openssl_sha256": "",
    }
