mod fixtures;

use std::collections::BTreeSet;

use fixtures::{FeaturePart, PackageBuilder, Relationship};
use pptx2html_core::parser::PptxParser;
use pptx2html_core::renderer::HtmlRenderer;
use pptx2html_core::{ConversionOptions, convert_bytes_with_metadata};

const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/";
const PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 16, 73, 68, 65, 84, 120, 1, 1, 5, 0, 250, 255, 0, 255, 0, 0,
    255, 5, 0, 1, 255, 250, 92, 136, 209, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

fn png_crc(kind: &[u8], data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in kind.iter().chain(data) {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

fn png_chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(kind);
    output.extend_from_slice(data);
    output.extend_from_slice(&png_crc(kind, data).to_be_bytes());
    output
}

fn framed_png(idat: &[u8], corrupt_idat_crc: bool) -> Vec<u8> {
    let mut output = b"\x89PNG\r\n\x1a\n".to_vec();
    output.extend(png_chunk(b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0, 0, 0]));
    let mut idat_chunk = png_chunk(b"IDAT", idat);
    if corrupt_idat_crc {
        let last = idat_chunk.len() - 1;
        idat_chunk[last] ^= 1;
    }
    output.extend(idat_chunk);
    output.extend(png_chunk(b"IEND", &[]));
    output
}

fn slide(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
 xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"
 xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math"
 xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
 xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
 xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
 xmlns:x14="http://schemas.microsoft.com/office/drawing/2010/main">
 <p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{body}</p:spTree></p:cSld>
</p:sld>"#
    )
}

fn frame(id: u32, name: &str, uri: &str, content: &str, x: i64) -> String {
    format!(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="{id}" name="{name}"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="{x}" y="100000"/><a:ext cx="1800000" cy="900000"/></p:xfrm><a:graphic><a:graphicData uri="{uri}">{content}</a:graphicData></a:graphic></p:graphicFrame>"#
    )
}

