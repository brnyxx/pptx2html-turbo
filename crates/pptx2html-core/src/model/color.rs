//! OOXML color system (ECMA-376 §20.1.2.3)
//!
//! Color resolution chain: ColorKind → ClrMap mapping → Theme ColorScheme lookup → apply modifiers → ResolvedColor

use crate::model::presentation::{ClrMap, ColorScheme};

/// Color variant
#[derive(Debug, Clone, Default, PartialEq)]
pub enum ColorKind {
    #[default]
    None,
    /// Direct RGB value (e.g. "FF0000")
    Rgb(String),
    /// Theme color reference (e.g. "accent1", "dk1")
    Theme(String),
    /// System color (e.g. "windowText")
    System(String),
    /// Preset color (e.g. "white", "black")
    Preset(String),
}

/// Color modifier (ECMA-376 §20.1.2.3.*)
#[derive(Debug, Clone, PartialEq)]
pub enum ColorModifier {
    /// White blend — preserves original color by val/100000 ratio, rest is white
    Tint(i32),
    /// Black blend — preserves original color by val/100000 ratio, rest is black
    Shade(i32),
    /// Opacity (0=fully transparent, 100000=opaque)
    Alpha(i32),
    /// Additive alpha offset (value in 1/1000 of percent, e.g., -25000 = -25%)
    AlphaOff(i32),
    /// Multiplicative alpha modifier (value in 1/1000 of percent, e.g., 50000 = 50%)
    AlphaMod(i32),
    /// Luminance multiply (HSL L channel)
    LumMod(i32),
    /// Luminance offset (HSL L channel)
    LumOff(i32),
    /// Saturation multiply (HSL S channel)
    SatMod(i32),
    /// Saturation offset (HSL S channel)
    SatOff(i32),
    /// Hue multiply (HSL H channel)
    HueMod(i32),
    /// Hue offset (HSL H channel)
    HueOff(i32),
    /// Complementary
    Comp,
    /// RGB inverse
    Inv,
    /// Grayscale conversion
    Gray,
}

/// Color + modifier combination
#[derive(Debug, Clone, Default)]
pub struct Color {
    pub kind: ColorKind,
    pub modifiers: Vec<ColorModifier>,
}

/// Resolved RGBA color
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

// ── Color construction helpers ──

impl Color {
    pub fn none() -> Self {
        Self {
            kind: ColorKind::None,
            modifiers: vec![],
        }
    }

    pub fn rgb(hex: impl Into<String>) -> Self {
        Self {
            kind: ColorKind::Rgb(hex.into()),
            modifiers: vec![],
        }
    }

    pub fn theme(name: impl Into<String>) -> Self {
        Self {
            kind: ColorKind::Theme(name.into()),
            modifiers: vec![],
        }
    }

    pub fn system(name: impl Into<String>) -> Self {
        Self {
            kind: ColorKind::System(name.into()),
            modifiers: vec![],
        }
    }

    pub fn preset(name: impl Into<String>) -> Self {
        Self {
            kind: ColorKind::Preset(name.into()),
            modifiers: vec![],
        }
    }

    pub fn is_none(&self) -> bool {
        matches!(self.kind, ColorKind::None)
    }

    pub fn with_modifier(mut self, m: ColorModifier) -> Self {
        self.modifiers.push(m);
        self
    }

    /// Resolve final RGBA color via theme/ClrMap lookup
    pub fn resolve(
        &self,
        scheme: Option<&ColorScheme>,
        clr_map: Option<&ClrMap>,
    ) -> Option<ResolvedColor> {
        let base = match &self.kind {
            ColorKind::None => return None,
            ColorKind::Rgb(hex) => parse_hex_rgb(hex)?,
            ColorKind::Theme(name) => {
                let mapped = clr_map
                    .and_then(|cm| cm.get(name))
                    .map(String::as_str)
                    .unwrap_or(name.as_str());
                // OOXML default aliases: tx1->dk1, tx2->dk2, bg1->lt1, bg2->lt2
                // These apply when ClrMap has no explicit mapping and ColorScheme
                // doesn't recognize the name directly (e.g. "tx1" is not a scheme slot).
                let scheme_key = scheme.and_then(|s| s.get(mapped)).or_else(|| {
                    let alias = ooxml_color_alias(mapped)?;
                    scheme.and_then(|s| s.get(alias))
                })?;
                parse_hex_rgb(&scheme_key)?
            }
            ColorKind::System(name) => parse_hex_rgb(system_color_fallback(name)?)?,
            ColorKind::Preset(name) => parse_hex_rgb(preset_color(name)?)?,
        };
        Some(apply_modifiers(base, &self.modifiers))
    }

    /// Fallback CSS without theme (backward compatible)
    pub fn to_css(&self) -> Option<String> {
        match &self.kind {
            ColorKind::None => None,
            ColorKind::Rgb(hex) => Some(format!("#{hex}")),
            ColorKind::Theme(name) => Some(format!("#{}", theme_fallback(name))),
            ColorKind::System(name) => Some(format!("#{}", system_color_fallback(name)?)),
            ColorKind::Preset(name) => Some(format!("#{}", preset_color(name)?)),
        }
    }
}

// ── ResolvedColor ──

impl ResolvedColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub fn with_alpha(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_css(&self) -> String {
        if self.a >= 254 {
            format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
        } else {
            format!(
                "rgba({}, {}, {}, {:.2})",
                self.r,
                self.g,
                self.b,
                self.a as f64 / 255.0
            )
        }
    }
}

// ── Apply color modifiers (ECMA-376 §20.1.2.3 order) ──

