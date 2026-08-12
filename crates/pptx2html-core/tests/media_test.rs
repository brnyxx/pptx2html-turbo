mod fixtures;

use pptx2html_core::convert_bytes_with_metadata;

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
fn foreign_relationship_and_foreign_nv_pr_never_authorize_media() {
    let foreign_relationship = r#"<x:Relationships xmlns:x="urn:foreign"><x:Relationship Id="rIdMedia" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/audio" Target="../media/media.wav"/></x:Relationships>"#;
    let pptx = MinimalPptx::new(&shape("audio", "rIdMedia", true))
        .with_slide_rels(foreign_relationship)
        .with_extra_file("ppt/media/media.wav", &wav())
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("foreign relationship converts inertly");
    assert!(!result.html.contains("<audio"));
    assert!(!result.html.contains("data:audio/wav"));

    let foreign_owner = shape("audio", "rIdMedia", true)
        .replace("<p:nvPr>", "<x:nvPr xmlns:x=\"urn:foreign\">")
        .replace("</p:nvPr>", "</x:nvPr>");
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{AUDIO_REL}" Target="../media/media.wav"/></Relationships>"#
    );
    let pptx = MinimalPptx::new(&foreign_owner)
        .with_slide_rels(&rels)
        .with_extra_file("ppt/media/media.wav", &wav())
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("foreign nvPr converts inertly");
    assert!(!result.html.contains("<audio"));
    assert!(!result.html.contains("data:audio/wav"));
}

#[test]
fn requested_metadata_is_preserved_but_autoplay_is_never_invented() {
    let media = shape("audio", "rIdMedia", true).replace(
        "<a:audioFile r:link=\"rIdMedia\"/>",
        "<a:audioFile r:link=\"rIdMedia\" trimStart=\"125\" trimEnd=\"875\" loop=\"1\" vol=\"42000\" autoplay=\"1\"/>",
    );
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{AUDIO_REL}" Target="../media/media.wav"/></Relationships>"#
    );
    let pptx = MinimalPptx::new(&media)
        .with_slide_rels(&rels)
        .with_extra_file("ppt/media/media.wav", &wav())
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("metadata converts");
    assert!(result.html.contains("data-media-trim-start=\"125\""));
    assert!(result.html.contains("data-media-trim-end=\"875\""));
    assert!(result.html.contains("data-media-loop=\"true\""));
    assert!(result.html.contains("data-media-volume=\"42000\""));
    assert!(
        result
            .html
            .contains("data-media-autoplay-requested=\"true\"")
    );
    assert!(!result.html.contains(" autoplay"));
}

#[test]
fn official_timing_media_node_metadata_is_preserved_without_execution() {
    let timing = r#"<p:timing><p:tnLst><p:video><p:cMediaNode vol="33000"><p:cTn repeatCount="indefinite"/><p:tgtEl><p:spTgt spid="2"/></p:tgtEl><p:stCondLst><p:cond delay="0"/></p:stCondLst></p:cMediaNode></p:video></p:tnLst></p:timing>"#;
    let body = format!("{}{timing}", shape("video", "rIdMedia", false));
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{VIDEO_REL}" Target="../media/media.mp4"/></Relationships>"#
    );
    let pptx = MinimalPptx::new(&body)
        .with_slide_rels(&rels)
        .with_extra_file(
            "ppt/media/media.mp4",
            include_bytes!("../../../evaluate/completion_decks/README.md"),
        )
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("timing metadata remains observable");
    assert!(result.html.contains("data-media-volume=\"33000\""));
    assert!(result.html.contains("data-media-loop=\"true\""));
    assert!(!result.html.contains(" autoplay"));
}

#[test]
fn fake_mp4_substrings_and_foreign_content_type_attributes_are_rejected() {
    let fake = b"\0\0\0\x18ftypavc1avcCxxxx";
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{VIDEO_REL}" Target="../media/media.mp4"/></Relationships>"#
    );
    let pptx = MinimalPptx::new(&shape("video", "rIdMedia", false))
        .with_slide_rels(&rels)
        .with_extra_file("ppt/media/media.mp4", fake)
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("fake mp4 converts as fallback");
    assert!(!result.html.contains("<video"));
    assert!(
        result
            .html
            .contains("data-media-fallback=\"unsupported-codec\"")
    );
}

#[test]
fn poster_requires_safe_internal_official_image_relationship() {
    let body = shape("audio", "rIdMedia", false).replace(
        "<p:blipFill/>",
        "<p:blipFill><a:blip r:embed=\"rIdPoster\"/></p:blipFill>",
    );
    let rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMedia" Type="{AUDIO_REL}" Target="../media/media.wav"/><Relationship Id="rIdPoster" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="https://user:secret@example.invalid/poster.png?token=secret" TargetMode="External"/></Relationships>"#
    );
    let pptx = MinimalPptx::new(&body)
        .with_slide_rels(&rels)
        .with_extra_file("ppt/media/media.wav", &wav())
        .build();
    let result = convert_bytes_with_metadata(&pptx).expect("unsafe poster stays inert");
    assert!(result.html.contains("<audio"));
    assert!(!result.html.contains(" poster="));
    assert!(!result.html.contains("example.invalid"));
    assert!(!result.html.contains("token=secret"));
}

#[test]
fn internal_official_pcm_wav_renders_native_controls_and_media_action() {
    let pptx = package("audio", AUDIO_REL, "../media/media.wav", None, &wav());
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
            "wrong-relationship-type",
        ),
        (AUDIO_REL, "../../../escape.wav", "unsafe-target"),
    ] {
        let pptx = package("audio", relationship_type, target, None, &wav());
        let result = convert_bytes_with_metadata(&pptx).expect("fixture converts");
        assert!(!result.html.contains("<audio"));
        assert!(
            result
                .html
                .contains("class=\"media-fallback media-placeholder\"")
        );
        assert!(
            result
                .html
                .contains(&format!("data-media-fallback=\"{expected}\""))
        );
    }
}
