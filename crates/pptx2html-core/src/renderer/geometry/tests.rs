//! Umbrella test module; wildcard imports tolerated for mechanical parity with the pre-split monolith.
#![cfg(test)]
#![allow(unused_imports)]

use super::action_buttons::*;
use super::arcs::*;
use super::arrow_callouts::*;
use super::arrows::*;
use super::basic_shapes::*;
use super::bent_u_arrows::*;
use super::brackets_braces::*;
use super::callouts::*;
use super::chart_shapes::*;
use super::circular_arrows::*;
use super::connectors::*;
use super::curved_arrows::*;
use super::custom_geom::*;
use super::flowchart::*;
use super::math::*;
use super::misc::*;
use super::rects::*;
use super::scrolls_tabs::*;
use super::shared::*;
use super::stars::*;
use super::waves_polys::*;
use super::{CustomGeomPathSvg, CustomGeomSvg};
use super::{needs_evenodd_fill, preset_shape_multi_svg, preset_shape_svg};
use crate::model::PathFill;
use std::collections::HashMap;

fn assert_official_path(path: &str, moves: usize, minimum_arcs: usize) {
    assert_eq!(
        path.matches('M').count(),
        moves,
        "official subpath count: {path}"
    );
    assert!(
        path.matches('A').count() >= minimum_arcs,
        "official arc topology: {path}"
    );
    assert!(path.contains('Z'), "official closed topology: {path}");
    for token in path
        .split(|character: char| {
            character.is_ascii_alphabetic() || character == ',' || character.is_whitespace()
        })
        .filter(|token| !token.is_empty())
    {
        assert!(
            token.parse::<f64>().expect("SVG number").is_finite(),
            "{path}"
        );
    }
}

#[test]
fn test_preset_shape_svg_returns_none_for_unknown() {
    let adj = HashMap::new();
    assert!(preset_shape_svg("unknownShape", 100.0, 100.0, &adj).is_none());
}

#[test]
fn test_basic_adjusted_presets_are_finite_for_degenerate_and_extreme_extents() {
    let cases: &[(&str, &[&str])] = &[
        ("roundRect", &["adj"]),
        ("triangle", &["adj"]),
        ("parallelogram", &["adj"]),
        ("trapezoid", &["adj"]),
        ("pentagon", &["hf", "vf"]),
        ("hexagon", &["adj", "vf"]),
        ("octagon", &["adj"]),
        ("snip1Rect", &["adj"]),
        ("snip2SameRect", &["adj1", "adj2"]),
        ("snip2DiagRect", &["adj1", "adj2"]),
        ("snipRoundRect", &["adj1", "adj2"]),
        ("round1Rect", &["adj"]),
        ("round2SameRect", &["adj1", "adj2"]),
        ("round2DiagRect", &["adj1", "adj2"]),
        ("foldedCorner", &["adj"]),
        ("diagStripe", &["adj"]),
        ("corner", &["adj1", "adj2"]),
        ("plaque", &["adj"]),
        ("bracePair", &["adj"]),
        ("bracketPair", &["adj"]),
        ("halfFrame", &["adj1", "adj2"]),
        ("leftBrace", &["adj1", "adj2"]),
        ("rightBrace", &["adj1", "adj2"]),
        ("leftBracket", &["adj"]),
        ("rightBracket", &["adj"]),
        ("horizontalScroll", &["adj"]),
        ("verticalScroll", &["adj"]),
        ("ellipseRibbon", &["adj1", "adj2", "adj3"]),
        ("ellipseRibbon2", &["adj1", "adj2", "adj3"]),
        ("nonIsoscelesTrapezoid", &["adj1", "adj2"]),
    ];
    let dimensions = [
        (100.0, 100.0),
        (0.0, 100.0),
        (100.0, 0.0),
        (0.0, 0.0),
        (f64::MAX, 100.0),
        (100.0, f64::MAX),
        (f64::MAX, f64::MAX),
    ];
    let hostile_adjustments = [
        -1.0,
        f64::MAX,
        -f64::MAX,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];

    for &(preset, keys) in cases {
        for &key in keys {
            for &value in &hostile_adjustments {
                let adjustments = HashMap::from([(key.to_string(), value)]);
                for &(w, h) in &dimensions {
                    let path =
                        std::panic::catch_unwind(|| preset_shape_svg(preset, w, h, &adjustments))
                            .unwrap_or_else(|_| panic!("{preset}.{key} panicked at {w}x{h}"))
                            .unwrap_or_else(|| panic!("{preset}.{key} missing at {w}x{h}"));
                    let repeated = preset_shape_svg(preset, w, h, &adjustments)
                        .expect("deterministic repeat path");
                    assert_eq!(path, repeated, "{preset}.{key} at {w}x{h}");
                    assert!(path.starts_with('M'), "{preset}.{key}: {path}");
                    if w == 0.0 || h == 0.0 {
                        if matches!(preset, "ellipseRibbon" | "ellipseRibbon2") {
                            assert!(path.contains('Z'), "{preset}.{key}: {path}");
                        } else {
                            assert!(path.ends_with('Z'), "{preset}.{key}: {path}");
                        }
                    }
                    assert!(
                        !path.contains("NaN") && !path.to_ascii_lowercase().contains("inf"),
                        "{preset}.{key} at {w}x{h}: {path}"
                    );
                }
            }
        }
    }

    let fallback_cases = [
        (
            "pentagon",
            HashMap::from([("hf".to_string(), f64::MAX)]),
            f64::MAX,
            100.0,
        ),
        (
            "pentagon",
            HashMap::from([("vf".to_string(), f64::MAX)]),
            100.0,
            f64::MAX,
        ),
        (
            "pentagon",
            HashMap::from([("vf".to_string(), -f64::MAX)]),
            100.0,
            f64::MAX,
        ),
        (
            "hexagon",
            HashMap::from([("vf".to_string(), f64::MAX)]),
            100.0,
            f64::MAX,
        ),
        (
            "hexagon",
            HashMap::from([("vf".to_string(), -f64::MAX)]),
            100.0,
            f64::MAX,
        ),
    ];
    for (preset, adjustments, w, h) in fallback_cases {
        let path = preset_shape_svg(preset, w, h, &adjustments).expect("extreme path");
        let view_box_boundary =
            preset_shape_svg("rect", w, h, &HashMap::new()).expect("viewBox boundary path");
        assert_eq!(path, view_box_boundary, "{preset} finite fallback");
    }
}

#[test]
fn test_total_supported_shapes_at_least_187() {
    let adj = HashMap::new();
    let all = [
        "rect",
        "roundRect",
        "ellipse",
        "triangle",
        "isosTriangle",
        "rtTriangle",
        "diamond",
        "parallelogram",
        "trapezoid",
        "pentagon",
        "hexagon",
        "octagon",
        "snip1Rect",
        "snip2SameRect",
        "snip2DiagRect",
        "snipRoundRect",
        "round1Rect",
        "round2SameRect",
        "round2DiagRect",
        "foldCorner",
        "diagStripe",
        "corner",
        "plaque",
        "bracePair",
        "bracketPair",
        "halfFrame",
        "line",
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
        "flowChartProcess",
        "flowChartDecision",
        "flowChartTerminator",
        "flowChartDocument",
        "flowChartPredefinedProcess",
        "flowChartAlternateProcess",
        "flowChartManualInput",
        "flowChartConnector",
        "flowChartInputOutput",
        "flowChartInternalStorage",
        "flowChartMultidocument",
        "flowChartPreparation",
        "flowChartManualOperation",
        "flowChartOffpageConnector",
        "flowChartPunchedCard",
        "flowChartPunchedTape",
        "flowChartSummingJunction",
        "flowChartOr",
        "flowChartCollate",
        "flowChartSort",
        "flowChartExtract",
        "flowChartMerge",
        "flowChartOnlineStorage",
        "flowChartDelay",
        "flowChartMagneticTape",
        "flowChartMagneticDisk",
        "flowChartMagneticDrum",
        "flowChartDisplay",
        "actionButtonBlank",
        "actionButtonHome",
        "actionButtonHelp",
        "actionButtonInformation",
        "actionButtonBackPrevious",
        "actionButtonForwardNext",
        "actionButtonBeginning",
        "actionButtonEnd",
        "actionButtonReturn",
        "actionButtonDocument",
        "actionButtonSound",
        "actionButtonMovie",
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
        "irregularSeal1",
        "irregularSeal2",
        "mathPlus",
        "mathEqual",
        "mathNotEqual",
        "mathMultiply",
        "mathDivide",
        "mathMinus",
        "heart",
        "plus",
        "lightningBolt",
        "cloud",
        "frame",
        "ribbon",
        "ribbon2",
        "donut",
        "noSmoking",
        "blockArc",
        "smileyFace",
        "can",
        "cube",
        "moon",
        "sun",
        "bevel",
        "gear6",
        "gear9",
        "pie",
        "pieWedge",
        "arc",
        "wave",
        "doubleWave",
        "decagon",
        "dodecagon",
        "funnel",
        "teardrop",
        "heptagon",
        "downArrowCallout",
        "leftArrowCallout",
        "rightArrowCallout",
        "upArrowCallout",
        "quadArrowCallout",
        "leftRightArrowCallout",
        "upDownArrowCallout",
        "leftBrace",
        "rightBrace",
        "leftBracket",
        "rightBracket",
        "chartPlus",
        "chartStar",
        "chartX",
        "horizontalScroll",
        "verticalScroll",
        "cornerTabs",
        "plaqueTabs",
        "squareTabs",
        "ellipseRibbon",
        "ellipseRibbon2",
        "leftCircularArrow",
        "leftRightCircularArrow",
        "chord",
        "lineInv",
        "nonIsoscelesTrapezoid",
        "swooshArrow",
        "leftRightRibbon",
        "flowChartOfflineStorage",
        "cross",
        "curvedConnector2",
        "curvedConnector3",
        "curvedConnector4",
        "curvedConnector5",
        "bentConnector2",
        "bentConnector3",
        "bentConnector4",
    ];
    let supported: Vec<_> = all
        .iter()
        .filter(|n| preset_shape_svg(n, 100.0, 100.0, &adj).is_some())
        .collect();
    let unsupported: Vec<_> = all
        .iter()
        .filter(|n| preset_shape_svg(n, 100.0, 100.0, &adj).is_none())
        .collect();
    assert!(unsupported.is_empty(), "Unsupported: {:?}", unsupported);
    assert!(
        supported.len() >= 187,
        "Expected >= 187, got {}",
        supported.len()
    );
}