/// Returns the spec-defined sort priority for a modifier.
/// ECMA-376 Part 1, §20.1.2.3 application order:
///   1. Alpha/AlphaOff/AlphaMod
///   2. Hue/HueOff/HueMod
///   3. Sat/SatOff/SatMod
///   4. Lum/LumOff/LumMod
///   5. Tint/Shade
///   6. Complement/Inverse/Grayscale
fn modifier_order(m: &ColorModifier) -> u8 {
    match m {
        ColorModifier::Alpha(_) | ColorModifier::AlphaOff(_) | ColorModifier::AlphaMod(_) => 0,
        ColorModifier::HueMod(_) | ColorModifier::HueOff(_) => 1,
        ColorModifier::SatMod(_) | ColorModifier::SatOff(_) => 2,
        ColorModifier::LumMod(_) | ColorModifier::LumOff(_) => 3,
        ColorModifier::Tint(_) | ColorModifier::Shade(_) => 4,
        ColorModifier::Comp | ColorModifier::Inv | ColorModifier::Gray => 5,
    }
}

fn apply_modifiers(mut c: ResolvedColor, modifiers: &[ColorModifier]) -> ResolvedColor {
    // Sort modifiers into ECMA-376 spec order (stable sort preserves
    // relative order within the same priority group, e.g. LumMod before LumOff)
    let mut sorted: Vec<&ColorModifier> = modifiers.iter().collect();
    sorted.sort_by_key(|m| modifier_order(m));

    for m in sorted {
        match m {
            ColorModifier::Tint(val) => {
                // OOXML spec: tint adjusts HSL luminance toward white
                // result_L = L * tint + (1.0 - tint)
                let f = *val as f64 / 100_000.0;
                let (h, s, l) = rgb_to_hsl(c.r, c.g, c.b);
                let new_l = (l * f + (1.0 - f)).clamp(0.0, 1.0);
                let (r, g, b) = hsl_to_rgb(h, s, new_l);
                c.r = r;
                c.g = g;
                c.b = b;
            }
            ColorModifier::Shade(val) => {
                // OOXML spec: shade adjusts HSL luminance toward black
                // result_L = L * shade
                let f = *val as f64 / 100_000.0;
                let (h, s, l) = rgb_to_hsl(c.r, c.g, c.b);
                let new_l = (l * f).clamp(0.0, 1.0);
                let (r, g, b) = hsl_to_rgb(h, s, new_l);
                c.r = r;
                c.g = g;
                c.b = b;
            }
            ColorModifier::Alpha(val) => {
                c.a = clamp_u8(255.0 * *val as f64 / 100_000.0);
            }
            ColorModifier::AlphaOff(val) => {
                c.a = (c.a as i32 + (*val * 255 / 100_000)).clamp(0, 255) as u8;
            }
            ColorModifier::AlphaMod(val) => {
                c.a = (c.a as i32 * *val / 100_000).clamp(0, 255) as u8;
            }
            ColorModifier::LumMod(val) => {
                let (h, s, mut l) = rgb_to_hsl(c.r, c.g, c.b);
                l *= *val as f64 / 100_000.0;
                let (r, g, b) = hsl_to_rgb(h, s, l.clamp(0.0, 1.0));
                c.r = r;
                c.g = g;
                c.b = b;
            }
            ColorModifier::LumOff(val) => {
                let (h, s, mut l) = rgb_to_hsl(c.r, c.g, c.b);
                l += *val as f64 / 100_000.0;
                let (r, g, b) = hsl_to_rgb(h, s, l.clamp(0.0, 1.0));
                c.r = r;
                c.g = g;
                c.b = b;
            }
            ColorModifier::SatMod(val) => {
                let (h, mut s, l) = rgb_to_hsl(c.r, c.g, c.b);
                s *= *val as f64 / 100_000.0;
                let (r, g, b) = hsl_to_rgb(h, s.clamp(0.0, 1.0), l);
                c.r = r;
                c.g = g;
                c.b = b;
            }
            ColorModifier::SatOff(val) => {
                let (h, mut s, l) = rgb_to_hsl(c.r, c.g, c.b);
                s += *val as f64 / 100_000.0;
                let (r, g, b) = hsl_to_rgb(h, s.clamp(0.0, 1.0), l);
                c.r = r;
                c.g = g;
                c.b = b;
            }
            ColorModifier::HueMod(val) => {
                let (mut h, s, l) = rgb_to_hsl(c.r, c.g, c.b);
                h *= *val as f64 / 100_000.0;
                h = ((h % 360.0) + 360.0) % 360.0;
                let (r, g, b) = hsl_to_rgb(h, s, l);
                c.r = r;
                c.g = g;
                c.b = b;
            }
            ColorModifier::HueOff(val) => {
                let (mut h, s, l) = rgb_to_hsl(c.r, c.g, c.b);
                h += *val as f64 * 360.0 / 100_000.0;
                h = ((h % 360.0) + 360.0) % 360.0;
                let (r, g, b) = hsl_to_rgb(h, s, l);
                c.r = r;
                c.g = g;
                c.b = b;
            }
            ColorModifier::Inv => {
                c.r ^= 0xFF;
                c.g ^= 0xFF;
                c.b ^= 0xFF;
            }
            ColorModifier::Comp => {
                let (mut h, s, l) = rgb_to_hsl(c.r, c.g, c.b);
                h = (h + 180.0) % 360.0;
                let (r, g, b) = hsl_to_rgb(h, s, l);
                c.r = r;
                c.g = g;
                c.b = b;
            }
            ColorModifier::Gray => {
                let gray = clamp_u8(c.r as f64 * 0.299 + c.g as f64 * 0.587 + c.b as f64 * 0.114);
                c.r = gray;
                c.g = gray;
                c.b = gray;
            }
        }
    }
    c
}

// ── HSL conversion (H: 0-360, S: 0-1, L: 0-1) ──

