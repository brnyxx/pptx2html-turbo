from __future__ import annotations

from typing import assert_never

from evaluate.multiformat_corpus_types import (
    CorpusError,
    CorpusRules,
    DocumentFormat,
    SecurityOutcome,
)
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    string_list,
    string_value,
)


def corpus_rules(
    contract: dict[str, JsonValue],
    document_format: DocumentFormat,
) -> CorpusRules:
    corpus = object_value(contract, "corpus")
    conformance_units = integer_value(corpus, "conformance_units")
    blind_files = integer_value(corpus, "blind_files")
    security_cases = integer_value(corpus, "security_cases")
    strata = set(
        string_list(
            object_value(contract, "strata"),
            document_format.value,
        )
    )
    quotas = integer_map(
        object_value(
            object_value(contract, "stratum_quotas"),
            document_format.value,
        ),
        "contract.stratum_quotas",
    )
    if (
        set(quotas) != strata
        or any(value <= 0 for value in quotas.values())
        or sum(quotas.values()) != conformance_units
    ):
        raise CorpusError("contract.stratum_quotas", document_format.value)
    security_outcomes = _security_outcomes(contract, document_format)
    if len(security_outcomes) != security_cases:
        raise CorpusError("contract.security_case_outcomes", document_format.value)
    paired_format = _paired_format(document_format)
    paired_quotas = _paired_quotas(contract, document_format, paired_format)
    return CorpusRules(
        conformance_units,
        blind_files,
        security_cases,
        quotas,
        security_outcomes,
        paired_quotas,
        paired_format,
    )


def integer_map(
    values: dict[str, JsonValue],
    reason: str,
) -> dict[str, int]:
    result: dict[str, int] = {}
    for name in values:
        result[name] = integer_value(values, name)
    if not result:
        raise CorpusError(reason, "must not be empty")
    return result


def _security_outcomes(
    contract: dict[str, JsonValue],
    document_format: DocumentFormat,
) -> dict[str, SecurityOutcome]:
    values = object_value(
        object_value(contract, "security_case_outcomes"),
        document_format.value,
    )
    result: dict[str, SecurityOutcome] = {}
    for family in values:
        try:
            result[family] = SecurityOutcome(string_value(values, family))
        except ValueError as error:
            raise CorpusError(
                "contract.security_case_outcomes",
                family,
            ) from error
    return result


def _paired_format(document_format: DocumentFormat) -> DocumentFormat | None:
    match document_format:
        case DocumentFormat.DOC:
            return DocumentFormat.DOCX
        case DocumentFormat.XLS:
            return DocumentFormat.XLSX
        case DocumentFormat.PPT:
            return DocumentFormat.PPTX
        case (
            DocumentFormat.DOCX
            | DocumentFormat.XLSX
            | DocumentFormat.PPTX
            | DocumentFormat.PDF
        ):
            return None
        case _ as unreachable:
            assert_never(unreachable)


def _paired_quotas(
    contract: dict[str, JsonValue],
    document_format: DocumentFormat,
    paired_format: DocumentFormat | None,
) -> dict[str, int] | None:
    if paired_format is None:
        return None
    values = integer_map(
        object_value(
            object_value(contract, "legacy_paired_stratum_quotas"),
            document_format.value,
        ),
        "contract.legacy_paired_stratum_quotas",
    )
    expected_strata = set(
        string_list(
            object_value(contract, "strata"),
            paired_format.value,
        )
    )
    if set(values) != expected_strata or sum(values.values()) != 60:
        raise CorpusError(
            "contract.legacy_paired_stratum_quotas",
            document_format.value,
        )
    return values
