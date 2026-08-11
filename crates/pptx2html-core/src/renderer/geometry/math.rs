// Auto-split from renderer/geometry.rs (mechanical move, no logic edits).
// Family: math

use super::shared::scale_normalized_path;
use std::collections::HashMap;
pub(super) fn math_equal_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if adj.is_empty() {
        return scale_normalized_path(
            "M 0.112890,0.196584 L 0.883509,0.196584 0.883509,0.491445 0.112890,0.491445 0.112890,0.196584 Z M 0.112890,0.549220 L 0.883509,0.549220 0.883509,0.843965 0.112890,0.843965 0.112890,0.549220 Z",
            w,
            h,
        );
    }

    let a1 = adj.get("adj1").copied().unwrap_or(23520.0) / 100_000.0;
    let a2 = adj.get("adj2").copied().unwrap_or(11760.0) / 100_000.0;
    let bar_h = h * a2;
    let gap = h * a1;
    let cy = h / 2.0;
    let y1 = cy - gap / 2.0 - bar_h;
    let y2 = cy - gap / 2.0;
    let y3 = cy + gap / 2.0;
    let y4 = cy + gap / 2.0 + bar_h;
    format!(
        "M0,{y1:.1} L{w:.1},{y1:.1} L{w:.1},{y2:.1} L0,{y2:.1} Z M0,{y3:.1} L{w:.1},{y3:.1} L{w:.1},{y4:.1} L0,{y4:.1} Z",
        y1 = y1,
        y2 = y2,
        y3 = y3,
        y4 = y4,
        w = w
    )
}
pub(super) fn math_not_equal_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if adj.is_empty() {
        return scale_normalized_path(
            "M 0.450237,0.991620 L 0.274882,0.913408 L 0.267773,0.882682 L 0.289100,0.801676 L 0.047393,0.801676 L 0.011848,0.787709 L 0.014218,0.544693 L 0.374408,0.544693 L 0.395735,0.463687 L 0.014218,0.449721 L 0.007109,0.217877 L 0.018957,0.203911 L 0.478673,0.203911 L 0.545024,0.008380 L 0.732227,0.083799 L 0.706161,0.203911 L 0.985782,0.203911 L 0.985782,0.449721 L 0.644550,0.455307 L 0.618483,0.474860 L 0.599526,0.541899 L 0.985782,0.544693 L 0.992891,0.782123 L 0.962085,0.801676 L 0.530806,0.801676 L 0.509479,0.826816 L 0.466825,0.977654 L 0.450237,0.991620 Z",
            w,
            h,
        );
    }

    let a1 = adj.get("adj1").copied().unwrap_or(23520.0) / 100_000.0;
    let a2_angle = adj.get("adj2").copied().unwrap_or(6600000.0);
    let a3 = adj.get("adj3").copied().unwrap_or(11760.0) / 100_000.0;
    let bar_h = h * a3;
    let gap = h * a1;
    let cy = h / 2.0;
    let y1 = cy - gap / 2.0 - bar_h;
    let y2 = cy - gap / 2.0;
    let y3 = cy + gap / 2.0;
    let y4 = cy + gap / 2.0 + bar_h;
    let skew = (a2_angle - 6_600_000.0) / 21_600_000.0;
    let x1 = w * (0.65 + skew * 0.6);
    let x2 = w * (0.35 - skew * 0.6);
    format!(
        "M0,{y1:.1} L{w:.1},{y1:.1} L{w:.1},{y2:.1} L0,{y2:.1} Z M0,{y3:.1} L{w:.1},{y3:.1} L{w:.1},{y4:.1} L0,{y4:.1} Z M{x1:.1},0 L{x2:.1},{h:.1}",
        y1 = y1,
        y2 = y2,
        y3 = y3,
        y4 = y4,
        w = w,
        x1 = x1,
        x2 = x2,
        h = h
    )
}
pub(super) const MATH_MULTIPLY_DEFAULT_NORMALIZED_PATH: &str = r#"M 0.301724,0.827586 L 0.155172,0.685345 L 0.155172,0.672414 L 0.323276,0.508621 L 0.323276,0.495690 L 0.155172,0.331897 L 0.155172,0.318966 L 0.318966,0.155172 L 0.331897,0.155172 L 0.495690,0.323276 L 0.508621,0.323276 L 0.672414,0.155172 L 0.685345,0.155172 L 0.849138,0.318966 L 0.849138,0.331897 L 0.676724,0.504310 L 0.849138,0.672414 L 0.849138,0.685345 L 0.685345,0.849138 L 0.672414,0.849138 L 0.504310,0.676724 L 0.331897,0.849138 L 0.318966,0.849138 Z"#;
pub(super) fn math_multiply_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if adj.is_empty() {
        return scale_normalized_path(MATH_MULTIPLY_DEFAULT_NORMALIZED_PATH, w, h);
    }

    let a1 = adj.get("adj1").copied().unwrap_or(23520.0) / 100_000.0;
    let (cx, cy) = (w / 2.0, h / 2.0);
    let d = w.min(h) * a1;
    let t = w.min(h) * 0.06;
    format!(
        "M{x1:.1},{y1t:.1} L{x1t:.1},{y1:.1} L{cx:.1},{y3:.1} L{x2t:.1},{y1:.1} L{x2:.1},{y1t:.1} L{x4:.1},{cy:.1} L{x2:.1},{y2t:.1} L{x2t:.1},{y2:.1} L{cx:.1},{y4:.1} L{x1t:.1},{y2:.1} L{x1:.1},{y2t:.1} L{x3:.1},{cy:.1} Z",
        cx = cx,
        cy = cy,
        x1 = cx - d,
        y1 = cy - d,
        x1t = cx - d + t,
        y1t = cy - d + t,
        x2 = cx + d,
        y2 = cy + d,
        x2t = cx + d - t,
        y2t = cy + d - t,
        x3 = cx - d + t * 0.5,
        x4 = cx + d - t * 0.5,
        y3 = cy - d + t * 0.5,
        y4 = cy + d - t * 0.5
    )
}
pub(super) const MATH_DIVIDE_DEFAULT_NORMALIZED_PATH: &str = r#"M 0.493506,0.112069 L 0.536797,0.116379 L 0.580087,0.137931 L 0.606061,0.163793 L 0.619048,0.189655 L 0.619048,0.202586 L 0.623377,0.202586 L 0.623377,0.219828 L 0.627706,0.219828 L 0.627706,0.254310 L 0.623377,0.254310 L 0.623377,0.271552 L 0.619048,0.271552 L 0.619048,0.284483 L 0.606061,0.310345 L 0.567100,0.344828 L 0.536797,0.353448 L 0.536797,0.357759 L 0.515152,0.357759 L 0.515152,0.362069 L 0.467532,0.357759 L 0.424242,0.336207 L 0.393939,0.301724 L 0.385281,0.271552 L 0.380952,0.271552 L 0.380952,0.241379 L 0.376623,0.241379 L 0.380952,0.202586 L 0.402597,0.159483 L 0.437229,0.129310 L 0.467532,0.120690 L 0.467532,0.116379 L 0.493506,0.116379 Z M 0.874459,0.379310 L 0.878788,0.383621 L 0.878788,0.620690 L 0.874459,0.625000 L 0.129870,0.625000 L 0.125541,0.620690 L 0.125541,0.383621 L 0.129870,0.379310 Z M 0.437229,0.659483 L 0.467532,0.650862 L 0.467532,0.646552 L 0.489177,0.646552 L 0.489177,0.642241 L 0.536797,0.646552 L 0.536797,0.650862 L 0.549784,0.650862 L 0.575758,0.663793 L 0.606061,0.693966 L 0.619048,0.719828 L 0.619048,0.732759 L 0.623377,0.732759 L 0.623377,0.750000 L 0.627706,0.750000 L 0.627706,0.784483 L 0.623377,0.784483 L 0.623377,0.801724 L 0.619048,0.801724 L 0.619048,0.814655 L 0.606061,0.840517 L 0.567100,0.875000 L 0.536797,0.883621 L 0.536797,0.887931 L 0.510823,0.887931 L 0.510823,0.892241 L 0.467532,0.887931 L 0.424242,0.866379 L 0.393939,0.831897 L 0.385281,0.801724 L 0.380952,0.801724 L 0.380952,0.771552 L 0.376623,0.771552 L 0.380952,0.732759 L 0.402597,0.689655 Z"#;
pub(super) fn math_divide_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if adj.is_empty() {
        return scale_normalized_path(MATH_DIVIDE_DEFAULT_NORMALIZED_PATH, w, h);
    }

    let a1 = adj.get("adj1").copied().unwrap_or(23520.0) / 100_000.0;
    let a2 = adj.get("adj2").copied().unwrap_or(5765.0) / 100_000.0;
    let a3 = adj.get("adj3").copied().unwrap_or(11760.0) / 100_000.0;
    let cx = w / 2.0;
    let dr = w.min(h) * a2;
    let bar_h = h * a3;
    let gap = h * a1;
    let cy = h / 2.0;
    let y1 = cy - bar_h / 2.0;
    let y2 = cy + bar_h / 2.0;
    let dot_top = cy - gap / 2.0 - bar_h / 2.0;
    let dot_bot = cy + gap / 2.0 + bar_h / 2.0;
    format!(
        "M0,{y1:.1} L{w:.1},{y1:.1} L{w:.1},{y2:.1} L0,{y2:.1} Z M{cx:.1},{d1:.1} A{dr:.1},{dr:.1} 0 1,1 {cx:.1},{d1b:.1} A{dr:.1},{dr:.1} 0 1,1 {cx:.1},{d1:.1} Z M{cx:.1},{d2:.1} A{dr:.1},{dr:.1} 0 1,1 {cx:.1},{d2b:.1} A{dr:.1},{dr:.1} 0 1,1 {cx:.1},{d2:.1} Z",
        y1 = y1,
        y2 = y2,
        w = w,
        cx = cx,
        dr = dr,
        d1 = dot_top - dr,
        d1b = dot_top + dr,
        d2 = dot_bot - dr,
        d2b = dot_bot + dr
    )
}
pub(super) fn plus_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if !adj.contains_key("adj") {
        return scale_normalized_path(
            "M 0.352280,0.110805 L 0.647720,0.110805 0.647720,0.408107 0.899713,0.408107 0.899713,0.591893 0.647720,0.591893 0.647720,0.889195 0.352280,0.889195 0.352280,0.591893 0.100287,0.591893 0.100287,0.408107 0.352280,0.408107 0.352280,0.110805 Z",
            w,
            h,
        );
    }

    let a = adj.get("adj").copied().unwrap_or(25_000.0);
    plus_numeric_path(w, h, a)
}

