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

#[test]
fn test_preset_shape_svg_returns_none_for_unknown() {
    let adj = HashMap::new();
    assert!(preset_shape_svg("unknownShape", 100.0, 100.0, &adj).is_none());
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
    let adj = HashMap::new();
    let path = right_arrow_path(120.0, 100.0, &adj);

    assert_eq!(
        path,
        "M0,25.0 L80.0,25.0 L80.0,0 L120.0,50.0 L80.0,100.0 L80.0,75.0 L0,75.0 Z"
    );
}

#[test]
fn test_left_arrow_default_path_uses_narrower_head_length() {
    let adj = HashMap::new();
    let path = left_arrow_path(120.0, 100.0, &adj);

    assert_eq!(
        path,
        "M120.0,25.0 L40.0,25.0 L40.0,0 L0,50.0 L40.0,100.0 L40.0,75.0 L120.0,75.0 Z"
    );
}

#[test]
fn test_right_triangle_default_path_keeps_the_right_angle_on_the_left() {
    let adj = HashMap::new();
    let path = preset_shape_svg("rtTriangle", 120.0, 100.0, &adj).unwrap();

    assert_eq!(path, "M0,0 L120.0,100.0 L0,100.0 Z");
}

#[test]
fn test_up_arrow_default_path_widens_the_shaft() {
    let adj = HashMap::new();
    let path = up_arrow_path(120.0, 100.0, &adj);

    assert_eq!(
        path,
        "M15.0,100.0 L15.0,50.0 L0,50.0 L60.0,0 L120.0,50.0 L105.0,50.0 L105.0,100.0 Z"
    );
}

#[test]
fn test_down_arrow_default_path_widens_the_shaft() {
    let adj = HashMap::new();
    let path = down_arrow_path(120.0, 100.0, &adj);

    assert_eq!(
        path,
        "M15.0,0 L105.0,0 L105.0,50.0 L120.0,50.0 L60.0,100.0 L0,50.0 L15.0,50.0 Z"
    );
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
    let path = left_circular_arrow_path(100.0, 140.0);

    assert!(path.contains("59.6,133.6"));
    assert!(path.contains("6.3,85.1"));
    assert!(path.contains("99.5,80.1"));
    assert!(path.contains("64.8,132.1"));
}

#[test]
fn test_left_right_circular_arrow_default_path_tracks_arch_reference() {
    let path = left_right_circular_arrow_path(100.0, 140.0);

    assert!(path.contains("46.8,4.9"));
    assert!(path.contains("78.1,18.9"));
    assert!(path.contains("13.2,70.8"));
    assert!(path.contains("0.5,60.2"));
}

#[test]
fn test_circular_arrow_default_path_tracks_office_arc_span() {
    let adj = HashMap::new();
    let path = circular_arrow_path(160.0, 100.0, &adj);

    assert!(path.contains("53.2,22.5"));
    assert!(path.contains("6.0,50.9"));
    assert!(path.contains("150.6,29.5"));
    assert!(path.contains("143.7,48.0"));
}

