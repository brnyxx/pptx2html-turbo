from __future__ import annotations

import argparse
import shutil
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_corpus_types import CorpusError
from evaluate.multiformat_evidence import (
    EvidencePathError,
    resolve_evidence_path,
)
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    sha256_value,
    string_list,
    string_value,
)
from evaluate.multiformat_strict_json import read_strict_object


class OfficeInputBuildError(Exception):
    pass


def build_office_input_bundle(
    *,
    contract: Path,
    corpus_manifests: list[Path],
    evaluator_manifest: Path,
    oracle_lock: Path,
    output_dir: Path,
) -> Path:
    try:
        root = _prepare_output(output_dir)
        contract_values = read_strict_object(contract)
        required = set(string_list(contract_values, "required_formats"))
        corpora = _corpus_map(corpus_manifests)
        if set(corpora) != required:
            raise OfficeInputBuildError("required format corpus set differs")
        _copy_file(contract, root / "contract.json")
        _copy_file(evaluator_manifest, root / "evaluator-manifest.json")
        _copy_file(oracle_lock, root / "oracle-lock.json")
        files: list[dict[str, JsonValue]] = []
        source_ids: set[str] = set()
        for document_format, manifest in sorted(corpora.items()):
            target = root / "corpora" / document_format
            _copy_corpus(manifest.parent, target)
            copied_manifest = target / manifest.name
            values = read_strict_object(copied_manifest)
            tracks = object_value(values, "tracks")
            for track in ["conformance", "blind"]:
                for source in object_list(
                    object_value(tracks, track),
                    "items",
                    f"office.input.{track}",
                ):
                    source_id = string_value(source, "id")
                    if source_id in source_ids:
                        raise OfficeInputBuildError("office source id is duplicated")
                    source_ids.add(source_id)
                    source_path = resolve_evidence_path(
                        target,
                        string_value(source, "path"),
                    )
                    digest = sha256_value(source, "sha256")
                    if sha256_file(source_path) != digest:
                        raise OfficeInputBuildError("office source hash differs")
                    files.append(
                        {
                            "id": source_id,
                            "format": document_format,
                            "track": track,
                            "path": source_path.relative_to(root).as_posix(),
                            "sha256": digest,
                        }
                    )
        if not files:
            raise OfficeInputBuildError("office input has no positive sources")
        output = root / "office-input-manifest.json"
        write_canonical_json(
            output,
            {
                "schema_version": 1,
                "files": sorted(files, key=lambda item: str(item["id"])),
            },
        )
        return output
    except OfficeInputBuildError:
        raise
    except (
        EvidencePathError,
        CorpusError,
        OSError,
        TypeError,
        ValueError,
    ) as error:
        raise OfficeInputBuildError("office input bundle failed") from error


def _corpus_map(paths: list[Path]) -> dict[str, Path]:
    result: dict[str, Path] = {}
    for path in paths:
        resolved = path.resolve(strict=True)
        document_format = string_value(
            read_strict_object(resolved),
            "format",
        )
        if document_format in result:
            raise OfficeInputBuildError("corpus format is duplicated")
        result[document_format] = resolved
    return result


def _prepare_output(output_dir: Path) -> Path:
    if output_dir.exists() and any(output_dir.iterdir()):
        raise OfficeInputBuildError("office input output must be empty")
    output_dir.mkdir(parents=True, exist_ok=True)
    return output_dir.resolve(strict=True)


def _copy_corpus(source: Path, destination: Path) -> None:
    if any(item.is_symlink() for item in source.rglob("*")):
        raise OfficeInputBuildError("corpus contains a symlink")
    shutil.copytree(source, destination)


def _copy_file(source: Path, destination: Path) -> None:
    resolved = source.resolve(strict=True)
    shutil.copy2(resolved, destination)
    if sha256_file(destination) != sha256_file(resolved):
        raise OfficeInputBuildError("office input copy drifted")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Build a frozen seven-format Windows Office input bundle.",
    )
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument(
        "--corpus-manifest",
        type=Path,
        action="append",
        default=[],
    )
    parser.add_argument("--evaluator-manifest", type=Path, required=True)
    parser.add_argument("--oracle-lock", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        build_office_input_bundle(
            contract=arguments.contract,
            corpus_manifests=arguments.corpus_manifest,
            evaluator_manifest=arguments.evaluator_manifest,
            oracle_lock=arguments.oracle_lock,
            output_dir=arguments.output_dir,
        )
    except OfficeInputBuildError as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
