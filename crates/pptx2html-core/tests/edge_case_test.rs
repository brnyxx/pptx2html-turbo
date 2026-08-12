//! Edge-case and robustness tests for pptx2html-core

mod fixtures;

use pptx2html_core::model::*;

fn parse_pptx(data: &[u8]) -> pptx2html_core::model::Presentation {
    pptx2html_core::parser::PptxParser::parse_bytes(data).expect("PPTX parsing failed")
}

fn render_html(data: &[u8]) -> String {
    let pres = parse_pptx(data);
    pptx2html_core::renderer::HtmlRenderer::render(&pres).expect("HTML rendering failed")
}

// ── Empty / minimal presentations ──

#[test]
fn test_empty_presentation_zero_shapes() {
    let pptx = fixtures::MinimalPptx::new("").build();
    let pres = parse_pptx(&pptx);
    assert_eq!(pres.slides.len(), 1);
    assert!(pres.slides[0].shapes.is_empty());
}

#[test]
fn test_empty_slide_renders_valid_html() {
    let pptx = fixtures::MinimalPptx::new("").build();
    let html = render_html(&pptx);
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("<div class=\"slide\""));
    assert!(html.contains("</html>"));
}

#[test]
fn test_shape_with_no_text_body() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:solidFill><a:srgbClr val="AABBCC"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    assert_eq!(pres.slides[0].shapes.len(), 1);
    assert!(pres.slides[0].shapes[0].text_body.is_none());

    let html = render_html(&pptx);
    assert!(html.contains("#AABBCC"));
    // CSS contains ".text-body" class definition, but no actual <div class="text-body"> inside shape
    assert!(
        !html.contains("<div class=\"text-body"),
        "Shape without text body should not render text-body div"
    );
}

#[test]
fn test_shape_with_empty_text() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>   </a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(html.contains("text-body"));
}

#[test]
fn test_very_long_text() {
    let long_text = "A".repeat(2000);
    let slide = format!(
        r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="LongText"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>{long_text}</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#
    );

    let pptx = fixtures::MinimalPptx::new(&slide).build();
    let html = render_html(&pptx);
    assert!(html.contains(&long_text));
}

// ── Unicode text tests ──

#[test]
fn test_unicode_cjk_text() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="CJK"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>한국어 日本語 中文</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(html.contains("한국어"), "Korean text not found");
    assert!(html.contains("日本語"), "Japanese text not found");
    assert!(html.contains("中文"), "Chinese text not found");
}

#[test]
fn test_unicode_emoji_text() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Emoji"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>Hello World! 🚀🎉</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(html.contains("Hello World!"), "Text not found");
}

#[test]
fn test_hyperlink_parsed_and_rendered() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="LinkBox"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:r>
            <a:rPr>
              <a:hlinkClick r:id="rIdHyper"/>
            </a:rPr>
            <a:t>Open Example</a:t>
          </a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdHyper" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.com" TargetMode="External"/>
</Relationships>"#;

    let pptx = fixtures::MinimalPptx::new(slide)
        .with_slide_rels(slide_rels)
        .build();
    let pres = parse_pptx(&pptx);
    let run = &pres.slides[0].shapes[0]
        .text_body
        .as_ref()
        .unwrap()
        .paragraphs[0]
        .runs[0];
    assert_eq!(run.hyperlink.as_deref(), Some("https://example.com"));

    let html = render_html(&pptx);
    assert!(html.contains("href=\"https://example.com\""));
    assert!(html.contains("Open Example"));
}

#[test]
fn test_hidden_slide_parsed_from_presentation_xml() {
    let pres_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2" show="0"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_presentation_xml(pres_xml)
        .build();
    let pres = parse_pptx(&pptx);
    assert!(pres.slides[0].hidden, "Slide hidden flag should be parsed");

    let html = render_html(&pptx);
    assert!(
        !html.contains("data-slide=\"1\""),
        "Hidden slide should be excluded by default: {html}"
    );
}

#[test]
fn test_presentation_title_parsed_from_core_properties() {
    let core_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/"
                   xmlns:dcterms="http://purl.org/dc/terms/"
                   xmlns:dcmitype="http://purl.org/dc/dcmitype/"
                   xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <dc:title>Quarterly Review</dc:title>
</cp:coreProperties>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_core_properties(core_xml)
        .build();
    let pres = parse_pptx(&pptx);
    assert_eq!(pres.title.as_deref(), Some("Quarterly Review"));

    let html = render_html(&pptx);
    assert!(html.contains("<title>Quarterly Review</title>"));
}

// ── Unsupported content detection ──

#[test]
fn test_smartart_placeholder() {
    let slide = r#"
    <p:graphicFrame>
      <p:nvGraphicFramePr><p:cNvPr id="2" name="Diagram"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
      <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
      <a:graphic>
        <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram">
          <dgm:relIds xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" r:dm="rId2"/>
        </a:graphicData>
      </a:graphic>
    </p:graphicFrame>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    assert_eq!(pres.slides[0].shapes.len(), 1);
    assert!(
        matches!(&pres.slides[0].shapes[0].shape_type, ShapeType::Unsupported(data) if data.label == "SmartArt"),
        "Expected Unsupported(SmartArt)"
    );

    let html = render_html(&pptx);
    assert!(
        html.contains("[SmartArt]"),
        "SmartArt placeholder label not rendered"
    );
}

#[test]
fn test_ole_object_placeholder() {
    let slide = r#"
    <p:graphicFrame>
      <p:nvGraphicFramePr><p:cNvPr id="2" name="OLE"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
      <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
      <a:graphic>
        <a:graphicData uri="http://schemas.openxmlformats.org/schemas/oleObject">
          <p:oleObj r:id="rId2"/>
        </a:graphicData>
      </a:graphic>
    </p:graphicFrame>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    assert_eq!(pres.slides[0].shapes.len(), 1);
    assert!(
        matches!(&pres.slides[0].shapes[0].shape_type, ShapeType::Unsupported(data) if data.label == "OLE Object"),
        "Expected Unsupported(OLE Object)"
    );

    let html = render_html(&pptx);
    assert!(
        html.contains("[OLE Object]"),
        "OLE placeholder label not rendered"
    );
}

#[test]
fn test_math_equation_placeholder() {
    let slide = r#"
    <p:graphicFrame>
      <p:nvGraphicFramePr><p:cNvPr id="2" name="Math"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
      <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
      <a:graphic>
        <a:graphicData uri="http://schemas.openxmlformats.org/officeDocument/2006/math">
          <m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"/>
        </a:graphicData>
      </a:graphic>
    </p:graphicFrame>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    assert_eq!(pres.slides[0].shapes.len(), 1);
    assert!(
        matches!(&pres.slides[0].shapes[0].shape_type, ShapeType::Unsupported(data) if data.label == "Math Equation"),
        "Expected Unsupported(Math Equation)"
    );

    let html = render_html(&pptx);
    assert!(
        html.contains("[Math Equation]"),
        "Math placeholder label not rendered"
    );
}

// ── Malformed input handling ──

#[test]
fn test_invalid_zip_returns_error() {
    let result = pptx2html_core::convert_bytes(b"this is not a zip file");
    assert!(result.is_err(), "Should fail on invalid ZIP");
}

#[test]
fn test_password_protected_detection() {
    // Build a fake "encrypted" package with EncryptedPackage entry
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    zip.start_file("EncryptedPackage", opts).expect("zip entry");
    zip.write_all(b"fake-encrypted-data").expect("write");
    zip.start_file("EncryptionInfo", opts).expect("zip entry");
    zip.write_all(b"fake-info").expect("write");
    let data = zip.finish().expect("finish").into_inner();

    let result = pptx2html_core::convert_bytes(&data);
    assert!(result.is_err(), "Should reject password-protected PPTX");
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("password-protected"),
        "Error should mention password-protected: {err_msg}"
    );
}

// ── Preset shape rendering (no panic) ──

#[test]
fn test_all_basic_preset_shapes_no_panic() {
    let presets = [
        "rect",
        "roundRect",
        "ellipse",
        "triangle",
        "rtTriangle",
        "diamond",
        "parallelogram",
        "trapezoid",
        "pentagon",
        "hexagon",
        "octagon",
    ];

    for prst in &presets {
        let slide = format!(
            r#"
            <p:sp>
              <p:nvSpPr><p:cNvPr id="2" name="Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
              <p:spPr>
                <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
                <a:prstGeom prst="{prst}"/>
              </p:spPr>
            </p:sp>"#
        );
        let pptx = fixtures::MinimalPptx::new(&slide).build();
        let html = render_html(&pptx);
        assert!(
            html.contains("class=\"slide\""),
            "Preset shape '{prst}' failed to render"
        );
    }
}

#[test]
fn test_arrow_preset_shapes_no_panic() {
    let presets = [
        "rightArrow",
        "leftArrow",
        "upArrow",
        "downArrow",
        "leftRightArrow",
        "upDownArrow",
        "chevron",
    ];

    for prst in &presets {
        let slide = format!(
            r#"
            <p:sp>
              <p:nvSpPr><p:cNvPr id="2" name="Arrow"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
              <p:spPr>
                <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
                <a:prstGeom prst="{prst}"/>
              </p:spPr>
            </p:sp>"#
        );
        let pptx = fixtures::MinimalPptx::new(&slide).build();
        let html = render_html(&pptx);
        assert!(
            html.contains("class=\"slide\""),
            "Arrow shape '{prst}' failed to render"
        );
    }
}

