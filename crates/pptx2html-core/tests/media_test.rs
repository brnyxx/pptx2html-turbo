mod fixtures;

use pptx2html_core::model::{MediaFailure, MediaKind};
use pptx2html_core::{convert_bytes_with_metadata, parser::PptxParser};

use fixtures::MinimalPptx;

const AUDIO_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/audio";
const VIDEO_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/video";

fn shape(kind: &str, relationship_id: &str, action: bool) -> String {
    let action = if action {
        r#"<a:hlinkClick action="ppaction://media"/>"#
    } else {
        ""
    };
    format!(
        r#"<p:pic><p:nvPicPr><p:cNvPr id="2" name="media">{action}</p:cNvPr><p:cNvPicPr/><p:nvPr><a:{kind}File r:link="{relationship_id}"/></p:nvPr></p:nvPicPr><p:blipFill/><p:spPr><a:xfrm><a:off x="100000" y="200000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm></p:spPr></p:pic>"#
    )
}

fn wav() -> Vec<u8> {
    let samples = [0_u8; 16];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36_u32 + samples.len() as u32).to_le_bytes());
    bytes.extend_from_slice(
        b"WAVEfmt \x10\0\0\0\x01\0\x01\0\x40\x1f\0\0\x80\x3e\0\0\x02\0\x10\0data",
    );
    bytes.extend_from_slice(&(samples.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&samples);
    bytes
}

fn package(kind: &str, rel_type: &str, target: &str, mode: Option<&str>, data: &[u8]) -> Vec<u8> {
    let mode = mode
        .map(|value| format!(r#" TargetMode="{value}""#))
        .unwrap_or_default();
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{rel_type}" Target="{target}"{mode}/></Relationships>"#
    );
    MinimalPptx::new(&shape(kind, "rIdMedia", true))
        .with_slide_rels(&rels)
        .with_extra_file("ppt/media/media.wav", data)
        .build()
}

#[test]
fn internal_official_pcm_wav_renders_native_controls_and_media_action() {
    let pptx = package("audio", AUDIO_REL, "../media/media.wav", None, &wav());
    let presentation = PptxParser::parse_bytes(&pptx).expect("fixture parses");
    let media = presentation.slides[0].shapes[0]
        .media
        .as_ref()
        .expect("typed media");
    assert_eq!(media.kind, MediaKind::Audio);
    assert!(media.failure.is_none());

    let result = convert_bytes_with_metadata(&pptx).expect("fixture converts");
    assert!(
        result
            .html
            .contains("<audio class=\"shape-media shape-audio\" controls")
    );
    assert!(result.html.contains("data:audio/wav;base64,"));
    assert!(!result.html.contains("autoplay"));
    assert!(result.html.contains("data-action=\"media\""));
    assert!(result.html.contains("m.play()"));
}

#[test]
fn external_media_is_never_loaded_and_has_typed_placeholder_fallback() {
    let pptx = package(
        "video",
        VIDEO_REL,
        "https://user:secret@example.invalid/private.mp4?token=secret",
        Some("External"),
        b"not used",
    );
    let presentation = PptxParser::parse_bytes(&pptx).expect("fixture parses");
    let media = presentation.slides[0].shapes[0]
        .media
        .as_ref()
        .expect("typed media");
    assert_eq!(media.failure, Some(MediaFailure::ExternalTarget));

    let result = convert_bytes_with_metadata(&pptx).expect("fixture converts");
    assert!(
        result
            .html
            .contains("data-media-fallback=\"external-target\"")
    );
    assert!(!result.html.contains("example.invalid"));
    assert!(!result.html.contains("token=secret"));
    assert!(result.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "DRAWINGML_MEDIA_EXTERNAL_TARGET"
            && diagnostic.raw_reference.as_deref() == Some("rIdMedia")
    }));
}

#[test]
fn wrong_relationship_type_and_unsafe_owner_relative_path_fall_back() {
    for (relationship_type, target, expected) in [
        (
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image",
            "../media/media.wav",
            MediaFailure::WrongRelationshipType,
        ),
        (AUDIO_REL, "../../../escape.wav", MediaFailure::UnsafeTarget),
    ] {
        let pptx = package("audio", relationship_type, target, None, &wav());
        let presentation = PptxParser::parse_bytes(&pptx).expect("fixture parses");
        assert_eq!(
            presentation.slides[0].shapes[0]
                .media
                .as_ref()
                .and_then(|media| media.failure),
            Some(expected)
        );
        let result = convert_bytes_with_metadata(&pptx).expect("fixture converts");
        assert!(!result.html.contains("<audio"));
        assert!(
            result
                .html
                .contains("class=\"media-fallback media-placeholder\"")
        );
    }
}
