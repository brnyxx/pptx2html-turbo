//! Integration tests for hierarchy inheritance, style refs, and rendering

mod fixtures;

use pptx2html_core::model::*;
use pptx2html_core::{ProvenanceSource, ProvenanceSubject};

fn parse_pptx(data: &[u8]) -> pptx2html_core::model::Presentation {
    pptx2html_core::parser::PptxParser::parse_bytes(data).expect("PPTX parsing failed")
}

fn render_html(data: &[u8]) -> String {
    let pres = parse_pptx(data);
    pptx2html_core::renderer::HtmlRenderer::render(&pres).expect("HTML rendering failed")
}

// ── Background inheritance tests ──
// Note: Master/layout background parsing from <p:bg> is not yet implemented in parsers.
// These tests verify the inheritance resolution logic works via the model API.

#[test]
fn test_background_default_white() {
    // When no master/layout provides bg, slide gets default white background
    let pptx = fixtures::MinimalPptx::new("").build();
    let html = render_html(&pptx);
    assert!(
        html.contains("#FFFFFF"),
        "Default white background not found: {html}"
    );
}

#[test]
fn test_background_slide_explicit() {
    // Slide with explicit background fill as a shape fill
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Bg"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="0" y="0"/><a:ext cx="9144000" cy="6858000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:solidFill><a:srgbClr val="FF1122"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("#FF1122"),
        "Explicit slide fill not found: {html}"
    );
}

// ── Placeholder matching test ──

#[test]
fn test_title_placeholder_inherits_position() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let _layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title"/>
        <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    // Slide with title placeholder but no position (should inherit from layout)
    let _slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title 1"/>
        <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="2400"/><a:t>Inherited Position</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(_slide_body)
        .with_full_master(master_xml)
        .with_layout(_layout_xml)
        .build();
    let pres = parse_pptx(&pptx);

    // Verify the shape exists and has text
    assert_eq!(pres.slides[0].shapes.len(), 1);
    let shape = &pres.slides[0].shapes[0];
    assert!(shape.placeholder.is_some());

    // Position should be inherited from layout (457200 EMU -> ~48px)
    let html = render_html(&pptx);
    assert!(
        html.contains("Inherited Position"),
        "Shape text not found: {html}"
    );
    // 8229600 EMU = 864px width
    assert!(
        html.contains("864.0px"),
        "Layout width not inherited: {html}"
    );
}

#[test]
fn test_layout_placeholder_inherits_geometry_adjust_values_and_transform() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Body Shape"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm rot="1800000" flipH="1">
          <a:off x="914400" y="914400"/>
          <a:ext cx="1828800" cy="914400"/>
        </a:xfrm>
        <a:prstGeom prst="rightArrow">
          <a:avLst>
            <a:gd name="adj1" fmla="val 25000"/>
            <a:gd name="adj2" fmla="val 30000"/>
          </a:avLst>
        </a:prstGeom>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Body Placeholder"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>Inherited Geometry</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let html = render_html(&pptx);

    assert!(
        html.contains("Inherited Geometry"),
        "placeholder text missing: {html}"
    );
    assert!(
        html.contains("class=\"shape-svg\""),
        "expected inherited SVG geometry: {html}"
    );
    assert!(
        html.contains(
            "d=\"M0.00,36.00 L163.20,36.00 L163.20,0.00 L192.00,48.00 L163.20,96.00 L163.20,60.00 L0.00,60.00 Z\""
        ),
        "expected inherited rightArrow path with layout adjust values: {html}"
    );
    assert!(
        html.contains("transform: scale(-1,1) rotate(30.0deg)"),
        "expected inherited layout transform for placeholder geometry: {html}"
    );
}

#[test]
fn test_layout_placeholder_inherits_border_properties() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title"/>
        <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:ln w="19050">
          <a:solidFill><a:srgbClr val="00AAFF"/></a:solidFill>
          <a:prstDash val="dash"/>
        </a:ln>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title 1"/>
        <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="2400"/><a:t>Inherited Layout Border</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let html = render_html(&pptx);

    assert!(
        html.contains("outline: 1.5pt dashed #00AAFF"),
        "Layout border should be inherited by placeholder shape: {html}"
    );
}

#[test]
fn test_master_placeholder_inherits_border_when_layout_missing_it() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title"/>
        <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:ln w="12700">
          <a:solidFill><a:srgbClr val="AA5500"/></a:solidFill>
          <a:prstDash val="dot"/>
        </a:ln>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title 1"/>
        <p:cNvSpPr><a:spLocks noGrp="1"/></p:cNvSpPr>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="2400"/><a:t>Inherited Master Border</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let html = render_html(&pptx);

    assert!(
        html.contains("outline: 1.0pt dotted #AA5500"),
        "Master border should be inherited when layout has no matching placeholder: {html}"
    );
}

// ── Paragraph spacing tests ──

#[test]
fn test_line_spacing_percentage_rendering() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="2000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:pPr>
            <a:lnSpc><a:spcPct val="150000"/></a:lnSpc>
          </a:pPr>
          <a:r><a:rPr sz="1800"/><a:t>150% Line Spacing</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let para = &pres.slides[0].shapes[0]
        .text_body
        .as_ref()
        .unwrap()
        .paragraphs[0];
    assert!(matches!(para.line_spacing, Some(SpacingValue::Percent(p)) if (p - 1.5).abs() < 0.01));

    let html = render_html(&pptx);
    assert!(
        html.contains("line-height: 1.50"),
        "150% line spacing not found: {html}"
    );
}

#[test]
fn test_line_spacing_points_rendering() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="2000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:pPr>
            <a:lnSpc><a:spcPts val="2400"/></a:lnSpc>
          </a:pPr>
          <a:r><a:rPr sz="1800"/><a:t>24pt Line Spacing</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let para = &pres.slides[0].shapes[0]
        .text_body
        .as_ref()
        .unwrap()
        .paragraphs[0];
    assert!(matches!(para.line_spacing, Some(SpacingValue::Points(p)) if (p - 24.0).abs() < 0.1));

    let html = render_html(&pptx);
    assert!(
        html.contains("line-height: 24.0pt"),
        "24pt line spacing not found: {html}"
    );
}

