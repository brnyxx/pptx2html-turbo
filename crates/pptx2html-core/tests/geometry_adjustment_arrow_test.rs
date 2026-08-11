mod fixtures;

use fixtures::MinimalPptx;
use pptx2html_core::convert_bytes;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

const OFFICIAL_PRESET_COUNT: usize = 55;
const OFFICIAL_PAIR_COUNT: usize = 189;
const PREVIOUSLY_UNCONSUMED_COUNT: usize = 100;

const PRESETS: &[&str] = &[
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

const OPEN_PRESETS: &[&str] = &[
    "curvedConnector2",
    "curvedConnector3",
    "curvedConnector4",
    "curvedConnector5",
    "bentConnector2",
    "bentConnector3",
    "bentConnector4",
    "bentConnector5",
];

#[derive(Clone, Copy, Debug)]
struct Case<'a> {
    preset: &'a str,
    key: &'a str,
    default: f64,
    lower: f64,
    upper: f64,
}

fn json_array<'a>(source: &'a str, key: &str) -> &'a str {
    let field = format!(r#""{key}""#);
    let field_start = source.find(&field).expect("JSON field");
    let array_start = source[field_start + field.len()..]
        .find('[')
        .map(|offset| field_start + field.len() + offset)
        .expect("JSON array");
    let mut depth = 0;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, character) in source[array_start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return &source[array_start + 1..array_start + offset];
                }
            }
            _ => {}
        }
    }
    panic!("unterminated JSON array: {key}");
}

fn json_objects(array: &str) -> Vec<&str> {
    let mut objects = Vec::new();
    let mut start = None;
    let mut depth = 0;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, character) in array.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start = Some(offset);
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    objects.push(&array[start.expect("object start")..=offset]);
                }
            }
            _ => {}
        }
    }
    objects
}

fn json_string_field<'a>(source: &'a str, key: &str) -> &'a str {
    let field = format!(r#""{key}""#);
    let field_start = source.find(&field).expect("JSON string field");
    let value_source = &source[field_start + field.len()..];
    let value_start = value_source.find('"').expect("JSON string value") + 1;
    let value_end = value_source[value_start..]
        .find('"')
        .expect("JSON string end")
        + value_start;
    &value_source[value_start..value_end]
}

fn numeric_formula(formula: &str) -> Option<f64> {
    formula.strip_prefix("val ")?.parse().ok()
}

fn official_cases() -> Vec<Case<'static>> {
    let manifest = include_str!("../../../evaluate/preset_adjustments.json");
    let presets = PRESETS.iter().copied().collect::<BTreeSet<_>>();
    let mut cases = Vec::new();
    for row in json_objects(json_array(manifest, "presets")) {
        let preset = json_string_field(row, "name");
        if !presets.contains(preset) {
            continue;
        }
        for adjustment in json_objects(json_array(row, "adjustments")) {
            let key = json_string_field(adjustment, "name");
            let default = numeric_formula(json_string_field(adjustment, "default_formula"))
                .expect("val default");
            let constraints = json_objects(json_array(adjustment, "constraints"));
            let numeric_bounds = constraints.first().and_then(|constraint| {
                let minimum = json_string_field(constraint, "minimum_formula")
                    .parse()
                    .ok()?;
                let maximum = json_string_field(constraint, "maximum_formula")
                    .parse()
                    .ok()?;
                Some((minimum, maximum))
            });
            let (lower, upper) = numeric_bounds.unwrap_or_else(|| {
                let span = default.abs().max(10_000.0) / 2.0;
                (default - span, default + span)
            });
            cases.push(Case {
                preset,
                key,
                default,
                lower,
                upper,
            });
        }
    }
    cases
}

