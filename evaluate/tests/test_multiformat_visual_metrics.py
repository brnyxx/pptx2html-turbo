import struct
import tempfile
import unittest
import zlib
from pathlib import Path

from evaluate.multiformat_inventory_types import Box
from evaluate.multiformat_metric_types import MetricError
from evaluate.multiformat_visual_metrics import score_visual


class MultiFormatVisualMetricTests(unittest.TestCase):
    def test_identical_native_pngs_score_100_without_resize(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            image = root / "same.png"
            self._write_png(image, 192, 192, (40, 80, 120))

            result = score_visual(image, image, "#ffffff", ())

            self.assertEqual(result.ms_ssim, 100)
            self.assertEqual(result.active_tile_ssim, 100)
            self.assertEqual(result.color_similarity, 100)
            self.assertEqual(result.edge_f1, 100)

    def test_different_images_reduce_fixed_visual_metrics(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            reference = root / "reference.png"
            candidate = root / "candidate.png"
            self._write_png(reference, 192, 192, (0, 0, 0))
            self._write_png(candidate, 192, 192, (255, 255, 255))

            result = score_visual(
                reference,
                candidate,
                "#ffffff",
                (Box(0, 0, 192, 192),),
            )

            self.assertLess(result.ms_ssim, 100)
            self.assertLess(result.active_tile_ssim, 100)
            self.assertLess(result.color_similarity, 100)

    def test_dimension_mismatch_is_rejected_instead_of_resized(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            reference = root / "reference.png"
            candidate = root / "candidate.png"
            self._write_png(reference, 192, 192, (0, 0, 0))
            self._write_png(candidate, 193, 192, (0, 0, 0))

            with self.assertRaisesRegex(MetricError, "artifact.dimension"):
                score_visual(reference, candidate, "#ffffff", ())

    @staticmethod
    def _write_png(
        path: Path,
        width: int,
        height: int,
        color: tuple[int, int, int],
    ) -> None:
        row = b"\x00" + bytes(color) * width
        raw = row * height

        def chunk(kind: bytes, data: bytes) -> bytes:
            body = kind + data
            return (
                struct.pack(">I", len(data))
                + body
                + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
            )

        path.write_bytes(
            b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(raw))
            + chunk(b"IEND", b"")
        )
