use std::fmt::Write;

use base64::Engine;

use crate::model::bullet::{
    TEXT_FONT_SIZE_MAX_HUNDREDTHS_POINT, TEXT_FONT_SIZE_MIN_HUNDREDTHS_POINT,
};

use super::{
    BulletSize, CapabilityStage, ConversionDiagnostic, DEFAULT_FONT_SIZE_PT, DiagnosticLocation,
    FallbackKind, FeatureFamily, ParagraphDefaults, PictureBullet, RenderCtx, SupportTier,
    TextParagraph,
};

const MIN_TEXT_SIZE_PT: f64 = TEXT_FONT_SIZE_MIN_HUNDREDTHS_POINT / 100.0;
const MAX_TEXT_SIZE_PT: f64 = TEXT_FONT_SIZE_MAX_HUNDREDTHS_POINT / 100.0;
const MAX_PICTURE_BULLET_SIZE_PT: f64 = MAX_TEXT_SIZE_PT * 4.0;

pub(super) fn render(
    picture: &PictureBullet,
    paragraph: &TextParagraph,
    inherited: Option<&ParagraphDefaults>,
    font_scale: Option<f64>,
    ctx: &RenderCtx<'_>,
    html: &mut String,
) {
    let Some(image) = picture.image.as_ref() else {
        html.push_str(
            "<span class=\"picture-bullet picture-bullet-missing\" role=\"img\" aria-label=\"Picture bullet unavailable\">&#x25A1;</span> ",
        );
        push_missing_diagnostic(picture, ctx);
        return;
    };

    let text_size = valid_text_size(surrounding_text_size(paragraph, inherited))
        .unwrap_or(DEFAULT_FONT_SIZE_PT);
    let scale = font_scale
        .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
        .unwrap_or(1.0);
    let scaled_text_size = text_size * scale;
    let requested_size = match picture.size.unwrap_or(BulletSize::Text) {
        BulletSize::Text => scaled_text_size,
        BulletSize::Percentage(factor) if factor.is_finite() && (0.25..=4.0).contains(&factor) => {
            scaled_text_size * factor
        }
        BulletSize::Points(points) => valid_text_size(points).unwrap_or(scaled_text_size),
        BulletSize::Percentage(_) => scaled_text_size,
    };
    let size = valid_size(requested_size, MAX_PICTURE_BULLET_SIZE_PT).unwrap_or(scaled_text_size);
    let src = if ctx.embed_images {
        let encoded = base64::engine::general_purpose::STANDARD.encode(&image.data);
        format!("data:{};base64,{encoded}", image.content_type)
    } else {
        ctx.register_external_asset("picture-bullet", &image.content_type, &image.data)
    };
    let _ = write!(
        html,
        "<img class=\"picture-bullet\" src=\"{src}\" alt=\"\" aria-hidden=\"true\" style=\"width: {size:.1}pt; height: {size:.1}pt; object-fit: contain; vertical-align: middle; margin-right: 0.25em;\">"
    );
}

fn valid_size(value: f64, maximum: f64) -> Option<f64> {
    (value.is_finite() && value > 0.0 && value <= maximum).then_some(value)
}

fn valid_text_size(value: f64) -> Option<f64> {
    (value.is_finite() && (MIN_TEXT_SIZE_PT..=MAX_TEXT_SIZE_PT).contains(&value)).then_some(value)
}

fn surrounding_text_size(paragraph: &TextParagraph, inherited: Option<&ParagraphDefaults>) -> f64 {
    paragraph
        .runs
        .iter()
        .find(|run| !run.is_break && !run.text.trim().is_empty())
        .and_then(|run| run.style.font_size)
        .or_else(|| {
            paragraph
                .def_rpr
                .as_ref()
                .and_then(|defaults| defaults.font_size)
        })
        .or_else(|| {
            inherited
                .and_then(|defaults| defaults.def_run_props.as_ref())
                .and_then(|defaults| defaults.font_size)
        })
        .unwrap_or(DEFAULT_FONT_SIZE_PT)
}

fn push_missing_diagnostic(picture: &PictureBullet, ctx: &RenderCtx<'_>) {
    let mut collector = ctx.collector.borrow_mut();
    let slide_index = collector.current_slide_index;
    collector.diagnostics.push(ConversionDiagnostic {
        code: "PICTURE_BULLET_IMAGE_MISSING".to_owned(),
        family: FeatureFamily::Images,
        support_tier: SupportTier::Fallback,
        stage: Some(CapabilityStage::Rendered),
        location: DiagnosticLocation {
            slide_index: Some(slide_index),
            part_name: Some(format!("ppt/slides/slide{}.xml", slide_index + 1)),
            relationship_id: (!picture.relationship_id.is_empty())
                .then(|| picture.relationship_id.clone()),
            relationship_type: picture.relationship_type.clone(),
            qualified_element_name: Some("a:buBlip".to_owned()),
            ..Default::default()
        },
        raw_reference: (!picture.relationship_id.is_empty())
            .then(|| picture.relationship_id.clone()),
        fallback_kind: FallbackKind::IgnoredRelationship,
        reason: format!(
            "Picture bullet image unavailable: {}",
            picture
                .failure
                .map(|failure| failure.as_str())
                .unwrap_or("unknown image failure")
        ),
    });
}
