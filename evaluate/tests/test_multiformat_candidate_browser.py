import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_candidate_browser import (
    CandidateCaptureError,
    capture_html_units,
)
from evaluate.multiformat_corpus_types import DocumentFormat
from evaluate.multiformat_inventory import parse_inventory
from evaluate.multiformat_visual_metrics import png_dimensions


class MultiFormatCandidateBrowserTests(unittest.TestCase):
    def test_paged_html_captures_complete_units_and_semantic_inventory(self) -> None:
        html = """
        <html><body>
          <div id="page1-div" style="position:relative;width:300px;height:200px;background:#fff">
            <span style="position:absolute;left:10px;top:10px;font:16px Arial">Hello</span>
            <a href="https://example.test/docs" style="position:absolute;left:10px;top:40px">Link</a>
            <img alt="logo" src="data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=" style="position:absolute;left:80px;top:20px;width:20px;height:20px">
            <svg style="position:absolute;left:120px;top:20px;width:20px;height:20px"><rect width="20" height="20"/></svg>
          </div>
          <div id="page2-div" style="position:relative;width:300px;height:200px;background:#fff">
            <div data-cell-coordinate="A1" data-worksheet="Sheet1" style="position:absolute;left:5px;top:5px">42</div>
          </div>
        </body></html>
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            output = Path(temp_dir)

            result = capture_html_units(
                html,
                DocumentFormat.DOCX,
                ("unit-1", "unit-2"),
                output,
            )

            self.assertEqual(len(result.units), 2)
            self.assertEqual(png_dimensions(result.units[0].png), (300, 200))
            first = parse_inventory(result.units[0].inventory, "unit-1")
            second = parse_inventory(result.units[1].inventory, "unit-2")
            self.assertIn("Hello", [item.value for item in first.texts])
            self.assertIn("Link", [item.value for item in first.texts])
            self.assertEqual(
                {item.object_type for item in first.objects},
                {"image", "link", "svg"},
            )
            self.assertIn("42", [item.value for item in second.texts])
            self.assertEqual(result.external_requests, ())

    def test_spreadsheet_capture_requires_explicit_cell_coordinates(self) -> None:
        html = """
        <html><body>
          <div id="page1-div" style="position:relative;width:300px;height:200px">
            <div data-cell-coordinate="A1" data-worksheet="Sheet1">42</div>
            <div data-cell-coordinate="B2">must not invent worksheet</div>
          </div>
        </body></html>
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            result = capture_html_units(
                html,
                DocumentFormat.XLSX,
                ("sheet-unit-1",),
                Path(temp_dir),
            )

            inventory = parse_inventory(
                result.units[0].inventory,
                "sheet-unit-1",
            )
            self.assertEqual(inventory.texts, ())
            self.assertEqual(len(inventory.cells), 1)
            self.assertEqual(inventory.cells[0].coordinate, "A1")
            self.assertEqual(inventory.cells[0].displayed_value, "42")

    def test_presentation_capture_renders_exact_960_by_540(self) -> None:
        html = """
        <html><body>
          <div class="slide" id="slide-1" data-slide="1"
               style="position:relative;width:960px;height:540px;background:#fff">
            <span style="position:absolute;left:100px;top:500px;font:20px Arial">Bottom</span>
          </div>
        </body></html>
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            result = capture_html_units(
                html,
                DocumentFormat.PPTX,
                ("slide-1",),
                Path(temp_dir),
            )

            self.assertEqual(png_dimensions(result.units[0].png), (960, 540))
            inventory = parse_inventory(result.units[0].inventory, "slide-1")
            self.assertLess(inventory.texts[0].box.y, 540)

    def test_presentation_capture_rejects_noncanonical_dimensions(self) -> None:
        html = """
        <html><body>
          <div class="slide" id="slide-1" data-slide="1"
               style="position:relative;width:960px;height:720px;background:#fff">
          </div>
        </body></html>
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            with self.assertRaisesRegex(
                CandidateCaptureError,
                "presentation dimensions",
            ):
                capture_html_units(
                    html,
                    DocumentFormat.PPTX,
                    ("slide-1",),
                    Path(temp_dir),
                )

    def test_legacy_ppt_uses_native_page_container_as_slide(self) -> None:
        html = """
        <html><body>
          <div id="page1-div"
               style="position:relative;width:960px;height:540px;background:#fff">
            <span>Legacy slide</span>
          </div>
        </body></html>
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            result = capture_html_units(
                html,
                DocumentFormat.PPT,
                ("slide-page-1",),
                Path(temp_dir),
            )

            self.assertEqual(png_dimensions(result.units[0].png), (960, 540))

    def test_external_request_attempt_fails_closed(self) -> None:
        html = """
        <html><body>
          <div id="page1-div" style="width:300px;height:200px">
            <img src="https://example.invalid/tracker.png">
          </div>
        </body></html>
        """
        with tempfile.TemporaryDirectory() as temp_dir:
            with self.assertRaisesRegex(
                CandidateCaptureError,
                "network request",
            ):
                capture_html_units(
                    html,
                    DocumentFormat.PDF,
                    ("unit-1",),
                    Path(temp_dir),
                )
