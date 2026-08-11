use quick_xml::events::BytesStart;

use super::fill_parser::parse_color;
use super::xml_utils;
use crate::model::{
    Border, Color, ColorModifier, Emu, Fill, SolidFill, StyleRef, TableCellStyle, TableTextStyle,
};

pub(super) fn handle_start(
    local: &str,
    element: &BytesStart<'_>,
    style: &mut TableCellStyle,
    border_side: &mut Option<String>,
    in_fill: &mut bool,
    in_fill_ref: &mut bool,
    in_text_style: &mut bool,
) {
    if parse_color(local, element).is_some() {
        handle_empty(
            local,
            element,
            style,
            border_side.as_deref(),
            *in_fill,
            *in_fill_ref,
            *in_text_style,
        );
        return;
    }
    match local {
        "tcTxStyle" => {
            *in_text_style = true;
            style.text = TableTextStyle {
                bold: parse_on_off(element, "b"),
                italic: parse_on_off(element, "i"),
                ..Default::default()
            };
        }
        "fill" => *in_fill = true,
        "fillRef" => {
            *in_fill_ref = true;
            style.fill_ref = Some(StyleRef {
                idx: xml_utils::attr_str(element, "idx")
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(0),
                color: Color::none(),
            });
        }
        "left" | "right" | "top" | "bottom" | "insideH" | "insideV" => {
            *border_side = Some(local.to_owned());
            set_border(
                style,
                local,
                Border {
                    no_fill: true,
                    ..Default::default()
                },
            );
        }
        "ln" => {
            if let Some(side) = border_side.as_deref() {
                let border = Border {
                    width: xml_utils::attr_str(element, "w")
                        .map(|value| Emu::parse_emu(&value).to_pt())
                        .unwrap_or(0.0),
                    no_fill: false,
                    ..Default::default()
                };
                set_border(style, side, border);
            }
        }
        "fontRef" if *in_text_style => set_font_family(style, element),
        _ => {}
    }
}

pub(super) fn handle_empty(
    local: &str,
    element: &BytesStart<'_>,
    style: &mut TableCellStyle,
    border_side: Option<&str>,
    in_fill: bool,
    in_fill_ref: bool,
    in_text_style: bool,
) {
    if local == "fontRef" && in_text_style {
        set_font_family(style, element);
        return;
    }
    let modifier_value = xml_utils::attr_str(element, "val").and_then(|value| value.parse().ok());
    if let Some(modifier) = ColorModifier::from_ooxml(local, modifier_value) {
        if let Some(color) =
            active_color_mut(style, border_side, in_fill, in_fill_ref, in_text_style)
        {
            color.modifiers.push(modifier);
        }
        return;
    }
    if local == "noFill" {
        if let Some(side) = border_side {
            set_border(
                style,
                side,
                Border {
                    no_fill: true,
                    ..Default::default()
                },
            );
        } else if in_fill {
            style.fill = Some(Fill::NoFill);
        }
        return;
    }
    if local == "fillRef" {
        style.fill_ref = Some(StyleRef {
            idx: xml_utils::attr_str(element, "idx")
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
            color: Color::none(),
        });
        return;
    }
    let Some(color) = parse_color(local, element) else {
        return;
    };
    if let Some(side) = border_side {
        if let Some(border) = border_mut(style, side) {
            border.color = color;
            border.no_fill = false;
        }
    } else if in_fill_ref {
        if let Some(fill_ref) = style.fill_ref.as_mut() {
            fill_ref.color = color;
        }
    } else if in_fill {
        style.fill = Some(Fill::Solid(SolidFill { color }));
    } else if in_text_style {
        style.text.color = Some(color);
    }
}

fn active_color_mut<'a>(
    style: &'a mut TableCellStyle,
    border_side: Option<&str>,
    in_fill: bool,
    in_fill_ref: bool,
    in_text_style: bool,
) -> Option<&'a mut Color> {
    if let Some(side) = border_side {
        return border_mut(style, side).map(|border| &mut border.color);
    }
    if in_fill_ref {
        return style.fill_ref.as_mut().map(|fill_ref| &mut fill_ref.color);
    }
    if in_fill {
        return match style.fill.as_mut() {
            Some(Fill::Solid(fill)) => Some(&mut fill.color),
            _ => None,
        };
    }
    if in_text_style {
        return style.text.color.as_mut();
    }
    None
}

fn parse_on_off(element: &BytesStart<'_>, name: &str) -> Option<bool> {
    xml_utils::attr_str(element, name).and_then(|value| match value.as_str() {
        "1" | "true" | "on" => Some(true),
        "0" | "false" | "off" => Some(false),
        _ => None,
    })
}

fn set_font_family(style: &mut TableCellStyle, element: &BytesStart<'_>) {
    style.text.font_family = xml_utils::attr_str(element, "idx").map(|idx| {
        if idx == "major" {
            "+mj-lt".to_owned()
        } else {
            "+mn-lt".to_owned()
        }
    });
}

fn set_border(style: &mut TableCellStyle, side: &str, border: Border) {
    match side {
        "left" => style.left = Some(border),
        "right" => style.right = Some(border),
        "top" => style.top = Some(border),
        "bottom" => style.bottom = Some(border),
        "insideH" => style.inside_horizontal = Some(border),
        "insideV" => style.inside_vertical = Some(border),
        _ => {}
    }
}

fn border_mut<'a>(style: &'a mut TableCellStyle, side: &str) -> Option<&'a mut Border> {
    match side {
        "left" => style.left.as_mut(),
        "right" => style.right.as_mut(),
        "top" => style.top.as_mut(),
        "bottom" => style.bottom.as_mut(),
        "insideH" => style.inside_horizontal.as_mut(),
        "insideV" => style.inside_vertical.as_mut(),
        _ => None,
    }
}
