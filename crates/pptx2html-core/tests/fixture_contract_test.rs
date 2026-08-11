mod fixtures;

use std::io::{Cursor, Read};

use fixtures::{FeaturePart, PackageBuilder, Relationship, SlideXml};
use quick_xml::{events::Event, name::ResolveResult, reader::NsReader};
use zip::ZipArchive;

const PRESENTATIONML_NAMESPACE: &[u8] =
    b"http://schemas.openxmlformats.org/presentationml/2006/main";
const CHART_NAMESPACE: &[u8] = b"http://schemas.openxmlformats.org/drawingml/2006/chart";

fn entry_text(archive: &[u8], path: &str) -> String {
    let mut zip = ZipArchive::new(Cursor::new(archive)).expect("fixture archive opens");
    let mut entry = zip.by_name(path).expect("fixture entry exists");
    let mut text = String::new();
    entry
        .read_to_string(&mut text)
        .expect("fixture entry is UTF-8 XML");
    text
}

fn assert_document_root_namespace(xml: &str, expected_name: &[u8], expected_namespace: &[u8]) {
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut found_root = false;

    loop {
        let (namespace, event) = reader
            .read_resolved_event_into(&mut buffer)
            .expect("fixture XML parses");
        match event {
            Event::Start(element) | Event::Empty(element) if !found_root => {
                assert_eq!(element.name().as_ref(), expected_name);
                match namespace {
                    ResolveResult::Bound(namespace) => {
                        assert_eq!(namespace.as_ref(), expected_namespace);
                    }
                    ResolveResult::Unbound | ResolveResult::Unknown(_) => {
                        panic!("fixture XML root namespace is declared");
                    }
                }
                found_root = true;
            }
            Event::Eof => {
                assert!(found_root, "fixture XML has a root element");
                return;
            }
            _ => {}
        }
        buffer.clear();
    }
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
fn standalone_xml_feature_parts_have_declared_root_namespaces() {
    let given_package = PackageBuilder::new(SlideXml::from_body("").build())
        .with_part(FeaturePart::notes("<p:extension/>"))
        .with_part(FeaturePart::comments("<p:comment/>"))
        .with_part(FeaturePart::chart("<c:extension/>"));

    given_package.validate().expect("feature part XML is valid");
    let when_package = given_package.build().expect("fixture package builds");

    assert_document_root_namespace(
        &entry_text(&when_package, "ppt/notesSlides/notesSlide1.xml"),
        b"p:notes",
        PRESENTATIONML_NAMESPACE,
    );
    assert_document_root_namespace(
        &entry_text(&when_package, "ppt/comments/comment1.xml"),
        b"p:cmLst",
        PRESENTATIONML_NAMESPACE,
    );
    assert_document_root_namespace(
        &entry_text(&when_package, "ppt/charts/chart1.xml"),
        b"c:chartSpace",
        CHART_NAMESPACE,
    );
}

#[test]
fn package_builder_rejects_duplicate_part_paths() {
    let given_package = PackageBuilder::new(SlideXml::from_body("").build())
        .with_part(FeaturePart::media(
            "duplicate.bin",
            "application/octet-stream",
            b"first",
        ))
        .with_part(FeaturePart::media(
            "duplicate.bin",
            "application/octet-stream",
            b"second",
        ));

    let when_error = given_package
        .validate()
        .expect_err("duplicate part path is rejected");

    assert_eq!(when_error.code(), "DUPLICATE_PART_PATH");
}

#[test]
fn package_builder_rejects_invalid_part_paths() {
    for given_part in [
        FeaturePart::extra("", "application/octet-stream", b"invalid"),
        FeaturePart::extra(
            "/ppt/media/absolute.bin",
            "application/octet-stream",
            b"invalid",
        ),
        FeaturePart::extra("ppt/../outside.bin", "application/octet-stream", b"invalid"),
        FeaturePart::media(
            "nested\\invalid.bin",
            "application/octet-stream",
            b"invalid",
        ),
    ] {
        let given_package =
            PackageBuilder::new(SlideXml::from_body("").build()).with_part(given_part);

        let when_error = given_package
            .validate()
            .expect_err("invalid package part path is rejected");

        assert_eq!(when_error.code(), "INVALID_PART_PATH");
    }
}

#[test]
fn package_builder_rejects_invalid_internal_relationship_targets() {
    for given_target in [
        "",
        ".",
        "..",
        "/ppt/media/absolute.bin",
        "../media\\invalid.bin",
        "../../../outside.bin",
        "./media/relative.bin",
        "../charts//chart1.xml",
    ] {
        let given_package = PackageBuilder::new(SlideXml::from_body("").build())
            .with_slide_relationship(Relationship::internal(
                "rIdInvalid",
                "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image",
                given_target,
            ));

        let when_error = given_package
            .validate()
            .expect_err("invalid internal relationship target is rejected");

        assert_eq!(when_error.code(), "INVALID_RELATIONSHIP_TARGET");
    }
}

#[test]
fn package_builder_rejects_malformed_xml_feature_parts() {
    let given_package = PackageBuilder::new(SlideXml::from_body("").build())
        .with_part(FeaturePart::notes("<p:unclosed>"));

    let when_error = given_package
        .validate()
        .expect_err("malformed feature XML is rejected");

    assert_eq!(when_error.code(), "INVALID_XML_PART");
}

#[test]
fn package_builder_allows_unqualified_xml_names() {
    let given_package = PackageBuilder::new(SlideXml::from_body("").build())
        .with_part(FeaturePart::notes("<extension id=\"1\"/>"));

    given_package
        .validate()
        .expect("unqualified XML names are valid");
}

#[test]
fn package_builder_rejects_unknown_prefixed_xml_elements() {
    let given_package = PackageBuilder::new(SlideXml::from_body("").build())
        .with_part(FeaturePart::notes("<z:bad/>"));

    let when_error = given_package
        .validate()
        .expect_err("unknown XML element prefix is rejected");

    assert_eq!(when_error.code(), "INVALID_XML_PART");
}

#[test]
fn package_builder_rejects_unknown_prefixed_xml_attributes() {
    let given_package = PackageBuilder::new(SlideXml::from_body("").build())
        .with_part(FeaturePart::notes("<p:extension z:bad=\"value\"/>"));

    let when_error = given_package
        .validate()
        .expect_err("unknown XML attribute prefix is rejected");

    assert_eq!(when_error.code(), "INVALID_XML_PART");
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
