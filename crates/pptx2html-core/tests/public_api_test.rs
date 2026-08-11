mod fixtures;

use std::fs;

use fixtures::MinimalPptx;
use pptx2html_core::model::bullet::BulletChar as FeatureBulletChar;
use pptx2html_core::model::chart::ChartSpec as FeatureChartSpec;
use pptx2html_core::model::effects::ShapeEffects as FeatureShapeEffects;
use pptx2html_core::model::fill::Fill as FeatureFill;
use pptx2html_core::model::preserved::UnresolvedType as FeatureUnresolvedType;
use pptx2html_core::model::shape::Shape as FeatureShape;
use pptx2html_core::model::slide::{Shape as SlideModuleShape, TextBody as SlideModuleTextBody};
use pptx2html_core::model::table::TableCell as FeatureTableCell;
use pptx2html_core::model::text::TextBody as FeatureTextBody;
use pptx2html_core::model::{
    AutoFit, ChartSpec, Fill, Shape, ShapeEffects, TableCell, TextBody, TextMargins, VerticalAlign,
};
use pptx2html_core::{
    ConversionOptions, convert_bytes, convert_bytes_with_metadata, convert_bytes_with_options,
    convert_file, convert_file_with_metadata, convert_file_with_options, get_info,
    get_info_from_bytes,
};
use tempfile::tempdir;

fn basic_text_shape(text: &str) -> String {
    format!(
        r#"<p:sp>
  <p:nvSpPr>
    <p:cNvPr id="2" name="TextBox"/>
    <p:cNvSpPr txBox="1"/>
    <p:nvPr/>
  </p:nvSpPr>
  <p:spPr>
    <a:xfrm>
      <a:off x="0" y="0"/>
      <a:ext cx="914400" cy="457200"/>
    </a:xfrm>
    <a:prstGeom prst="rect"><a:avLst/></a:prstGeom>
  </p:spPr>
  <p:txBody>
    <a:bodyPr/>
    <a:lstStyle/>
    <a:p><a:r><a:t>{text}</a:t></a:r></a:p>
  </p:txBody>
</p:sp>"#
    )
}

#[test]
fn public_file_and_bytes_apis_delegate_consistently() {
    let bytes = MinimalPptx::new(&basic_text_shape("Public API"))
        .with_core_properties(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties"
                   xmlns:dc="http://purl.org/dc/elements/1.1/">
  <dc:title>API Deck</dc:title>
</cp:coreProperties>"#,
        )
        .build();
    let dir = tempdir().expect("tempdir");
    let path = dir.path().join("sample.pptx");
    fs::write(&path, &bytes).expect("write pptx");

    let opts = ConversionOptions {
        embed_images: false,
        slide_indices: Some(vec![1]),
        scale: 1.5,
        ..Default::default()
    };

    let file_html = convert_file(&path).expect("convert_file");
    let file_html_with_opts = convert_file_with_options(&path, &opts).expect("convert_file opts");
    let file_result = convert_file_with_metadata(&path).expect("convert_file metadata");
    let bytes_html = convert_bytes(&bytes).expect("convert_bytes");
    let bytes_html_with_opts =
        convert_bytes_with_options(&bytes, &opts).expect("convert_bytes opts");
    let bytes_result = convert_bytes_with_metadata(&bytes).expect("convert_bytes metadata");
    let info_from_file = get_info(&path).expect("get_info");
    let info_from_bytes = get_info_from_bytes(&bytes).expect("get_info_from_bytes");

    assert!(file_html.contains("Public API"));
    assert!(file_html_with_opts.contains("Public API"));
    assert!(file_html_with_opts.contains("class=\"slide-shell\""));
    assert!(file_html_with_opts.contains("transform: scale(1.5000);"));
    assert!(file_result.html.contains("Public API"));
    assert!(bytes_html.contains("Public API"));
    assert!(bytes_html_with_opts.contains("Public API"));
    assert!(bytes_html_with_opts.contains("width: 1440.0px; height: 1080.0px;"));
    assert!(bytes_result.html.contains("Public API"));

    assert_eq!(file_result.slide_count, 1);
    assert_eq!(bytes_result.slide_count, 1);
    assert!(file_result.unresolved_elements.is_empty());
    assert!(bytes_result.unresolved_elements.is_empty());

    assert_eq!(info_from_file.slide_count, 1);
    assert_eq!(info_from_file.width_px, 960.0);
    assert_eq!(info_from_file.height_px, 720.0);
    assert_eq!(info_from_file.title.as_deref(), Some("API Deck"));
    assert_eq!(info_from_bytes.slide_count, info_from_file.slide_count);
    assert_eq!(info_from_bytes.width_px, info_from_file.width_px);
    assert_eq!(info_from_bytes.height_px, info_from_file.height_px);
    assert_eq!(info_from_bytes.title, info_from_file.title);
}

