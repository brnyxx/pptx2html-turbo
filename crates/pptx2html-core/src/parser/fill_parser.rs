use quick_xml::events::BytesStart;

use super::xml_utils;
use crate::model::*;

pub(crate) fn assign_style_ref_color(
    ref_kind: &str,
    idx: &str,
    color: Color,
    builder: &mut Option<ShapeStyleRef>,
) {
    let Some(builder) = builder.as_mut() else {
        return;
    };
    match ref_kind {
        "fillRef" => builder.fill_ref = Some(style_ref(idx, color)),
        "lnRef" => builder.ln_ref = Some(style_ref(idx, color)),
        "effectRef" => builder.effect_ref = Some(style_ref(idx, color)),
        "fontRef" => {
            builder.font_ref = Some(FontRef {
                idx: idx.to_owned(),
                color,
            });
        }
        _ => {}
    }
}

pub(crate) fn ensure_style_ref(ref_kind: &str, idx: &str, builder: &mut Option<ShapeStyleRef>) {
    let Some(builder) = builder.as_mut() else {
        return;
    };
    match ref_kind {
        "fillRef" if builder.fill_ref.is_none() => {
            builder.fill_ref = Some(style_ref(idx, Color::none()));
        }
        "lnRef" if builder.ln_ref.is_none() => {
            builder.ln_ref = Some(style_ref(idx, Color::none()));
        }
        "effectRef" if builder.effect_ref.is_none() => {
            builder.effect_ref = Some(style_ref(idx, Color::none()));
        }
        "fontRef" if builder.font_ref.is_none() => {
            builder.font_ref = Some(FontRef {
                idx: idx.to_owned(),
                color: Color::none(),
            });
        }
        _ => {}
    }
}

pub(crate) fn assign_style_ref_no_color(
    ref_kind: &str,
    idx: &str,
    builder: &mut Option<ShapeStyleRef>,
) {
    let Some(builder) = builder.as_mut() else {
        return;
    };
    match ref_kind {
        "fillRef" => builder.fill_ref = Some(style_ref(idx, Color::none())),
        "lnRef" => builder.ln_ref = Some(style_ref(idx, Color::none())),
        "effectRef" => builder.effect_ref = Some(style_ref(idx, Color::none())),
        "fontRef" => {
            builder.font_ref = Some(FontRef {
                idx: idx.to_owned(),
                color: Color::none(),
            });
        }
        _ => {}
    }
}

fn style_ref(idx: &str, color: Color) -> StyleRef {
    StyleRef {
        idx: idx.parse::<u32>().unwrap_or(0),
        color,
    }
}

pub(crate) fn parse_line_end(element: &BytesStart<'_>) -> Option<LineEnd> {
    let end_type = match xml_utils::attr_str(element, "type").as_deref() {
        Some("arrow") => LineEndType::Arrow,
        Some("triangle") => LineEndType::Triangle,
        Some("stealth") => LineEndType::Stealth,
        Some("diamond") => LineEndType::Diamond,
        Some("oval") => LineEndType::Oval,
        Some("none") | None | Some(_) => return None,
    };
    let width = parse_line_end_size(xml_utils::attr_str(element, "w").as_deref());
    let length = parse_line_end_size(xml_utils::attr_str(element, "len").as_deref());
    Some(LineEnd {
        end_type,
        width,
        length,
    })
}

fn parse_line_end_size(value: Option<&str>) -> LineEndSize {
    match value {
        Some("sm") => LineEndSize::Small,
        Some("lg") => LineEndSize::Large,
        _ => LineEndSize::Medium,
    }
}
