mod fixtures;

use std::fs;
use std::io::{Cursor, Read, Write};
use std::process::Command;

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
    AutoFit, ChartSpec, FallbackKind, Fill, Shape, ShapeEffects, TableCell, TextBody, TextMargins,
    VerticalAlign,
};
use pptx2html_core::{
    ConversionOptions, convert_bytes, convert_bytes_with_metadata, convert_bytes_with_options,
    convert_file, convert_file_with_metadata, convert_file_with_options, get_info,
    get_info_from_bytes,
};

#[test]
fn presentation_extensions_are_preserved_as_typed_metadata() {
    let presentation = r#"<?xml version="1.0" encoding="UTF-8"?>
<p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
 xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
 xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
 xmlns:demo="urn:pptx2html:demo">
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
  <p:extLst>
    <p:ext uri="{DEMO-EXTENSION}"><demo:payload enabled="1">EXTENSION_SENTINEL</demo:payload></p:ext>
  </p:extLst>
</p:presentation>"#;
    let bytes = fixtures::MinimalPptx::new("<p:sp/>")
        .with_presentation_xml(presentation)
        .build();

    let result = convert_bytes_with_metadata(&bytes).expect("extension fixture converts");
    let diagnostics: Vec<_> = result
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == "PRESENTATION_EXTENSION_METADATA")
        .collect();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].support_tier.as_str(), "fallback");
    assert_eq!(diagnostics[0].stage.unwrap().as_str(), "parsed");
    let raw = diagnostics[0]
        .raw_reference
        .as_deref()
        .expect("extension raw reference");
    assert!(raw.contains("uri={DEMO-EXTENSION}"));
    assert!(raw.contains("EXTENSION_SENTINEL"));
}

#[test]
fn bibliography_sources_are_preserved_as_typed_metadata() {
    let sources = br#"<?xml version="1.0" encoding="UTF-8"?>
<b:Sources xmlns:b="http://schemas.openxmlformats.org/officeDocument/2006/bibliography"
 SelectedStyle="\APASixthEditionOfficeOnline.xsl" StyleName="APA">
  <b:Source>
    <b:Tag>Doe2026</b:Tag><b:SourceType>JournalArticle</b:SourceType>
    <b:Title>Deterministic PPTX Conversion</b:Title><b:Year>2026</b:Year>
    <b:Author><b:Author><b:NameList><b:Person><b:Last>Doe</b:Last>
      <b:First>Jane</b:First></b:Person></b:NameList></b:Author></b:Author>
  </b:Source>
</b:Sources>"#;
    let bytes = MinimalPptx::new("<p:sp/>")
        .with_extra_file("ppt/bibliography/sources.xml", sources)
        .build();

    let result = convert_bytes_with_metadata(&bytes).expect("bibliography fixture converts");
    let diagnostics: Vec<_> = result
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == "BIBLIOGRAPHY_SOURCE_METADATA")
        .collect();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].support_tier.as_str(), "fallback");
    assert_eq!(diagnostics[0].stage.unwrap().as_str(), "parsed");
    let raw = diagnostics[0]
        .raw_reference
        .as_deref()
        .expect("bibliography raw reference");
    assert!(raw.contains("tag=Doe2026"));
    assert!(raw.contains("source_type=JournalArticle"));
    assert!(raw.contains("title=Deterministic PPTX Conversion"));
    assert!(raw.contains("year=2026"));
    assert!(raw.contains("author=Jane Doe"));
    assert!(result.diagnostics().iter().all(|diagnostic| {
        diagnostic.code != "OOXML_PART_UNSUPPORTED"
            || diagnostic.location.part_name.as_deref() != Some("ppt/bibliography/sources.xml")
    }));
}

#[test]
fn additional_characteristics_are_preserved_as_typed_metadata() {
    let characteristics = br#"<?xml version="1.0" encoding="UTF-8"?>
<ac:AdditionalCharacteristics
 xmlns:ac="http://schemas.openxmlformats.org/officeDocument/2006/additionalCharacteristics">
  <ac:Characteristic name="supports3D" relation="ge" val="1"
    vocabulary="urn:pptx2html:capabilities"/>
  <ac:Characteristic name="rendererVersion" relation="eq" val="1.1.0"/>
</ac:AdditionalCharacteristics>"#;
    let bytes = MinimalPptx::new("<p:sp/>")
        .with_extra_file("ppt/additionalCharacteristics.xml", characteristics)
        .build();

    let result = convert_bytes_with_metadata(&bytes).expect("characteristics fixture converts");
    let diagnostics: Vec<_> = result
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == "ADDITIONAL_CHARACTERISTIC_METADATA")
        .collect();

    assert_eq!(diagnostics.len(), 2);
    let raw: Vec<_> = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.raw_reference.as_deref())
        .collect();
    let supports_3d = raw
        .iter()
        .find(|value| value.contains("name=supports3D"))
        .expect("supports3D characteristic");
    assert!(supports_3d.contains("relation=ge"));
    assert!(supports_3d.contains("value=1"));
    assert!(supports_3d.contains("vocabulary=urn:pptx2html:capabilities"));
    let renderer = raw
        .iter()
        .find(|value| value.contains("name=rendererVersion"))
        .expect("renderer version characteristic");
    assert!(renderer.contains("relation=eq"));
    assert!(renderer.contains("value=1.1.0"));
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.support_tier.as_str() == "fallback"
            && diagnostic
                .stage
                .is_some_and(|stage| stage.as_str() == "parsed")
    }));
    assert!(result.diagnostics().iter().all(|diagnostic| {
        diagnostic.code != "OOXML_PART_UNSUPPORTED"
            || diagnostic.location.part_name.as_deref() != Some("ppt/additionalCharacteristics.xml")
    }));
}

