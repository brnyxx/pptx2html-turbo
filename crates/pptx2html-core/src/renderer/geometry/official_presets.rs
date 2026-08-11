use std::collections::HashMap;
use std::sync::OnceLock;

use log::warn;

use super::official_presets_formula::GuideEnvironment;
use super::official_presets_path::render_path;
use super::official_presets_xml::{PresetDefinition, parse_definitions};
use super::shared::{CustomGeomPathSvg, CustomGeomSvg};
use crate::model::PathFill;
static DEFINITIONS: OnceLock<Result<HashMap<String, PresetDefinition>, String>> = OnceLock::new();

const OFFICIAL_NAMES: [&str; 55] = [
    "rightArrow",
    "leftArrow",
    "upArrow",
    "downArrow",
    "leftRightArrow",
    "upDownArrow",
    "bentArrow",
    "chevron",
    "notchedRightArrow",
    "stripedRightArrow",
    "curvedRightArrow",
    "curvedLeftArrow",
    "curvedUpArrow",
    "curvedDownArrow",
    "circularArrow",
    "bentUpArrow",
    "uturnArrow",
    "leftRightUpArrow",
    "quadArrow",
    "leftUpArrow",
    "homePlate",
    "wedgeRoundRectCallout",
    "wedgeEllipseCallout",
    "cloudCallout",
    "callout1",
    "callout2",
    "callout3",
    "borderCallout1",
    "borderCallout2",
    "borderCallout3",
    "accentCallout1",
    "accentCallout2",
    "accentCallout3",
    "accentBorderCallout1",
    "accentBorderCallout2",
    "accentBorderCallout3",
    "wedgeRectCallout",
    "downArrowCallout",
    "leftArrowCallout",
    "rightArrowCallout",
    "upArrowCallout",
    "quadArrowCallout",
    "leftRightArrowCallout",
    "upDownArrowCallout",
    "leftCircularArrow",
    "leftRightCircularArrow",
    "swooshArrow",
    "curvedConnector2",
    "curvedConnector3",
    "curvedConnector4",
    "curvedConnector5",
    "bentConnector2",
    "bentConnector3",
    "bentConnector4",
    "bentConnector5",
];

pub(super) enum OfficialPresetRender {
    NotOfficial,
    Rendered(CustomGeomSvg),
    Invalid(CustomGeomSvg),
}

pub(super) fn route(
    name: &str,
    width: f64,
    height: f64,
    adjustments: &HashMap<String, f64>,
) -> OfficialPresetRender {
    let definitions = if OFFICIAL_NAMES.contains(&name) {
        definitions()
    } else if super::official_remaining_presets::contains(name) {
        super::official_remaining_presets::definitions()
    } else {
        return OfficialPresetRender::NotOfficial;
    };
    match definitions.and_then(|definitions| {
        render_from_definitions(definitions, name, width, height, adjustments)
    }) {
        Ok(svg) => OfficialPresetRender::Rendered(svg),
        Err(error) => {
            warn!("official preset fallback: preset={name} error={error}");
            OfficialPresetRender::Invalid(invalid_fallback(width, height))
        }
    }
}

fn render_from_definitions(
    definitions: &HashMap<String, PresetDefinition>,
    name: &str,
    width: f64,
    height: f64,
    adjustments: &HashMap<String, f64>,
) -> Result<CustomGeomSvg, String> {
    let definition = definitions
        .get(name)
        .ok_or_else(|| format!("unknown official preset: {name}"))?;
    let mut environment = GuideEnvironment::new(width, height);
    for adjustment in &definition.adjustments {
        let default = environment.evaluate(&adjustment.formula)?;
        let value = adjustments
            .get(&adjustment.name)
            .copied()
            .filter(|value| value.is_finite())
            .unwrap_or(default);
        environment.insert(&adjustment.name, value)?;
    }
    for guide in &definition.guides {
        let value = environment.evaluate(&guide.formula)?;
        environment.insert(&guide.name, value)?;
    }
    let paths = definition
        .paths
        .iter()
        .map(|path| render_path(path, &environment, width, height))
        .collect::<Result<Vec<_>, _>>()?;
    if paths.is_empty() {
        return Err(format!("official preset has no paths: {name}"));
    }
    Ok(CustomGeomSvg { paths })
}

