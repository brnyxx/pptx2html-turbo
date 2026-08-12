use super::color::Color;

pub use super::effects::{GlowEffect, OuterShadow, ShapeEffects};
pub use super::fill::{
    Fill, GradientFill, GradientStop, GradientType, ImageFill, PatternFill, SolidFill,
};
pub use super::text::{
    Alignment, FontStyle, StrikethroughType, TextCapitalization, TextShadow, TextStyle,
    UnderlineType,
};

/// Border
#[derive(Debug, Clone, Default)]
pub struct Border {
    pub width: f64, // in pt
    pub color: Color,
    pub style: BorderStyle,
    pub dash_style: DashStyle,
    pub cap: LineCap,
    pub compound: CompoundLine,
    pub alignment: LineAlignment,
    pub join: LineJoin,
    pub miter_limit: Option<f64>,
    pub head_end: Option<LineEnd>,
    pub tail_end: Option<LineEnd>,
    /// Explicit `<a:noFill/>` inside `<a:ln>` — suppress border, do NOT
    /// inherit from theme lnRef (analogous to `Fill::NoFill`).
    pub no_fill: bool,
}

#[derive(Debug, Clone, Default)]
pub enum BorderStyle {
    #[default]
    None,
    Solid,
    Dashed,
    Dotted,
}

/// Dash style for SVG stroke-dasharray rendering
#[derive(Debug, Clone, Default)]
pub enum DashStyle {
    #[default]
    Solid,
    Dash,
    Dot,
    DashDot,
    LongDash,
    LongDashDot,
    LongDashDotDot,
    SystemDash,
    SystemDot,
    SystemDashDot,
    SystemDashDotDot,
}

/// Line cap style (ECMA-376 ST_LineCap)
#[derive(Debug, Clone, Default)]
pub enum LineCap {
    #[default]
    Flat,
    Square,
    Round,
}

#[derive(Debug, Clone, Default)]
pub enum CompoundLine {
    #[default]
    Single,
    Double,
    ThickThin,
    ThinThick,
    Triple,
}

#[derive(Debug, Clone, Default)]
pub enum LineAlignment {
    #[default]
    Center,
    Inset,
}

/// Line join style (ECMA-376 ST_LineJoinType)
#[derive(Debug, Clone, Default)]
pub enum LineJoin {
    #[default]
    Miter,
    Bevel,
    Round,
}

/// Line ending (arrowhead) for connectors/lines
#[derive(Debug, Clone)]
pub struct LineEnd {
    pub end_type: LineEndType,
    pub width: LineEndSize,
    pub length: LineEndSize,
}

/// Line ending arrowhead type (ECMA-376 ST_LineEndType)
#[derive(Debug, Clone, Default)]
pub enum LineEndType {
    #[default]
    None,
    Arrow,
    Triangle,
    Stealth,
    Diamond,
    Oval,
}

/// Line ending size (ECMA-376 ST_LineEndWidth / ST_LineEndLength)
#[derive(Debug, Clone, Default)]
pub enum LineEndSize {
    Small,
    #[default]
    Medium,
    Large,
}

