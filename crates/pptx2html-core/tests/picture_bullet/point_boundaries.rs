use super::{
    FeaturePart, IMAGE_RELATIONSHIP, PNG, PackageBuilder, Relationship,
    convert_bytes_with_options_metadata, options, paragraph, shape, slide,
};
use pptx2html_core::model::{Bullet, BulletSize};
use pptx2html_core::parser::PptxParser;

#[test]
fn picture_point_sizes_follow_official_text_font_size_boundaries() {
    // Given
    let picture_paragraphs = [
        ("99", "Picture below"),
        ("100", "Picture minimum"),
        ("400000", "Picture maximum"),
        ("400001", "Picture above"),
    ]
    .map(|(value, text)| {
        paragraph(
            "r:embed=\"rIdBullet\"",
            &format!(r#"<a:buSzPts val="{value}"/>"#),
            text,
        )
    })
    .join("");
    let character_paragraphs = [
        ("99", "Character below"),
        ("100", "Character minimum"),
        ("400000", "Character maximum"),
        ("400001", "Character above"),
    ]
    .map(|(value, text)| {
        format!(
            r#"<a:p><a:pPr><a:buSzPts val="{value}"/><a:buChar char="*"/></a:pPr><a:r><a:rPr sz="2000"/><a:t>{text}</a:t></a:r></a:p>"#
        )
    })
    .join("");
    let package = PackageBuilder::new(slide(&shape(&format!(
        "{picture_paragraphs}{character_paragraphs}"
    ))))
    .with_slide_relationship(Relationship::internal(
        "rIdBullet",
        IMAGE_RELATIONSHIP,
        "../media/bullet.png",
    ))
    .with_part(FeaturePart::media("bullet.png", "image/png", PNG))
    .build()
    .expect("point boundary fixture builds");

    // When
    let presentation = PptxParser::parse_bytes(&package).expect("point boundary fixture parses");
    let result = convert_bytes_with_options_metadata(&package, &options(true))
        .expect("point boundary fixture renders");

    // Then
    let paragraphs = &presentation.slides[0].shapes[0]
        .text_body
        .as_ref()
        .expect("shape text body")
        .paragraphs;
    let picture_sizes = paragraphs[..4]
        .iter()
        .map(
            |paragraph| match paragraph.bullet.as_ref().expect("picture bullet") {
                Bullet::Picture(picture) => picture.size,
                bullet => panic!("expected picture bullet, got {bullet:?}"),
            },
        )
        .collect::<Vec<_>>();
    assert_eq!(
        picture_sizes,
        vec![
            Some(BulletSize::Text),
            Some(BulletSize::Points(1.0)),
            Some(BulletSize::Points(4_000.0)),
            Some(BulletSize::Text),
        ]
    );
    let character_sizes = paragraphs[4..]
        .iter()
        .map(
            |paragraph| match paragraph.bullet.as_ref().expect("character bullet") {
                Bullet::Char(character) => character.size_pct.expect("point size sentinel"),
                bullet => panic!("expected character bullet, got {bullet:?}"),
            },
        )
        .collect::<Vec<_>>();
    assert_eq!(character_sizes, vec![-0.99, -1.0, -4_000.0, -4_000.01]);
    assert_eq!(result.html.matches("height: 20.0pt").count(), 2);
    assert_eq!(result.html.matches("height: 1.0pt").count(), 1);
    assert_eq!(result.html.matches("height: 4000.0pt").count(), 1);
    assert!(!result.html.contains("height: 0.9pt"));
    assert!(!result.html.contains("height: 4000.1pt"));
}
