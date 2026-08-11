mod fixtures;

use fixtures::MinimalPptx;
use pptx2html_core::convert_bytes;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

const OFFICIAL_BASIC_PRESET_COUNT: usize = 69;
const OFFICIAL_BASIC_ADJUSTMENT_COUNT: usize = 46;

#[derive(Clone, Copy)]
struct AdjustmentCase {
    preset: &'static str,
    key: &'static str,
    default: f64,
    lower: f64,
    representative: f64,
    upper: f64,
    constrained: bool,
}

const CASES: &[AdjustmentCase] = &[
    case("roundRect", "adj", 16_667.0, 0.0, 25_000.0, 50_000.0, true),
    case("triangle", "adj", 50_000.0, 0.0, 25_000.0, 100_000.0, true),
    case(
        "parallelogram",
        "adj",
        25_000.0,
        0.0,
        75_000.0,
        160_000.0,
        true,
    ),
    case("trapezoid", "adj", 25_000.0, 0.0, 40_000.0, 80_000.0, true),
    case(
        "pentagon", "hf", 105_146.0, 90_000.0, 100_000.0, 120_000.0, false,
    ),
    case(
        "pentagon", "vf", 110_557.0, 90_000.0, 100_000.0, 125_000.0, false,
    ),
    case("hexagon", "adj", 25_000.0, 0.0, 40_000.0, 80_000.0, true),
    case(
        "hexagon", "vf", 115_470.0, 90_000.0, 100_000.0, 130_000.0, false,
    ),
    case("octagon", "adj", 29_289.0, 0.0, 20_000.0, 50_000.0, true),
    case("snip1Rect", "adj", 16_667.0, 0.0, 25_000.0, 50_000.0, true),
    case(
        "snip2SameRect",
        "adj1",
        16_667.0,
        0.0,
        25_000.0,
        50_000.0,
        true,
    ),
    case("snip2SameRect", "adj2", 0.0, 0.0, 25_000.0, 50_000.0, true),
    case("snip2DiagRect", "adj1", 0.0, 0.0, 25_000.0, 50_000.0, true),
    case(
        "snip2DiagRect",
        "adj2",
        16_667.0,
        0.0,
        25_000.0,
        50_000.0,
        true,
    ),
    case(
        "snipRoundRect",
        "adj1",
        16_667.0,
        0.0,
        25_000.0,
        50_000.0,
        true,
    ),
    case(
        "snipRoundRect",
        "adj2",
        16_667.0,
        0.0,
        25_000.0,
        50_000.0,
        true,
    ),
    case("round1Rect", "adj", 16_667.0, 0.0, 25_000.0, 50_000.0, true),
    case(
        "round2SameRect",
        "adj1",
        16_667.0,
        0.0,
        25_000.0,
        50_000.0,
        true,
    ),
    case("round2SameRect", "adj2", 0.0, 0.0, 25_000.0, 50_000.0, true),
    case(
        "round2DiagRect",
        "adj1",
        16_667.0,
        0.0,
        25_000.0,
        50_000.0,
        true,
    ),
    case("round2DiagRect", "adj2", 0.0, 0.0, 25_000.0, 50_000.0, true),
    case(
        "foldedCorner",
        "adj",
        16_667.0,
        0.0,
        25_000.0,
        50_000.0,
        true,
    ),
    case(
        "diagStripe",
        "adj",
        50_000.0,
        0.0,
        25_000.0,
        100_000.0,
        true,
    ),
    case("corner", "adj1", 50_000.0, 0.0, 75_000.0, 100_000.0, true),
    case("corner", "adj2", 50_000.0, 0.0, 75_000.0, 160_000.0, true),
    case("plaque", "adj", 16_667.0, 0.0, 25_000.0, 50_000.0, true),
    case("bracePair", "adj", 8_333.0, 0.0, 12_500.0, 25_000.0, true),
    case(
        "bracketPair",
        "adj",
        16_667.0,
        0.0,
        25_000.0,
        50_000.0,
        true,
    ),
    case("halfFrame", "adj1", 33_333.0, 0.0, 50_000.0, 79_166.0, true),
    case(
        "halfFrame",
        "adj2",
        33_333.0,
        0.0,
        75_000.0,
        160_000.0,
        true,
    ),
    case("leftBrace", "adj1", 8_333.0, 0.0, 12_500.0, 25_000.0, true),
    case(
        "leftBrace",
        "adj2",
        50_000.0,
        0.0,
        25_000.0,
        100_000.0,
        true,
    ),
    case("rightBrace", "adj1", 8_333.0, 0.0, 12_500.0, 25_000.0, true),
    case(
        "rightBrace",
        "adj2",
        50_000.0,
        0.0,
        25_000.0,
        100_000.0,
        true,
    ),
    case("leftBracket", "adj", 8_333.0, 0.0, 25_000.0, 50_000.0, true),
    case(
        "rightBracket",
        "adj",
        8_333.0,
        0.0,
        25_000.0,
        50_000.0,
        true,
    ),
    case(
        "horizontalScroll",
        "adj",
        12_500.0,
        0.0,
        6_250.0,
        25_000.0,
        true,
    ),
    case(
        "verticalScroll",
        "adj",
        12_500.0,
        0.0,
        6_250.0,
        25_000.0,
        true,
    ),
    case(
        "ellipseRibbon",
        "adj1",
        25_000.0,
        0.0,
        50_000.0,
        100_000.0,
        true,
    ),
    case(
        "ellipseRibbon",
        "adj2",
        50_000.0,
        25_000.0,
        37_500.0,
        75_000.0,
        true,
    ),
    case(
        "ellipseRibbon",
        "adj3",
        12_500.0,
        0.0,
        6_250.0,
        25_000.0,
        true,
    ),
    case(
        "ellipseRibbon2",
        "adj1",
        25_000.0,
        0.0,
        50_000.0,
        100_000.0,
        true,
    ),
    case(
        "ellipseRibbon2",
        "adj2",
        50_000.0,
        25_000.0,
        62_500.0,
        100_000.0,
        true,
    ),
    case(
        "ellipseRibbon2",
        "adj3",
        12_500.0,
        0.0,
        6_250.0,
        25_000.0,
        true,
    ),
    case(
        "nonIsoscelesTrapezoid",
        "adj1",
        25_000.0,
        0.0,
        40_000.0,
        80_000.0,
        true,
    ),
    case(
        "nonIsoscelesTrapezoid",
        "adj2",
        25_000.0,
        0.0,
        40_000.0,
        80_000.0,
        true,
    ),
];

