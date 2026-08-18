import tempfile
import unittest
from pathlib import Path

from PIL import Image

from evaluate.candidate_render import render_html_to_pngs


class CandidateRenderDimensionsTests(unittest.TestCase):
    def test_scales_intrinsic_slide_to_exact_output_dimensions(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            html = root / "sample.html"
            html.write_text(
                """
                <!doctype html>
                <style>
                  html, body { margin: 0; }
                  .slide { width: 1280px; height: 720px; background: #123456; }
                </style>
                <div class="slide"></div>
                """,
                encoding="utf-8",
            )

            paths = render_html_to_pngs(
                html,
                root / "candidates",
                output_width=960,
                output_height=540,
            )

            self.assertEqual(len(paths), 1)
            with Image.open(paths[0]) as image:
                self.assertEqual(image.size, (960, 540))
                self.assertEqual(image.convert("RGB").getpixel((959, 539)), (18, 52, 86))


if __name__ == "__main__":
    unittest.main()
