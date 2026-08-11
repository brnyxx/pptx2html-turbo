// Auto-split from renderer/geometry.rs (mechanical move, no logic edits).
// Family: rects

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
pub(super) fn snip_round_rect_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let ss = w.min(h);
    let r = ss * finite(adj.get("adj1").copied(), 16_667.0).clamp(0.0, 50_000.0) / 100_000.0;
    let snip = ss * finite(adj.get("adj2").copied(), 16_667.0).clamp(0.0, 50_000.0) / 100_000.0;
    format!(
        "M{r:.1},0 L{x:.1},0 L{w:.1},{snip:.1} L{w:.1},{h:.1} L0,{h:.1} L0,{r:.1} A{r:.1},{r:.1} 0 0,1 {r:.1},0 Z",
        r = r,
        x = w - snip,
        w = w,
        snip = snip,
        h = h
    )
}
pub(super) fn round1_rect_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let r = w.min(h) * finite(adj.get("adj").copied(), 16_667.0).clamp(0.0, 50_000.0) / 100_000.0;
    format!(
        "M0,0 L{x:.1},0 A{r:.1},{r:.1} 0 0,1 {w:.1},{r:.1} L{w:.1},{h:.1} L0,{h:.1} Z",
        x = w - r,
        w = w,
        r = r,
        h = h
    )
}
pub(super) fn round2_same_rect_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let ss = w.min(h);
    let top = ss * finite(adj.get("adj1").copied(), 16_667.0).clamp(0.0, 50_000.0) / 100_000.0;
    let bottom = ss * finite(adj.get("adj2").copied(), 0.0).clamp(0.0, 50_000.0) / 100_000.0;
    format!(
        "M{top:.1},0 L{x_top:.1},0 A{top:.1},{top:.1} 0 0,1 {w:.1},{top:.1} L{w:.1},{y_bottom:.1} A{bottom:.1},{bottom:.1} 0 0,1 {x_bottom:.1},{h:.1} L{bottom:.1},{h:.1} A{bottom:.1},{bottom:.1} 0 0,1 0,{y_bottom:.1} L0,{top:.1} A{top:.1},{top:.1} 0 0,1 {top:.1},0 Z",
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
    let ss = w.min(h);
    let first = ss * finite(adj.get("adj1").copied(), 16_667.0).clamp(0.0, 50_000.0) / 100_000.0;
    let second = ss * finite(adj.get("adj2").copied(), 0.0).clamp(0.0, 50_000.0) / 100_000.0;
    format!(
        "M{first:.1},0 L{x_second:.1},0 A{second:.1},{second:.1} 0 0,1 {w:.1},{second:.1} L{w:.1},{y_first:.1} A{first:.1},{first:.1} 0 0,1 {x_first:.1},{h:.1} L{second:.1},{h:.1} A{second:.1},{second:.1} 0 0,1 0,{y_second:.1} L0,{first:.1} A{first:.1},{first:.1} 0 0,1 {first:.1},0 Z",
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
