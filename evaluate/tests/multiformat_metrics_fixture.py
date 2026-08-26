from __future__ import annotations

import hashlib
import json
import subprocess
import sys
from collections.abc import Iterator
from contextlib import contextmanager
from pathlib import Path
from unittest import mock

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from evaluate.jcs import canonicalize
from evaluate.materialize_multiformat_command_plan import materialize_command_plan
from evaluate.multiformat_command_evidence import load_command_plan
from evaluate.multiformat_review_registry import (
    RegisteredReviewer,
    ReviewerRegistry,
)
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
    pair_digests,
    sha256,
    write_unit_artifacts,
)

PROJECT_ROOT = Path(__file__).resolve().parents[2]

# Fixed test-only reviewer seeds. Deterministic keys let a test edit a signed
# decision and re-sign it canonically, which a freshly generated key forbids.
REVIEWER_SEEDS: dict[str, bytes] = {
    "reviewer-1": bytes(range(32)),
    "reviewer-2": bytes(range(32, 64)),
}
REVIEWER_ROLES: dict[str, str] = {
    "reviewer-1": "visual",
    "reviewer-2": "semantic-security",
}


def reviewer_key(reviewer_id: str) -> Ed25519PrivateKey:
    """Returns the deterministic test-only signing key for one reviewer."""
    return Ed25519PrivateKey.from_private_bytes(REVIEWER_SEEDS[reviewer_id])


def test_reviewer_registry() -> ReviewerRegistry:
    """Builds the registry that trusts exactly these test-only reviewers.

    Metrics re-load reviewer trust from the registry, so fixture-built packets
    are only valid against a registry carrying the same public keys. Tests
    patch the consumer-side loader with this value; the tracked production
    registry is never involved.
    """
    return ReviewerRegistry(
        tuple(
            RegisteredReviewer(
                reviewer_id,
                REVIEWER_ROLES[reviewer_id],
                reviewer_key(reviewer_id).public_key().public_bytes_raw(),
                hashlib.sha256(
                    reviewer_key(reviewer_id).public_key().public_bytes_raw()
                ).hexdigest(),
            )
            for reviewer_id in REVIEWER_SEEDS
        )
    )


@contextmanager
def patched_reviewer_registry() -> Iterator[None]:
    """Points every registry consumer at the test-only reviewer set."""
    registry = test_reviewer_registry()
    with (
        mock.patch(
            "evaluate.multiformat_review_packet_trust.load_reviewer_registry",
            return_value=registry,
        ),
        mock.patch(
            "evaluate.multiformat_metric_review.load_reviewer_registry",
            return_value=registry,
        ),
    ):
        yield


def sign_decision_value(decision: dict[str, JsonValue]) -> bytes:
    """Canonically re-signs an edited decision with its reviewer's key."""
    key = reviewer_key(_reviewer_id(decision))
    payload = {field: decision[field] for field in decision if field != "signature"}
    signature = key.sign(canonicalize(payload)).hex()
    return canonicalize({**payload, "signature": signature})


def _reviewer_id(decision: dict[str, JsonValue]) -> str:
    value = decision["reviewer_id"]
    if not isinstance(value, str):
        raise TypeError("reviewer_id")
    return value


def write_metrics(
    contract: Path,
    corpus: Path,
    evaluator_hash: str,
    oracle_hash: str,
    evidence_root: Path | None = None,
    outer_lock: Path | None = None,
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
        record: dict[str, JsonValue] = {
            "source_id": source["id"],
            "source_sha256": source["sha256"],
            "critical_defect": False,
            "units": list(units),
        }
        blind.append(record)
    python = Path(sys.executable).resolve().as_posix()
    commands_path = root / f"{document_format}-commands.json"
    cargo = subprocess.run(
        ["rustup", "which", "cargo"], check=True, capture_output=True, text=True
    ).stdout.strip()
    rustc = subprocess.run(
        ["rustup", "which", "rustc"], check=True, capture_output=True, text=True
    ).stdout.strip()
    authority = outer_lock or root / f"{document_format}-toolchain-lock.json"
    if outer_lock is None:
        authority.write_text(
            json.dumps(
                {
                    "rust_toolchain": {
                        "cargo": {"path": cargo, "sha256": sha256(Path(cargo))},
                        "rustc": {"path": rustc, "sha256": sha256(Path(rustc))},
                    }
                }
            ),
            encoding="utf-8",
        )
    env = Path("/usr/bin/env").resolve().as_posix()
    path_arg = f"PATH={Path(rustc).parent}:/usr/bin:/bin"
    materialize_command_plan(
        commands_path,
        (python, "-m", "evaluate.run_multiformat_security_case"),
        {
            "tests": (
                env,
                path_arg,
                cargo,
                "test",
                "-p",
                "document2html-core",
                "-p",
                "document2html-native",
            ),
            "builds": (
                env,
                path_arg,
                cargo,
                "build",
                "--release",
                "-p",
                "pptx2html-cli",
                "--bin",
                "document2html",
            ),
            "diagnostics": (
                env,
                path_arg,
                cargo,
                "clippy",
                "-p",
                "document2html-core",
                "-p",
                "document2html-native",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ),
            "contract_checks": (
                python,
                "-m",
                "evaluate.check_exactness_contract",
                "--repo-root",
                PROJECT_ROOT.as_posix(),
            ),
        },
        (env, path_arg, cargo, "test", "--release", "-p", "document2html-native"),
        outer_lock=authority,
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
        "conformance": {"units": list(conformance)},
        "blind": {"files": list(blind)},
        "security": {"cases": list(security)},
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
    reviewers = tuple(
        (reviewer_id, REVIEWER_ROLES[reviewer_id]) for reviewer_id in REVIEWER_SEEDS
    )
    keys = tuple(reviewer_key(reviewer_id) for reviewer_id, _role in reviewers)
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
            pair_digests(oracle[pair_id], candidate[pair_id], pair_id)
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
        path = review_root / f"{document_format}-{reviewer_id}.json"
        path.write_bytes(sign_decision_value(decision))
        decisions.append(binding(root, path))
    return {"packet": binding(root, packet_path), "decisions": decisions}
