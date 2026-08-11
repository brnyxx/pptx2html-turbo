use std::collections::HashMap;
use std::sync::OnceLock;

use super::official_presets_xml::{PresetDefinition, parse_definitions_with_count};

const XML: &str = include_str!("official_remaining_presets.xml");
const NAMES: [&str; 63] = [
    "heptagon",
    "decagon",
    "dodecagon",
    "star4",
    "star5",
    "star6",
    "star7",
    "star8",
    "star10",
    "star12",
    "star16",
    "star24",
    "star32",
    "teardrop",
    "pieWedge",
    "pie",
    "blockArc",
    "donut",
    "noSmoking",
    "cube",
    "can",
    "lightningBolt",
    "heart",
    "sun",
    "moon",
    "smileyFace",
    "irregularSeal1",
    "irregularSeal2",
    "bevel",
    "frame",
    "chord",
    "arc",
    "cloud",
    "ribbon",
    "ribbon2",
    "leftRightRibbon",
    "wave",
    "doubleWave",
    "plus",
    "actionButtonBlank",
    "actionButtonHome",
    "actionButtonHelp",
    "actionButtonInformation",
    "actionButtonForwardNext",
    "actionButtonBackPrevious",
    "actionButtonEnd",
    "actionButtonBeginning",
    "actionButtonReturn",
    "actionButtonDocument",
    "actionButtonSound",
    "actionButtonMovie",
    "gear6",
    "gear9",
    "funnel",
    "mathPlus",
    "mathMinus",
    "mathMultiply",
    "mathDivide",
    "mathEqual",
    "mathNotEqual",
    "chartX",
    "chartStar",
    "chartPlus",
];
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