#[test]
fn should_include_slide_honors_hidden_indices_and_ranges() {
    let default_opts = ConversionOptions::default();
    assert!(default_opts.should_include_slide(1, false));
    assert!(!default_opts.should_include_slide(1, true));

    let indices_opts = ConversionOptions {
        include_hidden: true,
        slide_indices: Some(vec![1, 3]),
        ..Default::default()
    };
    assert!(indices_opts.should_include_slide(1, false));
    assert!(indices_opts.should_include_slide(3, true));
    assert!(!indices_opts.should_include_slide(2, false));

    let range_opts = ConversionOptions {
        slide_range: Some((2, 4)),
        ..Default::default()
    };
    assert!(!range_opts.should_include_slide(1, false));
    assert!(range_opts.should_include_slide(2, false));
    assert!(range_opts.should_include_slide(4, false));
    assert!(!range_opts.should_include_slide(5, false));

    let invalid_scale_opts = ConversionOptions {
        scale: 0.0,
        ..Default::default()
    };
    assert_eq!(invalid_scale_opts.effective_scale(), 1.0);

    let scaled_opts = ConversionOptions {
        scale: 2.0,
        ..Default::default()
    };
    assert_eq!(scaled_opts.effective_scale(), 2.0);
}

#[test]
fn public_model_paths_and_defaults_remain_compatible() {
    let root_shape = Shape::default();
    let slide_shape = SlideModuleShape::default();
    let root_body = TextBody::default();
    let slide_body = SlideModuleTextBody::default();
    let root_fill = Fill::default();
    let root_effects = ShapeEffects::default();
    let cell = TableCell::default();
    let margins = TextMargins::default();
    let chart = ChartSpec::default();
    let feature_shape = FeatureShape::default();
    let feature_body = FeatureTextBody::default();
    let feature_cell = FeatureTableCell::default();
    let feature_chart = FeatureChartSpec::default();
    let feature_fill = FeatureFill::default();
    let feature_effects = FeatureShapeEffects::default();
    let feature_bullet = FeatureBulletChar {
        char: "-".to_string(),
        font: None,
        size_pct: None,
        color: None,
    };
    let unresolved = FeatureUnresolvedType::SmartArt;

    let _: FeatureShape = root_shape.clone();
    let _: FeatureShape = slide_shape.clone();
    let _: FeatureTextBody = root_body.clone();
    let _: FeatureTextBody = slide_body.clone();

    assert_eq!(root_shape.id, slide_shape.id);
    assert!(root_body.paragraphs.is_empty());
    assert!(slide_body.paragraphs.is_empty());
    assert!(matches!(root_fill, Fill::None));
    assert!(root_effects.outer_shadow.is_none());
    assert_eq!(cell.margin_left, 7.2);
    assert_eq!(cell.margin_top, 3.6);
    assert_eq!(margins.left, 7.2);
    assert_eq!(margins.top, 3.6);
    assert!(matches!(cell.vertical_align, VerticalAlign::Top));
    assert!(matches!(root_body.auto_fit, AutoFit::None));
    assert!(chart.series.is_empty());
    assert_eq!(feature_shape.id, root_shape.id);
    assert!(feature_body.paragraphs.is_empty());
    assert_eq!(feature_cell.margin_left, cell.margin_left);
    assert!(feature_chart.series.is_empty());
    assert!(matches!(feature_fill, FeatureFill::None));
    assert!(feature_effects.outer_shadow.is_none());
    assert_eq!(feature_bullet.char, "-");
    assert_eq!(unresolved, FeatureUnresolvedType::SmartArt);
}