#[test]
fn test_space_before_after_rendering() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="2000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:pPr>
            <a:spcBef><a:spcPts val="1200"/></a:spcBef>
            <a:spcAft><a:spcPts val="600"/></a:spcAft>
          </a:pPr>
          <a:r><a:rPr sz="1800"/><a:t>Spaced Paragraph</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let para = &pres.slides[0].shapes[0]
        .text_body
        .as_ref()
        .unwrap()
        .paragraphs[0];
    assert!(matches!(para.space_before, Some(SpacingValue::Points(p)) if (p - 12.0).abs() < 0.1));
    assert!(matches!(para.space_after, Some(SpacingValue::Points(p)) if (p - 6.0).abs() < 0.1));

    let html = render_html(&pptx);
    assert!(
        html.contains("margin-top: 12.0pt"),
        "Space before not found: {html}"
    );
    assert!(
        html.contains("margin-bottom: 6.0pt"),
        "Space after not found: {html}"
    );
}

// ── Master shapes test ──

#[test]
fn test_show_master_sp_default_true() {
    // Master has both a placeholder shape (ftr) and a non-placeholder decorative shape.
    // Only the non-placeholder shape should render; placeholder shapes are
    // property-inheritance sources, not renderable content.
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="10" name="Footer"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="ftr"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="6000000"/><a:ext cx="3000000" cy="500000"/></a:xfrm>
      </p:spPr>
    </p:sp>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="20" name="Decorative Bar"/>
        <p:cNvSpPr/>
        <p:nvPr/>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="0" y="6500000"/><a:ext cx="9144000" cy="358000"/></a:xfrm>
        <a:solidFill><a:srgbClr val="0070C0"/></a:solidFill>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Footer Placeholder"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="ftr"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="6000000"/><a:ext cx="3000000" cy="500000"/></a:xfrm>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let pres = parse_pptx(&pptx);

    // Master should have both shapes parsed
    assert!(!pres.masters.is_empty(), "No masters parsed");
    assert_eq!(
        pres.masters[0].shapes.len(),
        2,
        "Master should have 2 shapes"
    );

    // Only the non-placeholder decorative shape should render
    let html = render_html(&pptx);
    let shape_count = html.matches("class=\"shape\"").count();
    assert_eq!(
        shape_count, 1,
        "Expected exactly 1 non-placeholder master shape rendered, got {shape_count}: {html}"
    );
}

#[test]
fn test_master_decorative_shape_preserves_geometry_adjust_values_and_transform() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="20" name="Master Arrow"/>
        <p:cNvSpPr/>
        <p:nvPr/>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm rot="1800000" flipH="1">
          <a:off x="914400" y="914400"/>
          <a:ext cx="1828800" cy="914400"/>
        </a:xfrm>
        <a:prstGeom prst="rightArrow">
          <a:avLst>
            <a:gd name="adj1" fmla="val 25000"/>
            <a:gd name="adj2" fmla="val 30000"/>
          </a:avLst>
        </a:prstGeom>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let html = render_html(&pptx);

    assert!(
        html.contains("class=\"shape-svg\""),
        "expected master decorative geometry SVG: {html}"
    );
    assert!(
        html.contains(
            "d=\"M0.00,36.00 L163.20,36.00 L163.20,0.00 L192.00,48.00 L163.20,96.00 L163.20,60.00 L0.00,60.00 Z\""
        ),
        "expected master decorative rightArrow path with adjust values: {html}"
    );
    assert!(
        html.contains("transform: scale(-1,1) rotate(30.0deg)"),
        "expected master decorative transform to survive parsing: {html}"
    );
}

#[test]
fn test_master_placeholder_not_rendered() {
    // Master has only a placeholder shape -- nothing should render
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="10" name="Footer"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="ftr"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="6000000"/><a:ext cx="3000000" cy="500000"/></a:xfrm>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Footer Placeholder"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="ftr"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="6000000"/><a:ext cx="3000000" cy="500000"/></a:xfrm>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();

    let html = render_html(&pptx);
    assert!(
        !html.contains("class=\"shape\""),
        "Master placeholder shape should not render as standalone HTML: {html}"
    );
}

#[test]
fn test_master_placeholder_hidden_when_layout_omits_it() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="10" name="Footer"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="ftr"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="6000000"/><a:ext cx="3000000" cy="500000"/></a:xfrm>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    // Layout has zero placeholder shapes -> master placeholder is hidden
    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();

    let html = render_html(&pptx);
    // Layout omits ftr placeholder -> master footer should not render
    assert!(
        !html.contains("class=\"shape\""),
        "Master placeholder rendered despite layout omitting it: {html}"
    );
}

// ── defaultTextStyle test ──

#[test]
fn test_default_text_style_parsed() {
    let pres_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
  <p:defaultTextStyle>
    <a:lvl1pPr algn="l">
      <a:defRPr sz="1800" b="0"/>
    </a:lvl1pPr>
    <a:lvl2pPr algn="l">
      <a:defRPr sz="1600"/>
    </a:lvl2pPr>
  </p:defaultTextStyle>
</p:presentation>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_presentation_xml(pres_xml)
        .build();
    let pres = parse_pptx(&pptx);

    let dts = pres
        .default_text_style
        .as_ref()
        .expect("defaultTextStyle not parsed");
    let lvl1 = dts.levels[0].as_ref().expect("Level 1 not parsed");
    assert!(matches!(lvl1.alignment, Some(Alignment::Left)));
    let run_defaults = lvl1.def_run_props.as_ref().expect("defRPr not parsed");
    assert_eq!(run_defaults.font_size, Some(18.0));
    assert_eq!(run_defaults.bold, Some(false));

    let lvl2 = dts.levels[1].as_ref().expect("Level 2 not parsed");
    let rd2 = lvl2
        .def_run_props
        .as_ref()
        .expect("Level 2 defRPr not parsed");
    assert_eq!(rd2.font_size, Some(16.0));
}