const PREVIOUSLY_IGNORED: &[(&str, &str)] = &[
    ("triangle", "adj"),
    ("pentagon", "hf"),
    ("pentagon", "vf"),
    ("hexagon", "vf"),
    ("round2SameRect", "adj1"),
    ("round2SameRect", "adj2"),
    ("round2DiagRect", "adj1"),
    ("round2DiagRect", "adj2"),
    ("snipRoundRect", "adj1"),
    ("snipRoundRect", "adj2"),
    ("snip2SameRect", "adj1"),
    ("snip2SameRect", "adj2"),
    ("snip2DiagRect", "adj1"),
    ("snip2DiagRect", "adj2"),
];

const fn case(
    preset: &'static str,
    key: &'static str,
    default: f64,
    lower: f64,
    representative: f64,
    upper: f64,
    constrained: bool,
) -> AdjustmentCase {
    AdjustmentCase {
        preset,
        key,
        default,
        lower,
        representative,
        upper,
        constrained,
    }
}

fn render_path(preset: &str, adjustment: Option<(&str, f64)>) -> String {
    let adjustments = adjustment.map_or_else(Vec::new, |value| vec![value]);
    render_path_with(preset, &adjustments, 1_524_000, 952_500)
}

fn render_path_with(
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
  <p:spPr>
    <a:xfrm><a:off x="0" y="0"/><a:ext cx="{width_emu}" cy="{height_emu}"/></a:xfrm>
    <a:prstGeom prst="{preset}"><a:avLst>{adjustment_xml}</a:avLst></a:prstGeom>
    <a:solidFill><a:srgbClr val="336699"/></a:solidFill>
  </p:spPr>
</p:sp>"#
    );
    let html = convert_bytes(&MinimalPptx::new(&slide).build()).expect("convert adjusted shape");
    let path_start = html.find("<path d=\"").expect("rendered SVG path") + 9;
    let path_end = html[path_start..]
        .find('"')
        .map(|offset| path_start + offset)
        .expect("SVG path terminator");
    html[path_start..path_end].to_string()
}

