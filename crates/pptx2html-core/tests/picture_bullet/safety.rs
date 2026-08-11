use super::{
    CHART_RELATIONSHIP, FeaturePart, IMAGE_RELATIONSHIP, MinimalPptx, PNG, PackageBuilder,
    Relationship, convert_bytes_with_options_metadata, options, paragraph, replace_package_entry,
    shape, slide,
};

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
