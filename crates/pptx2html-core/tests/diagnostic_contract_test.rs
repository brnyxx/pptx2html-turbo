mod fixtures;

use std::collections::BTreeSet;
use std::io::{Cursor, Read, Write};
use std::process::{Command, Stdio};

use fixtures::{FeaturePart, MinimalPptx, PackageBuilder, Relationship, SlideXml};
use pptx2html_core::model::{
    CapabilityStage, ConversionDiagnostic, DiagnosticLocation, Emu, FallbackKind, FeatureFamily,
    Position, Presentation, Shape, ShapeType, Size, Slide, SupportTier, UnresolvedType,
    UnsupportedData,
};
use pptx2html_core::renderer::HtmlRenderer;
use pptx2html_core::{ConversionResult, convert_bytes, convert_bytes_with_metadata};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

const SCRIPT_OPEN: &str = "<script type=\"application/json\" id=\"pptx2html-diagnostics\">";
const SCRIPT_CLOSE: &str = "</script>";
const IMAGE_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";

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
            custom_geometry: None,
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

fn replace_package_entry(package: &[u8], name: &str, replacement: &[u8]) -> Vec<u8> {
    let mut archive = ZipArchive::new(Cursor::new(package)).expect("fixture package opens");
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .last_modified_time(DateTime::default());
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).expect("fixture entry opens");
        let entry_name = entry.name().to_owned();
        writer
            .start_file(&entry_name, options)
            .expect("fixture entry starts");
        if entry_name == name {
            writer
                .write_all(replacement)
                .expect("replacement entry writes");
        } else {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).expect("fixture entry reads");
            writer.write_all(&bytes).expect("fixture entry writes");
        }
    }
    writer
        .finish()
        .expect("fixture package finishes")
        .into_inner()
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
    assert!(result.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "DRAWINGML_SMARTART_FALLBACK"
            && diagnostic.location.relationship_id.as_deref() == Some("rId7")
    }));
    assert!(result.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "PRESENTATIONML_OLE_FALLBACK"
            && diagnostic.location.relationship_id.as_deref() == Some("rId8")
    }));
}

#[test]
fn custom_formula_relationship_markers_do_not_spoof_or_deduplicate_diagnostics() {
    const FORMULA: &str = "val r:id=\"spoof\"";
    let custom_shape = |id: u32, name: &str| {
        format!(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="{id}" name="{name}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm><a:custGeom><a:avLst/><a:gdLst><a:gd name="bad" fmla='val r:id="spoof"'/></a:gdLst><a:pathLst><a:path w="10" h="10"/></a:pathLst></a:custGeom></p:spPr></p:sp>"#
        )
    };
    let shapes = format!(
        "{}{}",
        custom_shape(2, "first spoof"),
        custom_shape(3, "second spoof")
    );
    let package = MinimalPptx::new(&shapes).build();

    let result = convert_bytes_with_metadata(&package).expect("custom fallbacks remain non-fatal");

    let diagnostics = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "DRAWINGML_CUSTOM_GEOMETRY_FALLBACK")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.raw_reference.as_deref() == Some(FORMULA))
    );
    let relationship_ids = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.location.relationship_id.as_deref())
        .collect::<BTreeSet<_>>();
    assert_eq!(relationship_ids.len(), 2);
    assert!(!relationship_ids.contains("spoof"));
    assert!(
        relationship_ids
            .iter()
            .all(|identity| identity.starts_with("unresolved-s0-e"))
    );
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
        Some("ppt/slides/slide1.xml#rIdSecret")
    );
    assert!(!result.html.contains("token=secret"));
    assert!(!result.html.contains("user:password"));
}

