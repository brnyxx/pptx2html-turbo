from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.jcs import canonicalize
from evaluate.materialize_multiformat_command_plan import materialize_command_plan
from evaluate.multiformat_command_evidence import load_command_plan
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import JsonValue
from evaluate.tests.multiformat_capture_fixture import (
    add_capture_units,
    candidate_capture_files,
    write_capture_manifests,
)
from evaluate.tests.multiformat_hard_gate_fixture import (
    determinism_run,
    quality_evidence,
    security_records,
)
from evaluate.tests.multiformat_metric_artifact_fixture import (
    binding,
    sha256,
    write_unit_artifacts,
)

PROJECT_ROOT = Path(__file__).resolve().parents[2]


def write_metrics(
    contract: Path,
    corpus: Path,
    evaluator_hash: str,
    oracle_hash: str,
    evidence_root: Path | None = None,
) -> Path:
    root = evidence_root or corpus.parent
    corpus_value = json.loads(corpus.read_text(encoding="utf-8"))
    tracks = corpus_value["tracks"]
    document_format = corpus_value["format"]
    project_revision = current_project_revision(PROJECT_ROOT)
    corpus_hash = sha256(corpus)
    width, height = (960, 540) if document_format in {"ppt", "pptx"} else (192, 192)
    conformance: list[dict[str, JsonValue]] = []
    pair_ids: list[str] = []
    capture_units: dict[str, list[dict[str, JsonValue]]] = {
        "oracle": [],
        "candidate": [],
    }
    for source in tracks["conformance"]["items"]:
        for unit in source["units"]:
            pair_ids.append(unit["id"])
            artifacts = write_unit_artifacts(
                root,
                unit["id"],
                width,
                height,
            )
            add_capture_units(
                capture_units,
                unit["id"],
                source["id"],
                source["sha256"],
                unit["ordinal"],
                artifacts,
            )
            conformance.append(
                {
                    "source_id": source["id"],
                    "source_sha256": source["sha256"],
                    "unit_id": unit["id"],
                    "ordinal": unit["ordinal"],
                    "critical_defect": False,
                    "artifacts": artifacts,
                }
            )
    blind: list[dict[str, JsonValue]] = []
    for source in tracks["blind"]["items"]:
        units: list[dict[str, JsonValue]] = []
        for ordinal in range(1, source["unit_count"] + 1):
            unit_id = f"{source['id']}-unit-{ordinal}"
            pair_ids.append(unit_id)
            artifacts = write_unit_artifacts(
                root,
                unit_id,
                width,
                height,
            )
            add_capture_units(
                capture_units,
                unit_id,
                source["id"],
                source["sha256"],
                ordinal,
                artifacts,
            )
            units.append(
                {
                    "unit_id": unit_id,
                    "ordinal": ordinal,
                    "critical_defect": False,
                    "artifacts": artifacts,
                }
            )
        blind.append(
            {
                "source_id": source["id"],
                "source_sha256": source["sha256"],
                "critical_defect": False,
                "units": units,
            }
        )
    python = Path(sys.executable).resolve().as_posix()
    commands_path = root / f"{document_format}-commands.json"
    cargo = subprocess.run(
        ["rustup", "which", "cargo"], check=True, capture_output=True, text=True
    ).stdout.strip()
    env = Path("/usr/bin/env").resolve().as_posix()
    materialize_command_plan(
        commands_path,
        (python, "-m", "evaluate.run_multiformat_security_case"),
        {
            "tests": (env, cargo, "test"),
            "builds": (env, cargo, "build"),
            "diagnostics": (env, cargo, "clippy"),
            "contract_checks": (python, "-m", "evaluate.check_exactness_contract"),
        },
        (env, cargo, "test", "--release"),
    )
    command_plan = load_command_plan(commands_path)
    security = security_records(
        root,
        tracks["security"]["items"],
        project_revision,
        evaluator_hash,
        corpus_hash,
        command_plan,
    )
    candidate_files = candidate_capture_files(
        root,
        document_format,
        capture_units["candidate"],
    )
    determinism_value: dict[str, JsonValue] = {
        "runs": [
            determinism_run(
                root,
                1,
                tracks,
                document_format,
                capture_units["candidate"],
                candidate_files,
            ),
            determinism_run(
                root,
                2,
                tracks,
                document_format,
                capture_units["candidate"],
                candidate_files,
            ),
        ]
    }
    capture_bindings, candidate_files = write_capture_manifests(
        root,
        document_format,
        capture_units,
        sha256(contract),
        corpus_hash,
        evaluator_hash,
        oracle_hash,
        project_revision,
        determinism_value,
    )
    quality, performance = quality_evidence(
        root,
        document_format,
        evaluator_hash,
        corpus_hash,
        project_revision,
        command_plan,
    )
    review = _signed_reviews(
        root,
        document_format,
        pair_ids,
        capture_units["oracle"],
        capture_units["candidate"],
        {
            "contract_sha256": sha256(contract),
            "corpus_manifest_sha256": sha256(corpus),
            "evaluator_manifest_sha256": evaluator_hash,
            "oracle_lock_sha256": oracle_hash,
            "project_revision": project_revision,
            **capture_bindings,
        },
    )
    value: dict[str, JsonValue] = {
        "schema_version": 2,
        "status": "READY",
        "format": document_format,
        "bindings": {
            "contract_sha256": sha256(contract),
            "corpus_manifest_sha256": sha256(corpus),
            "evaluator_manifest_sha256": evaluator_hash,
            "oracle_lock_sha256": oracle_hash,
            "project_revision": project_revision,
            **capture_bindings,
            "command_plan": binding(root, commands_path),
            "command_plan_sha256": command_plan.sha256,
        },
        "conformance": {"units": conformance},
        "blind": {"files": blind},
        "security": {"cases": security},
        "determinism": determinism_value,
        "review": review,
        "quality": quality,
        "performance": performance,
    }
    path = root / f"{document_format}-metrics.json"
    path.write_text(json.dumps(value, sort_keys=True), encoding="utf-8")
    return path


