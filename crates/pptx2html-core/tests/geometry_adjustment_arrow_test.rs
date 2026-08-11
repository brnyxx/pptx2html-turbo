mod arrow_adjustment_support;
mod fixtures;

use arrow_adjustment_support::{
    Case, assert_finite_html, official_cases, official_preset_names, render_html, svg_paths,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

const OFFICIAL_PRESET_COUNT: usize = 55;
const OFFICIAL_PAIR_COUNT: usize = 189;
const PREVIOUSLY_UNCONSUMED_COUNT: usize = 100;

#[test]
fn official_arrow_table_is_independently_joined_to_all_routes() {
    let cases = official_cases();
    assert_eq!(official_preset_names().len(), OFFICIAL_PRESET_COUNT);
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
