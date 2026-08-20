from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_types import DocumentFormat, SecurityOutcome
from evaluate.multiformat_metric_types import (
    BlindFileSpec,
    ConformanceUnitSpec,
    CorpusMetricSpec,
    MetricError,
    SecurityCaseSpec,
)
from evaluate.multiformat_schema import (
    integer_value,
    object_value,
    sha256_value,
    string_list,
    string_value,
)
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object


def load_metric_spec(corpus_path: Path) -> CorpusMetricSpec:
    try:
        corpus = read_strict_object(corpus_path)
        document_format = DocumentFormat(string_value(corpus, "format"))
        tracks = object_value(corpus, "tracks")
        conformance: dict[str, ConformanceUnitSpec] = {}
        for source in object_list(
            object_value(tracks, "conformance"),
            "items",
            "conformance.items",
        ):
            source_id = string_value(source, "id")
            source_hash = sha256_value(source, "sha256")
            for unit in object_list(source, "units", "conformance.units"):
                unit_id = string_value(unit, "id")
                if unit_id in conformance:
                    raise MetricError("conformance.unit_set", unit_id)
                conformance[unit_id] = ConformanceUnitSpec(
                    source_id,
                    source_hash,
                    unit_id,
                    integer_value(unit, "ordinal"),
                    string_value(unit, "primary_stratum"),
                    frozenset(string_list(unit, "applicable_metrics")),
                    string_value(unit, "background"),
                )
        blind: dict[str, BlindFileSpec] = {}
        for source in object_list(
            object_value(tracks, "blind"),
            "items",
            "blind.items",
        ):
            source_id = string_value(source, "id")
            if source_id in blind:
                raise MetricError("blind.file_set", source_id)
            blind[source_id] = BlindFileSpec(
                source_id,
                sha256_value(source, "sha256"),
                integer_value(source, "unit_count"),
                frozenset(string_list(source, "applicable_metrics")),
                string_value(source, "background"),
            )
        security: dict[str, SecurityCaseSpec] = {}
        for source in object_list(
            object_value(tracks, "security"),
            "items",
            "security.items",
        ):
            source_id = string_value(source, "id")
            if source_id in security:
                raise MetricError("security.case_set", source_id)
            security[source_id] = SecurityCaseSpec(
                source_id,
                sha256_value(source, "sha256"),
                string_value(source, "case_family"),
                SecurityOutcome(string_value(source, "expected_outcome")),
            )
        return CorpusMetricSpec(document_format, conformance, blind, security)
    except MetricError:
        raise
    except (
        StrictJsonError,
        TypeError,
        ValueError,
    ) as error:
        raise MetricError("metrics.corpus", corpus_path.as_posix()) from error
