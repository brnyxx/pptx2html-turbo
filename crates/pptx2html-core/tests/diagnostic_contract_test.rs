mod fixtures;

use std::collections::BTreeSet;

use fixtures::{FeaturePart, PackageBuilder, Relationship, SlideXml};
use pptx2html_core::model::{
    Emu, FallbackKind, Position, Presentation, Shape, ShapeType, Size, Slide, UnresolvedType,
    UnsupportedData,
};
use pptx2html_core::renderer::HtmlRenderer;
use pptx2html_core::{convert_bytes, convert_bytes_with_metadata};

const SCRIPT_OPEN: &str = "<script type=\"application/json\" id=\"pptx2html-diagnostics\">";
const SCRIPT_CLOSE: &str = "</script>";

fn script_payload(html: &str) -> &str {
    let start = html.find(SCRIPT_OPEN).expect("diagnostics script exists") + SCRIPT_OPEN.len();
    let remainder = &html[start..];
    let end = remainder
        .find(SCRIPT_CLOSE)
        .expect("diagnostics script closes");
    &remainder[..end]
}

fn unsupported_shape(element_type: UnresolvedType, label: &str, raw_xml: &str) -> Shape {
    Shape {
        id: 7,
        name: label.to_owned(),
        shape_type: ShapeType::Unsupported(UnsupportedData {
            label: label.to_owned(),
            element_type,
            raw_xml: Some(raw_xml.to_owned()),
        }),
        position: Position {
            x: Emu(100_000),
            y: Emu(200_000),
        },
        size: Size {
            width: Emu(3_000_000),
            height: Emu(2_000_000),
        },
        ..Default::default()
    }
}

#[test]
fn existing_fallbacks_emit_typed_diagnostics_with_locations() {
    let presentation = Presentation {
        slides: vec![Slide {
            shapes: vec![
                unsupported_shape(
                    UnresolvedType::SmartArt,
                    "SmartArt",
                    "<dgm:relIds r:dm=\"rId7\"/>",
                ),
                unsupported_shape(
                    UnresolvedType::OleObject,
                    "OLE Object",
                    "<p:oleObj r:id=\"rId8\"/>",
                ),
                unsupported_shape(UnresolvedType::MathEquation, "Math Equation", "<m:oMath/>"),
                unsupported_shape(
                    UnresolvedType::CustomGeometry,
                    "Custom Geometry",
                    "<a:custGeom/>",
                ),
            ],
            ..Default::default()
        }],
        ..Default::default()
    };

    let result = HtmlRenderer::render_with_options_metadata(
        &presentation,
        &pptx2html_core::ConversionOptions::default(),
    )
    .expect("fallback rendering succeeds");

    assert_eq!(result.diagnostics.len(), 4);
    assert_eq!(result.unresolved_elements.len(), 4);
    assert_eq!(result.unresolved_elements[0].slide_index, 0);
    assert_eq!(
        result.unresolved_elements[0].element_type,
        UnresolvedType::SmartArt
    );
    assert_eq!(
        result.unresolved_elements[0].raw_xml.as_deref(),
        Some("<dgm:relIds r:dm=\"rId7\"/>")
    );
    assert!(result.diagnostics.iter().all(|diagnostic| {
        diagnostic.location.slide_index == Some(0)
            && diagnostic.location.part_name.as_deref() == Some("ppt/slides/slide1.xml")
            && diagnostic.location.position.is_some()
            && diagnostic.location.size.is_some()
            && diagnostic.support_tier.to_string() == "fallback"
            && diagnostic.stage.is_some()
    }));
    assert!(result.diagnostics.iter().any(|diagnostic| {
        diagnostic.fallback_kind == FallbackKind::CustomGeometryPlaceholder
            && diagnostic.location.qualified_element_name.as_deref() == Some("a:custGeom")
    }));
}

#[test]
fn package_inventory_reports_unknown_relationship_and_redacts_external_target() {
    let package = PackageBuilder::new(SlideXml::from_body("").build())
        .with_slide_relationship(Relationship::external(
            "rIdSecret",
            "urn:example:relationships/future-widget",
            "https://user:password@example.test/private/widget.bin?token=secret#fragment",
        ))
        .build()
        .expect("fixture package builds");

    let result = convert_bytes_with_metadata(&package).expect("conversion remains non-fatal");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "OOXML_RELATIONSHIP_UNSUPPORTED")
        .expect("unknown relationship diagnostic");

    assert_eq!(diagnostic.location.slide_index, Some(0));
    assert_eq!(
        diagnostic.location.relationship_id.as_deref(),
        Some("rIdSecret")
    );
    assert_eq!(
        diagnostic.location.relationship_type.as_deref(),
        Some("urn:example:relationships/future-widget")
    );
    assert_eq!(
        diagnostic.raw_reference.as_deref(),
        Some("https://example.test/private/widget.bin")
    );
    assert!(!result.html.contains("token=secret"));
    assert!(!result.html.contains("user:password"));
}

