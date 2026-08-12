use std::collections::HashMap;

use super::geometry::Size;
use super::hierarchy::{FmtScheme, ListStyle, SlideLayout, SlideMaster};
use super::slide::Slide;

/// Top-level presentation structure
#[derive(Debug, Clone, Default)]
pub struct Presentation {
    pub slides: Vec<Slide>,
    pub slide_size: Size,
    pub title: Option<String>,
    pub themes: Vec<Theme>,
    pub masters: Vec<SlideMaster>,
    pub layouts: Vec<SlideLayout>,
    pub default_text_style: Option<ListStyle>,
    pub clr_map: ClrMap,
}

impl Presentation {
    /// Get primary theme (backward compat)
    pub fn primary_theme(&self) -> Option<&Theme> {
        self.themes.first()
    }
}

/// Theme data
#[derive(Debug, Clone, Default)]
pub struct Theme {
    pub name: String,
    pub color_scheme: ColorScheme,
    pub font_scheme: FontScheme,
    pub fmt_scheme: FmtScheme,
}

/// Theme color scheme (12 colors)
#[derive(Debug, Clone, Default)]
pub struct ColorScheme {
    pub dk1: String,
    pub lt1: String,
    pub dk2: String,
    pub lt2: String,
    pub accent1: String,
    pub accent2: String,
    pub accent3: String,
    pub accent4: String,
    pub accent5: String,
    pub accent6: String,
    pub hlink: String,
    pub fol_hlink: String,
}

impl ColorScheme {
    /// Look up hex color value by scheme name
    pub fn get(&self, name: &str) -> Option<String> {
        let hex = match name {
            "dk1" => &self.dk1,
            "lt1" => &self.lt1,
            "dk2" => &self.dk2,
            "lt2" => &self.lt2,
            "accent1" => &self.accent1,
            "accent2" => &self.accent2,
            "accent3" => &self.accent3,
            "accent4" => &self.accent4,
            "accent5" => &self.accent5,
            "accent6" => &self.accent6,
            "hlink" => &self.hlink,
            "folHlink" | "fol_hlink" => &self.fol_hlink,
            _ => return None,
        };
        if hex.is_empty() {
            None
        } else {
            Some(hex.clone())
        }
    }
}

/// Theme font scheme
#[derive(Debug, Clone, Default)]
pub struct FontScheme {
    pub major_latin: String,
    pub minor_latin: String,
    pub major_east_asian: Option<String>,
    pub minor_east_asian: Option<String>,
    pub major_complex_script: Option<String>,
    pub minor_complex_script: Option<String>,
}

impl FontScheme {
    /// Resolve "+mj-lt" / "+mn-lt" / "+mj-ea" / "+mn-ea" / "+mj-cs" / "+mn-cs" theme font references
    /// to actual typeface names. Returns None if the reference is not recognized
    /// or if the resolved name is empty.
    pub fn resolve_typeface<'a>(&'a self, typeface: &str) -> Option<&'a str> {
        let result = match typeface {
            "+mj-lt" => Some(self.major_latin.as_str()),
            "+mn-lt" => Some(self.minor_latin.as_str()),
            "+mj-ea" => self.major_east_asian.as_deref(),
            "+mn-ea" => self.minor_east_asian.as_deref(),
            "+mj-cs" => self.major_complex_script.as_deref(),
            "+mn-cs" => self.minor_complex_script.as_deref(),
            _ => None,
        };
        result.filter(|s| !s.is_empty())
    }
}

/// ClrMap -- color name mapping (from slideMaster `<a:clrMap>`)
///
/// e.g. stores tx1->dk1, bg1->lt1 mappings
#[derive(Debug, Clone, Default)]
pub struct ClrMap {
    map: HashMap<String, String>,
}

impl ClrMap {
    pub fn get(&self, key: &str) -> Option<&String> {
        // Compatibility aliases
        let normalized = match key {
            "t1" => "tx1",
            "t2" => "tx2",
            "followedHyperlink" => "folHlink",
            "hyperlink" => "hlink",
            other => other,
        };
        self.map.get(normalized)
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.map.insert(key.to_string(), value.to_string());
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}
