from __future__ import annotations

import hashlib
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_corpus_sources import validate_identifier
from evaluate.multiformat_corpus_types import (
    CorpusError,
    DocumentFormat,
    SecurityOutcome,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    string_list,
    string_value,
)
from evaluate.multiformat_security_publish import (
    SecurityPublishError,
    publish_security_snapshot,
)
from evaluate.multiformat_security_source import write_security_source
from evaluate.multiformat_source_fixture import SourceFixtureError
from evaluate.multiformat_strict_json import (
    StrictJsonError,
    parse_strict_object_bytes,
)

SecurityWriter = Callable[[Path, DocumentFormat, str], None]
BeforePublish = Callable[[], None]


class SecuritySnapshotError(Exception):
    __slots__ = ("detail", "reason")

    def __init__(self, reason: str, detail: str = "") -> None:
        self.reason = reason
        self.detail = detail
        super().__init__(reason, detail)

    def __str__(self) -> str:
        return f"{self.reason}: {self.detail}" if self.detail else self.reason


@dataclass(frozen=True, slots=True)
class SecuritySnapshotSummary:
    counts: dict[str, int]
    files: int
    manifest_sha256: str
    status: str = "GENERATED"


SnapshotValidator = Callable[
    [Path, Path],
    SecuritySnapshotSummary | None,
]


def generate_security_snapshot(
    contract_path: Path,
    output_dir: Path,
    *,
    writer: SecurityWriter = write_security_source,
    validator: SnapshotValidator | None = None,
    before_publish: BeforePublish | None = None,
) -> SecuritySnapshotSummary:
    try:
        contract_bytes = contract_path.read_bytes()
        contract = parse_strict_object_bytes(contract_bytes)
        formats = _contract_formats(contract)
        contract_digest = hashlib.sha256(contract_bytes).hexdigest()
        summary: SecuritySnapshotSummary | None = None

        def populate(staging: Path) -> None:
            nonlocal summary
            manifest = _write_snapshot(
                staging,
                formats,
                contract_digest,
                writer,
            )
            active_validator = validator or _default_validator
            active_validator(contract_path, manifest)
            if before_publish is not None:
                before_publish()
            if contract_path.read_bytes() != contract_bytes:
                raise SecuritySnapshotError("security snapshot contract changed")
            summary = _summary(staging, manifest, formats)

        publish_security_snapshot(output_dir, populate)
        if summary is None:
            raise SecuritySnapshotError("security snapshot summary is missing")
        return summary
    except SecuritySnapshotError:
        raise
    except (
        CorpusError,
        OSError,
        SecurityPublishError,
        SourceFixtureError,
        StrictJsonError,
        TypeError,
        ValueError,
    ) as error:
        raise SecuritySnapshotError(
            "security.snapshot.generation",
            str(error),
        ) from error


def _contract_formats(
    contract: dict[str, JsonValue],
) -> dict[DocumentFormat, dict[str, SecurityOutcome]]:
    if integer_value(contract, "schema_version") != 1:
        raise SecuritySnapshotError("security snapshot contract version differs")
    names = string_list(contract, "required_formats")
    expected = {item.value for item in DocumentFormat}
    if len(names) != len(set(names)) or set(names) != expected:
        raise SecuritySnapshotError("security snapshot format set differs")
    corpus = object_value(contract, "corpus")
    if integer_value(corpus, "security_cases") != 10:
        raise SecuritySnapshotError("security snapshot case count differs")
    outcome_values = object_value(contract, "security_case_outcomes")
    if set(outcome_values) != expected:
        raise SecuritySnapshotError("security snapshot outcome formats differ")
    result: dict[DocumentFormat, dict[str, SecurityOutcome]] = {}
    for name in sorted(names):
        values = object_value(outcome_values, name)
        if len(values) != 10:
            raise SecuritySnapshotError("security snapshot family count differs")
        outcomes: dict[str, SecurityOutcome] = {}
        for family in sorted(values):
            validate_identifier(family, "security.snapshot.family")
            outcome = SecurityOutcome(string_value(values, family))
            validate_identifier(
                f"security-{name}-{family}",
                "security.snapshot.id",
            )
            outcomes[family] = outcome
        result[DocumentFormat(name)] = outcomes
    return result


def _write_snapshot(
    staging: Path,
    formats: dict[DocumentFormat, dict[str, SecurityOutcome]],
    contract_digest: str,
    writer: SecurityWriter,
) -> Path:
    format_values: dict[str, JsonValue] = {}
    digests: set[str] = set()
    for document_format, outcomes in formats.items():
        source_root = staging / "sources" / document_format.value
        source_root.mkdir(parents=True)
        sources: list[JsonValue] = []
        for family, outcome in outcomes.items():
            relative = (
                f"sources/{document_format.value}/{family}.{document_format.value}"
            )
            source = staging / relative
            writer(source, document_format, family)
            digest = sha256_file(source)
            if digest in digests:
                raise SecuritySnapshotError("security snapshot source hash duplicates")
            digests.add(digest)
            sources.append(
                {
                    "case_family": family,
                    "expected_outcome": outcome.value,
                    "id": f"security-{document_format.value}-{family}",
                    "path": relative,
                    "sha256": digest,
                }
            )
        format_values[document_format.value] = {
            "expected_count": len(sources),
            "sources": sources,
        }
    manifest = staging / "security-sources.json"
    write_canonical_json(
        manifest,
        {
            "contract_sha256": contract_digest,
            "formats": format_values,
            "schema_version": 1,
            "status": "GENERATED",
        },
    )
    return manifest


def _summary(
    staging: Path,
    manifest: Path,
    formats: dict[DocumentFormat, dict[str, SecurityOutcome]],
) -> SecuritySnapshotSummary:
    return SecuritySnapshotSummary(
        counts={
            document_format.value: len(outcomes)
            for document_format, outcomes in formats.items()
        },
        files=sum(1 for path in staging.rglob("*") if path.is_file()),
        manifest_sha256=sha256_file(manifest),
    )


def _default_validator(
    contract_path: Path,
    manifest_path: Path,
) -> SecuritySnapshotSummary:
    from evaluate.multiformat_security_snapshot_validation import (
        validate_security_snapshot,
    )

    return validate_security_snapshot(contract_path, manifest_path)
