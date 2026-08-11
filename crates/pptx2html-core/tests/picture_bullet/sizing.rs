use super::{
    FeaturePart, IMAGE_RELATIONSHIP, PNG, PackageBuilder, Relationship,
    convert_bytes_with_options_metadata, options, paragraph, shape, slide,
};
use pptx2html_core::model::{
    AutoFit, Bullet, BulletSize, Emu, PictureBullet, PictureBulletImage, Presentation, Shape, Size,
    Slide, TextBody, TextParagraph, TextRun, TextStyle,
};
use pptx2html_core::renderer::HtmlRenderer;

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
fn malformed_norm_autofit_never_reaches_picture_bullet_css() {
    // Given
    let body = format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Bad autofit"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr><a:normAutofit fontScale="NaN"/></a:bodyPr><a:lstStyle/>{}</p:txBody></p:sp>"#,
        paragraph("r:embed=\"rIdBullet\"", "<a:buSzTx/>", "Autofit visible")
    );
    let package = PackageBuilder::new(slide(&body))
        .with_slide_relationship(Relationship::internal(
            "rIdBullet",
            IMAGE_RELATIONSHIP,
            "../media/bullet.png",
        ))
        .with_part(FeaturePart::media("bullet.png", "image/png", PNG))
        .build()
        .expect("malformed autofit fixture builds");

    // When
    let result = convert_bytes_with_options_metadata(&package, &options(true))
        .expect("malformed autofit conversion succeeds");

    // Then
    assert!(result.html.contains("Autofit visible"));
    assert!(result.html.contains("height: 20.0pt"));
    assert!(!result.html.contains("NaNpt"));
    assert!(!result.html.contains("infpt"));
}

#[test]
fn public_model_nonfinite_autofit_uses_finite_picture_bullet_size() {
    for (font_scale, bullet_size) in [
        (f64::NAN, BulletSize::Text),
        (f64::INFINITY, BulletSize::Points(f64::INFINITY)),
        (1.0, BulletSize::Points(-1.0)),
        (1.0, BulletSize::Percentage(f64::INFINITY)),
        (1.0, BulletSize::Points(f64::MAX)),
    ] {
        // Given
        let paragraph = TextParagraph {
            bullet: Some(Bullet::Picture(PictureBullet {
                relationship_id: "public-model".to_owned(),
                relationship_mode: None,
                relationship_type: None,
                target_mode: None,
                image: Some(PictureBulletImage {
                    data: PNG.to_vec(),
                    content_type: "image/png".to_owned(),
                }),
                failure: None,
                size: Some(bullet_size),
            })),
            runs: vec![TextRun {
                text: "Public model visible".to_owned(),
                style: TextStyle {
                    font_size: Some(20.0),
                    ..Default::default()
                },
                ..Default::default()
            }],
            ..Default::default()
        };
        let presentation = Presentation {
            slide_size: Size {
                width: Emu(9_144_000),
                height: Emu(6_858_000),
            },
            slides: vec![Slide {
                shapes: vec![Shape {
                    text_body: Some(TextBody {
                        paragraphs: vec![paragraph],
                        auto_fit: AutoFit::Normal {
                            font_scale: Some(font_scale),
                            line_spacing_reduction: None,
                        },
                        ..Default::default()
                    }),
                    ..Default::default()
                }],
                ..Default::default()
            }],
            ..Default::default()
        };

        // When
        let html = HtmlRenderer::render(&presentation).expect("public model renders");

        // Then
        assert!(html.contains("height: 20.0pt"));
        assert!(!html.contains("NaNpt"));
        assert!(!html.contains("infpt"));
    }
}