// ── Missing theme graceful fallback ──

#[test]
fn test_theme_color_fallback_without_theme() {
    // schemeClr with no theme available should not panic
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:r>
            <a:rPr sz="1800"><a:solidFill><a:schemeClr val="unknownSchemeColor"/></a:solidFill></a:rPr>
            <a:t>Fallback Color</a:t>
          </a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    // Should not panic even with unknown scheme color
    let html = render_html(&pptx);
    assert!(html.contains("Fallback Color"));
}

#[test]
fn test_invalid_color_value_graceful() {
    // An invalid srgbClr value should not panic
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:solidFill><a:srgbClr val="ZZZZZZ"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    // Should not panic, just skip the invalid color
    let html = render_html(&pptx);
    assert!(html.contains("class=\"slide\""));
}

// ── ConversionOptions edge cases ──

#[test]
fn test_conversion_options_empty_slide_range() {
    use pptx2html_core::ConversionOptions;

    let opts = ConversionOptions {
        slide_range: Some((5, 10)),
        ..Default::default()
    };
    // Slide 1 with range (5, 10) should be excluded
    assert!(!opts.should_include_slide(1, false));
    assert!(opts.should_include_slide(5, false));
    assert!(opts.should_include_slide(10, false));
    assert!(!opts.should_include_slide(11, false));
}

#[test]
fn test_conversion_options_hidden_slide() {
    use pptx2html_core::ConversionOptions;

    let opts_no_hidden = ConversionOptions::default();
    assert!(!opts_no_hidden.should_include_slide(1, true));

    let opts_with_hidden = ConversionOptions {
        include_hidden: true,
        ..Default::default()
    };
    assert!(opts_with_hidden.should_include_slide(1, true));
}

// ── Group shape nesting ──

#[test]
fn test_nested_group_shapes() {
    // A group containing a shape — should not panic
    let slide = r#"
    <p:grpSp>
      <p:nvGrpSpPr><p:cNvPr id="10" name="Group"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr>
        <a:xfrm>
          <a:off x="100000" y="100000"/>
          <a:ext cx="5000000" cy="3000000"/>
          <a:chOff x="0" y="0"/>
          <a:chExt cx="5000000" cy="3000000"/>
        </a:xfrm>
      </p:grpSpPr>
      <p:sp>
        <p:nvSpPr><p:cNvPr id="11" name="InnerShape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
        <p:spPr>
          <a:xfrm><a:off x="500000" y="500000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
          <a:prstGeom prst="rect"/>
          <a:solidFill><a:srgbClr val="AABB00"/></a:solidFill>
        </p:spPr>
      </p:sp>
    </p:grpSp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    assert_eq!(pres.slides[0].shapes.len(), 1);
    assert!(
        matches!(&pres.slides[0].shapes[0].shape_type, ShapeType::Group(children, _) if children.len() == 1),
        "Expected group with one child"
    );

    let html = render_html(&pptx);
    assert!(html.contains("#AABB00"), "Inner shape fill not rendered");
}

// ── get_info / get_info_from_bytes ──

#[test]
fn test_get_info_from_bytes() {
    let pptx = fixtures::MinimalPptx::new("").build();
    let info = pptx2html_core::get_info_from_bytes(&pptx).expect("get_info_from_bytes failed");
    assert_eq!(info.slide_count, 1);
    assert!((info.width_px - 960.0).abs() < 0.1);
    assert!((info.height_px - 720.0).abs() < 0.1);
}

// ── Metadata sideband tests ──

#[test]
fn test_smartart_metadata() {
    use pptx2html_core::model::slide::UnresolvedType;

    let slide = r#"
    <p:graphicFrame>
      <p:nvGraphicFramePr><p:cNvPr id="2" name="Diagram"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
      <p:xfrm><a:off x="100000" y="200000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
      <a:graphic>
        <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram">
          <dgm:relIds xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" r:dm="rId1" r:lo="rId2"/>
        </a:graphicData>
      </a:graphic>
    </p:graphicFrame>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let result = pptx2html_core::convert_bytes_with_metadata(&pptx).expect("conversion failed");

    // HTML contains structured placeholder
    assert!(
        result.html.contains("unresolved-element"),
        "HTML should contain unresolved-element class"
    );
    assert!(
        result.html.contains("data-type=\"smartart\""),
        "HTML should contain data-type=smartart attribute"
    );
    assert!(
        result.html.contains("data-slide=\"0\""),
        "HTML should contain data-slide attribute"
    );
    assert!(
        result.html.contains("id=\"unresolved-s0-e0\""),
        "HTML should contain structured placeholder ID"
    );

    // Metadata
    assert_eq!(result.unresolved_elements.len(), 1);
    let elem = &result.unresolved_elements[0];
    assert_eq!(elem.element_type, UnresolvedType::SmartArt);
    assert_eq!(elem.slide_index, 0);
    assert!(elem.placeholder_id.starts_with("unresolved-"));

    // Raw XML captured
    assert!(
        elem.raw_xml.is_some(),
        "SmartArt raw_xml should be captured"
    );
    let raw = elem.raw_xml.as_ref().expect("raw_xml");
    assert!(
        raw.contains("relIds") || raw.contains("dgm"),
        "Raw XML should contain diagram-related content: {raw}"
    );
}

#[test]
fn test_math_metadata() {
    use pptx2html_core::model::slide::UnresolvedType;

    let slide = r#"
    <p:graphicFrame>
      <p:nvGraphicFramePr><p:cNvPr id="2" name="Math"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
      <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
      <a:graphic>
        <a:graphicData uri="http://schemas.openxmlformats.org/officeDocument/2006/math">
          <m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"/>
        </a:graphicData>
      </a:graphic>
    </p:graphicFrame>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let result = pptx2html_core::convert_bytes_with_metadata(&pptx).expect("conversion failed");

    assert_eq!(result.unresolved_elements.len(), 1);
    let elem = &result.unresolved_elements[0];
    assert_eq!(elem.element_type, UnresolvedType::MathEquation);
    assert!(result.html.contains("data-type=\"math\""));
}

#[test]
fn test_ole_metadata() {
    use pptx2html_core::model::slide::UnresolvedType;

    let slide = r#"
    <p:graphicFrame>
      <p:nvGraphicFramePr><p:cNvPr id="2" name="OLE"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
      <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
      <a:graphic>
        <a:graphicData uri="http://schemas.openxmlformats.org/schemas/oleObject">
          <p:oleObj r:id="rId2" progId="Excel.Sheet.12"/>
        </a:graphicData>
      </a:graphic>
    </p:graphicFrame>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let result = pptx2html_core::convert_bytes_with_metadata(&pptx).expect("conversion failed");

    assert_eq!(result.unresolved_elements.len(), 1);
    let elem = &result.unresolved_elements[0];
    assert_eq!(elem.element_type, UnresolvedType::OleObject);
    assert!(result.html.contains("data-type=\"ole\""));

    // Raw XML should capture OLE attributes
    assert!(elem.raw_xml.is_some());
    let raw = elem.raw_xml.as_ref().expect("raw_xml");
    assert!(
        raw.contains("oleObj") || raw.contains("progId"),
        "OLE raw XML should contain object attributes: {raw}"
    );
}

#[test]
fn test_no_unresolved_for_normal_shapes() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>Normal text</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let result = pptx2html_core::convert_bytes_with_metadata(&pptx).expect("conversion failed");

    assert!(
        result.unresolved_elements.is_empty(),
        "Normal shapes should produce no unresolved elements"
    );
    assert_eq!(result.slide_count, 1);
    assert!(result.html.contains("Normal text"));
}

#[test]
fn test_backward_compat_convert_bytes() {
    let slide = r#"
    <p:graphicFrame>
      <p:nvGraphicFramePr><p:cNvPr id="2" name="Diagram"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
      <p:xfrm><a:off x="100000" y="200000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
      <a:graphic>
        <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram">
          <dgm:relIds xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" r:dm="rId1"/>
        </a:graphicData>
      </a:graphic>
    </p:graphicFrame>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();

    // Existing API still returns String, not ConversionResult
    let html: String = pptx2html_core::convert_bytes(&pptx).expect("conversion failed");
    assert!(
        html.contains("[SmartArt]"),
        "Backward compat: label should still appear"
    );
    assert!(html.contains("<!DOCTYPE html>"), "Should be complete HTML");
}

#[test]
fn test_multiple_unresolved_elements_unique_ids() {
    let slide = r#"
    <p:graphicFrame>
      <p:nvGraphicFramePr><p:cNvPr id="2" name="SmartArt1"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
      <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="4000000" cy="2000000"/></p:xfrm>
      <a:graphic>
        <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram">
          <dgm:relIds xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram" r:dm="rId1"/>
        </a:graphicData>
      </a:graphic>
    </p:graphicFrame>
    <p:graphicFrame>
      <p:nvGraphicFramePr><p:cNvPr id="3" name="Math1"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
      <p:xfrm><a:off x="100000" y="3000000"/><a:ext cx="4000000" cy="2000000"/></p:xfrm>
      <a:graphic>
        <a:graphicData uri="http://schemas.openxmlformats.org/officeDocument/2006/math">
          <m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"/>
        </a:graphicData>
      </a:graphic>
    </p:graphicFrame>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let result = pptx2html_core::convert_bytes_with_metadata(&pptx).expect("conversion failed");

    assert_eq!(result.unresolved_elements.len(), 2);

    // Each element should have a unique placeholder_id
    let id0 = &result.unresolved_elements[0].placeholder_id;
    let id1 = &result.unresolved_elements[1].placeholder_id;
    assert_ne!(id0, id1, "Placeholder IDs must be unique");

    // IDs should be present in HTML
    assert!(result.html.contains(id0.as_str()));
    assert!(result.html.contains(id1.as_str()));
}

