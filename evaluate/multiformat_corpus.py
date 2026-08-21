from __future__ import annotations

import argparse
import json
import sys
from collections.abc import Sequence
from pathlib import Path
from typing import TYPE_CHECKING, assert_never

from evaluate.multiformat_corpus_conformance import validate_conformance
from evaluate.multiformat_corpus_contract import corpus_rules, integer_map
from evaluate.multiformat_corpus_tracks import (
    validate_blind,
    validate_incomplete_tracks,
    validate_security,
)
from evaluate.multiformat_corpus_types import (
    CorpusError,
    CorpusStatus,
    CorpusValidation,
    DocumentFormat,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_list,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object

if TYPE_CHECKING:
    from evaluate.multiformat_corpus_identity import AdmittedCorpusValidation

__all__ = [
    "CorpusError",
    "CorpusStatus",
    "CorpusValidation",
    "validate_corpus_manifest",
    "validate_frozen_corpus",
]


def validate_frozen_corpus(
    contract_path: Path,
    corpus_path: Path,
) -> CorpusValidation | AdmittedCorpusValidation:
    """Validate an aggregate corpus directory or legacy per-format manifest."""
    if corpus_path.is_dir():
        from evaluate.multiformat_corpus_identity import validate_admitted_corpus

        return validate_admitted_corpus(contract_path, corpus_path)
    return validate_corpus_manifest(contract_path, corpus_path)


def validate_corpus_manifest(
    contract_path: Path,
    manifest_path: Path,
) -> CorpusValidation:
    try:
        return _validate_corpus_manifest(contract_path, manifest_path)
    except CorpusError:
        raise
    except (
        OSError,
        UnicodeError,
        json.JSONDecodeError,
        TypeError,
        ValueError,
    ) as error:
        raise CorpusError("manifest.schema", str(error)) from error


def _validate_corpus_manifest(
    contract_path: Path,
    manifest_path: Path,
) -> CorpusValidation:
    contract = read_strict_object(contract_path)
    manifest = read_strict_object(manifest_path)
    _require_keys(
        manifest,
        {
            "schema_version",
            "status",
            "format",
            "contract_sha256",
            "stratum_quotas",
            "tracks",
        },
        "manifest.schema",
    )
    if integer_value(manifest, "schema_version") != 2:
        raise CorpusError("manifest.schema_version", "expected 2")
    try:
        status = CorpusStatus(string_value(manifest, "status"))
        document_format = DocumentFormat(string_value(manifest, "format"))
    except ValueError as error:
        raise CorpusError("manifest.schema", str(error)) from error
    if document_format.value not in string_list(contract, "required_formats"):
        raise CorpusError("manifest.format", document_format.value)
    if sha256_value(manifest, "contract_sha256") != sha256_file(contract_path):
        raise CorpusError("manifest.contract_sha256", document_format.value)

    rules = corpus_rules(contract, document_format)
    expected_counts = {
        "conformance": rules.conformance_units,
        "blind": rules.blind_files,
        "security": rules.security_cases,
    }
    manifest_quotas = integer_map(
        object_value(manifest, "stratum_quotas"),
        "manifest.stratum_quotas",
    )
    if manifest_quotas != rules.quotas:
        raise CorpusError("manifest.stratum_quotas", document_format.value)
    tracks = object_value(manifest, "tracks")
    match status:
        case CorpusStatus.INCOMPLETE:
            validate_incomplete_tracks(tracks, expected_counts)
            return CorpusValidation(status, document_format, 0, 0, 0, 0)
        case CorpusStatus.READY:
            pass
        case _ as unreachable:
            assert_never(unreachable)

    _require_keys(tracks, {"conformance", "blind", "security"}, "tracks")
    root = manifest_path.parent
    conformance = validate_conformance(
        object_value(tracks, "conformance"),
        root,
        document_format,
        rules,
    )
    blind, producers = validate_blind(
        object_value(tracks, "blind"),
        root,
        document_format,
        expected_counts["blind"],
    )
    security = validate_security(
        object_value(tracks, "security"),
        root,
        document_format,
        expected_counts["security"],
        rules.security_outcomes,
    )
    if len(conformance.item_ids | blind.item_ids | security.item_ids) != (
        len(conformance.item_ids) + len(blind.item_ids) + len(security.item_ids)
    ):
        raise CorpusError("tracks.id", "duplicate item id")
    if len(conformance.source_paths | blind.source_paths | security.source_paths) != (
        len(conformance.source_paths)
        + len(blind.source_paths)
        + len(security.source_paths)
    ):
        raise CorpusError("tracks.path", "duplicate source path")
    if len(
        conformance.source_hashes | blind.source_hashes | security.source_hashes
    ) != (
        len(conformance.source_hashes)
        + len(blind.source_hashes)
        + len(security.source_hashes)
    ):
        raise CorpusError("tracks.sha256", "duplicate source digest")
    return CorpusValidation(
        status,
        document_format,
        conformance.count,
        blind.count,
        security.count,
        producers,
    )


def _require_keys(
    values: dict[str, JsonValue],
    expected: set[str],
    reason: str,
) -> None:
    if set(values) != expected:
        raise CorpusError(reason, "unexpected object fields")


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Validate one frozen multi-format corpus manifest.",
    )
    parser.add_argument(
        "--contract",
        type=Path,
        default=Path("evaluate/multiformat/contract.v1.json"),
    )
    parser.add_argument("--manifest", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        result = validate_corpus_manifest(args.contract, args.manifest)
    except CorpusError as error:
        sys.stdout.write(
            json.dumps(
                {"status": "FAIL", "reason": error.reason, "detail": error.detail},
                ensure_ascii=True,
                sort_keys=True,
            )
            + "\n"
        )
        return 1
    sys.stdout.write(
        json.dumps(result.to_json_value(), ensure_ascii=True, sort_keys=True) + "\n"
    )
    return 0 if result.status is CorpusStatus.READY else 2


if __name__ == "__main__":
    raise SystemExit(main())