fn path_numbers(path: &str) -> Vec<f64> {
    path.split(|character: char| {
        character.is_ascii_alphabetic() || character == ',' || character.is_whitespace()
    })
    .filter(|token| !token.is_empty())
    .map(|token| token.parse::<f64>().expect("SVG numeric token"))
    .collect()
}

fn assert_approx(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 0.11,
        "expected {expected}, got {actual}"
    );
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
                    objects.push(&array[start.expect("JSON object start")..=offset]);
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
        .map(|offset| value_start + offset)
        .expect("JSON string end");
    &value_source[value_start..value_end]
}

fn assert_finite_path(path: &str, case: AdjustmentCase, value: f64) {
    assert!(
        !path.contains("NaN") && !path.to_ascii_lowercase().contains("inf"),
        "{} {} produced non-finite SVG for {value}: {path}",
        case.preset,
        case.key
    );
    let mut number_count = 0;
    for token in path.split(|character: char| {
        character.is_ascii_alphabetic() || character == ',' || character.is_whitespace()
    }) {
        if token.is_empty() {
            continue;
        }
        let number = token.parse::<f64>().expect("SVG numeric token");
        assert!(
            number.is_finite(),
            "{} {} has {number}",
            case.preset,
            case.key
        );
        number_count += 1;
    }
    assert!(
        number_count > 0,
        "{} {} has no SVG numbers",
        case.preset,
        case.key
    );
}

#[test]
fn official_basic_adjustment_table_matches_task_two_bundle() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let checker = Command::new("python3")
        .current_dir(&repo_root)
        .args([
            "evaluate/check_preset_adjustments.py",
            "--repo-root",
            ".",
            "--bundle",
            "basic",
        ])
        .output()
        .expect("run Task 2 checker");
    assert!(
        checker.status.success(),
        "Task 2 checker failed: {}",
        String::from_utf8_lossy(&checker.stderr)
    );
    let checker_stdout = String::from_utf8(checker.stdout).expect("checker UTF-8");
    assert!(
        checker_stdout.contains(&format!("presets={OFFICIAL_BASIC_PRESET_COUNT}")),
        "unexpected checker inventory: {checker_stdout}"
    );
    assert!(checker_stdout.contains("manifest_keys_never_consumed=0"));

    let table_presets = CASES
        .iter()
        .map(|case| case.preset)
        .collect::<BTreeSet<_>>();
    let manifest = include_str!("../../../evaluate/preset_adjustments.json");
    let mut manifest_cases = BTreeMap::new();
    for row in json_objects(json_array(manifest, "presets")) {
        let preset = json_string_field(row, "name");
        if !table_presets.contains(preset) {
            continue;
        }
        for adjustment in json_objects(json_array(row, "adjustments")) {
            let key = json_string_field(adjustment, "name");
            let default_formula = json_string_field(adjustment, "default_formula");
            let constrained = !json_objects(json_array(adjustment, "constraints")).is_empty();
            manifest_cases.insert((preset, key), (default_formula, constrained));
        }
    }

    assert_eq!(manifest_cases.len(), OFFICIAL_BASIC_ADJUSTMENT_COUNT);
    assert_eq!(CASES.len(), manifest_cases.len());
    for case in CASES {
        let &(default_formula, constrained) = manifest_cases
            .get(&(case.preset, case.key))
            .unwrap_or_else(|| panic!("manifest missing {}.{}", case.preset, case.key));
        assert_eq!(
            default_formula,
            format!("val {:.0}", case.default),
            "{}.{} default",
            case.preset,
            case.key
        );
        assert_eq!(
            constrained, case.constrained,
            "{}.{} constraint",
            case.preset, case.key
        );
    }
}

