from __future__ import annotations

import hashlib
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_corpus_sources import validate_source
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_legacy_ppt_canonicalizer import (
    LegacyPptCanonicalizationError,
    canonicalize_legacy_ppt_bytes,
)
from evaluate.tests.multiformat_legacy_ppt_fixture import (
    USER_EDIT,
    current_user_target_type,
    extract_package_zips,
    make_legacy_ppt_fixture,
    ppt_offsets,
)


class MultiFormatLegacyPptCanonicalizerTests(unittest.TestCase):
    def test_timestamp_variants_are_identical_idempotent_valid_ppts(self) -> None:
        early = make_legacy_ppt_fixture(0x5021, 0x1882)
        late = make_legacy_ppt_fixture(0x579F, 0xBF7D)
        self.assertNotEqual(early.value, late.value)

        canonical_early = canonicalize_legacy_ppt_bytes(early.value)
        canonical_late = canonicalize_legacy_ppt_bytes(late.value)

        self.assertEqual(canonical_early, canonical_late)
        self.assertEqual(
            canonicalize_legacy_ppt_bytes(canonical_early),
            canonical_early,
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            path = root / "canonical.ppt"
            _ = path.write_bytes(canonical_early)
            _ = validate_source(
                {
                    "id": "canonical-ppt",
                    "path": path.name,
                    "sha256": hashlib.sha256(canonical_early).hexdigest(),
                },
                root,
                DocumentFormat.PPT,
                require_valid_format=True,
            )
        canonical_packages = extract_package_zips(canonical_early)
        for original, package in zip(
            early.package_zips,
            canonical_packages,
            strict=True,
        ):
            self.assertEqual(_zip_timestamps(package), {(0, 0x21)})
            self.assertEqual(package, _with_canonical_timestamps(original))
        persists, edits, current = ppt_offsets(canonical_early)
        self.assertEqual(len(persists), 3)
        self.assertEqual(edits[0], 0)
        self.assertEqual(current_user_target_type(canonical_early), USER_EDIT)
        self.assertTrue(all(offset < 4_608 for offset in persists + edits + (current,)))

    def test_uncompressed_instance_zero_storage_is_preserved(self) -> None:
        fixture = make_legacy_ppt_fixture(
            0x579F,
            0xBF7D,
            compressed=False,
        )
        self.assertEqual(canonicalize_legacy_ppt_bytes(fixture.value), fixture.value)

    def test_non_chart_package_zip_is_preserved(self) -> None:
        fixture = make_legacy_ppt_fixture(
            0x579F,
            0xBF7D,
            chart_package=False,
        )
        self.assertEqual(canonicalize_legacy_ppt_bytes(fixture.value), fixture.value)

    def test_malformed_package_zip_fails_closed(self) -> None:
        fixture = make_legacy_ppt_fixture(
            0x579F,
            0xBF7D,
            malformed_package=True,
        )
        with self.assertRaises(LegacyPptCanonicalizationError):
            _ = canonicalize_legacy_ppt_bytes(fixture.value)

    def test_non_package_ole_record_is_preserved(self) -> None:
        fixture = make_legacy_ppt_fixture(
            0x579F,
            0xBF7D,
            include_packages=False,
        )
        self.assertEqual(canonicalize_legacy_ppt_bytes(fixture.value), fixture.value)

    def test_malformed_input_fails_closed(self) -> None:
        fixture = make_legacy_ppt_fixture(0x579F, 0xBF7D)
        with self.assertRaises(LegacyPptCanonicalizationError):
            _ = canonicalize_legacy_ppt_bytes(fixture.value[:-512])


def _with_canonical_timestamps(value: bytes) -> bytes:
    result = bytearray(value)
    for signature, time_offset, date_offset in (
        (b"PK\x03\x04", 10, 12),
        (b"PK\x01\x02", 12, 14),
    ):
        start = 0
        while (start := value.find(signature, start)) >= 0:
            result[start + time_offset : start + time_offset + 2] = b"\x00\x00"
            result[start + date_offset : start + date_offset + 2] = b"\x21\x00"
            start += 4
    return bytes(result)


def _zip_timestamps(value: bytes) -> set[tuple[int, int]]:
    timestamps: set[tuple[int, int]] = set()
    for signature, time_offset, date_offset in (
        (b"PK\x03\x04", 10, 12),
        (b"PK\x01\x02", 12, 14),
    ):
        start = 0
        while True:
            start = value.find(signature, start)
            if start < 0:
                break
            timestamps.add(
                (
                    int.from_bytes(
                        value[start + time_offset : start + time_offset + 2], "little"
                    ),
                    int.from_bytes(
                        value[start + date_offset : start + date_offset + 2], "little"
                    ),
                )
            )
            start += 4
    return timestamps


if __name__ == "__main__":
    _ = unittest.main()