fn render_html(
    preset: &str,
    adjustments: &[(&str, f64)],
    width_emu: i64,
    height_emu: i64,
) -> String {
    let adjustment_xml = adjustments
        .iter()
        .map(|(key, value)| format!(r#"<a:gd name="{key}" fmla="val {value}"/>"#))
        .collect::<String>();
    let slide = format!(
        r#"<p:sp>
  <p:nvSpPr><p:cNvPr id="2" name="Adjusted shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="{width_emu}" cy="{height_emu}"/></a:xfrm>
    <a:prstGeom prst="{preset}"><a:avLst>{adjustment_xml}</a:avLst></a:prstGeom>
    <a:solidFill><a:srgbClr val="336699"/></a:solidFill><a:ln w="12700"><a:solidFill><a:srgbClr val="112233"/></a:solidFill></a:ln>
  </p:spPr></p:sp>"#
    );
    convert_bytes(&MinimalPptx::new(&slide).build()).expect("convert adjusted arrow")
}

fn svg_paths(html: &str) -> Vec<&str> {
    html.match_indices("<path ")
        .map(|(start, _)| {
            let element = &html[start..html[start..].find('>').expect("path end") + start];
            let d_start = element.find("d=\"").expect("path d") + 3;
            let d_end = element[d_start..].find('"').expect("path d end") + d_start;
            &element[d_start..d_end]
        })
        .collect()
}

fn path_numbers(path: &str) -> Vec<f64> {
    path.split(|character: char| {
        character.is_ascii_alphabetic() || character == ',' || character.is_whitespace()
    })
    .filter(|token| !token.is_empty())
    .map(|token| token.parse::<f64>().expect("SVG number"))
    .collect()
}

fn assert_finite_html(html: &str, label: &str) {
    assert!(
        !html.contains("NaN") && !html.to_ascii_lowercase().contains("inf"),
        "{label}: non-finite SVG"
    );
    let paths = svg_paths(html);
    assert!(!paths.is_empty(), "{label}: no path");
    for path in paths {
        let numbers = path_numbers(path);
        assert!(!numbers.is_empty(), "{label}: empty numeric path");
        assert!(
            numbers.iter().all(|value| value.is_finite()),
            "{label}: non-finite number"
        );
    }
}

#[test]
fn official_arrow_table_is_independently_joined_to_all_routes() {
    let cases = official_cases();
    assert_eq!(PRESETS.len(), OFFICIAL_PRESET_COUNT);
    assert_eq!(cases.len(), OFFICIAL_PAIR_COUNT);
    assert_eq!(
        cases
            .iter()
            .map(|case| (case.preset, case.key))
            .collect::<BTreeSet<_>>()
            .len(),
        OFFICIAL_PAIR_COUNT
    );

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new("python3")
        .current_dir(repo_root)
        .args([
            "evaluate/check_preset_adjustments.py",
            "--repo-root",
            ".",
            "--bundle",
            "arrows",
        ])
        .output()
        .expect("run checker");
    assert!(
        output.status.success(),
        "checker: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("checker UTF-8");
    assert!(stdout.contains("presets=55"), "{stdout}");
    assert!(
        stdout.contains("manifest_keys_never_consumed=0"),
        "baseline had {PREVIOUSLY_UNCONSUMED_COUNT} unconsumed pairs: {stdout}"
    );
}

#[test]
fn every_official_key_has_defaults_bounds_representatives_and_three_midpoints() {
    for case in official_cases() {
        let absent = render_html(case.preset, &[], 1_524_000, 952_500);
        let explicit = render_html(case.preset, &[(case.key, case.default)], 1_524_000, 952_500);
        assert_eq!(
            svg_paths(&absent),
            svg_paths(&explicit),
            "{}.{} absent default",
            case.preset,
            case.key
        );

        let values = [
            case.lower,
            case.upper,
            (case.lower + case.upper) / 2.0,
            case.lower * 0.75 + case.upper * 0.25,
            case.lower * 0.55 + case.upper * 0.45,
            case.lower * 0.25 + case.upper * 0.75,
        ];
        let rendered =
            values.map(|value| render_html(case.preset, &[(case.key, value)], 1_524_000, 952_500));
        for (value, html) in values.into_iter().zip(&rendered) {
            assert_finite_html(html, &format!("{}.{}={value}", case.preset, case.key));
        }
        assert!(
            rendered
                .windows(2)
                .any(|pair| svg_paths(&pair[0]) != svg_paths(&pair[1])),
            "{}.{} is ignored or quantized",
            case.preset,
            case.key
        );
    }
}

#[test]
fn hostile_values_and_degenerate_extents_are_finite_for_every_key() {
    let extents = [
        (0, 952_500),
        (1_524_000, 0),
        (0, 0),
        (1, 1),
        (i64::MAX, 1),
        (1, i64::MAX),
        (i64::MAX, i64::MAX),
    ];
    for case in official_cases() {
        let default = render_html(case.preset, &[(case.key, case.default)], 1_524_000, 952_500);
        for hostile in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let html = render_html(case.preset, &[(case.key, hostile)], 1_524_000, 952_500);
            assert_eq!(
                svg_paths(&html),
                svg_paths(&default),
                "{}.{} non-finite default",
                case.preset,
                case.key
            );
            assert_finite_html(&html, &format!("{}.{} hostile", case.preset, case.key));
        }
        for extreme in [f64::MAX, -f64::MAX] {
            assert_finite_html(
                &render_html(case.preset, &[(case.key, extreme)], 1_524_000, 952_500),
                &format!("{}.{} extreme", case.preset, case.key),
            );
        }
        for (width, height) in extents {
            assert_finite_html(
                &render_html(case.preset, &[(case.key, case.default)], width, height),
                &format!("{}.{} {width}x{height}", case.preset, case.key),
            );
        }
    }
}

#[test]
fn every_multi_key_preset_is_order_independent_and_key_isolated() {
    let mut grouped: BTreeMap<&str, Vec<Case<'_>>> = BTreeMap::new();
    for case in official_cases() {
        grouped.entry(case.preset).or_default().push(case);
    }
    for (preset, cases) in grouped.into_iter().filter(|(_, cases)| cases.len() > 1) {
        let base = cases
            .iter()
            .map(|case| (case.key, case.default))
            .collect::<Vec<_>>();
        let forward = render_html(preset, &base, 1_524_000, 952_500);
        let reversed = base.iter().copied().rev().collect::<Vec<_>>();
        assert_eq!(
            svg_paths(&forward),
            svg_paths(&render_html(preset, &reversed, 1_524_000, 952_500)),
            "{preset} order"
        );
        for (index, case) in cases.iter().enumerate() {
            let mut changed = base.clone();
            changed[index].1 = if (case.lower - case.default).abs() > 0.5 {
                case.lower
            } else {
                case.upper
            };
            assert_ne!(
                svg_paths(&forward),
                svg_paths(&render_html(preset, &changed, 1_524_000, 952_500)),
                "{}.{} isolation",
                preset,
                case.key
            );
        }
    }
}

#[test]
fn mirrored_arrows_have_opposite_orientation_with_matching_topology() {
    for (left, right, adjustments, left_topology, right_topology) in [
        (
            "leftArrow",
            "rightArrow",
            vec![("adj1", 30_000.0), ("adj2", 40_000.0)],
            "MLLLLLLZ",
            "MLLLLLLZ",
        ),
        (
            "curvedLeftArrow",
            "curvedRightArrow",
            vec![("adj1", 20_000.0), ("adj2", 45_000.0), ("adj3", 30_000.0)],
            "MLLAALZ",
            "MALLLLAZ",
        ),
        (
            "leftArrowCallout",
            "rightArrowCallout",
            vec![
                ("adj1", 20_000.0),
                ("adj2", 15_000.0),
                ("adj3", 30_000.0),
                ("adj4", 55_000.0),
            ],
            "MLLLLLLLLLLZ",
            "MLLLLLLLLLLZ",
        ),
    ] {
        let left_html = render_html(left, &adjustments, 1_524_000, 952_500);
        let right_html = render_html(right, &adjustments, 1_524_000, 952_500);
        let left_path = svg_paths(&left_html)[0].to_owned();
        let right_path = svg_paths(&right_html)[0].to_owned();
        assert_ne!(left_path, right_path, "{left}/{right} orientation");
        let topology = |path: &str| {
            path.chars()
                .filter(|character| matches!(character, 'M' | 'L' | 'C' | 'Q' | 'A' | 'Z'))
                .collect::<String>()
        };
        assert_eq!(
            topology(&left_path),
            left_topology,
            "{left} official topology"
        );
        assert_eq!(
            topology(&right_path),
            right_topology,
            "{right} official topology"
        );
    }
}

#[test]
fn shape_paths_are_closed_connectors_are_open_and_end_at_bounds() {
    for preset in PRESETS {
        let html = render_html(preset, &[], 1_524_000, 952_500);
        let paths = svg_paths(&html);
        let main = paths.first().expect("main path");
        if OPEN_PRESETS.contains(preset) {
            assert!(!main.trim_end().ends_with('Z'), "{preset} connector closed");
            let numbers = path_numbers(main);
            assert_eq!(&numbers[..2], &[0.0, 0.0], "{preset} start");
            assert_eq!(
                &numbers[numbers.len() - 2..],
                &[160.0, 100.0],
                "{preset} end"
            );
        } else {
            assert!(main.contains('Z'), "{preset} main path open: {main}");
        }
    }
}

#[test]
fn curved_arrow_multi_paths_preserve_fill_and_stroke_roles() {
    let html = render_html(
        "curvedLeftArrow",
        &[("adj1", 12_000.0), ("adj2", 70_000.0), ("adj3", 18_000.0)],
        1_524_000,
        952_500,
    );
    assert!(
        svg_paths(&html).len() >= 2,
        "curved arrow lost layered paths"
    );
    assert!(html.contains("fill=\"#"), "main fill missing");
    assert!(html.contains("stroke=\"#"), "stroke role missing");
}

#[test]
fn official_formula_landmarks_and_coupled_constraints_hold() {
    let right_html = render_html(
        "rightArrow",
        &[("adj1", 50_000.0), ("adj2", 50_000.0)],
        1_524_000,
        952_500,
    );
    let right = path_numbers(svg_paths(&right_html)[0]);
    assert_eq!(
        right,
        [
            0.0, 25.0, 110.0, 25.0, 110.0, 0.0, 160.0, 50.0, 110.0, 100.0, 110.0, 75.0, 0.0, 75.0
        ]
    );

    for preset in [
        "rightArrow",
        "leftArrow",
        "downArrow",
        "leftRightArrow",
        "upDownArrow",
    ] {
        let clamped = render_html(
            preset,
            &[("adj1", 80_000.0), ("adj2", f64::MAX)],
            1_524_000,
            952_500,
        );
        let numbers = path_numbers(svg_paths(&clamped)[0]);
        assert!(
            numbers.iter().all(|value| (0.0..=160.0).contains(value)),
            "{preset} coupled max escaped bounds"
        );
    }

    let connector_html = render_html(
        "bentConnector5",
        &[("adj1", 20_000.0), ("adj2", 30_000.0), ("adj3", 70_000.0)],
        1_524_000,
        952_500,
    );
    let connector = path_numbers(svg_paths(&connector_html)[0]);
    assert_eq!(&connector[..4], &[0.0, 0.0, 32.0, 0.0]);
    assert_eq!(&connector[connector.len() - 2..], &[160.0, 100.0]);
}

#[test]
fn microsoft_up_arrow_definition_covers_both_supplemental_handles() {
    let absent = render_html("upArrow", &[], 1_524_000, 952_500);
    let explicit = render_html(
        "upArrow",
        &[("adj1", 50_000.0), ("adj2", 50_000.0)],
        1_524_000,
        952_500,
    );
    assert_eq!(svg_paths(&absent), svg_paths(&explicit));
    for key in ["adj1", "adj2"] {
        let variants = [0.0, 12_500.0, 25_000.0, 37_500.0, 75_000.0, 100_000.0]
            .map(|value| render_html("upArrow", &[(key, value)], 1_524_000, 952_500));
        assert!(
            variants
                .windows(2)
                .all(|pair| svg_paths(&pair[0]) != svg_paths(&pair[1])),
            "upArrow.{key} must follow the accepted official definition continuously"
        );
        for html in variants {
            assert_finite_html(&html, &format!("upArrow.{key}"));
        }
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let html = render_html("upArrow", &[(key, value)], 1_524_000, 952_500);
            assert_eq!(svg_paths(&html), svg_paths(&absent));
        }
    }
    let topology = svg_paths(&absent)[0]
        .chars()
        .filter(|character| matches!(character, 'M' | 'L' | 'C' | 'Q' | 'A' | 'Z'))
        .collect::<String>();
    assert_eq!(topology, "MLLLLLLZ");
}
