use super::color::Color;

/// Fill (shape/slide background)
#[derive(Debug, Clone, Default)]
pub enum Fill {
    /// No fill specified -- inheritance/theme fallback should apply
    #[default]
    None,
    /// Explicit `<a:noFill>` -- transparent, do NOT apply theme fallback
    NoFill,
    Solid(SolidFill),
    Gradient(GradientFill),
    Image(ImageFill),
}

/// Image fill data for backgrounds
#[derive(Debug, Clone, Default)]
pub struct ImageFill {
    pub rel_id: String,
    pub data: Vec<u8>,
    pub content_type: String,
}

impl Fill {
    /// Extract the primary color reference from this fill (for SVG rendering)
    pub fn color_ref(&self) -> Color {
        match self {
            Fill::Solid(sf) => sf.color.clone(),
            Fill::Gradient(gf) => gf
                .stops
                .first()
                .map(|s| s.color.clone())
                .unwrap_or_else(Color::none),
            _ => Color::none(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SolidFill {
    pub color: Color,
}

/// Gradient type (ECMA-376 §20.1.8.46 — a:path, a:lin)
#[derive(Debug, Clone, Default, PartialEq)]
pub enum GradientType {
    #[default]
    Linear, // <a:lin ang="...">
    Radial,      // <a:path path="circle">
    Rectangular, // <a:path path="rect">
    Shape,       // <a:path path="shape">
}

impl GradientType {
    /// Parse OOXML `<a:path path="...">` attribute value
    pub fn from_path_attr(val: &str) -> Self {
        match val {
            "circle" => Self::Radial,
            "rect" => Self::Rectangular,
            "shape" => Self::Shape,
            _ => Self::Radial, // default for unrecognized path types
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GradientFill {
    pub gradient_type: GradientType,
    pub stops: Vec<GradientStop>,
    pub angle: f64, // in degrees (used for Linear)
}

#[derive(Debug, Clone, Default)]
pub struct GradientStop {
    pub position: f64, // 0.0 ~ 1.0
    pub color: Color,
}