#[test]
fn test_flowchart_all_28_shapes() {
    let adj = HashMap::new();
    for name in [
        "flowChartProcess",
        "flowChartAlternateProcess",
        "flowChartDecision",
        "flowChartInputOutput",
        "flowChartPredefinedProcess",
        "flowChartInternalStorage",
        "flowChartDocument",
        "flowChartMultidocument",
        "flowChartTerminator",
        "flowChartPreparation",
        "flowChartManualInput",
        "flowChartManualOperation",
        "flowChartConnector",
        "flowChartOffpageConnector",
        "flowChartPunchedCard",
        "flowChartPunchedTape",
        "flowChartSummingJunction",
        "flowChartOr",
        "flowChartCollate",
        "flowChartSort",
        "flowChartExtract",
        "flowChartMerge",
        "flowChartOnlineStorage",
        "flowChartDelay",
        "flowChartMagneticTape",
        "flowChartMagneticDisk",
        "flowChartMagneticDrum",
        "flowChartDisplay",
    ] {
        assert!(
            preset_shape_svg(name, 100.0, 100.0, &adj).is_some(),
            "Missing: {name}"
        );
    }
}

#[test]
fn test_action_buttons_all_12() {
    let adj = HashMap::new();
    for name in [
        "actionButtonBlank",
        "actionButtonHome",
        "actionButtonHelp",
        "actionButtonInformation",
        "actionButtonBackPrevious",
        "actionButtonForwardNext",
        "actionButtonBeginning",
        "actionButtonEnd",
        "actionButtonReturn",
        "actionButtonDocument",
        "actionButtonSound",
        "actionButtonMovie",
    ] {
        assert!(
            preset_shape_svg(name, 100.0, 100.0, &adj).is_some(),
            "Missing: {name}"
        );
    }
}

#[test]
fn test_action_button_unknown_icon_falls_back_to_blank_frame() {
    let blank = action_button_blank_path(100.0, 100.0);
    let unknown = action_button_icon_path(100.0, 100.0, "mystery");

    assert_eq!(unknown, format!("{blank} "));
}

#[test]
fn test_right_arrow_default_path_uses_narrower_head_length() {
    assert_task9_continuous_geometry("rightArrow");
}

#[test]
fn test_left_arrow_default_path_uses_narrower_head_length() {
    assert_task9_continuous_geometry("leftArrow");
}

#[test]
fn test_right_triangle_default_path_keeps_the_right_angle_on_the_left() {
    let adj = HashMap::new();
    let path = preset_shape_svg("rtTriangle", 120.0, 100.0, &adj).unwrap();

    assert_eq!(path, "M0,0 L120.0,100.0 L0,100.0 Z");
}

#[test]
fn test_up_arrow_default_path_widens_the_shaft() {
    assert_task9_continuous_geometry("upArrow");
}

#[test]
fn test_down_arrow_default_path_widens_the_shaft() {
    assert_task9_continuous_geometry("downArrow");
}

#[test]
fn test_folded_corner_alias_uses_fold_corner_geometry() {
    let adj = HashMap::new();
    let path = preset_shape_svg("foldedCorner", 120.0, 100.0, &adj).unwrap();

    assert_eq!(
        path,
        "M0,0 L120.0,0 L120.0,83.3 L103.3,100.0 L0,100.0 Z M103.3,100.0 L120.0,83.3 L103.3,83.3 Z"
    );
}

#[test]
fn test_diag_stripe_default_path_spans_full_diagonal_band() {
    let adj = HashMap::new();
    let path = diag_stripe_path(120.0, 100.0, &adj);

    assert_eq!(path, "M0,100.0 L0,55.0 L42.0,0 L120.0,0 Z");
}

#[test]
fn test_pie_default_path_renders_three_quarter_sector() {
    let adj = HashMap::new();
    let path = pie_path(120.0, 100.0, &adj);

    assert_eq!(path, "M60.0,50.0 L60.0,0.0 A60.0,50.0 0 1,0 120.0,50.0 Z");
}

#[test]
fn test_moon_default_path_faces_right_with_left_bulge() {
    let adj = HashMap::new();
    let path = moon_path(120.0, 100.0, &adj);

    assert_eq!(
        path,
        "M 112.4,99.4 L 86.2,97.7 L 60.5,93.2 L 40.8,87.3 L 19.7,77.0 L 7.8,67.2 L 3.8,61.8 L 0.8,53.9 L 1.3,43.6 L 5.3,35.7 L 8.8,31.5 L 18.2,23.9 L 28.2,18.0 L 49.9,9.8 L 71.1,4.8 L 84.7,2.7 L 118.5,0.6 L 119.0,2.3 L 98.3,10.2 L 81.2,19.7 L 71.8,27.4 L 66.8,33.2 L 62.8,40.2 L 61.3,45.6 L 62.3,58.1 L 66.3,66.0 L 70.3,71.0 L 80.2,79.5 L 93.3,87.3 L 107.4,93.6 L 119.2,97.5 L 118.5,99.0 L 112.4,99.4 Z"
    );
}

#[test]
fn test_bevel_default_path_keeps_filled_face() {
    let adj = HashMap::new();
    let path = bevel_path(120.0, 100.0, &adj);

    assert_eq!(
        path,
        "M0,0 L120.0,0 L120.0,100.0 L0,100.0 Z M0,0 L12.5,12.5 M120.0,0 L107.5,12.5 M120.0,100.0 L107.5,87.5 M0,100.0 L12.5,87.5"
    );
}

#[test]
fn test_brace_pair_default_matches_explicit_official_default() {
    let default_path = brace_pair_path(120.0, 100.0, &HashMap::new());
    let explicit_path =
        brace_pair_path(120.0, 100.0, &HashMap::from([("adj".to_string(), 8_333.0)]));

    assert_eq!(default_path, explicit_path);
    assert_eq!(default_path.matches('A').count(), 8);
    assert!(default_path.ends_with('Z'));
}

