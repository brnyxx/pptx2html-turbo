from __future__ import annotations

import argparse
import shutil
from pathlib import Path

from evaluate.multiformat_candidate_artifacts import write_canonical_json
from evaluate.multiformat_corpus_items import object_list
from evaluate.multiformat_evidence import (
    EvidencePathError,
    resolve_evidence_path,
)
from evaluate.multiformat_public_pool import (
    PublicPoolError,
    validate_public_pool,
)
from evaluate.multiformat_schema import (
    JsonValue,
    object_value,
    sha256_file,
    sha256_value,
    string_value,
)
from evaluate.multiformat_strict_json import (
    StrictJsonError,
    read_strict_object,
)


class PublicPoolInputError(Exception):
    pass


def build_public_pool_input(
    config: Path,
    pool_manifest: Path,
    output_dir: Path,
) -> Path:
    if output_dir.exists():
        raise PublicPoolInputError("public pool input output already exists")
    try:
        validate_public_pool(config, pool_manifest)
        output_dir.mkdir(parents=True)
        pool_root = pool_manifest.resolve(strict=True).parent
        values = read_strict_object(pool_manifest)
        source_ids: set[str] = set()
        files: list[dict[str, JsonValue]] = []
        for document_format, format_value in object_value(
            values,
            "formats",
        ).items():
            if not isinstance(format_value, dict):
                raise PublicPoolInputError("public pool format is invalid")
            for source in object_list(
                format_value,
                "sources",
                "public.pool.input.sources",
            ):
                source_id = string_value(source, "id")
                if source_id in source_ids:
                    raise PublicPoolInputError("public pool source id is duplicated")
                source_ids.add(source_id)
                relative_path = string_value(source, "path")
                source_path = resolve_evidence_path(pool_root, relative_path)
                digest = sha256_value(source, "sha256")
                destination = output_dir / relative_path
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source_path, destination)
                if sha256_file(destination) != digest:
                    raise PublicPoolInputError("public pool source copy drifted")
                files.append(
                    {
                        "id": source_id,
                        "format": document_format,
                        "track": "blind",
                        "path": relative_path,
                        "sha256": digest,
                    }
                )
        if not files:
            raise PublicPoolInputError("public pool input has no sources")
        copied_config = output_dir / "public-pool-config.json"
        copied_pool = output_dir / "public-pool.json"
        shutil.copy2(config, copied_config)
        shutil.copy2(pool_manifest, copied_pool)
        output = output_dir / "office-input-manifest.json"
        write_canonical_json(
            output,
            {
                "schema_version": 1,
                "public_pool_config_sha256": sha256_file(copied_config),
                "public_pool_manifest_sha256": sha256_file(copied_pool),
                "files": sorted(files, key=lambda item: str(item["id"])),
            },
        )
        _validate_file_set(output_dir, files)
        return output
    except PublicPoolInputError:
        if output_dir.exists():
            shutil.rmtree(output_dir)
        raise
    except (
        EvidencePathError,
        OSError,
        PublicPoolError,
        StrictJsonError,
        TypeError,
        ValueError,
    ) as error:
        if output_dir.exists():
            shutil.rmtree(output_dir)
        raise PublicPoolInputError("public pool input build failed") from error


def _validate_file_set(
    root: Path,
    files: list[dict[str, JsonValue]],
) -> None:
    expected = {
        root / "office-input-manifest.json",
        root / "public-pool-config.json",
        root / "public-pool.json",
        *(root / string_value(item, "path") for item in files),
    }
    actual = {path for path in root.rglob("*") if path.is_file() or path.is_symlink()}
    if actual != expected:
        raise PublicPoolInputError("public pool input file set differs")


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Build a frozen Office capture input from a public pool.",
    )
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--pool-manifest", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        build_public_pool_input(
            arguments.config,
            arguments.pool_manifest,
            arguments.output_dir,
        )
    except PublicPoolInputError as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
