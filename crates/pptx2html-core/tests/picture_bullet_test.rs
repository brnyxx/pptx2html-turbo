mod fixtures;

use std::io::{Cursor, Read, Write};

use fixtures::{FeaturePart, MinimalPptx, PackageBuilder, Relationship, SlideXml};
use pptx2html_core::{ConversionOptions, convert_bytes_with_options_metadata};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

const IMAGE_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
const CHART_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart";
const PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 8, 215, 99, 248, 207, 192, 240, 31, 0, 5,
    0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

fn shape(paragraphs: &str) -> String {
    format!(
        r#"<p:sp>
  <p:nvSpPr><p:cNvPr id="2" name="Picture bullets"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="5000000" cy="3000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>
  <p:txBody><a:bodyPr/><a:lstStyle/>{paragraphs}</p:txBody>
</p:sp>"#
    )
}

fn paragraph(reference: &str, size: &str, text: &str) -> String {
    format!(
        r#"<a:p><a:pPr>{size}<a:buBlip><a:blip {reference}/></a:buBlip></a:pPr><a:r><a:rPr sz="2000"/><a:t>{text}</a:t></a:r></a:p>"#
    )
}

fn slide(body: &str) -> String {
    SlideXml::from_body(body).build().replacen(
        "xmlns:mc=",
        "xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:mc=",
        1,
    )
}

fn options(embed_images: bool) -> ConversionOptions {
    ConversionOptions {
        embed_images,
        ..Default::default()
    }
}

fn replace_package_entry(package: &[u8], entry_name: &str, replacement: &[u8]) -> Vec<u8> {
    let mut archive = ZipArchive::new(Cursor::new(package)).expect("fixture archive opens");
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).expect("fixture entry opens");
        let name = entry.name().to_owned();
        writer
            .start_file(&name, SimpleFileOptions::default())
            .expect("fixture replacement entry starts");
        if name == entry_name {
            writer
                .write_all(replacement)
                .expect("replacement content types write");
        } else {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).expect("fixture entry reads");
            writer.write_all(&bytes).expect("fixture entry writes");
        }
    }
    writer
        .finish()
        .expect("replacement archive finishes")
        .into_inner()
}

#[test]
fn embedded_png_has_equivalent_marker_semantics_in_both_asset_modes() {
    // Given
    let body = shape(&paragraph(
        "r:embed=\"rIdBullet\"",
        "<a:buSzTx/>",
        "Visible text",
    ));
    let package = PackageBuilder::new(slide(&body))
        .with_slide_relationship(Relationship::internal(
            "rIdBullet",
            IMAGE_RELATIONSHIP,
            "../media/bullet.png",
        ))
        .with_part(FeaturePart::media("bullet.png", "image/png", PNG))
        .build()
        .expect("picture bullet fixture builds");

    // When
    let embedded = convert_bytes_with_options_metadata(&package, &options(true))
        .expect("embedded conversion succeeds");
    let external = convert_bytes_with_options_metadata(&package, &options(false))
        .expect("external conversion succeeds");

    // Then
    assert_eq!(embedded.html.matches("class=\"picture-bullet\"").count(), 1);
    assert_eq!(external.html.matches("class=\"picture-bullet\"").count(), 1);
    assert!(embedded.html.contains("src=\"data:image/png;base64,"));
    assert!(!external.html.contains("data:image/png"));
    assert!(
        external
            .html
            .contains("src=\"images/slide-1/picture-bullet-0.png\"")
    );
    assert!(embedded.html.contains("height: 20.0pt"));
    assert!(external.html.contains("height: 20.0pt"));
    assert!(embedded.html.contains("Visible text"));
    assert!(external.html.contains("Visible text"));
    assert!(embedded.external_assets.is_empty());
    assert_eq!(external.external_assets.len(), 1);
    assert_eq!(
        external.external_assets[0].relative_path,
        "images/slide-1/picture-bullet-0.png"
    );
    assert_eq!(external.external_assets[0].content_type, "image/png");
    assert_eq!(external.external_assets[0].data, PNG);
    assert!(embedded.diagnostics.is_empty());
    assert!(external.diagnostics.is_empty());
}