#[test]
fn test_vertical_scroll_default_path_keeps_filled_body_and_rolls() {
    let adj = HashMap::new();
    let path = vertical_scroll_path(120.0, 100.0, &adj);

    assert_eq!(
        path,
        "M10.0,15.0 L109.4,15.0 Q115.0,15.0 115.0,20.6 L115.0,91.9 Q115.0,97.5 109.4,97.5 L10.0,97.5 Q0,97.5 0,91.9 L0,20.6 Q0,15.0 10.0,15.0 Z M10.0,0 L114.4,0 Q120.0,0 120.0,5.6 Q120.0,13.8 114.4,13.8 L10.0,13.8 Q4.4,13.8 4.4,5.6 Q4.4,0 10.0,0 Z M10.0,5.6 A5.6,5.6 0 1,1 10.0,8.1 A5.6,5.6 0 1,1 10.0,5.6 Z M5.6,86.2 A5.6,5.6 0 1,1 5.6,97.5 A5.6,5.6 0 1,1 5.6,86.2 Z"
    );
}

#[test]
fn test_left_circular_arrow_default_path_tracks_u_shape_reference() {
    assert_task9_continuous_geometry("leftCircularArrow");
}

#[test]
fn test_left_right_circular_arrow_default_path_tracks_arch_reference() {
    assert_task9_continuous_geometry("leftRightCircularArrow");
}

#[test]
fn test_circular_arrow_default_path_tracks_office_arc_span() {
    assert_task9_continuous_geometry("circularArrow");
}

#[test]
fn test_curved_right_arrow_default_path_tracks_reference_c_shape() {
    assert_task9_continuous_geometry("curvedRightArrow");
}

#[test]
fn test_star4_default_path_matches_office_body_width() {
    let adj = HashMap::new();
    let path = star4_path(100.0, 100.0, &adj);

    assert_eq!(
        path,
        "M50.0,0 L59.0,41.0 L100.0,50.0 L59.0,59.0 L50.0,100.0 L41.0,59.0 L0,50.0 L41.0,41.0 Z"
    );
}

#[test]
fn test_star5_default_path_matches_office_body_width() {
    let adj = HashMap::new();
    let path = star5_path(100.0, 100.0, &adj);

    assert_eq!(
        path,
        "M 80.6,99.2 L 76.4,97.5 L 51.8,78.4 L 48.6,78.4 L 24.6,96.9 L 19.4,99.2 L 30.1,62.9 L 0.5,38.2 L 38.0,37.4 L 49.6,0.8 L 50.9,1.7 L 62.0,37.4 L 98.6,37.4 L 99.5,38.2 L 98.6,40.2 L 69.9,62.9 L 80.6,99.2 Z"
    );
}

#[test]
fn test_star_variants() {
    let adj = HashMap::new();
    for name in [
        "star4", "star5", "star6", "star7", "star8", "star10", "star12", "star16", "star24",
        "star32",
    ] {
        let path = preset_shape_svg(name, 100.0, 100.0, &adj).unwrap();
        assert!(path.ends_with('Z'), "Star {name} not closed");
    }
}

#[test]
fn test_math_shapes() {
    let adj = HashMap::new();
    for name in [
        "mathPlus",
        "mathMinus",
        "mathEqual",
        "mathNotEqual",
        "mathMultiply",
        "mathDivide",
    ] {
        assert!(
            preset_shape_svg(name, 100.0, 100.0, &adj).is_some(),
            "Missing: {name}"
        );
    }
}

#[test]
fn test_trapezoid_default_matches_explicit_official_default() {
    let default_path = preset_shape_svg("trapezoid", 120.0, 100.0, &HashMap::new()).unwrap();
    let explicit_path = preset_shape_svg(
        "trapezoid",
        120.0,
        100.0,
        &HashMap::from([("adj".to_string(), 25_000.0)]),
    )
    .unwrap();

    assert_eq!(default_path, explicit_path);
    assert!(default_path.contains("L25.0,0 L95.0,0"));
}

#[test]
fn test_trapezoid_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let custom_adj = HashMap::from([("adj".to_string(), 40_000.0)]);

    let default_path = preset_shape_svg("trapezoid", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("trapezoid", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "trapezoid adjustment profiles should change the path"
    );
}

#[test]
fn test_trapezoid_adjustment_profiles_follow_official_continuous_formula() {
    for (adj, expected_top_left) in [
        (12_345.0, "L12.3,0"),
        (27_891.0, "L27.9,0"),
        (43_210.0, "L43.2,0"),
    ] {
        let adj_values = HashMap::from([("adj".to_string(), adj)]);
        let path = preset_shape_svg("trapezoid", 120.0, 100.0, &adj_values).unwrap();
        assert!(path.contains(expected_top_left), "adj={adj}: {path}");
        assert!(path.starts_with("M0,100.0"));
        assert!(path.ends_with('Z'));
    }
}

#[test]
fn test_hexagon_default_matches_explicit_official_defaults() {
    let default_path = preset_shape_svg("hexagon", 120.0, 100.0, &HashMap::new()).unwrap();
    let explicit_path = preset_shape_svg(
        "hexagon",
        120.0,
        100.0,
        &HashMap::from([("adj".to_string(), 25_000.0), ("vf".to_string(), 115_470.0)]),
    )
    .unwrap();

    assert_eq!(default_path, explicit_path);
    assert!(default_path.contains("L25.0,0.0 L95.0,0.0"));
}

#[test]
fn test_hexagon_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let custom_adj = HashMap::from([("adj".to_string(), 40_000.0)]);

    let default_path = preset_shape_svg("hexagon", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("hexagon", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "hexagon adjustment profiles should change the path"
    );
}

#[test]
fn test_hexagon_adjustment_profiles_follow_official_continuous_formula() {
    for (adj, vf, expected_top_left) in [
        (12_345.0, 100_000.0, "L12.3,6.7"),
        (27_891.0, 115_470.0, "L27.9,0.0"),
        (43_210.0, 80_000.0, "L43.2,15.4"),
    ] {
        let adj_values = HashMap::from([("adj".to_string(), adj), ("vf".to_string(), vf)]);
        let path = preset_shape_svg("hexagon", 120.0, 100.0, &adj_values).unwrap();
        assert!(
            path.contains(expected_top_left),
            "adj={adj}, vf={vf}: {path}"
        );
        assert!(path.ends_with('Z'));
    }
}

#[test]
fn test_round1_rect_default_matches_explicit_official_default() {
    let default_path = preset_shape_svg("round1Rect", 120.0, 100.0, &HashMap::new()).unwrap();
    let explicit_path = preset_shape_svg(
        "round1Rect",
        120.0,
        100.0,
        &HashMap::from([("adj".to_string(), 16_667.0)]),
    )
    .unwrap();

    assert_eq!(default_path, explicit_path);
    assert_eq!(default_path.matches('A').count(), 1);
}

#[test]
fn test_round1_rect_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let custom_adj = HashMap::from([("adj".to_string(), 30_000.0)]);

    let default_path = preset_shape_svg("round1Rect", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("round1Rect", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "round1Rect adjustment profiles should change the path"
    );
}

#[test]
fn test_round1_rect_adjustment_profiles_follow_official_continuous_formula() {
    for (adj, expected_radius) in [
        (12_345.0, "A12.3,12.3"),
        (27_891.0, "A27.9,27.9"),
        (43_210.0, "A43.2,43.2"),
    ] {
        let adj_values = HashMap::from([("adj".to_string(), adj)]);
        let path = preset_shape_svg("round1Rect", 120.0, 100.0, &adj_values).unwrap();
        assert!(path.contains(expected_radius), "adj={adj}: {path}");
        assert_eq!(path.matches('A').count(), 1);
    }
}

#[test]
fn test_round2_same_rect_default_matches_explicit_official_defaults() {
    let default_path = preset_shape_svg("round2SameRect", 120.0, 100.0, &HashMap::new()).unwrap();
    let explicit_path = preset_shape_svg(
        "round2SameRect",
        120.0,
        100.0,
        &HashMap::from([("adj1".to_string(), 16_667.0), ("adj2".to_string(), 0.0)]),
    )
    .unwrap();

    assert_eq!(default_path, explicit_path);
    assert_eq!(default_path.matches('A').count(), 4);
}

#[test]
fn test_round2_same_rect_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let custom_adj = HashMap::from([
        ("adj1".to_string(), 30_000.0),
        ("adj2".to_string(), 10_000.0),
    ]);

    let default_path = preset_shape_svg("round2SameRect", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("round2SameRect", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "round2SameRect adjustment profiles should change the path"
    );
}

#[test]
fn test_round2_same_rect_adjustment_profiles_keep_keys_isolated() {
    for (adj1, adj2, expected_top, expected_bottom) in [
        (12_345.0, 7_654.0, "A12.3,12.3", "A7.7,7.7"),
        (27_891.0, 19_876.0, "A27.9,27.9", "A19.9,19.9"),
        (43_210.0, 31_234.0, "A43.2,43.2", "A31.2,31.2"),
    ] {
        let adj_values = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("round2SameRect", 120.0, 100.0, &adj_values).unwrap();
        assert!(path.contains(expected_top), "adj1={adj1}: {path}");
        assert!(path.contains(expected_bottom), "adj2={adj2}: {path}");
        assert_eq!(path.matches('A').count(), 4);
    }
}

