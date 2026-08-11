import hashlib
import os
import subprocess
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
from xml.etree import ElementTree

from evaluate.create_adjustment_benchmark_deck import canonicalize_pptx


ROOT = Path(__file__).resolve().parents[2]
GENERATOR = ROOT / "evaluate" / "create_adjustment_benchmark_deck.py"
FIXED_TIME = (1980, 1, 1, 0, 0, 0)


class AdjustmentBenchmarkDeckTests(unittest.TestCase):
    def test_canonicalization_preserves_every_member_payload(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "fixture.pptx"
            expected = {
                "z.xml": b"<z />",
                "_rels/.rels": b'<Relationships xmlns="urn:test" />',
            }
            with zipfile.ZipFile(path, "w") as archive:
                for name, payload in expected.items():
                    archive.writestr(name, payload)

            canonicalize_pptx(path)

            with zipfile.ZipFile(path) as archive:
                self.assertEqual(archive.namelist(), sorted(expected))
                self.assertEqual(
                    {name: archive.read(name) for name in archive.namelist()},
                    expected,
                )
                self.assertTrue(
                    all(info.date_time == FIXED_TIME for info in archive.infolist())
                )

    def test_clean_runs_are_raw_byte_deterministic_and_valid(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            outputs = []
            for name, timezone in (("utc", "UTC"), ("seoul", "Asia/Seoul")):
                output = root / name
                environment = os.environ.copy()
                environment["TZ"] = timezone
                result = subprocess.run(
                    [
                        sys.executable,
                        str(GENERATOR),
                        "--scenario",
                        "curved-arrows",
                        "--output-dir",
                        str(output),
                    ],
                    cwd=ROOT,
                    env=environment,
                    capture_output=True,
                    text=True,
                    check=False,
                )
                self.assertEqual(result.returncode, 0, result.stderr)
                outputs.append(output)

            first = (outputs[0] / "curved_arrows.pptx").read_bytes()
            second = (outputs[1] / "curved_arrows.pptx").read_bytes()
            self.assertEqual(
                hashlib.sha256(first).digest(), hashlib.sha256(second).digest()
            )
            self.assertEqual(first, second)

            with zipfile.ZipFile(outputs[0] / "curved_arrows.pptx") as archive:
                self.assertEqual(archive.namelist(), sorted(archive.namelist()))
                self.assertTrue(
                    all(info.date_time == FIXED_TIME for info in archive.infolist())
                )
                for member in archive.namelist():
                    if member.endswith((".xml", ".rels")):
                        ElementTree.fromstring(archive.read(member))


if __name__ == "__main__":
    unittest.main()