// ── Shape style reference parsing test ──

#[test]
fn test_shape_style_ref_parsing() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:style>
        <a:lnRef idx="2"><a:schemeClr val="accent1"/></a:lnRef>
        <a:fillRef idx="1"><a:schemeClr val="accent1"/></a:fillRef>
        <a:effectRef idx="0"><a:schemeClr val="accent1"/></a:effectRef>
        <a:fontRef idx="minor"><a:schemeClr val="dk1"/></a:fontRef>
      </p:style>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>Style Ref Test</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    let style_ref = shape.style_ref.as_ref().expect("style_ref not parsed");
    let fill_ref = style_ref.fill_ref.as_ref().expect("fillRef not parsed");
    assert_eq!(fill_ref.idx, 1);
    assert!(matches!(fill_ref.color.kind, color::ColorKind::Theme(ref n) if n == "accent1"));

    let ln_ref = style_ref.ln_ref.as_ref().expect("lnRef not parsed");
    assert_eq!(ln_ref.idx, 2);

    let effect_ref = style_ref.effect_ref.as_ref().expect("effectRef not parsed");
    assert_eq!(effect_ref.idx, 0);

    let font_ref = style_ref.font_ref.as_ref().expect("fontRef not parsed");
    assert_eq!(font_ref.idx, "minor");
    assert!(matches!(font_ref.color.kind, color::ColorKind::Theme(ref n) if n == "dk1"));
}

// ── Style ref fill resolution in rendering ──

#[test]
fn test_style_ref_fill_rendered() {
    // Theme with fmtScheme fillStyleLst
    let theme_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="TestTheme">
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
      <a:majorFont><a:latin typeface="Calibri Light"/></a:majorFont>
      <a:minorFont><a:latin typeface="Calibri"/></a:minorFont>
    </a:fontScheme>
    <a:fmtScheme name="Office">
      <a:fillStyleLst>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
        <a:solidFill><a:srgbClr val="AABBCC"/></a:solidFill>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
      </a:fillStyleLst>
      <a:lnStyleLst>
        <a:ln w="9525"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
        <a:ln w="19050"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
      </a:lnStyleLst>
      <a:effectStyleLst>
        <a:effectStyle><a:effectLst/></a:effectStyle>
      </a:effectStyleLst>
      <a:bgFillStyleLst>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
      </a:bgFillStyleLst>
    </a:fmtScheme>
  </a:themeElements>
</a:theme>"#;

    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    // Shape with fillRef idx=1 and accent1 color, no explicit fill
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="StyleRef"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:style>
        <a:lnRef idx="0"><a:schemeClr val="accent1"/></a:lnRef>
        <a:fillRef idx="1"><a:schemeClr val="accent1"/></a:fillRef>
        <a:effectRef idx="0"><a:schemeClr val="accent1"/></a:effectRef>
        <a:fontRef idx="minor"><a:schemeClr val="dk1"/></a:fontRef>
      </p:style>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>Filled via StyleRef</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide)
        .with_full_theme(theme_xml)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let html = render_html(&pptx);
    // fillRef idx=1 with accent1 -> phClr placeholder replaced by accent1 -> #4472C4
    assert!(
        html.contains("#4472C4"),
        "StyleRef fill color not rendered: {html}"
    );
}

// ── Style ref with explicit fill override test ──

#[test]
fn test_explicit_fill_overrides_style_ref() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:solidFill><a:srgbClr val="DD0000"/></a:solidFill>
      </p:spPr>
      <p:style>
        <a:lnRef idx="0"><a:schemeClr val="accent1"/></a:lnRef>
        <a:fillRef idx="1"><a:schemeClr val="accent1"/></a:fillRef>
        <a:effectRef idx="0"><a:schemeClr val="accent1"/></a:effectRef>
        <a:fontRef idx="minor"><a:schemeClr val="dk1"/></a:fontRef>
      </p:style>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    // Explicit fill should take priority over style ref
    assert!(
        html.contains("#DD0000"),
        "Explicit fill should override style ref: {html}"
    );
}

// ── Theme FmtScheme parsing test ──

#[test]
fn test_fmt_scheme_parsed() {
    let theme_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Test">
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
      <a:majorFont><a:latin typeface="Calibri Light"/></a:majorFont>
      <a:minorFont><a:latin typeface="Calibri"/></a:minorFont>
    </a:fontScheme>
    <a:fmtScheme name="Office">
      <a:fillStyleLst>
        <a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>
        <a:solidFill><a:srgbClr val="00FF00"/></a:solidFill>
      </a:fillStyleLst>
      <a:lnStyleLst>
        <a:ln w="12700"><a:solidFill><a:srgbClr val="0000FF"/></a:solidFill></a:ln>
      </a:lnStyleLst>
      <a:effectStyleLst>
        <a:effectStyle><a:effectLst/></a:effectStyle>
      </a:effectStyleLst>
      <a:bgFillStyleLst>
        <a:solidFill><a:srgbClr val="CCCCCC"/></a:solidFill>
      </a:bgFillStyleLst>
    </a:fmtScheme>
  </a:themeElements>
</a:theme>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_full_theme(theme_xml)
        .build();
    let pres = parse_pptx(&pptx);
    let theme = pres.primary_theme().expect("Theme not found");

    assert_eq!(theme.fmt_scheme.fill_style_lst.len(), 2);
    assert_eq!(theme.fmt_scheme.ln_style_lst.len(), 1);
    assert_eq!(theme.fmt_scheme.bg_fill_style_lst.len(), 1);

    // Check line style width (12700 EMU = 1pt)
    assert!((theme.fmt_scheme.ln_style_lst[0].width - 1.0).abs() < 0.1);
}