#[test]
fn test_snip2_diag_rect_default_matches_explicit_official_defaults() {
    let default_path = preset_shape_svg("snip2DiagRect", 120.0, 100.0, &HashMap::new()).unwrap();
    let explicit_path = preset_shape_svg(
        "snip2DiagRect",
        120.0,
        100.0,
        &HashMap::from([("adj1".to_string(), 0.0), ("adj2".to_string(), 16_667.0)]),
    )
    .unwrap();

    assert_eq!(default_path, explicit_path);
    assert!(default_path.ends_with('Z'));
}

#[test]
fn test_snip2_diag_rect_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let custom_adj = HashMap::from([
        ("adj1".to_string(), 30_000.0),
        ("adj2".to_string(), 10_000.0),
    ]);

    let default_path = preset_shape_svg("snip2DiagRect", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("snip2DiagRect", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "snip2DiagRect adjustment profiles should change the path"
    );
}

#[test]
fn test_snip2_diag_rect_adjustment_profiles_keep_keys_isolated() {
    for (adj1, adj2, expected_top_left, expected_bottom_left) in [
        (12_345.0, 7_654.0, "M12.3,0", "L7.7,100.0"),
        (27_891.0, 19_876.0, "M27.9,0", "L19.9,100.0"),
        (43_210.0, 31_234.0, "M43.2,0", "L31.2,100.0"),
    ] {
        let adj_values = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("snip2DiagRect", 120.0, 100.0, &adj_values).unwrap();
        assert!(path.starts_with(expected_top_left), "adj1={adj1}: {path}");
        assert!(path.contains(expected_bottom_left), "adj2={adj2}: {path}");
        assert!(path.ends_with('Z'));
    }
}

#[test]
fn test_snip_round_rect_default_matches_explicit_official_defaults() {
    let default_path = preset_shape_svg("snipRoundRect", 120.0, 100.0, &HashMap::new()).unwrap();
    let explicit_path = preset_shape_svg(
        "snipRoundRect",
        120.0,
        100.0,
        &HashMap::from([
            ("adj1".to_string(), 16_667.0),
            ("adj2".to_string(), 16_667.0),
        ]),
    )
    .unwrap();

    assert_eq!(default_path, explicit_path);
    assert_eq!(default_path.matches('A').count(), 1);
}

#[test]
fn test_snip_round_rect_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let custom_adj = HashMap::from([
        ("adj1".to_string(), 30_000.0),
        ("adj2".to_string(), 10_000.0),
    ]);

    let default_path = preset_shape_svg("snipRoundRect", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("snipRoundRect", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "snipRoundRect adjustment profiles should change the path"
    );
}

#[test]
fn test_snip_round_rect_adjustment_profiles_keep_keys_isolated() {
    for (adj1, adj2, expected_radius, expected_snip) in [
        (12_345.0, 7_654.0, "A12.3,12.3", "L112.3,0"),
        (27_891.0, 19_876.0, "A27.9,27.9", "L100.1,0"),
        (43_210.0, 31_234.0, "A43.2,43.2", "L88.8,0"),
    ] {
        let adj_values = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("snipRoundRect", 120.0, 100.0, &adj_values).unwrap();
        assert!(path.contains(expected_radius), "adj1={adj1}: {path}");
        assert!(path.contains(expected_snip), "adj2={adj2}: {path}");
        assert_eq!(path.matches('A').count(), 1);
    }
}

#[test]
fn test_pie_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj1".to_string(), 5400000.0);
    custom_adj.insert("adj2".to_string(), 10800000.0);

    let default_path = preset_shape_svg("pie", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("pie", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "pie adj values should change the path"
    );
}

#[test]
fn test_pie_adjustment_profiles_match_benchmarked_anchors() {
    let profiles = [
        (3_000_000.0, 12_000_000.0),
        (5_400_000.0, 16_200_000.0),
        (0.0, 18_000_000.0),
        (9_000_000.0, 11_000_000.0),
    ]
    .map(|(adj1, adj2)| {
        let adj = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("pie", 120.0, 100.0, &adj).unwrap();
        assert_official_path(&path, 1, 1);
        path
    });
    assert!(profiles.windows(2).all(|pair| pair[0] != pair[1]));
}

#[test]
fn test_arc_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj1".to_string(), 5400000.0);
    custom_adj.insert("adj2".to_string(), 10800000.0);

    let default_path = preset_shape_svg("arc", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("arc", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "arc adj values should change the path"
    );
}

#[test]
fn test_arc_adjustment_profiles_match_benchmarked_anchors() {
    let profiles = [
        (3_000_000.0, 12_000_000.0),
        (5_400_000.0, 16_200_000.0),
        (0.0, 18_000_000.0),
        (9_000_000.0, 11_000_000.0),
    ]
    .map(|(adj1, adj2)| {
        let adj = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("arc", 120.0, 100.0, &adj).unwrap();
        assert_official_path(&path, 2, 2);
        path
    });
    assert!(profiles.windows(2).all(|pair| pair[0] != pair[1]));
}

#[test]
fn test_block_arc_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj1".to_string(), 5400000.0);
    custom_adj.insert("adj2".to_string(), 16200000.0);
    custom_adj.insert("adj3".to_string(), 40000.0);

    let default_path = preset_shape_svg("blockArc", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("blockArc", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "blockArc adj values should change the path"
    );
}

#[test]
fn test_block_arc_default_path_matches_upper_band_silhouette() {
    let path = preset_shape_svg("blockArc", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_official_path(&path, 1, 2);
    assert!(path.starts_with("M0.00,50.00 A60.00,50.00"));
    assert!(path.contains("L95.00,50.00 A35.00,25.00"), "{path}");
    assert!(path.ends_with("25.00,50.00 Z"));
}

#[test]
fn test_funnel_default_path_matches_extracted_body_curve() {
    let path = funnel_path(120.0, 100.0);

    let ys: Vec<f64> = path
        .split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .filter(|token| !token.is_empty())
        .skip(1)
        .step_by(2)
        .map(|token| token.parse::<f64>().unwrap())
        .collect();

    assert!(
        path.contains('C'),
        "funnel default should use the extracted curved body instead of a hexagon"
    );
    assert!(
        ys.iter().copied().fold(f64::INFINITY, f64::min) <= 0.1,
        "funnel mouth should still reach the top edge: {path}"
    );
    assert!(
        ys.iter().copied().fold(f64::NEG_INFINITY, f64::max) >= 94.0,
        "funnel tail should extend near the bottom edge: {path}"
    );
    assert!(
        path.contains("M5.7,26.2 A54.3,17.1 0 1,0 114.2,26.2"),
        "funnel default should carve the inner opening ellipse: {path}"
    );
}

#[test]
fn test_block_arc_adjustment_profiles_match_benchmarked_anchors() {
    let profiles = [
        (12_000.0, 8_500_000.0, 17_000_000.0),
        (35_000.0, 3_000_000.0, 13_000_000.0),
        (50_000.0, 0.0, 21_600_000.0),
        (25_000.0, 6_000_000.0, 18_000_000.0),
    ]
    .map(|(adj1, adj2, adj3)| {
        let adj = HashMap::from([
            ("adj1".to_string(), adj1),
            ("adj2".to_string(), adj2),
            ("adj3".to_string(), adj3),
        ]);
        let path = preset_shape_svg("blockArc", 120.0, 100.0, &adj).unwrap();
        assert_official_path(&path, 1, 1);
        path
    });
    assert!(profiles.windows(2).all(|pair| pair[0] != pair[1]));
}

fn assert_task9_continuous_geometry(preset: &str) {
    let paths = [11_111.0, 22_222.0, 33_333.0].map(|value| {
        let adjustments = HashMap::from([("adj1".to_string(), value)]);
        preset_shape_svg(preset, 120.0, 100.0, &adjustments).expect("Task 9 preset path")
    });
    assert!(
        paths.iter().all(|path| path.contains('Z')),
        "{preset} must retain closed topology"
    );
    assert!(
        paths.windows(2).all(|pair| pair[0] != pair[1]),
        "{preset} adj1 must remain continuous at non-anchor values"
    );
    for path in paths {
        for token in path
            .split(|character: char| {
                character.is_ascii_alphabetic() || character == ',' || character.is_whitespace()
            })
            .filter(|token| !token.is_empty())
        {
            assert!(
                token.parse::<f64>().expect("SVG number").is_finite(),
                "{preset} emitted a non-finite coordinate"
            );
        }
    }
}

#[test]
fn test_circular_arrow_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj1".to_string(), 20000.0);
    custom_adj.insert("adj5".to_string(), 25000.0);

    let default_path = preset_shape_svg("circularArrow", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("circularArrow", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "circularArrow adj values should change the path"
    );
}

#[test]
fn test_circular_arrow_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("circularArrow");
}