#[test]
fn custom_xml_data_and_properties_are_preserved_as_typed_metadata() {
    let payload = br#"<?xml version="1.0" encoding="UTF-8"?>
<demo:project xmlns:demo="urn:pptx2html:custom-data" id="alpha">
  <demo:title>CUSTOM_XML_SENTINEL</demo:title>
</demo:project>"#;
    let properties = br#"<?xml version="1.0" encoding="UTF-8"?>
<ds:datastoreItem
 xmlns:ds="http://schemas.openxmlformats.org/officeDocument/2006/customXml"
 ds:itemID="{11111111-2222-3333-4444-555555555555}">
  <ds:schemaRefs><ds:schemaRef ds:uri="urn:pptx2html:custom-data"/></ds:schemaRefs>
</ds:datastoreItem>"#;
    let bytes = MinimalPptx::new("<p:sp/>")
        .with_extra_file("customXml/item1.xml", payload)
        .with_extra_file("customXml/itemProps1.xml", properties)
        .build();

    let result = convert_bytes_with_metadata(&bytes).expect("custom XML fixture converts");
    let diagnostics: Vec<_> = result
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == "CUSTOM_XML_DATA_METADATA")
        .collect();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].support_tier.as_str(), "fallback");
    assert_eq!(diagnostics[0].stage.unwrap().as_str(), "parsed");
    let raw = diagnostics[0]
        .raw_reference
        .as_deref()
        .expect("custom XML raw reference");
    assert!(raw.contains("data_part=customXml/item1.xml"));
    assert!(raw.contains("properties_part=customXml/itemProps1.xml"));
    assert!(raw.contains("item_id={11111111-2222-3333-4444-555555555555}"));
    assert!(raw.contains("schema_uri=urn:pptx2html:custom-data"));
    assert!(raw.contains("root={urn:pptx2html:custom-data}project"));
    assert!(raw.contains("CUSTOM_XML_SENTINEL"));
}
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

fn with_thumbnail(bytes: &[u8]) -> Vec<u8> {
    let mut source = ZipArchive::new(Cursor::new(bytes)).expect("fixture zip");
    let cursor = Cursor::new(Vec::new());
    let mut target = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();
    for index in 0..source.len() {
        let mut file = source.by_index(index).expect("fixture entry");
        let mut data = Vec::new();
        file.read_to_end(&mut data).expect("read fixture entry");
        target
            .start_file(file.name(), options)
            .expect("copy fixture entry");
        if file.name() == "_rels/.rels" {
            let relationships = br#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
  <Relationship Id="rIdThumb" Type="http://schemas.openxmlformats.org/package/2006/relationships/metadata/thumbnail" Target="docProps/thumbnail.png"/>
</Relationships>"#;
            target
                .write_all(relationships)
                .expect("write thumbnail root relationships");
        } else {
            target.write_all(&data).expect("copy fixture data");
        }
    }
    target
        .start_file("docProps/thumbnail.png", options)
        .expect("thumbnail entry");
    target
        .write_all(&[137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 0])
        .expect("thumbnail data");
    target
        .finish()
        .expect("finish thumbnail fixture")
        .into_inner()
}

#[test]
fn package_thumbnail_is_preserved_as_typed_metadata() {
    let bytes = with_thumbnail(&MinimalPptx::new("<p:sp/>").build());
    let result = convert_bytes_with_metadata(&bytes).expect("thumbnail fixture converts");
    let diagnostics: Vec<_> = result
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == "PACKAGE_THUMBNAIL_METADATA")
        .collect();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].support_tier.as_str(), "fallback");
    assert_eq!(diagnostics[0].stage.unwrap().as_str(), "parsed");
    let raw = diagnostics[0]
        .raw_reference
        .as_deref()
        .expect("thumbnail raw reference");
    assert!(raw.contains("part=docProps/thumbnail.png"));
    assert!(raw.contains("relationship_id=rIdThumb"));
    assert!(raw.contains("content_type=image/png"));
    assert!(raw.contains("byte_length=12"));
    assert!(raw.contains("signature=png"));
    assert!(result.diagnostics().iter().all(|diagnostic| {
        diagnostic.code != "OOXML_RELATIONSHIP_UNSUPPORTED"
            || diagnostic.location.relationship_id.as_deref() != Some("rIdThumb")
    }));
}

