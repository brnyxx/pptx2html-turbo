use super::action::ActionSet;
use super::bullet::Bullet;
use super::color::Color;
use super::hierarchy::{ListStyle, SpacingValue};

/// Underline type (ECMA-376 ST_TextUnderlineType)
#[derive(Debug, Clone, Default, PartialEq)]
pub enum UnderlineType {
    #[default]
    None,
    Single,
    Double,
    Heavy,
    Dotted,
    DottedHeavy,
    Dashed,
    DashHeavy,
    DashLong,
    DashLongHeavy,
    DotDash,
    DotDashHeavy,
    DotDotDash,
    DotDotDashHeavy,
    Wavy,
    WavyHeavy,
    WavyDouble,
}

impl UnderlineType {
    /// Parse OOXML `u` attribute value
    pub fn from_ooxml(val: &str) -> Self {
        match val {
            "sng" => Self::Single,
            "dbl" => Self::Double,
            "heavy" => Self::Heavy,
            "dotted" => Self::Dotted,
            "dottedHeavy" => Self::DottedHeavy,
            "dash" => Self::Dashed,
            "dashHeavy" => Self::DashHeavy,
            "dashLong" => Self::DashLong,
            "dashLongHeavy" => Self::DashLongHeavy,
            "dotDash" => Self::DotDash,
            "dotDashHeavy" => Self::DotDashHeavy,
            "dotDotDash" => Self::DotDotDash,
            "dotDotDashHeavy" => Self::DotDotDashHeavy,
            "wavy" => Self::Wavy,
            "wavyHeavy" => Self::WavyHeavy,
            "wavyDbl" => Self::WavyDouble,
            _ => Self::None,
        }
    }

    /// Generate CSS properties for this underline type
    pub fn to_css(&self) -> Option<String> {
        match self {
            Self::None => Option::None,
            Self::Single => Some("text-decoration: underline".to_string()),
            Self::Double => {
                Some("text-decoration: underline; text-decoration-style: double".to_string())
            }
            Self::Heavy => {
                Some("text-decoration: underline; text-decoration-thickness: 2px".to_string())
            }
            Self::Dotted | Self::DottedHeavy => {
                Some("text-decoration: underline; text-decoration-style: dotted".to_string())
            }
            Self::Dashed | Self::DashHeavy | Self::DashLong | Self::DashLongHeavy => {
                Some("text-decoration: underline; text-decoration-style: dashed".to_string())
            }
            Self::Wavy | Self::WavyHeavy | Self::WavyDouble => {
                Some("text-decoration: underline; text-decoration-style: wavy".to_string())
            }
            Self::DotDash | Self::DotDashHeavy | Self::DotDotDash | Self::DotDotDashHeavy => {
                Some("text-decoration: underline; text-decoration-style: dashed".to_string())
            }
        }
    }
}

/// Strikethrough type (ECMA-376 ST_TextStrikeType)
#[derive(Debug, Clone, Default, PartialEq)]
pub enum StrikethroughType {
    #[default]
    None,
    Single,
    Double,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum TextCapitalization {
    #[default]
    None,
    All,
    Small,
}

impl TextCapitalization {
    pub fn from_ooxml(val: &str) -> Self {
        match val {
            "all" => Self::All,
            "small" => Self::Small,
            _ => Self::None,
        }
    }

    pub fn to_css(&self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::All => Some("text-transform: uppercase"),
            Self::Small => Some("font-variant: small-caps"),
        }
    }
}

impl StrikethroughType {
    /// Parse OOXML `strike` attribute value
    pub fn from_ooxml(val: &str) -> Self {
        match val {
            "sngStrike" => Self::Single,
            "dblStrike" => Self::Double,
            _ => Self::None,
        }
    }

    /// Generate CSS properties for this strikethrough type
    pub fn to_css(&self) -> Option<&'static str> {
        match self {
            Self::None => Option::None,
            Self::Single => Some("text-decoration: line-through"),
            Self::Double => Some("text-decoration: line-through; text-decoration-style: double"),
        }
    }
}

/// Text style
#[derive(Debug, Clone, Default)]
pub struct TextStyle {
    pub font_family: Option<String>,
    pub font_size: Option<f64>, // in pt
    pub bold: bool,
    pub italic: bool,
    pub underline: UnderlineType,
    pub strikethrough: StrikethroughType,
    pub capitalization: TextCapitalization,
    pub color: Color,
    pub baseline: Option<i32>, // superscript(+)/subscript(-) offset (1/1000 %)
    pub letter_spacing: Option<f64>, // in pt
    pub highlight: Option<Color>, // text highlight (background color)
    pub shadow: Option<TextShadow>, // text shadow from effectLst/outerShdw
}

/// Text shadow parameters
#[derive(Debug, Clone)]
pub struct TextShadow {
    pub color: Color,
    pub blur_rad: f64, // blur radius in pt
    pub dist: f64,     // distance in pt
    pub dir: f64,      // direction angle in degrees
}