impl LineEndSize {
    /// Multiplier relative to stroke width for SVG markers
    /// (markerUnits="userSpaceOnUse"). OOXML w/len sm/med/lg map to
    /// proportional multiples of the line width so that thin lines get
    /// small markers and thick lines get proportionally larger ones.
    pub fn multiplier(&self) -> f64 {
        match self {
            Self::Small => 2.0,
            Self::Medium => 3.0,
            Self::Large => 4.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Alignment, Fill, GradientFill, GradientStop, GradientType, LineEndSize, StrikethroughType,
        TextCapitalization, UnderlineType,
    };
    use crate::model::Color;

    #[test]
    fn underline_type_parses_ooxml_values_and_generates_css() {
        let cases = [
            ("sng", UnderlineType::Single, "text-decoration: underline"),
            (
                "dbl",
                UnderlineType::Double,
                "text-decoration: underline; text-decoration-style: double",
            ),
            (
                "heavy",
                UnderlineType::Heavy,
                "text-decoration: underline; text-decoration-thickness: 2px",
            ),
            (
                "dotted",
                UnderlineType::Dotted,
                "text-decoration: underline; text-decoration-style: dotted",
            ),
            (
                "dottedHeavy",
                UnderlineType::DottedHeavy,
                "text-decoration: underline; text-decoration-style: dotted",
            ),
            (
                "dash",
                UnderlineType::Dashed,
                "text-decoration: underline; text-decoration-style: dashed",
            ),
            (
                "dashHeavy",
                UnderlineType::DashHeavy,
                "text-decoration: underline; text-decoration-style: dashed",
            ),
            (
                "dashLong",
                UnderlineType::DashLong,
                "text-decoration: underline; text-decoration-style: dashed",
            ),
            (
                "dashLongHeavy",
                UnderlineType::DashLongHeavy,
                "text-decoration: underline; text-decoration-style: dashed",
            ),
            (
                "dotDash",
                UnderlineType::DotDash,
                "text-decoration: underline; text-decoration-style: dashed",
            ),
            (
                "dotDashHeavy",
                UnderlineType::DotDashHeavy,
                "text-decoration: underline; text-decoration-style: dashed",
            ),
            (
                "dotDotDash",
                UnderlineType::DotDotDash,
                "text-decoration: underline; text-decoration-style: dashed",
            ),
            (
                "dotDotDashHeavy",
                UnderlineType::DotDotDashHeavy,
                "text-decoration: underline; text-decoration-style: dashed",
            ),
            (
                "wavy",
                UnderlineType::Wavy,
                "text-decoration: underline; text-decoration-style: wavy",
            ),
            (
                "wavyHeavy",
                UnderlineType::WavyHeavy,
                "text-decoration: underline; text-decoration-style: wavy",
            ),
            (
                "wavyDbl",
                UnderlineType::WavyDouble,
                "text-decoration: underline; text-decoration-style: wavy",
            ),
        ];

        for (input, expected, expected_css) in cases {
            let parsed = UnderlineType::from_ooxml(input);
            assert_eq!(format!("{parsed:?}"), format!("{expected:?}"));
            assert_eq!(parsed.to_css().as_deref(), Some(expected_css));
        }

        let none = UnderlineType::from_ooxml("unknown");
        assert_eq!(format!("{none:?}"), format!("{:?}", UnderlineType::None));
        assert_eq!(none.to_css(), None);
    }

    #[test]
    fn text_capitalization_and_strikethrough_map_to_css() {
        assert_eq!(
            TextCapitalization::from_ooxml("all").to_css(),
            Some("text-transform: uppercase")
        );
        assert_eq!(
            TextCapitalization::from_ooxml("small").to_css(),
            Some("font-variant: small-caps")
        );
        assert_eq!(TextCapitalization::from_ooxml("other").to_css(), None);

        assert_eq!(
            StrikethroughType::from_ooxml("sngStrike").to_css(),
            Some("text-decoration: line-through")
        );
        assert_eq!(
            StrikethroughType::from_ooxml("dblStrike").to_css(),
            Some("text-decoration: line-through; text-decoration-style: double")
        );
        assert_eq!(StrikethroughType::from_ooxml("none").to_css(), None);
    }

    #[test]
    fn alignment_fill_gradient_type_and_line_end_size_helpers_are_stable() {
        assert_eq!(Alignment::from_ooxml("ctr").to_css(), "center");
        assert_eq!(Alignment::from_ooxml("r").to_css(), "right");
        assert_eq!(Alignment::from_ooxml("just").to_css(), "justify");
        assert_eq!(Alignment::from_ooxml("other").to_css(), "left");

        let solid = Fill::Solid(super::SolidFill {
            color: Color::rgb("112233"),
        });
        assert_eq!(solid.color_ref().to_css().as_deref(), Some("#112233"));

        let gradient = Fill::Gradient(GradientFill {
            gradient_type: GradientType::Linear,
            stops: vec![GradientStop {
                position: 0.0,
                color: Color::rgb("445566"),
            }],
            angle: 45.0,
        });
        assert_eq!(gradient.color_ref().to_css().as_deref(), Some("#445566"));
        assert_eq!(Fill::default().color_ref().to_css(), None);

        assert_eq!(GradientType::from_path_attr("circle"), GradientType::Radial);
        assert_eq!(
            GradientType::from_path_attr("rect"),
            GradientType::Rectangular
        );
        assert_eq!(GradientType::from_path_attr("shape"), GradientType::Shape);
        assert_eq!(GradientType::from_path_attr("other"), GradientType::Radial);

        assert_eq!(LineEndSize::Small.multiplier(), 2.0);
        assert_eq!(LineEndSize::Medium.multiplier(), 3.0);
        assert_eq!(LineEndSize::Large.multiplier(), 4.5);
    }
}
