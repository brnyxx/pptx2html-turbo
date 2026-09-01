"""Real two-run reference determinism for the east-Asian font policy.

Exercises the same profile seeding the signed reference producer applies, so a
regression that stops seeding fails here rather than during a wave.
"""

from __future__ import annotations

import hashlib
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from collections.abc import Mapping
from pathlib import Path
from typing import ClassVar

from evaluate.multiformat_conformance_pdf import canonicalize_pdf_bytes
from evaluate.multiformat_east_asian_fonts import (
    load_policy,
    require_substitute,
    seed_profile,
)
from evaluate.tests.multiformat_east_asian_docx_fixture import (
    build_cjk_docx,
    convert_docx_to_doc,
)


@unittest.skipUnless(sys.platform == "darwin", "CoreText policy requires macOS")
@unittest.skipUnless(shutil.which("soffice"), "LibreOffice is not installed")
def raw_pdf(source: Path, workspace: Path, *, seeded: bool) -> bytes:
    policy = load_policy()
    profile = workspace / "profile"
    home = workspace / "home"
    output = workspace / "out"
    for directory in (profile, home, output):
        directory.mkdir(parents=True)
    if seeded:
        _ = seed_profile(profile, require_substitute(policy).family, policy)
    result = subprocess.run(
        (
            shutil.which("soffice") or "soffice",
            "--headless",
            "--nologo",
            "--nodefault",
            "--nolockcheck",
            "--nofirststartwizard",
            f"-env:UserInstallation={profile.resolve().as_uri()}",
            "--convert-to",
            "pdf",
            "--outdir",
            output.as_posix(),
            source.as_posix(),
        ),
        env={
            "HOME": home.as_posix(),
            "LANG": "C.UTF-8",
            "LC_ALL": "C.UTF-8",
            "TZ": "UTC",
            "PATH": "/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin",
        },
        capture_output=True,
        timeout=300,
        check=False,
    )
    if result.returncode != 0:
        raise AssertionError(f"LibreOffice failed: {result.stderr!r}")
    produced = output / f"{source.stem}.pdf"
    if not produced.is_file():
        raise AssertionError("LibreOffice produced no reference PDF")
    return produced.read_bytes()


def canonical_pdf(source: Path, workspace: Path, *, seeded: bool) -> bytes:
    """The artifact a wave actually admits: canonicalized reference bytes."""
    return canonicalize_pdf_bytes(raw_pdf(source, workspace, seeded=seeded))


def build_fixture(directory: Path, extension: str) -> Path:
    """Write a CJK DOCX, exporting a legacy DOC when that format is asked for."""
    docx = directory / "cjk.docx"
    docx.write_bytes(build_cjk_docx())
    if extension == "docx":
        return docx
    return convert_docx_to_doc(docx, directory)


class EastAsianFontReferenceDeterminismTests(unittest.TestCase):
    """Real two-run determinism for the reference LibreOffice invocation.

    This exercises the same seeding the signed reference producer applies, so a
    regression that stops seeding the profile fails here rather than during a
    wave.
    """

    maxDiff = None

    @staticmethod
    def _embedded_fonts(pdf: bytes) -> set[str]:
        return {
            name.decode("ascii")
            for name in re.findall(rb"/BaseFont\s*/[A-Z]{6}\+([A-Za-z0-9\-]+)", pdf)
        }

    @staticmethod
    def _font_fingerprint(pdf: bytes) -> tuple[tuple[str, ...], tuple[str, ...]]:
        """Order-independent identity of every embedded face and content stream.

        LibreOffice assigns PDF object ids to fonts in a run-dependent order,
        so two byte-different reference PDFs can carry identical fonts. That
        ordering is font-independent — an ASCII-only document using two bundled
        Latin faces reorders the same way — and the shared canonicalizer emits
        objects by id, so it does not normalize it. Comparing sorted content
        instead of raw bytes asserts the property this policy owns: the same
        faces with the same glyph subsets on every run.
        """
        faces = sorted(
            name.decode("ascii")
            for name in re.findall(rb"/BaseFont\s*/([A-Za-z0-9+\-]+)", pdf)
        )
        subsets = sorted(
            length.decode("ascii") for length in re.findall(rb"/Length1 (\d+)", pdf)
        )
        streams = sorted(
            hashlib.sha256(block).hexdigest()
            for block in re.findall(rb"stream\r?\n(.*?)endstream", pdf, re.DOTALL)
        )
        return tuple(faces) + tuple(subsets), tuple(streams)

    def test_seeded_reference_runs_are_byte_identical_for_docx_and_doc(self) -> None:
        for extension in ("docx", "doc"):
            with (
                self.subTest(format=extension),
                tempfile.TemporaryDirectory() as directory,
            ):
                root = Path(directory)
                source = build_fixture(root, extension)

                # When the reference policy is seeded on both runs.
                first = canonical_pdf(source, root / "run1", seeded=True)
                second = canonical_pdf(source, root / "run2", seeded=True)

                # Then both runs embed the same faces and the same glyph
                # subsets, so no run-dependent host face reached evidence.
                self.assertEqual(
                    self._embedded_fonts(first),
                    self._embedded_fonts(second),
                )
                self.assertEqual(
                    self._font_fingerprint(first),
                    self._font_fingerprint(second),
                )

    def test_unseeded_reference_is_the_regression_this_policy_removes(self) -> None:
        """Without seeding, repeated reference runs pick different host faces.

        This is the defect the policy exists to remove. It is asserted over
        enough runs that a chance agreement cannot make it pass.
        """
        # Given
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = build_fixture(root, "docx")

            # When the profile is left unseeded.
            unseeded = {
                frozenset(
                    self._embedded_fonts(
                        canonical_pdf(source, root / f"u{index}", seeded=False)
                    )
                )
                for index in range(6)
            }
            seeded = {
                frozenset(
                    self._embedded_fonts(
                        canonical_pdf(source, root / f"s{index}", seeded=True)
                    )
                )
                for index in range(6)
            }

        # Then only the seeded runs converge on one face set.
        self.assertGreater(len(unseeded), 1, "expected the unseeded regression")
        self.assertEqual(len(seeded), 1)

    def test_seeded_reference_pins_the_policy_substitute_face(self) -> None:
        # Given
        policy = load_policy()
        expected = require_substitute(policy).family.replace(" ", "")

        # When
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = build_fixture(root, "docx")
            pdf = canonical_pdf(source, root / "run", seeded=True)

        # Then the requested-but-absent family resolved to the pinned face.
        self.assertIn(expected, self._embedded_fonts(pdf))