#[test]
fn picture_bullet_sizes_follow_text_percentage_boundaries_and_points() {
    // Given
    let paragraphs = [
        paragraph("r:embed=\"rIdBullet\"", "<a:buSzTx/>", "Text size"),
        paragraph(
            "r:embed=\"rIdBullet\"",
            "<a:buSzPct val=\"25000\"/>",
            "Quarter",
        ),
        paragraph(
            "r:embed=\"rIdBullet\"",
            "<a:buSzPct val=\"400000\"/>",
            "Four times",
        ),
        paragraph(
            "r:embed=\"rIdBullet\"",
            "<a:buSzPts val=\"1250\"/>",
            "Points",
        ),
    ]
    .join("");
    let package = PackageBuilder::new(slide(&shape(&paragraphs)))
        .with_slide_relationship(Relationship::internal(
            "rIdBullet",
            IMAGE_RELATIONSHIP,
            "../media/bullet.png",
        ))
        .with_part(FeaturePart::media("bullet.png", "image/png", PNG))
        .build()
        .expect("picture bullet size fixture builds");

    // When
    let result =
        convert_bytes_with_options_metadata(&package, &options(true)).expect("conversion succeeds");

    // Then
    assert_eq!(result.html.matches("class=\"picture-bullet\"").count(), 4);
    for expected in [
        "height: 20.0pt",
        "height: 5.0pt",
        "height: 80.0pt",
        "height: 12.5pt",
    ] {
        assert!(result.html.contains(expected), "missing {expected}");
    }
}

#[test]
fn invalid_picture_bullet_sizes_use_safe_text_size_fallback() {
    // Given
    let paragraphs = [
        paragraph(
            "r:embed=\"rIdBullet\"",
            "<a:buSzPct val=\"24999\"/>",
            "Below",
        ),
        paragraph(
            "r:embed=\"rIdBullet\"",
            "<a:buSzPct val=\"400001\"/>",
            "Above",
        ),
        paragraph(
            "r:embed=\"rIdBullet\"",
            "<a:buSzPct val=\"-1\"/>",
            "Negative",
        ),
        paragraph("r:embed=\"rIdBullet\"", "<a:buSzPct val=\"NaN\"/>", "NaN"),
        paragraph(
            "r:embed=\"rIdBullet\"",
            "<a:buSzPct val=\"inf\"/>",
            "Infinity",
        ),
        paragraph(
            "r:embed=\"rIdBullet\"",
            "<a:buSzPct val=\"1e999\"/>",
            "Overflow",
        ),
        paragraph("r:embed=\"rIdBullet\"", "<a:buSzPct/>", "Missing"),
        paragraph(
            "r:embed=\"rIdBullet\"",
            "<a:buSzPts val=\"-1\"/>",
            "Bad points",
        ),
    ]
    .join("");
    let package = PackageBuilder::new(slide(&shape(&paragraphs)))
        .with_slide_relationship(Relationship::internal(
            "rIdBullet",
            IMAGE_RELATIONSHIP,
            "../media/bullet.png",
        ))
        .with_part(FeaturePart::media("bullet.png", "image/png", PNG))
        .build()
        .expect("invalid size fixture builds");

    // When
    let result = convert_bytes_with_options_metadata(&package, &options(true))
        .expect("invalid size conversion succeeds");

    // Then
    assert_eq!(result.html.matches("class=\"picture-bullet\"").count(), 8);
    assert_eq!(result.html.matches("height: 20.0pt").count(), 8);
    for unsafe_css in [
        "height: NaN",
        "height: inf",
        "height: Infinity",
        "height: -",
    ] {
        assert!(
            !result.html.contains(unsafe_css),
            "unsafe CSS token {unsafe_css}"
        );
    }
}

#[test]
fn missing_reference_and_duplicate_failures_have_deterministic_identity() {
    // Given
    let paragraphs = [
        paragraph("", "<a:buSzTx/>", "No reference"),
        paragraph("r:embed=\"rIdMissing\"", "<a:buSzTx/>", "Missing one"),
        paragraph("r:embed=\"rIdMissing\"", "<a:buSzTx/>", "Missing two"),
    ]
    .join("");
    let package = PackageBuilder::new(slide(&shape(&paragraphs)))
        .build()
        .expect("missing picture references fixture builds");

    // When
    let first = convert_bytes_with_options_metadata(&package, &options(true))
        .expect("first fallback conversion succeeds");
    let second = convert_bytes_with_options_metadata(&package, &options(true))
        .expect("second fallback conversion succeeds");

    // Then
    assert_eq!(first.html.matches("picture-bullet-missing").count(), 3);
    let diagnostics = first
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "PICTURE_BULLET_IMAGE_MISSING")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].location.relationship_id, None);
    assert_eq!(
        diagnostics[1].location.relationship_id.as_deref(),
        Some("rIdMissing")
    );
    assert_eq!(first.diagnostics, second.diagnostics);
}

