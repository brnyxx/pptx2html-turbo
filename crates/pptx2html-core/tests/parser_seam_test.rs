mod fixtures;

use fixtures::{FeaturePart, PackageBuilder, Relationship, SlideXml};
use pptx2html_core::model::{Bullet, Fill, ShapeType};
use pptx2html_core::parser::PptxParser;
use pptx2html_core::parser::relationships::{TargetMode, parse_relationship_records};

const HYPERLINK_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink";
const SLIDE_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide";
const CHART_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart";

#[test]
fn parsed_model_preserves_multi_feature_fixture_across_parser_seams() {
    // Given
    let slide = SlideXml::from_body(
        r#"
<p:sp xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:nvSpPr><p:cNvPr id="2" name="Feature Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
  <p:spPr>
    <a:xfrm><a:off x="100000" y="200000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
    <a:prstGeom prst="rect"/><a:solidFill><a:srgbClr val="336699"/></a:solidFill>
  </p:spPr>
  <p:txBody><a:bodyPr/><a:p><a:pPr><a:buChar char="-"/></a:pPr><a:r>
    <a:rPr b="1"><a:hlinkClick r:id="rIdExternal"/></a:rPr><a:t>Linked bullet</a:t>
  </a:r></a:p></p:txBody>
</p:sp>
<p:graphicFrame>
  <p:nvGraphicFramePr><p:cNvPr id="3" name="Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
  <p:xfrm><a:off x="0" y="0"/><a:ext cx="2000000" cy="1000000"/></p:xfrm>
  <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
    <a:tbl><a:tblPr firstRow="1"/><a:tblGrid><a:gridCol w="2000000"/></a:tblGrid>
      <a:tr h="1000000"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>Cell</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr>
    </a:tbl>
  </a:graphicData></a:graphic>
</p:graphicFrame>
<p:graphicFrame xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:nvGraphicFramePr><p:cNvPr id="4" name="Chart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
  <p:xfrm><a:off x="0" y="0"/><a:ext cx="2000000" cy="1000000"/></p:xfrm>
  <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
    <c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="rIdChart"/>
  </a:graphicData></a:graphic>
</p:graphicFrame>
<p:graphicFrame>
  <p:nvGraphicFramePr><p:cNvPr id="5" name="SmartArt"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
  <p:xfrm><a:off x="0" y="0"/><a:ext cx="2000000" cy="1000000"/></p:xfrm>
  <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/diagram"><a:extLst/></a:graphicData></a:graphic>
</p:graphicFrame>"#,
    )
    .build();
    let package = PackageBuilder::new(slide)
        .with_slide_relationship(Relationship::external(
            "rIdExternal",
            HYPERLINK_RELATIONSHIP,
            "../slides/slide2.xml",
        ))
        .with_slide_relationship(Relationship::internal(
            "rIdInternal",
            SLIDE_RELATIONSHIP,
            "../slides/slide2.xml",
        ))
        .with_slide_relationship(Relationship::internal(
            "rIdChart",
            CHART_RELATIONSHIP,
            "../charts/chart1.xml",
        ))
        .with_part(FeaturePart::extra(
            "ppt/slides/slide2.xml",
            "application/vnd.openxmlformats-officedocument.presentationml.slide+xml",
            b"<p:sld xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"/>",
        ))
        .with_part(FeaturePart::chart(
            "<c:chart><c:plotArea><c:barChart/></c:plotArea></c:chart>",
        ));
    let bytes = package.build().expect("multi-feature fixture builds");

    // When
    let presentation = PptxParser::parse_bytes(&bytes).expect("multi-feature fixture parses");

    // Then
    let shapes = &presentation.slides[0].shapes;
    assert_eq!(shapes.len(), 4);
    assert!(matches!(shapes[0].fill, Fill::Solid(_)));
    let paragraph = &shapes[0].text_body.as_ref().expect("text body").paragraphs[0];
    assert!(matches!(paragraph.bullet, Some(Bullet::Char(_))));
    assert_eq!(paragraph.runs[0].text, "Linked bullet");
    assert!(paragraph.runs[0].style.bold);
    assert_eq!(
        paragraph.runs[0].hyperlink.as_deref(),
        Some("../slides/slide2.xml")
    );
    assert!(matches!(shapes[1].shape_type, ShapeType::Table(_)));
    assert!(matches!(shapes[2].shape_type, ShapeType::Chart(_)));
    assert!(matches!(shapes[3].shape_type, ShapeType::Unsupported(_)));
}

#[test]
fn external_target_mode_remains_distinguishable_from_similar_internal_target() {
    // Given
    let relationships_xml = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdExternal" Type="{HYPERLINK_RELATIONSHIP}" Target="../slides/slide2.xml" TargetMode="External"/>
  <Relationship Id="rIdInternal" Type="{SLIDE_RELATIONSHIP}" Target="../slides/slide2.xml"/>
</Relationships>"#,
    );

    // When
    let relationships =
        parse_relationship_records(&relationships_xml).expect("relationships parse");

    // Then
    assert_eq!(relationships[0].id, "rIdExternal");
    assert_eq!(relationships[0].target_mode, TargetMode::External);
    assert_eq!(relationships[1].id, "rIdInternal");
    assert_eq!(relationships[1].target_mode, TargetMode::Internal);
    assert_eq!(relationships[0].target, relationships[1].target);
}