/// Font style (run-level)
#[derive(Debug, Clone, Default)]
pub struct FontStyle {
    pub latin: Option<String>,
    pub east_asian: Option<String>,
    pub complex_script: Option<String>,
}

/// Text alignment
#[derive(Debug, Clone, Default)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

impl Alignment {
    pub fn from_ooxml(val: &str) -> Self {
        match val {
            "ctr" => Self::Center,
            "r" => Self::Right,
            "just" => Self::Justify,
            _ => Self::Left,
        }
    }

    pub fn to_css(&self) -> &str {
        match self {
            Self::Left => "left",
            Self::Center => "center",
            Self::Right => "right",
            Self::Justify => "justify",
        }
    }
}

/// Text body
#[derive(Debug, Clone)]
pub struct TextBody {
    pub paragraphs: Vec<TextParagraph>,
    pub list_style: Option<ListStyle>,
    pub vertical_align: VerticalAlign,
    pub vertical_align_explicit: bool,
    pub anchor_center: bool,
    pub text_rotation_deg: f64,
    pub margin_top_explicit: bool,
    pub margin_bottom_explicit: bool,
    pub margin_left_explicit: bool,
    pub margin_right_explicit: bool,
    pub word_wrap: bool,
    pub word_wrap_explicit: bool,
    pub auto_fit: AutoFit,
    pub margins: TextMargins,
}

impl Default for TextBody {
    fn default() -> Self {
        Self {
            paragraphs: Vec::new(),
            list_style: None,
            vertical_align: VerticalAlign::Top,
            vertical_align_explicit: false,
            anchor_center: false,
            text_rotation_deg: 0.0,
            margin_top_explicit: false,
            margin_bottom_explicit: false,
            margin_left_explicit: false,
            margin_right_explicit: false,
            word_wrap: true,
            word_wrap_explicit: false,
            auto_fit: AutoFit::None,
            margins: TextMargins::default(),
        }
    }
}

/// Text paragraph
#[derive(Debug, Clone, Default)]
pub struct TextParagraph {
    pub runs: Vec<TextRun>,
    pub alignment: Alignment,
    pub rtl: bool,
    pub line_spacing: Option<SpacingValue>,
    pub space_before: Option<SpacingValue>,
    pub space_after: Option<SpacingValue>,
    pub indent: Option<f64>,
    pub margin_left: Option<f64>,
    pub bullet: Option<Bullet>,
    pub level: u32,
    /// Paragraph-level default run properties (from <a:defRPr> inside <a:pPr>)
    pub def_rpr: Option<ParagraphDefRPr>,
}

/// Paragraph-level default run properties parsed from <a:defRPr> inside <a:pPr>
#[derive(Debug, Clone, Default)]
pub struct ParagraphDefRPr {
    pub font_size: Option<f64>,
    pub letter_spacing: Option<f64>,
    pub baseline: Option<i32>,
    pub capitalization: Option<TextCapitalization>,
    pub underline: Option<UnderlineType>,
    pub strikethrough: Option<StrikethroughType>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub color: Option<Color>,
    pub font_latin: Option<String>,
    pub font_ea: Option<String>,
    pub font_cs: Option<String>,
}

/// Text run (text segment with uniform style)
#[derive(Debug, Clone, Default)]
pub struct TextRun {
    pub text: String,
    pub style: TextStyle,
    pub font: FontStyle,
    pub hyperlink: Option<String>,
    pub actions: ActionSet,
    pub is_break: bool, // <a:br> line break
}

/// Vertical alignment
#[derive(Debug, Clone, Default)]
pub enum VerticalAlign {
    #[default]
    Top,
    Middle,
    Bottom,
}

impl VerticalAlign {
    pub fn from_ooxml(val: &str) -> Self {
        match val {
            "ctr" => Self::Middle,
            "b" => Self::Bottom,
            _ => Self::Top,
        }
    }
}

/// Text auto-fit
#[derive(Debug, Clone, Default)]
pub enum AutoFit {
    #[default]
    None,
    NoAutoFit,
    Normal {
        font_scale: Option<f64>,             // 0.0-1.0 (e.g., 0.625 for 62.5%)
        line_spacing_reduction: Option<f64>, // 0.0-1.0 (e.g., 0.2 for 20%)
    },
    Shrink,
}

/// Text internal margins
#[derive(Debug, Clone)]
pub struct TextMargins {
    pub top: f64,
    pub bottom: f64,
    pub left: f64,
    pub right: f64,
}

impl Default for TextMargins {
    fn default() -> Self {
        Self {
            top: 3.6, // OOXML default 45720 EMU ~ 3.6pt
            bottom: 3.6,
            left: 7.2, // 91440 EMU ~ 7.2pt
            right: 7.2,
        }
    }
}
