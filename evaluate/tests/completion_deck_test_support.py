import binascii
import hashlib
import struct
import subprocess
import sys
import unittest
import zlib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
GENERATOR = ROOT / "evaluate" / "create_completion_decks.py"
CANONICAL_MANIFEST = ROOT / "evaluate" / "preset_adjustments.json"
DECKS = tuple(
    "patterns picture-bullets table-styles actions rtl-text handout-master extensions bibliography additional-characteristics custom-xml thumbnail theme-override slide-synchronization notes-comments reflection-3d media timing-transitions charts fallback-domains".split()
)
REQUIRED_IDS = set(
    """adjustment-basic adjustment-arrows adjustment-remaining custom-geometry-unknown-formula
    pattern-fill-known pattern-fill-unknown picture-bullet-embedded picture-bullet-missing
    table-style-regions table-style-missing action-external action-internal action-unsafe
    action-table-frame action-group rtl-text handout-master extensions bibliography additional-characteristics custom-xml thumbnail theme-override slide-synchronization
    notes-slide comments-legacy comments-modern comment-author-missing reflection
    drawingml-3d-fallback media-audio media-video media-unsupported transition-cut transition-fade
    animation-bounded animation-unsupported chart-direct chart-preview-fallback chart-placeholder
    fallback-smartart fallback-ole fallback-math fallback-alternate-content fallback-unknown-extension""".split()
)
COMMON_PARTS = set(
    """[Content_Types].xml _rels/.rels ppt/presentation.xml ppt/_rels/presentation.xml.rels
    ppt/presProps.xml ppt/slideMasters/slideMaster1.xml ppt/slideMasters/_rels/slideMaster1.xml.rels
    ppt/slideLayouts/slideLayout1.xml ppt/slideLayouts/_rels/slideLayout1.xml.rels
    ppt/theme/theme1.xml ppt/slides/slide1.xml ppt/slides/_rels/slide1.xml.rels""".split()
)


def contract(_root: Path) -> Path:
    return CANONICAL_MANIFEST


def copy_contract(root: Path) -> Path:
    path = root / "preset-adjustments.json"
    path.write_bytes(CANONICAL_MANIFEST.read_bytes())
    return path


def run_generator(
    output: Path,
    source: Path | None = None,
    *,
    module: bool = False,
) -> subprocess.CompletedProcess[str]:
    command = (
        [sys.executable, "-m", "evaluate.create_completion_decks"]
        if module
        else [sys.executable, str(GENERATOR)]
    )
    command.extend(("--output-dir", str(output)))
    if source is not None:
        command.extend(("--adjustment-manifest", str(source)))
    return subprocess.run(
        command,
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )


def generate(case: unittest.TestCase, output: Path, source: Path) -> None:
    result = run_generator(output, source)
    if result.returncode:
        case.fail(result.stderr)


def assert_png(case: unittest.TestCase, payload: bytes) -> None:
    case.assertEqual(payload[:8], b"\x89PNG\r\n\x1a\n")
    position, image_data = 8, bytearray()
    while position < len(payload):
        length = struct.unpack(">I", payload[position : position + 4])[0]
        kind, data = (
            payload[position + 4 : position + 8],
            payload[position + 8 : position + 8 + length],
        )
        checksum = struct.unpack(
            ">I", payload[position + 8 + length : position + 12 + length]
        )[0]
        case.assertEqual(checksum, binascii.crc32(kind + data) & 0xFFFFFFFF)
        if kind == b"IDAT":
            image_data.extend(data)
        position += 12 + length
    case.assertEqual(zlib.decompress(image_data), b"\x00\xff\x00\x00\xff")


def tree_hashes(root: Path) -> dict[str, str]:
    return {
        path.name: hashlib.sha256(path.read_bytes()).hexdigest()
        for path in sorted(root.iterdir())
    }