#[test]
fn test_conversion_result_slide_count() {
    use pptx2html_core::ConversionOptions;

    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();

    // Default options
    let result = pptx2html_core::convert_bytes_with_metadata(&pptx).expect("conversion failed");
    assert_eq!(result.slide_count, 1);

    // With options
    let opts = ConversionOptions::default();
    let result = pptx2html_core::convert_bytes_with_options_metadata(&pptx, &opts)
        .expect("conversion failed");
    assert_eq!(result.slide_count, 1);
}

// ── Custom Geometry (custGeom) tests ──

#[test]
fn test_custgeom_triangle_parsed() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="CustTriangle"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="0" y="21600"/></a:moveTo>
              <a:lnTo><a:pt x="10800" y="0"/></a:lnTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
              <a:close/>
            </a:path>
          </a:pathLst>
        </a:custGeom>
        <a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    assert_eq!(pres.slides[0].shapes.len(), 1);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => {
            assert_eq!(geom.paths.len(), 1);
            let path = &geom.paths[0];
            assert!((path.width - 21600.0).abs() < 0.01);
            assert!((path.height - 21600.0).abs() < 0.01);
            assert_eq!(path.commands.len(), 4); // moveTo + 2 lnTo + close
        }
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_renders_svg_path() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="CustShape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:pathLst>
            <a:path w="100" h="100">
              <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="100" y="0"/></a:lnTo>
              <a:lnTo><a:pt x="100" y="100"/></a:lnTo>
              <a:lnTo><a:pt x="0" y="100"/></a:lnTo>
              <a:close/>
            </a:path>
          </a:pathLst>
        </a:custGeom>
        <a:solidFill><a:srgbClr val="0000FF"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    // SVG should be present with path element
    assert!(html.contains("<svg viewBox="), "Should contain SVG element");
    assert!(html.contains("<path d="), "Should contain SVG path");
    assert!(html.contains("shape-svg"), "Should have shape-svg class");
}

#[test]
fn test_custgeom_cubic_bezier() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="CustBez"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
              <a:cubicBezTo>
                <a:pt x="7200" y="0"/>
                <a:pt x="14400" y="21600"/>
                <a:pt x="21600" y="21600"/>
              </a:cubicBezTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => {
            assert_eq!(geom.paths[0].commands.len(), 2); // moveTo + cubicBezTo
            match &geom.paths[0].commands[1] {
                PathCommand::CubicBezTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => {
                    assert!((x1 - 7200.0).abs() < 0.01);
                    assert!((*y1).abs() < 0.01);
                    assert!((x2 - 14400.0).abs() < 0.01);
                    assert!((y2 - 21600.0).abs() < 0.01);
                    assert!((x - 21600.0).abs() < 0.01);
                    assert!((y - 21600.0).abs() < 0.01);
                }
                other => panic!("Expected CubicBezTo, got {:?}", other),
            }
        }
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_quad_bezier() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="CustQuad"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:pathLst>
            <a:path w="100" h="100">
              <a:moveTo><a:pt x="0" y="100"/></a:moveTo>
              <a:quadBezTo>
                <a:pt x="50" y="0"/>
                <a:pt x="100" y="100"/>
              </a:quadBezTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => {
            assert_eq!(geom.paths[0].commands.len(), 2); // moveTo + quadBezTo
            match &geom.paths[0].commands[1] {
                PathCommand::QuadBezTo { x1, y1, x, y } => {
                    assert!((x1 - 50.0).abs() < 0.01);
                    assert!((*y1).abs() < 0.01);
                    assert!((x - 100.0).abs() < 0.01);
                    assert!((y - 100.0).abs() < 0.01);
                }
                other => panic!("Expected QuadBezTo, got {:?}", other),
            }
        }
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_arc_to() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="CustArc"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="0" y="10800"/></a:moveTo>
              <a:arcTo wR="5400" hR="5400" stAng="0" swAng="5400000"/>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => {
            assert_eq!(geom.paths[0].commands.len(), 2); // moveTo + arcTo
            match &geom.paths[0].commands[1] {
                PathCommand::ArcTo {
                    wr,
                    hr,
                    start_angle,
                    swing_angle,
                } => {
                    assert!((wr - 5400.0).abs() < 0.01);
                    assert!((hr - 5400.0).abs() < 0.01);
                    assert!((*start_angle).abs() < 0.01);
                    assert!((swing_angle - 5400000.0).abs() < 0.01);
                }
                other => panic!("Expected ArcTo, got {:?}", other),
            }
        }
        other => panic!("Expected CustomGeom, got {:?}", other),
    }

    // Verify it renders as SVG with arc command
    let html = render_html(&pptx);
    assert!(html.contains("<path d="), "Should render SVG path with arc");
}

#[test]
fn test_custgeom_multiple_paths() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="MultiPath"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:pathLst>
            <a:path w="100" h="100">
              <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="100" y="100"/></a:lnTo>
            </a:path>
            <a:path w="100" h="100" fill="none">
              <a:moveTo><a:pt x="100" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="0" y="100"/></a:lnTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => {
            assert_eq!(geom.paths.len(), 2);
            assert!(matches!(geom.paths[0].fill, PathFill::Norm));
            assert!(matches!(geom.paths[1].fill, PathFill::None));
        }
        other => panic!("Expected CustomGeom, got {:?}", other),
    }

    // Verify both paths render in SVG
    let html = render_html(&pptx);
    let path_count = html.matches("<path d=").count();
    assert!(path_count >= 2, "Expected 2+ SVG paths, got {}", path_count);
}

#[test]
fn test_custgeom_with_text_body() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="CustWithText"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:pathLst>
            <a:path w="100" h="100">
              <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="100" y="0"/></a:lnTo>
              <a:lnTo><a:pt x="50" y="100"/></a:lnTo>
              <a:close/>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>Hello Custom</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("Hello Custom"),
        "Text should be rendered alongside custom geometry"
    );
    assert!(html.contains("<path d="), "SVG path should be present");
}

#[test]
fn test_custgeom_empty_pathlist() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="EmptyCust"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:pathLst/>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => {
            assert!(geom.paths.is_empty());
        }
        other => panic!("Expected CustomGeom with empty paths, got {:?}", other),
    }
    // Should not crash when rendering
    let _html = render_html(&pptx);
}