#[test]
fn relationship_inventory_rejects_supported_looking_foreign_uris_without_target_leakage() {
    let package = PackageBuilder::new(SlideXml::from_body("").build())
        .with_slide_relationship(Relationship::internal(
            "rIdKnownImage",
            IMAGE_RELATIONSHIP,
            "../media/public.png",
        ))
        .with_slide_relationship(Relationship::external(
            "rIdSpoofExternal",
            "https://attacker.example/relationships/image",
            "https://user:password@example.test/private/secret.png?token=secret#fragment",
        ))
        .with_slide_relationship(Relationship::internal(
            "rIdSpoofInternal",
            "urn:attacker:relationships/image",
            "../media/top-secret.bin",
        ))
        .with_part(FeaturePart::media("public.png", "image/png", b"public"))
        .with_part(FeaturePart::media(
            "top-secret.bin",
            "application/octet-stream",
            b"private",
        ))
        .build()
        .expect("fixture package builds");

    let result = convert_bytes_with_metadata(&package).expect("conversion remains non-fatal");
    let unsupported = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "OOXML_RELATIONSHIP_UNSUPPORTED")
        .collect::<Vec<_>>();

    assert_eq!(unsupported.len(), 2);
    assert_eq!(
        unsupported
            .iter()
            .filter_map(|diagnostic| diagnostic.location.relationship_id.as_deref())
            .collect::<Vec<_>>(),
        ["rIdSpoofExternal", "rIdSpoofInternal"]
    );
    assert!(unsupported.iter().all(|diagnostic| {
        diagnostic.raw_reference.as_deref()
            == diagnostic.location.relationship_id.as_deref().map(|id| {
                if id == "rIdSpoofExternal" {
                    "ppt/slides/slide1.xml#rIdSpoofExternal"
                } else {
                    "ppt/slides/slide1.xml#rIdSpoofInternal"
                }
            })
    }));
    for secret in ["user:password", "token=secret", "top-secret.bin"] {
        assert!(!result.html.contains(secret));
    }
}

#[test]
fn root_relationship_diagnostic_uses_the_package_root_location() {
    let package = PackageBuilder::new(SlideXml::from_body("").build())
        .build()
        .expect("fixture package builds");
    let root_relationships = r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
  <Relationship Id="rIdRootUnknown" Type="urn:example:relationships/root-secret" Target="https://user:password@example.test/private?token=secret" TargetMode="External"/>
</Relationships>"#;
    let package = replace_package_entry(&package, "_rels/.rels", root_relationships.as_bytes());

    let result = convert_bytes_with_metadata(&package).expect("conversion remains non-fatal");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.location.relationship_id.as_deref() == Some("rIdRootUnknown"))
        .expect("root relationship diagnostic exists");

    assert_eq!(diagnostic.location.part_name.as_deref(), Some("/"));
    assert_eq!(diagnostic.location.slide_index, None);
    assert_eq!(
        diagnostic.raw_reference.as_deref(),
        Some("/#rIdRootUnknown")
    );
    assert!(!result.html.contains("user:password"));
    assert!(!result.html.contains("token=secret"));
}