def _signed_reviews(
    root: Path,
    document_format: str,
    pair_ids: list[str],
    oracle_units: list[dict[str, JsonValue]],
    candidate_units: list[dict[str, JsonValue]],
    bindings: dict[str, JsonValue],
) -> dict[str, JsonValue]:
    oracle = {str(unit["unit_id"]): unit for unit in oracle_units}
    candidate = {str(unit["unit_id"]): unit for unit in candidate_units}
    keys = (Ed25519PrivateKey.generate(), Ed25519PrivateKey.generate())
    reviewers = (("reviewer-1", "visual"), ("reviewer-2", "semantic-security"))
    review_root = root / "reviews"
    review_root.mkdir(exist_ok=True)
    packet_path = review_root / f"{document_format}-packet.json"
    packet: dict[str, JsonValue] = {
        "schema_version": 2,
        "status": "READY",
        "checklist_version": "multiformat-review-v2",
        "bindings": bindings,
        "reviewers": [
            {
                "reviewer_id": reviewer_id,
                "reviewer_role": role,
                "algorithm": "ed25519",
                "public_key": key.public_key().public_bytes_raw().hex(),
                "public_key_sha256": hashlib.sha256(
                    key.public_key().public_bytes_raw()
                ).hexdigest(),
            }
            for (reviewer_id, role), key in zip(reviewers, keys, strict=True)
        ],
        "pairs": [
            {
                "pair_id": pair_id,
                "reference_png_sha256": oracle[pair_id]["png"]["sha256"],
                "candidate_png_sha256": candidate[pair_id]["png"]["sha256"],
                "reference_inventory_sha256": oracle[pair_id]["inventory"]["sha256"],
                "candidate_inventory_sha256": candidate[pair_id]["inventory"]["sha256"],
            }
            for pair_id in sorted(pair_ids)
        ],
    }
    packet_path.write_bytes(canonicalize(packet))
    packet_hash = sha256(packet_path)
    decisions: list[JsonValue] = []
    for (reviewer_id, role), key in zip(reviewers, keys, strict=True):
        decision: dict[str, JsonValue] = {
            "schema_version": 2,
            "packet_sha256": packet_hash,
            "reviewer_id": reviewer_id,
            "reviewer_role": role,
            "public_key_sha256": hashlib.sha256(
                key.public_key().public_bytes_raw()
            ).hexdigest(),
            "checklist_version": "multiformat-review-v2",
            "pairs": [
                {
                    "pair_id": pair_id,
                    "decision": "PASS",
                    "critical_defect": False,
                }
                for pair_id in sorted(pair_ids)
            ],
        }
        signed = {**decision, "signature": key.sign(canonicalize(decision)).hex()}
        path = review_root / f"{document_format}-{reviewer_id}.json"
        path.write_bytes(canonicalize(signed))
        decisions.append(binding(root, path))
    return {"packet": binding(root, packet_path), "decisions": decisions}
