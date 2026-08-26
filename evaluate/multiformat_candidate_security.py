from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_candidate_conversion import (
    CandidateConversionError,
    ConversionResult,
    run_conversion,
)
from evaluate.multiformat_candidate_security_browser import (
    SecurityBrowserFacts,
    inspect_security_html,
)
from evaluate.multiformat_candidate_types import (
    CandidateCaptureError,
    CandidateRuntimePaths,
)
from evaluate.multiformat_corpus import validate_corpus_manifest
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_types import (
    CorpusStatus,
    DocumentFormat,
    SecurityOutcome,
)
from evaluate.multiformat_evidence import resolve_evidence_path
from evaluate.multiformat_schema import (
    sha256_file,
    sha256_value,
    string_value,
    object_value,
)
from evaluate.multiformat_strict_json import read_strict_object


class CandidateSecurityError(CandidateCaptureError):
    pass


@dataclass(frozen=True, slots=True)
class CandidateSecuritySource:
    source_id: str
    path: Path
    sha256: str
    case_family: str
    expected_outcome: SecurityOutcome


def load_candidate_security_sources(
    contract_path: Path, corpus_path: Path
) -> tuple[DocumentFormat, tuple[CandidateSecuritySource, ...]]:
    validation = validate_corpus_manifest(contract_path, corpus_path)
    if validation.status is not CorpusStatus.READY:
        raise CandidateSecurityError("security corpus is not READY")
    manifest = read_strict_object(corpus_path)
    document_format = DocumentFormat(string_value(manifest, "format"))
    track = object_value(object_value(manifest, "tracks"), "security")
    items = object_list(track, "items", "security.items")
    if len(items) != 10:
        raise CandidateSecurityError("candidate security requires exactly 10 cases")
    root = corpus_path.parent.resolve(strict=True)
    sources = tuple(
        CandidateSecuritySource(
            string_value(item, "id"),
            resolve_evidence_path(root, string_value(item, "path")),
            sha256_value(item, "sha256"),
            string_value(item, "case_family"),
            SecurityOutcome(string_value(item, "expected_outcome")),
        )
        for item in items
    )
    if len({source.source_id for source in sources}) != 10:
        raise CandidateSecurityError("candidate security IDs are not unique")
    return document_format, sources


def capture_candidate_security(
    contract_path: Path,
    corpus_path: Path,
    evaluator_path: Path,
    output_dir: Path,
    runtime: CandidateRuntimePaths,
    project_revision: str,
) -> tuple[Path, ...]:
    document_format, sources = load_candidate_security_sources(
        contract_path, corpus_path
    )
    output_dir.mkdir(parents=True, exist_ok=False)
    results = tuple(
        _execute_case(
            source,
            document_format,
            output_dir,
            runtime,
            project_revision,
            sha256_file(evaluator_path),
            sha256_file(corpus_path),
        )
        for source in sources
    )
    return results


def _execute_case(
    source: CandidateSecuritySource,
    document_format: DocumentFormat,
    output_dir: Path,
    runtime: CandidateRuntimePaths,
    project_revision: str,
    evaluator_hash: str,
    corpus_hash: str,
) -> Path:
    if sha256_file(source.path) != source.sha256:
        raise CandidateSecurityError(f"security source changed: {source.source_id}")
    typed_error: str | None = None
    conversion: ConversionResult | None = None
    case_root = output_dir / source.source_id
    case_root.mkdir(parents=True, exist_ok=False)
    try:
        conversion = run_conversion(
            runtime.converter,
            source.path,
            document_format,
            case_root / "conversion",
            soffice=runtime.soffice,
            pdftohtml=runtime.pdftohtml,
            pdfinfo=runtime.pdfinfo,
            timeout_seconds=runtime.timeout_seconds,
        )
        observed = SecurityOutcome.SAFE_CONVERT
    except CandidateConversionError as error:
        if not str(error).startswith("converter exit code "):
            raise CandidateSecurityError(
                f"security execution infrastructure failed: {source.source_id}"
            ) from error
        observed = SecurityOutcome.REJECT
        typed_error = "document2html.conversion-rejected"
    facts = SecurityBrowserFacts((), False)
    if conversion is not None:
        facts = inspect_security_html(
            conversion.html,
            chromium=runtime.chromium,
            browser_version=runtime.browser_version,
            font_config=runtime.font_config,
        )
    passed = (
        observed is source.expected_outcome
        and not facts.external_requests
        and not facts.active_content_executed
    )
    result = case_root / "execution.json"
    write_canonical_json(
        result,
        {
            "schema_version": 1,
            "status": "PASS" if passed else "FAIL",
            "source_id": source.source_id,
            "source_sha256": source.sha256,
            "case_family": source.case_family,
            "expected_outcome": source.expected_outcome.value,
            "observed_outcome": observed.value,
            "typed_error": typed_error,
            "network_isolation": "disabled",
            "external_fetches": list(facts.external_requests),
            "active_content_executed": facts.active_content_executed,
            "within_limits": True,
            "project_revision": project_revision,
            "evaluator_manifest_sha256": evaluator_hash,
            "corpus_manifest_sha256": corpus_hash,
        },
    )
    if not passed:
        raise CandidateSecurityError(f"security outcome failed: {source.source_id}")
    return result


__all__ = [
    "CandidateSecurityError",
    "CandidateSecuritySource",
    "capture_candidate_security",
    "load_candidate_security_sources",
]