fn invalid_fallback(width: f64, height: f64) -> CustomGeomSvg {
    let width = finite_extent(width);
    let height = finite_extent(height);
    CustomGeomSvg {
        paths: vec![CustomGeomPathSvg {
            d: format!("M0.00,0.00 L{width:.2},0.00 L{width:.2},{height:.2} L0.00,{height:.2} Z"),
            fill: PathFill::Norm,
            stroke: true,
        }],
    }
}

fn finite_extent(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn definitions() -> Result<&'static HashMap<String, PresetDefinition>, String> {
    DEFINITIONS
        .get_or_init(parse_definitions)
        .as_ref()
        .map_err(Clone::clone)
}

#[cfg(test)]
pub(super) fn route_with_xml(
    name: &str,
    width: f64,
    height: f64,
    adjustments: &HashMap<String, f64>,
    xml: &str,
) -> OfficialPresetRender {
    if !OFFICIAL_NAMES.contains(&name) {
        return OfficialPresetRender::NotOfficial;
    }
    match super::official_presets_xml::parse_definitions_with_count(xml, 55).and_then(
        |definitions| render_from_definitions(&definitions, name, width, height, adjustments),
    ) {
        Ok(svg) => OfficialPresetRender::Rendered(svg),
        Err(_) => OfficialPresetRender::Invalid(invalid_fallback(width, height)),
    }
}

#[cfg(test)]
mod tests {
    use super::{OfficialPresetRender, definitions, route, route_with_xml};
    use crate::renderer::geometry::official_presets_xml::source_xml;
    use std::collections::HashMap;

    #[test]
    fn official_asset_parses_all_presets_and_paths() {
        let parsed = definitions().expect("official definitions");
        assert_eq!(parsed.len(), 55);
        assert!(
            parsed
                .values()
                .all(|definition| !definition.paths.is_empty())
        );
        for name in parsed.keys() {
            let OfficialPresetRender::Rendered(svg) = route(name, 160.0, 100.0, &HashMap::new())
            else {
                panic!("official SVG must render")
            };
            assert!(svg.paths.iter().all(|path| !path.d.is_empty()));
        }
    }

    #[test]
    fn official_faults_return_typed_deterministic_fallback_without_legacy() {
        let source = source_xml();
        let faults = [
            source.replacen("fmla=\"val 50000\"", "fmla=\"bad 50000\"", 1),
            source.replacen("<lnTo>", "<unknownTo>", 1),
            source.replacen("<path>", "<path invented=\"1\">", 1),
            source.replacen(
                "<pt x=\"l\" y=\"y1\" />",
                "<pt x=\"missingNumericGuide\" y=\"y1\" />",
                1,
            ),
            source.replacen("<rightArrow>", "<rightArrow", 1),
            source.replacen("fmla=\"val 50000\"", "fmla=\"val 1e999\"", 1),
        ];
        for malformed in faults {
            let first = route_with_xml("rightArrow", 160.0, 100.0, &HashMap::new(), &malformed);
            let second = route_with_xml("rightArrow", 160.0, 100.0, &HashMap::new(), &malformed);
            let (OfficialPresetRender::Invalid(first), OfficialPresetRender::Invalid(second)) =
                (first, second)
            else {
                panic!("official fault must return Invalid, never NotOfficial")
            };
            assert_eq!(first.paths[0].d, second.paths[0].d);
            assert_eq!(
                first.paths[0].d,
                "M0.00,0.00 L160.00,0.00 L160.00,100.00 L0.00,100.00 Z"
            );
        }
    }

    #[test]
    fn non_task9_name_is_the_only_not_official_outcome() {
        assert!(matches!(
            route_with_xml("rect", 160.0, 100.0, &HashMap::new(), source_xml()),
            OfficialPresetRender::NotOfficial
        ));
    }
}