#[test]
fn every_previously_ignored_key_is_observable() {
    // Given: every key the Task 2 checker found unconsumed before Task 8.
    let cases = PREVIOUSLY_IGNORED.iter().map(|&(preset, key)| {
        CASES
            .iter()
            .copied()
            .find(|case| case.preset == preset && case.key == key)
            .expect("ignored key exists in official table")
    });

    // When: each key is rendered at two valid official values.
    let unchanged = cases
        .filter_map(|case| {
            let lower = render_path(case.preset, Some((case.key, case.lower)));
            let upper = render_path(case.preset, Some((case.key, case.upper)));
            (lower == upper).then_some(format!("{}.{}", case.preset, case.key))
        })
        .collect::<Vec<_>>();

    // Then: the renderer consumes every formerly ignored key.
    assert!(
        unchanged.is_empty(),
        "still ignored: {}",
        unchanged.join(", ")
    );
}

#[test]
fn triangle_and_pentagon_dispatch_forward_adjustments() {
    // Given: the two basic shapes whose dispatcher previously dropped adjustments.
    let triangle_default = render_path("triangle", Some(("adj", 50_000.0)));
    let pentagon_default = render_path("pentagon", Some(("hf", 105_146.0)));

    // When: distinct valid values cross the public parser and dispatcher seam.
    let triangle_adjusted = render_path("triangle", Some(("adj", 25_000.0)));
    let pentagon_adjusted = render_path("pentagon", Some(("hf", 90_000.0)));

    // Then: both dispatcher calls forward their adjustment maps into geometry.
    assert_ne!(triangle_default, triangle_adjusted);
    assert_ne!(pentagon_default, pentagon_adjusted);
}

#[test]
fn every_official_basic_adjustment_changes_finite_parser_to_svg_geometry() {
    for &case in CASES {
        // Given: default, lower, upper, and representative official values.
        let unspecified = render_path(case.preset, None);

        // When: each value is parsed from a:avLst and rendered through the public API.
        let default = render_path(case.preset, Some((case.key, case.default)));
        let lower = render_path(case.preset, Some((case.key, case.lower)));
        let representative = render_path(case.preset, Some((case.key, case.representative)));
        let upper = render_path(case.preset, Some((case.key, case.upper)));

        // Then: all paths are finite and the key affects geometry.
        for (value, path) in [
            (case.default, &unspecified),
            (case.default, &default),
            (case.lower, &lower),
            (case.representative, &representative),
            (case.upper, &upper),
        ] {
            assert_finite_path(path, case, value);
        }
        assert_ne!(
            lower, upper,
            "{} {} did not affect the SVG path",
            case.preset, case.key
        );
    }
}

#[test]
fn hostile_and_out_of_contract_values_never_emit_non_finite_svg() {
    for &case in CASES {
        // Given: non-finite parser values and values beyond official handle constraints.
        let default = render_path(case.preset, Some((case.key, case.default)));

        // When: hostile values cross the parser-to-renderer boundary.
        for hostile in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let path = render_path(case.preset, Some((case.key, hostile)));

            // Then: non-finite values use the official default and remain finite.
            assert_eq!(
                path, default,
                "{} {} did not reject {hostile}",
                case.preset, case.key
            );
            assert_finite_path(&path, case, hostile);
        }

        if case.constrained {
            let below = render_path(case.preset, Some((case.key, case.lower - 1_000_000.0)));
            let above = render_path(case.preset, Some((case.key, case.upper + 1_000_000.0)));
            let lower = render_path(case.preset, Some((case.key, case.lower)));
            let upper = render_path(case.preset, Some((case.key, case.upper)));
            assert_eq!(below, lower, "{} {} lower clamp", case.preset, case.key);
            assert_eq!(above, upper, "{} {} upper clamp", case.preset, case.key);
        }
    }
}

#[test]
fn explicit_official_defaults_equal_absent_defaults() {
    for &case in CASES {
        let absent = render_path(case.preset, None);
        let explicit = render_path(case.preset, Some((case.key, case.default)));
        assert_eq!(
            absent, explicit,
            "{}.{} official default",
            case.preset, case.key
        );
    }
}

