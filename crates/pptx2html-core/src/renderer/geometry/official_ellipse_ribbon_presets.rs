use std::collections::HashMap;
use std::sync::OnceLock;

use super::official_presets_xml::{PresetDefinition, parse_definitions_with_count};

// ECMA-376 Part 1, 5th edition presetShapeDefinitions.xml.
// Geometry member SHA-256: 2f7c868d857c1e3c4b5a6068759fe0e07d77ad58377a6618d1b02ba3507b6939.
const XML: &str = include_str!("official_ellipse_ribbon_presets.xml");
const NAMES: [&str; 2] = ["ellipseRibbon", "ellipseRibbon2"];
static DEFINITIONS: OnceLock<Result<HashMap<String, PresetDefinition>, String>> = OnceLock::new();

pub(super) fn contains(name: &str) -> bool {
    NAMES.contains(&name)
}

pub(super) fn definitions() -> Result<&'static HashMap<String, PresetDefinition>, String> {
    DEFINITIONS
        .get_or_init(|| parse_definitions_with_count(XML, NAMES.len()))
        .as_ref()
        .map_err(Clone::clone)
}
