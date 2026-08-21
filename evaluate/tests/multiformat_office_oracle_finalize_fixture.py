from __future__ import annotations

import json
from collections import defaultdict
from pathlib import Path
from typing import TypedDict

from evaluate.multiformat_metric_links import load_metric_spec
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.tests.multiformat_attestation_fixture import (
    create_test_verifier,
    write_receipt_signer,
)
from evaluate.tests.multiformat_candidate_gate_lock_fixture import (
    write_gate_oracle_lock,
)
from evaluate.tests.multiformat_metric_artifact_fixture import write_png
from evaluate.tests.multiformat_small_corpus_fixture import ready_fixture

PROJECT_ROOT = Path(__file__).resolve().parents[2]


class FinalizerFixture(TypedDict):
    batch_manifest: Path
    contract: Path
    corpus_manifest: Path
    evaluator_manifest: Path
    oracle_lock: Path
    output_dir: Path
    receipt_signer: Path
    public_key: Path
    openssl: Path
    project_revision: str
    run_nonce: str


def write_finalizer_fixture(root: Path) -> FinalizerFixture:
    fixture_root = root / "corpus"
    fixture_root.mkdir()
    contract, corpus = ready_fixture(fixture_root)
    evaluator = root / "evaluator.json"
    evaluator.write_text('{"schema_version":1}', encoding="utf-8")
    lock = write_gate_oracle_lock(root, PROJECT_ROOT)
    lock_values = json.loads(lock.read_text(encoding="utf-8"))
    verifier = create_test_verifier(root, name="office-oracle")
    signer = write_receipt_signer(
        root,
        verifier,
        name="office-oracle",
    )
    batch_root = root / "batch"
    batch_root.mkdir()
    spec = load_metric_spec(corpus)
    by_source: defaultdict[str, list[tuple[str, str, int]]] = defaultdict(list)
    for unit_id, identity in spec.capture_identities().items():
        source_id, source_hash, ordinal = identity
        by_source[source_id].append((unit_id, source_hash, ordinal))
    files = [
        _batch_file(batch_root, source_id, identities)
        for source_id, identities in sorted(by_source.items())
    ]
    batch_manifest = batch_root / "manifest.json"
    batch_manifest.write_text(
        json.dumps(
            {
                "schema_version": 2,
                "batch_id": "office-test",
                "capture_timestamp": "2026-08-21T00:00:00Z",
                "golden_set_revision": "3" * 40,
                "font_bundle_sha256": lock_values["font_bundle_sha256"],
                "network_isolation": "disabled",
                "runtime": {
                    "windows": "Windows 11 23H2",
                    "architecture": "x86_64",
                    "office_channel": "test",
                    "word": "test-build",
                    "excel": "test-build",
                    "powerpoint": "test-build",
                    "pdf_primary": "test-mupdf",
                    "pdf_secondary": "test-renderer",
                    "pdf_text": "test-pdftotext",
                },
                "files": files,
            },
            sort_keys=True,
        ),
        encoding="utf-8",
    )
    return {
        "batch_manifest": batch_manifest,
        "contract": contract,
        "corpus_manifest": corpus,
        "evaluator_manifest": evaluator,
        "oracle_lock": lock,
        "output_dir": root / "output",
        "receipt_signer": signer,
        "public_key": verifier.public_key,
        "openssl": verifier.openssl,
        "project_revision": "3" * 40,
        "run_nonce": "4" * 64,
    }


def office_batch_binding(root: Path, path: Path) -> dict[str, str]:
    return {
        "path": path.relative_to(root).as_posix(),
        "sha256": sha256_file(path),
    }


def _batch_file(
    batch_root: Path,
    source_id: str,
    identities: list[tuple[str, str, int]],
) -> dict[str, JsonValue]:
    source_dir = batch_root / source_id
    source_dir.mkdir()
    visual_units = []
    layout_pages = []
    for _, _, ordinal in sorted(identities, key=lambda item: item[2]):
        png = source_dir / f"unit-{ordinal}.png"
        write_png(png, 192, 192, (40, 80, 120))
        visual_units.append(
            {
                "png": office_batch_binding(batch_root, png),
                "width": 192,
                "height": 192,
            }
        )
        layout_pages.append(
            '<page width="192" height="192"><flow><block>'
            f'<line xMin="10" yMin="10" xMax="100" yMax="24">'
            f'<word xMin="10" yMin="10" xMax="100" yMax="24">'
            f"Hello {ordinal}</word></line>"
            "</block></flow></page>"
        )
    semantic = source_dir / "semantic.json"
    pdf = source_dir / "reference.pdf"
    layout = source_dir / "layout.xml"
    semantic.write_text("{}", encoding="utf-8")
    pdf.write_bytes(b"%PDF-1.7\n%%EOF\n")
    layout.write_text(
        "<doc>" + "".join(layout_pages) + "</doc>",
        encoding="utf-8",
    )
    return {
        "id": source_id,
        "format": "docx",
        "source_sha256": identities[0][1],
        "pdf": office_batch_binding(batch_root, pdf),
        "semantic": office_batch_binding(batch_root, semantic),
        "layout": office_batch_binding(batch_root, layout),
        "visual_units": visual_units,
    }
