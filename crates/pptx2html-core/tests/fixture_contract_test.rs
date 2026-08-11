mod fixtures;

use std::io::{Cursor, Read};

use fixtures::{FeaturePart, PackageBuilder, Relationship, SlideXml};
use zip::ZipArchive;

fn entry_text(archive: &[u8], path: &str) -> String {
    let mut zip = ZipArchive::new(Cursor::new(archive)).expect("fixture archive opens");
    let mut entry = zip.by_name(path).expect("fixture entry exists");
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .expect("fixture entry is UTF-8 XML");
    text
}

#[test]
fn package_builder_emits_deterministic_feature_parts_when_declared() {
    let given_slide = SlideXml::from_body("<p:sp><p:nvSpPr/></p:sp>")
        .with_alternate_content(
            "<mc:AlternateContent><mc:Choice Requires=\"p14\"/></mc:AlternateContent>",
        )
        .build();
    let given_package = PackageBuilder::new(given_slide)
        .with_slide_relationship(Relationship::internal(
            "rIdChart",
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart",
            "../charts/chart1.xml",
        ))
        .with_slide_relationship(Relationship::external(
            "rIdExternal",
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink",
            "https://example.test/fixture",
        ))
        .with_part(FeaturePart::notes("<p:notes/>"))
        .with_part(FeaturePart::comments("<p:cmLst/>"))
        .with_part(FeaturePart::media("sound1.wav", "audio/wav", b"wave"))
        .with_part(FeaturePart::chart("<c:chartSpace/>"))
        .with_part(FeaturePart::extra(
            "ppt/embeddings/custom.bin",
            "application/vnd.example.custom",
            b"extra",
        ));

    given_package
        .validate()
        .expect("declared package relationships resolve");
    let when_first = given_package.build().expect("first package builds");
    let when_second = given_package.build().expect("second package builds");

    assert_eq!(when_first, when_second);
    let content_types = entry_text(&when_first, "[Content_Types].xml");
    for (part_name, content_type) in [
        (
            "/ppt/notesSlides/notesSlide1.xml",
            "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml",
        ),
        (
            "/ppt/comments/comment1.xml",
            "application/vnd.openxmlformats-officedocument.presentationml.comments+xml",
        ),
        ("/ppt/media/sound1.wav", "audio/wav"),
        (
            "/ppt/charts/chart1.xml",
            "application/vnd.openxmlformats-officedocument.drawingml.chart+xml",
        ),
        (
            "/ppt/embeddings/custom.bin",
            "application/vnd.example.custom",
        ),
    ] {
        assert!(content_types.contains(&format!(
            "PartName=\"{part_name}\" ContentType=\"{content_type}\""
        )));
    }
    assert!(entry_text(&when_first, "ppt/slides/slide1.xml").contains("mc:AlternateContent"));
    assert!(
        entry_text(&when_first, "ppt/slides/_rels/slide1.xml.rels").contains("Id=\"rIdChart\"")
    );
    assert!(
        entry_text(&when_first, "ppt/slides/_rels/slide1.xml.rels")
            .contains("TargetMode=\"External\"")
    );
    assert!(entry_text(&when_first, "ppt/notesSlides/notesSlide1.xml").contains("p:notes"));
    assert!(entry_text(&when_first, "ppt/comments/comment1.xml").contains("p:cmLst"));
    assert!(entry_text(&when_first, "ppt/charts/chart1.xml").contains("c:chartSpace"));
    let mut zip = ZipArchive::new(Cursor::new(when_first)).expect("fixture archive opens");
    assert!(zip.by_name("ppt/media/sound1.wav").is_ok());
    assert!(zip.by_name("ppt/embeddings/custom.bin").is_ok());
}

#[test]
fn package_builder_uses_isolated_temp_namespaces_when_written_twice() {
    let given_package = PackageBuilder::new(SlideXml::from_body("").build());

    let when_first = given_package
        .write_to_temp("fixture-contract")
        .expect("first temporary package writes");
    let when_second = given_package
        .write_to_temp("fixture-contract")
        .expect("second temporary package writes");

    assert_ne!(when_first.path().parent(), when_second.path().parent());
    assert!(when_first.path().is_file());
    assert!(when_second.path().is_file());
}

#[test]
fn relationship_validator_reports_dangling_relationship_when_target_is_missing() {
    let given_package = PackageBuilder::new(SlideXml::from_body("").build())
        .with_slide_relationship(Relationship::internal(
            "rIdMissing",
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image",
            "../media/missing.png",
        ));

    let when_error = given_package
        .validate()
        .expect_err("missing relationship target is rejected");

    assert_eq!(when_error.code(), "DANGLING_RELATIONSHIP");
    assert_eq!(when_error.target(), Some("ppt/media/missing.png"));
}