#[test]
fn layout_theme_override_is_preserved_as_typed_metadata() {
    let layout_relationships = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdMaster" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>
  <Relationship Id="rIdThemeOverride" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/themeOverride" Target="../theme/themeOverride1.xml"/>
</Relationships>"#;
    let theme_override = br#"<?xml version="1.0" encoding="UTF-8"?>
<a:themeOverride xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main">
  <a:clrScheme name="Layout Override">
    <a:dk1><a:srgbClr val="101010"/></a:dk1>
    <a:lt1><a:srgbClr val="F0F0F0"/></a:lt1>
    <a:accent1><a:srgbClr val="FF0000"/></a:accent1>
  </a:clrScheme>
  <a:fontScheme name="Override Fonts"><a:majorFont/><a:minorFont/></a:fontScheme>
</a:themeOverride>"#;
    let bytes = MinimalPptx::new("<p:sp/>")
        .with_layout("<p:sldLayout/>")
        .with_slide_layout_rel()
        .with_layout_rels(layout_relationships)
        .with_extra_file("ppt/theme/themeOverride1.xml", theme_override)
        .build();

    let result = convert_bytes_with_metadata(&bytes).expect("theme override fixture converts");
    let diagnostics: Vec<_> = result
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == "THEME_OVERRIDE_METADATA")
        .collect();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].support_tier.as_str(), "fallback");
    assert_eq!(diagnostics[0].stage.unwrap().as_str(), "parsed");
    let raw = diagnostics[0]
        .raw_reference
        .as_deref()
        .expect("theme override raw reference");
    assert!(raw.contains("owner=ppt/slideLayouts/slideLayout1.xml"));
    assert!(raw.contains("part=ppt/theme/themeOverride1.xml"));
    assert!(raw.contains("relationship_id=rIdThemeOverride"));
    assert!(raw.contains("color_scheme=Layout Override"));
    assert!(raw.contains("color_slot_count=3"));
    assert!(raw.contains("font_scheme=Override Fonts"));
}

#[test]
fn slide_synchronization_is_preserved_as_typed_metadata() {
    let slide_relationships = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdSync" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideUpdateInfo" Target="../slideUpdateInfo/slideUpdateInfo1.xml"/>
</Relationships>"#;
    let synchronization = br#"<?xml version="1.0" encoding="UTF-8"?>
<p:sldSyncPr xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
 serverSldId="server-slide-42"
 serverSldModifiedTime="2026-08-12T10:30:00Z"
 clientInsertedTime="2026-08-12T10:31:00Z"/>"#;
    let bytes = MinimalPptx::new("<p:sp/>")
        .with_slide_rels(slide_relationships)
        .with_extra_file("ppt/slideUpdateInfo/slideUpdateInfo1.xml", synchronization)
        .build();

    let result = convert_bytes_with_metadata(&bytes).expect("slide sync fixture converts");
    let diagnostics: Vec<_> = result
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == "SLIDE_SYNCHRONIZATION_METADATA")
        .collect();

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].support_tier.as_str(), "fallback");
    assert_eq!(diagnostics[0].stage.unwrap().as_str(), "parsed");
    let raw = diagnostics[0]
        .raw_reference
        .as_deref()
        .expect("slide sync raw reference");
    assert!(raw.contains("owner=ppt/slides/slide1.xml"));
    assert!(raw.contains("part=ppt/slideUpdateInfo/slideUpdateInfo1.xml"));
    assert!(raw.contains("server_slide_id=server-slide-42"));
    assert!(raw.contains("server_modified=2026-08-12T10:30:00Z"));
    assert!(raw.contains("client_inserted=2026-08-12T10:31:00Z"));
    assert!(result.diagnostics().iter().all(|diagnostic| {
        diagnostic.code != "OOXML_PART_UNSUPPORTED"
            || diagnostic.location.part_name.as_deref()
                != Some("ppt/slideUpdateInfo/slideUpdateInfo1.xml")
    }));
}