#[test]
fn official_axes_are_continuous_at_three_non_anchor_values() {
    let axes = [
        ("triangle", "adj", [31_111.0, 33_333.0, 35_555.0]),
        ("pentagon", "hf", [97_111.0, 99_333.0, 101_555.0]),
        ("pentagon", "vf", [97_111.0, 99_333.0, 101_555.0]),
        ("hexagon", "adj", [11_111.0, 12_222.0, 13_333.0]),
        ("hexagon", "vf", [101_111.0, 103_333.0, 105_555.0]),
        ("trapezoid", "adj", [11_111.0, 12_222.0, 13_333.0]),
        ("round1Rect", "adj", [11_111.0, 12_222.0, 13_333.0]),
        ("bracePair", "adj", [9_111.0, 10_222.0, 11_333.0]),
        ("bracketPair", "adj", [11_111.0, 12_222.0, 13_333.0]),
        ("halfFrame", "adj1", [21_111.0, 22_222.0, 23_333.0]),
        ("halfFrame", "adj2", [21_111.0, 22_222.0, 23_333.0]),
        ("leftBrace", "adj1", [7_111.0, 9_333.0, 11_555.0]),
        ("leftBrace", "adj2", [41_111.0, 53_333.0, 65_555.0]),
        ("rightBrace", "adj1", [7_111.0, 9_333.0, 11_555.0]),
        ("rightBrace", "adj2", [41_111.0, 53_333.0, 65_555.0]),
        ("snip2SameRect", "adj1", [11_111.0, 12_222.0, 13_333.0]),
        ("snip2SameRect", "adj2", [11_111.0, 12_222.0, 13_333.0]),
        ("snip2DiagRect", "adj1", [11_111.0, 12_222.0, 13_333.0]),
        ("snip2DiagRect", "adj2", [11_111.0, 12_222.0, 13_333.0]),
        ("snipRoundRect", "adj1", [11_111.0, 12_222.0, 13_333.0]),
        ("snipRoundRect", "adj2", [11_111.0, 12_222.0, 13_333.0]),
        ("round2SameRect", "adj1", [11_111.0, 12_222.0, 13_333.0]),
        ("round2SameRect", "adj2", [11_111.0, 12_222.0, 13_333.0]),
        ("round2DiagRect", "adj1", [11_111.0, 12_222.0, 13_333.0]),
        ("round2DiagRect", "adj2", [11_111.0, 12_222.0, 13_333.0]),
    ];
    let collapsed = axes
        .iter()
        .filter_map(|(preset, key, values)| {
            let paths = values.map(|value| render_path(preset, Some((key, value))));
            ((paths[0] == paths[1]) || (paths[1] == paths[2])).then_some(format!("{preset}.{key}"))
        })
        .collect::<Vec<_>>();
    assert!(
        collapsed.is_empty(),
        "quantized axes: {}",
        collapsed.join(", ")
    );
}

