from __future__ import annotations

import importlib.metadata
import json
import platform
from dataclasses import dataclass
from pathlib import Path

from playwright.sync_api import sync_playwright

from evaluate.multiformat_candidate_attestation import (
    attestation_scope_sha256,
)
from evaluate.multiformat_candidate_fonts import prepare_font_environment
from evaluate.multiformat_evaluator_files import EVALUATOR_FILES
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    read_object,
    sha256_file,
)
from evaluate.tests.multiformat_attestation_fixture import (
    create_test_verifier,
    verifier_lock,
    write_receipt_signer,
    write_signed_attestation,
)
from evaluate.tests.multiformat_candidate_fake_runtime import (
    write_converter,
    write_tool,
)
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture


@dataclass(frozen=True, slots=True)
class PipelineFixture:
    contract: Path
    corpus: Path
    evaluator: Path
    oracle_lock: Path
    evidence_root: Path
    output: Path
    converter: Path
    soffice: Path
    pdftohtml: Path
    pdfinfo: Path
    chromium: Path
    font_bundle: Path
    sandbox_attestation: Path
    sandbox_public_key: Path
    openssl: Path
    receipt_signer: Path


def prepare_pipeline_fixture(
    root: Path,
    project_root: Path,
) -> PipelineFixture:
    contract, corpus = ready_fixture(root)
    main_contract = read_object(
        project_root / "evaluate" / "multiformat" / "contract.v1.json"
    )
    contract_value = json.loads(contract.read_text(encoding="utf-8"))
    contract_value["metric_parameters"] = object_value(
        main_contract,
        "metric_parameters",
    )
    _write_json(contract, contract_value)
    corpus_value = json.loads(corpus.read_text(encoding="utf-8"))
    corpus_value["contract_sha256"] = sha256_file(contract)
    _write_json(corpus, corpus_value)
    evaluator_lock = read_object(
        project_root / "evaluate" / "multiformat" / "evaluator-lock.v1.json"
    )
    evaluator = root / "evaluator-manifest.json"
    _write_json(
        evaluator,
        {
            "schema_version": 2,
            "contract_sha256": sha256_file(contract),
            "project_revision": current_project_revision(project_root),
            "python": evaluator_lock["python"],
            "unicode_version": evaluator_lock["unicode_version"],
            "algorithm_parameters": object_value(
                contract_value,
                "metric_parameters",
            ),
            "dependencies": object_value(evaluator_lock, "dependencies"),
            "files": [
                {
                    "path": relative_path,
                    "sha256": sha256_file(project_root / relative_path),
                }
                for relative_path in EVALUATOR_FILES
            ],
        },
    )
    with sync_playwright() as playwright:
        chromium = Path(playwright.chromium.executable_path).resolve(strict=True)
        browser = playwright.chromium.launch(headless=True)
        browser_version = browser.version
        browser.close()
    evidence_root = root / "evidence"
    evidence_root.mkdir()
    font_bundle = evidence_root / "font-bundle.json"
    font_file = evidence_root / "TestFont.ttf"
    font_file.write_bytes(b"test-font")
    _write_json(
        font_bundle,
        {
            "schema_version": 1,
            "fonts": [
                {
                    "path": font_file.name,
                    "sha256": sha256_file(font_file),
                }
            ],
        },
    )
    font_environment = prepare_font_environment(
        font_bundle,
        root / "fixture-font-runtime",
    )
    converter = write_converter(root)
    soffice = write_tool(root, "soffice")
    pdftohtml = write_tool(root, "pdftohtml")
    pdfinfo = write_tool(root, "pdfinfo")
    verifier = create_test_verifier(evidence_root)
    office_verifier = create_test_verifier(
        evidence_root,
        name="office-oracle",
    )
    receipt_signer = write_receipt_signer(root, verifier)
    public_key = verifier.public_key
    openssl = verifier.openssl
    oracle_lock = evidence_root / "oracle-lock.json"
    _write_json(
        oracle_lock,
        {
            "schema_version": 1,
            "status": "locked",
            "office": {
                "os": "test",
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
                "chromium": browser_version,
                "executable_sha256": sha256_file(chromium),
                "playwright": importlib.metadata.version("playwright"),
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
                "font_environment_sha256": font_environment.environment_sha256,
            },
            "candidate_runtime": {
                "build_revision": current_project_revision(project_root),
                "converter_sha256": sha256_file(converter),
                "converter_version": "document2html test-version",
                "soffice_sha256": sha256_file(soffice),
                "soffice_version": "soffice test-version",
                "pdftohtml_sha256": sha256_file(pdftohtml),
                "pdftohtml_version": "pdftohtml test-version",
                "pdfinfo_sha256": sha256_file(pdfinfo),
                "pdfinfo_version": "pdfinfo test-version",
                "receipt_signer_sha256": sha256_file(receipt_signer),
                "receipt_signer_version": "receipt-signer test-version",
            },
            "sandbox_verifier": {
                **verifier_lock(verifier),
                "openssl_sha256": sha256_file(openssl),
            },
            "office_oracle_verifier": {
                **verifier_lock(
                    office_verifier,
                    verifier_id="test-office-oracle",
                ),
                "openssl_sha256": sha256_file(openssl),
            },
            "font_bundle_sha256": sha256_file(font_bundle),
        },
    )
    sandbox = evidence_root / "sandbox.json"
    payload: dict[str, JsonValue] = {
        "schema_version": 1,
        "status": "PASS",
        "network_isolation": "disabled",
        "golden_access": "denied",
        "project_revision": current_project_revision(project_root),
        "scope_sha256": attestation_scope_sha256(
            contract,
            corpus,
            evaluator,
            oracle_lock,
        ),
        "font_environment_sha256": font_environment.environment_sha256,
        "font_isolation": "locked-bundle-only",
        "run_nonce": "e" * 64,
        "verifier_id": "test-verifier",
    }
    write_signed_attestation(
        sandbox,
        verifier,
        payload,
    )
    return PipelineFixture(
        contract,
        corpus,
        evaluator,
        oracle_lock,
        evidence_root,
        evidence_root / "candidate",
        converter,
        soffice,
        pdftohtml,
        pdfinfo,
        chromium,
        font_bundle,
        sandbox,
        public_key,
        openssl,
        receipt_signer,
    )


def _write_json(path: Path, value: JsonValue) -> None:
    path.write_text(
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