#[test]
fn test_curved_right_arrow_default_path_tracks_reference_c_shape() {
    let adj = HashMap::new();
    let path = curved_right_arrow_path(100.0, 140.0, &adj);

    assert_eq!(
        path,
        "M 0.0,54.3 L 0.0,54.3 C 0.0,63.9 4.7,73.3 13.4,81.5 22.2,89.8 34.8,96.6 50.0,101.4 57.8,103.9 66.3,105.7 75.0,107.0 L 75.0,96.0 100.0,119.7 75.0,140.0 75.0,129.0 75.0,129.0 C 66.2,127.8 57.8,125.9 50.0,123.4 34.8,118.7 22.2,111.8 13.4,103.5 4.6,95.3 0.0,85.9 0.0,76.4 L 0.0,54.3 0.0,54.3 C 0.0,44.8 4.7,35.4 13.4,27.2 22.2,18.9 34.8,12.1 50.0,7.3 65.2,2.5 82.4,0.0 100.0,0.0 L 100.0,22.0 100.0,22.0 C 82.4,22.0 65.2,24.5 50.0,29.3 34.8,34.1 22.2,41.0 13.4,49.2 8.1,54.2 4.3,59.7 2.1,65.4"
    );
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
    for (adj1, adj2, anchor) in [
        (3_000_000.0, 12_000_000.0, PIE_ADJ_SMALL_NORMALIZED_PATH),
        (5_400_000.0, 16_200_000.0, PIE_ADJ_HALF_NORMALIZED_PATH),
        (0.0, 18_000_000.0, PIE_ADJ_WIDE_NORMALIZED_PATH),
        (9_000_000.0, 11_000_000.0, PIE_ADJ_SLIVER_NORMALIZED_PATH),
    ] {
        let adj = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("pie", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "pie benchmark profile ({adj1}, {adj2}) should map to the tuned anchor path"
        );
    }
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
    for (adj1, adj2, anchor) in [
        (3_000_000.0, 12_000_000.0, ARC_ADJ_SMALL_NORMALIZED_PATH),
        (5_400_000.0, 16_200_000.0, ARC_ADJ_HALF_NORMALIZED_PATH),
        (0.0, 18_000_000.0, ARC_ADJ_WIDE_NORMALIZED_PATH),
        (9_000_000.0, 11_000_000.0, ARC_ADJ_SLIVER_NORMALIZED_PATH),
    ] {
        let adj = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("arc", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "arc benchmark profile ({adj1}, {adj2}) should map to the tuned anchor path"
        );
    }
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
    let default_adj = HashMap::new();
    let path = preset_shape_svg("blockArc", 120.0, 100.0, &default_adj).unwrap();

    let ys: Vec<f64> = path
        .split(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
        .filter(|token| !token.is_empty())
        .skip(1)
        .step_by(2)
        .map(|token| token.parse::<f64>().unwrap())
        .collect();

    assert!(
        path.contains('C'),
        "blockArc default should preserve the extracted curved band outline"
    );
    assert!(
        ys.iter().copied().fold(f64::NEG_INFINITY, f64::max) <= 50.1,
        "blockArc default should stay in the upper half of its box: {path}"
    );
    assert!(
        ys.iter().copied().fold(f64::INFINITY, f64::min) <= 0.1,
        "blockArc default should still reach the top edge: {path}"
    );
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
    for (adj1, adj2, adj3, anchor) in [
        (
            12_000.0,
            8_500_000.0,
            17_000_000.0,
            BLOCK_ARC_ADJ_NARROW_NORMALIZED_PATH,
        ),
        (
            35_000.0,
            3_000_000.0,
            13_000_000.0,
            BLOCK_ARC_ADJ_WIDE_NORMALIZED_PATH,
        ),
        (
            50_000.0,
            0.0,
            21_600_000.0,
            BLOCK_ARC_ADJ_RING_NORMALIZED_PATH,
        ),
        (
            25_000.0,
            6_000_000.0,
            18_000_000.0,
            BLOCK_ARC_ADJ_OFFSET_NORMALIZED_PATH,
        ),
    ] {
        let adj = HashMap::from([
            ("adj1".to_string(), adj1),
            ("adj2".to_string(), adj2),
            ("adj3".to_string(), adj3),
        ]);
        let path = preset_shape_svg("blockArc", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "blockArc benchmark profile ({adj1}, {adj2}, {adj3}) should map to the tuned anchor path"
        );
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
    for (adj1, adj5, anchor) in [
        (
            -20_000.0,
            10_000.0,
            CIRCULAR_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
        ),
        (25_000.0, 35_000.0, CIRCULAR_ARROW_ADJ_WIDE_NORMALIZED_PATH),
        (45_000.0, 15_000.0, CIRCULAR_ARROW_ADJ_SWEEP_NORMALIZED_PATH),
        (12_500.0, 45_000.0, CIRCULAR_ARROW_ADJ_THICK_NORMALIZED_PATH),
    ] {
        let adj = HashMap::from([("adj1".to_string(), adj1), ("adj5".to_string(), adj5)]);
        let path = preset_shape_svg("circularArrow", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "circularArrow benchmark profile ({adj1}, {adj5}) should map to the tuned anchor path"
        );
    }
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
    let make_adj = |adj1: f64, adj2: f64, adj3: f64| {
        HashMap::from([
            ("adj1".to_string(), adj1),
            ("adj2".to_string(), adj2),
            ("adj3".to_string(), adj3),
        ])
    };

    let tight_adj = make_adj(12_000.0, 70_000.0, 18_000.0);
    let wide_adj = make_adj(42_000.0, 30_000.0, 42_000.0);

    for (preset, tight_anchor, wide_anchor) in [
        (
            "curvedRightArrow",
            CURVED_RIGHT_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
            CURVED_RIGHT_ARROW_ADJ_WIDE_NORMALIZED_PATH,
        ),
        (
            "curvedLeftArrow",
            CURVED_LEFT_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
            CURVED_LEFT_ARROW_ADJ_WIDE_NORMALIZED_PATH,
        ),
        (
            "curvedUpArrow",
            CURVED_UP_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
            CURVED_UP_ARROW_ADJ_WIDE_NORMALIZED_PATH,
        ),
        (
            "curvedDownArrow",
            CURVED_DOWN_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
            CURVED_DOWN_ARROW_ADJ_WIDE_NORMALIZED_PATH,
        ),
    ] {
        let tight_path = preset_shape_svg(preset, 120.0, 100.0, &tight_adj).unwrap();
        let wide_path = preset_shape_svg(preset, 120.0, 100.0, &wide_adj).unwrap();

        assert_eq!(
            tight_path,
            scale_normalized_path(tight_anchor, 120.0, 100.0),
            "{preset} tight benchmark profile should map to the tuned anchor path"
        );
        assert_eq!(
            wide_path,
            scale_normalized_path(wide_anchor, 120.0, 100.0),
            "{preset} wide benchmark profile should map to the tuned anchor path"
        );
    }
}

#[test]
fn test_curved_arrow_multi_svg_is_available_for_left_up_down_benchmark_profiles() {
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

    for preset in ["curvedLeftArrow", "curvedUpArrow", "curvedDownArrow"] {
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

    assert!(
        preset_shape_multi_svg("curvedRightArrow", 120.0, 100.0, &tight_adj).is_none(),
        "curvedRightArrow should keep the single-path renderer for now"
    );
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
    for (adj1, adj2, anchor) in [
        (10_000.0, 0.0, WAVE_ADJ_LIGHT_NORMALIZED_PATH),
        (12_500.0, 40_000.0, WAVE_ADJ_SHIFT_NORMALIZED_PATH),
        (30_000.0, 0.0, WAVE_ADJ_DEEP_NORMALIZED_PATH),
        (30_000.0, 40_000.0, WAVE_ADJ_DEEP_SHIFT_NORMALIZED_PATH),
    ] {
        let adj = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("wave", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "wave benchmark profile ({adj1}, {adj2}) should map to the tuned anchor path"
        );
    }
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
    for (adj1, adj2, anchor) in [
        (10_000.0, 0.0, DOUBLE_WAVE_ADJ_LIGHT_NORMALIZED_PATH),
        (12_500.0, 40_000.0, DOUBLE_WAVE_ADJ_SHIFT_NORMALIZED_PATH),
        (30_000.0, 0.0, DOUBLE_WAVE_ADJ_DEEP_NORMALIZED_PATH),
        (
            30_000.0,
            40_000.0,
            DOUBLE_WAVE_ADJ_DEEP_SHIFT_NORMALIZED_PATH,
        ),
    ] {
        let adj = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("doubleWave", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "doubleWave benchmark profile ({adj1}, {adj2}) should map to the tuned anchor path"
        );
    }
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
    for (adj1, adj2, adj3, adj4, anchor) in [
        (
            15_000.0,
            15_000.0,
            15_000.0,
            35_000.0,
            BENT_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
        ),
        (
            35_000.0,
            35_000.0,
            35_000.0,
            50_000.0,
            BENT_ARROW_ADJ_WIDE_NORMALIZED_PATH,
        ),
        (
            20_000.0,
            20_000.0,
            50_000.0,
            65_000.0,
            BENT_ARROW_ADJ_TALL_NORMALIZED_PATH,
        ),
        (
            45_000.0,
            15_000.0,
            25_000.0,
            25_000.0,
            BENT_ARROW_ADJ_THICK_NORMALIZED_PATH,
        ),
    ] {
        let adj = HashMap::from([
            ("adj1".to_string(), adj1),
            ("adj2".to_string(), adj2),
            ("adj3".to_string(), adj3),
            ("adj4".to_string(), adj4),
        ]);
        let path = preset_shape_svg("bentArrow", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "bentArrow benchmark profile ({adj1}, {adj2}, {adj3}, {adj4}) should map to the tuned anchor path"
        );
    }
}

#[test]
fn test_notched_right_arrow_default_path_preserves_legacy_polygon() {
    let path = preset_shape_svg("notchedRightArrow", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_eq!(
        path,
        "M0,25.0 L80.0,25.0 L80.0,0 L120.0,50.0 L80.0,100.0 L80.0,75.0 L0,75.0 L20.0,50.0 Z"
    );
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
    for (adj1, adj2, anchor) in [
        (
            15_000.0,
            15_000.0,
            NOTCHED_RIGHT_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
        ),
        (
            35_000.0,
            35_000.0,
            NOTCHED_RIGHT_ARROW_ADJ_WIDE_NORMALIZED_PATH,
        ),
        (
            20_000.0,
            50_000.0,
            NOTCHED_RIGHT_ARROW_ADJ_LONG_NORMALIZED_PATH,
        ),
        (
            45_000.0,
            20_000.0,
            NOTCHED_RIGHT_ARROW_ADJ_THICK_NORMALIZED_PATH,
        ),
    ] {
        let adj = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("notchedRightArrow", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "notchedRightArrow benchmark profile ({adj1}, {adj2}) should map to the tuned anchor path"
        );
    }
}

#[test]
fn test_striped_right_arrow_default_path_preserves_legacy_polygon_and_stripes() {
    let path = preset_shape_svg("stripedRightArrow", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_eq!(
        path,
        "M0,25.0 L3.0,25.0 L3.0,75.0 L0,75.0 Z M6.0,25.0 L9.0,25.0 L9.0,75.0 L6.0,75.0 Z M12.0,25.0 L80.0,25.0 L80.0,0 L120.0,50.0 L80.0,100.0 L80.0,75.0 L12.0,75.0 Z"
    );
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
    for (adj1, adj2, anchor) in [
        (
            15_000.0,
            15_000.0,
            STRIPED_RIGHT_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
        ),
        (
            35_000.0,
            35_000.0,
            STRIPED_RIGHT_ARROW_ADJ_WIDE_NORMALIZED_PATH,
        ),
        (
            20_000.0,
            50_000.0,
            STRIPED_RIGHT_ARROW_ADJ_LONG_NORMALIZED_PATH,
        ),
        (
            45_000.0,
            20_000.0,
            STRIPED_RIGHT_ARROW_ADJ_THICK_NORMALIZED_PATH,
        ),
    ] {
        let adj = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("stripedRightArrow", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "stripedRightArrow benchmark profile ({adj1}, {adj2}) should map to the tuned anchor path"
        );
    }
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
    for (adj1, adj2, adj3, anchor) in [
        (
            15_000.0,
            15_000.0,
            15_000.0,
            BENT_UP_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
        ),
        (
            35_000.0,
            35_000.0,
            40_000.0,
            BENT_UP_ARROW_ADJ_WIDE_NORMALIZED_PATH,
        ),
        (
            25_000.0,
            15_000.0,
            50_000.0,
            BENT_UP_ARROW_ADJ_TALL_NORMALIZED_PATH,
        ),
        (
            20_000.0,
            45_000.0,
            20_000.0,
            BENT_UP_ARROW_ADJ_DEEP_NORMALIZED_PATH,
        ),
    ] {
        let adj = HashMap::from([
            ("adj1".to_string(), adj1),
            ("adj2".to_string(), adj2),
            ("adj3".to_string(), adj3),
        ]);
        let path = preset_shape_svg("bentUpArrow", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "bentUpArrow benchmark profile ({adj1}, {adj2}, {adj3}) should map to the tuned anchor path"
        );
    }
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
    for (adj1, adj2, adj3, anchor) in [
        (
            15_000.0,
            15_000.0,
            15_000.0,
            LEFT_RIGHT_UP_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
        ),
        (
            35_000.0,
            35_000.0,
            35_000.0,
            LEFT_RIGHT_UP_ARROW_ADJ_WIDE_NORMALIZED_PATH,
        ),
        (
            20_000.0,
            20_000.0,
            50_000.0,
            LEFT_RIGHT_UP_ARROW_ADJ_TALL_NORMALIZED_PATH,
        ),
        (
            45_000.0,
            15_000.0,
            25_000.0,
            LEFT_RIGHT_UP_ARROW_ADJ_THICK_NORMALIZED_PATH,
        ),
    ] {
        let adj = HashMap::from([
            ("adj1".to_string(), adj1),
            ("adj2".to_string(), adj2),
            ("adj3".to_string(), adj3),
        ]);
        let path = preset_shape_svg("leftRightUpArrow", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "leftRightUpArrow benchmark profile ({adj1}, {adj2}, {adj3}) should map to the tuned anchor path"
        );
    }
}

#[test]
fn test_left_right_up_arrow_default_path_matches_extracted_polygon() {
    let adj = HashMap::new();
    let path = left_right_up_arrow_path(120.0, 100.0, &adj);

    assert_eq!(
        path,
        "M0.0,75.0 L18.6,50.0 L18.6,62.5 L50.7,62.5 L50.7,25.0 L41.3,25.0 L60.0,0.0 L78.6,25.0 L69.3,25.0 L69.3,62.5 L101.3,62.5 L101.3,50.0 L120.0,75.0 L101.3,100.0 L101.3,87.5 L18.6,87.5 L18.6,100.0 Z"
    );
}

#[test]
fn test_quad_arrow_adjustment_profiles_match_benchmarked_anchors() {
    for (adj1, adj2, adj3, anchor) in [
        (
            15_000.0,
            15_000.0,
            15_000.0,
            QUAD_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
        ),
        (
            35_000.0,
            35_000.0,
            35_000.0,
            QUAD_ARROW_ADJ_WIDE_NORMALIZED_PATH,
        ),
        (
            20_000.0,
            20_000.0,
            50_000.0,
            QUAD_ARROW_ADJ_TALL_NORMALIZED_PATH,
        ),
        (
            45_000.0,
            15_000.0,
            25_000.0,
            QUAD_ARROW_ADJ_THICK_NORMALIZED_PATH,
        ),
    ] {
        let adj = HashMap::from([
            ("adj1".to_string(), adj1),
            ("adj2".to_string(), adj2),
            ("adj3".to_string(), adj3),
        ]);
        let path = preset_shape_svg("quadArrow", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "quadArrow benchmark profile ({adj1}, {adj2}, {adj3}) should map to the tuned anchor path"
        );
    }
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
    for (adj1, adj2, adj3, anchor) in [
        (
            15_000.0,
            15_000.0,
            15_000.0,
            LEFT_UP_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
        ),
        (
            35_000.0,
            35_000.0,
            45_000.0,
            LEFT_UP_ARROW_ADJ_WIDE_NORMALIZED_PATH,
        ),
        (
            20_000.0,
            20_000.0,
            50_000.0,
            LEFT_UP_ARROW_ADJ_LONG_NORMALIZED_PATH,
        ),
        (
            40_000.0,
            15_000.0,
            25_000.0,
            LEFT_UP_ARROW_ADJ_THICK_NORMALIZED_PATH,
        ),
    ] {
        let adj = HashMap::from([
            ("adj1".to_string(), adj1),
            ("adj2".to_string(), adj2),
            ("adj3".to_string(), adj3),
        ]);
        let path = preset_shape_svg("leftUpArrow", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "leftUpArrow benchmark profile ({adj1}, {adj2}, {adj3}) should map to the tuned anchor path"
        );
    }
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
    for (adj1, adj2, adj3, adj4, adj5, anchor) in [
        (
            15_000.0,
            15_000.0,
            15_000.0,
            35_000.0,
            65_000.0,
            UTURN_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
        ),
        (
            35_000.0,
            30_000.0,
            35_000.0,
            50_000.0,
            85_000.0,
            UTURN_ARROW_ADJ_WIDE_NORMALIZED_PATH,
        ),
        (
            20_000.0,
            12_000.0,
            25_000.0,
            65_000.0,
            85_000.0,
            UTURN_ARROW_ADJ_SHALLOW_NORMALIZED_PATH,
        ),
        (
            25_000.0,
            45_000.0,
            10_000.0,
            25_000.0,
            70_000.0,
            UTURN_ARROW_ADJ_DEEP_NORMALIZED_PATH,
        ),
    ] {
        let adj = HashMap::from([
            ("adj1".to_string(), adj1),
            ("adj2".to_string(), adj2),
            ("adj3".to_string(), adj3),
            ("adj4".to_string(), adj4),
            ("adj5".to_string(), adj5),
        ]);
        let path = preset_shape_svg("uturnArrow", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "uturnArrow benchmark profile ({adj1}, {adj2}, {adj3}, {adj4}, {adj5}) should map to the tuned anchor path"
        );
    }
}

#[test]
fn test_uturn_arrow_default_path_matches_extracted_office_outline() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("uturnArrow", 120.0, 100.0, &default_adj).unwrap();

    assert_eq!(
        path,
        "M 31.9,97.2 L 3.4,96.2 3.4,41.8 5.3,32.2 14.9,17.4 30.0,7.2 47.3,2.8 66.5,4.0 82.3,10.4 93.1,19.4 101.3,33.4 102.7,49.0 116.6,50.2 87.6,73.6 59.5,50.2 73.4,48.6 73.0,40.6 69.1,33.8 57.8,27.6 43.4,29.2 37.0,33.8 33.1,41.0 Z"
    );
}

#[test]
fn test_swoosh_arrow_default_path_matches_extracted_office_outline() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("swooshArrow", 120.0, 100.0, &default_adj).unwrap();

    assert_eq!(
        path,
        "M 4.4,97.3 L 3.4,95.9 7.8,83.5 16.7,67.3 29.5,51.9 42.0,41.1 70.7,24.9 103.5,14.6 103.5,2.7 116.6,21.4 107.5,50.8 106.1,40.3 103.1,39.5 81.2,43.2 53.7,52.4 38.8,60.5 23.0,73.0 Z"
    );
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
    for (adj1, adj2, anchor) in [
        (-20_000.0, 30_000.0, CLOUD_CALLOUT_ADJ_LEFT_NORMALIZED_PATH),
        (20_000.0, 30_000.0, CLOUD_CALLOUT_ADJ_RIGHT_NORMALIZED_PATH),
        (0.0, 80_000.0, CLOUD_CALLOUT_ADJ_LOW_NORMALIZED_PATH),
        (0.0, 10_000.0, CLOUD_CALLOUT_ADJ_HIGH_NORMALIZED_PATH),
    ] {
        let adj = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("cloudCallout", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "cloudCallout benchmark profile ({adj1}, {adj2}) should map to the tuned anchor path"
        );
    }
}

#[test]
fn test_wedge_round_rect_callout_default_path_preserves_legacy_polygon() {
    let path = preset_shape_svg("wedgeRoundRectCallout", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_eq!(
        path,
        "M6.0,0 L114.0,0 Q120.0,0 120.0,6.0 L120.0,94.0 Q120.0,100.0 114.0,100.0 L24.0,100.0 L35.0,62.5 L12.0,100.0 L6.0,100.0 Q0,100.0 0,94.0 L0,6.0 Q0,0 6.0,0 Z"
    );
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
    for (adj1, adj2) in [(-20_833.0, 62_500.0), (0.0, 62_500.0)] {
        let adj_values = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("wedgeRoundRectCallout", 120.0, 100.0, &adj_values).unwrap();
        assert_eq!(
            path,
            wedge_round_rect_callout_analytic_path(120.0, 100.0, &adj_values),
            "wedgeRoundRectCallout low-tail profile ({adj1}, {adj2}) should stay on the analytic branch"
        );
    }

    for ((adj1, adj2), anchor) in [
        (
            (20_833.0, 20_000.0),
            WEDGE_ROUND_RECT_CALLOUT_ADJ_RIGHT_HIGH_NORMALIZED_PATH,
        ),
        (
            (-30_000.0, 15_000.0),
            WEDGE_ROUND_RECT_CALLOUT_ADJ_LEFT_HIGH_NORMALIZED_PATH,
        ),
    ] {
        let adj_values = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("wedgeRoundRectCallout", 120.0, 100.0, &adj_values).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "wedgeRoundRectCallout high-tail profile ({adj1}, {adj2}) should map to the tuned anchor path"
        );
    }
}

#[test]
fn test_wedge_ellipse_callout_default_path_preserves_legacy_multipath() {
    let path = preset_shape_svg("wedgeEllipseCallout", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_eq!(
        path,
        "M60.0,0 A60.0,50.0 0 1,1 60.0,100.0 A60.0,50.0 0 1,1 60.0,0 Z M42.0,93.0 L35.0,62.5 L54.0,93.0"
    );
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
    for (adj1, adj2) in [(-20_833.0, 62_500.0), (0.0, 62_500.0)] {
        let adj_values = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("wedgeEllipseCallout", 120.0, 100.0, &adj_values).unwrap();
        assert_eq!(
            path,
            wedge_ellipse_callout_analytic_path(120.0, 100.0, &adj_values),
            "wedgeEllipseCallout low-tail profile ({adj1}, {adj2}) should stay on the analytic branch"
        );
    }

    for ((adj1, adj2), anchor) in [
        (
            (20_833.0, 20_000.0),
            WEDGE_ELLIPSE_CALLOUT_ADJ_RIGHT_HIGH_NORMALIZED_PATH,
        ),
        (
            (-30_000.0, 15_000.0),
            WEDGE_ELLIPSE_CALLOUT_ADJ_LEFT_HIGH_NORMALIZED_PATH,
        ),
    ] {
        let adj_values = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("wedgeEllipseCallout", 120.0, 100.0, &adj_values).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "wedgeEllipseCallout high-tail profile ({adj1}, {adj2}) should map to the tuned anchor path"
        );
    }
}

#[test]
fn test_wedge_rect_callout_default_path_preserves_legacy_polygon() {
    let path = preset_shape_svg("wedgeRectCallout", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_eq!(
        path,
        "M0,0 L120.0,0 L120.0,100.0 L24.0,100.0 L35.0,62.5 L12.0,100.0 L0,100.0 Z"
    );
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
    for (adj1, adj2) in [(-20_833.0, 62_500.0), (0.0, 62_500.0)] {
        let adj_values = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("wedgeRectCallout", 120.0, 100.0, &adj_values).unwrap();
        assert_eq!(
            path,
            wedge_rect_callout_analytic_path(120.0, 100.0, &adj_values),
            "wedgeRectCallout low-tail profile ({adj1}, {adj2}) should stay on the analytic branch"
        );
    }

    for ((adj1, adj2), anchor) in [
        (
            (20_833.0, 20_000.0),
            WEDGE_RECT_CALLOUT_ADJ_RIGHT_HIGH_NORMALIZED_PATH,
        ),
        (
            (-30_000.0, 15_000.0),
            WEDGE_RECT_CALLOUT_ADJ_LEFT_HIGH_NORMALIZED_PATH,
        ),
    ] {
        let adj_values = HashMap::from([("adj1".to_string(), adj1), ("adj2".to_string(), adj2)]);
        let path = preset_shape_svg("wedgeRectCallout", 120.0, 100.0, &adj_values).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "wedgeRectCallout high-tail profile ({adj1}, {adj2}) should map to the tuned anchor path"
        );
    }
}

#[test]
fn test_math_not_equal_adjust_values_change_path() {
    let default_adj = HashMap::new();
    let mut custom_adj = HashMap::new();
    custom_adj.insert("adj2".to_string(), 9600000.0);

    let default_path = preset_shape_svg("mathNotEqual", 120.0, 100.0, &default_adj).unwrap();
    let custom_path = preset_shape_svg("mathNotEqual", 120.0, 100.0, &custom_adj).unwrap();

    assert_ne!(
        default_path, custom_path,
        "mathNotEqual adj2 should change the path"
    );
}

#[test]
fn test_math_not_equal_default_path_matches_extracted_office_polygon() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("mathNotEqual", 120.0, 100.0, &default_adj).unwrap();

    assert_eq!(
        path,
        "M 54.0,99.2 L 33.0,91.3 L 32.1,88.3 L 34.7,80.2 L 5.7,80.2 L 1.4,78.8 L 1.7,54.5 L 44.9,54.5 L 47.5,46.4 L 1.7,45.0 L 0.9,21.8 L 2.3,20.4 L 57.4,20.4 L 65.4,0.8 L 87.9,8.4 L 84.7,20.4 L 118.3,20.4 L 118.3,45.0 L 77.3,45.5 L 74.2,47.5 L 71.9,54.2 L 118.3,54.5 L 119.1,78.2 L 115.5,80.2 L 63.7,80.2 L 61.1,82.7 L 56.0,97.8 L 54.0,99.2 Z"
    );
}

#[test]
fn test_math_divide_default_path_matches_extracted_office_geometry() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("mathDivide", 120.0, 100.0, &default_adj).unwrap();

    assert_eq!(path.matches('M').count(), 3);
    assert!(path.contains("59.2,11.2"));
    assert!(path.contains("104.9,37.9"));
    assert!(path.contains("15.6,37.9"));
}

#[test]
fn test_math_equal_default_path_matches_extracted_office_geometry() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("mathEqual", 120.0, 100.0, &default_adj).unwrap();

    assert_eq!(
        path,
        "M 13.5,19.7 L 106.0,19.7 106.0,49.1 13.5,49.1 13.5,19.7 Z M 13.5,54.9 L 106.0,54.9 106.0,84.4 13.5,84.4 13.5,54.9 Z"
    );
}

