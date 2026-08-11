use quick_xml::events::BytesStart;

use super::slide_parser::ShapeBuilder;
use super::table_parser::TableSaxState;
use super::xml_utils;
use crate::model::{Color, ColorModifier, Fill, PatternFill, PatternPreset, Slide};

#[derive(Clone, Copy)]
enum PatternTarget {
    Shape,
    TableCell,
    Background,
}

#[derive(Clone, Copy)]
enum ColorRole {
    Foreground,
    Background,
}

#[derive(Default)]
pub(crate) struct PatternSaxState {
    target: Option<PatternTarget>,
    builder: PatternBuilder,
}

#[derive(Default)]
pub(crate) struct PatternBuilder {
    preset: Option<PatternPreset>,
    color_role: Option<ColorRole>,
    foreground: Option<Color>,
    background: Option<Color>,
}

impl PatternBuilder {
    pub(crate) fn begin(&mut self, element: &BytesStart<'_>) {
        self.preset = Some(PatternPreset::from_ooxml(
            pattern_preset(element).as_deref().unwrap_or(""),
        ));
        self.color_role = None;
        self.foreground = None;
        self.background = None;
    }

    pub(crate) fn start_color_role(&mut self, local: &str) -> bool {
        self.color_role = match local {
            "fgClr" => Some(ColorRole::Foreground),
            "bgClr" => Some(ColorRole::Background),
            _ => return false,
        };
        true
    }

    pub(crate) fn finish_color_role(&mut self) {
        self.color_role = None;
    }

    pub(crate) fn assign_color(&mut self, color: Color) -> bool {
        match self.color_role {
            Some(ColorRole::Foreground) => self.foreground = Some(color),
            Some(ColorRole::Background) => self.background = Some(color),
            None => return false,
        }
        true
    }

    pub(crate) fn parse_color(&mut self, local: &str, element: &BytesStart<'_>) -> bool {
        let color = match local {
            "srgbClr" => xml_utils::attr_str(element, "val").map(Color::rgb),
            "schemeClr" => xml_utils::attr_str(element, "val").map(Color::theme),
            "prstClr" => xml_utils::attr_str(element, "val").map(Color::preset),
            "sysClr" => xml_utils::attr_str(element, "val")
                .map(Color::system)
                .or_else(|| xml_utils::attr_str(element, "lastClr").map(Color::rgb)),
            _ => None,
        };
        color.is_some_and(|color| self.assign_color(color))
    }

    pub(crate) fn append_modifier(&mut self, local: &str, element: &BytesStart<'_>) -> bool {
        let value = xml_utils::attr_str(element, "val").and_then(|value| value.parse().ok());
        let Some(modifier) = ColorModifier::from_ooxml(local, value) else {
            return false;
        };
        let color = match self.color_role {
            Some(ColorRole::Foreground) => self.foreground.as_mut(),
            Some(ColorRole::Background) => self.background.as_mut(),
            None => None,
        };
        if let Some(color) = color {
            color.modifiers.push(modifier);
            return true;
        }
        false
    }

    pub(crate) fn finish(&mut self) -> PatternFill {
        self.color_role = None;
        PatternFill {
            preset: self
                .preset
                .take()
                .unwrap_or_else(|| PatternPreset::Unknown(String::new())),
            foreground: self.foreground.take(),
            background: self.background.take(),
        }
    }
}

impl PatternSaxState {
    pub(crate) fn is_active(&self) -> bool {
        self.target.is_some()
    }

    pub(crate) fn start(
        &mut self,
        element: &BytesStart<'_>,
        in_shape_properties: bool,
        in_table_cell_properties: bool,
        in_background: bool,
    ) -> bool {
        let target = if in_background {
            PatternTarget::Background
        } else if in_table_cell_properties {
            PatternTarget::TableCell
        } else if in_shape_properties {
            PatternTarget::Shape
        } else {
            return false;
        };
        self.target = Some(target);
        self.builder.begin(element);
        true
    }

    pub(crate) fn start_color_role(&mut self, local: &str) -> bool {
        if !self.is_active() {
            return false;
        }
        self.builder.start_color_role(local)
    }

    pub(crate) fn finish_color_role(&mut self, local: &str) -> bool {
        if !self.is_active() || !matches!(local, "fgClr" | "bgClr") {
            return false;
        }
        self.builder.finish_color_role();
        true
    }

    pub(crate) fn assign_color(&mut self, color: Color) -> bool {
        self.builder.assign_color(color)
    }

    pub(crate) fn finish(
        &mut self,
        shape: &mut Option<ShapeBuilder>,
        table: &mut TableSaxState,
        slide: &mut Slide,
    ) -> bool {
        let Some(target) = self.target.take() else {
            return false;
        };
        let fill = Fill::Pattern(self.builder.finish());
        match target {
            PatternTarget::Shape => {
                if let Some(shape) = shape.as_mut() {
                    shape.fill = fill;
                }
            }
            PatternTarget::TableCell => {
                if let Some(cell) = table.cell.as_mut() {
                    cell.fill = fill;
                }
            }
            PatternTarget::Background => slide.background = Some(fill),
        }
        true
    }
}

fn pattern_preset(element: &BytesStart<'_>) -> Option<String> {
    element.attributes().flatten().find_map(|attribute| {
        (xml_utils::local_name(attribute.key.as_ref()) == "prst")
            .then(|| {
                attribute
                    .unescape_value()
                    .ok()
                    .map(|value| value.into_owned())
            })
            .flatten()
    })
}