// ── Slide with background percentage space ──

#[test]
fn test_space_before_percentage() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="2000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:pPr>
            <a:spcBef><a:spcPct val="50000"/></a:spcBef>
          </a:pPr>
          <a:r><a:rPr sz="1800"/><a:t>50% Space Before</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let para = &pres.slides[0].shapes[0]
        .text_body
        .as_ref()
        .unwrap()
        .paragraphs[0];
    assert!(matches!(para.space_before, Some(SpacingValue::Percent(p)) if (p - 0.5).abs() < 0.01));

    let html = render_html(&pptx);
    assert!(
        html.contains("margin-top: 0.5em"),
        "50% space before not found: {html}"
    );
}

// ── Shape with no text body still renders ──

#[test]
fn test_shape_no_text_body() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:solidFill><a:srgbClr val="FFAA00"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    // Shape should have fill but no text body
    assert!(matches!(shape.fill, Fill::Solid(_)));
    assert!(
        shape.text_body.is_none(),
        "text_body should be None for shape without txBody"
    );

    let html = render_html(&pptx);
    assert!(html.contains("#FFAA00"), "Shape fill not rendered: {html}");
}

// ── Layout type parsed test ──

#[test]
fn test_layout_type_parsed() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
             type="title">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let pres = parse_pptx(&pptx);
    assert!(!pres.layouts.is_empty(), "No layouts parsed");
    assert_eq!(pres.layouts[0].layout_type.as_deref(), Some("title"));
}

// ── Multiple paragraphs with different styles ──

#[test]
fn test_multiple_paragraphs_spacing() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:pPr algn="l">
            <a:lnSpc><a:spcPct val="120000"/></a:lnSpc>
          </a:pPr>
          <a:r><a:rPr sz="1800"/><a:t>First paragraph</a:t></a:r>
        </a:p>
        <a:p>
          <a:pPr algn="ctr">
            <a:lnSpc><a:spcPts val="1800"/></a:lnSpc>
            <a:spcBef><a:spcPts val="600"/></a:spcBef>
          </a:pPr>
          <a:r><a:rPr sz="2400" b="1"/><a:t>Second paragraph</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let paras = &pres.slides[0].shapes[0]
        .text_body
        .as_ref()
        .unwrap()
        .paragraphs;
    assert_eq!(paras.len(), 2);

    // First paragraph: left aligned, 120% line spacing
    assert!(matches!(paras[0].alignment, Alignment::Left));
    assert!(
        matches!(paras[0].line_spacing, Some(SpacingValue::Percent(p)) if (p - 1.2).abs() < 0.01)
    );

    // Second paragraph: center aligned, 18pt line spacing, 6pt space before
    assert!(matches!(paras[1].alignment, Alignment::Center));
    assert!(
        matches!(paras[1].line_spacing, Some(SpacingValue::Points(p)) if (p - 18.0).abs() < 0.1)
    );
    assert!(
        matches!(paras[1].space_before, Some(SpacingValue::Points(p)) if (p - 6.0).abs() < 0.1)
    );

    let html = render_html(&pptx);
    assert!(
        html.contains("First paragraph"),
        "First paragraph not found"
    );
    assert!(
        html.contains("Second paragraph"),
        "Second paragraph not found"
    );
    assert!(
        html.contains("text-align: center"),
        "Center alignment not rendered"
    );
    assert!(
        html.contains("line-height: 18.0pt"),
        "18pt line-height not rendered"
    );
}

// ── Verify style ref with no color child ──

#[test]
fn test_style_ref_no_color_child() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:style>
        <a:lnRef idx="0"/>
        <a:fillRef idx="0"/>
        <a:effectRef idx="0"/>
        <a:fontRef idx="minor"/>
      </p:style>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    let sr = shape.style_ref.as_ref().expect("style_ref not parsed");
    assert_eq!(sr.fill_ref.as_ref().unwrap().idx, 0);
    assert_eq!(sr.ln_ref.as_ref().unwrap().idx, 0);
    assert_eq!(sr.effect_ref.as_ref().unwrap().idx, 0);
    assert_eq!(sr.font_ref.as_ref().unwrap().idx, "minor");
}

// ── Bullet rendering test ──

#[test]
fn test_bullet_char_rendered() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="2000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:pPr>
            <a:buChar char="&#x2022;"/>
          </a:pPr>
          <a:r><a:rPr sz="1800"/><a:t>Bullet item</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("class=\"bullet\""),
        "Bullet span not found: {html}"
    );
    assert!(
        html.contains("Bullet item"),
        "Bullet text not found: {html}"
    );
}

// ── FmtScheme convenience methods test ──

#[test]
fn test_fmt_scheme_get_fill_style() {
    use pptx2html_core::model::hierarchy::FmtScheme;
    use pptx2html_core::model::{Color, Fill, SolidFill};

    let fmt = FmtScheme {
        fill_style_lst: vec![
            Fill::Solid(SolidFill {
                color: Color::rgb("AA0000"),
            }),
            Fill::Solid(SolidFill {
                color: Color::rgb("BB0000"),
            }),
        ],
        ln_style_lst: vec![],
        effect_style_lst: vec![],
        bg_fill_style_lst: vec![Fill::Solid(SolidFill {
            color: Color::rgb("CC0000"),
        })],
    };

    // idx 0 → None
    assert!(fmt.get_fill_style(0).is_none());
    // idx 1 → fill_style_lst[0]
    assert!(matches!(fmt.get_fill_style(1), Some(Fill::Solid(_))));
    // idx 2 → fill_style_lst[1]
    assert!(matches!(fmt.get_fill_style(2), Some(Fill::Solid(_))));
    // idx 3 → out of range
    assert!(fmt.get_fill_style(3).is_none());
    // idx 1001 → bg_fill_style_lst[0]
    assert!(matches!(fmt.get_fill_style(1001), Some(Fill::Solid(_))));
    // idx 1002 → out of range
    assert!(fmt.get_fill_style(1002).is_none());
}