#[test]
fn test_custgeom_gd_val_drives_point_coordinates() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuidedCust"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="dx1" fmla="val 7200"/>
            <a:gd name="dy1" fmla="val 10800"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="dx1" y="dy1"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[0] {
            PathCommand::MoveTo { x, y } => {
                assert!(
                    (x - 7200.0).abs() < 0.01,
                    "guide x should resolve to 7200, got {x}"
                );
                assert!(
                    (y - 10800.0).abs() < 0.01,
                    "guide y should resolve to 10800, got {y}"
                );
            }
            other => panic!("Expected MoveTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_gd_val_drives_arc_attributes() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuidedArc"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="rx1" fmla="val 5400"/>
            <a:gd name="ang1" fmla="val 5400000"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="0" y="10800"/></a:moveTo>
              <a:arcTo wR="rx1" hR="rx1" stAng="0" swAng="ang1"/>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[1] {
            PathCommand::ArcTo {
                wr,
                hr,
                start_angle,
                swing_angle,
            } => {
                assert!(
                    (wr - 5400.0).abs() < 0.01,
                    "guide wR should resolve to 5400, got {wr}"
                );
                assert!(
                    (hr - 5400.0).abs() < 0.01,
                    "guide hR should resolve to 5400, got {hr}"
                );
                assert!((*start_angle).abs() < 0.01);
                assert!(
                    (swing_angle - 5400000.0).abs() < 0.01,
                    "guide swAng should resolve to 5400000, got {swing_angle}"
                );
            }
            other => panic!("Expected ArcTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_rect_uses_guide_values() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuidedRect"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1270000" cy="1270000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="l1" fmla="val 5400"/>
            <a:gd name="t1" fmla="val 2160"/>
          </a:gdLst>
          <a:rect l="l1" t="t1" r="16200" b="19440"/>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="0"/></a:lnTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
              <a:lnTo><a:pt x="0" y="21600"/></a:lnTo>
              <a:close/>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
      <p:txBody>
        <a:bodyPr lIns="0" tIns="0" rIns="0" bIns="0"/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>Rect text</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => {
            let rect = geom
                .text_rect
                .as_ref()
                .expect("expected custom geometry text rect");
            assert!(
                (rect.left - 5400.0).abs() < 0.01,
                "expected left 5400, got {}",
                rect.left
            );
            assert!(
                (rect.top - 2160.0).abs() < 0.01,
                "expected top 2160, got {}",
                rect.top
            );
            assert!(
                (rect.right - 16200.0).abs() < 0.01,
                "expected right 16200, got {}",
                rect.right
            );
            assert!(
                (rect.bottom - 19440.0).abs() < 0.01,
                "expected bottom 19440, got {}",
                rect.bottom
            );
        }
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_parses_xy_and_polar_adjust_handles() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Handles"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1270000" cy="1270000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="gx" fmla="val 5400"/>
            <a:gd name="gy" fmla="val 10800"/>
            <a:gd name="gr" fmla="val 7200"/>
            <a:gd name="ga" fmla="val 5400000"/>
          </a:gdLst>
          <a:ahLst>
            <a:ahXY gdRefX="gx" minX="0" maxX="21600" gdRefY="gy" minY="0" maxY="21600">
              <a:pos x="gx" y="gy"/>
            </a:ahXY>
            <a:ahPolar gdRefR="gr" minR="0" maxR="21600" gdRefAng="ga" minAng="0" maxAng="21600000">
              <a:pos x="10800" y="10800"/>
            </a:ahPolar>
          </a:ahLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pres = parse_pptx(&fixtures::MinimalPptx::new(slide).build());
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => {
            assert_eq!(
                geom.adjust_handles.len(),
                2,
                "expected two custom geometry handles"
            );
            match &geom.adjust_handles[0] {
                AdjustHandle::XY(handle) => {
                    assert_eq!(handle.gd_ref_x.as_deref(), Some("gx"));
                    assert_eq!(handle.gd_ref_y.as_deref(), Some("gy"));
                    assert!((handle.pos_x - 5400.0).abs() < 0.01);
                    assert!((handle.pos_y - 10800.0).abs() < 0.01);
                }
                other => panic!("Expected XY handle, got {:?}", other),
            }
            match &geom.adjust_handles[1] {
                AdjustHandle::Polar(handle) => {
                    assert_eq!(handle.gd_ref_r.as_deref(), Some("gr"));
                    assert_eq!(handle.gd_ref_ang.as_deref(), Some("ga"));
                    assert!((handle.pos_x - 10800.0).abs() < 0.01);
                    assert!((handle.pos_y - 10800.0).abs() < 0.01);
                }
                other => panic!("Expected Polar handle, got {:?}", other),
            }
        }
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_parses_connection_sites() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Connections"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1270000" cy="1270000"/></a:xfrm>
        <a:custGeom>
          <a:cxnLst>
            <a:cxn ang="5400000"><a:pos x="21600" y="10800"/></a:cxn>
            <a:cxn ang="10800000"><a:pos x="10800" y="21600"/></a:cxn>
          </a:cxnLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pres = parse_pptx(&fixtures::MinimalPptx::new(slide).build());
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => {
            assert_eq!(
                geom.connection_sites.len(),
                2,
                "expected two connection sites"
            );
            assert!((geom.connection_sites[0].x - 21600.0).abs() < 0.01);
            assert!((geom.connection_sites[0].y - 10800.0).abs() < 0.01);
            assert!((geom.connection_sites[0].angle - 5400000.0).abs() < 0.01);
            assert!((geom.connection_sites[1].x - 10800.0).abs() < 0.01);
            assert!((geom.connection_sites[1].y - 21600.0).abs() < 0.01);
            assert!((geom.connection_sites[1].angle - 10800000.0).abs() < 0.01);
        }
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_guide_plus_minus_formula_drives_point() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuidePlusMinus"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="base" fmla="val 4000"/>
            <a:gd name="sum1" fmla="+- base 5000 1000"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="sum1" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[0] {
            PathCommand::MoveTo { x, .. } => {
                assert!(
                    (x - 8000.0).abs() < 0.01,
                    "expected 8000 from +- formula, got {x}"
                );
            }
            other => panic!("Expected MoveTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_guide_mul_div_formula_drives_arc_radius() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuideMulDiv"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="r1" fmla="*/ 10800 2 3"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="0" y="10800"/></a:moveTo>
              <a:arcTo wR="r1" hR="r1" stAng="0" swAng="5400000"/>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[1] {
            PathCommand::ArcTo { wr, hr, .. } => {
                assert!(
                    (wr - 7200.0).abs() < 0.01,
                    "expected 7200 from */ formula, got {wr}"
                );
                assert!(
                    (hr - 7200.0).abs() < 0.01,
                    "expected 7200 from */ formula, got {hr}"
                );
            }
            other => panic!("Expected ArcTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_guide_pin_min_max_formulas_drive_point() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuideClamp"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="a" fmla="val 3000"/>
            <a:gd name="b" fmla="val 9000"/>
            <a:gd name="m1" fmla="min a b"/>
            <a:gd name="m2" fmla="max a b"/>
            <a:gd name="clamped" fmla="pin 4000 m1 m2"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="clamped" y="m2"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[0] {
            PathCommand::MoveTo { x, y } => {
                assert!(
                    (x - 4000.0).abs() < 0.01,
                    "expected pin result 4000, got {x}"
                );
                assert!(
                    (y - 9000.0).abs() < 0.01,
                    "expected max result 9000, got {y}"
                );
            }
            other => panic!("Expected MoveTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_guide_dependency_chain_resolves_in_order() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuideChain"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="a" fmla="val 2000"/>
            <a:gd name="b" fmla="+- a 5000 1000"/>
            <a:gd name="c" fmla="*/ b 3 2"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="c" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[0] {
            PathCommand::MoveTo { x, .. } => {
                assert!(
                    (x - 9000.0).abs() < 0.01,
                    "expected chained guide result 9000, got {x}"
                );
            }
            other => panic!("Expected MoveTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_mul_div_zero_denominator_returns_fallback() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuideDivZero"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="z" fmla="val 0"/>
            <a:gd name="r1" fmla="*/ 10800 2 z"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="0" y="10800"/></a:moveTo>
              <a:arcTo wR="r1" hR="r1" stAng="0" swAng="5400000"/>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    let ShapeType::Unsupported(data) = &shape.shape_type else {
        panic!(
            "Expected unsupported custom geometry, got {:?}",
            shape.shape_type
        )
    };
    assert!(matches!(data.element_type, UnresolvedType::CustomGeometry));
    assert_eq!(data.raw_xml.as_deref(), Some("*/ 10800 2 z"));

    let result = pptx2html_core::convert_bytes_with_metadata(&pptx).expect("fallback conversion");
    let diagnostic = result
        .diagnostics()
        .iter()
        .find(|item| item.code == "DRAWINGML_CUSTOM_GEOMETRY_FALLBACK")
        .expect("custom geometry fallback diagnostic");
    assert!(matches!(diagnostic.support_tier, SupportTier::Fallback));
    assert!(matches!(
        diagnostic.fallback_kind,
        FallbackKind::CustomGeometryPlaceholder
    ));
    assert_eq!(diagnostic.raw_reference.as_deref(), Some("*/ 10800 2 z"));
}

#[test]
fn test_custgeom_ifelse_formula_drives_point_coordinate() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuideIfElse"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="cond" fmla="val 1"/>
            <a:gd name="x1" fmla="?: cond 7000 3000"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="x1" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[0] {
            PathCommand::MoveTo { x, .. } => {
                assert!(
                    (x - 7000.0).abs() < 0.01,
                    "expected ifelse true branch 7000, got {x}"
                );
            }
            other => panic!("Expected MoveTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_ifelse_formula_uses_false_branch_for_zero() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuideIfElseFalse"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="cond" fmla="val 0"/>
            <a:gd name="r1" fmla="?: cond 7000 3000"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="0" y="10800"/></a:moveTo>
              <a:arcTo wR="r1" hR="r1" stAng="0" swAng="5400000"/>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[1] {
            PathCommand::ArcTo { wr, hr, .. } => {
                assert!(
                    (wr - 3000.0).abs() < 0.01,
                    "expected ifelse false branch 3000, got {wr}"
                );
                assert!(
                    (hr - 3000.0).abs() < 0.01,
                    "expected ifelse false branch 3000, got {hr}"
                );
            }
            other => panic!("Expected ArcTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_sqrt_formula_drives_point_coordinate() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuideSqrt"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="base" fmla="val 8100"/>
            <a:gd name="root" fmla="sqrt base"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="root" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[0] {
            PathCommand::MoveTo { x, .. } => {
                assert!((x - 90.0).abs() < 0.01, "expected sqrt result 90, got {x}");
            }
            other => panic!("Expected MoveTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_mod_formula_uses_vector_length_semantics() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuideMod"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="len" fmla="mod 3 4 12"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="len" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[0] {
            PathCommand::MoveTo { x, .. } => {
                assert!(
                    (x - 13.0).abs() < 0.01,
                    "expected vector-length mod result 13, got {x}"
                );
            }
            other => panic!("Expected MoveTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_abs_formula_normalizes_negative_value() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuideAbs"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="neg" fmla="val -4500"/>
            <a:gd name="pos" fmla="abs neg"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="pos" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[0] {
            PathCommand::MoveTo { x, .. } => {
                assert!(
                    (x - 4500.0).abs() < 0.01,
                    "expected abs result 4500, got {x}"
                );
            }
            other => panic!("Expected MoveTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_sin_formula_scales_by_angle() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuideSin"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="s1" fmla="sin 100000 5400000"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="s1" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pres = parse_pptx(&fixtures::MinimalPptx::new(slide).build());
    let shape = &pres.slides[0].shapes[0];
    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[0] {
            PathCommand::MoveTo { x, .. } => {
                assert!(
                    (x - 100000.0).abs() < 0.1,
                    "expected sin result 100000, got {x}"
                );
            }
            other => panic!("Expected MoveTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_cos_formula_scales_by_angle() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuideCos"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="c1" fmla="cos 100000 5400000"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="c1" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pres = parse_pptx(&fixtures::MinimalPptx::new(slide).build());
    let shape = &pres.slides[0].shapes[0];
    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[0] {
            PathCommand::MoveTo { x, .. } => {
                assert!(x.abs() < 0.1, "expected cos result near 0, got {x}");
            }
            other => panic!("Expected MoveTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_cat2_and_sat2_formulas_drive_point() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuideCatSat"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="x1" fmla="cat2 100 3 4"/>
            <a:gd name="y1" fmla="sat2 100 3 4"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="x1" y="y1"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pres = parse_pptx(&fixtures::MinimalPptx::new(slide).build());
    let shape = &pres.slides[0].shapes[0];
    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[0] {
            PathCommand::MoveTo { x, y } => {
                assert!((x - 60.0).abs() < 0.1, "expected cat2 result 60, got {x}");
                assert!((y - 80.0).abs() < 0.1, "expected sat2 result 80, got {y}");
            }
            other => panic!("Expected MoveTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_at2_formula_drives_arc_angle() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuideAt2"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="ang1" fmla="at2 3 4"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="0" y="10800"/></a:moveTo>
              <a:arcTo wR="5400" hR="5400" stAng="0" swAng="ang1"/>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pres = parse_pptx(&fixtures::MinimalPptx::new(slide).build());
    let shape = &pres.slides[0].shapes[0];
    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[1] {
            PathCommand::ArcTo { swing_angle, .. } => {
                assert!(
                    (swing_angle - 3_187_806.14).abs() < 1.0,
                    "expected at2 result near 3187806.14, got {swing_angle}"
                );
            }
            other => panic!("Expected ArcTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_adddiv_formula_drives_point_coordinate() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuideAddDiv"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="mid" fmla="+/ 12000 6000 3"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="mid" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pres = parse_pptx(&fixtures::MinimalPptx::new(slide).build());
    let shape = &pres.slides[0].shapes[0];
    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[0] {
            PathCommand::MoveTo { x, .. } => {
                assert!(
                    (x - 6000.0).abs() < 0.01,
                    "expected adddiv result 6000, got {x}"
                );
            }
            other => panic!("Expected MoveTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

#[test]
fn test_custgeom_tan_formula_drives_point_coordinate() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GuideTan"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:custGeom>
          <a:gdLst>
            <a:gd name="t1" fmla="tan 100000 2700000"/>
          </a:gdLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="t1" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>"#;

    let pres = parse_pptx(&fixtures::MinimalPptx::new(slide).build());
    let shape = &pres.slides[0].shapes[0];
    match &shape.shape_type {
        ShapeType::CustomGeom(geom) => match &geom.paths[0].commands[0] {
            PathCommand::MoveTo { x, .. } => {
                assert!(
                    (x - 100000.0).abs() < 0.5,
                    "expected tan result near 100000, got {x}"
                );
            }
            other => panic!("Expected MoveTo, got {:?}", other),
        },
        other => panic!("Expected CustomGeom, got {:?}", other),
    }
}

// ── Auto-fit (normAutofit / spAutoFit) ──

#[test]
fn test_norm_autofit_font_scale_parsed() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit fontScale="62500" lnSpcReduction="20000"/>
        </a:bodyPr>
        <a:p><a:r><a:rPr sz="2400"/><a:t>Scaled text</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    let tb = shape.text_body.as_ref().expect("text_body");

    match &tb.auto_fit {
        AutoFit::Normal {
            font_scale,
            line_spacing_reduction,
        } => {
            let fs = font_scale.expect("font_scale should be Some");
            assert!(
                (fs - 0.625).abs() < 0.001,
                "font_scale: expected ~0.625, got {fs}"
            );
            let lr = line_spacing_reduction.expect("line_spacing_reduction should be Some");
            assert!(
                (lr - 0.2).abs() < 0.001,
                "line_spacing_reduction: expected ~0.2, got {lr}"
            );
        }
        other => panic!("Expected AutoFit::Normal, got {:?}", other),
    }
}

#[test]
fn test_norm_autofit_without_attributes() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit/>
        </a:bodyPr>
        <a:p><a:r><a:rPr sz="1800"/><a:t>Text</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let tb = pres.slides[0].shapes[0]
        .text_body
        .as_ref()
        .expect("text_body");

    match &tb.auto_fit {
        AutoFit::Normal {
            font_scale,
            line_spacing_reduction,
        } => {
            assert!(font_scale.is_none(), "font_scale should be None");
            assert!(
                line_spacing_reduction.is_none(),
                "line_spacing_reduction should be None"
            );
        }
        other => panic!("Expected AutoFit::Normal, got {:?}", other),
    }
}

#[test]
fn test_sp_auto_fit_parsed_as_shrink() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr><a:spAutoFit/></a:bodyPr>
        <a:p><a:r><a:rPr sz="1800"/><a:t>Shrink</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let tb = pres.slides[0].shapes[0]
        .text_body
        .as_ref()
        .expect("text_body");
    assert!(matches!(tb.auto_fit, AutoFit::Shrink));
}

#[test]
fn test_sp_auto_fit_allows_text_body_to_grow() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Grow"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr><a:spAutoFit/></a:bodyPr>
        <a:p><a:r><a:t>Grow to fit text</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk = &html[tb_start..tb_start + 300.min(html.len() - tb_start)];
    assert!(
        tb_chunk.contains("height: auto") && tb_chunk.contains("min-height: 100%"),
        "Expected spAutoFit text-body to opt into growth-oriented sizing: {tb_chunk}"
    );
}

#[test]
fn test_sp_auto_fit_does_not_force_emergency_wrap_for_unbreakable_token() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="GrowLongToken"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1600000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr><a:spAutoFit/></a:bodyPr>
        <a:p><a:r><a:rPr sz="1800"/><a:t>SupercalifragilisticexpialidociousWithoutSpaces</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk = &html[tb_start..tb_start + 360.min(html.len() - tb_start)];

    assert!(
        tb_chunk.contains("height: auto") && tb_chunk.contains("min-height: 100%"),
        "spAutoFit should keep growth-oriented sizing for long tokens: {tb_chunk}"
    );
    assert!(
        !tb_chunk.contains("overflow-wrap: anywhere"),
        "spAutoFit should grow instead of forcing overflow-wrap:anywhere: {tb_chunk}"
    );
    assert!(
        !tb_chunk.contains("emergency-wrap"),
        "spAutoFit should not opt into the emergency-wrap class for long tokens: {tb_chunk}"
    );
}

#[test]
fn test_no_autofit_is_distinct_from_unspecified_autofit() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr><a:noAutofit/></a:bodyPr>
        <a:p><a:r><a:rPr sz="1800"/><a:t>No autofit</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let tb = pres.slides[0].shapes[0]
        .text_body
        .as_ref()
        .expect("text_body");

    assert!(
        !matches!(tb.auto_fit, AutoFit::None),
        "Expected explicit <a:noAutofit/> to be distinct from unspecified AutoFit::None"
    );
}

#[test]
fn test_norm_autofit_renders_scaled_font_size() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit fontScale="50000"/>
        </a:bodyPr>
        <a:p><a:r><a:rPr sz="2000"/><a:t>Half size</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    // Original 20pt * 0.5 = 10pt
    assert!(
        html.contains("font-size: 10.0pt"),
        "Expected scaled font-size 10.0pt in HTML: {html}"
    );
}

#[test]
fn test_norm_autofit_renders_overflow_hidden() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit fontScale="80000"/>
        </a:bodyPr>
        <a:p><a:r><a:rPr sz="1800"/><a:t>Clipped</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    // Check that the text-body div's inline style contains overflow:hidden
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk = &html[tb_start..tb_start + 300.min(html.len() - tb_start)];
    assert!(
        tb_chunk.contains("overflow: hidden"),
        "Expected overflow:hidden on text-body when fontScale is present, got: {tb_chunk}"
    );
}

#[test]
fn test_norm_autofit_line_spacing_reduction() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit lnSpcReduction="20000"/>
        </a:bodyPr>
        <a:p>
          <a:pPr><a:lnSpc><a:spcPct val="150000"/></a:lnSpc></a:pPr>
          <a:r><a:rPr sz="1800"/><a:t>Reduced spacing</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    // line-height 1.5 * (1.0 - 0.2) = 1.2
    assert!(
        html.contains("line-height: 1.20"),
        "Expected reduced line-height 1.20 in HTML: {html}"
    );
}

#[test]
fn test_norm_autofit_font_scale_is_clamped_to_one() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit fontScale="150000"/>
        </a:bodyPr>
        <a:p><a:r><a:rPr sz="1800"/><a:t>Clamp scale</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("font-size: 18.0pt") && !html.contains("font-size: 27.0pt"),
        "Expected fontScale >100% to clamp to 1.0 instead of enlarging text: {html}"
    );
}

#[test]
fn test_norm_autofit_line_spacing_reduction_is_clamped_to_one() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit lnSpcReduction="150000"/>
        </a:bodyPr>
        <a:p>
          <a:pPr><a:lnSpc><a:spcPct val="150000"/></a:lnSpc></a:pPr>
          <a:r><a:rPr sz="1800"/><a:t>Clamp spacing</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("line-height: 0.00"),
        "Expected lnSpcReduction >100% to clamp at 1.0 before rendering: {html}"
    );
}

#[test]
fn test_norm_autofit_cjk_sentence_does_not_force_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="CjkAutofit"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit fontScale="70000"/>
        </a:bodyPr>
        <a:p><a:r><a:rPr sz="1800"/><a:t>자동줄바꿈이가능한한글문장은긴토큰처럼취급되면안됩니다</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        !tb_chunk.contains("emergency-wrap"),
        "CJK autofit text should not opt into emergency wrapping by default: {tb_chunk}"
    );
    assert!(
        !tb_chunk.contains("overflow-wrap: anywhere"),
        "CJK autofit text should rely on natural line breaking, not emergency wrap: {tb_chunk}"
    );
}

#[test]
fn test_norm_autofit_fullwidth_sentence_does_not_force_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="FullwidthAutofit"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1800000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit fontScale="70000"/>
        </a:bodyPr>
        <a:p><a:r><a:rPr sz="1800"/><a:t>ＡＢＣＤＥＦＧＨＩＪ</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        !tb_chunk.contains("emergency-wrap"),
        "Fullwidth autofit text should not opt into emergency wrapping by default: {tb_chunk}"
    );
    assert!(
        !tb_chunk.contains("overflow-wrap: anywhere"),
        "Fullwidth autofit text should rely on natural line breaking, not emergency wrap: {tb_chunk}"
    );
}

#[test]
fn test_norm_autofit_mixed_script_sentence_does_not_force_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="MixedScriptAutofit"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="700000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit fontScale="70000"/>
        </a:bodyPr>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢Alpha漢Beta</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        !tb_chunk.contains("emergency-wrap"),
        "Mixed-script autofit text should use natural script-boundary line breaking before emergency wrapping: {tb_chunk}"
    );
    assert!(
        !tb_chunk.contains("overflow-wrap: anywhere"),
        "Mixed-script autofit text should not emit overflow-wrap:anywhere when script boundaries already provide breaks: {tb_chunk}"
    );
}

#[test]
fn test_norm_autofit_devanagari_combining_clusters_do_not_force_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="IndicAutofit"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="700000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit fontScale="70000"/>
        </a:bodyPr>
        <a:p><a:r><a:rPr sz="1800"><a:latin typeface="Calibri"/><a:cs typeface="Nirmala UI"/></a:rPr><a:t>क़क़क़</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        !tb_chunk.contains("emergency-wrap"),
        "Indic combining clusters should stay on normal wrap paths before emergency wrapping: {tb_chunk}"
    );
    assert!(
        !tb_chunk.contains("overflow-wrap: anywhere"),
        "Indic combining clusters should not emit overflow-wrap:anywhere when only combining marks inflate the measured width: {tb_chunk}"
    );
}

#[test]
fn test_cjk_nonstarter_punctuation_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="CjkPunctuationWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢、漢、漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "CJK non-starter punctuation clusters should opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "CJK non-starter punctuation clusters should emit overflow-wrap:anywhere when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_ideographic_full_stop_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="IdeographicFullStopWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢。漢。漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Ideographic full-stop clusters should opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Ideographic full-stop clusters should emit overflow-wrap:anywhere when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_fullwidth_comma_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="FullwidthCommaWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢，漢，漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Fullwidth comma clusters should opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Fullwidth comma clusters should emit overflow-wrap:anywhere when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_fullwidth_exclamation_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="FullwidthExclamationWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢！漢！漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Fullwidth exclamation clusters should opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Fullwidth exclamation clusters should emit overflow-wrap:anywhere when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_fullwidth_question_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="FullwidthQuestionWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢？漢？漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Fullwidth question clusters should opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Fullwidth question clusters should emit overflow-wrap:anywhere when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_fullwidth_colon_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="FullwidthColonWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢：漢：漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Fullwidth colon clusters should opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Fullwidth colon clusters should emit overflow-wrap:anywhere when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_fullwidth_semicolon_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="FullwidthSemicolonWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢；漢；漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Fullwidth semicolon clusters should opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Fullwidth semicolon clusters should emit overflow-wrap:anywhere when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_fullwidth_full_stop_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="FullwidthFullStopWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢．漢．漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Fullwidth full-stop clusters should opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Fullwidth full-stop clusters should emit overflow-wrap:anywhere when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_slash_separated_text_does_not_mark_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="SlashWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1000000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>Alpha/Beta/Gamma</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        !tb_chunk.contains("emergency-wrap"),
        "Slash-separated text should use ordinary break opportunities instead of emergency wrapping: {tb_chunk}"
    );
    assert!(
        !tb_chunk.contains("overflow-wrap: anywhere"),
        "Slash-separated text should not emit overflow-wrap:anywhere when normal slash breaks are available: {tb_chunk}"
    );
}

#[test]
fn test_hyphen_separated_text_does_not_mark_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="HyphenWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1000000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>Alpha-Beta-Gamma</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        !tb_chunk.contains("emergency-wrap"),
        "Hyphen-separated text should use ordinary break opportunities instead of emergency wrapping: {tb_chunk}"
    );
    assert!(
        !tb_chunk.contains("overflow-wrap: anywhere"),
        "Hyphen-separated text should not emit overflow-wrap:anywhere when normal hyphen breaks are available: {tb_chunk}"
    );
}

#[test]
fn test_cjk_opening_punctuation_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="CjkOpeningWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>（漢（漢（漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "CJK opening punctuation clusters should opt into emergency wrapping when the opener+glyph cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "CJK opening punctuation clusters should emit overflow-wrap:anywhere when the opener+glyph cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_cjk_angle_bracket_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="CjkAngleWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢》漢》漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "CJK angle-bracket punctuation clusters should opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "CJK angle-bracket punctuation clusters should emit overflow-wrap:anywhere when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_white_square_bracket_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="WhiteSquareWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>〚漢〛〚漢〛</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "White square bracket clusters should opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "White square bracket clusters should emit overflow-wrap:anywhere when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_tortoise_shell_bracket_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="TortoiseShellWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>〔漢〕〔漢〕</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Tortoise-shell bracket clusters should opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Tortoise-shell bracket clusters should emit overflow-wrap:anywhere when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_lenticular_bracket_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="LenticularWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>〘漢〙〘漢〙</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Lenticular bracket clusters should opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Lenticular bracket clusters should emit overflow-wrap:anywhere when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_mixed_font_unbreakable_token_spanning_runs_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="MixedFontWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1600000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:r><a:rPr sz="1800"><a:latin typeface="Calibri"/></a:rPr><a:t>overflow</a:t></a:r>
          <a:r><a:rPr sz="1800"><a:latin typeface="Aptos"/></a:rPr><a:t>detector</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Unbreakable mixed-font tokens that span runs should opt into emergency wrapping: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Emergency wrapping should emit overflow-wrap:anywhere for mixed-font split tokens: {tb_chunk}"
    );
}

#[test]
fn test_norm_autofit_inherited_paragraph_size_marks_mixed_font_token_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="InheritedParaSizeWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1800000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit fontScale="70000"/>
        </a:bodyPr>
        <a:p>
          <a:pPr><a:defRPr sz="2800"/></a:pPr>
          <a:r><a:rPr><a:latin typeface="Calibri"/></a:rPr><a:t>overflow</a:t></a:r>
          <a:r><a:rPr><a:latin typeface="Aptos"/></a:rPr><a:t>detector</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Paragraph default font size should influence mixed-font emergency wrapping under autofit: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Inherited paragraph font size should still produce overflow-wrap:anywhere when the combined token is too wide: {tb_chunk}"
    );
}

#[test]
fn test_norm_autofit_no_font_scale_no_overflow() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit/>
        </a:bodyPr>
        <a:p><a:r><a:rPr sz="1800"/><a:t>No scale</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    // No fontScale → text-body div should not have overflow:hidden in its style
    // Extract the text-body div's style to check (global CSS has overflow:hidden on .slide, so check inline)
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk = &html[tb_start..tb_start + 300.min(html.len() - tb_start)];
    assert!(
        !tb_chunk.contains("overflow: hidden"),
        "text-body should not have overflow:hidden when no fontScale, got: {tb_chunk}"
    );
    assert!(
        html.contains("font-size: 18.0pt"),
        "Font size should remain 18.0pt without fontScale"
    );
}