pub(super) fn math_plus_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let a = adj.get("adj1").copied().unwrap_or(23_520.0);
    plus_numeric_path(w, h, a)
}

fn plus_numeric_path(w: f64, h: f64, adjustment: f64) -> String {
    let a = adjustment;
    let ax = w * a / 100_000.0;
    let ay = h * a / 100_000.0;
    let (x1, y1) = (w - ax, h - ay);
    format!(
        "M{ax:.1},0 L{x1:.1},0 L{x1:.1},{ay:.1} L{w:.1},{ay:.1} L{w:.1},{y1:.1} L{x1:.1},{y1:.1} L{x1:.1},{h:.1} L{ax:.1},{h:.1} L{ax:.1},{y1:.1} L0,{y1:.1} L0,{ay:.1} L{ax:.1},{ay:.1} Z"
    )
}
pub(super) fn preset_plus_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if !adj.contains_key("adj") {
        return scale_normalized_path(
            "M 0.733333,1.002075 L 0.254167,0.997925 0.239583,0.979253 0.239583,0.771784 0.229167,0.761411 0.033333,0.761411 -0.002083,0.738589 -0.002083,0.261411 0.008333,0.238589 0.239583,0.232365 0.239583,0.024896 0.250000,-0.002075 0.741667,-0.002075 0.756250,0.020747 0.760417,0.232365 0.983333,0.238589 0.997917,0.257261 1.002083,0.734440 0.991667,0.753112 0.962500,0.761411 0.766667,0.761411 0.756250,0.780083 0.756250,0.983402 Z",
            w,
            h,
        );
    }

    plus_path(w, h, adj)
}
pub(super) fn math_minus_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if adj.is_empty() {
        return scale_normalized_path(
            "M 0.100287,0.363463 L 0.899713,0.363463 0.899713,0.636537 0.100287,0.636537 0.100287,0.363463 Z",
            w,
            h,
        );
    }

    let a1 = adj.get("adj1").copied().unwrap_or(11760.0) / 100_000.0;
    let bar_h = h * a1;
    let cy = h / 2.0;
    format!(
        "M0,{y1:.1} L{w:.1},{y1:.1} L{w:.1},{y2:.1} L0,{y2:.1} Z",
        y1 = cy - bar_h / 2.0,
        y2 = cy + bar_h / 2.0,
        w = w
    )
}
