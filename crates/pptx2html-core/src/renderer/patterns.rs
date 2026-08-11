use std::fmt::Write;

use base64::Engine;

use crate::model::{Color, ColorKind, ColorModifier, PatternFill};

use super::pattern_tiles::tile_spec;

pub(super) fn css(fill: &PatternFill, foreground: &str, background: &str) -> Option<String> {
    let tile = tile_spec(&fill.preset, foreground)?;
    let svg = format!(
        "<svg xmlns='http://www.w3.org/2000/svg' width='{0}' height='{1}' viewBox='0 0 {0} {1}'><rect width='100%' height='100%' fill='{background}'/>{2}</svg>",
        tile.width, tile.height, tile.motif
    );
    let encoded = base64::engine::general_purpose::STANDARD.encode(svg.as_bytes());
    Some(format!(
        "background-image: url(data:image/svg+xml;base64,{encoded}); background-repeat: repeat; background-size: {}px {}px",
        tile.width, tile.height
    ))
}

pub(super) fn svg_def(
    fill: &PatternFill,
    id: &str,
    foreground: &str,
    background: &str,
    output: &mut String,
) -> Option<String> {
    let tile = tile_spec(&fill.preset, foreground)?;
    let _ = write!(
        output,
        "<pattern id=\"{id}\" patternUnits=\"userSpaceOnUse\" width=\"{}\" height=\"{}\"><rect width=\"100%\" height=\"100%\" fill=\"{background}\"/>{}</pattern>",
        tile.width, tile.height, tile.motif
    );
    Some(format!("url(#{id})"))
}

pub(super) fn raw_semantics(fill: &PatternFill) -> String {
    let mut json = String::from("{\"preset\":");
    super::fallback::write_json_string(&mut json, fill.preset.as_ooxml());
    json.push_str(",\"foreground\":");
    write_color(&mut json, fill.foreground.as_ref());
    json.push_str(",\"background\":");
    write_color(&mut json, fill.background.as_ref());
    json.push('}');
    json
}

fn write_color(json: &mut String, color: Option<&Color>) {
    let Some(color) = color else {
        json.push_str("null");
        return;
    };
    json.push_str("{\"kind\":");
    let (kind, value) = match &color.kind {
        ColorKind::None => ("none", None),
        ColorKind::Rgb(value) => ("srgbClr", Some(value.as_str())),
        ColorKind::Theme(value) => ("schemeClr", Some(value.as_str())),
        ColorKind::System(value) => ("sysClr", Some(value.as_str())),
        ColorKind::Preset(value) => ("prstClr", Some(value.as_str())),
    };
    super::fallback::write_json_string(json, kind);
    json.push_str(",\"value\":");
    if let Some(value) = value {
        super::fallback::write_json_string(json, value);
    } else {
        json.push_str("null");
    }
    json.push_str(",\"modifiers\":[");
    for (index, modifier) in color.modifiers.iter().enumerate() {
        if index > 0 {
            json.push(',');
        }
        write_modifier(json, modifier);
    }
    json.push_str("]}");
}

fn write_modifier(json: &mut String, modifier: &ColorModifier) {
    let (kind, value) = match modifier {
        ColorModifier::Tint(value) => ("tint", Some(*value)),
        ColorModifier::Shade(value) => ("shade", Some(*value)),
        ColorModifier::Alpha(value) => ("alpha", Some(*value)),
        ColorModifier::AlphaOff(value) => ("alphaOff", Some(*value)),
        ColorModifier::AlphaMod(value) => ("alphaMod", Some(*value)),
        ColorModifier::LumMod(value) => ("lumMod", Some(*value)),
        ColorModifier::LumOff(value) => ("lumOff", Some(*value)),
        ColorModifier::SatMod(value) => ("satMod", Some(*value)),
        ColorModifier::SatOff(value) => ("satOff", Some(*value)),
        ColorModifier::HueMod(value) => ("hueMod", Some(*value)),
        ColorModifier::HueOff(value) => ("hueOff", Some(*value)),
        ColorModifier::Comp => ("comp", None),
        ColorModifier::Inv => ("inv", None),
        ColorModifier::Gray => ("gray", None),
    };
    json.push_str("{\"type\":");
    super::fallback::write_json_string(json, kind);
    json.push_str(",\"value\":");
    if let Some(value) = value {
        let _ = write!(json, "{value}");
    } else {
        json.push_str("null");
    }
    json.push('}');
}