#[test]
fn test_font_resolution_ledger_tracks_theme_font_fallback() {
    let theme_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme">
  <a:themeElements>
    <a:clrScheme name="Office">
      <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
      <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="Office">
      <a:majorFont><a:latin typeface="Noto Sans KR"/></a:majorFont>
      <a:minorFont><a:latin typeface="Pretendard"/></a:minorFont>
    </a:fontScheme>
  </a:themeElements>
</a:theme>"#;

    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"><a:latin typeface="+mn-lt"/></a:rPr><a:t>Theme font fallback</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide)
        .with_full_theme(theme_xml)
        .build();
    let result = pptx2html_core::convert_bytes_with_metadata(&pptx).expect("conversion");
    let entry = result
        .font_resolution_entries
        .iter()
        .find(|entry| entry.run_text == "Theme font fallback")
        .expect("font ledger entry");

    assert_eq!(entry.requested_typeface.as_deref(), Some("+mn-lt"));
    assert_eq!(entry.resolved_typeface.as_deref(), Some("Pretendard"));
    assert!(entry.fallback_used);
}

#[test]
fn test_font_resolution_ledger_tracks_literal_font_without_fallback() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"><a:latin typeface="Calibri"/></a:rPr><a:t>Literal font</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let result = pptx2html_core::convert_bytes_with_metadata(&pptx).expect("conversion");
    let entry = result
        .font_resolution_entries
        .iter()
        .find(|entry| entry.run_text == "Literal font")
        .expect("font ledger entry");

    assert_eq!(entry.requested_typeface.as_deref(), Some("Calibri"));
    assert_eq!(entry.resolved_typeface.as_deref(), Some("Calibri"));
    assert!(!entry.fallback_used);
}