#[test]
fn test_math_plus_default_path_matches_extracted_office_geometry() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("mathPlus", 120.0, 100.0, &default_adj).unwrap();

    assert_eq!(
        path,
        "M 42.3,11.1 L 77.7,11.1 77.7,40.8 108.0,40.8 108.0,59.2 77.7,59.2 77.7,88.9 42.3,88.9 42.3,59.2 12.0,59.2 12.0,40.8 42.3,40.8 42.3,11.1 Z"
    );
}

#[test]
fn test_plus_default_path_matches_benchmark_cross_outline() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("plus", 120.0, 100.0, &default_adj).unwrap();

    assert_eq!(
        path,
        "M 88.0,100.2 L 30.5,99.8 28.7,97.9 28.7,77.2 27.5,76.1 4.0,76.1 -0.2,73.9 -0.2,26.1 1.0,23.9 28.7,23.2 28.7,2.5 30.0,-0.2 89.0,-0.2 90.8,2.1 91.3,23.2 118.0,23.9 119.8,25.7 120.2,73.4 119.0,75.3 115.5,76.1 92.0,76.1 90.8,78.0 90.8,98.3 Z"
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
        "M 12.0,36.3 L 108.0,36.3 108.0,63.7 12.0,63.7 12.0,36.3 Z"
    );
}