@unittest.skipUnless(sys.platform == "darwin", "CoreText policy requires macOS")
@unittest.skipUnless(shutil.which("soffice"), "LibreOffice is not installed")
class ReferenceCanonicalByteDeterminismTests(unittest.TestCase):
    """Canonicalized reference bytes must be identical across many real runs.

    LibreOffice numbers indirect objects per run, so this fails against a
    canonicalizer that preserves input numbering. Eight runs make the
    two observed layouts overwhelmingly likely to both appear.
    """

    RUNS = 8

    def test_repeated_real_runs_canonicalize_to_identical_bytes(self) -> None:
        for extension in ("docx", "doc"):
            with (
                self.subTest(format=extension),
                tempfile.TemporaryDirectory() as directory,
            ):
                root = Path(directory)
                source = build_fixture(root, extension)

                # When the same source is converted repeatedly.
                digests = {
                    hashlib.sha256(
                        canonical_pdf(source, root / f"run{index}", seeded=True)
                    ).hexdigest()
                    for index in range(self.RUNS)
                }

                # Then every run yields one canonical artifact.
                self.assertEqual(len(digests), 1, f"{self.RUNS} runs disagreed")


@unittest.skipUnless(sys.platform == "darwin", "CoreText policy requires macOS")
@unittest.skipUnless(shutil.which("soffice"), "LibreOffice is not installed")
@unittest.skipUnless(shutil.which("pdftoppm"), "Poppler render is not installed")
@unittest.skipUnless(shutil.which("pdftotext"), "Poppler text is not installed")
class ReferenceCanonicalEquivalenceTests(unittest.TestCase):
    """Canonical numbering must not change what the document renders or says."""

    _METADATA = re.compile(
        rb'<meta name="(?:CreationDate|ModDate)" content="[^"]*"/>\s*'
    )
    _ENV: ClassVar[Mapping[str, str]] = {
        "LANG": "C.UTF-8",
        "LC_ALL": "C.UTF-8",
        "TZ": "UTC",
        "PATH": "/usr/bin:/bin:/opt/homebrew/bin",
    }

    def _pages(self, pdf: bytes, directory: Path, tag: str) -> list[str]:
        directory.mkdir(parents=True, exist_ok=True)
        source = directory / f"{tag}.pdf"
        source.write_bytes(pdf)
        _ = subprocess.run(
            (
                shutil.which("pdftoppm") or "pdftoppm",
                "-png",
                "-r",
                "144",
                source.as_posix(),
                (directory / f"{tag}-page").as_posix(),
            ),
            env=self._ENV,
            capture_output=True,
            timeout=300,
            check=True,
        )
        return [
            hashlib.sha256(page.read_bytes()).hexdigest()
            for page in sorted(directory.glob(f"{tag}-page*.png"))
        ]

    def _text(self, pdf: bytes, directory: Path, tag: str, *, layout: bool) -> str:
        directory.mkdir(parents=True, exist_ok=True)
        source = directory / f"{tag}-{layout}.pdf"
        source.write_bytes(pdf)
        arguments = [shutil.which("pdftotext") or "pdftotext"]
        if layout:
            arguments += ["-bbox-layout", "-enc", "UTF-8"]
        arguments += [source.as_posix(), "-"]
        completed = subprocess.run(
            tuple(arguments),
            env=self._ENV,
            capture_output=True,
            timeout=300,
            check=True,
        )
        return hashlib.sha256(self._METADATA.sub(b"", completed.stdout)).hexdigest()

    def test_canonical_form_preserves_render_text_and_geometry(self) -> None:
        for extension in ("docx", "doc"):
            with (
                self.subTest(format=extension),
                tempfile.TemporaryDirectory() as directory,
            ):
                root = Path(directory)
                source = build_fixture(root, extension)
                raw = raw_pdf(source, root / "run", seeded=True)
                canonical = canonicalize_pdf_bytes(raw)

                # Given canonicalization actually rewrote the bytes.
                self.assertNotEqual(raw, canonical)

                # Then rendering, geometry, and text are unchanged.
                self.assertEqual(
                    self._pages(raw, root / "raw", "raw"),
                    self._pages(canonical, root / "can", "can"),
                )
                for layout in (True, False):
                    self.assertEqual(
                        self._text(raw, root / "traw", "raw", layout=layout),
                        self._text(canonical, root / "tcan", "can", layout=layout),
                    )