#[test]
fn test_font_resolution_ledger_tracks_complex_script_run_font() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="ArabicBox"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"><a:cs typeface="Amiri"/></a:rPr><a:t>مرحبا بالعالم</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let result = pptx2html_core::convert_bytes_with_metadata(&pptx).expect("conversion");
    let entry = result
        .font_resolution_entries
        .iter()
        .find(|entry| entry.run_text == "مرحبا بالعالم")
        .expect("font ledger entry");

    assert_eq!(entry.requested_typeface.as_deref(), Some("Amiri"));
    assert_eq!(entry.resolved_typeface.as_deref(), Some("Amiri"));
    assert!(!entry.fallback_used);
    assert!(result.html.contains("font-family: 'Amiri'"));
}

#[test]
fn test_font_resolution_ledger_tracks_complex_script_paragraph_default_font() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="ArabicDefaultBox"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:pPr><a:defRPr sz="1800"><a:cs typeface="Scheherazade New"/></a:defRPr></a:pPr>
          <a:r><a:t>السلام عليكم</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let result = pptx2html_core::convert_bytes_with_metadata(&pptx).expect("conversion");
    let entry = result
        .font_resolution_entries
        .iter()
        .find(|entry| entry.run_text == "السلام عليكم")
        .expect("font ledger entry");

    assert_eq!(
        entry.requested_typeface.as_deref(),
        Some("Scheherazade New")
    );
    assert_eq!(entry.resolved_typeface.as_deref(), Some("Scheherazade New"));
    assert_eq!(
        entry.source,
        Some(pptx2html_core::FontResolutionSource::ParagraphDefaults)
    );
    assert!(result.html.contains("font-family: 'Scheherazade New'"));
}