#[test]
fn test_curved_right_arrow_default_path_matches_extracted_office_outline() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("curvedRightArrow", 120.0, 100.0, &default_adj).unwrap();

    assert!(path.contains("120.0,85.5"));
    assert!(path.contains("90.0,100.0"));
    assert!(path.contains("0.0,54.5"));
}

#[test]
fn test_curved_left_arrow_default_path_matches_extracted_office_outline() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("curvedLeftArrow", 120.0, 100.0, &default_adj).unwrap();

    assert!(path.contains("120.0,56.5"));
    assert!(path.contains("0.0,75.3"));
    assert!(path.contains("18.6,100.0"));
}

#[test]
fn test_curved_up_arrow_default_path_matches_extracted_office_outline() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("curvedUpArrow", 120.0, 100.0, &default_adj).unwrap();

    assert!(path.contains("90.9,0.0"));
    assert!(path.contains("120.0,25.0"));
    assert!(path.contains("37.9,100.0"));
}

#[test]
fn test_curved_down_arrow_default_path_matches_extracted_office_outline() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("curvedDownArrow", 120.0, 100.0, &default_adj).unwrap();

    assert!(path.contains("90.3,100.0"));
    assert!(path.contains("120.0,84.4"));
    assert!(path.contains("52.7,8.4"));
}

