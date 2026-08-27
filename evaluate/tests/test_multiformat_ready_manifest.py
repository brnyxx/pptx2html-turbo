from __future__ import annotations

import json
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path
from typing import ClassVar, cast

from evaluate.multiformat_corpus import validate_corpus_manifest
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_ready_manifest import (
    ReadyManifestError,
    ReadyManifestFailure,
    build_format_manifest,
)
from evaluate.multiformat_ready_types import (
    ReadyBlind,
    ReadyConformance,
    ReadySource,
    ReadySourceSet,
)
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.tests.multiformat_ready_manifest_fixture import (
    CONTRACT,
    make_manifest_sources,
)

_FIXTURE_ROOT = tempfile.TemporaryDirectory()
_SOURCES = make_manifest_sources(Path(_FIXTURE_ROOT.name))


class MultiFormatReadyManifestTests(unittest.TestCase):
    root: ClassVar[tempfile.TemporaryDirectory[str]] = _FIXTURE_ROOT
    sources: ClassVar[ReadySourceSet] = _SOURCES

    def test_all_seven_manifests_are_exact_and_independently_valid(self) -> None:
        digest = sha256_file(CONTRACT)
        for document_format in DocumentFormat:
            with self.subTest(document_format=document_format):
                manifest = build_format_manifest(digest, document_format, self.sources)
                path = Path(self.root.name) / document_format.value / "manifest.json"
                _ = path.write_text(json.dumps(manifest), encoding="utf-8")
                result = validate_corpus_manifest(CONTRACT, path)
                self.assertEqual(
                    (
                        result.conformance_units,
                        result.blind_files,
                        result.security_cases,
                    ),
                    (100, 75, 10),
                )
                tracks = cast(dict[str, JsonValue], manifest["tracks"])
                track = cast(dict[str, JsonValue], tracks["conformance"])
                items = cast(list[dict[str, JsonValue]], track["items"])
                for item in items:
                    units = cast(list[dict[str, JsonValue]], item["units"])
                    self.assertEqual(units[0]["ordinal"], 1)
                    self.assertEqual(units[0]["id"], item["id"])
                    self.assertEqual(
                        units[0]["secondary_features"],
                        [
                            next(
                                source.details.feature_seed
                                for source in self.sources.sources
                                if source.source_id == item["id"]
                                and isinstance(source.details, ReadyConformance)
                            )
                        ],
                    )

    def test_output_is_independent_of_input_order(self) -> None:
        digest = sha256_file(CONTRACT)
        reversed_set = ReadySourceSet(
            tuple(reversed(self.sources.sources)),
            tuple(reversed(self.sources.supports)),
        )
        for document_format in DocumentFormat:
            self.assertEqual(
                build_format_manifest(digest, document_format, self.sources),
                build_format_manifest(digest, document_format, reversed_set),
            )

    def test_paired_legacy_conformance_rejects_multiple_units(self) -> None:
        source = next(
            item
            for item in self.sources.sources
            if item.document_format is DocumentFormat.DOC
            and isinstance(item.details, ReadyConformance)
            and item.details.primary_stratum == "paired-legacy"
        )
        values = _replace(
            self.sources.sources,
            source,
            replace(source, unit_count=2),
        )

        with self.assertRaises(ReadyManifestError) as raised:
            build_format_manifest(
                sha256_file(CONTRACT),
                DocumentFormat.DOC,
                ReadySourceSet(values, self.sources.supports),
            )
        self.assertEqual(raised.exception.failure, ReadyManifestFailure.CONFORMANCE)

    def test_rejects_invalid_inventory_with_typed_failures(self) -> None:
        target = DocumentFormat.PPTX
        any_source = next(
            item for item in self.sources.sources if item.document_format is target
        )
        blind = next(
            item
            for item in self.sources.sources
            if item.document_format is target and isinstance(item.details, ReadyBlind)
        )
        conformance = next(
            item
            for item in self.sources.sources
            if item.document_format is target
            and isinstance(item.details, ReadyConformance)
        )
        invalid_background = replace(
            blind, details=replace(blind.details, background="dark")
        )
        invalid_stratum = replace(
            conformance,
            details=replace(conformance.details, primary_stratum="unknown"),
        )
        mutations = {
            ReadyManifestFailure.COUNT: tuple(
                item for item in self.sources.sources if item is not any_source
            ),
            ReadyManifestFailure.BACKGROUND: _replace(
                self.sources.sources, blind, invalid_background
            ),
            ReadyManifestFailure.DIGEST: _replace(
                self.sources.sources,
                blind,
                replace(blind, source_sha256=conformance.source_sha256),
            ),
            ReadyManifestFailure.STRATUM: _replace(
                self.sources.sources, conformance, invalid_stratum
            ),
        }
        for failure, values in mutations.items():
            with (
                self.subTest(failure=failure),
                self.assertRaises(ReadyManifestError) as raised,
            ):
                _ = build_format_manifest(
                    sha256_file(CONTRACT),
                    target,
                    ReadySourceSet(values, self.sources.supports),
                )
            self.assertEqual(raised.exception.failure, failure)

    def test_rejects_non_owner_prefixed_support_binding(self) -> None:
        support = next(
            item
            for item in self.sources.supports
            if item.owner_format is DocumentFormat.DOC
        )
        invalid = replace(support, support_id=support.modern_case_id)
        supports = tuple(
            invalid if item is support else item for item in self.sources.supports
        )
        with self.assertRaises(ReadyManifestError) as raised:
            _ = build_format_manifest(
                sha256_file(CONTRACT),
                DocumentFormat.DOC,
                ReadySourceSet(self.sources.sources, supports),
            )
        self.assertEqual(raised.exception.failure, ReadyManifestFailure.SUPPORT)


def _replace(
    values: tuple[ReadySource, ...], old: ReadySource, new: ReadySource
) -> tuple[ReadySource, ...]:
    return tuple(new if item is old else item for item in values)


if __name__ == "__main__":
    _ = unittest.main()