#[test]
fn test_font_resolution_ledger_prefers_complex_script_font_for_devanagari() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="IndicBox"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"><a:latin typeface="Calibri"/><a:cs typeface="Nirmala UI"/></a:rPr><a:t>नमस्ते दुनिया</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let result = pptx2html_core::convert_bytes_with_metadata(&pptx).expect("conversion");
    let entry = result
        .font_resolution_entries
        .iter()
        .find(|entry| entry.run_text == "नमस्ते दुनिया")
        .expect("font ledger entry");

    assert_eq!(entry.requested_typeface.as_deref(), Some("Nirmala UI"));
    assert_eq!(entry.resolved_typeface.as_deref(), Some("Nirmala UI"));
    assert!(result.html.contains("font-family: 'Nirmala UI'"));
}

#[test]
fn test_font_resolution_ledger_tracks_theme_complex_script_font_fallback() {
    let theme_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme">
  <a:themeElements>
    <a:clrScheme name="Office">
      <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
      <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="Office">
      <a:majorFont>
        <a:latin typeface="Noto Sans"/>
        <a:cs typeface="Times New Roman"/>
      </a:majorFont>
      <a:minorFont>
        <a:latin typeface="Pretendard"/>
        <a:cs typeface="Amiri"/>
      </a:minorFont>
    </a:fontScheme>
  </a:themeElements>
</a:theme>"#;

    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="ThemeArabicBox"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"><a:cs typeface="+mn-cs"/></a:rPr><a:t>مرحبا بالسمة</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide)
        .with_full_theme(theme_xml)
        .build();
    let result = pptx2html_core::convert_bytes_with_metadata(&pptx).expect("conversion");
    let entry = result
        .font_resolution_entries
        .iter()
        .find(|entry| entry.run_text == "مرحبا بالسمة")
        .expect("font ledger entry");

    assert_eq!(entry.requested_typeface.as_deref(), Some("+mn-cs"));
    assert_eq!(entry.resolved_typeface.as_deref(), Some("Amiri"));
    assert!(entry.fallback_used);
    assert!(result.html.contains("font-family: 'Amiri'"));
}

#[test]
fn test_mixed_script_single_run_splits_font_family_segments() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="MixedRunBox"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"><a:latin typeface="Calibri"/><a:cs typeface="Amiri"/></a:rPr><a:t>Hello مرحبا world</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);

    assert!(html.contains("font-family: 'Calibri'"));
    assert!(html.contains("font-family: 'Amiri'"));
    assert!(
        html.contains("Hello"),
        "latin content should survive as its own segment: {html}"
    );
    assert!(
        html.contains("مرحبا"),
        "arabic content should survive as its own segment: {html}"
    );
    assert!(
        html.contains("world"),
        "trailing latin content should survive as its own segment: {html}"
    );
}

#[test]
fn test_mixed_script_single_run_splits_theme_complex_script_segments() {
    let theme_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Office Theme">
  <a:themeElements>
    <a:clrScheme name="Office">
      <a:dk1><a:sysClr val="windowText" lastClr="000000"/></a:dk1>
      <a:lt1><a:sysClr val="window" lastClr="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="Office">
      <a:majorFont>
        <a:latin typeface="Aptos Display"/>
        <a:cs typeface="Times New Roman"/>
      </a:majorFont>
      <a:minorFont>
        <a:latin typeface="Aptos"/>
        <a:cs typeface="Amiri"/>
      </a:minorFont>
    </a:fontScheme>
  </a:themeElements>
</a:theme>"#;

    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="ThemeMixedRunBox"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"><a:latin typeface="+mn-lt"/><a:cs typeface="+mn-cs"/></a:rPr><a:t>Hello مرحبا</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide)
        .with_full_theme(theme_xml)
        .build();
    let html = render_html(&pptx);

    assert!(html.contains("font-family: 'Aptos'"));
    assert!(html.contains("font-family: 'Amiri'"));
    assert!(
        html.contains("Hello"),
        "latin themed segment should survive: {html}"
    );
    assert!(
        html.contains("مرحبا"),
        "arabic themed segment should survive: {html}"
    );
}

#[test]
fn test_mixed_script_single_run_keeps_emoji_zwj_cluster_in_single_segment() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="EmojiClusterBox"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"><a:latin typeface="Calibri"/><a:cs typeface="Segoe UI Emoji"/></a:rPr><a:t>Hello 👩‍💻 مرحبا</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);

    assert!(
        html.contains("👩‍💻"),
        "emoji cluster text should survive intact: {html}"
    );
    assert!(
        html.contains("font-family: 'Segoe UI Emoji'"),
        "emoji cluster should use complex-script/emoji font path: {html}"
    );
    assert!(
        !html.contains("👩</span><span class=\"run-segment\""),
        "emoji cluster must not split at ZWJ boundaries: {html}"
    );
}

#[test]
fn test_mixed_script_single_run_uses_emoji_font_for_emoji_segment() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="EmojiFontBox"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"><a:latin typeface="Calibri"/><a:cs typeface="Segoe UI Emoji"/></a:rPr><a:t>A👩‍💻B</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);

    let emoji_segment_idx = html.find("👩‍💻</span>").expect("emoji cluster segment");
    let emoji_window = &html[emoji_segment_idx.saturating_sub(120)..emoji_segment_idx + 24];
    assert!(
        emoji_window.contains("Segoe UI Emoji"),
        "emoji cluster should resolve to emoji font, not latin fallback: {emoji_window}"
    );

    let leading_a_idx = html.find(">A</span>").expect("leading latin segment");
    let leading_window = &html[leading_a_idx.saturating_sub(120)..leading_a_idx + 10];
    assert!(
        leading_window.contains("Calibri"),
        "leading latin segment should keep latin font: {leading_window}"
    );

    let trailing_b_idx = html.find(">B</span>").expect("trailing latin segment");
    let trailing_window = &html[trailing_b_idx.saturating_sub(120)..trailing_b_idx + 10];
    assert!(
        trailing_window.contains("Calibri"),
        "trailing latin segment should keep latin font: {trailing_window}"
    );
}

#[test]
fn test_rtl_paragraph_is_parsed_and_rendered() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="RTL Box"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="4000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:pPr rtl="1"/>
          <a:r><a:t>مرحبا PowerPoint 2026 بالعالم</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    assert!(
        pres.slides[0].shapes[0]
            .text_body
            .as_ref()
            .unwrap()
            .paragraphs[0]
            .rtl
    );

    let html = render_html(&pptx);
    assert!(
        html.contains("direction: rtl"),
        "Expected RTL direction in HTML: {html}"
    );
    assert!(
        html.contains("unicode-bidi: isolate"),
        "Expected isolated Unicode bidi layout in HTML: {html}"
    );
    assert!(
        !html.contains("unicode-bidi: bidi-override"),
        "RTL paragraphs must preserve mixed-script Unicode bidi ordering: {html}"
    );
}