#[test]
fn test_gear6_default_path_matches_extracted_office_outline() {
    let path = preset_shape_svg("gear6", 120.0, 100.0, &HashMap::new()).unwrap();

    assert!(path.contains("60.0,0.0"));
    assert!(path.contains("115.3,27.5"));
    assert!(path.contains("60.0,100.0"));
    assert!(
        !path.contains(" A"),
        "gear6 default silhouette should not cut a center hole"
    );
}

#[test]
fn test_gear9_default_path_matches_extracted_office_outline() {
    let path = preset_shape_svg("gear9", 120.0, 100.0, &HashMap::new()).unwrap();

    assert!(path.contains("60.0,0.0"));
    assert!(path.contains("120.0,44.7"));
    assert!(path.contains("82.8,96.4"));
    assert!(
        !path.contains(" A"),
        "gear9 default silhouette should not cut a center hole"
    );
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

    assert!(path.contains("79.7,2.3"));
    assert!(path.contains("120.0,43.0"));
    assert!(path.contains("60.2,50.6"));
    assert!(path.contains("60.2,0.0"));
}

#[test]
fn test_no_smoking_default_path_carves_inner_ring_hole() {
    let path = preset_shape_svg("noSmoking", 120.0, 100.0, &HashMap::new()).unwrap();

    assert!(path.matches('M').count() >= 3);
    assert!(path.contains("60.0,18.8"));
    assert!(path.contains("24.2,8.0"));
    assert!(path.contains("95.8,92.0"));
}