#[test]
fn every_multi_key_shape_preserves_order_and_key_isolation() {
    let cases = [
        (
            "pentagon",
            vec![("hf", 90_000.0), ("vf", 90_000.0)],
            vec![100_000.0, 100_000.0],
        ),
        (
            "hexagon",
            vec![("adj", 20_000.0), ("vf", 90_000.0)],
            vec![35_000.0, 110_000.0],
        ),
        (
            "snip2SameRect",
            vec![("adj1", 10_000.0), ("adj2", 20_000.0)],
            vec![30_000.0, 40_000.0],
        ),
        (
            "snip2DiagRect",
            vec![("adj1", 10_000.0), ("adj2", 20_000.0)],
            vec![30_000.0, 40_000.0],
        ),
        (
            "snipRoundRect",
            vec![("adj1", 10_000.0), ("adj2", 20_000.0)],
            vec![30_000.0, 40_000.0],
        ),
        (
            "round2SameRect",
            vec![("adj1", 10_000.0), ("adj2", 20_000.0)],
            vec![30_000.0, 40_000.0],
        ),
        (
            "round2DiagRect",
            vec![("adj1", 10_000.0), ("adj2", 20_000.0)],
            vec![30_000.0, 40_000.0],
        ),
        (
            "halfFrame",
            vec![("adj1", 20_000.0), ("adj2", 30_000.0)],
            vec![30_000.0, 40_000.0],
        ),
        (
            "leftBrace",
            vec![("adj1", 8_000.0), ("adj2", 40_000.0)],
            vec![12_000.0, 60_000.0],
        ),
        (
            "rightBrace",
            vec![("adj1", 8_000.0), ("adj2", 40_000.0)],
            vec![12_000.0, 60_000.0],
        ),
        (
            "ellipseRibbon",
            vec![("adj1", 20_000.0), ("adj2", 40_000.0), ("adj3", 10_000.0)],
            vec![40_000.0, 60_000.0, 15_000.0],
        ),
        (
            "ellipseRibbon2",
            vec![("adj1", 20_000.0), ("adj2", 40_000.0), ("adj3", 10_000.0)],
            vec![40_000.0, 60_000.0, 15_000.0],
        ),
        (
            "corner",
            vec![("adj1", 30_000.0), ("adj2", 40_000.0)],
            vec![40_000.0, 50_000.0],
        ),
        (
            "nonIsoscelesTrapezoid",
            vec![("adj1", 20_000.0), ("adj2", 30_000.0)],
            vec![30_000.0, 40_000.0],
        ),
    ];

    for (preset, base, alternatives) in cases {
        let forward = render_path_with(preset, &base, 1_524_000, 952_500);
        let reversed = base.iter().copied().rev().collect::<Vec<_>>();
        let reversed = render_path_with(preset, &reversed, 1_524_000, 952_500);
        assert_eq!(forward, reversed, "{preset} key order");

        for (index, (key, _)) in base.iter().enumerate() {
            let mut isolated = base.clone();
            isolated[index].1 = alternatives[index];
            let changed = render_path_with(preset, &isolated, 1_524_000, 952_500);
            assert_ne!(forward, changed, "{preset}.{key} isolation");
        }
    }

    for preset in [
        "snip2DiagRect",
        "snipRoundRect",
        "round2SameRect",
        "round2DiagRect",
    ] {
        let forward = render_path_with(
            preset,
            &[("adj1", 12_345.0), ("adj2", 32_109.0)],
            1_524_000,
            952_500,
        );
        let with_unknown = render_path_with(
            preset,
            &[("adj", 45_000.0), ("adj1", 12_345.0), ("adj2", 32_109.0)],
            1_524_000,
            952_500,
        );
        assert_eq!(forward, with_unknown, "{preset} legacy override");
    }
}

#[test]
fn coupled_constraints_change_only_at_official_bounds() {
    for preset in ["leftBrace", "rightBrace"] {
        let clamped = render_path_with(
            preset,
            &[("adj1", 40_000.0), ("adj2", 20_000.0)],
            952_500,
            952_500,
        );
        let boundary = render_path_with(
            preset,
            &[("adj1", 10_000.0), ("adj2", 20_000.0)],
            952_500,
            952_500,
        );
        assert_eq!(clamped, boundary, "{preset} dynamic maxAdj1");
    }

    for preset in ["ellipseRibbon", "ellipseRibbon2"] {
        let clamped = render_path_with(
            preset,
            &[("adj1", 40_000.0), ("adj2", 60_000.0), ("adj3", 0.0)],
            952_500,
            952_500,
        );
        let boundary = render_path_with(
            preset,
            &[("adj1", 40_000.0), ("adj2", 60_000.0), ("adj3", 10_000.0)],
            952_500,
            952_500,
        );
        assert_eq!(clamped, boundary, "{preset} adj3 lower bound from adj1");
    }

    let clamped = render_path_with(
        "halfFrame",
        &[("adj1", 90_000.0), ("adj2", 80_000.0)],
        952_500,
        952_500,
    );
    let boundary = render_path_with(
        "halfFrame",
        &[("adj1", 20_000.0), ("adj2", 80_000.0)],
        952_500,
        952_500,
    );
    assert_eq!(clamped, boundary, "halfFrame dynamic maxAdj1");
}