#[test]
fn test_fmt_scheme_get_line_style() {
    use pptx2html_core::model::hierarchy::FmtScheme;
    use pptx2html_core::model::{Border, BorderStyle, Color};

    let fmt = FmtScheme {
        fill_style_lst: vec![],
        ln_style_lst: vec![Border {
            width: 0.75,
            color: Color::none(),
            style: BorderStyle::Solid,
            ..Default::default()
        }],
        effect_style_lst: vec![],
        bg_fill_style_lst: vec![],
    };

    assert!(fmt.get_line_style(0).is_none());
    let ln = fmt.get_line_style(1).unwrap();
    assert!((ln.width - 0.75).abs() < 0.01);
    assert!(fmt.get_line_style(2).is_none());
}

// ── FontScheme resolve_typeface test ──

#[test]
fn test_font_scheme_resolve_typeface() {
    use pptx2html_core::model::presentation::FontScheme;

    let fs = FontScheme {
        major_latin: "Calibri Light".to_string(),
        minor_latin: "Calibri".to_string(),
        major_east_asian: Some("Malgun Gothic".to_string()),
        minor_east_asian: Some("Malgun Gothic".to_string()),
        major_complex_script: Some("Times New Roman".to_string()),
        minor_complex_script: Some("Amiri".to_string()),
    };

    assert_eq!(fs.resolve_typeface("+mj-lt"), Some("Calibri Light"));
    assert_eq!(fs.resolve_typeface("+mn-lt"), Some("Calibri"));
    assert_eq!(fs.resolve_typeface("+mj-ea"), Some("Malgun Gothic"));
    assert_eq!(fs.resolve_typeface("+mn-ea"), Some("Malgun Gothic"));
    assert_eq!(fs.resolve_typeface("+mj-cs"), Some("Times New Roman"));
    assert_eq!(fs.resolve_typeface("+mn-cs"), Some("Amiri"));
    assert_eq!(fs.resolve_typeface("Arial"), None);
}

// ── Text style inheritance tests ──

#[test]
fn test_title_inherits_font_size_from_tx_styles() {
    // Master with txStyles titleStyle that sets font size to 4400 (44pt)
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:txStyles>
    <p:titleStyle>
      <a:lvl1pPr algn="l">
        <a:defRPr sz="4400" b="1">
          <a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>
        </a:defRPr>
      </a:lvl1pPr>
    </p:titleStyle>
    <p:bodyStyle>
      <a:lvl1pPr>
        <a:defRPr sz="2400"/>
      </a:lvl1pPr>
    </p:bodyStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    // Slide with title placeholder — no explicit font size on run
    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title 1"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr/><a:t>Inherited Title</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let html = render_html(&pptx);

    // Title should inherit 44pt font size from titleStyle lvl1pPr defRPr
    assert!(
        html.contains("font-size: 44.0pt"),
        "Title should inherit 44pt font size: {html}"
    );
    // Title should inherit bold from titleStyle
    assert!(
        html.contains("font-weight: bold"),
        "Title should inherit bold: {html}"
    );
    // Title should inherit red color from titleStyle
    assert!(
        html.contains("color: #FF0000"),
        "Title should inherit red color: {html}"
    );
}

#[test]
fn test_layout_lst_style_overrides_master_tx_styles() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:txStyles>
    <p:titleStyle>
      <a:lvl1pPr><a:defRPr sz="4400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:defRPr></a:lvl1pPr>
    </p:titleStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title Placeholder"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:lstStyle>
          <a:lvl1pPr><a:defRPr sz="3000"><a:solidFill><a:srgbClr val="00AA00"/></a:solidFill></a:defRPr></a:lvl1pPr>
        </a:lstStyle>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Title 1"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>Layout List Style Wins</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();

    let html = render_html(&pptx);
    assert!(html.contains("Layout List Style Wins"));
    assert!(
        html.contains("font-size: 30.0pt"),
        "Expected layout lstStyle font size to win over master txStyles: {html}"
    );
    assert!(
        html.contains("color: #00AA00"),
        "Expected layout lstStyle color to win over master txStyles: {html}"
    );
    assert!(
        !html.contains("font-size: 44.0pt") && !html.contains("color: #FF0000"),
        "Master txStyles should not win when layout lstStyle exists: {html}"
    );
}

#[test]
fn test_slide_lst_style_overrides_layout_master_and_default_text_style() {
    let pres_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
  <p:defaultTextStyle>
    <a:lvl1pPr>
      <a:defRPr sz="1800"><a:solidFill><a:srgbClr val="222222"/></a:solidFill></a:defRPr>
    </a:lvl1pPr>
  </p:defaultTextStyle>
</p:presentation>"#;

    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:txStyles>
    <p:titleStyle>
      <a:lvl1pPr><a:defRPr sz="4400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:defRPr></a:lvl1pPr>
    </p:titleStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title Placeholder"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:lstStyle>
          <a:lvl1pPr><a:defRPr sz="3000"><a:solidFill><a:srgbClr val="00AA00"/></a:solidFill></a:defRPr></a:lvl1pPr>
        </a:lstStyle>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Title 1"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:lstStyle>
          <a:lvl1pPr><a:defRPr sz="2600"><a:solidFill><a:srgbClr val="112233"/></a:solidFill></a:defRPr></a:lvl1pPr>
        </a:lstStyle>
        <a:p><a:r><a:t>Slide List Style Wins</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .with_presentation_xml(pres_xml)
        .build();

    let html = render_html(&pptx);
    assert!(html.contains("Slide List Style Wins"));
    assert!(
        html.contains("font-size: 26.0pt"),
        "Expected slide lstStyle font size to win over layout/master/default text style: {html}"
    );
    assert!(
        html.contains("color: #112233"),
        "Expected slide lstStyle color to win over layout/master/default text style: {html}"
    );
    assert!(
        !html.contains("font-size: 30.0pt")
            && !html.contains("font-size: 44.0pt")
            && !html.contains("font-size: 18.0pt")
            && !html.contains("color: #00AA00")
            && !html.contains("color: #FF0000")
            && !html.contains("color: #222222"),
        "Slide lstStyle should have highest priority among inherited text styles: {html}"
    );
}