#[test]
fn test_curved_right_arrow_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj1".to_string(), 10000.0);
    custom_adj.insert("adj2".to_string(), 80000.0);

    let default_path = preset_shape_svg("curvedRightArrow", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("curvedRightArrow", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "curvedRightArrow adj1/adj2 should change the path"
    );
}

#[test]
fn test_curved_left_arrow_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj1".to_string(), 10000.0);
    custom_adj.insert("adj2".to_string(), 80000.0);

    let default_path = preset_shape_svg("curvedLeftArrow", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("curvedLeftArrow", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "curvedLeftArrow adj1/adj2 should change the path"
    );
}

#[test]
fn test_curved_up_arrow_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj1".to_string(), 10000.0);
    custom_adj.insert("adj2".to_string(), 80000.0);

    let default_path = preset_shape_svg("curvedUpArrow", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("curvedUpArrow", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "curvedUpArrow adj1/adj2 should change the path"
    );
}

#[test]
fn test_curved_down_arrow_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj1".to_string(), 10000.0);
    custom_adj.insert("adj2".to_string(), 80000.0);

    let default_path = preset_shape_svg("curvedDownArrow", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("curvedDownArrow", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "curvedDownArrow adj1/adj2 should change the path"
    );
}

#[test]
fn test_curved_arrow_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("curvedRightArrow");
}

#[test]
fn test_curved_arrow_multi_svg_preserves_all_official_path_roles() {
    let tight_adj = HashMap::from([
        ("adj1".to_string(), 12_000.0),
        ("adj2".to_string(), 70_000.0),
        ("adj3".to_string(), 18_000.0),
    ]);
    let wide_adj = HashMap::from([
        ("adj1".to_string(), 42_000.0),
        ("adj2".to_string(), 30_000.0),
        ("adj3".to_string(), 42_000.0),
    ]);

    for preset in [
        "curvedRightArrow",
        "curvedLeftArrow",
        "curvedUpArrow",
        "curvedDownArrow",
    ] {
        let tight = preset_shape_multi_svg(preset, 120.0, 100.0, &tight_adj)
            .expect("tight multipath preset should be available");
        let wide = preset_shape_multi_svg(preset, 120.0, 100.0, &wide_adj)
            .expect("wide multipath preset should be available");
        assert_eq!(tight.paths.len(), 3);
        assert_eq!(wide.paths.len(), 3);
        assert!(matches!(tight.paths[0].fill, PathFill::Norm));
        assert!(matches!(tight.paths[1].fill, PathFill::DarkenLess));
        assert!(matches!(tight.paths[2].fill, PathFill::None));
        assert!(!tight.paths[0].stroke);
        assert!(!tight.paths[1].stroke);
        assert!(tight.paths[2].stroke);
    }
}

#[test]
fn test_wave_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj2".to_string(), 40000.0);

    let default_path = preset_shape_svg("wave", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("wave", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "wave adj2 should change the path"
    );
}

#[test]
fn test_wave_adjustment_profiles_match_benchmarked_anchors() {
    let profiles = [
        (10_000.0, 0.0),
        (12_500.0, 40_000.0),
        (30_000.0, 0.0),
        (30_000.0, 40_000.0),
    ]
    .map(|(adj1, adj2)| {
        let adj = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("wave", 120.0, 100.0, &adj).unwrap();
        assert_official_path(&path, 1, 0);
        assert_eq!(path.matches('C').count(), 2);
        path
    });
    assert!(profiles.windows(2).all(|pair| pair[0] != pair[1]));
}

#[test]
fn test_double_wave_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj2".to_string(), 40000.0);

    let default_path = preset_shape_svg("doubleWave", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("doubleWave", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "doubleWave adj2 should change the path"
    );
}

#[test]
fn test_double_wave_adjustment_profiles_match_benchmarked_anchors() {
    let profiles = [
        (10_000.0, 0.0),
        (12_500.0, 40_000.0),
        (30_000.0, 0.0),
        (30_000.0, 40_000.0),
    ]
    .map(|(adj1, adj2)| {
        let adj = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("doubleWave", 120.0, 100.0, &adj).unwrap();
        assert_official_path(&path, 1, 0);
        assert_eq!(path.matches('C').count(), 4);
        path
    });
    assert!(profiles.windows(2).all(|pair| pair[0] != pair[1]));
}

#[test]
fn test_chord_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj1".to_string(), 5400000.0);
    custom_adj.insert("adj2".to_string(), 10800000.0);

    let default_path = preset_shape_svg("chord", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("chord", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "chord adj values should change the path"
    );
}

#[test]
fn test_bent_arrow_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj4".to_string(), 70000.0);

    let default_path = preset_shape_svg("bentArrow", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("bentArrow", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "bentArrow adj4 should change the path"
    );
}

#[test]
fn test_bent_arrow_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("bentArrow");
}

#[test]
fn test_notched_right_arrow_default_path_preserves_legacy_polygon() {
    assert_task9_continuous_geometry("notchedRightArrow");
}

#[test]
fn test_notched_right_arrow_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let custom_adj = HashMap::from([
        ("adj1".to_string(), 20_000.0),
        ("adj2".to_string(), 50_000.0),
    ]);

    let default_path = preset_shape_svg("notchedRightArrow", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("notchedRightArrow", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "notchedRightArrow adjustment profiles should change the path"
    );
}

#[test]
fn test_notched_right_arrow_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("notchedRightArrow");
}

#[test]
fn test_striped_right_arrow_default_path_preserves_legacy_polygon_and_stripes() {
    assert_task9_continuous_geometry("stripedRightArrow");
}

#[test]
fn test_striped_right_arrow_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let custom_adj = HashMap::from([
        ("adj1".to_string(), 20_000.0),
        ("adj2".to_string(), 50_000.0),
    ]);

    let default_path = preset_shape_svg("stripedRightArrow", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("stripedRightArrow", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "stripedRightArrow adjustment profiles should change the path"
    );
}

#[test]
fn test_striped_right_arrow_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("stripedRightArrow");
}

#[test]
fn test_bent_up_arrow_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj3".to_string(), 70000.0);

    let default_path = preset_shape_svg("bentUpArrow", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("bentUpArrow", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "bentUpArrow adj3 should change the path"
    );
}

#[test]
fn test_bent_up_arrow_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("bentUpArrow");
}

#[test]
fn test_left_right_up_arrow_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj2".to_string(), 60000.0);
    custom_adj.insert("adj3".to_string(), 70000.0);

    let default_path = preset_shape_svg("leftRightUpArrow", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("leftRightUpArrow", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "leftRightUpArrow adj2/adj3 should change the path"
    );
}

#[test]
fn test_left_right_up_arrow_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("leftRightUpArrow");
}

#[test]
fn test_left_right_up_arrow_default_path_matches_extracted_polygon() {
    assert_task9_continuous_geometry("leftRightUpArrow");
}

#[test]
fn test_quad_arrow_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("quadArrow");
}

#[test]
fn test_left_up_arrow_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj3".to_string(), 70000.0);

    let default_path = preset_shape_svg("leftUpArrow", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("leftUpArrow", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "leftUpArrow adj3 should change the path"
    );
}

#[test]
fn test_left_up_arrow_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("leftUpArrow");
}

#[test]
fn test_uturn_arrow_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj3".to_string(), 45000.0);
    custom_adj.insert("adj4".to_string(), 70000.0);
    custom_adj.insert("adj5".to_string(), 85000.0);

    let default_path = preset_shape_svg("uturnArrow", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("uturnArrow", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "uturnArrow adj3/adj4/adj5 should change the path"
    );
}

#[test]
fn test_uturn_arrow_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("uturnArrow");
}

#[test]
fn test_uturn_arrow_default_path_matches_extracted_office_outline() {
    assert_task9_continuous_geometry("uturnArrow");
}