fn domain_package() -> Vec<u8> {
    let smartart = frame(
        7,
        "SmartArt",
        "http://schemas.openxmlformats.org/drawingml/2006/diagram",
        r#"<dgm:relIds r:dm="rIdData" r:lo="rIdLayout" r:qs="rIdStyle" r:cs="rIdColors"/><a:blip r:embed="rIdSmartPreview"/>"#,
        100000,
    );
    let ole = frame(
        7,
        "OLE",
        "http://schemas.openxmlformats.org/presentationml/2006/ole",
        r#"<p:oleObj r:id="rIdOle" progId="Package"><p:embed/><a:blip r:embed="rIdOlePreview"/></p:oleObj>"#,
        2100000,
    );
    let math = frame(
        8,
        "Math",
        "http://schemas.openxmlformats.org/officeDocument/2006/math",
        r#"<m:oMath><m:r><m:t>x&lt;/script&gt;+1</m:t></m:r></m:oMath>"#,
        4100000,
    );
    let alternate = r#"<mc:AlternateContent><mc:Choice Requires="x14"><p:sp><p:nvSpPr><p:cNvPr id="20" name="unsupported-choice"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>UNSUPPORTED BRANCH</a:t></a:r></a:p></p:txBody></p:sp></mc:Choice><mc:Choice Requires="p a"><p:sp><p:nvSpPr><p:cNvPr id="21" name="supported-choice"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>SUPPORTED BRANCH</a:t></a:r></a:p></p:txBody></p:sp></mc:Choice><mc:Fallback><p:sp><p:nvSpPr><p:cNvPr id="22" name="fallback-branch"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>FALLBACK BRANCH</a:t></a:r></a:p></p:txBody></p:sp></mc:Fallback></mc:AlternateContent>"#;
    PackageBuilder::new(slide(&format!("{smartart}{ole}{math}{alternate}<future:widget xmlns:future=\"urn:test:future\" secret=\"never\"/>")))
        .with_slide_relationship(Relationship::internal("rIdData", &(REL.to_owned()+"diagramData"), "../diagrams/data1.xml"))
        .with_slide_relationship(Relationship::internal("rIdLayout", &(REL.to_owned()+"diagramLayout"), "../diagrams/layout1.xml"))
        .with_slide_relationship(Relationship::internal("rIdStyle", &(REL.to_owned()+"diagramQuickStyle"), "../diagrams/quickStyle1.xml"))
        .with_slide_relationship(Relationship::internal("rIdColors", &(REL.to_owned()+"diagramColors"), "../diagrams/colors1.xml"))
        .with_slide_relationship(Relationship::internal("rIdSmartPreview", &(REL.to_owned()+"image"), "../media/smart.png"))
        .with_slide_relationship(Relationship::internal("rIdOle", &(REL.to_owned()+"oleObject"), "../embeddings/object.bin"))
        .with_slide_relationship(Relationship::internal("rIdOlePreview", &(REL.to_owned()+"image"), "../media/ole.png"))
        .with_slide_relationship(Relationship::external("rIdSecret", "urn:test:future-relationship", "https://user:password@example.test/x?token=secret#frag"))
        .with_part(FeaturePart::extra("ppt/diagrams/data1.xml", "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml", br#"<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"/>"#))
        .with_part(FeaturePart::extra("ppt/diagrams/layout1.xml", "application/vnd.openxmlformats-officedocument.drawingml.diagramLayout+xml", br#"<dgm:layoutDef xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"/>"#))
        .with_part(FeaturePart::extra("ppt/diagrams/quickStyle1.xml", "application/vnd.openxmlformats-officedocument.drawingml.diagramStyle+xml", br#"<dgm:styleDef xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"/>"#))
        .with_part(FeaturePart::extra("ppt/diagrams/colors1.xml", "application/vnd.openxmlformats-officedocument.drawingml.diagramColors+xml", br#"<dgm:colorsDef xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"/>"#))
        .with_part(FeaturePart::media("smart.png", "image/png", PNG))
        .with_part(FeaturePart::media("ole.png", "image/png", PNG))
        .with_part(FeaturePart::extra("ppt/embeddings/object.bin", "application/vnd.openxmlformats-officedocument.oleObject", b"MZ</script><script>payload()</script>OLE_SECRET"))
        .with_part(FeaturePart::extra("ppt/extensions/future.bin", "application/x-future-secret", b"UNKNOWN_SECRET"))
        .build().expect("domain fixture builds")
}

#[test]
fn smartart_relationship_closure_and_safe_preview_are_preserved() {
    let result = convert_bytes_with_metadata(&domain_package()).expect("conversion succeeds");
    assert!(result.html.contains("data-type=\"smartart\""));
    assert!(result.html.contains("data:image/png;base64,"));
    for identity in [
        "rIdData",
        "rIdLayout",
        "rIdStyle",
        "rIdColors",
        "ppt/diagrams/data1.xml",
        "ppt/diagrams/layout1.xml",
        "ppt/diagrams/quickStyle1.xml",
        "ppt/diagrams/colors1.xml",
    ] {
        assert!(
            result.html.contains(identity),
            "missing closure identity {identity}"
        );
    }
}

#[test]
fn ole_uses_only_allowlisted_preview_and_never_emits_payload() {
    let result = convert_bytes_with_metadata(&domain_package()).expect("conversion succeeds");
    assert!(result.html.contains("data-type=\"ole\""));
    assert!(result.html.contains("rIdOle"));
    for secret in ["OLE_SECRET", "payload()", "MZ</script>", "UNKNOWN_SECRET"] {
        assert!(
            !result.html.contains(secret),
            "binary payload leaked: {secret}"
        );
    }
}

#[test]
fn embedded_packages_are_preserved_as_typed_metadata() {
    const PACKAGE: &[u8] = b"PK\x03\x04EMBEDDED_WORKBOOK_SECRET";
    let package = PackageBuilder::new(slide("<p:sp/>"))
        .with_slide_relationship(Relationship::internal(
            "rIdPackage",
            &(REL.to_owned() + "package"),
            "../embeddings/workbook1.xlsx",
        ))
        .with_part(FeaturePart::extra(
            "ppt/embeddings/workbook1.xlsx",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            PACKAGE,
        ))
        .build()
        .expect("embedded package fixture");

    let result = convert_bytes_with_metadata(&package).expect("conversion succeeds");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "EMBEDDED_PACKAGE_METADATA")
        .expect("embedded package metadata");

    assert_eq!(diagnostic.support_tier.as_str(), "fallback");
    assert_eq!(diagnostic.stage.unwrap().as_str(), "parsed");
    assert_eq!(diagnostic.location.slide_index, Some(0));
    assert_eq!(
        diagnostic.location.part_name.as_deref(),
        Some("ppt/embeddings/workbook1.xlsx")
    );
    assert_eq!(
        diagnostic.location.relationship_id.as_deref(),
        Some("rIdPackage")
    );
    let raw = diagnostic
        .raw_reference
        .as_deref()
        .expect("package raw reference");
    assert!(raw.contains("owner=ppt/slides/slide1.xml"));
    assert!(raw.contains("part=ppt/embeddings/workbook1.xlsx"));
    assert!(raw.contains("relationship_id=rIdPackage"));
    assert!(raw.contains(
        "content_type=application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    ));
    assert!(raw.contains(&format!("byte_length={}", PACKAGE.len())));
    assert!(!raw.contains("EMBEDDED_WORKBOOK_SECRET"));
    assert!(!result.html.contains("EMBEDDED_WORKBOOK_SECRET"));
}

#[test]
fn math_raw_metadata_is_bounded_and_script_safe() {
    let result = convert_bytes_with_metadata(&domain_package()).expect("conversion succeeds");
    assert!(result.html.contains("data-type=\"math\""));
    assert!(
        result.html.contains("x\\\\u003c/script\\\\u003e+1")
            || result.html.contains("\\\\u003c/script\\\\u003e+1")
    );
    assert_eq!(result.html.matches("</script>").count(), 1);
    assert!(
        result
            .diagnostics
            .iter()
            .filter_map(|d| d.raw_reference.as_deref())
            .all(|raw| raw.len() <= 16_384)
    );
}

#[test]
fn alternate_content_preserves_all_branches_but_renders_exactly_one_supported_choice() {
    let result = convert_bytes_with_metadata(&domain_package()).expect("conversion succeeds");
    let visible = result
        .html
        .split("<script type=\"application/json\"")
        .next()
        .unwrap();
    assert!(visible.contains("SUPPORTED BRANCH"));
    assert!(!visible.contains("UNSUPPORTED BRANCH"));
    assert!(!visible.contains("FALLBACK BRANCH"));
    let alternate = result
        .diagnostics
        .iter()
        .find(|d| d.code == "OOXML_ALTERNATE_CONTENT_PRESERVED")
        .expect("alternate metadata");
    let raw = alternate.raw_reference.as_deref().expect("branch metadata");
    for token in [
        "x14",
        "p",
        "a",
        "unsupported-choice",
        "supported-choice",
        "fallback-branch",
        "selected_branch\":1",
    ] {
        assert!(
            raw.contains(token),
            "missing AlternateContent token {token}"
        );
    }
}

#[test]
fn spoofed_diagram_namespace_and_preview_mime_are_rejected() {
    let smartart = frame(
        9,
        "Spoofed SmartArt",
        "http://schemas.openxmlformats.org/drawingml/2006/diagram",
        r#"<dgm:relIds r:dm="rIdSpoofData"/><a:blip r:embed="rIdSpoofPreview"/>"#,
        100000,
    );
    let package = PackageBuilder::new(slide(&smartart))
        .with_slide_relationship(Relationship::internal(
            "rIdSpoofData",
            &(REL.to_owned() + "diagramData"),
            "../diagrams/spoof.xml",
        ))
        .with_slide_relationship(Relationship::internal(
            "rIdSpoofPreview",
            &(REL.to_owned() + "image"),
            "../media/spoof.png",
        ))
        .with_part(FeaturePart::extra(
            "ppt/diagrams/spoof.xml",
            "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml",
            br#"<dgm:dataModel xmlns:dgm="urn:spoof"/>"#,
        ))
        .with_part(FeaturePart::media("spoof.png", "text/html", PNG))
        .build()
        .expect("spoof fixture builds");

    let result = convert_bytes_with_metadata(&package).expect("conversion remains non-fatal");
    assert!(!result.html.contains("data:image/png;base64,"));
    assert!(result.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "OOXML_EMBEDDED_RELATIONSHIP_INVALID"
            && diagnostic.location.relationship_id.as_deref() == Some("rIdSpoofData")
    }));
}

#[test]
fn namespace_scope_controls_alternate_content_selection() {
    let alternate = r#"<mc:AlternateContent xmlns:x="urn:unsupported"><mc:Choice Requires="x"><p:sp><p:nvSpPr><p:cNvPr id="31" name="bad"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>SCOPED BAD CHOICE</a:t></a:r></a:p></p:txBody></p:sp></mc:Choice><mc:Fallback><p:sp><p:nvSpPr><p:cNvPr id="32" name="good"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>SCOPED GOOD FALLBACK</a:t></a:r></a:p></p:txBody></p:sp></mc:Fallback></mc:AlternateContent><p:extLst xmlns:x="http://schemas.openxmlformats.org/presentationml/2006/main"/>"#;
    let package = PackageBuilder::new(slide(alternate))
        .build()
        .expect("scoped fixture builds");

    let result = convert_bytes_with_metadata(&package).expect("conversion succeeds");
    let visible = result
        .html
        .split("<script type=\"application/json\"")
        .next()
        .unwrap();
    assert!(visible.contains("SCOPED GOOD FALLBACK"));
    assert!(!visible.contains("SCOPED BAD CHOICE"));
    let metadata = result
        .diagnostics
        .iter()
        .find(|item| item.code == "OOXML_ALTERNATE_CONTENT_PRESERVED")
        .and_then(|item| item.raw_reference.as_deref())
        .expect("alternate metadata");
    assert!(metadata.contains("selected_branch\":1"));
}

#[test]
fn nested_alternate_content_inherits_and_shadows_lexical_namespaces() {
    let nested = r#"<mc:AlternateContent xmlns:x="urn:unsupported"><mc:Choice Requires="p"><mc:AlternateContent><mc:Choice Requires="x"><p:sp><p:nvSpPr><p:cNvPr id="37" name="nested-bad"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>NESTED BAD</a:t></a:r></a:p></p:txBody></p:sp></mc:Choice><mc:Fallback><p:sp><p:nvSpPr><p:cNvPr id="38" name="nested-good"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>NESTED GOOD</a:t></a:r></a:p></p:txBody></p:sp></mc:Fallback></mc:AlternateContent></mc:Choice><mc:Fallback/></mc:AlternateContent>"#;
    let package = PackageBuilder::new(slide(nested))
        .build()
        .expect("nested MC fixture builds");
    let result = convert_bytes_with_metadata(&package).expect("nested MC conversion succeeds");
    let visible = result
        .html
        .split("<script type=\"application/json\"")
        .next()
        .unwrap();
    assert!(visible.contains("NESTED GOOD"));
    assert!(!visible.contains("NESTED BAD"));
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|item| item.code == "OOXML_ALTERNATE_CONTENT_PRESERVED")
            .count(),
        2
    );
}

#[test]
fn smartart_closure_is_transitive_graph_only_and_existing_part_bounded() {
    let smartart = frame(
        33,
        "Transitive SmartArt",
        "http://schemas.openxmlformats.org/drawingml/2006/diagram",
        r#"<dgm:relIds r:dm="rIdData"/>"#,
        100000,
    );
    let package = PackageBuilder::new(slide(&smartart))
        .with_slide_relationship(Relationship::internal("rIdData", &(REL.to_owned() + "diagramData"), "../diagrams/data-root.xml"))
        .with_part(FeaturePart::extra("ppt/diagrams/data-root.xml", "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml", br#"<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"/>"#))
        .with_part(FeaturePart::extra("ppt/diagrams/_rels/data-root.xml.rels", "application/vnd.openxmlformats-package.relationships+xml", format!(r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdNext" Type="{REL}diagramLayout" Target="layout-child.xml"/><Relationship Id="rIdUnrelated" Type="{REL}slide" Target="../private/secret-name.xml"/><Relationship Id="rIdMissing" Type="{REL}diagramColors" Target="missing-colors.xml"/></Relationships>"#).as_bytes()))
        .with_part(FeaturePart::extra("ppt/diagrams/layout-child.xml", "application/vnd.openxmlformats-officedocument.drawingml.diagramLayout+xml", br#"<dgm:layoutDef xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"/>"#))
        .with_part(FeaturePart::extra("ppt/private/secret-name.xml", "application/vnd.openxmlformats-officedocument.presentationml.slide+xml", br#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"/>"#))
        .build()
        .expect("transitive fixture builds");

    let result = convert_bytes_with_metadata(&package).expect("conversion succeeds");
    let smartart = result
        .diagnostics
        .iter()
        .find(|item| item.code == "DRAWINGML_SMARTART_FALLBACK")
        .and_then(|item| item.raw_reference.as_deref())
        .expect("SmartArt metadata");
    assert!(smartart.contains("ppt/diagrams/layout-child.xml"));
    assert!(!smartart.contains("secret-name"));
    assert!(!smartart.contains("missing-colors"));
    assert!(!smartart.contains(&format!("{REL}slide")));
}

#[test]
fn smartart_closure_enforces_depth_count_and_relationship_size_limits() {
    let smartart = frame(
        36,
        "Bounded SmartArt",
        "http://schemas.openxmlformats.org/drawingml/2006/diagram",
        r#"<dgm:relIds r:dm="rIdData"/>"#,
        100000,
    );
    let mut depth_builder =
        PackageBuilder::new(slide(&smartart)).with_slide_relationship(Relationship::internal(
            "rIdData",
            &(REL.to_owned() + "diagramData"),
            "../diagrams/depth-0.xml",
        ));
    for index in 0..=9 {
        depth_builder = depth_builder.with_part(FeaturePart::extra(
            &format!("ppt/diagrams/depth-{index}.xml"),
            "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml",
            br#"<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"/>"#,
        ));
        if index < 9 {
            let relationships = format!(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdNext" Type="{REL}diagramData" Target="depth-{}.xml"/></Relationships>"#,
                index + 1
            );
            depth_builder = depth_builder.with_part(FeaturePart::extra(
                &format!("ppt/diagrams/_rels/depth-{index}.xml.rels"),
                "application/vnd.openxmlformats-package.relationships+xml",
                relationships.as_bytes(),
            ));
        }
    }
    let depth_result =
        convert_bytes_with_metadata(&depth_builder.build().expect("depth fixture builds"))
            .expect("depth conversion succeeds");
    let depth_metadata = depth_result
        .diagnostics
        .iter()
        .find(|item| item.code == "DRAWINGML_SMARTART_FALLBACK")
        .and_then(|item| item.raw_reference.as_deref())
        .expect("depth metadata");
    assert!(depth_metadata.contains("depth-8.xml"));
    assert!(!depth_metadata.contains("depth-9.xml"));

    let mut count_relationships = String::from(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
    );
    let mut count_builder = PackageBuilder::new(slide(&smartart)).with_slide_relationship(
        Relationship::internal("rIdData", &(REL.to_owned() + "diagramData"), "../diagrams/count-root.xml"),
    ).with_part(FeaturePart::extra(
        "ppt/diagrams/count-root.xml",
        "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml",
        br#"<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"/>"#,
    ));
    for index in 0..40 {
        count_relationships.push_str(&format!(r#"<Relationship Id="rId{index:02}" Type="{REL}diagramLayout" Target="count-{index:02}.xml"/>"#));
        count_builder = count_builder.with_part(FeaturePart::extra(
            &format!("ppt/diagrams/count-{index:02}.xml"),
            "application/vnd.openxmlformats-officedocument.drawingml.diagramLayout+xml",
            br#"<dgm:layoutDef xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"/>"#,
        ));
    }
    count_relationships.push_str("</Relationships>");
    count_builder = count_builder.with_part(FeaturePart::extra(
        "ppt/diagrams/_rels/count-root.xml.rels",
        "application/vnd.openxmlformats-package.relationships+xml",
        count_relationships.as_bytes(),
    ));
    let count_result =
        convert_bytes_with_metadata(&count_builder.build().expect("count fixture builds"))
            .expect("count conversion succeeds");
    let count_metadata = count_result
        .diagnostics
        .iter()
        .find(|item| item.code == "DRAWINGML_SMARTART_FALLBACK")
        .and_then(|item| item.raw_reference.as_deref())
        .expect("count metadata");
    assert!(count_metadata.matches("\"part_name\"").count() <= 32);

    let oversized_relationships = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{}<Relationship Id="rIdChild" Type="{REL}diagramLayout" Target="oversized-child.xml"/></Relationships>"#,
        " ".repeat(17_000)
    );
    let size_package = PackageBuilder::new(slide(&smartart))
        .with_slide_relationship(Relationship::internal("rIdData", &(REL.to_owned() + "diagramData"), "../diagrams/oversized-root.xml"))
        .with_part(FeaturePart::extra("ppt/diagrams/oversized-root.xml", "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml", br#"<dgm:dataModel xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"/>"#))
        .with_part(FeaturePart::extra("ppt/diagrams/_rels/oversized-root.xml.rels", "application/vnd.openxmlformats-package.relationships+xml", oversized_relationships.as_bytes()))
        .with_part(FeaturePart::extra("ppt/diagrams/oversized-child.xml", "application/vnd.openxmlformats-officedocument.drawingml.diagramLayout+xml", br#"<dgm:layoutDef xmlns:dgm="http://schemas.openxmlformats.org/drawingml/2006/diagram"/>"#))
        .build()
        .expect("size fixture builds");
    let size_result = convert_bytes_with_metadata(&size_package).expect("size conversion succeeds");
    let size_metadata = size_result
        .diagnostics
        .iter()
        .find(|item| item.code == "DRAWINGML_SMARTART_FALLBACK")
        .and_then(|item| item.raw_reference.as_deref())
        .expect("size metadata");
    assert!(!size_metadata.contains("oversized-child.xml"));
}

#[test]
fn parsed_inventory_is_stable_after_another_parse_and_across_threads() {
    let first = PptxParser::parse_bytes(&domain_package()).expect("first parse succeeds");
    let immediate = HtmlRenderer::render(&first).expect("immediate render succeeds");
    let unrelated = PackageBuilder::new(slide(""))
        .build()
        .expect("other fixture builds");
    PptxParser::parse_bytes(&unrelated).expect("other parse succeeds");
    let after_other_parse = HtmlRenderer::render(&first).expect("later render succeeds");
    let cross_thread =
        std::thread::spawn(move || HtmlRenderer::render(&first).expect("thread render succeeds"))
            .join()
            .expect("renderer thread joins");

    assert!(immediate.contains("ppt/diagrams/data1.xml"));
    assert_eq!(immediate, after_other_parse);
    assert_eq!(immediate, cross_thread);
}

#[test]
fn long_unicode_math_metadata_remains_bounded_valid_json() {
    let text = format!("{}&lt;/script&gt;", "😀".repeat(4_500));
    let math = frame(
        34,
        "Long Math",
        "http://schemas.openxmlformats.org/officeDocument/2006/math",
        &format!(r#"<m:oMath><m:r><m:t>{text}</m:t></m:r></m:oMath>"#),
        100000,
    );
    let package = PackageBuilder::new(slide(&math))
        .build()
        .expect("math fixture builds");
    let result = convert_bytes_with_metadata(&package).expect("conversion succeeds");
    let raw = result
        .diagnostics
        .iter()
        .find(|item| item.code == "PRESENTATIONML_MATH_FALLBACK")
        .and_then(|item| item.raw_reference.as_deref())
        .expect("math metadata");
    assert!(raw.len() <= 16_384);
    assert!(raw.starts_with('{') && raw.ends_with('}'));
    assert!(!raw.contains("</script>"));
    let status = std::process::Command::new("python3")
        .args(["-c", "import json,sys; json.loads(sys.stdin.read())"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child
                .stdin
                .as_mut()
                .expect("piped stdin")
                .write_all(raw.as_bytes())?;
            child.wait()
        })
        .expect("python validates metadata");
    assert!(status.success(), "raw_reference must be valid JSON");
}

#[test]
fn valid_safe_subset_png_preview_is_emitted() {
    let result = convert_bytes_with_metadata(&domain_package()).expect("conversion succeeds");
    assert!(result.html.contains("data:image/png;base64,"));
}

#[test]
fn invalid_idat_and_bad_crc_png_secrets_are_rejected() {
    for (name, png) in [
        (
            "invalid-idat",
            framed_png(b"REVIEWER_INVALID_IDAT_SECRET", false),
        ),
        ("bad-crc", framed_png(&PNG[41..57], true)),
        (
            "compressed-deflate",
            framed_png(
                &[8, 215, 99, 248, 207, 192, 240, 31, 0, 5, 0, 1, 255],
                false,
            ),
        ),
    ] {
        let ole = frame(
            39,
            name,
            "http://schemas.openxmlformats.org/presentationml/2006/ole",
            r#"<p:oleObj r:id="rIdOle"><a:blip r:embed="rIdPreview"/></p:oleObj>"#,
            100000,
        );
        let package = PackageBuilder::new(slide(&ole))
            .with_slide_relationship(Relationship::internal(
                "rIdOle",
                &(REL.to_owned() + "oleObject"),
                "../embeddings/object.bin",
            ))
            .with_slide_relationship(Relationship::internal(
                "rIdPreview",
                &(REL.to_owned() + "image"),
                &format!("../media/{name}.png"),
            ))
            .with_part(FeaturePart::extra(
                "ppt/embeddings/object.bin",
                "application/vnd.openxmlformats-officedocument.oleObject",
                b"object",
            ))
            .with_part(FeaturePart::media(
                &format!("{name}.png"),
                "image/png",
                &png,
            ))
            .build()
            .expect("invalid preview fixture builds");
        let result = convert_bytes_with_metadata(&package).expect("conversion succeeds");
        assert!(
            !result.html.contains("data:image/png;base64,"),
            "accepted {name}"
        );
        assert!(!result.html.contains("REVIEWER_INVALID_IDAT_SECRET"));
    }
}

#[test]
fn signature_plus_arbitrary_suffix_preview_is_rejected() {
    let ole = frame(
        35,
        "Fake Preview",
        "http://schemas.openxmlformats.org/presentationml/2006/ole",
        r#"<p:oleObj r:id="rIdOle"><a:blip r:embed="rIdPreview"/></p:oleObj>"#,
        100000,
    );
    let fake = b"\x89PNG\r\n\x1a\nARBITRARY_SUFFIX_PAYLOAD";
    let package = PackageBuilder::new(slide(&ole))
        .with_slide_relationship(Relationship::internal(
            "rIdOle",
            &(REL.to_owned() + "oleObject"),
            "../embeddings/object.bin",
        ))
        .with_slide_relationship(Relationship::internal(
            "rIdPreview",
            &(REL.to_owned() + "image"),
            "../media/fake.png",
        ))
        .with_part(FeaturePart::extra(
            "ppt/embeddings/object.bin",
            "application/vnd.openxmlformats-officedocument.oleObject",
            b"object",
        ))
        .with_part(FeaturePart::media("fake.png", "image/png", fake))
        .build()
        .expect("fake preview fixture builds");

    let result = convert_bytes_with_metadata(&package).expect("conversion succeeds");
    assert!(!result.html.contains("data:image/png;base64,"));
    assert!(!result.html.contains("QVJCSVRSQVJZX1NVRkZJWF9QQVlMT0FE"));
}

#[test]
fn package_wide_unknown_part_inventory_is_count_and_metadata_bounded() {
    let mut builder = PackageBuilder::new(slide(""));
    for index in 0..5_000 {
        builder = builder.with_part(FeaturePart::extra(
            &format!("ppt/future-domain/part-{index:04}.bin"),
            "application/x-future",
            format!("UNKNOWN_PAYLOAD_SECRET_{index}").as_bytes(),
        ));
    }
    let package = builder.build().expect("large unknown inventory builds");
    let result = convert_bytes_with_metadata(&package).expect("conversion succeeds");
    let parts = result
        .diagnostics
        .iter()
        .filter(|item| item.code == "OOXML_PART_UNSUPPORTED")
        .collect::<Vec<_>>();
    let truncations = result
        .diagnostics
        .iter()
        .filter(|item| item.code == "OOXML_PART_INVENTORY_TRUNCATED")
        .collect::<Vec<_>>();
    assert_eq!(parts.len(), 128);
    assert_eq!(truncations.len(), 1);
    assert_eq!(
        truncations[0].raw_reference.as_deref(),
        Some("{\"omitted_count\":4872}")
    );
    assert!(result.html.len() < 250_000);
    assert!(!result.html.contains("UNKNOWN_PAYLOAD_SECRET"));
    assert_eq!(
        parts
            .first()
            .and_then(|item| item.location.part_name.as_deref()),
        Some("ppt/future-domain/part-0000.bin")
    );
    assert_eq!(
        parts
            .last()
            .and_then(|item| item.location.part_name.as_deref()),
        Some("ppt/future-domain/part-0127.bin")
    );
}

#[test]
fn unknown_package_part_outside_legacy_directories_is_inventoried_without_payload() {
    let package = PackageBuilder::new(slide(""))
        .with_part(FeaturePart::extra(
            "ppt/future-domain/opaque.bin",
            "application/x-future",
            b"FUTURE_PART_SECRET_PAYLOAD",
        ))
        .build()
        .expect("unknown part fixture builds");
    let result = convert_bytes_with_metadata(&package).expect("conversion succeeds");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|item| {
            item.code == "OOXML_PART_UNSUPPORTED"
                && item.location.part_name.as_deref() == Some("ppt/future-domain/opaque.bin")
        })
        .expect("unknown package part diagnostic");
    assert_eq!(
        diagnostic.raw_reference.as_deref(),
        Some("ppt/future-domain/opaque.bin")
    );
    assert!(!result.html.contains("FUTURE_PART_SECRET_PAYLOAD"));
}

#[test]
fn unknown_inventory_and_external_redaction_are_stable() {
    let package = domain_package();
    let first = convert_bytes_with_metadata(&package).expect("first conversion succeeds");
    let second = convert_bytes_with_metadata(&package).expect("second conversion succeeds");
    assert_eq!(first.html, second.html);
    assert_eq!(first.diagnostics, second.diagnostics);
    for code in [
        "OOXML_ELEMENT_UNSUPPORTED",
        "OOXML_RELATIONSHIP_UNSUPPORTED",
        "OOXML_PART_UNSUPPORTED",
    ] {
        assert!(
            first.diagnostics.iter().any(|d| d.code == code),
            "missing {code}"
        );
    }
    for secret in [
        "user:password",
        "token=secret",
        "#frag",
        "secret=\\\"never\\\"",
    ] {
        assert!(!first.html.contains(secret), "secret leaked: {secret}");
    }
}

#[test]
fn duplicate_source_objects_keep_stable_identity_and_unique_visible_ids() {
    let result = convert_bytes_with_metadata(&domain_package()).expect("conversion succeeds");
    let ids = result
        .unresolved_elements
        .iter()
        .map(|item| item.placeholder_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(ids.len(), result.unresolved_elements.len());
    assert!(result.unresolved_elements.len() >= 3);
    assert!(
        ids.iter()
            .all(|id| result.html.matches(&format!("id=\"{id}\"")).count() == 1)
    );
    let selected = pptx2html_core::convert_bytes_with_options_metadata(
        &domain_package(),
        &ConversionOptions {
            slide_indices: Some(vec![1]),
            ..Default::default()
        },
    )
    .expect("selected conversion succeeds");
    assert_eq!(
        result
            .unresolved_elements
            .iter()
            .map(|x| &x.placeholder_id)
            .collect::<Vec<_>>(),
        selected
            .unresolved_elements
            .iter()
            .map(|x| &x.placeholder_id)
            .collect::<Vec<_>>()
    );
}
