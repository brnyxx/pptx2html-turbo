mod arrow_adjustment_support;
mod fixtures;

use arrow_adjustment_support::{
    assert_finite_html, official_cases, official_preset_names, path_numbers, render_html, svg_paths,
};

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
    let cases = official_cases();
    assert_eq!(cases.len(), 189);
    assert!(cases.iter().all(|case| {
        !case.preset.is_empty()
            && !case.key.is_empty()
            && case.default.is_finite()
            && case.lower.is_finite()
            && case.upper.is_finite()
    }));
    for preset in official_preset_names() {
        let html = render_html(preset, &[], 1_524_000, 952_500);
        let paths = svg_paths(&html);
        let main = paths.first().expect("main path");
        if OPEN_PRESETS.contains(&preset) {
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
fn cloud_callout_full_turns_emit_visible_split_arcs() {
    let html = render_html("cloudCallout", &[], 914_400, 914_400);
    let paths = svg_paths(&html);
    assert!(paths.len() >= 4, "cloudCallout layered paths missing");
    for (index, path) in paths[1..4].iter().enumerate() {
        assert_eq!(path.matches('A').count(), 2, "circle {index}: {path}");
        let start = path
            .split_once('M')
            .and_then(|(_, value)| value.split_once(' '))
            .map(|(point, _)| point)
            .expect("circle start");
        let endpoints = path
            .trim_end_matches(" Z")
            .split('A')
            .skip(1)
            .map(|segment| segment.split_whitespace().last().expect("arc endpoint"))
            .collect::<Vec<_>>();
        assert_ne!(endpoints[0], start, "circle {index} first arc disappeared");
        assert_eq!(endpoints[1], start, "circle {index} endpoint drifted");
        assert_finite_html(&html, "cloudCallout full turns");
    }
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