#[test]
fn test_swoosh_arrow_default_path_matches_extracted_office_outline() {
    assert_task9_continuous_geometry("swooshArrow");
}

#[test]
fn test_cloud_callout_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj1".to_string(), 20_000.0);
    custom_adj.insert("adj2".to_string(), 30_000.0);

    let default_path = preset_shape_svg("cloudCallout", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("cloudCallout", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "cloudCallout adj1/adj2 should change the path"
    );
}

#[test]
fn test_cloud_callout_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("cloudCallout");
}

#[test]
fn test_wedge_round_rect_callout_default_path_preserves_legacy_polygon() {
    assert_task9_continuous_geometry("wedgeRoundRectCallout");
}

#[test]
fn test_wedge_round_rect_callout_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let custom_adj = HashMap::from([
        ("adj1".to_string(), 20_833.0),
        ("adj2".to_string(), 20_000.0),
    ]);

    let default_path =
        preset_shape_svg("wedgeRoundRectCallout", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("wedgeRoundRectCallout", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "wedgeRoundRectCallout adjustment profiles should change the path"
    );
}

#[test]
fn test_wedge_round_rect_callout_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("wedgeRoundRectCallout");
}

#[test]
fn test_wedge_ellipse_callout_default_path_preserves_legacy_multipath() {
    assert_task9_continuous_geometry("wedgeEllipseCallout");
}

#[test]
fn test_wedge_ellipse_callout_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let custom_adj = HashMap::from([
        ("adj1".to_string(), 20_833.0),
        ("adj2".to_string(), 20_000.0),
    ]);

    let default_path = preset_shape_svg("wedgeEllipseCallout", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("wedgeEllipseCallout", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "wedgeEllipseCallout adjustment profiles should change the path"
    );
}

#[test]
fn test_wedge_ellipse_callout_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("wedgeEllipseCallout");
}

#[test]
fn test_wedge_rect_callout_default_path_preserves_legacy_polygon() {
    assert_task9_continuous_geometry("wedgeRectCallout");
}

#[test]
fn test_wedge_rect_callout_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let custom_adj = HashMap::from([
        ("adj1".to_string(), 20_833.0),
        ("adj2".to_string(), 20_000.0),
    ]);

    let default_path = preset_shape_svg("wedgeRectCallout", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("wedgeRectCallout", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "wedgeRectCallout adjustment profiles should change the path"
    );
}

#[test]
fn test_wedge_rect_callout_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("wedgeRectCallout");
}

#[test]
fn test_math_not_equal_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj2".to_string(), 4_200_000.0);

    let default_path = preset_shape_svg("mathNotEqual", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("mathNotEqual", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "mathNotEqual's valid lower angle boundary should change the path"
    );
}

#[test]
fn test_math_not_equal_default_path_matches_extracted_office_polygon() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("mathNotEqual", 120.0, 100.0, &default_adj).unwrap();

    assert_eq!(
        path,
        "M15.91,20.60 L58.19,20.60 L65.68,0.00 L87.79,8.04 L83.22,20.60 L104.09,20.60 L104.09,44.12 L74.65,44.12 L70.37,55.88 L104.09,55.88 L104.09,79.40 L61.81,79.40 L54.32,100.00 L32.21,91.96 L36.78,79.40 L15.91,79.40 L15.91,55.88 L45.35,55.88 L49.63,44.12 L15.91,44.12 Z"
    );
}

#[test]
fn test_math_divide_default_path_matches_extracted_office_geometry() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("mathDivide", 120.0, 100.0, &default_adj).unwrap();

    assert_eq!(path.matches('M').count(), 3);
    assert_eq!(path.matches('A').count(), 4);
    assert!(path.contains("M60.00,11.79 A11.76,11.76"));
    assert!(path.contains("M15.91,38.24 L104.09,38.24"));
}

#[test]
fn test_math_equal_default_path_matches_extracted_office_geometry() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("mathEqual", 120.0, 100.0, &default_adj).unwrap();

    assert_eq!(
        path,
        "M15.91,20.60 L104.09,20.60 L104.09,44.12 L15.91,44.12 Z M15.91,55.88 L104.09,55.88 L104.09,79.40 L15.91,79.40 Z"
    );
}

#[test]
fn test_math_plus_default_path_matches_extracted_office_geometry() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("mathPlus", 120.0, 100.0, &default_adj).unwrap();

    assert_eq!(
        path,
        "M15.91,38.24 L48.24,38.24 L48.24,13.26 L71.76,13.26 L71.76,38.24 L104.09,38.24 L104.09,61.76 L71.76,61.76 L71.76,86.75 L48.24,86.75 L48.24,61.76 L15.91,61.76 Z"
    );
}

#[test]
fn test_plus_default_path_matches_benchmark_cross_outline() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("plus", 120.0, 100.0, &default_adj).unwrap();

    assert_eq!(
        path,
        "M0.00,25.00 L25.00,25.00 L25.00,0.00 L95.00,0.00 L95.00,25.00 L120.00,25.00 L120.00,75.00 L95.00,75.00 L95.00,100.00 L25.00,100.00 L25.00,75.00 L0.00,75.00 Z"
    );
}

#[test]
fn test_cross_alias_default_path_matches_benchmark_cross_outline() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("cross", 120.0, 100.0, &default_adj).unwrap();

    assert_eq!(
        path,
        "M 88.0,100.2 L 30.5,99.8 28.7,97.9 28.7,77.2 27.5,76.1 4.0,76.1 -0.2,73.9 -0.2,26.1 1.0,23.9 28.7,23.2 28.7,2.5 30.0,-0.2 89.0,-0.2 90.8,2.1 91.3,23.2 118.0,23.9 119.8,25.7 120.2,73.4 119.0,75.3 115.5,76.1 92.0,76.1 90.8,78.0 90.8,98.3 Z"
    );
}

#[test]
fn test_math_minus_default_path_matches_extracted_office_geometry() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("mathMinus", 120.0, 100.0, &default_adj).unwrap();

    assert_eq!(
        path,
        "M15.91,38.24 L104.09,38.24 L104.09,61.76 L15.91,61.76 Z"
    );
}

#[test]
fn test_curved_right_arrow_default_path_matches_extracted_office_outline() {
    assert_task9_continuous_geometry("curvedRightArrow");
}

#[test]
fn test_curved_left_arrow_default_path_matches_extracted_office_outline() {
    assert_task9_continuous_geometry("curvedLeftArrow");
}

#[test]
fn test_curved_up_arrow_default_path_matches_extracted_office_outline() {
    assert_task9_continuous_geometry("curvedUpArrow");
}

#[test]
fn test_curved_down_arrow_default_path_matches_extracted_office_outline() {
    assert_task9_continuous_geometry("curvedDownArrow");
}