#[test]
fn test_layout_body_pr_norm_autofit_is_inherited_by_slide_placeholder() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title Placeholder"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit fontScale="50000"/>
        </a:bodyPr>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Title 1"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="2000"/><a:t>Inherited autofit</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();

    let html = render_html(&pptx);
    assert!(html.contains("Inherited autofit"));
    assert!(
        html.contains("font-size: 10.0pt"),
        "Expected layout bodyPr normAutofit to scale slide placeholder text to 10pt: {html}"
    );
}

#[test]
fn test_layout_norm_autofit_font_scale_override_keeps_inherited_line_spacing_reduction() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title Placeholder"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit fontScale="70000" lnSpcReduction="20000"/>
        </a:bodyPr>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Title 1"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit fontScale="50000"/>
        </a:bodyPr>
        <a:p>
          <a:pPr><a:lnSpc><a:spcPct val="150000"/></a:lnSpc></a:pPr>
          <a:r><a:rPr sz="2000"/><a:t>Merged autofit</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();

    let html = render_html(&pptx);
    assert!(html.contains("Merged autofit"));
    assert!(
        html.contains("font-size: 10.0pt"),
        "Expected child normAutofit fontScale override to keep 10pt text: {html}"
    );
    assert!(
        html.contains("line-height: 1.20"),
        "Expected inherited lnSpcReduction to survive child fontScale override: {html}"
    );
}

#[test]
fn test_layout_body_pr_vertical_align_is_inherited_by_slide_placeholder() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title Placeholder"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr anchor="ctr"/>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Title 1"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="2000"/><a:t>Inherited vertical align</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();

    let html = render_html(&pptx);
    assert!(html.contains("Inherited vertical align"));
    assert!(
        html.contains("text-body v-middle"),
        "Expected layout bodyPr anchor='ctr' to yield v-middle on slide placeholder: {html}"
    );
}

#[test]
fn test_layout_body_pr_wrap_none_is_inherited_by_slide_placeholder() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title Placeholder"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr wrap="none"/>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Title 1"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>Inherited no wrap</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();

    let html = render_html(&pptx);
    assert!(html.contains("Inherited no wrap"));
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk = &html[tb_start..tb_start + 250.min(html.len() - tb_start)];
    assert!(
        tb_chunk.contains("white-space: nowrap"),
        "Expected layout bodyPr wrap='none' to set nowrap on slide placeholder: {tb_chunk}"
    );
}

#[test]
fn test_layout_body_pr_margins_are_inherited_by_slide_placeholder() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title Placeholder"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr lIns="0" tIns="0" rIns="0" bIns="0"/>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Title 1"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>Inherited margins</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();

    let html = render_html(&pptx);
    assert!(html.contains("Inherited margins"));
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk = &html[tb_start..tb_start + 250.min(html.len() - tb_start)];
    assert!(
        tb_chunk.contains("padding: 0.0pt 0.0pt 0.0pt 0.0pt"),
        "Expected layout bodyPr insets to override slide default text padding: {tb_chunk}"
    );
}

#[test]
fn test_layout_body_pr_vertical_text_is_inherited_by_slide_placeholder() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title Placeholder"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr vert="vert270"/>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Title 1"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>Inherited vertical text</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();

    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk = &html[tb_start..tb_start + 300.min(html.len() - tb_start)];
    assert!(tb_chunk.contains("writing-mode: vertical-lr"));
    assert!(
        tb_chunk.contains("rotate(180deg)"),
        "Expected layout bodyPr vert='vert270' to rotate inherited placeholder text: {tb_chunk}"
    );
}

#[test]
fn test_slide_body_pr_horz_overrides_inherited_vertical_text() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title Placeholder"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr vert="vert270"/>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Title 1"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr vert="horz"/>
        <a:p><a:r><a:t>Horizontal override</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();

    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk = &html[tb_start..tb_start + 300.min(html.len() - tb_start)];
    assert!(
        !tb_chunk.contains("writing-mode: vertical-lr")
            && !tb_chunk.contains("writing-mode: vertical-rl")
            && !tb_chunk.contains("rotate(180deg)"),
        "Expected slide vert='horz' to clear inherited vertical text modes: {tb_chunk}"
    );
}

#[test]
fn test_layout_body_pr_anchor_ctr_is_inherited_by_slide_placeholder() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title Placeholder"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr anchorCtr="1"/>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Title 1"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>Inherited anchorCtr</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();

    let html = render_html(&pptx);
    assert!(
        html.contains("class=\"text-body v-top h-center\""),
        "Expected layout bodyPr anchorCtr to be inherited by slide placeholder: {html}"
    );
}

#[test]
fn test_layout_body_pr_rotation_is_inherited_by_slide_placeholder() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title Placeholder"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr rot="5400000"/>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Title 1"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>Inherited rotation</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();

    let html = render_html(&pptx);
    assert!(
        html.contains("transform: rotate(90.0deg)"),
        "Expected layout bodyPr rotation to be inherited by slide placeholder: {html}"
    );
}