#[test]
fn test_chord_default_path_matches_office_outline() {
    let path = preset_shape_svg("chord", 120.0, 100.0, &HashMap::new()).unwrap();
    assert!(path.contains("14.5,16.8"));
    assert!(path.contains("61.3,0.0"));
    assert!(path.contains("101.8,81.0"));
    assert!(path.contains("0.0,59.5"));
}

#[test]
fn test_can_default_uses_filled_top_ellipse_without_evenodd_hole() {
    let path = preset_shape_svg("can", 120.0, 100.0, &HashMap::new()).unwrap();

    assert!(!needs_evenodd_fill("can"));
    assert!(path.matches('M').count() >= 2);
    assert!(path.contains("M0,25.0"));
    assert!(path.contains("120.0,25.0 A60.0,25.0 0 0,0 0,25.0 Z"));
}

#[test]
fn test_pie_wedge_default_path_matches_reference_orientation() {
    let path = preset_shape_svg("pieWedge", 120.0, 100.0, &HashMap::new()).unwrap();

    assert_eq!(
        path,
        "M0,100.0 L120.0,100.0 L120.0,0 A120.0,100.0 0 0,0 0,100.0 Z"
    );
}

#[test]
fn test_quad_arrow_callout_adjustment_profiles_match_benchmarked_anchors() {
    for (adj1, adj2, adj3, adj4, anchor) in [
        (
            15_000.0,
            15_000.0,
            15_000.0,
            15_000.0,
            QUAD_ARROW_CALLOUT_ADJ_TIGHT_NORMALIZED_PATH,
        ),
        (
            35_000.0,
            35_000.0,
            35_000.0,
            35_000.0,
            QUAD_ARROW_CALLOUT_ADJ_WIDE_NORMALIZED_PATH,
        ),
        (
            20_000.0,
            50_000.0,
            25_000.0,
            50_000.0,
            QUAD_ARROW_CALLOUT_ADJ_LONG_NORMALIZED_PATH,
        ),
        (
            45_000.0,
            20_000.0,
            45_000.0,
            20_000.0,
            QUAD_ARROW_CALLOUT_ADJ_THICK_NORMALIZED_PATH,
        ),
    ] {
        let adj = HashMap::from([
            ("adj1".to_string(), adj1),
            ("adj2".to_string(), adj2),
            ("adj3".to_string(), adj3),
            ("adj4".to_string(), adj4),
        ]);
        let path = preset_shape_svg("quadArrowCallout", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "quadArrowCallout benchmark profile ({adj1}, {adj2}, {adj3}, {adj4}) should map to the tuned anchor path"
        );
    }
}