#[test]
fn wrong_kind_diagnostic_preserves_actual_relationship_type() {
    // Given
    let package = invalid_picture_bullet_package(
        Some(Relationship::internal(
            "rIdBullet",
            CHART_RELATIONSHIP,
            "../media/bullet.png",
        )),
        Some(FeaturePart::media("bullet.png", "image/png", PNG)),
    );

    // When
    let result = convert_bytes_with_options_metadata(&package, &options(true))
        .expect("wrong kind conversion succeeds with fallback");

    // Then
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "PICTURE_BULLET_IMAGE_MISSING")
        .expect("picture bullet fallback diagnostic exists");
    assert_eq!(
        diagnostic.location.relationship_type.as_deref(),
        Some(CHART_RELATIONSHIP)
    );
    assert!(diagnostic.reason.contains("wrong relationship kind"));
}

#[test]
fn malformed_content_types_xml_uses_safe_visible_fallback() {
    // Given
    let body = shape(&paragraph(
        "r:embed=\"rIdBullet\"",
        "<a:buSzTx/>",
        "Malformed content types visible",
    ));
    let valid = PackageBuilder::new(slide(&body))
        .with_slide_relationship(Relationship::internal(
            "rIdBullet",
            IMAGE_RELATIONSHIP,
            "../media/bullet.png",
        ))
        .with_part(FeaturePart::media("bullet.png", "image/png", PNG))
        .build()
        .expect("valid picture bullet fixture builds");
    let malformed = replace_package_entry(&valid, "[Content_Types].xml", b"<Types><Override");

    // When
    let result = convert_bytes_with_options_metadata(&malformed, &options(true))
        .expect("malformed content types conversion stays non-fatal");

    // Then
    assert!(result.html.contains("Malformed content types visible"));
    assert_eq!(result.html.matches("picture-bullet-missing").count(), 1);
    assert!(!result.html.contains("data:image"));
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "PICTURE_BULLET_IMAGE_MISSING")
        .expect("missing image diagnostic exists");
    assert!(diagnostic.reason.contains("missing package content type"));
}

fn invalid_picture_bullet_package(
    relationship: Option<Relationship>,
    part: Option<FeaturePart>,
) -> Vec<u8> {
    let body = shape(&paragraph(
        "r:embed=\"rIdBullet\"",
        "<a:buSzTx/>",
        "Still visible",
    ));
    let mut builder = PackageBuilder::new(slide(&body));
    if let Some(relationship) = relationship {
        builder = builder.with_slide_relationship(relationship);
    }
    if let Some(part) = part {
        builder = builder.with_part(part);
    }
    builder
        .build()
        .expect("invalid semantic fixture remains structurally valid")
}

#[test]
fn invalid_images_keep_text_and_emit_one_stable_missing_diagnostic_each() {
    // Given
    let cases = [
        invalid_picture_bullet_package(None, None),
        invalid_picture_bullet_package(
            Some(Relationship::internal(
                "rIdBullet",
                CHART_RELATIONSHIP,
                "../media/bullet.png",
            )),
            Some(FeaturePart::media("bullet.png", "image/png", PNG)),
        ),
        invalid_picture_bullet_package(
            Some(Relationship::internal(
                "rIdBullet",
                IMAGE_RELATIONSHIP,
                "../media/empty.png",
            )),
            Some(FeaturePart::media("empty.png", "image/png", b"")),
        ),
        invalid_picture_bullet_package(
            Some(Relationship::internal(
                "rIdBullet",
                IMAGE_RELATIONSHIP,
                "../media/bullet.svg",
            )),
            Some(FeaturePart::media(
                "bullet.svg",
                "image/svg+xml",
                b"<svg><script>alert(1)</script></svg>",
            )),
        ),
    ];

    for package in cases {
        // When
        let result = convert_bytes_with_options_metadata(&package, &options(true))
            .expect("conversion succeeds with fallback");

        // Then
        assert!(result.html.contains("Still visible"));
        assert_eq!(result.html.matches("picture-bullet-missing").count(), 1);
        assert!(!result.html.contains("<script>alert(1)</script>"));
        let diagnostics = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "PICTURE_BULLET_IMAGE_MISSING")
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].location.relationship_id.as_deref(),
            Some("rIdBullet")
        );
        assert_eq!(
            diagnostics[0].location.qualified_element_name.as_deref(),
            Some("a:buBlip")
        );
    }
}