#[test]
fn test_placeholder_inheritance_provenance_is_collected() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:txStyles>
    <p:titleStyle>
      <a:lvl1pPr><a:defRPr sz="4400"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:defRPr></a:lvl1pPr>
    </p:titleStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title Placeholder"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
        <a:solidFill><a:srgbClr val="00CCFF"/></a:solidFill>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:lstStyle>
          <a:lvl1pPr><a:defRPr sz="3000"><a:solidFill><a:srgbClr val="00AA00"/></a:solidFill></a:defRPr></a:lvl1pPr>
        </a:lstStyle>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Title 1"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:t>Provenance Check</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let result = pptx2html_core::convert_bytes_with_metadata(&pptx).expect("conversion");

    let shape_entry = result
        .provenance_entries
        .iter()
        .find(|entry| entry.subject == ProvenanceSubject::Shape)
        .expect("shape provenance entry");
    assert_eq!(
        shape_entry.text_source,
        Some(ProvenanceSource::LayoutListStyle)
    );

    let bg_entry = result
        .provenance_entries
        .iter()
        .find(|entry| entry.subject == ProvenanceSubject::SlideBackground)
        .expect("background provenance entry");
    assert_eq!(
        bg_entry.background_source,
        Some(ProvenanceSource::HardcodedDefault)
    );
}

#[test]
fn test_explicit_style_overrides_inherited() {
    // Master with txStyles that sets 44pt for title
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:txStyles>
    <p:titleStyle>
      <a:lvl1pPr>
        <a:defRPr sz="4400"/>
      </a:lvl1pPr>
    </p:titleStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="274638"/><a:ext cx="8229600" cy="1143000"/></a:xfrm>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    // Slide with title and explicit 20pt font size — should override inherited 44pt
    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Title 1"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="title"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="2000"/><a:t>Explicit 20pt</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let html = render_html(&pptx);

    // Explicit 20pt should override inherited 44pt
    assert!(
        html.contains("font-size: 20.0pt"),
        "Explicit font-size should override inherited: {html}"
    );
    assert!(
        !html.contains("font-size: 44.0pt"),
        "Inherited 44pt should not appear: {html}"
    );
}

#[test]
fn test_body_inherits_spacing_from_tx_styles() {
    // Master with bodyStyle lvl1pPr that has space-before 10pt
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:txStyles>
    <p:bodyStyle>
      <a:lvl1pPr marL="342900" indent="-342900">
        <a:spcBef><a:spcPts val="1000"/></a:spcBef>
        <a:defRPr sz="2800"/>
      </a:lvl1pPr>
    </p:bodyStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Body"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="1600200"/><a:ext cx="8229600" cy="4525963"/></a:xfrm>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    // Slide with body placeholder — no explicit spacing
    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Content"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr/><a:t>Body text</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let html = render_html(&pptx);

    // Body should inherit space-before 10pt from bodyStyle
    assert!(
        html.contains("margin-top: 10.0pt"),
        "Body should inherit space-before: {html}"
    );
    // Body should inherit font-size 28pt from bodyStyle defRPr
    assert!(
        html.contains("font-size: 28.0pt"),
        "Body should inherit font-size 28pt: {html}"
    );
}

#[test]
fn test_body_inherited_txstyle_font_size_affects_autofit_wrap_policy() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:txStyles>
    <p:bodyStyle>
      <a:lvl1pPr>
        <a:defRPr sz="2800"/>
      </a:lvl1pPr>
    </p:bodyStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Body"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="1600200"/><a:ext cx="2600000" cy="4525963"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit fontScale="70000"/>
        </a:bodyPr>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Content"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:r><a:rPr><a:latin typeface="Calibri"/></a:rPr><a:t>overflow</a:t></a:r>
          <a:r><a:rPr><a:latin typeface="Aptos"/></a:rPr><a:t>detector</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let html = render_html(&pptx);

    assert!(
        html.contains("font-size: 19.6pt"),
        "Inherited bodyStyle font-size should still render at 19.6pt under normAutofit: {html}"
    );
    assert!(
        html.contains("emergency-wrap"),
        "Inherited bodyStyle font-size should affect narrow autofit wrap policy: {html}"
    );
    assert!(
        html.contains("overflow-wrap: anywhere"),
        "Inherited bodyStyle font-size should still trigger emergency overflow wrapping when the combined token is too wide: {html}"
    );
}

#[test]
fn test_body_inherited_txstyle_mixed_script_sentence_avoids_emergency_wrap() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:txStyles>
    <p:bodyStyle>
      <a:lvl1pPr>
        <a:defRPr sz="2800"/>
      </a:lvl1pPr>
    </p:bodyStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Body"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="1600200"/><a:ext cx="700000" cy="4525963"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr>
          <a:normAutofit fontScale="70000"/>
        </a:bodyPr>
      </p:txBody>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Content"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:r><a:rPr/><a:t>漢ABC漢DEF</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let html = render_html(&pptx);
    let tb_start = html.find("class=\"text-body").expect("text-body div");
    let tb_chunk: String = html[tb_start..].chars().take(320).collect();

    assert!(
        html.contains("font-size: 19.6pt"),
        "Inherited bodyStyle font-size should still render at 19.6pt under normAutofit: {html}"
    );
    assert!(
        !tb_chunk.contains("emergency-wrap"),
        "Inherited mixed-script autofit text should use natural script-boundary wrapping before emergency wrapping: {tb_chunk}"
    );
    assert!(
        !tb_chunk.contains("overflow-wrap: anywhere"),
        "Inherited mixed-script autofit text should not emit overflow-wrap:anywhere when script boundaries already provide breaks: {tb_chunk}"
    );
}

#[test]
fn test_body_inherits_letter_spacing_from_tx_styles() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:txStyles>
    <p:bodyStyle>
      <a:lvl1pPr>
        <a:defRPr sz="2800" spc="200"><a:solidFill><a:srgbClr val="222222"/></a:solidFill></a:defRPr>
      </a:lvl1pPr>
    </p:bodyStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Body"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="1600200"/><a:ext cx="8229600" cy="4525963"/></a:xfrm>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Content"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr/><a:t>Body tracking</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let html = render_html(&pptx);

    assert!(
        html.contains("letter-spacing: 2.00pt"),
        "Body should inherit letter spacing 2pt from bodyStyle defRPr: {html}"
    );
}

