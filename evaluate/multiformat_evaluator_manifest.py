from __future__ import annotations

import importlib.metadata
import platform
import unicodedata
from pathlib import Path

from evaluate.multiformat_corpus_items import object_list, require_keys
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_evaluator_files import EVALUATOR_FILES
from evaluate.multiformat_evidence import resolve_evidence_path
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_revision import current_project_revision
from evaluate.multiformat_schema import (
    JsonValue,
    integer_value,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import StrictJsonError, read_strict_object


def validate_evaluator_manifest(
    project_root: Path,
    contract_path: Path,
    manifest_path: Path,
) -> str:
    try:
        project_root = project_root.resolve(strict=True)
        contract = read_strict_object(contract_path)
        manifest = read_strict_object(manifest_path)
        evaluator_lock = read_strict_object(
            project_root / "evaluate" / "multiformat" / "evaluator-lock.v1.json"
        )
        require_keys(
            manifest,
            {
                "schema_version",
                "contract_sha256",
                "project_revision",
                "python",
                "unicode_version",
                "algorithm_parameters",
                "dependencies",
                "files",
            },
            "evaluator.schema",
        )
        if (
            integer_value(manifest, "schema_version") != 2
            or sha256_value(manifest, "contract_sha256") != sha256_file(contract_path)
            or string_value(manifest, "project_revision")
            != current_project_revision(project_root)
            or string_value(manifest, "python")
            != string_value(evaluator_lock, "python")
            or string_value(manifest, "unicode_version")
            != string_value(evaluator_lock, "unicode_version")
            or object_value(manifest, "algorithm_parameters")
            != object_value(contract, "metric_parameters")
            or object_value(manifest, "algorithm_parameters")
            != object_value(evaluator_lock, "algorithm_parameters")
            or object_value(manifest, "dependencies")
            != object_value(evaluator_lock, "dependencies")
        ):
            raise MetricError("evaluator.manifest_mismatch", manifest_path.as_posix())
        if (
            f"{platform.python_version_tuple()[0]}.{platform.python_version_tuple()[1]}"
            != string_value(evaluator_lock, "python")
            or unicodedata.unidata_version
            != string_value(evaluator_lock, "unicode_version")
        ):
            raise MetricError("evaluator.runtime", platform.python_version())
        _validate_dependencies(object_value(manifest, "dependencies"))
        files = object_list(manifest, "files", "evaluator.files")
        actual: dict[str, str] = {}
        for binding in files:
            require_keys(binding, {"path", "sha256"}, "evaluator.file")
            relative_path = string_value(binding, "path")
            if relative_path in actual:
                raise MetricError("evaluator.file_set_mismatch", relative_path)
            path = resolve_evidence_path(project_root, relative_path)
            digest = sha256_value(binding, "sha256")
            if sha256_file(path) != digest:
                raise MetricError("evaluator.manifest_mismatch", relative_path)
            actual[relative_path] = digest
        if set(actual) != set(EVALUATOR_FILES):
            raise MetricError("evaluator.file_set_mismatch", "unexpected file set")
        return sha256_file(manifest_path)
    except MetricError:
        raise
    except (
        StrictJsonError,
        CorpusError,
        OSError,
        TypeError,
        ValueError,
    ) as error:
        raise MetricError(
            "evaluator.manifest_mismatch", manifest_path.as_posix()
        ) from error


def _validate_dependencies(values: dict[str, JsonValue]) -> None:
    for name in values:
        expected = string_value(values, name)
        try:
            actual = importlib.metadata.version(name)
        except importlib.metadata.PackageNotFoundError as error:
            raise MetricError("evaluator.dependency", name) from error
        if actual != expected:
            raise MetricError(
                "evaluator.dependency",
                f"{name}: expected {expected}, got {actual}",
            )
