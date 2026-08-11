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

fn multi_feature_fixture() -> Vec<u8> {
    let slide = SlideXml::from_body(
        r#"
<p:sp xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <p:nvSpPr><p:cNvPr id="2" name="Feature Shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
  <p:spPr>
    <a:xfrm><a:off x="100000" y="200000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
    <a:prstGeom prst="rect"/><a:solidFill><a:srgbClr val="336699"/></a:solidFill>
    <a:effectLst><a:outerShdw blurRad="12700" dist="25400" dir="5400000"><a:srgbClr val="112233"/></a:outerShdw></a:effectLst>
  </p:spPr>
  <p:txBody><a:bodyPr/><a:p><a:pPr><a:buChar char="-"/></a:pPr><a:r>
    <a:rPr b="1"><a:hlinkClick r:id="rIdExternal"/></a:rPr><a:t>Linked bullet</a:t>
  </a:r></a:p></p:txBody>
</p:sp>
<p:graphicFrame>
  <p:nvGraphicFramePr><p:cNvPr id="3" name="Table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
  <p:xfrm><a:off x="0" y="0"/><a:ext cx="2000000" cy="1000000"/></p:xfrm>
  <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
    <a:tbl><a:tblPr firstRow="1"></a:tblPr><a:tblGrid><a:gridCol w="2000000"/></a:tblGrid>
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
            "<c:chart><c:plotArea><c:barChart><c:barDir val=\"col\"/><c:grouping val=\"clustered\"/><c:ser><c:idx val=\"0\"/><c:order val=\"0\"/><c:tx><c:v>Series</c:v></c:tx><c:cat><c:strLit><c:pt idx=\"0\"><c:v>A</c:v></c:pt></c:strLit></c:cat><c:val><c:numLit><c:pt idx=\"0\"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser></c:barChart></c:plotArea></c:chart>",
        ));
    package.build().expect("multi-feature fixture builds")
}

fn parsed_shapes() -> Vec<pptx2html_core::model::Shape> {
    let presentation =
        PptxParser::parse_bytes(&multi_feature_fixture()).expect("multi-feature fixture parses");
    presentation.slides[0].shapes.clone()
}

#[test]
fn fill_and_effect_model_preserve_values_across_parser_seam() {
    // Given
    let bytes = multi_feature_fixture();

    // When
    let presentation = PptxParser::parse_bytes(&bytes).expect("multi-feature fixture parses");

    // Then
    let shape = &presentation.slides[0].shapes[0];
    let Fill::Solid(fill) = &shape.fill else {
        panic!("shape fill remains solid");
    };
    assert_eq!(fill.color.to_css().as_deref(), Some("#336699"));
    let shadow = shape
        .effects
        .outer_shadow
        .as_ref()
        .expect("outer shadow is preserved");
    assert_eq!(shadow.color.to_css().as_deref(), Some("#112233"));
    assert_eq!(shadow.blur_radius, 1.0);
    assert_eq!(shadow.distance, 2.0);
}

#[test]
fn text_and_bullet_model_preserve_values_across_parser_seam() {
    // Given
    let shapes = parsed_shapes();

    // When
    let paragraph = &shapes[0].text_body.as_ref().expect("text body").paragraphs[0];

    // Then
    assert!(matches!(paragraph.bullet, Some(Bullet::Char(_))));
    assert_eq!(paragraph.runs[0].text, "Linked bullet");
    assert!(paragraph.runs[0].style.bold);
}

#[test]
fn action_hyperlink_model_preserves_target_across_parser_seam() {
    // Given
    let shapes = parsed_shapes();

    // When
    let run = &shapes[0].text_body.as_ref().expect("text body").paragraphs[0].runs[0];

    // Then
    assert_eq!(run.hyperlink.as_deref(), Some("../slides/slide2.xml"));
}

#[test]
fn table_model_preserves_rows_cells_and_text_across_parser_seam() {
    // Given
    let shapes = parsed_shapes();

    // When
    let ShapeType::Table(table) = &shapes[1].shape_type else {
        panic!("table shape is preserved");
    };

    // Then
    assert!(table.first_row);
    assert_eq!(table.col_widths, vec![2_000_000.0 / 9_525.0]);
    assert_eq!(table.rows.len(), 1);
    assert_eq!(table.rows[0].height, 1_000_000.0 / 9_525.0);
    assert_eq!(
        table.rows[0].cells[0]
            .text_body
            .as_ref()
            .expect("cell text body")
            .paragraphs[0]
            .runs[0]
            .text,
        "Cell"
    );
}

#[test]
fn chart_graphic_frame_model_preserves_relationship_across_parser_seam() {
    // Given
    let shapes = parsed_shapes();

    // When
    let ShapeType::Chart(chart) = &shapes[2].shape_type else {
        panic!("chart shape is preserved");
    };

    // Then
    assert_eq!(chart.rel_id, "rIdChart");
    assert!(chart.direct_spec.is_some());
}

#[test]
fn preserved_model_retains_classification_and_raw_xml_across_parser_seam() {
    // Given
    let shapes = parsed_shapes();

    // When
    let ShapeType::Unsupported(unsupported) = &shapes[3].shape_type else {
        panic!("unsupported shape is preserved");
    };

    // Then
    assert_eq!(unsupported.label, "SmartArt");
    assert_eq!(
        unsupported.element_type,
        pptx2html_core::model::slide::UnresolvedType::SmartArt
    );
    assert_eq!(unsupported.raw_xml.as_deref(), Some("<a:extLst/>"));
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
