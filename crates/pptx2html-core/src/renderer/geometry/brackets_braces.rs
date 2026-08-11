// Auto-split from renderer/geometry.rs (mechanical move, no logic edits).
// Family: brackets_braces

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
    let rx = w / 2.0;
    let ry = ss * a1 / 100_000.0;
    let cy = h * a2 / 100_000.0;
    format!(
        "M{w:.1},{h:.1} A{rx:.1},{ry:.1} 0 0,1 {rx:.1},{y4:.1} L{rx:.1},{y3:.1} A{rx:.1},{ry:.1} 0 0,0 0,{cy:.1} A{rx:.1},{ry:.1} 0 0,0 {rx:.1},{y2:.1} L{rx:.1},{ry:.1} A{rx:.1},{ry:.1} 0 0,1 {w:.1},0 Z",
        w = w,
        h = h,
        rx = rx,
        ry = ry,
        y4 = h - ry,
        y3 = cy + ry,
        cy = cy,
        y2 = cy - ry,
    )
}
pub(super) fn right_brace_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let ss = w.min(h);
    let a2 = finite(adj.get("adj2").copied(), 50_000.0).clamp(0.0, 100_000.0);
    let max_adj1 = a2.min(100_000.0 - a2) * h / (2.0 * ss);
    let a1 = finite(adj.get("adj1").copied(), 8_333.0).clamp(0.0, max_adj1);
    let rx = w / 2.0;
    let ry = ss * a1 / 100_000.0;
    let cy = h * a2 / 100_000.0;
    format!(
        "M0,0 A{rx:.1},{ry:.1} 0 0,1 {rx:.1},{ry:.1} L{rx:.1},{y2:.1} A{rx:.1},{ry:.1} 0 0,0 {w:.1},{cy:.1} A{rx:.1},{ry:.1} 0 0,0 {rx:.1},{y3:.1} L{rx:.1},{y4:.1} A{rx:.1},{ry:.1} 0 0,1 0,{h:.1} Z",
        rx = rx,
        ry = ry,
        y2 = cy - ry,
        cy = cy,
        y3 = cy + ry,
        y4 = h - ry,
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
