use super::{
    FeaturePart, IMAGE_RELATIONSHIP, PNG, PackageBuilder, Relationship,
    convert_bytes_with_options_metadata, options, paragraph, shape, slide,
};

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
