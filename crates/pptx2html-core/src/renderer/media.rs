use std::cell::RefCell;
use std::fmt::Write;

use base64::Engine;

use super::{RenderCtx, UnresolvedCollector};
use crate::model::{
    CapabilityStage, ConversionDiagnostic, DiagnosticLocation, FallbackKind, FeatureFamily,
    ImageFill, MediaData, MediaFailure, MediaKind, Presentation, SupportTier,
};

pub(super) fn render(
    media: &MediaData,
    poster: Option<&ImageFill>,
    shape_id: u32,
    ctx: &RenderCtx<'_>,
    html: &mut String,
) {
    if let Some(failure) = media.failure {
        let metadata = metadata_attributes(media);
        let has_poster = poster.is_some_and(|image| !image.data.is_empty());
        if has_poster && let Some(image) = poster {
            let src = image_source(image, ctx);
            let _ = writeln!(
                html,
                "<img class=\"shape-image media-fallback media-poster\" data-media-kind=\"{}\" data-media-fallback=\"{}\"{metadata} src=\"{src}\" alt=\"Unsupported {}\">",
                media.kind.as_str(),
                failure.as_str(),
                media.kind.as_str(),
            );
        } else {
            let _ = writeln!(
                html,
                "<div class=\"media-fallback media-placeholder\" data-media-kind=\"{}\" data-media-fallback=\"{}\"{metadata} role=\"img\" aria-label=\"Unsupported {}\">[{} unavailable]</div>",
                media.kind.as_str(),
                failure.as_str(),
                media.kind.as_str(),
                media.kind.as_str(),
            );
        }
        push_diagnostic(media, shape_id, has_poster, ctx);
        return;
    }

    let content_type = match media.kind {
        MediaKind::Audio => "audio/wav",
        MediaKind::Video => "video/mp4",
    };
    let source = if ctx.embed_images {
        let encoded = base64::engine::general_purpose::STANDARD.encode(&media.data);
        format!("data:{content_type};base64,{encoded}")
    } else {
        ctx.register_external_asset(media.kind.as_str(), content_type, &media.data)
    };
    match media.kind {
        MediaKind::Audio => {
            let metadata = metadata_attributes(media);
            let _ = writeln!(
                html,
                "<audio class=\"shape-media shape-audio\" controls preload=\"metadata\"{metadata} src=\"{source}\"></audio>"
            );
        }
        MediaKind::Video => {
            let poster_attribute = poster
                .filter(|image| !image.data.is_empty())
                .map(|image| format!(" poster=\"{}\"", image_source(image, ctx)))
                .unwrap_or_default();
            let metadata = metadata_attributes(media);
            let _ = writeln!(
                html,
                "<video class=\"shape-media shape-video\" controls preload=\"metadata\"{metadata}{poster_attribute} src=\"{source}\"></video>"
            );
        }
    }
}

fn metadata_attributes(media: &MediaData) -> String {
    let mut attributes = String::new();
    if let Some(value) = media.trim_start {
        let _ = write!(attributes, " data-media-trim-start=\"{value}\"");
    }
    if let Some(value) = media.trim_end {
        let _ = write!(attributes, " data-media-trim-end=\"{value}\"");
    }
    if let Some(value) = media.loop_requested {
        let _ = write!(attributes, " data-media-loop=\"{value}\"");
    }
    if let Some(value) = media.volume {
        let _ = write!(attributes, " data-media-volume=\"{value}\"");
    }
    if let Some(value) = media.autoplay_requested {
        let _ = write!(attributes, " data-media-autoplay-requested=\"{value}\"");
    }
    attributes
}

fn image_source(image: &ImageFill, ctx: &RenderCtx<'_>) -> String {
    let mime = if image.content_type.is_empty() {
        "image/png"
    } else {
        &image.content_type
    };
    if ctx.embed_images {
        let encoded = base64::engine::general_purpose::STANDARD.encode(&image.data);
        format!("data:{mime};base64,{encoded}")
    } else {
        ctx.register_external_asset("media-poster", mime, &image.data)
    }
}

fn diagnostic_reason(
    media: &MediaData,
    failure: MediaFailure,
    shape_id: u32,
    has_poster: bool,
) -> String {
    let fallback = if has_poster { "poster" } else { "placeholder" };
    let content_type = if failure == MediaFailure::UnsupportedContentType {
        media
            .content_type
            .as_deref()
            .map(|value| format!(";content_type={value}"))
            .unwrap_or_default()
    } else {
        String::new()
    };
    format!(
        "kind={};failure={};fallback={fallback};identity=shape-{shape_id}{content_type}",
        media.kind.as_str(),
        failure.as_str()
    )
}

fn push_diagnostic(media: &MediaData, shape_id: u32, has_poster: bool, ctx: &RenderCtx<'_>) {
    let Some(failure) = media.failure else {
        return;
    };
    let slide_index = ctx.collector.borrow().current_slide_index;
    ctx.collector
        .borrow_mut()
        .diagnostics
        .push(ConversionDiagnostic {
            code: failure.code().to_owned(),
            family: FeatureFamily::Media,
            support_tier: SupportTier::Fallback,
            stage: Some(CapabilityStage::Rendered),
            location: DiagnosticLocation {
                slide_index: Some(slide_index),
                part_name: Some(format!("ppt/slides/slide{}.xml", slide_index + 1)),
                relationship_id: Some(media.relationship_id.clone()),
                relationship_type: media.relationship_type.clone(),
                qualified_element_name: Some(
                    match media.kind {
                        MediaKind::Audio => "a:audioFile",
                        MediaKind::Video => "a:videoFile",
                    }
                    .to_owned(),
                ),
                ..Default::default()
            },
            raw_reference: Some(media.relationship_id.clone()),
            fallback_kind: FallbackKind::PreservedPart,
            reason: diagnostic_reason(media, failure, shape_id, has_poster),
        });
}

pub(super) fn append_diagnostics(
    _presentation: &Presentation,
    _collector: &RefCell<UnresolvedCollector>,
) {
}