#[test]
fn element_inventory_uses_resolved_namespace_and_local_name_for_start_and_empty_elements() {
    let slide = r#"
    <alt:sp xmlns:alt="http://schemas.openxmlformats.org/presentationml/2006/main"/>
    <p:futureStart id="same"></p:futureStart>
    <p:futureStart id="same"></p:futureStart>
    <a:futureEmpty id="empty"/>
    <p:sp xmlns:p="urn:spoof" id="spoof"/>
    <!-- <p:futureComment id="comment"/> -->
    <a:t>&lt;p:futureText id="text"/&gt;</a:t>"#;
    let package = PackageBuilder::new(SlideXml::from_body(slide).build())
        .build()
        .expect("fixture package builds");

    let first = convert_bytes_with_metadata(&package).expect("first conversion succeeds");
    let second = convert_bytes_with_metadata(&package).expect("second conversion succeeds");
    let names = first
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "OOXML_ELEMENT_UNSUPPORTED")
        .filter_map(|diagnostic| diagnostic.location.qualified_element_name.as_deref())
        .collect::<Vec<_>>();

    assert_eq!(names, ["a:futureEmpty", "p:futureStart", "p:sp"]);
    assert_eq!(first.diagnostics, second.diagnostics);
    assert!(!names.contains(&"alt:sp"));
    assert!(!names.contains(&"p:futureComment"));
    assert!(!names.contains(&"p:futureText"));
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
    let status = Command::new("python3")
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
    const ATTACK: &str = "</script><script>alert(1)</script>\0\u{1}\t\n\r<&>\u{2028}\u{2029}";
    let presentation = Presentation {
        slides: vec![Slide {
            shapes: vec![unsupported_shape(
                UnresolvedType::MathEquation,
                "Math Equation",
                ATTACK,
            )],
            ..Default::default()
        }],
        ..Default::default()
    };
    let result = HtmlRenderer::render_with_options_metadata(
        &presentation,
        &pptx2html_core::ConversionOptions::default(),
    )
    .expect("conversion remains non-fatal");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "PRESENTATIONML_MATH_FALLBACK")
        .expect("math fallback diagnostic");
    assert_eq!(diagnostic.raw_reference.as_deref(), Some(ATTACK));

    let payload = script_payload(&result.html);
    assert_eq!(result.diagnostics_json(), payload);
    assert!(!payload.contains(ATTACK));
    assert_eq!(result.html.matches(SCRIPT_CLOSE).count(), 1);

    let mut child = Command::new("python3")
        .args([
            "-c",
            "import json,sys; data=sys.stdin.buffer.read(); size,data=data.split(b'\\n',1); size=int(size); expected=data[:size].decode(); payload=data[size:].decode(); actual=next(item['raw_reference'] for item in json.loads(payload) if item['code']=='PRESENTATIONML_MATH_FALLBACK'); raise SystemExit(0 if actual == expected else 1)",
        ])
        .stdin(Stdio::piped())
        .spawn()
        .expect("python3 is available for the JSON contract");
    let stdin = child.stdin.as_mut().expect("python stdin is piped");
    writeln!(stdin, "{}", ATTACK.len()).expect("length prefix writes");
    stdin
        .write_all(ATTACK.as_bytes())
        .expect("expected value writes");
    stdin
        .write_all(payload.as_bytes())
        .expect("JSON payload writes");
    assert!(
        child.wait().expect("python JSON check completes").success(),
        "hostile raw reference must round-trip exactly"
    );
}

#[test]
fn conversion_result_constructor_provides_a_stable_public_construction_path() {
    let result = ConversionResult::new("<html></html>", 2);

    assert_eq!(result.html, "<html></html>");
    assert_eq!(result.slide_count, 2);
    assert!(result.external_assets.is_empty());
    assert!(result.font_resolution_entries.is_empty());
    assert!(result.provenance_entries.is_empty());
    assert!(result.diagnostics().is_empty());
    assert_eq!(result.diagnostics_json(), "[]");
    assert!(result.unresolved_elements.is_empty());
}

#[test]
fn canonical_json_serializes_authoritative_diagnostics_not_stale_html() {
    let mut result = ConversionResult::new(
        concat!(
            "<html><body>",
            "<script type=\"application/json\" id=\"pptx2html-diagnostics\">[]</script>",
            "</body></html>"
        )
        .to_owned(),
        1,
    );
    result.diagnostics.push(ConversionDiagnostic {
        code: "TEST_FALLBACK".to_owned(),
        family: FeatureFamily::Unsupported,
        support_tier: SupportTier::Fallback,
        stage: Some(CapabilityStage::Rendered),
        location: DiagnosticLocation {
            slide_index: Some(0),
            part_name: Some("ppt/slides/slide1.xml".to_owned()),
            ..Default::default()
        },
        raw_reference: Some("</script>\u{2028}\u{2029}".to_owned()),
        fallback_kind: FallbackKind::UnknownElement,
        reason: "script-safe".to_owned(),
    });

    let json = result.diagnostics_json();
    assert!(json.starts_with("[{\"code\":\"TEST_FALLBACK\""), "{json}");
    assert!(
        json.contains("\\u003C/script\\u003E\\u2028\\u2029"),
        "{json}"
    );
    assert!(!json.contains("</script>"), "{json}");
}