#[test]
fn linked_and_dangling_images_use_fallback_without_exposing_targets() {
    // Given
    let linked = PackageBuilder::new(slide(&shape(&paragraph(
        "r:link=\"rIdBullet\"",
        "<a:buSzTx/>",
        "Linked visible",
    ))))
    .with_slide_relationship(Relationship::external(
        "rIdBullet",
        IMAGE_RELATIONSHIP,
        "https://user:password@example.invalid/bullet.png?token=secret",
    ))
    .build()
    .expect("linked fixture builds");
    let dangling = MinimalPptx::new(&shape(&paragraph(
        "r:embed=\"rIdBullet\"",
        "<a:buSzTx/>",
        "Dangling visible",
    )))
    .with_slide_rels(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdBullet" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/missing.png"/></Relationships>"#,
    )
    .build();

    for package in [linked, dangling] {
        // When
        let result = convert_bytes_with_options_metadata(&package, &options(false))
            .expect("conversion succeeds with fallback");

        // Then
        assert_eq!(result.html.matches("picture-bullet-missing").count(), 1);
        assert!(result.html.contains("visible"));
        assert!(!result.html.contains("password"));
        assert!(!result.html.contains("token=secret"));
        assert!(result.external_assets.is_empty());
        assert_eq!(
            result
                .diagnostics
                .iter()
                .filter(|d| d.code == "PICTURE_BULLET_IMAGE_MISSING")
                .count(),
            1
        );
    }
}

#[test]
fn empty_paragraph_does_not_emit_picture_bullet() {
    // Given
    let body = shape(
        r#"<a:p><a:pPr><a:buSzTx/><a:buBlip><a:blip r:embed="rIdBullet"/></a:buBlip></a:pPr></a:p>"#,
    );
    let package = PackageBuilder::new(slide(&body))
        .with_slide_relationship(Relationship::internal(
            "rIdBullet",
            IMAGE_RELATIONSHIP,
            "../media/bullet.png",
        ))
        .with_part(FeaturePart::media("bullet.png", "image/png", PNG))
        .build()
        .expect("empty paragraph fixture builds");

    // When
    let result =
        convert_bytes_with_options_metadata(&package, &options(true)).expect("conversion succeeds");

    // Then
    assert!(!result.html.contains("picture-bullet"));
    assert!(
        result
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "PICTURE_BULLET_IMAGE_MISSING")
    );
}

#[test]
fn table_cell_picture_bullets_share_image_and_fallback_semantics() {
    // Given
    let cell = |relationship_id: &str, text: &str| {
        format!(
            r#"<a:tc><a:txBody><a:bodyPr/><a:p><a:pPr><a:buSzTx/><a:buBlip><a:blip r:embed="{relationship_id}"/></a:buBlip></a:pPr><a:r><a:rPr sz="2000"/><a:t>{text}</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>"#
        )
    };
    let table = format!(
        r#"<p:graphicFrame>
  <p:nvGraphicFramePr><p:cNvPr id="3" name="Picture bullet table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
  <p:xfrm><a:off x="0" y="0"/><a:ext cx="4000000" cy="1000000"/></p:xfrm>
  <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
    <a:tbl><a:tblPr/><a:tblGrid><a:gridCol w="2000000"/><a:gridCol w="2000000"/></a:tblGrid>
      <a:tr h="1000000">{}{}</a:tr>
    </a:tbl>
  </a:graphicData></a:graphic>
</p:graphicFrame>"#,
        cell("rIdBullet", "Table image"),
        cell("rIdMissing", "Table fallback"),
    );
    let package = PackageBuilder::new(slide(&table))
        .with_slide_relationship(Relationship::internal(
            "rIdBullet",
            IMAGE_RELATIONSHIP,
            "../media/bullet.png",
        ))
        .with_part(FeaturePart::media("bullet.png", "image/png", PNG))
        .build()
        .expect("table picture bullet fixture builds");

    // When
    let result = convert_bytes_with_options_metadata(&package, &options(false))
        .expect("table picture bullet conversion succeeds");

    // Then
    assert_eq!(result.html.matches("class=\"picture-bullet\"").count(), 1);
    assert_eq!(result.html.matches("picture-bullet-missing").count(), 1);
    assert!(result.html.contains("Table image"));
    assert!(result.html.contains("Table fallback"));
    assert_eq!(result.external_assets.len(), 1);
    assert_eq!(result.external_assets[0].data, PNG);
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "PICTURE_BULLET_IMAGE_MISSING")
            .count(),
        1
    );
}