#[test]
fn previous_shape_effects_struct_literal_remains_source_compatible() {
    let effects = ShapeEffects {
        outer_shadow: None,
        glow: None,
    };
    assert!(effects.outer_shadow.is_none());
    assert!(effects.glow.is_none());
}

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
fn exhaustive_presentation_literal_external_crate_contract_is_explicit() {
    let directory = tempdir().expect("external crate tempdir");
    let core_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    fs::create_dir(directory.path().join("src")).expect("create external source directory");
    fs::write(
        directory.path().join("Cargo.toml"),
        format!(
            r#"[package]
name = "presentation-literal-compat"
version = "0.1.0"
edition = "2024"

[dependencies]
pptx2html-core = {{ path = {:?} }}
"#,
            core_path
        ),
    )
    .expect("write external manifest");
    let legacy_literal = r#"use pptx2html_core::model::presentation::Theme;
use pptx2html_core::model::{ClrMap, ListStyle, Presentation, Size, Slide, SlideLayout, SlideMaster};
fn main() {
    let _ = Presentation {
        slides: Vec::<Slide>::new(),
        slide_size: Size::default(),
        title: None,
        themes: Vec::<Theme>::new(),
        masters: Vec::<SlideMaster>::new(),
        layouts: Vec::<SlideLayout>::new(),
        default_text_style: None::<ListStyle>,
        clr_map: ClrMap::default(),
    };
}
"#;
    fs::write(directory.path().join("src/main.rs"), legacy_literal)
        .expect("write legacy external literal");
    let target = directory.path().join("target");
    let legacy = Command::new("cargo")
        .args(["check", "--offline"])
        .current_dir(directory.path())
        .env("CARGO_TARGET_DIR", &target)
        .output()
        .expect("compile legacy external crate");
    let legacy_stderr = String::from_utf8_lossy(&legacy.stderr);
    assert!(
        !legacy.status.success(),
        "legacy exhaustive literal unexpectedly compiled"
    );
    assert!(
        legacy_stderr.contains("missing field `embedded_inventory`"),
        "unexpected compatibility failure: {legacy_stderr}"
    );

    let updated_literal = legacy_literal.replace(
        "        clr_map: ClrMap::default(),\n",
        "        clr_map: ClrMap::default(),\n        embedded_inventory: Default::default(),\n",
    );
    fs::write(directory.path().join("src/main.rs"), updated_literal)
        .expect("write updated external literal");
    let updated = Command::new("cargo")
        .args(["check", "--offline"])
        .current_dir(directory.path())
        .env("CARGO_TARGET_DIR", target)
        .output()
        .expect("compile updated external crate");
    assert!(
        updated.status.success(),
        "updated exhaustive literal failed: {}",
        String::from_utf8_lossy(&updated.stderr)
    );
}

fn legacy_fallback_kind_name(kind: FallbackKind) -> &'static str {
    match kind {
        FallbackKind::SmartArtPlaceholder => "smartart-placeholder",
        FallbackKind::OlePlaceholder => "ole-placeholder",
        FallbackKind::MathPlaceholder => "math-placeholder",
        FallbackKind::CustomGeometryPlaceholder => "custom-geometry-placeholder",
        FallbackKind::PreservedPart => "preserved-part",
        FallbackKind::IgnoredRelationship => "ignored-relationship",
        FallbackKind::UnknownElement => "unknown-element",
        FallbackKind::TableStyleDefinitionUnavailable => "table-style-definition-unavailable",
        FallbackKind::ActionMetadata => "action-metadata",
    }
}

#[test]
fn legacy_exhaustive_fallback_kind_consumers_remain_source_compatible() {
    assert_eq!(
        legacy_fallback_kind_name(FallbackKind::PreservedPart),
        "preserved-part"
    );
}

#[test]
fn public_model_paths_and_defaults_remain_compatible() {
    let inventory_marker = pptx2html_core::model::embedded::EmbeddedInventory;
    let root_shape = Shape::default();
    let slide_shape = SlideModuleShape::default();
    let root_body = TextBody::default();
    let slide_body = SlideModuleTextBody::default();
    let root_fill = Fill::default();
    let root_effects = ShapeEffects::default();
    let cell = TableCell::default();
    let margins = TextMargins::default();
    let chart = ChartSpec::default();
    let legacy_chart_data = pptx2html_core::model::ChartData {
        rel_id: "rIdChart".to_owned(),
        preview_image: None,
        preview_mime: None,
        direct_spec: None,
    };
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
    assert_eq!(legacy_chart_data.rel_id, "rIdChart");
    assert_eq!(feature_shape.id, root_shape.id);
    assert!(feature_body.paragraphs.is_empty());
    assert_eq!(feature_cell.margin_left, cell.margin_left);
    assert!(feature_chart.series.is_empty());
    assert!(matches!(feature_fill, FeatureFill::None));
    assert!(feature_effects.outer_shadow.is_none());
    assert_eq!(feature_bullet.char, "-");
    assert_eq!(unresolved, FeatureUnresolvedType::SmartArt);
    assert_eq!(
        inventory_marker,
        pptx2html_core::model::embedded::EmbeddedInventory
    );
}