#[test]
fn off_slide_unsupported_part_is_observable_without_a_shape() {
    let package = PackageBuilder::new(SlideXml::from_body("").build())
        .with_part(FeaturePart::extra(
            "ppt/embeddings/object1.bin",
            "application/vnd.ms-office.oleObject",
            b"binary-secret-must-not-appear",
        ))
        .build()
        .expect("fixture package builds");

    let result = convert_bytes_with_metadata(&package).expect("conversion remains non-fatal");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "OOXML_PART_UNSUPPORTED")
        .expect("off-slide part diagnostic");

    assert_eq!(diagnostic.location.slide_index, None);
    assert_eq!(
        diagnostic.location.part_name.as_deref(),
        Some("ppt/embeddings/object1.bin")
    );
    assert_eq!(
        diagnostic.raw_reference.as_deref(),
        Some("ppt/embeddings/object1.bin")
    );
    assert!(!result.html.contains("binary-secret-must-not-appear"));
}

#[test]
fn diagnostics_are_deterministically_ordered_and_deduplicated() {
    let slide = r#"
    <future:widget xmlns:future="urn:example:future" id="one"/>
    <future:widget xmlns:future="urn:example:future" id="one"/>"#;
    let package = PackageBuilder::new(SlideXml::from_body(slide).build())
        .with_slide_relationship(Relationship::external(
            "rIdZ",
            "urn:example:relationships/future-widget",
            "https://example.test/widget",
        ))
        .build()
        .expect("fixture package builds");

    let first = convert_bytes_with_metadata(&package).expect("first conversion succeeds");
    let second = convert_bytes_with_metadata(&package).expect("second conversion succeeds");
    assert_eq!(first.diagnostics, second.diagnostics);

    let keys = first
        .diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.location.part_name.as_deref(),
                diagnostic.location.slide_index,
                diagnostic.location.qualified_element_name.as_deref(),
                diagnostic.location.relationship_id.as_deref(),
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(keys.len(), first.diagnostics.len());
}

#[test]
fn normal_conversion_always_embeds_an_empty_json_array() {
    let package = PackageBuilder::new(SlideXml::from_body("").build())
        .build()
        .expect("fixture package builds");

    let html = convert_bytes(&package).expect("conversion succeeds");
    assert_eq!(script_payload(&html), "[]");
}

#[test]
fn embedded_diagnostic_manifest_is_parseable_json() {
    let package = PackageBuilder::new(SlideXml::from_body("").build())
        .with_part(FeaturePart::extra(
            "ppt/embeddings/object1.bin",
            "application/vnd.ms-office.oleObject",
            b"opaque",
        ))
        .build()
        .expect("fixture package builds");
    let html = convert_bytes(&package).expect("conversion succeeds");
    let status = std::process::Command::new("python3")
        .args([
            "-c",
            "import json,sys; json.loads(sys.argv[1])",
            script_payload(&html),
        ])
        .status()
        .expect("python3 is available for the JSON contract");
    assert!(status.success(), "embedded diagnostics must be valid JSON");
}

#[test]
fn malicious_raw_xml_is_script_safe_and_preserved_in_metadata() {
    const ATTACK: &str = "</script><script>alert(1)</script>";
    let slide = r#"
    <p:graphicFrame>
      <p:nvGraphicFramePr><p:cNvPr id="2" name="Math"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
      <p:xfrm><a:off x="100000" y="200000"/><a:ext cx="3000000" cy="2000000"/></p:xfrm>
      <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/officeDocument/2006/math">
        <m:oMath xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math">&lt;/script&gt;&lt;script&gt;alert(1)&lt;/script&gt;</m:oMath>
      </a:graphicData></a:graphic>
    </p:graphicFrame>"#;
    let package = PackageBuilder::new(SlideXml::from_body(slide).build())
        .build()
        .expect("fixture package builds");

    let result = convert_bytes_with_metadata(&package).expect("conversion remains non-fatal");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "PRESENTATIONML_MATH_FALLBACK")
        .expect("math fallback diagnostic");
    assert!(
        diagnostic
            .raw_reference
            .as_deref()
            .is_some_and(|raw| raw.contains(ATTACK))
    );

    let payload = script_payload(&result.html);
    assert!(!payload.contains(ATTACK));
    assert!(
        payload.contains("\\u003C/script\\u003E\\u003Cscript\\u003Ealert(1)\\u003C/script\\u003E")
    );
    assert_eq!(result.html.matches(SCRIPT_CLOSE).count(), 1);
}
