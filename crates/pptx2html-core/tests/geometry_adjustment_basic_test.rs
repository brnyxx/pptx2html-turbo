mod fixtures;

use fixtures::MinimalPptx;
use pptx2html_core::convert_bytes;

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
    let adjustment_xml = adjustment.map_or_else(String::new, |(key, value)| {
        format!(r#"<a:gd name="{key}" fmla="val {value}"/>"#)
    });
    let slide = format!(
        r#"<p:sp>
  <p:nvSpPr><p:cNvPr id="2" name="Adjusted shape"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
  <p:spPr>
    <a:xfrm><a:off x="0" y="0"/><a:ext cx="1524000" cy="952500"/></a:xfrm>
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
    // Given: Task 2's ECMA-derived basic-family contract.
    let presets = CASES
        .iter()
        .map(|case| case.preset)
        .collect::<std::collections::HashSet<_>>();

    // When: the independently encoded adjustment table is counted.
    let adjustment_count = CASES.len();

    // Then: all 46 keys are represented and belong to the 69-preset basic bundle.
    assert_eq!(adjustment_count, OFFICIAL_BASIC_ADJUSTMENT_COUNT);
    assert!(presets.len() <= OFFICIAL_BASIC_PRESET_COUNT);
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
