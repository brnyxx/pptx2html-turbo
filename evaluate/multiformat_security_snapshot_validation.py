from __future__ import annotations

import hashlib
import json
import stat
from dataclasses import dataclass
from pathlib import Path

from evaluate.multiformat_corpus_contract import corpus_rules
from evaluate.multiformat_corpus_items import (
    add_unique,
    object_list,
    require_keys,
)
from evaluate.multiformat_corpus_sources import validate_source
from evaluate.multiformat_corpus_types import (
    CorpusError,
    DocumentFormat,
    SecurityOutcome,
)
from evaluate.multiformat_package_validation import MAX_SOURCE_BYTES
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_list,
    string_value,
)
from evaluate.multiformat_security_fixture import validate_security_fixture
from evaluate.multiformat_security_snapshot import (
    SecuritySnapshotError,
    SecuritySnapshotSummary,
)
from evaluate.multiformat_strict_json import (
    StrictJsonError,
    parse_strict_object_bytes,
)

MANIFEST_NAME = "security-sources.json"


@dataclass(frozen=True, slots=True)
class _ExpectedSource:
    document_format: DocumentFormat
    family: str
    outcome: SecurityOutcome
    values: dict[str, JsonValue]
    relative_path: str


def validate_security_snapshot(
    contract_path: Path,
    manifest_path: Path,
) -> SecuritySnapshotSummary:
    try:
        if manifest_path.name != MANIFEST_NAME:
            raise SecuritySnapshotError("security.snapshot.manifest.name")
        contract_bytes = contract_path.read_bytes()
        contract = parse_strict_object_bytes(contract_bytes)
        formats = _contract_formats(contract)
        root = manifest_path.parent
        manifest_bytes = _read_regular_file(manifest_path)
        manifest = parse_strict_object_bytes(manifest_bytes)
        if manifest_bytes != _canonical_bytes(manifest):
            raise SecuritySnapshotError("security.snapshot.manifest.canonical")
        expected_sources = _manifest_sources(
            manifest,
            formats,
            hashlib.sha256(contract_bytes).hexdigest(),
        )
        expected_files = {MANIFEST_NAME}
        expected_files.update(source.relative_path for source in expected_sources)
        expected_dirs = {"sources"}
        expected_dirs.update(f"sources/{item.value}" for item in formats)
        _validate_tree(root, expected_files, expected_dirs)
        _validate_sources(root, expected_sources)
        return SecuritySnapshotSummary(
            counts={
                document_format.value: len(outcomes)
                for document_format, outcomes in formats.items()
            },
            files=len(expected_files),
            manifest_sha256=hashlib.sha256(manifest_bytes).hexdigest(),
        )
    except SecuritySnapshotError:
        raise
    except (
        CorpusError,
        OSError,
        StrictJsonError,
        TypeError,
        ValueError,
    ) as error:
        raise SecuritySnapshotError(
            "security.snapshot.validation",
            str(error),
        ) from error


def _contract_formats(
    contract: dict[str, JsonValue],
) -> dict[DocumentFormat, dict[str, SecurityOutcome]]:
    if integer_value(contract, "schema_version") != 1:
        raise SecuritySnapshotError("security.snapshot.contract.version")
    names = string_list(contract, "required_formats")
    expected = {item.value for item in DocumentFormat}
    if len(names) != len(set(names)) or set(names) != expected:
        raise SecuritySnapshotError("security.snapshot.contract.formats")
    values = object_value(contract, "security_case_outcomes")
    if set(values) != expected:
        raise SecuritySnapshotError("security.snapshot.contract.outcomes")
    result: dict[DocumentFormat, dict[str, SecurityOutcome]] = {}
    for name in sorted(names):
        document_format = DocumentFormat(name)
        rules = corpus_rules(contract, document_format)
        if len(rules.security_outcomes) != 10:
            raise SecuritySnapshotError("security.snapshot.contract.count")
        result[document_format] = dict(sorted(rules.security_outcomes.items()))
    return result


