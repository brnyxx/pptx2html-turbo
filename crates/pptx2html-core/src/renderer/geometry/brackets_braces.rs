// Auto-split from renderer/geometry.rs (mechanical move, no logic edits).
// Family: brackets_braces

use super::shared::scale_normalized_path;
use std::collections::HashMap;

fn finite(value: Option<f64>, default: f64) -> f64 {
    value.filter(|value| value.is_finite()).unwrap_or(default)
}

pub(super) fn brace_pair_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let a = finite(adj.get("adj").copied(), 8_333.0).clamp(0.0, 25_000.0);
    let r = w.min(h) * a / 100_000.0;
    let cy = h / 2.0;
    let x2 = r * 2.0;
    let x3 = w - x2;
    let x4 = w - r;
    let y2 = cy - r;
    let y3 = cy + r;
    let y4 = h - r;
    format!(
        "M{x2:.1},{h:.1} A{r:.1},{r:.1} 0 0,1 {r:.1},{y4:.1} L{r:.1},{y3:.1} A{r:.1},{r:.1} 0 0,0 0,{cy:.1} A{r:.1},{r:.1} 0 0,0 {r:.1},{y2:.1} L{r:.1},{r:.1} A{r:.1},{r:.1} 0 0,1 {x2:.1},0 L{x3:.1},0 A{r:.1},{r:.1} 0 0,1 {x4:.1},{r:.1} L{x4:.1},{y2:.1} A{r:.1},{r:.1} 0 0,0 {w:.1},{cy:.1} A{r:.1},{r:.1} 0 0,0 {x4:.1},{y3:.1} L{x4:.1},{y4:.1} A{r:.1},{r:.1} 0 0,1 {x3:.1},{h:.1} Z",
    )
}
pub(super) fn bracket_pair_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let a = finite(adj.get("adj").copied(), 16_667.0).clamp(0.0, 50_000.0);
    let r = w.min(h) * a / 100_000.0;
    format!(
        "M0,{r:.1} A{r:.1},{r:.1} 0 0,1 {r:.1},0 L{x:.1},0 A{r:.1},{r:.1} 0 0,1 {w:.1},{r:.1} L{w:.1},{y:.1} A{r:.1},{r:.1} 0 0,1 {x:.1},{h:.1} L{r:.1},{h:.1} A{r:.1},{r:.1} 0 0,1 0,{y:.1} Z",
        x = w - r,
        y = h - r,
    )
}
pub(super) fn half_frame_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let ss = w.min(h);
    let max_adj2 = 100_000.0 * w / ss;
    let a2 = finite(adj.get("adj2").copied(), 33_333.0).clamp(0.0, max_adj2);
    let tx = ss * a2 / 100_000.0;
    let max_adj1 = 100_000.0 * (h - h * tx / w) / ss;
    let a1 = finite(adj.get("adj1").copied(), 33_333.0).clamp(0.0, max_adj1);
    let y1 = ss * a1 / 100_000.0;
    let x2 = w - y1 * w / h;
    let y2 = h - tx * h / w;
    format!("M0,0 L{w:.1},0 L{x2:.1},{y1:.1} L{tx:.1},{y1:.1} L{tx:.1},{y2:.1} L0,{h:.1} Z")
}
// Brackets and braces
pub(super) fn left_brace_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let ss = w.min(h);
    let a2 = finite(adj.get("adj2").copied(), 50_000.0).clamp(0.0, 100_000.0);
    let max_adj1 = a2.min(100_000.0 - a2) * h / (2.0 * ss);
    let a1 = finite(adj.get("adj1").copied(), 8_333.0).clamp(0.0, max_adj1);
    if (a1 - 8_333.0).abs() < f64::EPSILON && (a2 - 50_000.0).abs() < f64::EPSILON {
        return scale_normalized_path(
            "M 0.970146,0.954632 L 0.970146,0.954632 C 0.887509,0.954632 0.806544,0.951051 0.735133,0.944126 0.663721,0.937202 0.604251,0.927412 0.562933,0.915473 0.521615,0.903534 0.500119,0.890162 0.500119,0.876313 L 0.499881,0.562798 0.499881,0.562798 0.499881,0.562798 C 0.499881,0.548949 0.478147,0.535578 0.436828,0.523639 0.395510,0.511700 0.336279,0.501910 0.264867,0.494986 0.193456,0.488061 0.112252,0.484479 0.029615,0.484479 L 0.029615,0.484479 C 0.112252,0.484479 0.193456,0.480898 0.264867,0.473973 0.336279,0.467049 0.395510,0.457259 0.436828,0.445320 0.478147,0.433381 0.499881,0.419771 0.499881,0.406160 L 0.499881,0.092884 0.499881,0.092884 C 0.499881,0.079035 0.521615,0.065664 0.562933,0.053725 0.604251,0.041786 0.663482,0.031996 0.734894,0.025072 0.806305,0.018147 0.887509,0.014565 0.970146,0.014565 L 0.970146,0.954632 Z",
            w,
            h,
        );
    }

    let r = ss * a1 / 100_000.0;
    let cy = h * a2 / 100_000.0;
    let x = w * 0.7;
    format!(
        "M{x:.1},0 Q{xm:.1},0 {xm:.1},{r:.1} L{xm:.1},{y1:.1} Q{xm:.1},{cy:.1} 0,{cy:.1} Q{xm:.1},{cy:.1} {xm:.1},{y2:.1} L{xm:.1},{y3:.1} Q{xm:.1},{h:.1} {x:.1},{h:.1}",
        x = x,
        xm = x * 0.5,
        r = r,
        y1 = cy - r,
        cy = cy,
        y2 = cy + r,
        y3 = h - r,
        h = h
    )
}
pub(super) fn right_brace_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let ss = w.min(h);
    let a2 = finite(adj.get("adj2").copied(), 50_000.0).clamp(0.0, 100_000.0);
    let max_adj1 = a2.min(100_000.0 - a2) * h / (2.0 * ss);
    let a1 = finite(adj.get("adj1").copied(), 8_333.0).clamp(0.0, max_adj1);
    if (a1 - 8_333.0).abs() < f64::EPSILON && (a2 - 50_000.0).abs() < f64::EPSILON {
        return scale_normalized_path(
            "M 0.029854,0.014333 L 0.029854,0.014333 C 0.112491,0.014333 0.193456,0.017917 0.264867,0.024845 0.336279,0.031773 0.395749,0.041567 0.437067,0.053512 0.478385,0.065456 0.500119,0.078834 0.500119,0.092690 L 0.500119,0.092690 0.499881,0.406116 0.499881,0.406116 C 0.499881,0.419971 0.521615,0.433349 0.562933,0.445294 0.604251,0.457238 0.663482,0.467033 0.734894,0.473961 0.806305,0.480889 0.887509,0.484472 0.970146,0.484472 L 0.970146,0.484472 C 0.887509,0.484472 0.806305,0.488055 0.734894,0.494983 0.663482,0.501911 0.604251,0.511706 0.562933,0.523650 0.521615,0.535595 0.499881,0.549212 0.499881,0.562828 L 0.499881,0.876254 0.499881,0.876254 C 0.499881,0.890110 0.478147,0.903488 0.436828,0.915432 0.395510,0.927377 0.336279,0.937172 0.264867,0.944099 0.193456,0.951027 0.112252,0.954611 0.029854,0.954611 L 0.029854,0.014333 Z",
            w,
            h,
        );
    }

    let r = ss * a1 / 100_000.0;
    let cy = h * a2 / 100_000.0;
    let x = w * 0.3;
    format!(
        "M{x:.1},0 Q{xm:.1},0 {xm:.1},{r:.1} L{xm:.1},{y1:.1} Q{xm:.1},{cy:.1} {w:.1},{cy:.1} Q{xm:.1},{cy:.1} {xm:.1},{y2:.1} L{xm:.1},{y3:.1} Q{xm:.1},{h:.1} {x:.1},{h:.1}",
        x = x,
        xm = w - x * 0.5,
        r = r,
        y1 = cy - r,
        cy = cy,
        y2 = cy + r,
        y3 = h - r,
        h = h,
        w = w
    )
}
pub(super) fn left_bracket_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let ss = w.min(h);
    let max_adj = 50_000.0 * h / ss;
    let a = finite(adj.get("adj").copied(), 8_333.0).clamp(0.0, max_adj);
    let r = ss * a / 100_000.0;
    let x = w * 0.7;
    format!(
        "M{x:.1},0 L{r:.1},0 Q0,0 0,{r:.1} L0,{y:.1} Q0,{h:.1} {r:.1},{h:.1} L{x:.1},{h:.1}",
        x = x,
        r = r,
        y = h - r,
        h = h
    )
}
pub(super) fn right_bracket_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let ss = w.min(h);
    let max_adj = 50_000.0 * h / ss;
    let a = finite(adj.get("adj").copied(), 8_333.0).clamp(0.0, max_adj);
    let r = ss * a / 100_000.0;
    let x = w * 0.3;
    format!(
        "M{x:.1},0 L{xr:.1},0 Q{w:.1},0 {w:.1},{r:.1} L{w:.1},{y:.1} Q{w:.1},{h:.1} {xr:.1},{h:.1} L{x:.1},{h:.1}",
        x = x,
        xr = w - r,
        w = w,
        r = r,
        y = h - r,
        h = h
    )
}
