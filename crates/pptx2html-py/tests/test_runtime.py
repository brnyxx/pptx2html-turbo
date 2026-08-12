import tempfile
import unittest
import zipfile
from importlib import metadata
from pathlib import Path

import pptx2html


class PythonBindingRuntimeTests(unittest.TestCase):
    def test_runtime_exports_match_public_stub_names(self) -> None:
        self.assertTrue(hasattr(pptx2html, "PresentationInfo"))
        self.assertTrue(hasattr(pptx2html, "ConversionResult"))
        self.assertTrue(hasattr(pptx2html, "ConversionDiagnostic"))
        self.assertTrue(hasattr(pptx2html, "DiagnosticLocation"))
        self.assertTrue(hasattr(pptx2html, "DiagnosticPosition"))
        self.assertTrue(hasattr(pptx2html, "DiagnosticSize"))
        self.assertTrue(hasattr(pptx2html, "UnresolvedElement"))

        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "sample.pptx"
            self._write_minimal_pptx(path)

            info = pptx2html.get_info(str(path))
            result = pptx2html.convert_with_metadata(str(path), slides=[1])

            self.assertEqual(type(info).__name__, "PresentationInfo")
            self.assertEqual(type(result).__name__, "ConversionResult")
            self.assertEqual(result.slide_count, 1)
            self.assertIsInstance(result.diagnostics, list)
            self.assertEqual(result.diagnostics_json, "[]")
            self.assertIsInstance(result.unresolved_elements, list)

    def test_typed_diagnostics_match_canonical_json(self) -> None:
        import json

        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "fallback.pptx"
            self._write_minimal_pptx(path, fallback=True)
            result = pptx2html.convert_with_metadata(str(path))
            raw = json.loads(result.diagnostics_json)

            self.assertGreater(len(result.diagnostics), 0)
            typed = []
            for diagnostic in result.diagnostics:
                position = diagnostic.location.position
                size = diagnostic.location.size
                typed.append(
                    {
                        "code": diagnostic.code,
                        "family": diagnostic.family,
                        "support_tier": diagnostic.support_tier,
                        "stage": diagnostic.stage,
                        "location": {
                            "slide_index": diagnostic.location.slide_index,
                            "part_name": diagnostic.location.part_name,
                            "relationship_id": diagnostic.location.relationship_id,
                            "relationship_type": diagnostic.location.relationship_type,
                            "qualified_element_name": diagnostic.location.qualified_element_name,
                            "position": None
                            if position is None
                            else {"x": position.x, "y": position.y},
                            "size": None
                            if size is None
                            else {"width": size.width, "height": size.height},
                        },
                        "raw_reference": diagnostic.raw_reference,
                        "fallback_kind": diagnostic.fallback_kind,
                        "reason": diagnostic.reason,
                    }
                )

            self.assertEqual(typed, raw)
            marker = '<script type="application/json" id="pptx2html-diagnostics">'
            embedded = result.html.split(marker, 1)[1].split("</script>", 1)[0]
            self.assertEqual(result.diagnostics_json, embedded)
            self.assertNotIn("private.example", result.diagnostics_json)
            self.assertTrue(
                any(
                    item.location.relationship_id == "rIdSecret"
                    for item in result.diagnostics
                )
            )
            with self.assertRaises(AttributeError):
                result.diagnostics[0].code = "changed"

    def test_missing_file_raises_runtime_error(self) -> None:
        with self.assertRaises(RuntimeError):
            pptx2html.convert_file("missing-file-does-not-exist.pptx")

    def test_installed_metadata_exposes_repository_urls(self) -> None:
        project_urls = metadata.metadata("pptx2html").get_all("Project-URL") or []

        self.assertIn(
            "Repository, https://github.com/kim62210/pptx2html-turbo",
            project_urls,
        )
        self.assertIn(
            "Issues, https://github.com/kim62210/pptx2html-turbo/issues",
            project_urls,
        )
        self.assertIn(
            "Homepage, https://github.com/kim62210/pptx2html-turbo",
            project_urls,
        )

    def test_public_conversion_apis_work_from_installed_wheel(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "sample.pptx"
            self._write_minimal_pptx(path)
            data = path.read_bytes()

            html_from_file = pptx2html.convert_file(str(path))
            html_from_bytes = pptx2html.convert_bytes(data)
            html_from_options = pptx2html.convert(str(path), slides=[1])
            metadata_result = pptx2html.convert_bytes_with_metadata(data, slides=[1])

            self.assertIn("Slide One", html_from_file)
            self.assertIn("Slide One", html_from_bytes)
            self.assertIn("Slide One", html_from_options)
            self.assertEqual(type(metadata_result).__name__, "ConversionResult")
            self.assertEqual(metadata_result.slide_count, 1)
            self.assertEqual(len(metadata_result.unresolved_elements), 0)

    def test_bytes_apis_raise_runtime_error_for_invalid_input(self) -> None:
        with self.assertRaises(RuntimeError):
            pptx2html.convert_bytes(b"not-a-pptx")

        with self.assertRaises(RuntimeError):
            pptx2html.convert_bytes_with_metadata(b"not-a-pptx")

    def test_one_based_slide_filtering_is_preserved_for_bytes_api(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            path = Path(tmpdir) / "sample.pptx"
            self._write_minimal_pptx(path, ["Slide One", "Slide Two"])
            data = path.read_bytes()

            html = pptx2html.convert_bytes_with_metadata(data, slides=[2]).html

            self.assertIn("Slide Two", html)
            self.assertNotIn("Slide One", html)

    def _write_minimal_pptx(
        self, path: Path, slides: list[str] | None = None, fallback: bool = False
    ) -> None:
        slide_texts = slides or ["Slide One"]
        with zipfile.ZipFile(path, "w") as archive:
            archive.writestr(
                "[Content_Types].xml", self._content_types(len(slide_texts))
            )
            archive.writestr("_rels/.rels", self._root_rels())
            archive.writestr(
                "ppt/presentation.xml",
                self._presentation_xml(len(slide_texts)),
            )
            archive.writestr(
                "ppt/_rels/presentation.xml.rels",
                self._presentation_rels(len(slide_texts)),
            )
            for index, text in enumerate(slide_texts, start=1):
                archive.writestr(
                    f"ppt/slides/slide{index}.xml", self._slide_xml(text, fallback)
                )
                archive.writestr(
                    f"ppt/slides/_rels/slide{index}.xml.rels",
                    self._unsafe_relationships() if fallback else self._empty_relationships(),
                )
            archive.writestr("ppt/theme/theme1.xml", self._theme_xml())

    def _content_types(self, slide_count: int) -> str:
        slide_overrides = "\n".join(
            f'  <Override PartName="/ppt/slides/slide{index}.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>'
            for index in range(1, slide_count + 1)
        )
        return f"""<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">
  <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>
  <Default Extension=\"xml\" ContentType=\"application/xml\"/>
  <Override PartName=\"/ppt/presentation.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml\"/>
{slide_overrides}
  <Override PartName=\"/ppt/theme/theme1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/>
</Types>"""

    def _root_rels(self) -> str:
        return """<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">
  <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"ppt/presentation.xml\"/>
</Relationships>"""

    def _presentation_xml(self, slide_count: int) -> str:
        slide_ids = "\n".join(
            f'    <p:sldId id="{255 + index}" r:id="rId{index}"/>'
            for index in range(1, slide_count + 1)
        )
        return f"""<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<p:presentation xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"
                xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"
                xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">
  <p:sldMasterIdLst/>
  <p:sldIdLst>
{slide_ids}
  </p:sldIdLst>
  <p:sldSz cx=\"9144000\" cy=\"6858000\"/>
  <p:notesSz cx=\"6858000\" cy=\"9144000\"/>
</p:presentation>"""

    def _presentation_rels(self, slide_count: int) -> str:
        slide_relationships = "\n".join(
            f'  <Relationship Id="rId{index}" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide{index}.xml"/>'
            for index in range(1, slide_count + 1)
        )
        return f"""<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">
{slide_relationships}
  <Relationship Id=\"rId{slide_count + 1}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme\" Target=\"theme/theme1.xml\"/>
</Relationships>"""

    def _slide_xml(self, text: str, fallback: bool = False) -> str:
        unsupported = (
            '<future:widget xmlns="urn:example:future" id="fallback"/>'
            if fallback
            else ""
        )
        return f"""<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\"
       xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"
       xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:sp>
        <p:nvSpPr>
          <p:cNvPr id=\"2\" name=\"TextBox 1\"/>
          <p:cNvSpPr txBox=\"1\"/>
          <p:nvPr/>
        </p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"914400\" cy=\"457200\"/></a:xfrm>
          <a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>
        </p:spPr>
        <p:txBody>
          <a:bodyPr/>
          <a:lstStyle/>
          <a:p>
            <a:r>
              <a:rPr lang=\"en-US\" sz=\"1800\"/>
              <a:t>{text}</a:t>
            </a:r>
          </a:p>
        </p:txBody>
      </p:sp>
      {unsupported}
    </p:spTree>
  </p:cSld>
</p:sld>"""

    def _empty_relationships(self) -> str:
        return """<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"/>"""

    def _unsafe_relationships(self) -> str:
        return """<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">
  <Relationship Id=\"rIdSecret\" Type=\"urn:example:future\" Target=\"https://private.example/token\" TargetMode=\"External\"/>
</Relationships>"""

    def _theme_xml(self) -> str:
        return """<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<a:theme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" name=\"TestTheme\">
  <a:themeElements>
    <a:clrScheme name=\"TestColors\">
      <a:dk1><a:srgbClr val=\"000000\"/></a:dk1>
      <a:lt1><a:srgbClr val=\"FFFFFF\"/></a:lt1>
      <a:dk2><a:srgbClr val=\"1F1F1F\"/></a:dk2>
      <a:lt2><a:srgbClr val=\"F7F7F7\"/></a:lt2>
      <a:accent1><a:srgbClr val=\"4472C4\"/></a:accent1>
      <a:accent2><a:srgbClr val=\"ED7D31\"/></a:accent2>
      <a:accent3><a:srgbClr val=\"A5A5A5\"/></a:accent3>
      <a:accent4><a:srgbClr val=\"FFC000\"/></a:accent4>
      <a:accent5><a:srgbClr val=\"5B9BD5\"/></a:accent5>
      <a:accent6><a:srgbClr val=\"70AD47\"/></a:accent6>
      <a:hlink><a:srgbClr val=\"0563C1\"/></a:hlink>
      <a:folHlink><a:srgbClr val=\"954F72\"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name=\"TestFonts\">
      <a:majorFont><a:latin typeface=\"Calibri\"/></a:majorFont>
      <a:minorFont><a:latin typeface=\"Calibri\"/></a:minorFont>
    </a:fontScheme>
    <a:fmtScheme name=\"TestFmt\"/>
  </a:themeElements>
</a:theme>"""


if __name__ == "__main__":
    unittest.main()
