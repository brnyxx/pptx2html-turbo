/// EMU (English Metric Unit) — base length unit in OOXML
/// 1 inch = 914400 EMU, 1 pt = 12700 EMU, 1 cm = 360000 EMU
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Emu(pub i64);

impl Emu {
    pub fn to_px(self) -> f64 {
        // At 96 DPI: 1 inch = 914400 EMU = 96 px
        self.0 as f64 / 914400.0 * 96.0
    }

    pub fn to_pt(self) -> f64 {
        self.0 as f64 / 12700.0
    }

    pub fn to_cm(self) -> f64 {
        self.0 as f64 / 360000.0
    }

    /// Parse an EMU value from string, returning zero for invalid input.
    pub fn parse_emu(s: &str) -> Self {
        Self(s.parse::<i64>().unwrap_or(0))
    }
}

/// 2D position (relative to top-left origin)
#[derive(Debug, Clone, Copy, Default)]
pub struct Position {
    pub x: Emu,
    pub y: Emu,
}

/// 2D size
#[derive(Debug, Clone, Copy, Default)]
pub struct Size {
    pub width: Emu,
    pub height: Emu,
}

/// Custom geometry parsed from `<a:custGeom>`
#[derive(Debug, Clone)]
pub struct CustomGeometry {
    pub paths: Vec<GeometryPath>,
    pub text_rect: Option<GeomRect>,
    pub adjust_handles: Vec<AdjustHandle>,
    pub connection_sites: Vec<ConnectionSite>,
    pub guides: Vec<CustomGuide>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CustomGuide {
    pub name: String,
    pub raw_formula: String,
    pub evaluation: Result<f64, GuideFormulaError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuideFormulaError {
    Empty,
    UnknownOperator(String),
    InvalidArity {
        operator: String,
        expected: usize,
        actual: usize,
    },
    UnresolvedToken(String),
    NonFiniteToken(String),
    NonFiniteResult,
    DivisionByZero,
}

#[derive(Debug, Clone)]
pub struct GeomRect {
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

#[derive(Debug, Clone)]
pub enum AdjustHandle {
    XY(XYAdjustHandle),
    Polar(PolarAdjustHandle),
}

#[derive(Debug, Clone)]
pub struct XYAdjustHandle {
    pub gd_ref_x: Option<String>,
    pub gd_ref_y: Option<String>,
    pub min_x: Option<f64>,
    pub max_x: Option<f64>,
    pub min_y: Option<f64>,
    pub max_y: Option<f64>,
    pub pos_x: f64,
    pub pos_y: f64,
}

#[derive(Debug, Clone)]
pub struct PolarAdjustHandle {
    pub gd_ref_r: Option<String>,
    pub gd_ref_ang: Option<String>,
    pub min_r: Option<f64>,
    pub max_r: Option<f64>,
    pub min_ang: Option<f64>,
    pub max_ang: Option<f64>,
    pub pos_x: f64,
    pub pos_y: f64,
}

#[derive(Debug, Clone)]
pub struct ConnectionSite {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
}

/// A single path inside `<a:pathLst>`
#[derive(Debug, Clone)]
pub struct GeometryPath {
    pub width: f64,
    pub height: f64,
    pub commands: Vec<PathCommand>,
    pub fill: PathFill,
}

/// DrawingML path command
#[derive(Debug, Clone)]
pub enum PathCommand {
    MoveTo {
        x: f64,
        y: f64,
    },
    LineTo {
        x: f64,
        y: f64,
    },
    CubicBezTo {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        x: f64,
        y: f64,
    },
    QuadBezTo {
        x1: f64,
        y1: f64,
        x: f64,
        y: f64,
    },
    ArcTo {
        wr: f64,
        hr: f64,
        start_angle: f64,
        swing_angle: f64,
    },
    Close,
}

/// Path fill mode
#[derive(Debug, Clone, Default)]
pub enum PathFill {
    #[default]
    Norm,
    None,
    Lighten,
    Darken,
    LightenLess,
    DarkenLess,
}

#[cfg(test)]
mod tests {
    use super::{Emu, PathFill};

    #[test]
    fn emu_conversions_use_expected_ooxml_ratios() {
        let inch = Emu(914_400);
        assert_eq!(inch.to_px(), 96.0);
        assert_eq!(inch.to_pt(), 72.0);

        let centimeter = Emu(360_000);
        assert_eq!(centimeter.to_cm(), 1.0);
    }

    #[test]
    fn parse_emu_defaults_invalid_values_to_zero() {
        assert_eq!(Emu::parse_emu("12700").0, 12_700);
        assert_eq!(Emu::parse_emu("not-a-number").0, 0);
    }

    #[test]
    fn path_fill_defaults_to_normal_fill_mode() {
        assert!(matches!(PathFill::default(), PathFill::Norm));
    }
}
