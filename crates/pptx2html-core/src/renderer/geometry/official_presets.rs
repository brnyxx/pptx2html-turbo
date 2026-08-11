use std::collections::HashMap;
use std::sync::OnceLock;

use super::official_presets_formula::GuideEnvironment;
use super::official_presets_path::render_path;
use super::official_presets_xml::{PresetDefinition, parse_definitions};
use super::shared::CustomGeomSvg;
static DEFINITIONS: OnceLock<Result<HashMap<String, PresetDefinition>, String>> = OnceLock::new();

pub(super) fn owns(name: &str) -> bool {
    definitions().is_ok_and(|definitions| definitions.contains_key(name))
}

pub(super) fn render(
    name: &str,
    width: f64,
    height: f64,
    adjustments: &HashMap<String, f64>,
) -> Result<CustomGeomSvg, String> {
    let definition = definitions()?
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
        environment.insert(&adjustment.name, value);
    }
    for guide in &definition.guides {
        let value = environment.evaluate(&guide.formula)?;
        environment.insert(&guide.name, value);
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

fn definitions() -> Result<&'static HashMap<String, PresetDefinition>, String> {
    DEFINITIONS
        .get_or_init(parse_definitions)
        .as_ref()
        .map_err(Clone::clone)
}

#[cfg(test)]
mod tests {
    use super::{definitions, render};
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
            let svg = render(name, 160.0, 100.0, &HashMap::new()).expect("official SVG");
            assert!(svg.paths.iter().all(|path| !path.d.is_empty()));
        }
    }
}