#[test]
fn test_vertical_text_with_flip_keeps_combined_transform() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="VertFlip"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm flipH="1"><a:off x="100000" y="100000"/><a:ext cx="1000000" cy="3000000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr vert="vert270"/>
        <a:p><a:r><a:t>Rotated</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk = &html[tb_start..tb_start + 300.min(html.len() - tb_start)];
    assert!(tb_chunk.contains("writing-mode: vertical-lr"));
    assert!(
        tb_chunk.contains("scale(-1,1)"),
        "Expected flip transform: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("rotate(180deg)"),
        "Expected vert270 rotation: {tb_chunk}"
    );
}

#[test]
fn test_no_wrap_sets_inline_white_space_nowrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="NoWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr wrap="none"/>
        <a:p><a:r><a:t>No wrap text</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk = &html[tb_start..tb_start + 250.min(html.len() - tb_start)];
    assert!(
        tb_chunk.contains("white-space: nowrap"),
        "Expected inline nowrap style on text-body: {tb_chunk}"
    );
    assert!(
        html.contains(".text-body.nowrap .run { white-space: inherit; word-break: normal; overflow-wrap: normal; }"),
        "Expected a nowrap-specific run override rule in global CSS: {html}"
    );
    assert!(
        tb_chunk.contains("class=\"text-body") && tb_chunk.contains("nowrap"),
        "Expected nowrap text bodies to carry a dedicated class for child run overrides: {tb_chunk}"
    );
}

#[test]
fn test_default_body_pr_does_not_force_nowrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="DefaultWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>Default wrap text</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk = &html[tb_start..tb_start + 250.min(html.len() - tb_start)];
    assert!(
        !tb_chunk.contains("white-space: nowrap"),
        "Default bodyPr wrap should remain square/wrapped unless wrap='none' is explicit: {tb_chunk}"
    );
}

#[test]
fn test_wrapped_text_body_emits_overflow_wrap_anywhere() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="WrapAnywhere"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1800000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>SupercalifragilisticexpialidociousWithoutSpaces</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk = &html[tb_start..tb_start + 320.min(html.len() - tb_start)];
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Expected wrapped text body to opt into emergency line breaking: {tb_chunk}"
    );
}

#[test]
fn test_wrapped_text_body_does_not_force_emergency_wrap_for_regular_sentence() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="RegularWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2600000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>This sentence should wrap at spaces before emergency breaking is needed.</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk = &html[tb_start..tb_start + 320.min(html.len() - tb_start)];
    assert!(
        !tb_chunk.contains("overflow-wrap: anywhere"),
        "Regular wrapped sentences should not opt into emergency breaking: {tb_chunk}"
    );
    assert!(
        !tb_chunk.contains("emergency-wrap"),
        "Regular wrapped sentences should not carry an emergency-wrap marker: {tb_chunk}"
    );
}

#[test]
fn test_unbreakable_text_body_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="EmergencyWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>SupercalifragilisticexpialidociousWithoutSpaces</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk = &html[tb_start..tb_start + 360.min(html.len() - tb_start)];
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Unbreakable tokens should still opt into emergency wrapping: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Unbreakable tokens should carry an emergency-wrap marker: {tb_chunk}"
    );
}

#[test]
fn test_nbsp_separated_text_body_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="NbspWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1700000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>Alpha&#160;Beta&#160;Gamma</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "NBSP-separated text should still opt into emergency wrapping when the non-breaking token is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "NBSP-separated text should carry an emergency-wrap marker when the non-breaking token is too wide: {tb_chunk}"
    );
}

#[test]
fn test_soft_hyphen_text_body_does_not_mark_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="SoftHyphenWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1700000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>Alpha&#173;Beta&#173;Gamma</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        !tb_chunk.contains("overflow-wrap: anywhere"),
        "Soft-hyphenated text should use ordinary break opportunities instead of emergency wrapping: {tb_chunk}"
    );
    assert!(
        !tb_chunk.contains("emergency-wrap"),
        "Soft-hyphenated text should not carry an emergency-wrap marker when normal break opportunities exist: {tb_chunk}"
    );
}

#[test]
fn test_fullwidth_closing_parenthesis_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="FullwidthCloseParenWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢）漢）漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Fullwidth closing-parenthesis clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Fullwidth closing-parenthesis clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_ascii_closing_parenthesis_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="AsciiCloseParenWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="350000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>A)A)A)A</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "ASCII closing-parenthesis clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "ASCII closing-parenthesis clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_fullwidth_closing_square_bracket_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="FullwidthCloseSquareWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢］漢］漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Fullwidth closing-square-bracket clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Fullwidth closing-square-bracket clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_fullwidth_closing_curly_bracket_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="FullwidthCloseCurlyWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢｝漢｝漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Fullwidth closing-curly-bracket clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Fullwidth closing-curly-bracket clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_fullwidth_opening_square_bracket_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="FullwidthOpenSquareWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>［漢［漢［漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Fullwidth opening-square-bracket clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Fullwidth opening-square-bracket clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_fullwidth_opening_curly_bracket_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="FullwidthOpenCurlyWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>｛漢｛漢｛漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Fullwidth opening-curly-bracket clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Fullwidth opening-curly-bracket clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_black_square_bracket_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="BlackSquareBracketWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>【漢】【漢】</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Black square bracket clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Black square bracket clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_black_lenticular_bracket_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="BlackLenticularBracketWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>〖漢〗〖漢〗</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Black lenticular bracket clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Black lenticular bracket clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_corner_quote_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="CornerQuoteWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>「漢」「漢」</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Corner quote clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Corner quote clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_double_corner_quote_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="DoubleCornerQuoteWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>『漢』『漢』</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Double corner quote clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Double corner quote clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_reversed_double_prime_quote_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="ReversedDoublePrimeQuoteWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>〝漢〞〝漢〞</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Reversed double-prime quote clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Reversed double-prime quote clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_low_reversed_double_prime_quote_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="LowReversedDoublePrimeQuoteWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>〝漢〟〝漢〟</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Low reversed double-prime quote clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Low reversed double-prime quote clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_closing_corner_quote_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="ClosingCornerQuoteWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢」漢」漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Closing corner quote clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Closing corner quote clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_closing_double_corner_quote_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="ClosingDoubleCornerQuoteWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢』漢』漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Closing double corner quote clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Closing double corner quote clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_opening_angle_bracket_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="OpeningAngleBracketWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>《漢《漢《漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Opening angle-bracket clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Opening angle-bracket clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_fullwidth_opening_angle_bracket_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="FullwidthOpeningAngleWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>＜漢＜漢＜漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Fullwidth opening angle-bracket clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Fullwidth opening angle-bracket clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_fullwidth_closing_angle_bracket_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="FullwidthClosingAngleWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢＞漢＞漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Fullwidth closing angle-bracket clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Fullwidth closing angle-bracket clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_single_closing_angle_bracket_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="SingleClosingAngleBracketWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢〉漢〉漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Single closing angle-bracket clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Single closing angle-bracket clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_wave_dash_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="WaveDashWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢〜漢〜漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Wave-dash clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Wave-dash clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_fullwidth_tilde_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="FullwidthTildeWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>漢～漢～漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Fullwidth tilde clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Fullwidth tilde clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_single_angle_bracket_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="SingleAngleBracketWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>〈漢〈漢〈漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Single angle-bracket clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Single angle-bracket clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_fullwidth_opening_parenthesis_cluster_marks_emergency_wrap() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="FullwidthOpenParenWrap"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="500000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>（漢（漢（漢</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();
    assert!(
        tb_chunk.contains("overflow-wrap: anywhere"),
        "Fullwidth opening-parenthesis clusters should still opt into emergency wrapping when the cluster is too wide: {tb_chunk}"
    );
    assert!(
        tb_chunk.contains("emergency-wrap"),
        "Fullwidth opening-parenthesis clusters should carry an emergency-wrap marker when the cluster is too wide: {tb_chunk}"
    );
}

#[test]
fn test_run_without_any_size_uses_hardcoded_default_font_size() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="DefaultSize"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>Default size text</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("font-size: 18.0pt"),
        "Expected renderer hardcoded default font size of 18pt when no size is specified: {html}"
    );
}

#[test]
fn test_run_cap_all_renders_uppercase_transform() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="AllCaps"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr cap="all"/><a:t>All caps text</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("text-transform: uppercase"),
        "Expected cap='all' to render uppercase transform: {html}"
    );
}

#[test]
fn test_run_cap_small_renders_small_caps_variant() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="SmallCaps"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr cap="small"/><a:t>Small caps text</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("font-variant: small-caps"),
        "Expected cap='small' to render small-caps variant: {html}"
    );
}

#[test]
fn test_body_pr_anchor_ctr_renders_horizontal_centering() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="AnchorCtr"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr anchorCtr="1"/>
        <a:p><a:r><a:t>Centered body</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains(".text-body.h-center { align-items: center; }")
            && html.contains("class=\"text-body v-top h-center\""),
        "Expected anchorCtr to add horizontal centering class and CSS rule: {html}"
    );
}

#[test]
fn test_body_pr_rotation_renders_text_transform() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="TextRot"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr rot="5400000"/>
        <a:p><a:r><a:t>Rotated body</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("transform: rotate(90.0deg)"),
        "Expected bodyPr rot to render a text-body rotation transform: {html}"
    );
}
