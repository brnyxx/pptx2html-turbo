import binascii
import hashlib
import json
import posixpath
import struct
import subprocess
import sys
import unittest
import zipfile
import zlib
from pathlib import Path
from xml.etree import ElementTree


ROOT = Path(__file__).resolve().parents[2]
GENERATOR = ROOT / "evaluate" / "create_completion_decks.py"
DECKS = tuple(
    "patterns picture-bullets table-styles actions notes-comments reflection-3d media timing-transitions charts fallback-domains".split()
)
REQUIRED_IDS = set(
    """adjustment-basic adjustment-arrows adjustment-remaining custom-geometry-unknown-formula
    pattern-fill-known pattern-fill-unknown picture-bullet-embedded picture-bullet-missing
    table-style-regions table-style-missing action-external action-internal action-unsafe
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


def contract(root: Path) -> Path:
    names = [
        "roundRect",
        "rightArrow",
        "wave",
        *(f"synthetic{i:03}" for i in range(184)),
    ]
    keys = {"roundRect": "adjBasic", "rightArrow": "adjArrow", "wave": "adjWave"}
    template = {
        "default_formula": "val 11111",
        "source_status": "available",
        "range_status": "explicit",
        "constraints": [
            {
                "handle": "xy",
                "axis": "x",
                "minimum_formula": "val 100",
                "maximum_formula": "val 200",
            }
        ],
    }
    rows = [
        {
            "name": name,
            "source_status": "available",
            "adjustments": [{**template, "name": keys[name]}] if name in keys else [],
            "preservation": {"fidelity": "fixture", "reason": "synthetic"},
        }
        for name in names
    ]
    path = root / "preset-adjustments.json"
    path.write_text(
        json.dumps(
            {
                "official_preset_names": names,
                "official_preset_names_sha256": "synthetic",
                "dispatcher_aliases": {},
                "presets": rows,
            }
        ),
        encoding="utf-8",
    )
    return path


def run_generator(output: Path, source: Path) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [
            sys.executable,
            str(GENERATOR),
            "--output-dir",
            str(output),
            "--adjustment-manifest",
            str(source),
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )


def generate(case: unittest.TestCase, output: Path, source: Path) -> None:
    result = run_generator(output, source)
    if result.returncode:
        case.fail(result.stderr)


def assert_stimuli(case: unittest.TestCase, root: Path) -> None:
    for feature in json.loads((root / "manifest.json").read_text())["features"]:
        stimulus = feature["stimulus"]
        with zipfile.ZipFile(root / feature["deck"]) as archive:
            case.assertIn(stimulus["part"], archive.namelist(), feature["id"])
            case.assertIn(
                stimulus["token"].encode(),
                archive.read(stimulus["part"]),
                feature["id"],
            )


def remove_token(deck: Path, stimulus: dict[str, str]) -> None:
    copy = deck.with_suffix(".copy")
    with zipfile.ZipFile(deck) as source, zipfile.ZipFile(copy, "w") as target:
        for info in source.infolist():
            payload = source.read(info.filename)
            if info.filename == stimulus["part"]:
                payload = payload.replace(stimulus["token"].encode(), b"")
            target.writestr(info, payload)
    deck.write_bytes(copy.read_bytes())


def assert_relationship_closure(
    case: unittest.TestCase, archive: zipfile.ZipFile
) -> None:
    names = set(archive.namelist())
    ns = {"r": "http://schemas.openxmlformats.org/package/2006/relationships"}
    for part in (name for name in names if name.endswith(".rels")):
        source = "" if part == "_rels/.rels" else part.replace("/_rels/", "/")[:-5]
        for rel in ElementTree.fromstring(archive.read(part)).findall(
            "r:Relationship", ns
        ):
            if rel.get("TargetMode") == "External" or rel.get("Id") == "rIdMissing":
                continue
            target = posixpath.normpath(
                posixpath.join(posixpath.dirname(source), rel.get("Target", ""))
            )
            case.assertIn(target, names, f"{part}:{rel.get('Id')}")


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
