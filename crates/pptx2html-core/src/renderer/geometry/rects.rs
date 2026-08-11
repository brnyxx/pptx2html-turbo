// Auto-split from renderer/geometry.rs (mechanical move, no logic edits).
// Family: rects

use super::shared::scale_normalized_path;
use std::collections::HashMap;

fn finite(value: Option<f64>, default: f64) -> f64 {
    value.filter(|value| value.is_finite()).unwrap_or(default)
}

pub(super) fn snip1_rect_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let a = finite(adj.get("adj").copied(), 16_667.0).clamp(0.0, 50_000.0);
    let d = w.min(h) * a / 100_000.0;
    format!(
        "M0,0 L{x:.1},0 L{w:.1},{d:.1} L{w:.1},{h:.1} L0,{h:.1} Z",
        x = w - d,
        w = w,
        d = d,
        h = h
    )
}
pub(super) fn snip2_same_rect_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let ss = w.min(h);
    let top = ss * finite(adj.get("adj1").copied(), 16_667.0).clamp(0.0, 50_000.0) / 100_000.0;
    let bottom = ss * finite(adj.get("adj2").copied(), 0.0).clamp(0.0, 50_000.0) / 100_000.0;
    format!(
        "M{top:.1},0 L{x_top:.1},0 L{w:.1},{top:.1} L{w:.1},{y_bottom:.1} L{x_bottom:.1},{h:.1} L{bottom:.1},{h:.1} L0,{y_bottom:.1} L0,{top:.1} Z",
        top = top,
        x_top = w - top,
        x_bottom = w - bottom,
        bottom = bottom,
        y_bottom = h - bottom,
        w = w,
        h = h
    )
}
pub(super) fn snip2_diag_rect_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if adj.contains_key("adj") {
        return scale_normalized_path(snip2_diag_rect_adjust_anchor(adj), w, h);
    }
    if adj.is_empty() {
        let d = w.min(h) * 16_667.0 / 100_000.0;
        return format!(
            "M{d:.1},0 L{x:.1},0 L{w:.1},{d:.1} L{w:.1},{y:.1} L{x:.1},{h:.1} L0,{h:.1} Z",
            d = d,
            x = w - d,
            w = w,
            y = h - d,
            h = h
        );
    }

    let ss = w.min(h);
    let left = ss * finite(adj.get("adj1").copied(), 0.0).clamp(0.0, 50_000.0) / 100_000.0;
    let right = ss * finite(adj.get("adj2").copied(), 16_667.0).clamp(0.0, 50_000.0) / 100_000.0;
    format!(
        "M{left:.1},0 L{x_right:.1},0 L{w:.1},{right:.1} L{w:.1},{y_left:.1} L{x_left:.1},{h:.1} L{right:.1},{h:.1} L0,{y_right:.1} L0,{left:.1} Z",
        left = left,
        right = right,
        x_right = w - right,
        x_left = w - left,
        w = w,
        y_left = h - left,
        y_right = h - right,
        h = h
    )
}
pub(super) const SNIP2_DIAG_RECT_ADJ_LIGHT_NORMALIZED_PATH: &str = r#"M 0.099888,0.000000 L 0.833296,0.000000 1.000000,0.166479 1.000000,0.899888 0.899888,1.000000 0.166479,1.000000 0.000000,0.833296 0.000000,0.099888 0.099888,0.000000 Z"#;
pub(super) const SNIP2_DIAG_RECT_ADJ_DEFAULTISH_NORMALIZED_PATH: &str = r#"M 0.166479,0.000000 L 0.833296,0.000000 1.000000,0.166479 1.000000,0.833296 0.833296,1.000000 0.166479,1.000000 0.000000,0.833296 0.000000,0.166479 0.166479,0.000000 Z"#;
pub(super) const SNIP2_DIAG_RECT_ADJ_DEEP_NORMALIZED_PATH: &str = r#"M 0.299888,0.000000 L 0.833296,0.000000 1.000000,0.166479 1.000000,0.699888 0.699888,1.000000 0.166479,1.000000 0.000000,0.833296 0.000000,0.299888 0.299888,0.000000 Z"#;
pub(super) const SNIP2_DIAG_RECT_ADJ_EXTREME_NORMALIZED_PATH: &str = r#"M 0.449944,0.000000 L 0.833296,0.000000 1.000000,0.166479 1.000000,0.549831 0.549831,1.000000 0.166479,1.000000 0.000000,0.833296 0.000000,0.449944 0.449944,0.000000 Z"#;
pub(super) fn snip2_diag_rect_adjust_anchor(adj: &HashMap<String, f64>) -> &'static str {
    let value = finite(adj.get("adj").copied(), 16_667.0).clamp(0.0, 50_000.0);
    let anchors = [
        (10_000.0, SNIP2_DIAG_RECT_ADJ_LIGHT_NORMALIZED_PATH),
        (16_667.0, SNIP2_DIAG_RECT_ADJ_DEFAULTISH_NORMALIZED_PATH),
        (30_000.0, SNIP2_DIAG_RECT_ADJ_DEEP_NORMALIZED_PATH),
        (45_000.0, SNIP2_DIAG_RECT_ADJ_EXTREME_NORMALIZED_PATH),
    ];

    anchors
        .into_iter()
        .min_by(|(ax, _), (ay, _)| {
            let dx = (value - *ax) / 35_000.0;
            let dy = (value - *ay) / 35_000.0;
            (dx * dx)
                .partial_cmp(&(dy * dy))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, path)| path)
        .unwrap_or(SNIP2_DIAG_RECT_ADJ_DEFAULTISH_NORMALIZED_PATH)
}
pub(super) const SNIP_ROUND_RECT_ADJ_LIGHT_NORMALIZED_PATH: &str = r#"M 0.099888,0.000000 L 0.833296,0.000000 1.000000,0.166479 1.000000,1.000000 0.000000,1.000000 0.000000,0.099888 0.000000,0.099888 C 0.000000,0.082340 0.004724,0.065017 0.013498,0.049944 0.022272,0.034646 0.034871,0.022047 0.049944,0.013273 0.065242,0.004499 0.082340,0.000000 0.100112,0.000000 L 0.099888,0.000000 Z"#;
pub(super) const SNIP_ROUND_RECT_ADJ_DEFAULTISH_NORMALIZED_PATH: &str = r#"M 0.166479,0.000225 L 0.833296,0.000225 1.000000,0.166667 1.000000,1.000000 0.000000,1.000000 0.000000,0.166667 0.000000,0.166667 C 0.000000,0.137427 0.007649,0.108637 0.022272,0.083446 0.036895,0.058030 0.058043,0.036887 0.083240,0.022267 0.108661,0.007647 0.137458,0.000000 0.166704,0.000000 L 0.166479,0.000225 Z"#;
pub(super) const SNIP_ROUND_RECT_ADJ_DEEP_NORMALIZED_PATH: &str = r#"M 0.299888,0.000000 L 0.833296,0.000000 1.000000,0.166479 1.000000,1.000000 0.000000,1.000000 0.000000,0.299888 0.000000,0.299888 C 0.000000,0.247244 0.013948,0.195501 0.040270,0.149831 0.066592,0.104387 0.104387,0.066367 0.150056,0.040045 0.195501,0.013723 0.247244,0.000000 0.300112,0.000000 L 0.299888,0.000000 Z"#;
pub(super) const SNIP_ROUND_RECT_ADJ_EXTREME_NORMALIZED_PATH: &str = r#"M 0.449944,0.000000 L 0.833296,0.000000 1.000000,0.166479 1.000000,1.000000 0.000000,1.000000 0.000000,0.449944 0.000000,0.449944 C 0.000000,0.370979 0.020697,0.293363 0.060292,0.224972 0.099888,0.156580 0.156580,0.099663 0.224972,0.060292 0.293363,0.020697 0.370979,0.000000 0.449944,0.000000 L 0.449944,0.000000 Z"#;
pub(super) fn snip_round_rect_adjust_anchor(adj: &HashMap<String, f64>) -> &'static str {
    let value = finite(adj.get("adj").copied(), 16_667.0).clamp(0.0, 50_000.0);
    let anchors = [
        (10_000.0, SNIP_ROUND_RECT_ADJ_LIGHT_NORMALIZED_PATH),
        (16_667.0, SNIP_ROUND_RECT_ADJ_DEFAULTISH_NORMALIZED_PATH),
        (30_000.0, SNIP_ROUND_RECT_ADJ_DEEP_NORMALIZED_PATH),
        (45_000.0, SNIP_ROUND_RECT_ADJ_EXTREME_NORMALIZED_PATH),
    ];

    anchors
        .into_iter()
        .min_by(|(ax, _), (ay, _)| {
            let dx = (value - *ax) / 35_000.0;
            let dy = (value - *ay) / 35_000.0;
            (dx * dx)
                .partial_cmp(&(dy * dy))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, path)| path)
        .unwrap_or(SNIP_ROUND_RECT_ADJ_DEFAULTISH_NORMALIZED_PATH)
}
pub(super) fn snip_round_rect_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if adj.contains_key("adj") {
        return scale_normalized_path(snip_round_rect_adjust_anchor(adj), w, h);
    }
    if adj.is_empty() {
        let r = w.min(h) * 16_667.0 / 100_000.0;
        return format!(
            "M{r:.1},0 L{x:.1},0 L{w:.1},{r:.1} L{w:.1},{y:.1} Q{w:.1},{h:.1} {x:.1},{h:.1} L{r:.1},{h:.1} Q0,{h:.1} 0,{y:.1} L0,{r:.1} Q0,0 {r:.1},0 Z",
            r = r,
            x = w - r,
            w = w,
            y = h - r,
            h = h
        );
    }

    let ss = w.min(h);
    let r = ss * finite(adj.get("adj1").copied(), 16_667.0).clamp(0.0, 50_000.0) / 100_000.0;
    let snip = ss * finite(adj.get("adj2").copied(), 16_667.0).clamp(0.0, 50_000.0) / 100_000.0;
    format!(
        "M{r:.1},0 L{x:.1},0 L{w:.1},{snip:.1} L{w:.1},{h:.1} L0,{h:.1} L0,{r:.1} Q0,0 {r:.1},0 Z",
        r = r,
        x = w - snip,
        w = w,
        snip = snip,
        h = h
    )
}
pub(super) const ROUND1_RECT_ADJ_LIGHT_NORMALIZED_PATH: &str = r#"M 0.000000,0.000000 L 0.899888,0.000000 0.899888,0.000000 C 0.917435,0.000000 0.934758,0.004724 0.949831,0.013498 0.965129,0.022272 0.977728,0.034871 0.986502,0.049944 0.995276,0.065242 1.000000,0.082340 1.000000,0.100112 L 1.000000,0.100112 1.000000,1.000000 0.000000,1.000000 0.000000,0.000000 Z"#;
pub(super) const ROUND1_RECT_ADJ_DEFAULTISH_NORMALIZED_PATH: &str = r#"M 0.000000,0.000000 L 0.833296,0.000000 0.833296,0.000000 C 0.862542,0.000000 0.891339,0.007649 0.916535,0.022272 0.941957,0.036895 0.963105,0.058043 0.977728,0.083240 0.992351,0.108661 1.000000,0.137458 1.000000,0.166704 L 1.000000,0.166704 1.000000,1.000000 0.000000,1.000000 0.000000,0.000000 Z"#;
pub(super) const ROUND1_RECT_ADJ_DEEP_NORMALIZED_PATH: &str = r#"M 0.000000,0.000000 L 0.699888,0.000000 0.699888,0.000000 C 0.752531,0.000000 0.804274,0.013948 0.849944,0.040270 0.895388,0.066592 0.933408,0.104387 0.959730,0.150056 0.986052,0.195501 1.000000,0.247244 1.000000,0.300112 L 1.000000,0.300112 1.000000,1.000000 0.000000,1.000000 0.000000,0.000000 Z"#;
pub(super) const ROUND1_RECT_ADJ_EXTREME_NORMALIZED_PATH: &str = r#"M 0.000000,0.000000 L 0.549831,0.000000 0.549831,0.000000 C 0.628796,0.000000 0.706412,0.020697 0.774803,0.060292 0.843195,0.099888 0.900112,0.156580 0.939483,0.224972 0.979078,0.293363 0.999775,0.370979 0.999775,0.449944 L 0.999775,0.449944 1.000000,1.000000 0.000000,1.000000 0.000000,0.000000 Z"#;
pub(super) fn round1_rect_adjust_anchor(adj: &HashMap<String, f64>) -> &'static str {
    let value = finite(adj.get("adj").copied(), 16_667.0).clamp(0.0, 50_000.0);
    let anchors = [
        (10_000.0, ROUND1_RECT_ADJ_LIGHT_NORMALIZED_PATH),
        (16_667.0, ROUND1_RECT_ADJ_DEFAULTISH_NORMALIZED_PATH),
        (30_000.0, ROUND1_RECT_ADJ_DEEP_NORMALIZED_PATH),
        (45_000.0, ROUND1_RECT_ADJ_EXTREME_NORMALIZED_PATH),
    ];

    anchors
        .into_iter()
        .min_by(|(ax, _), (ay, _)| {
            let dx = (value - *ax) / 35_000.0;
            let dy = (value - *ay) / 35_000.0;
            (dx * dx)
                .partial_cmp(&(dy * dy))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, path)| path)
        .unwrap_or(ROUND1_RECT_ADJ_DEFAULTISH_NORMALIZED_PATH)
}
pub(super) fn round1_rect_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if !adj.is_empty() {
        return scale_normalized_path(round1_rect_adjust_anchor(adj), w, h);
    }

    let r = w.min(h) * finite(adj.get("adj").copied(), 16_667.0).clamp(0.0, 50_000.0) / 100_000.0;
    format!(
        "M0,0 L{x:.1},0 Q{w:.1},0 {w:.1},{r:.1} L{w:.1},{h:.1} L0,{h:.1} Z",
        x = w - r,
        w = w,
        r = r,
        h = h
    )
}
pub(super) const ROUND2_SAME_RECT_ADJ_LIGHT_NORMALIZED_PATH: &str = r#"M 0.099888,0.000000 L 0.899888,0.000000 0.899888,0.000000 C 0.917435,0.000000 0.934758,0.004724 0.949831,0.013498 0.965129,0.022272 0.977728,0.034871 0.986502,0.049944 0.995276,0.065242 1.000000,0.082340 1.000000,0.100112 L 1.000000,0.100112 1.000000,1.000000 1.000000,1.000000 0.000000,1.000000 0.000000,1.000000 0.000000,0.099888 0.000000,0.099888 C 0.000000,0.082340 0.004724,0.065017 0.013498,0.049944 0.022272,0.034646 0.034871,0.022047 0.049944,0.013273 0.065242,0.004499 0.082340,0.000000 0.100112,0.000000 L 0.099888,0.000000 Z"#;
pub(super) const ROUND2_SAME_RECT_ADJ_DEFAULTISH_NORMALIZED_PATH: &str = r#"M 0.166479,0.000225 L 0.833296,0.000225 0.833296,0.000225 C 0.862542,0.000225 0.891339,0.007872 0.916535,0.022492 0.941957,0.037112 0.963105,0.058255 0.977728,0.083446 0.992351,0.108862 1.000000,0.137652 1.000000,0.166892 L 1.000000,0.166892 1.000000,1.000000 1.000000,1.000000 0.000000,1.000000 0.000000,1.000000 0.000000,0.166667 0.000000,0.166667 C 0.000000,0.137427 0.007649,0.108637 0.022272,0.083446 0.036895,0.058030 0.058043,0.036887 0.083240,0.022267 0.108661,0.007647 0.137458,0.000000 0.166704,0.000000 L 0.166479,0.000225 Z"#;
pub(super) const ROUND2_SAME_RECT_ADJ_DEEP_NORMALIZED_PATH: &str = r#"M 0.299888,0.000000 L 0.699888,0.000000 0.699888,0.000000 C 0.752531,0.000000 0.804274,0.013948 0.849944,0.040270 0.895388,0.066592 0.933408,0.104387 0.959730,0.150056 0.986052,0.195501 1.000000,0.247244 1.000000,0.300112 L 1.000000,0.300112 1.000000,1.000000 1.000000,1.000000 0.000000,1.000000 0.000000,1.000000 0.000000,0.299888 0.000000,0.299888 C 0.000000,0.247244 0.013948,0.195501 0.040270,0.149831 0.066592,0.104387 0.104387,0.066367 0.150056,0.040045 0.195501,0.013723 0.247244,0.000000 0.300112,0.000000 L 0.299888,0.000000 Z"#;
pub(super) const ROUND2_SAME_RECT_ADJ_EXTREME_NORMALIZED_PATH: &str = r#"M 0.449944,0.000000 L 0.549831,0.000000 0.549831,0.000000 C 0.628796,0.000000 0.706412,0.020697 0.774803,0.060292 0.843195,0.099888 0.900112,0.156580 0.939483,0.224972 0.979078,0.293363 0.999775,0.370979 0.999775,0.449944 L 0.999775,0.449944 1.000000,1.000000 1.000000,1.000000 0.000000,1.000000 0.000000,1.000000 0.000000,0.449944 0.000000,0.449944 C 0.000000,0.370979 0.020697,0.293363 0.060292,0.224972 0.099888,0.156580 0.156580,0.099663 0.224972,0.060292 0.293363,0.020697 0.370979,0.000000 0.449944,0.000000 L 0.449944,0.000000 Z"#;
pub(super) fn round2_same_rect_adjust_anchor(adj: &HashMap<String, f64>) -> &'static str {
    let value = finite(adj.get("adj").copied(), 16_667.0).clamp(0.0, 50_000.0);
    let anchors = [
        (10_000.0, ROUND2_SAME_RECT_ADJ_LIGHT_NORMALIZED_PATH),
        (16_667.0, ROUND2_SAME_RECT_ADJ_DEFAULTISH_NORMALIZED_PATH),
        (30_000.0, ROUND2_SAME_RECT_ADJ_DEEP_NORMALIZED_PATH),
        (45_000.0, ROUND2_SAME_RECT_ADJ_EXTREME_NORMALIZED_PATH),
    ];

    anchors
        .into_iter()
        .min_by(|(ax, _), (ay, _)| {
            let dx = (value - *ax) / 35_000.0;
            let dy = (value - *ay) / 35_000.0;
            (dx * dx)
                .partial_cmp(&(dy * dy))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, path)| path)
        .unwrap_or(ROUND2_SAME_RECT_ADJ_DEFAULTISH_NORMALIZED_PATH)
}
pub(super) fn round2_same_rect_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if adj.contains_key("adj") {
        return scale_normalized_path(round2_same_rect_adjust_anchor(adj), w, h);
    }
    if adj.is_empty() {
        let r = w.min(h) * 16_667.0 / 100_000.0;
        return format!(
            "M{r:.1},0 L{x:.1},0 Q{w:.1},0 {w:.1},{r:.1} L{w:.1},{h:.1} L0,{h:.1} L0,{r:.1} Q0,0 {r:.1},0 Z",
            r = r,
            x = w - r,
            w = w,
            h = h
        );
    }

    let ss = w.min(h);
    let top = ss * finite(adj.get("adj1").copied(), 16_667.0).clamp(0.0, 50_000.0) / 100_000.0;
    let bottom = ss * finite(adj.get("adj2").copied(), 0.0).clamp(0.0, 50_000.0) / 100_000.0;
    format!(
        "M{top:.1},0 L{x_top:.1},0 Q{w:.1},0 {w:.1},{top:.1} L{w:.1},{y_bottom:.1} Q{w:.1},{h:.1} {x_bottom:.1},{h:.1} L{bottom:.1},{h:.1} Q0,{h:.1} 0,{y_bottom:.1} L0,{top:.1} Q0,0 {top:.1},0 Z",
        top = top,
        x_top = w - top,
        x_bottom = w - bottom,
        bottom = bottom,
        y_bottom = h - bottom,
        w = w,
        h = h
    )
}
pub(super) fn round2_diag_rect_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if adj.contains_key("adj") {
        return scale_normalized_path(round2_diag_rect_adjust_anchor(adj), w, h);
    }
    if adj.is_empty() {
        let r = w.min(h) * 16_667.0 / 100_000.0;
        return format!(
            "M{r:.1},0 L{x:.1},0 Q{w:.1},0 {w:.1},{r:.1} L{w:.1},{y:.1} Q{w:.1},{h:.1} {x:.1},{h:.1} L0,{h:.1} L0,0 Z",
            r = r,
            x = w - r,
            w = w,
            y = h - r,
            h = h
        );
    }

    let ss = w.min(h);
    let first = ss * finite(adj.get("adj1").copied(), 16_667.0).clamp(0.0, 50_000.0) / 100_000.0;
    let second = ss * finite(adj.get("adj2").copied(), 0.0).clamp(0.0, 50_000.0) / 100_000.0;
    format!(
        "M{first:.1},0 L{x_second:.1},0 Q{w:.1},0 {w:.1},{second:.1} L{w:.1},{y_first:.1} Q{w:.1},{h:.1} {x_first:.1},{h:.1} L{second:.1},{h:.1} Q0,{h:.1} 0,{y_second:.1} L0,{first:.1} Q0,0 {first:.1},0 Z",
        first = first,
        second = second,
        x_first = w - first,
        x_second = w - second,
        w = w,
        y_first = h - first,
        y_second = h - second,
        h = h
    )
}
pub(super) fn fold_corner_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let a = finite(adj.get("adj").copied(), 16_667.0).clamp(0.0, 50_000.0);
    let d = w.min(h) * a / 100_000.0;
    format!(
        "M0,0 L{w:.1},0 L{w:.1},{y:.1} L{x:.1},{h:.1} L0,{h:.1} Z M{x:.1},{h:.1} L{w:.1},{y:.1} L{x:.1},{y:.1} Z",
        w = w,
        x = w - d,
        y = h - d,
        h = h
    )
}
pub(super) const ROUND2_DIAG_RECT_ADJ_LIGHT_NORMALIZED_PATH: &str = r#"M 0.099888,0.000000 L 1.000000,0.000000 1.000000,0.000000 1.000000,0.899888 1.000000,0.899888 C 1.000000,0.917435 0.995276,0.934758 0.986502,0.949831 0.977728,0.965129 0.965129,0.977728 0.950056,0.986502 0.934758,0.995276 0.917660,1.000000 0.900112,1.000000 L 0.000000,1.000000 0.000000,1.000000 0.000000,0.099888 0.000000,0.099888 C 0.000000,0.082340 0.004724,0.065017 0.013498,0.049944 0.022272,0.034646 0.034871,0.022047 0.049944,0.013273 0.065242,0.004499 0.082340,0.000000 0.100112,0.000000 L 0.099888,0.000000 Z"#;
pub(super) const ROUND2_DIAG_RECT_ADJ_DEFAULTISH_NORMALIZED_PATH: &str = r#"M 0.166479,0.000225 L 1.000000,0.000225 1.000000,0.000225 1.000000,0.833333 1.000000,0.833333 C 1.000000,0.862573 0.992351,0.891363 0.977728,0.916554 0.963105,0.941970 0.941957,0.963113 0.916760,0.977733 0.891339,0.992353 0.862542,1.000000 0.833296,1.000000 L 0.000000,1.000000 0.000000,1.000000 0.000000,0.166667 0.000000,0.166667 C 0.000000,0.137427 0.007649,0.108637 0.022272,0.083446 0.036895,0.058030 0.058043,0.036887 0.083240,0.022267 0.108661,0.007647 0.137458,0.000000 0.166704,0.000000 L 0.166479,0.000225 Z"#;
pub(super) const ROUND2_DIAG_RECT_ADJ_DEEP_NORMALIZED_PATH: &str = r#"M 0.299888,0.000000 L 1.000000,0.000000 1.000000,0.000000 1.000000,0.699888 1.000000,0.699888 C 1.000000,0.752531 0.986052,0.804274 0.959730,0.849944 0.933408,0.895388 0.895613,0.933408 0.849944,0.959730 0.804499,0.986052 0.752756,1.000000 0.700112,1.000000 L 0.000000,1.000000 0.000000,1.000000 0.000000,0.299888 0.000000,0.299888 C 0.000000,0.247244 0.013948,0.195501 0.040270,0.149831 0.066592,0.104387 0.104387,0.066367 0.150056,0.040045 0.195501,0.013723 0.247244,0.000000 0.300112,0.000000 L 0.299888,0.000000 Z"#;
pub(super) const ROUND2_DIAG_RECT_ADJ_EXTREME_NORMALIZED_PATH: &str = r#"M 0.449944,0.000000 L 1.000000,0.000000 1.000000,0.000000 1.000000,0.549831 1.000000,0.549831 C 1.000000,0.628796 0.979303,0.706412 0.939708,0.774803 0.900112,0.843195 0.843420,0.900112 0.775028,0.939483 0.706637,0.979078 0.629021,0.999775 0.550056,0.999775 L 0.000000,1.000000 0.000000,1.000000 0.000000,0.449944 0.000000,0.449944 C 0.000000,0.370979 0.020697,0.293363 0.060292,0.224972 0.099888,0.156580 0.156580,0.099663 0.224972,0.060292 0.293363,0.020697 0.370979,0.000000 0.449944,0.000000 L 0.449944,0.000000 Z"#;
pub(super) fn round2_diag_rect_adjust_anchor(adj: &HashMap<String, f64>) -> &'static str {
    let value = finite(adj.get("adj").copied(), 16_667.0).clamp(0.0, 50_000.0);
    let anchors = [
        (10_000.0, ROUND2_DIAG_RECT_ADJ_LIGHT_NORMALIZED_PATH),
        (16_667.0, ROUND2_DIAG_RECT_ADJ_DEFAULTISH_NORMALIZED_PATH),
        (30_000.0, ROUND2_DIAG_RECT_ADJ_DEEP_NORMALIZED_PATH),
        (45_000.0, ROUND2_DIAG_RECT_ADJ_EXTREME_NORMALIZED_PATH),
    ];

    anchors
        .into_iter()
        .min_by(|(ax, _), (ay, _)| {
            let dx = (value - *ax) / 35_000.0;
            let dy = (value - *ay) / 35_000.0;
            (dx * dx)
                .partial_cmp(&(dy * dy))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, path)| path)
        .unwrap_or(ROUND2_DIAG_RECT_ADJ_DEFAULTISH_NORMALIZED_PATH)
}
pub(super) fn diag_stripe_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let ratio = finite(adj.get("adj").copied(), 50_000.0).clamp(0.0, 100_000.0) / 100_000.0;
    let top_left_x = w * (0.2 + ratio.clamp(0.0, 1.0) * 0.3);
    let left_y = h * (0.7 - ratio.clamp(0.0, 1.0) * 0.3);
    format!(
        "M0,{h:.1} L0,{left_y:.1} L{top_left_x:.1},0 L{w:.1},0 Z",
        h = h,
        left_y = left_y,
        top_left_x = top_left_x,
        w = w
    )
}
pub(super) fn corner_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let ss = w.min(h);
    let max_adj1 = 100_000.0 * h / ss;
    let max_adj2 = 100_000.0 * w / ss;
    let a1 = finite(adj.get("adj1").copied(), 50_000.0).clamp(0.0, max_adj1);
    let a2 = finite(adj.get("adj2").copied(), 50_000.0).clamp(0.0, max_adj2);
    let dx = ss * a2 / 100_000.0;
    let dy = ss * a1 / 100_000.0;
    format!(
        "M0,0 L{dx:.1},0 L{dx:.1},{y:.1} L{w:.1},{y:.1} L{w:.1},{h:.1} L0,{h:.1} Z",
        dx = dx,
        y = h - dy,
        w = w,
        h = h
    )
}
pub(super) fn plaque_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let m = w.min(h);
    let a = finite(adj.get("adj").copied(), 16_667.0).clamp(0.0, 50_000.0);
    let r = m * a / 100_000.0;
    format!(
        "M0,{r:.1} Q0,0 {r:.1},0 L{x:.1},0 Q{w:.1},0 {w:.1},{r:.1} L{w:.1},{y:.1} Q{w:.1},{h:.1} {x:.1},{h:.1} L{r:.1},{h:.1} Q0,{h:.1} 0,{y:.1} Z",
        r = r,
        x = w - r,
        w = w,
        y = h - r,
        h = h
    )
}
pub(super) fn line_path(w: f64, h: f64) -> String {
    format!("M0,0 L{w:.1},{h:.1}")
}
pub(super) fn line_inv_path(w: f64, h: f64) -> String {
    format!("M0,{h:.1} L{w:.1},0", w = w, h = h)
}