#[test]
fn shape_list_style_picture_bullet_is_inherited_by_paragraph() {
    // Given
    let body = r#"<p:sp>
  <p:nvSpPr><p:cNvPr id="4" name="Inherited picture bullet"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="3000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr>
  <p:txBody><a:bodyPr/><a:lstStyle><a:lvl1pPr><a:buSzTx/><a:buBlip><a:blip r:embed="rIdBullet"/></a:buBlip></a:lvl1pPr></a:lstStyle>
    <a:p><a:pPr lvl="0"/><a:r><a:rPr sz="2000"/><a:t>Inherited visible</a:t></a:r></a:p>
  </p:txBody>
</p:sp>"#;
    let package = PackageBuilder::new(slide(body))
        .with_slide_relationship(Relationship::internal(
            "rIdBullet",
            IMAGE_RELATIONSHIP,
            "../media/bullet.png",
        ))
        .with_part(FeaturePart::media("bullet.png", "image/png", PNG))
        .build()
        .expect("inherited picture bullet fixture builds");

    // When
    let result = convert_bytes_with_options_metadata(&package, &options(true))
        .expect("inherited picture bullet conversion succeeds");

    // Then
    assert_eq!(result.html.matches("class=\"picture-bullet\"").count(), 1);
    assert!(result.html.contains("height: 20.0pt"));
    assert!(result.html.contains("Inherited visible"));
    assert!(
        result
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "PICTURE_BULLET_IMAGE_MISSING")
    );
}

#[test]
fn owner_inherited_picture_bullets_emit_explicit_fallback_diagnostics() {
    // Given
    let visible = shape(r#"<a:p><a:r><a:rPr sz="2000"/><a:t>Owner visible</a:t></a:r></a:p>"#);
    let relationship = |required: &str| {
        format!(
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{required}<Relationship Id="rIdPB" Type="{IMAGE_RELATIONSHIP}" Target="https://user:password@example.invalid/bullet.png?token=secret" TargetMode="External"/></Relationships>"#
        )
    };
    let bullet_style =
        r#"<a:lvl1pPr><a:buSzTx/><a:buBlip><a:blip r:link="rIdPB"/></a:buBlip></a:lvl1pPr>"#;
    let presentation = format!(
        r#"<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/><p:defaultTextStyle>{bullet_style}</p:defaultTextStyle></p:presentation>"#
    );
    let master = format!(
        r#"<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><p:txStyles><p:titleStyle/><p:bodyStyle>{bullet_style}</p:bodyStyle><p:otherStyle/></p:txStyles><p:clrMap/></p:sldMaster>"#
    );
    let layout = format!(
        r#"<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="layout style"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle>{bullet_style}</a:lstStyle></p:txBody></p:sp></p:spTree></p:cSld></p:sldLayout>"#
    );
    let standard_presentation_rels = r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>"#;
    let standard_master_rels = r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>"#;
    let standard_layout_rels = r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>"#;
    let presentation_package = MinimalPptx::new(&visible)
        .with_presentation_xml(&presentation)
        .build();
    let presentation_package = replace_package_entry(
        &presentation_package,
        "ppt/_rels/presentation.xml.rels",
        relationship(standard_presentation_rels).as_bytes(),
    );
    let master_package = MinimalPptx::new(&visible).with_full_master(&master).build();
    let master_package = replace_package_entry(
        &master_package,
        "ppt/slideMasters/_rels/slideMaster1.xml.rels",
        relationship(standard_master_rels).as_bytes(),
    );
    let cases = [
        (presentation_package, "ppt/presentation.xml"),
        (master_package, "ppt/slideMasters/slideMaster1.xml"),
        (
            MinimalPptx::new(&visible)
                .with_full_master(&master.replace(bullet_style, ""))
                .with_layout(&layout)
                .with_layout_rels(&relationship(standard_layout_rels))
                .with_slide_layout_rel()
                .build(),
            "ppt/slideLayouts/slideLayout1.xml",
        ),
    ];

    for (package, owner_part) in cases {
        // When
        let result = convert_bytes_with_options_metadata(&package, &options(true))
            .expect("owner inheritance fallback conversion succeeds");

        // Then
        assert!(result.html.contains("Owner visible"));
        let diagnostics = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "PICTURE_BULLET_INHERITANCE_UNSUPPORTED")
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 1, "owner {owner_part}");
        assert_eq!(
            diagnostics[0].location.part_name.as_deref(),
            Some(owner_part)
        );
        assert_eq!(
            diagnostics[0].location.relationship_id.as_deref(),
            Some("rIdPB")
        );
        assert_eq!(
            diagnostics[0].location.relationship_type.as_deref(),
            Some(IMAGE_RELATIONSHIP)
        );
        assert_eq!(
            diagnostics[0].location.qualified_element_name.as_deref(),
            Some("a:buBlip")
        );
        assert!(diagnostics[0].reason.contains("External"));
        assert!(!result.html.contains("password"));
        assert!(!result.html.contains("token=secret"));
    }
}