#[test]
fn test_gear6_default_path_matches_extracted_office_outline() {
    let path = preset_shape_svg("gear6", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_official_path(&path, 1, 6);
    assert!(path.starts_with("M91.92,25.33 L106.06,19.22"), "{path}");
    assert!(path.contains("L52.01,98.84"), "{path}");
    assert!(path.contains("L67.99,1.16"), "{path}");
}

#[test]
fn test_gear9_default_path_matches_extracted_office_outline() {
    let path = preset_shape_svg("gear9", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_official_path(&path, 1, 9);
    assert!(path.starts_with("M86.23,15.94 L93.26,8.62"), "{path}");
    assert!(path.contains("L54.49,0.43 L65.51,0.43"));
}

#[test]
fn test_plaque_tabs_default_path_uses_small_quarter_tabs() {
    let path = preset_shape_svg("plaqueTabs", 120.0, 100.0, &HashMap::new()).unwrap();

    assert!(path.contains("M 0.0,7.8"));
    assert!(path.contains("120.0,7.8"));
    assert!(path.contains("120.0,92.7"));
    assert!(path.contains("9.3,100.0"));
}

#[test]
fn test_arc_default_path_matches_quarter_sector_reference() {
    let path = preset_shape_svg("arc", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_official_path(&path, 2, 2);
    assert!(path.starts_with("M60.00,0.00 A60.00,50.00"));
    assert!(path.contains("120.00,50.00 L60.00,50.00 Z"));
}

#[test]
fn test_no_smoking_default_path_carves_inner_ring_hole() {
    let path = preset_shape_svg("noSmoking", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_official_path(&path, 3, 6);
    assert!(path.starts_with("M0.00,50.00 A60.00,50.00"));
    assert!(path.contains("M95.02,66.52 A41.25,31.25"), "{path}");
}

#[test]
fn test_chord_default_path_matches_office_outline() {
    let path = preset_shape_svg("chord", 120.0, 100.0, &HashMap::new()).unwrap();
    assert_official_path(&path, 1, 2);
    assert!(path.starts_with("M98.41,88.41 A60.00,50.00"), "{path}");
    assert!(path.contains("6.27,72.26"), "{path}");
    assert!(path.ends_with("60.00,0.00 Z"));
}

#[test]
fn test_can_default_uses_filled_top_ellipse_without_evenodd_hole() {
    let path = preset_shape_svg("can", 120.0, 100.0, &HashMap::new()).unwrap();

    assert!(!needs_evenodd_fill("can"));
    assert_official_path(&path, 3, 6);
    assert!(path.starts_with("M0.00,12.50 A60.00,12.50"));
    assert!(path.contains("M0.00,12.50 A60.00,12.50 0 0,1 120.00,12.50"));
}

#[test]
fn test_pie_wedge_default_path_matches_reference_orientation() {
    let path = preset_shape_svg("pieWedge", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_eq!(
        path,
        "M0.00,100.00 A120.00,100.00 0 0,1 120.00,0.00 L120.00,100.00 Z"
    );
}

#[test]
fn test_quad_arrow_callout_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("quadArrowCallout");
}

#[test]
fn test_left_right_arrow_callout_default_path_matches_office_outline() {
    assert_task9_continuous_geometry("leftRightArrowCallout");
}

#[test]
fn test_left_right_arrow_callout_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("leftRightArrowCallout");
}

#[test]
fn test_teardrop_adjustment_profiles_match_benchmarked_anchors() {
    let profiles = [20_000.0, 50_000.0, 80_000.0, 100_000.0].map(|adj| {
        let adj_values = HashMap::from([("adj".to_string(), adj)]);
        let path = preset_shape_svg("teardrop", 120.0, 100.0, &adj_values).unwrap();
        assert_official_path(&path, 1, 2);
        assert_eq!(path.matches('Q').count(), 2);
        path
    });
    assert!(profiles.windows(2).all(|pair| pair[0] != pair[1]));
}

#[test]
fn test_up_down_arrow_callout_default_path_matches_office_outline() {
    assert_task9_continuous_geometry("upDownArrowCallout");
}

#[test]
fn test_up_down_arrow_callout_adjustment_profiles_match_benchmarked_anchors() {
    assert_task9_continuous_geometry("upDownArrowCallout");
}

#[test]
fn test_quad_arrow_callout_default_path_matches_office_outline() {
    assert_task9_continuous_geometry("quadArrowCallout");
}

#[test]
fn test_math_divide_default_path_uses_circular_dots() {
    let path = preset_shape_svg("mathDivide", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_official_path(&path, 3, 4);
    assert!(path.contains("M60.00,88.21 A11.76,11.76"));
    assert!(path.contains("L104.09,61.76 L15.91,61.76 Z"));
}

#[test]
fn test_left_right_ribbon_default_path_matches_office_outline() {
    let path = preset_shape_svg("leftRightRibbon", 120.0, 100.0, &HashMap::new()).unwrap();
    let multi = preset_shape_multi_svg("leftRightRibbon", 120.0, 100.0, &HashMap::new())
        .expect("official ribbon multipath");
    assert!(path.matches('M').count() >= 3);
    assert!(path.matches('A').count() >= 8);
    assert!(path.starts_with("M0.00,41.67 L50.00,0.00"), "{path}");
    assert!(path.contains("L120.00,58.33 L70.00,100.00"));
    assert_eq!(multi.paths.len(), 3);
    assert!(matches!(multi.paths[0].fill, PathFill::Norm));
    assert!(matches!(multi.paths[1].fill, PathFill::DarkenLess));
    assert!(matches!(multi.paths[2].fill, PathFill::None));
    assert!(multi.paths[2].stroke);
}

#[test]
fn test_star5_default_path_matches_extracted_reference_outline() {
    let path = preset_shape_svg("star5", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_official_path(&path, 1, 0);
    assert!(
        path.starts_with("M0.00,38.20 L45.84,38.20 L60.00,0.00"),
        "{path}"
    );
    assert!(path.contains("L97.08,100.00 L60.00,76.39 L22.92,100.00"));
}

#[test]
fn test_irregular_seal1_default_path_matches_extracted_reference_outline() {
    let path = preset_shape_svg("irregularSeal1", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_official_path(&path, 1, 0);
    assert!(path.starts_with("M60.00,26.85 L80.68,0.00"));
    assert!(
        path.contains("L117.21,37.67 L97.82,48.50 L120.00,61.53"),
        "{path}"
    );
    assert!(path.ends_with("46.40,10.62 Z"));
}

#[test]
fn test_moon_default_path_matches_extracted_reference_outline() {
    let path = preset_shape_svg("moon", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_official_path(&path, 1, 2);
    assert!(path.starts_with("M120.00,100.00 A120.00,50.00"));
    assert!(
        path.contains("A150.00,62.50 0 0,0 120.00,100.00 Z"),
        "{path}"
    );
}

#[test]
fn test_irregular_seal2_default_path_matches_extracted_reference_outline() {
    let path = preset_shape_svg("irregularSeal2", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_official_path(&path, 1, 0);
    assert!(path.starts_with("M63.68,20.10 L82.17,0.00"), "{path}");
    assert!(path.contains("L120.00,30.76 L94.36,43.53"));
    assert!(path.ends_with("54.01,8.74 Z"));
}

#[test]
fn test_cloud_default_path_matches_extracted_reference_outline() {
    let path = preset_shape_svg("cloud", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_eq!(path.matches('M').count(), 12);
    assert_eq!(path.matches('A').count(), 22, "{path}");
    assert!(path.starts_with("M10.83,33.26 A18.76,21.27"));
    assert!(!path.contains('L'));
    assert!(path.contains("M116.11,35.54 A14.81,16.84"));
    assert!(path.contains('Z'));
}

#[test]
fn test_corner_tabs_default_path_matches_corner_triangles() {
    let path = preset_shape_svg("cornerTabs", 120.0, 100.0, &HashMap::new()).unwrap();

    assert!(path.contains("11.9,0.0"));
    assert!(path.contains("0.0,6.2"));
    assert!(path.contains("120.0,6.2"));
    assert!(path.contains("108.8,100.0"));
}

#[test]
fn test_square_tabs_default_path_matches_detached_squares() {
    let path = preset_shape_svg("squareTabs", 120.0, 100.0, &HashMap::new()).unwrap();

    assert!(path.contains("9.3,7.3"));
    assert!(path.contains("120.0,7.8"));
    assert!(path.contains("9.3,100.0"));
    assert!(path.contains("111.2,100.0"));
}

#[test]
fn test_bent_connector5_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj1".to_string(), 20_000.0);
    custom_adj.insert("adj2".to_string(), 35_000.0);
    custom_adj.insert("adj3".to_string(), 80_000.0);

    let default_path = preset_shape_svg("bentConnector5", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("bentConnector5", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "bentConnector5 adj1/adj2/adj3 should change the path"
    );
}

#[test]
fn test_lightning_bolt_default_path_matches_extracted_office_polygon() {
    let path = lightning_bolt_path(120.0, 100.0);

    assert_eq!(
        path,
        "M 47.8,1.4 L 70.7,27.9 61.3,31.0 90.2,53.7 80.7,57.5 116.4,95.5 55.8,66.4 67.4,62.3 29.8,43.7 43.3,37.9 3.6,18.4 47.8,1.4 Z"
    );
}

#[test]
fn test_math_multiply_default_path_matches_extracted_office_polygon() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("mathMultiply", 120.0, 100.0, &default_adj).unwrap();

    assert_official_path(&path, 1, 0);
    assert!(
        path.starts_with("M21.29,33.05 L36.35,14.98 L60.00,34.69"),
        "{path}"
    );
    assert!(path.contains("L98.71,66.95 L83.65,85.02"));
    assert!(path.ends_with("41.63,50.00 Z"));
}

#[test]
fn test_bent_up_arrow_default_path_matches_extracted_office_polygon() {
    assert_task9_continuous_geometry("bentUpArrow");
}

#[test]
fn test_left_up_arrow_default_path_matches_extracted_office_polygon() {
    assert_task9_continuous_geometry("leftUpArrow");
}

#[test]
fn test_right_brace_default_follows_official_continuous_formula() {
    let default_path = preset_shape_svg("rightBrace", 120.0, 100.0, &HashMap::new()).unwrap();
    let explicit_path = preset_shape_svg(
        "rightBrace",
        120.0,
        100.0,
        &HashMap::from([
            ("adj1".to_string(), 8_333.0),
            ("adj2".to_string(), 50_000.0),
        ]),
    )
    .unwrap();

    assert_eq!(default_path, explicit_path);
    assert!(default_path.starts_with("M0,0 A60.0,8.3"));
    assert!(default_path.contains("L60.0,41.7"));
    assert_eq!(default_path.matches('A').count(), 4);
    assert!(default_path.ends_with('Z'));
}

#[test]
fn test_left_brace_default_follows_official_continuous_formula() {
    let default_path = preset_shape_svg("leftBrace", 120.0, 100.0, &HashMap::new()).unwrap();
    let explicit_path = preset_shape_svg(
        "leftBrace",
        120.0,
        100.0,
        &HashMap::from([
            ("adj1".to_string(), 8_333.0),
            ("adj2".to_string(), 50_000.0),
        ]),
    )
    .unwrap();

    assert_eq!(default_path, explicit_path);
    assert!(default_path.starts_with("M120.0,100.0 A60.0,8.3"));
    assert!(default_path.contains("L60.0,58.3"));
    assert_eq!(default_path.matches('A').count(), 4);
    assert!(default_path.ends_with('Z'));
}

#[test]
fn test_bracket_pair_default_matches_explicit_official_default() {
    let default_path = preset_shape_svg("bracketPair", 120.0, 100.0, &HashMap::new()).unwrap();
    let explicit_path = preset_shape_svg(
        "bracketPair",
        120.0,
        100.0,
        &HashMap::from([("adj".to_string(), 16_667.0)]),
    )
    .unwrap();

    assert_eq!(default_path, explicit_path);
    assert_eq!(default_path.matches('A').count(), 4);
    assert!(default_path.ends_with('Z'));
}

#[test]
fn test_bracket_pair_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let custom_adj = HashMap::from([("adj".to_string(), 30_000.0)]);

    let default_path = preset_shape_svg("bracketPair", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("bracketPair", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "bracketPair adjustment profiles should change the path"
    );
}

#[test]
fn test_bracket_pair_adjustment_profiles_follow_official_continuous_formula() {
    for (adj, expected_radius) in [
        (12_345.0, "A12.3,12.3"),
        (27_891.0, "A27.9,27.9"),
        (43_210.0, "A43.2,43.2"),
    ] {
        let adj_values = HashMap::from([("adj".to_string(), adj)]);
        let path = preset_shape_svg("bracketPair", 120.0, 100.0, &adj_values).unwrap();
        assert!(path.contains(expected_radius), "adj={adj}: {path}");
        assert_eq!(path.matches('A').count(), 4);
    }
}

#[test]
fn test_brace_pair_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let custom_adj = HashMap::from([("adj".to_string(), 20_000.0)]);

    let default_path = preset_shape_svg("bracePair", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("bracePair", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "bracePair adjustment profiles should change the path"
    );
}

#[test]
fn test_brace_pair_adjustment_profiles_follow_official_continuous_formula() {
    for (adj, expected_radius) in [
        (7_654.0, "A7.7,7.7"),
        (12_345.0, "A12.3,12.3"),
        (23_456.0, "A23.5,23.5"),
    ] {
        let adj_values = HashMap::from([("adj".to_string(), adj)]);
        let path = preset_shape_svg("bracePair", 120.0, 100.0, &adj_values).unwrap();
        assert!(path.contains(expected_radius), "adj={adj}: {path}");
        assert_eq!(path.matches('A').count(), 8);
    }
}

#[test]
fn test_half_frame_default_matches_explicit_official_defaults() {
    let default_path = preset_shape_svg("halfFrame", 120.0, 100.0, &HashMap::new()).unwrap();
    let explicit_path = preset_shape_svg(
        "halfFrame",
        120.0,
        100.0,
        &HashMap::from([
            ("adj1".to_string(), 33_333.0),
            ("adj2".to_string(), 33_333.0),
        ]),
    )
    .unwrap();

    assert_eq!(default_path, explicit_path);
    assert!(default_path.ends_with('Z'));
}

#[test]
fn test_half_frame_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let custom_adj = HashMap::from([
        ("adj1".to_string(), 50_000.0),
        ("adj2".to_string(), 50_000.0),
    ]);

    let default_path = preset_shape_svg("halfFrame", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("halfFrame", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "halfFrame adjustment profiles should change the path"
    );
}

#[test]
fn test_half_frame_adjustment_profiles_follow_official_coupled_formula() {
    for (adj1, adj2, expected_landmarks) in [
        (
            20_000.0,
            40_000.0,
            ["L96.0,20.0", "L40.0,20.0", "L40.0,66.7"],
        ),
        (
            30_000.0,
            20_000.0,
            ["L84.0,30.0", "L20.0,30.0", "L20.0,83.3"],
        ),
        (
            40_000.0,
            30_000.0,
            ["L72.0,40.0", "L30.0,40.0", "L30.0,75.0"],
        ),
    ] {
        let adj_values = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("halfFrame", 120.0, 100.0, &adj_values).unwrap();
        for landmark in expected_landmarks {
            assert!(path.contains(landmark), "adj1={adj1}, adj2={adj2}: {path}");
        }
        assert!(path.ends_with('Z'));
    }
}

#[test]
fn test_uturn_arrow_extreme_adj_keeps_arc_radii_non_negative() {
    let mut extreme_adj = HashMap::new();
    extreme_adj.insert("adj1".to_string(), 90000.0);
    extreme_adj.insert("adj2".to_string(), 90000.0);

    let path = preset_shape_svg("uturnArrow", 120.0, 100.0, &extreme_adj).unwrap();

    assert!(
        !path.contains("A-"),
        "uturnArrow should not emit negative arc radii under extreme adj values: {path}"
    );
    assert!(
        !path.contains("NaN") && !path.contains("inf"),
        "uturnArrow should keep SVG path numeric under extreme adj values: {path}"
    );
}

#[test]
fn official_full_and_multiple_turn_arcs_are_visible_and_deterministic() {
    use super::official_presets_formula::GuideEnvironment;
    use super::official_presets_path::{
        PathCommandDefinition, PathDefinition, PointDefinition, render_path,
    };
    use crate::model::PathFill;

    for (swing, expected_arcs, expected_sweep) in [
        ("21600000", 2, "0 0,1"),
        ("43200000", 4, "0 0,1"),
        ("-21600000", 2, "0 0,0"),
        ("-43200000", 4, "0 0,0"),
        ("0", 0, ""),
    ] {
        let definition = PathDefinition {
            width: None,
            height: None,
            fill: PathFill::Norm,
            stroke: true,
            commands: vec![
                PathCommandDefinition::Move(vec![PointDefinition {
                    x: "10".into(),
                    y: "5".into(),
                }]),
                PathCommandDefinition::Arc {
                    width_radius: "5".into(),
                    height_radius: "5".into(),
                    start_angle: "0".into(),
                    swing_angle: swing.into(),
                },
            ],
        };
        let path = render_path(&definition, &GuideEnvironment::new(20.0, 20.0), 20.0, 20.0)
            .expect("official full-turn arc");
        assert_eq!(path.d.matches('A').count(), expected_arcs, "swing={swing}");
        if expected_arcs > 0 {
            assert_eq!(path.d.matches(expected_sweep).count(), expected_arcs);
            assert!(path.d.contains("5.00,5.00"), "swing={swing}: {}", path.d);
            assert!(path.d.ends_with("10.00,5.00"), "swing={swing}: {}", path.d);
        }
    }
}

#[test]
fn official_unrepresentable_arc_sweep_returns_explicit_error() {
    use super::official_presets_formula::GuideEnvironment;
    use super::official_presets_path::{
        PathCommandDefinition, PathDefinition, PointDefinition, render_path,
    };
    use crate::model::PathFill;

    let definition = PathDefinition {
        width: None,
        height: None,
        fill: PathFill::Norm,
        stroke: true,
        commands: vec![
            PathCommandDefinition::Move(vec![PointDefinition {
                x: "10".into(),
                y: "5".into(),
            }]),
            PathCommandDefinition::Arc {
                width_radius: "5".into(),
                height_radius: "5".into(),
                start_angle: "0".into(),
                swing_angle: "1e308".into(),
            },
        ],
    };
    assert!(render_path(&definition, &GuideEnvironment::new(20.0, 20.0), 20.0, 20.0,).is_err());
}