#[test]
fn test_left_right_arrow_callout_default_path_matches_office_outline() {
    let path = preset_shape_svg("leftRightArrowCallout", 120.0, 100.0, &HashMap::new()).unwrap();
    assert!(path.contains("30.8,100.0"));
    assert!(path.contains("89.7,0.0"));
    assert!(path.contains("0.0,48.8"));
}

#[test]
fn test_left_right_arrow_callout_adjustment_profiles_match_benchmarked_anchors() {
    for (adj1, adj2, adj3, adj4, anchor) in [
        (
            15_000.0,
            15_000.0,
            15_000.0,
            15_000.0,
            LEFT_RIGHT_ARROW_CALLOUT_ADJ_TIGHT_NORMALIZED_PATH,
        ),
        (
            35_000.0,
            35_000.0,
            35_000.0,
            35_000.0,
            LEFT_RIGHT_ARROW_CALLOUT_ADJ_WIDE_NORMALIZED_PATH,
        ),
        (
            20_000.0,
            50_000.0,
            25_000.0,
            50_000.0,
            LEFT_RIGHT_ARROW_CALLOUT_ADJ_LONG_NORMALIZED_PATH,
        ),
        (
            45_000.0,
            20_000.0,
            45_000.0,
            20_000.0,
            LEFT_RIGHT_ARROW_CALLOUT_ADJ_THICK_NORMALIZED_PATH,
        ),
    ] {
        let adj = HashMap::from([
            ("adj1".to_string(), adj1),
            ("adj2".to_string(), adj2),
            ("adj3".to_string(), adj3),
            ("adj4".to_string(), adj4),
        ]);
        let path = preset_shape_svg("leftRightArrowCallout", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "leftRightArrowCallout benchmark profile ({adj1}, {adj2}, {adj3}, {adj4}) should map to the tuned anchor path"
        );
    }
}

#[test]
fn test_teardrop_adjustment_profiles_match_benchmarked_anchors() {
    for (adj, anchor) in [
        (20_000.0, TEARDROP_ADJ_LIGHT_NORMALIZED_PATH),
        (50_000.0, TEARDROP_ADJ_DEFAULT_NORMALIZED_PATH),
        (80_000.0, TEARDROP_ADJ_DEEP_NORMALIZED_PATH),
        (100_000.0, TEARDROP_ADJ_SHARP_NORMALIZED_PATH),
    ] {
        let adj_values = HashMap::from([("adj".to_string(), adj)]);
        let path = preset_shape_svg("teardrop", 120.0, 100.0, &adj_values).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "teardrop benchmark profile ({adj}) should map to the tuned anchor path"
        );
    }
}

#[test]
fn test_up_down_arrow_callout_default_path_matches_office_outline() {
    let path = preset_shape_svg("upDownArrowCallout", 120.0, 100.0, &HashMap::new()).unwrap();
    assert!(path.contains("59.1,0.0"));
    assert!(path.contains("120.0,75.0"));
    assert!(path.contains("0.0,25.6"));
}