#[test]
fn brace_official_defaults_are_continuous_with_epsilon_neighbors() {
    for preset in ["leftBrace", "rightBrace"] {
        let default = render_path_with(
            preset,
            &[("adj1", 8_333.0), ("adj2", 50_000.0)],
            1_524_000,
            952_500,
        );
        for neighbor in [
            [("adj1", 8_332.0), ("adj2", 50_000.0)],
            [("adj1", 8_334.0), ("adj2", 50_000.0)],
            [("adj1", 8_333.0), ("adj2", 49_999.0)],
            [("adj1", 8_333.0), ("adj2", 50_001.0)],
        ] {
            let neighbor = render_path_with(preset, &neighbor, 1_524_000, 952_500);
            assert_eq!(default, neighbor, "{preset} default discontinuity");
        }
    }
}

#[test]
fn official_landmarks_match_ecma_derived_coordinates() {
    let triangle = path_numbers(&render_path("triangle", Some(("adj", 37_500.0))));
    assert_eq!(triangle, [0.0, 100.0, 60.0, 0.0, 160.0, 100.0]);

    let hexagon = path_numbers(&render_path_with(
        "hexagon",
        &[("adj", 33_333.0), ("vf", 100_000.0)],
        1_524_000,
        952_500,
    ));
    for (actual, expected) in hexagon.iter().zip([
        0.0, 50.0, 33.3, 6.7, 126.7, 6.7, 160.0, 50.0, 126.7, 93.3, 33.3, 93.3,
    ]) {
        assert_approx(*actual, expected);
    }

    let snip = path_numbers(&render_path_with(
        "snip2DiagRect",
        &[("adj1", 20_000.0), ("adj2", 30_000.0)],
        1_524_000,
        952_500,
    ));
    assert_eq!(
        snip,
        [
            20.0, 0.0, 130.0, 0.0, 160.0, 30.0, 160.0, 80.0, 140.0, 100.0, 30.0, 100.0, 0.0, 70.0,
            0.0, 20.0
        ]
    );

    let half_frame = path_numbers(&render_path_with(
        "halfFrame",
        &[("adj1", 20_000.0), ("adj2", 40_000.0)],
        1_524_000,
        952_500,
    ));
    assert_eq!(
        half_frame,
        [
            0.0, 0.0, 160.0, 0.0, 128.0, 20.0, 40.0, 20.0, 40.0, 75.0, 0.0, 100.0
        ]
    );
}

#[test]
fn every_multi_key_shape_matches_official_derived_landmarks() {
    let cases = [
        (
            "pentagon",
            vec![("hf", 100_000.0), ("vf", 100_000.0)],
            ["M3.9,34.5", "L127.0,90.5"],
        ),
        (
            "hexagon",
            vec![("adj", 20_000.0), ("vf", 90_000.0)],
            ["L20.0,11.0", "L140.0,89.0"],
        ),
        (
            "snip2SameRect",
            vec![("adj1", 10_000.0), ("adj2", 20_000.0)],
            ["M10.0,0", "L140.0,100.0"],
        ),
        (
            "snip2DiagRect",
            vec![("adj1", 10_000.0), ("adj2", 20_000.0)],
            ["M10.0,0", "L20.0,100.0"],
        ),
        (
            "snipRoundRect",
            vec![("adj1", 10_000.0), ("adj2", 20_000.0)],
            ["M10.0,0", "L140.0,0"],
        ),
        (
            "round2SameRect",
            vec![("adj1", 10_000.0), ("adj2", 20_000.0)],
            ["A10.0,10.0", "A20.0,20.0"],
        ),
        (
            "round2DiagRect",
            vec![("adj1", 10_000.0), ("adj2", 20_000.0)],
            ["A10.0,10.0", "A20.0,20.0"],
        ),
        (
            "halfFrame",
            vec![("adj1", 20_000.0), ("adj2", 30_000.0)],
            ["L128.0,20.0", "L30.0,81.2"],
        ),
        (
            "leftBrace",
            vec![("adj1", 8_333.0), ("adj2", 50_000.0)],
            ["A80.0,8.3", "L80.0,58.3"],
        ),
        (
            "rightBrace",
            vec![("adj1", 8_333.0), ("adj2", 50_000.0)],
            ["A80.0,8.3", "L80.0,41.7"],
        ),
        (
            "ellipseRibbon",
            vec![("adj1", 20_000.0), ("adj2", 40_000.0), ("adj3", 10_000.0)],
            ["M0,80.0", "L160.0,10.0"],
        ),
        (
            "ellipseRibbon2",
            vec![("adj1", 20_000.0), ("adj2", 40_000.0), ("adj3", 10_000.0)],
            ["M0,20.0", "L160.0,90.0"],
        ),
        (
            "corner",
            vec![("adj1", 30_000.0), ("adj2", 40_000.0)],
            ["L40.0,0", "L40.0,70.0"],
        ),
        (
            "nonIsoscelesTrapezoid",
            vec![("adj1", 20_000.0), ("adj2", 30_000.0)],
            ["M20.0,0", "L130.0,0"],
        ),
    ];

    for (preset, adjustments, landmarks) in cases {
        let path = render_path_with(preset, &adjustments, 1_524_000, 952_500);
        for landmark in landmarks {
            assert!(
                path.contains(landmark),
                "{preset}: missing {landmark} in {path}"
            );
        }
    }
}