fn rgb_to_hsl(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let rf = r as f64 / 255.0;
    let gf = g as f64 / 255.0;
    let bf = b as f64 / 255.0;

    let max = rf.max(gf).max(bf);
    let min = rf.min(gf).min(bf);
    let delta = max - min;
    let l = (max + min) / 2.0;

    if delta < 1e-10 {
        return (0.0, 0.0, l);
    }

    let s = if l < 0.5 {
        delta / (max + min)
    } else {
        delta / (2.0 - max - min)
    };

    let h = if (max - rf).abs() < 1e-10 {
        ((gf - bf) / delta) % 6.0
    } else if (max - gf).abs() < 1e-10 {
        (bf - rf) / delta + 2.0
    } else {
        (rf - gf) / delta + 4.0
    };

    let h = ((h * 60.0) + 360.0) % 360.0;
    (h, s, l)
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (u8, u8, u8) {
    if s < 1e-10 {
        let v = clamp_u8(l * 255.0);
        return (v, v, v);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let h_norm = h / 360.0;

    let r = hue_to_rgb(p, q, h_norm + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h_norm);
    let b = hue_to_rgb(p, q, h_norm - 1.0 / 3.0);

    (
        clamp_u8(r * 255.0),
        clamp_u8(g * 255.0),
        clamp_u8(b * 255.0),
    )
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

fn clamp_u8(v: f64) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

// ── Hex parsing ──

fn parse_hex_rgb(hex: &str) -> Option<ResolvedColor> {
    let hex = hex.trim_start_matches('#');
    if hex.len() < 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(ResolvedColor::new(r, g, b))
}

// ── OOXML default color aliases (ECMA-376 §20.1.6.1) ──
// When ClrMap has no explicit mapping, these names are NOT ColorScheme slots;
// they must be remapped to the actual scheme slot names.
fn ooxml_color_alias(name: &str) -> Option<&str> {
    match name {
        "tx1" => Some("dk1"),
        "tx2" => Some("dk2"),
        "bg1" => Some("lt1"),
        "bg2" => Some("lt2"),
        _ => None,
    }
}

// ── Theme color fallback ──

fn theme_fallback(name: &str) -> &str {
    match name {
        "dk1" | "tx1" => "000000",
        "lt1" | "bg1" => "FFFFFF",
        "dk2" | "tx2" => "44546A",
        "lt2" | "bg2" => "E7E6E6",
        "accent1" => "4472C4",
        "accent2" => "ED7D31",
        "accent3" => "A5A5A5",
        "accent4" => "FFC000",
        "accent5" => "5B9BD5",
        "accent6" => "70AD47",
        "hlink" => "0563C1",
        "folHlink" => "954F72",
        _ => "000000",
    }
}

fn system_color_fallback(name: &str) -> Option<&str> {
    Some(match name {
        "windowText" | "btnText" | "menuText" | "infoText" | "captionText" => "000000",
        "window" | "menu" | "btnFace" | "info" | "btnHighlight" => "FFFFFF",
        "highlight" => "0078D7",
        "highlightText" => "FFFFFF",
        "grayText" => "808080",
        _ => return None,
    })
}

fn preset_color(name: &str) -> Option<&str> {
    Some(match name {
        "black" => "000000",
        "white" => "FFFFFF",
        "red" => "FF0000",
        "green" | "lime" => "00FF00",
        "blue" => "0000FF",
        "yellow" => "FFFF00",
        "cyan" | "aqua" => "00FFFF",
        "magenta" | "fuchsia" => "FF00FF",
        "silver" => "C0C0C0",
        "gray" | "grey" => "808080",
        "maroon" => "800000",
        "olive" => "808000",
        "darkGreen" => "006400",
        "navy" => "000080",
        "teal" => "008080",
        "purple" => "800080",
        "orange" => "FFA500",
        "darkRed" | "dkRed" => "8B0000",
        "darkBlue" | "dkBlue" => "00008B",
        "coral" => "FF7F50",
        "cornflowerBlue" => "6495ED",
        "cornsilk" => "FFF8DC",
        "crimson" => "DC143C",
        "darkCyan" | "dkCyan" => "008B8B",
        "darkGoldenrod" => "B8860B",
        "darkGray" | "darkGrey" | "dkGray" => "A9A9A9",
        "darkKhaki" => "BDB76B",
        "darkMagenta" => "8B008B",
        "darkOliveGreen" => "556B2F",
        "darkOrange" => "FF8C00",
        "darkOrchid" => "9932CC",
        "darkSalmon" => "E9967A",
        "darkSeaGreen" => "8FBC8F",
        "darkSlateBlue" => "483D8B",
        "darkSlateGray" | "darkSlateGrey" => "2F4F4F",
        "darkTurquoise" => "00CED1",
        "darkViolet" => "9400D3",
        "deepPink" => "FF1493",
        "deepSkyBlue" => "00BFFF",
        "dimGray" | "dimGrey" => "696969",
        "dodgerBlue" => "1E90FF",
        "firebrick" => "B22222",
        "floralWhite" => "FFFAF0",
        "forestGreen" => "228B22",
        "gainsboro" => "DCDCDC",
        "ghostWhite" => "F8F8FF",
        "gold" => "FFD700",
        "goldenrod" => "DAA520",
        "honeydew" => "F0FFF0",
        "hotPink" => "FF69B4",
        "indianRed" => "CD5C5C",
        "indigo" => "4B0082",
        "ivory" => "FFFFF0",
        "khaki" => "F0E68C",
        "lavender" => "E6E6FA",
        "lavenderBlush" => "FFF0F5",
        "lawnGreen" => "7CFC00",
        "lemonChiffon" => "FFFACD",
        "lightBlue" | "ltBlue" => "ADD8E6",
        "lightCoral" => "F08080",
        "lightCyan" => "E0FFFF",
        "lightGoldenrodYellow" => "FAFAD2",
        "lightGray" | "lightGrey" | "ltGray" => "D3D3D3",
        "lightGreen" => "90EE90",
        "lightPink" => "FFB6C1",
        "lightSalmon" => "FFA07A",
        "lightSeaGreen" => "20B2AA",
        "lightSkyBlue" => "87CEFA",
        "lightSlateGray" | "lightSlateGrey" => "778899",
        "lightSteelBlue" => "B0C4DE",
        "lightYellow" => "FFFFE0",
        "limeGreen" => "32CD32",
        "linen" => "FAF0E6",
        "mediumAquamarine" | "medAquamarine" => "66CDAA",
        "mediumBlue" | "medBlue" => "0000CD",
        "mediumOrchid" | "medOrchid" => "BA55D3",
        "mediumPurple" | "medPurple" => "9370DB",
        "mediumSeaGreen" | "medSeaGreen" => "3CB371",
        "mediumSlateBlue" | "medSlateBlue" => "7B68EE",
        "mediumSpringGreen" | "medSpringGreen" => "00FA9A",
        "mediumTurquoise" | "medTurquoise" => "48D1CC",
        "mediumVioletRed" | "medVioletRed" => "C71585",
        "midnightBlue" => "191970",
        "mintCream" => "F5FFFA",
        "mistyRose" => "FFE4E1",
        "moccasin" => "FFE4B5",
        "navajoWhite" => "FFDEAD",
        "oldLace" => "FDF5E6",
        "oliveDrab" => "6B8E23",
        "orangeRed" => "FF4500",
        "orchid" => "DA70D6",
        "paleGoldenrod" => "EEE8AA",
        "paleGreen" => "98FB98",
        "paleTurquoise" => "AFEEEE",
        "paleVioletRed" => "DB7093",
        "papayaWhip" => "FFEFD5",
        "peachPuff" => "FFDAB9",
        "peru" => "CD853F",
        "pink" => "FFC0CB",
        "plum" => "DDA0DD",
        "powderBlue" => "B0E0E6",
        "rosyBrown" => "BC8F8F",
        "royalBlue" => "4169E1",
        "saddleBrown" => "8B4513",
        "salmon" => "FA8072",
        "sandyBrown" => "F4A460",
        "seaGreen" => "2E8B57",
        "seaShell" => "FFF5EE",
        "sienna" => "A0522D",
        "skyBlue" => "87CEEB",
        "slateBlue" => "6A5ACD",
        "slateGray" | "slateGrey" => "708090",
        "snow" => "FFFAFA",
        "springGreen" => "00FF7F",
        "steelBlue" => "4682B4",
        "tan" => "D2B48C",
        "thistle" => "D8BFD8",
        "tomato" => "FF6347",
        "turquoise" => "40E0D0",
        "violet" => "EE82EE",
        "wheat" => "F5DEB3",
        "whiteSmoke" => "F5F5F5",
        "yellowGreen" => "9ACD32",
        _ => return None,
    })
}

// ── Modifier parsing helpers ──

impl ColorModifier {
    pub fn from_ooxml(name: &str, val: Option<i32>) -> Option<Self> {
        Some(match name {
            "tint" => Self::Tint(val.unwrap_or(100_000)),
            "shade" => Self::Shade(val.unwrap_or(100_000)),
            "alpha" => Self::Alpha(val.unwrap_or(100_000)),
            "alphaOff" => Self::AlphaOff(val.unwrap_or(0)),
            "alphaMod" => Self::AlphaMod(val.unwrap_or(100_000)),
            "lumMod" => Self::LumMod(val.unwrap_or(100_000)),
            "lumOff" => Self::LumOff(val.unwrap_or(0)),
            "satMod" => Self::SatMod(val.unwrap_or(100_000)),
            "satOff" => Self::SatOff(val.unwrap_or(0)),
            "hueMod" => Self::HueMod(val.unwrap_or(100_000)),
            "hueOff" => Self::HueOff(val.unwrap_or(0)),
            "comp" => Self::Comp,
            "inv" => Self::Inv,
            "gray" => Self::Gray,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_rgb() {
        let c = parse_hex_rgb("FF0000").unwrap();
        assert_eq!(c, ResolvedColor::new(255, 0, 0));

        let c = parse_hex_rgb("#00FF00").unwrap();
        assert_eq!(c, ResolvedColor::new(0, 255, 0));

        assert!(parse_hex_rgb("FFF").is_none());
    }

    #[test]
    fn test_rgb_resolve() {
        let color = Color::rgb("4472C4");
        let resolved = color.resolve(None, None).unwrap();
        assert_eq!(resolved, ResolvedColor::new(0x44, 0x72, 0xC4));
    }

    #[test]
    fn test_theme_resolve_with_scheme() {
        let scheme = ColorScheme {
            accent1: "FF5733".to_string(),
            ..Default::default()
        };
        let color = Color::theme("accent1");
        let resolved = color.resolve(Some(&scheme), None).unwrap();
        assert_eq!(resolved, ResolvedColor::new(0xFF, 0x57, 0x33));
    }

    #[test]
    fn test_theme_with_clrmap() {
        let scheme = ColorScheme {
            dk1: "1A1A1A".to_string(),
            ..Default::default()
        };
        let mut clr_map = ClrMap::default();
        clr_map.set("tx1", "dk1");
        let color = Color::theme("tx1");
        let resolved = color.resolve(Some(&scheme), Some(&clr_map)).unwrap();
        assert_eq!(resolved, ResolvedColor::new(0x1A, 0x1A, 0x1A));
    }

    #[test]
    fn test_tx1_default_alias_no_clrmap() {
        // tx1 should resolve to dk1 via OOXML default alias when no ClrMap exists
        let scheme = ColorScheme {
            dk1: "333333".to_string(),
            ..Default::default()
        };
        let color = Color::theme("tx1");
        let resolved = color.resolve(Some(&scheme), None).unwrap();
        assert_eq!(resolved, ResolvedColor::new(0x33, 0x33, 0x33));
    }

    #[test]
    fn test_tx1_default_alias_empty_clrmap() {
        // tx1 should resolve to dk1 via OOXML default alias when ClrMap is present but empty
        let scheme = ColorScheme {
            dk1: "222222".to_string(),
            ..Default::default()
        };
        let clr_map = ClrMap::default();
        let color = Color::theme("tx1");
        let resolved = color.resolve(Some(&scheme), Some(&clr_map)).unwrap();
        assert_eq!(resolved, ResolvedColor::new(0x22, 0x22, 0x22));
    }

    #[test]
    fn test_bg1_default_alias() {
        // bg1 should resolve to lt1 via OOXML default alias
        let scheme = ColorScheme {
            lt1: "FAFAFA".to_string(),
            ..Default::default()
        };
        let color = Color::theme("bg1");
        let resolved = color.resolve(Some(&scheme), None).unwrap();
        assert_eq!(resolved, ResolvedColor::new(0xFA, 0xFA, 0xFA));
    }

    #[test]
    fn test_tx2_bg2_default_aliases() {
        let scheme = ColorScheme {
            dk2: "44546A".to_string(),
            lt2: "E7E6E6".to_string(),
            ..Default::default()
        };
        let tx2 = Color::theme("tx2").resolve(Some(&scheme), None).unwrap();
        assert_eq!(tx2, ResolvedColor::new(0x44, 0x54, 0x6A));
        let bg2 = Color::theme("bg2").resolve(Some(&scheme), None).unwrap();
        assert_eq!(bg2, ResolvedColor::new(0xE7, 0xE6, 0xE6));
    }

    #[test]
    fn test_tint_modifier() {
        // OOXML tint on HSL luminance: new_L = L * tint + (1.0 - tint)
        // Pure black (L=0.0) + tint 50000 → new_L = 0*0.5 + 0.5 = 0.5 → gray
        let color = Color::rgb("000000").with_modifier(ColorModifier::Tint(50000));
        let r = color.resolve(None, None).unwrap();
        assert_eq!(r, ResolvedColor::new(128, 128, 128));
    }

    #[test]
    fn test_shade_modifier() {
        // OOXML shade on HSL luminance: new_L = L * shade
        // Pure white (L=1.0) + shade 50000 → new_L = 1.0*0.5 = 0.5 → gray
        let color = Color::rgb("FFFFFF").with_modifier(ColorModifier::Shade(50000));
        let r = color.resolve(None, None).unwrap();
        assert_eq!(r, ResolvedColor::new(128, 128, 128));
    }

    #[test]
    fn test_alpha_modifier() {
        let color = Color::rgb("FF0000").with_modifier(ColorModifier::Alpha(50000));
        let r = color.resolve(None, None).unwrap();
        assert_eq!(r.r, 255);
        assert_eq!(r.a, 128);
        assert_eq!(r.to_css(), "rgba(255, 0, 0, 0.50)");
    }

    #[test]
    fn test_lum_mod_off() {
        // accent1 color with lumMod 75000 + lumOff 25000
        let color = Color::rgb("4472C4")
            .with_modifier(ColorModifier::LumMod(75000))
            .with_modifier(ColorModifier::LumOff(25000));
        let r = color.resolve(None, None).unwrap();
        // Result should be a lightened blue
        assert!(r.r > 0x44);
        assert!(r.b >= 0xC4); // should be brighter
    }

    #[test]
    fn test_inv_modifier() {
        let color = Color::rgb("FF0000").with_modifier(ColorModifier::Inv);
        let r = color.resolve(None, None).unwrap();
        assert_eq!(r, ResolvedColor::new(0, 255, 255));
    }

    #[test]
    fn test_gray_modifier() {
        let color = Color::rgb("FF0000").with_modifier(ColorModifier::Gray);
        let r = color.resolve(None, None).unwrap();
        // Grayscale of red: 0.299*255 ~ 76
        assert_eq!(r.r, r.g);
        assert_eq!(r.g, r.b);
    }

    #[test]
    fn test_hsl_roundtrip() {
        for (r, g, b) in [
            (255, 0, 0),
            (0, 255, 0),
            (0, 0, 255),
            (128, 64, 192),
            (0, 0, 0),
            (255, 255, 255),
        ] {
            let (h, s, l) = rgb_to_hsl(r, g, b);
            let (r2, g2, b2) = hsl_to_rgb(h, s, l);
            assert!(
                (r as i16 - r2 as i16).abs() <= 1,
                "R mismatch for ({r},{g},{b}): got ({r2},{g2},{b2})"
            );
            assert!(
                (g as i16 - g2 as i16).abs() <= 1,
                "G mismatch for ({r},{g},{b}): got ({r2},{g2},{b2})"
            );
            assert!(
                (b as i16 - b2 as i16).abs() <= 1,
                "B mismatch for ({r},{g},{b}): got ({r2},{g2},{b2})"
            );
        }
    }

    #[test]
    fn test_preset_color() {
        let color = Color::preset("red");
        let r = color.resolve(None, None).unwrap();
        assert_eq!(r, ResolvedColor::new(255, 0, 0));
    }

    #[test]
    fn test_system_color() {
        let color = Color::system("windowText");
        let r = color.resolve(None, None).unwrap();
        assert_eq!(r, ResolvedColor::new(0, 0, 0));
    }

    #[test]
    fn test_resolved_color_css() {
        assert_eq!(ResolvedColor::new(255, 0, 0).to_css(), "#FF0000");
        assert_eq!(
            ResolvedColor::with_alpha(255, 0, 0, 128).to_css(),
            "rgba(255, 0, 0, 0.50)"
        );
    }

    #[test]
    fn test_none_resolve() {
        assert!(Color::none().resolve(None, None).is_none());
    }

    // ── ECMA-376 modifier application order tests ──

    #[test]
    fn test_modifier_order_alpha_before_lum() {
        // Alpha should be applied first (order 0), then LumMod (order 3).
        // Regardless of insertion order, alpha should not affect RGB channels.
        let color = Color::rgb("FF0000")
            .with_modifier(ColorModifier::LumMod(50000))
            .with_modifier(ColorModifier::Alpha(50000));
        let r = color.resolve(None, None).unwrap();
        // Alpha applied first: a=128, then LumMod darkens
        assert_eq!(r.a, 128);
        assert!(r.r < 255); // LumMod darkened the red
    }

    #[test]
    fn test_modifier_order_sat_before_lum() {
        // SatMod (order 2) should apply before LumMod (order 3).
        // If we desaturate first then darken, result differs from darkening first.
        let color_spec_order = Color::rgb("4472C4")
            .with_modifier(ColorModifier::SatMod(0))
            .with_modifier(ColorModifier::LumMod(50000));
        let r1 = color_spec_order.resolve(None, None).unwrap();

        // SatMod=0 → grayscale (in HSL), then LumMod=50000 darkens
        // Result should be a dark gray
        assert_eq!(r1.r, r1.g);
        assert_eq!(r1.g, r1.b);
        assert!(r1.r < 128); // darkened gray
    }

    #[test]
    fn test_modifier_order_hue_before_sat() {
        // HueOff (order 1) before SatMod (order 2).
        // Even if SatMod is listed first, hue shift should happen first.
        let color = Color::rgb("FF0000")
            .with_modifier(ColorModifier::SatMod(100_000))
            .with_modifier(ColorModifier::HueOff(33333)); // ~120° shift → green
        let r = color.resolve(None, None).unwrap();
        // Hue shifted from red (0°) to ~120° (green area)
        assert!(r.g > r.r);
    }

    #[test]
    fn test_modifier_order_lum_before_tint() {
        // LumMod (order 3) before Tint (order 4).
        // Darken first, then tint toward white.
        let color = Color::rgb("4472C4")
            .with_modifier(ColorModifier::Tint(50000))
            .with_modifier(ColorModifier::LumMod(50000));
        let r = color.resolve(None, None).unwrap();
        // LumMod darkens to ~half luminance, then tint pushes toward white
        // Result should be a lighter color than just LumMod alone
        let color_lum_only = Color::rgb("4472C4").with_modifier(ColorModifier::LumMod(50000));
        let r_lum = color_lum_only.resolve(None, None).unwrap();
        // Tinted result should have higher luminance values
        let avg_tinted = (r.r as u16 + r.g as u16 + r.b as u16) / 3;
        let avg_lum = (r_lum.r as u16 + r_lum.g as u16 + r_lum.b as u16) / 3;
        assert!(avg_tinted > avg_lum);
    }

    #[test]
    fn test_modifier_order_tint_before_comp() {
        // Tint (order 4) before Comp (order 5).
        let color = Color::rgb("FF0000")
            .with_modifier(ColorModifier::Comp)
            .with_modifier(ColorModifier::Tint(50000));
        let r = color.resolve(None, None).unwrap();
        // Tint first → lighter red, then complement → cyan-ish
        assert!(r.g > 100 && r.b > 100);
    }

    // ── Edge case tests ──

    #[test]
    fn test_lum_mod_zero_produces_black() {
        // lumMod=0 should reduce luminance to 0 → black
        let color = Color::rgb("4472C4").with_modifier(ColorModifier::LumMod(0));
        let r = color.resolve(None, None).unwrap();
        assert_eq!(r, ResolvedColor::new(0, 0, 0));
    }

    #[test]
    fn test_lum_off_100000_produces_white() {
        // lumOff=100000 (100%) should push luminance to 1.0 → white
        // Start from black (L=0), lumOff=100000 → L = 0 + 1.0 = 1.0
        let color = Color::rgb("000000").with_modifier(ColorModifier::LumOff(100_000));
        let r = color.resolve(None, None).unwrap();
        assert_eq!(r, ResolvedColor::new(255, 255, 255));
    }

    #[test]
    fn test_sat_mod_zero_produces_grayscale() {
        // satMod=0 should remove all saturation → grayscale
        let color = Color::rgb("FF0000").with_modifier(ColorModifier::SatMod(0));
        let r = color.resolve(None, None).unwrap();
        assert_eq!(r.r, r.g);
        assert_eq!(r.g, r.b);
    }

    #[test]
    fn test_hue_mod_wraps_over_360() {
        // hueMod that results in > 360° should wrap around
        // Red (H~0°) * 200% = 0° → still red-ish (0 * 2 = 0)
        // Use a color with non-zero hue: blue has H=240°
        // 240 * 2.0 = 480 → wraps to 120° (green area)
        let color = Color::rgb("0000FF").with_modifier(ColorModifier::HueMod(200_000));
        let r = color.resolve(None, None).unwrap();
        // H=120° is green
        assert!(r.g > r.r);
        assert!(r.g > r.b);
    }

    #[test]
    fn test_tint_on_colored_input() {
        // Tint 40000 on accent1 (4472C4): L * 0.4 + 0.6
        // Original HSL L ≈ 0.52 → new_L = 0.52*0.4 + 0.6 = 0.808
        // Should produce a lighter blue
        let color = Color::rgb("4472C4").with_modifier(ColorModifier::Tint(40000));
        let r = color.resolve(None, None).unwrap();
        assert!(r.r > 0x44);
        assert!(r.g > 0x72);
        assert!(r.b > 0xC4);
    }

    #[test]
    fn test_shade_on_colored_input() {
        // Shade 75000 on accent1 (4472C4): L * 0.75
        // Should produce a darker blue
        let color = Color::rgb("4472C4").with_modifier(ColorModifier::Shade(75000));
        let r = color.resolve(None, None).unwrap();
        // Luminance decreased, so overall brightness should be lower
        let original_avg = (0x44u16 + 0x72 + 0xC4) / 3;
        let result_avg = (r.r as u16 + r.g as u16 + r.b as u16) / 3;
        assert!(result_avg < original_avg);
    }

    #[test]
    fn test_tint_100000_is_identity() {
        // Tint 100000 (100%): new_L = L * 1.0 + 0.0 = L → no change
        let color = Color::rgb("4472C4").with_modifier(ColorModifier::Tint(100_000));
        let r = color.resolve(None, None).unwrap();
        assert_eq!(r, ResolvedColor::new(0x44, 0x72, 0xC4));
    }

    #[test]
    fn test_shade_100000_is_identity() {
        // Shade 100000 (100%): new_L = L * 1.0 = L → no change
        let color = Color::rgb("4472C4").with_modifier(ColorModifier::Shade(100_000));
        let r = color.resolve(None, None).unwrap();
        assert_eq!(r, ResolvedColor::new(0x44, 0x72, 0xC4));
    }

    #[test]
    fn test_tint_zero_produces_white() {
        // Tint 0: new_L = L * 0 + 1.0 = 1.0 → white
        let color = Color::rgb("4472C4").with_modifier(ColorModifier::Tint(0));
        let r = color.resolve(None, None).unwrap();
        assert_eq!(r, ResolvedColor::new(255, 255, 255));
    }

    #[test]
    fn test_shade_zero_produces_black() {
        // Shade 0: new_L = L * 0 = 0 → black
        let color = Color::rgb("4472C4").with_modifier(ColorModifier::Shade(0));
        let r = color.resolve(None, None).unwrap();
        assert_eq!(r, ResolvedColor::new(0, 0, 0));
    }

    #[test]
    fn test_combined_lum_mod_off_typical() {
        // Common OOXML pattern: lumMod=60000, lumOff=40000
        // First mod: L * 0.6, then off: + 0.4
        // For black (L=0): 0*0.6 + 0.4 = 0.4 → medium gray
        let color = Color::rgb("000000")
            .with_modifier(ColorModifier::LumMod(60000))
            .with_modifier(ColorModifier::LumOff(40000));
        let r = color.resolve(None, None).unwrap();
        // L=0.4 → gray ~102
        assert!((r.r as i16 - 102).abs() <= 1);
        assert_eq!(r.r, r.g);
        assert_eq!(r.g, r.b);
    }

    #[test]
    fn test_system_and_preset_color_css_fallbacks() {
        assert_eq!(
            Color::system("windowText").to_css(),
            Some("#000000".to_string())
        );
        assert_eq!(
            Color::preset("orange").to_css(),
            Some("#FFA500".to_string())
        );
        assert_eq!(Color::system("missing").to_css(), None);
        assert_eq!(Color::preset("missing").to_css(), None);
    }

    #[test]
    fn test_resolved_color_css_uses_rgba_for_transparent_values() {
        let color = ResolvedColor::with_alpha(16, 32, 48, 128);
        assert_eq!(color.to_css(), "rgba(16, 32, 48, 0.50)");
    }

    #[test]
    fn test_alpha_offset_and_modifier_cover_remaining_alpha_branches() {
        let color = Color::rgb("804020")
            .with_modifier(ColorModifier::AlphaOff(-50_000))
            .with_modifier(ColorModifier::AlphaMod(50_000));
        let resolved = color.resolve(None, None).unwrap();
        assert_eq!(resolved.a, 64);
    }

    #[test]
    fn test_sat_off_modifier_increases_saturation() {
        let base = ResolvedColor::new(100, 110, 120);
        let saturated = apply_modifiers(base, &[ColorModifier::SatOff(25_000)]);
        assert_ne!(saturated, base);
    }

    #[test]
    fn test_color_modifier_from_ooxml_covers_supported_names() {
        let cases = [
            ("tint", Some(50_000), Some(ColorModifier::Tint(50_000))),
            ("shade", None, Some(ColorModifier::Shade(100_000))),
            ("alpha", Some(80_000), Some(ColorModifier::Alpha(80_000))),
            ("alphaOff", None, Some(ColorModifier::AlphaOff(0))),
            ("alphaMod", None, Some(ColorModifier::AlphaMod(100_000))),
            ("lumMod", Some(75_000), Some(ColorModifier::LumMod(75_000))),
            ("lumOff", None, Some(ColorModifier::LumOff(0))),
            ("satMod", Some(90_000), Some(ColorModifier::SatMod(90_000))),
            ("satOff", None, Some(ColorModifier::SatOff(0))),
            ("hueMod", Some(50_000), Some(ColorModifier::HueMod(50_000))),
            ("hueOff", None, Some(ColorModifier::HueOff(0))),
            ("comp", None, Some(ColorModifier::Comp)),
            ("inv", None, Some(ColorModifier::Inv)),
            ("gray", None, Some(ColorModifier::Gray)),
            ("unknown", Some(12_345), None),
        ];

        for (name, value, expected) in cases {
            assert_eq!(ColorModifier::from_ooxml(name, value), expected);
        }
    }

    #[test]
    fn test_system_color_fallback_covers_every_supported_arm() {
        let cases = [
            ("windowText", Some("000000")),
            ("window", Some("FFFFFF")),
            ("highlight", Some("0078D7")),
            ("highlightText", Some("FFFFFF")),
            ("grayText", Some("808080")),
            ("unsupported", None),
        ];

        for (name, expected) in cases {
            assert_eq!(system_color_fallback(name), expected);
        }
    }

    #[test]
    fn test_preset_color_covers_supported_palette_entries() {
        let cases = [
            ("black", "000000"),
            ("white", "FFFFFF"),
            ("red", "FF0000"),
            ("green", "00FF00"),
            ("blue", "0000FF"),
            ("yellow", "FFFF00"),
            ("cyan", "00FFFF"),
            ("magenta", "FF00FF"),
            ("silver", "C0C0C0"),
            ("gray", "808080"),
            ("maroon", "800000"),
            ("olive", "808000"),
            ("darkGreen", "006400"),
            ("navy", "000080"),
            ("teal", "008080"),
            ("purple", "800080"),
            ("orange", "FFA500"),
            ("darkRed", "8B0000"),
            ("darkBlue", "00008B"),
            ("coral", "FF7F50"),
            ("cornflowerBlue", "6495ED"),
            ("cornsilk", "FFF8DC"),
            ("crimson", "DC143C"),
            ("darkCyan", "008B8B"),
            ("darkGoldenrod", "B8860B"),
            ("darkGray", "A9A9A9"),
            ("darkKhaki", "BDB76B"),
            ("darkMagenta", "8B008B"),
            ("darkOliveGreen", "556B2F"),
            ("darkOrange", "FF8C00"),
            ("darkOrchid", "9932CC"),
            ("darkSalmon", "E9967A"),
            ("darkSeaGreen", "8FBC8F"),
            ("darkSlateBlue", "483D8B"),
            ("darkSlateGray", "2F4F4F"),
            ("darkTurquoise", "00CED1"),
            ("darkViolet", "9400D3"),
            ("deepPink", "FF1493"),
            ("deepSkyBlue", "00BFFF"),
            ("dimGray", "696969"),
            ("dodgerBlue", "1E90FF"),
            ("firebrick", "B22222"),
            ("floralWhite", "FFFAF0"),
            ("forestGreen", "228B22"),
            ("gainsboro", "DCDCDC"),
            ("ghostWhite", "F8F8FF"),
            ("gold", "FFD700"),
            ("goldenrod", "DAA520"),
            ("honeydew", "F0FFF0"),
            ("hotPink", "FF69B4"),
            ("indianRed", "CD5C5C"),
            ("indigo", "4B0082"),
            ("ivory", "FFFFF0"),
            ("khaki", "F0E68C"),
            ("lavender", "E6E6FA"),
            ("lavenderBlush", "FFF0F5"),
            ("lawnGreen", "7CFC00"),
            ("lemonChiffon", "FFFACD"),
            ("lightBlue", "ADD8E6"),
            ("lightCoral", "F08080"),
            ("lightCyan", "E0FFFF"),
            ("lightGoldenrodYellow", "FAFAD2"),
            ("lightGray", "D3D3D3"),
            ("lightGreen", "90EE90"),
            ("lightPink", "FFB6C1"),
            ("lightSalmon", "FFA07A"),
            ("lightSeaGreen", "20B2AA"),
            ("lightSkyBlue", "87CEFA"),
            ("lightSlateGray", "778899"),
            ("lightSteelBlue", "B0C4DE"),
            ("lightYellow", "FFFFE0"),
            ("limeGreen", "32CD32"),
            ("linen", "FAF0E6"),
            ("mediumAquamarine", "66CDAA"),
            ("mediumBlue", "0000CD"),
            ("mediumOrchid", "BA55D3"),
            ("mediumPurple", "9370DB"),
            ("mediumSeaGreen", "3CB371"),
            ("mediumSlateBlue", "7B68EE"),
            ("mediumSpringGreen", "00FA9A"),
            ("mediumTurquoise", "48D1CC"),
            ("mediumVioletRed", "C71585"),
            ("midnightBlue", "191970"),
            ("mintCream", "F5FFFA"),
            ("mistyRose", "FFE4E1"),
            ("moccasin", "FFE4B5"),
            ("navajoWhite", "FFDEAD"),
            ("oldLace", "FDF5E6"),
            ("oliveDrab", "6B8E23"),
            ("orangeRed", "FF4500"),
            ("orchid", "DA70D6"),
            ("paleGoldenrod", "EEE8AA"),
            ("paleGreen", "98FB98"),
            ("paleTurquoise", "AFEEEE"),
            ("paleVioletRed", "DB7093"),
            ("papayaWhip", "FFEFD5"),
            ("peachPuff", "FFDAB9"),
            ("peru", "CD853F"),
            ("pink", "FFC0CB"),
            ("plum", "DDA0DD"),
            ("powderBlue", "B0E0E6"),
            ("rosyBrown", "BC8F8F"),
            ("royalBlue", "4169E1"),
            ("saddleBrown", "8B4513"),
            ("salmon", "FA8072"),
            ("sandyBrown", "F4A460"),
            ("seaGreen", "2E8B57"),
            ("seaShell", "FFF5EE"),
            ("sienna", "A0522D"),
            ("skyBlue", "87CEEB"),
            ("slateBlue", "6A5ACD"),
            ("slateGray", "708090"),
            ("snow", "FFFAFA"),
            ("springGreen", "00FF7F"),
            ("steelBlue", "4682B4"),
            ("tan", "D2B48C"),
            ("thistle", "D8BFD8"),
            ("tomato", "FF6347"),
            ("turquoise", "40E0D0"),
            ("violet", "EE82EE"),
            ("wheat", "F5DEB3"),
            ("whiteSmoke", "F5F5F5"),
            ("yellowGreen", "9ACD32"),
        ];

        for (name, expected) in cases {
            assert_eq!(preset_color(name), Some(expected), "preset color {name}");
        }
        assert_eq!(preset_color("unsupported"), None);
    }
}