#[test]
fn test_up_down_arrow_callout_adjustment_profiles_match_benchmarked_anchors() {
    for (adj1, adj2, adj3, adj4, anchor) in [
        (
            15_000.0,
            15_000.0,
            15_000.0,
            15_000.0,
            UP_DOWN_ARROW_CALLOUT_ADJ_TIGHT_NORMALIZED_PATH,
        ),
        (
            35_000.0,
            35_000.0,
            35_000.0,
            35_000.0,
            UP_DOWN_ARROW_CALLOUT_ADJ_WIDE_NORMALIZED_PATH,
        ),
        (
            20_000.0,
            50_000.0,
            25_000.0,
            50_000.0,
            UP_DOWN_ARROW_CALLOUT_ADJ_LONG_NORMALIZED_PATH,
        ),
        (
            45_000.0,
            20_000.0,
            45_000.0,
            20_000.0,
            UP_DOWN_ARROW_CALLOUT_ADJ_THICK_NORMALIZED_PATH,
        ),
    ] {
        let adj = HashMap::from([
            ("adj1".to_string(), adj1),
            ("adj2".to_string(), adj2),
            ("adj3".to_string(), adj3),
            ("adj4".to_string(), adj4),
        ]);
        let path = preset_shape_svg("upDownArrowCallout", 120.0, 100.0, &adj).unwrap();
        assert_eq!(
            path,
            scale_normalized_path(anchor, 120.0, 100.0),
            "upDownArrowCallout benchmark profile ({adj1}, {adj2}, {adj3}, {adj4}) should map to the tuned anchor path"
        );
    }
}

#[test]
fn test_quad_arrow_callout_default_path_matches_office_outline() {
    let path = preset_shape_svg("quadArrowCallout", 120.0, 100.0, &HashMap::new()).unwrap();
    assert!(path.contains("117.2,52.5"));
    assert!(path.contains("62.5,0.0"));
    assert!(path.contains("0.0,49.3"));
}

#[test]
fn test_math_divide_default_path_uses_circular_dots() {
    let path = preset_shape_svg("mathDivide", 120.0, 100.0, &HashMap::new()).unwrap();

    assert!(path.contains("59.2,11.2"));
    assert!(path.contains("104.9,37.9"));
    assert!(path.contains("15.6,37.9"));
    assert!(path.contains("61.3,88.8"));
}

#[test]
fn test_left_right_ribbon_default_path_matches_office_outline() {
    let path = preset_shape_svg("leftRightRibbon", 120.0, 100.0, &HashMap::new()).unwrap();
    assert!(path.contains("16.6,54.7"));
    assert!(path.contains("56.9,0.0"));
    assert!(path.contains("120.0,57.8"));
    assert!(path.contains("63.1,100.0"));
}

#[test]
fn test_star5_default_path_matches_extracted_reference_outline() {
    let path = preset_shape_svg("star5", 120.0, 100.0, &HashMap::new()).unwrap();

    assert!(path.contains("M 96.8,99.2"));
    assert!(path.contains("L 59.6,0.8"));
    assert!(path.contains("L 118.3,37.4"));
    assert!(path.contains("L 83.9,62.9"));
}

#[test]
fn test_irregular_seal1_default_path_matches_extracted_reference_outline() {
    let path = preset_shape_svg("irregularSeal1", 120.0, 100.0, &HashMap::new()).unwrap();

    assert!(path.contains("M 47.9,99.2"));
    assert!(path.contains("L 80.6,0.8"));
    assert!(path.contains("L 116.8,38.2"));
    assert!(path.contains("L 1.1,66.9"));
}

#[test]
fn test_moon_default_path_matches_extracted_reference_outline() {
    let path = preset_shape_svg("moon", 120.0, 100.0, &HashMap::new()).unwrap();

    assert!(path.contains("M 112.4,99.4"));
    assert!(path.contains("L 0.8,53.9"));
    assert!(path.contains("L 118.5,0.6"));
    assert!(path.contains("L 118.5,99.0"));
}

#[test]
fn test_irregular_seal2_default_path_matches_extracted_reference_outline() {
    let path = preset_shape_svg("irregularSeal2", 120.0, 100.0, &HashMap::new()).unwrap();

    assert!(path.contains("M 28.6,96.9"));
    assert!(path.contains("L 80.2,3.8"));
    assert!(path.contains("L 116.2,31.5"));
    assert!(path.contains("L 48.1,89.3"));
}

#[test]
fn test_cloud_default_path_matches_extracted_reference_outline() {
    let path = preset_shape_svg("cloud", 120.0, 100.0, &HashMap::new()).unwrap();

    assert!(path.contains("M 63.5,99.8"));
    assert!(path.contains("L 25.1,10.3"));
    assert!(path.contains("L 119.0,45.7"));
    assert!(path.contains("L 77.8,87.0"));
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

    assert!(path.contains("36.2,82.8"));
    assert!(path.contains("18.6,68.5"));
    assert!(path.contains("101.9,31.9"));
    assert!(path.contains("60.5,67.7"));
}

#[test]
fn test_bent_up_arrow_default_path_matches_extracted_office_polygon() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("bentUpArrow", 120.0, 100.0, &default_adj).unwrap();

    assert_eq!(
        path,
        "M 0.0,73.7 L 90.6,73.7 90.9,26.0 81.8,24.3 99.9,0.0 119.6,22.8 112.4,31.2 112.4,99.4 0.0,99.4 0.0,73.7 Z"
    );
}

#[test]
fn test_left_up_arrow_default_path_matches_extracted_office_polygon() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("leftUpArrow", 120.0, 100.0, &default_adj).unwrap();

    assert_eq!(
        path,
        "M 4.7,81.4 L 32.3,66.7 32.3,74.0 73.8,74.0 73.8,16.0 60.0,16.0 87.6,1.2 115.3,16.0 101.4,16.0 101.4,88.8 32.3,88.8 32.3,96.2 4.7,81.4 Z"
    );
}

#[test]
fn test_right_brace_default_path_matches_extracted_office_outline() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("rightBrace", 120.0, 100.0, &default_adj).unwrap();

    assert!(path.contains("M 3.6,1.4"));
    assert!(path.contains("116.4,48.4"));
    assert!(path.contains("3.6,95.5"));
}

#[test]
fn test_left_brace_default_path_matches_extracted_office_outline() {
    let default_adj = HashMap::new();
    let path = preset_shape_svg("leftBrace", 120.0, 100.0, &default_adj).unwrap();

    assert!(path.contains("M 116.4,95.5"));
    assert!(path.contains("3.6,48.4"));
    assert!(path.contains("116.4,1.5"));
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