def _manifest_sources(
    manifest: dict[str, JsonValue],
    formats: dict[DocumentFormat, dict[str, SecurityOutcome]],
    contract_digest: str,
) -> list[_ExpectedSource]:
    require_keys(
        manifest,
        {
            "contract_sha256",
            "formats",
            "schema_version",
            "status",
        },
        "security.snapshot.manifest",
    )
    if (
        integer_value(manifest, "schema_version") != 1
        or string_value(manifest, "status") != "GENERATED"
        or sha256_value(manifest, "contract_sha256") != contract_digest
    ):
        raise SecuritySnapshotError("security.snapshot.manifest.binding")
    format_values = object_value(manifest, "formats")
    if set(format_values) != {item.value for item in formats}:
        raise SecuritySnapshotError("security.snapshot.manifest.formats")
    result: list[_ExpectedSource] = []
    for document_format, outcomes in formats.items():
        format_value = object_value(format_values, document_format.value)
        require_keys(
            format_value,
            {"expected_count", "sources"},
            "security.snapshot.format",
        )
        sources = object_list(
            format_value,
            "sources",
            "security.snapshot.sources",
        )
        if integer_value(format_value, "expected_count") != 10 or len(sources) != 10:
            raise SecuritySnapshotError("security.snapshot.manifest.count")
        for source, (family, outcome) in zip(sources, outcomes.items(), strict=True):
            result.append(
                _expected_source(
                    document_format,
                    family,
                    outcome,
                    source,
                )
            )
    return result


def _expected_source(
    document_format: DocumentFormat,
    family: str,
    outcome: SecurityOutcome,
    source: dict[str, JsonValue],
) -> _ExpectedSource:
    require_keys(
        source,
        {"case_family", "expected_outcome", "id", "path", "sha256"},
        "security.snapshot.source",
    )
    relative = f"sources/{document_format.value}/{family}.{document_format.value}"
    if (
        string_value(source, "case_family") != family
        or string_value(source, "expected_outcome") != outcome.value
        or string_value(source, "id") != f"security-{document_format.value}-{family}"
        or string_value(source, "path") != relative
    ):
        raise SecuritySnapshotError("security.snapshot.source.binding")
    sha256_value(source, "sha256")
    return _ExpectedSource(document_format, family, outcome, source, relative)


def _validate_tree(
    root: Path,
    expected_files: set[str],
    expected_dirs: set[str],
) -> None:
    root_value = root.lstat()
    if root.is_symlink() or not stat.S_ISDIR(root_value.st_mode):
        raise SecuritySnapshotError("security.snapshot.root")
    files: set[str] = set()
    directories: set[str] = set()
    for path in root.rglob("*"):
        relative = path.relative_to(root).as_posix()
        value = path.lstat()
        if path.is_symlink():
            raise SecuritySnapshotError("security.snapshot.filesystem", relative)
        if stat.S_ISDIR(value.st_mode):
            directories.add(relative)
        elif stat.S_ISREG(value.st_mode) and value.st_nlink == 1:
            files.add(relative)
        else:
            raise SecuritySnapshotError("security.snapshot.filesystem", relative)
    if files != expected_files or directories != expected_dirs:
        raise SecuritySnapshotError("security.snapshot.fileset")


def _validate_sources(root: Path, sources: list[_ExpectedSource]) -> None:
    ids: set[str] = set()
    paths: set[str] = set()
    digests: set[str] = set()
    for source in sources:
        path = root / source.relative_path
        value = path.lstat()
        if not 0 < value.st_size <= MAX_SOURCE_BYTES:
            raise SecuritySnapshotError("security.snapshot.source.size")
        digest = sha256_file(path)
        if digest != sha256_value(source.values, "sha256"):
            raise SecuritySnapshotError("security.snapshot.source.sha256")
        add_unique(ids, string_value(source.values, "id"), "security.snapshot.id")
        add_unique(paths, source.relative_path, "security.snapshot.path")
        add_unique(digests, digest, "security.snapshot.sha256")
        validate_security_fixture(path, source.document_format, source.family)
        if source.outcome is SecurityOutcome.SAFE_CONVERT:
            validate_source(
                source.values,
                root,
                source.document_format,
                require_valid_format=True,
            )


def _read_regular_file(path: Path) -> bytes:
    value = path.lstat()
    if path.is_symlink() or not stat.S_ISREG(value.st_mode) or value.st_nlink != 1:
        raise SecuritySnapshotError("security.snapshot.manifest.file")
    return path.read_bytes()


def _canonical_bytes(value: dict[str, JsonValue]) -> bytes:
    return (
        json.dumps(value, ensure_ascii=True, indent=2, sort_keys=True) + "\n"
    ).encode()