#[test]
fn official_paths_keep_closed_topology_and_expected_arc_counts() {
    for (preset, adjustments, arc_count) in [
        ("round1Rect", vec![("adj", 23_456.0)], 1),
        (
            "round2SameRect",
            vec![("adj1", 12_345.0), ("adj2", 23_456.0)],
            4,
        ),
        (
            "round2DiagRect",
            vec![("adj1", 12_345.0), ("adj2", 23_456.0)],
            4,
        ),
        (
            "snipRoundRect",
            vec![("adj1", 12_345.0), ("adj2", 23_456.0)],
            1,
        ),
        ("bracePair", vec![("adj", 12_345.0)], 8),
        ("bracketPair", vec![("adj", 23_456.0)], 4),
        ("leftBrace", vec![("adj1", 8_333.0), ("adj2", 50_000.0)], 4),
        ("rightBrace", vec![("adj1", 8_333.0), ("adj2", 50_000.0)], 4),
    ] {
        let path = render_path_with(preset, &adjustments, 1_524_000, 952_500);
        assert_eq!(
            path.chars().filter(|command| *command == 'M').count(),
            1,
            "{preset}"
        );
        assert_eq!(
            path.chars().filter(|command| *command == 'A').count(),
            arc_count,
            "{preset}"
        );
        assert_eq!(
            path.chars().filter(|command| *command == 'Z').count(),
            1,
            "{preset}"
        );
        assert!(path.trim_end().ends_with('Z'), "{preset} closedness");
    }
}

#[test]
fn safe_official_values_remain_inside_square_view_box() {
    for (preset, adjustments) in [
        ("triangle", vec![("adj", 37_500.0)]),
        ("pentagon", vec![("hf", 100_000.0), ("vf", 100_000.0)]),
        ("hexagon", vec![("adj", 33_333.0), ("vf", 100_000.0)]),
        (
            "snip2DiagRect",
            vec![("adj1", 20_000.0), ("adj2", 30_000.0)],
        ),
        (
            "round2SameRect",
            vec![("adj1", 20_000.0), ("adj2", 30_000.0)],
        ),
        ("halfFrame", vec![("adj1", 20_000.0), ("adj2", 40_000.0)]),
        ("leftBrace", vec![("adj1", 8_333.0), ("adj2", 50_000.0)]),
        ("rightBrace", vec![("adj1", 8_333.0), ("adj2", 50_000.0)]),
    ] {
        let path = render_path_with(preset, &adjustments, 952_500, 952_500);
        assert!(
            path_numbers(&path)
                .into_iter()
                .all(|value| (0.0..=100.0).contains(&value)),
            "{preset} escaped square viewBox: {path}"
        );
    }
}