#[test]
fn test_body_inherits_baseline_from_tx_styles() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:txStyles>
    <p:bodyStyle>
      <a:lvl1pPr>
        <a:defRPr sz="2800" baseline="30000"/>
      </a:lvl1pPr>
    </p:bodyStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Body"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="1600200"/><a:ext cx="8229600" cy="4525963"/></a:xfrm>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Content"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr/><a:t>Body baseline</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let html = render_html(&pptx);

    assert!(
        html.contains("vertical-align: 30.0%; font-size: 0.70em"),
        "Body should inherit baseline offset from bodyStyle defRPr: {html}"
    );
}

#[test]
fn test_body_inherits_underline_and_strikethrough_from_tx_styles() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:txStyles>
    <p:bodyStyle>
      <a:lvl1pPr>
        <a:defRPr sz="2800" u="sng" strike="sngStrike"/>
      </a:lvl1pPr>
    </p:bodyStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Body"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="1600200"/><a:ext cx="8229600" cy="4525963"/></a:xfrm>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Content"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr/><a:t>Body decoration</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let html = render_html(&pptx);

    assert!(
        html.contains("text-decoration: underline")
            && html.contains("text-decoration: line-through"),
        "Body should inherit underline and strikethrough from bodyStyle defRPr: {html}"
    );
}

#[test]
fn test_body_inherits_capitalization_from_tx_styles() {
    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
  <p:txStyles>
    <p:bodyStyle>
      <a:lvl1pPr>
        <a:defRPr sz="2800" cap="all"/>
      </a:lvl1pPr>
    </p:bodyStyle>
  </p:txStyles>
</p:sldMaster>"#;

    let layout_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="2" name="Body"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="457200" y="1600200"/><a:ext cx="8229600" cy="4525963"/></a:xfrm>
      </p:spPr>
    </p:sp>
  </p:spTree></p:cSld>
</p:sldLayout>"#;

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr>
        <p:cNvPr id="3" name="Content"/>
        <p:cNvSpPr/>
        <p:nvPr><p:ph type="body" idx="1"/></p:nvPr>
      </p:nvSpPr>
      <p:spPr/>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr/><a:t>Body caps</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_layout(layout_xml)
        .build();
    let html = render_html(&pptx);

    assert!(
        html.contains("text-transform: uppercase"),
        "Body should inherit capitalization from bodyStyle defRPr: {html}"
    );
}

#[test]
fn test_default_text_style_inherited_by_non_placeholder() {
    // defaultTextStyle sets lvl1pPr defRPr sz=1800 (18pt)
    let pres_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
  <p:defaultTextStyle>
    <a:lvl1pPr>
      <a:defRPr sz="1800"/>
    </a:lvl1pPr>
  </p:defaultTextStyle>
</p:presentation>"#;

    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    // Non-placeholder shape — should get font-size from defaultTextStyle
    let slide_body = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Text Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="2000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr/><a:t>Default size text</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_presentation_xml(pres_xml)
        .build();
    let html = render_html(&pptx);

    // Non-placeholder shapes get otherStyle first, then defaultTextStyle
    assert!(
        html.contains("font-size: 18.0pt"),
        "Non-placeholder should inherit 18pt from defaultTextStyle: {html}"
    );
}

#[test]
fn test_font_ref_provides_font_family() {
    // Theme with fmtScheme
    let theme_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="TestTheme">
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
      <a:majorFont><a:latin typeface="Calibri Light"/></a:majorFont>
      <a:minorFont><a:latin typeface="Calibri"/></a:minorFont>
    </a:fontScheme>
    <a:fmtScheme name="Office">
      <a:fillStyleLst>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
      </a:fillStyleLst>
      <a:lnStyleLst>
        <a:ln w="6350"><a:solidFill><a:schemeClr val="phClr"/></a:solidFill></a:ln>
      </a:lnStyleLst>
      <a:bgFillStyleLst>
        <a:solidFill><a:schemeClr val="phClr"/></a:solidFill>
      </a:bgFillStyleLst>
    </a:fmtScheme>
  </a:themeElements>
</a:theme>"#;

    let master_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
             xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
             xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMap bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink"/>
</p:sldMaster>"#;

    // Shape with fontRef pointing to "minor" (should resolve to Calibri)
    let slide_body = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="2000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:style>
        <a:lnRef idx="1"><a:schemeClr val="accent1"/></a:lnRef>
        <a:fillRef idx="0"><a:schemeClr val="accent1"/></a:fillRef>
        <a:effectRef idx="0"><a:schemeClr val="accent1"/></a:effectRef>
        <a:fontRef idx="minor"><a:schemeClr val="dk1"/></a:fontRef>
      </p:style>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>Font from ref</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_master(master_xml)
        .with_full_theme(theme_xml)
        .build();
    let html = render_html(&pptx);

    // fontRef "minor" should resolve to "Calibri" from theme font scheme
    assert!(
        html.contains("font-family: 'Calibri'"),
        "fontRef should resolve to Calibri: {html}"
    );
}

#[test]
fn test_font_typeface_theme_ref_resolved() {
    // Shape with explicit font "+mn-lt" (theme minor latin reference)
    let theme_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="TestTheme">
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

    let slide_body = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="2000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p><a:r><a:rPr sz="1800"><a:latin typeface="+mn-lt"/></a:rPr><a:t>Theme font</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide_body)
        .with_full_theme(theme_xml)
        .build();
    let html = render_html(&pptx);

    // "+mn-lt" should resolve to "Pretendard"
    assert!(
        html.contains("font-family: 'Pretendard'"),
        "+mn-lt should resolve to Pretendard: {html}"
    );
}
