from pathlib import Path
import unittest

import document2html


FIXTURE = (
    Path(__file__).resolve().parents[2]
    / "pptx2html-cli"
    / "tests"
    / "fixtures"
    / "single-slide.pptx"
)


class InstalledRuntimeTests(unittest.TestCase):
    def test_detects_and_converts_pptx_through_installed_module(self) -> None:
        data = FIXTURE.read_bytes()

        self.assertEqual(document2html.detect_format(data, FIXTURE.name), "pptx")
        self.assertEqual(
            document2html.supported_formats(),
            ["pptx", "docx", "doc", "xlsx", "xls", "ppt", "pdf"],
        )
        result = document2html.convert_bytes(data, FIXTURE.name)
        self.assertEqual(result.format, "pptx")
        self.assertEqual(result.unit_count, 1)
        self.assertIn("<!DOCTYPE html>", result.html)


if __name__ == "__main__":
    unittest.main()
