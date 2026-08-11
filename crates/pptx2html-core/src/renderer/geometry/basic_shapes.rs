// Auto-split from renderer/geometry.rs (mechanical move, no logic edits).
// Family: basic_shapes

use std::collections::HashMap;

fn finite(value: Option<f64>, default: f64) -> f64 {
    value.filter(|value| value.is_finite()).unwrap_or(default)
}

fn scaled(value: f64, guide: f64) -> f64 {
    let result = value * (guide / 100_000.0);
    if result.is_finite() {
        result
    } else if result.is_sign_negative() {
        -f64::MAX
    } else {
        f64::MAX
    }
}

fn degenerate_path(w: f64, h: f64) -> Option<String> {
    if w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0 {
        return None;
    }
    let w = if w.is_finite() { w.max(0.0) } else { 0.0 };
    let h = if h.is_finite() { h.max(0.0) } else { 0.0 };
    Some(format!("M0,0 L{w:.1},0 L{w:.1},{h:.1} L0,{h:.1} Z"))
}

pub(super) fn rect_path(w: f64, h: f64) -> String {
    format!("M0,0 L{w:.1},0 L{w:.1},{h:.1} L0,{h:.1} Z")
}
pub(super) fn round_rect_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let a = finite(adj.get("adj").copied(), 16_667.0).clamp(0.0, 50_000.0);
    let m = w.min(h);
    let r = scaled(m, a).min(m / 2.0);
    format!(
        "M{r:.1},0 L{x1:.1},0 Q{w:.1},0 {w:.1},{r:.1} L{w:.1},{y1:.1} Q{w:.1},{h:.1} {x1:.1},{h:.1} L{r:.1},{h:.1} Q0,{h:.1} 0,{y1:.1} L0,{r:.1} Q0,0 {r:.1},0 Z",
        r = r,
        x1 = w - r,
        y1 = h - r,
        w = w,
        h = h
    )
}
pub(super) fn ellipse_path(w: f64, h: f64) -> String {
    let rx = w / 2.0;
    let ry = h / 2.0;
    format!(
        "M{cx:.1},0 A{rx:.1},{ry:.1} 0 1,1 {cx:.1},{h:.1} A{rx:.1},{ry:.1} 0 1,1 {cx:.1},0 Z",
        cx = rx,
        rx = rx,
        ry = ry,
        h = h
    )
}
pub(super) fn triangle_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let a = finite(adj.get("adj").copied(), 50_000.0).clamp(0.0, 100_000.0);
    format!("M0,{h:.1} L{:.1},0 L{w:.1},{h:.1} Z", scaled(w, a))
}
pub(super) fn rt_triangle_path(w: f64, h: f64) -> String {
    format!("M0,0 L{w:.1},{h:.1} L0,{h:.1} Z")
}
pub(super) fn diamond_path(w: f64, h: f64) -> String {
    let cx = w / 2.0;
    let cy = h / 2.0;
    format!("M{cx:.1},0 L{w:.1},{cy:.1} L{cx:.1},{h:.1} L0,{cy:.1} Z")
}
pub(super) fn parallelogram_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if let Some(path) = degenerate_path(w, h) {
        return path;
    }
    let ss = w.min(h);
    let max_adj = 100_000.0 * (w / ss);
    let a = finite(adj.get("adj").copied(), 25_000.0).clamp(0.0, max_adj);
    let o = scaled(ss, a);
    format!(
        "M{o:.1},0 L{w:.1},0 L{x:.1},{h:.1} L0,{h:.1} Z",
        o = o,
        w = w,
        x = w - o,
        h = h
    )
}
pub(super) fn hexagon_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if let Some(path) = degenerate_path(w, h) {
        return path;
    }
    let ss = w.min(h);
    let max_adj = 50_000.0 * (w / ss);
    let a = finite(adj.get("adj").copied(), 25_000.0).clamp(0.0, max_adj);
    let vf = finite(adj.get("vf").copied(), 115_470.0);
    let o = scaled(ss, a);
    let cy = h / 2.0;
    let dy = scaled(h / 2.0, vf) * 60.0_f64.to_radians().sin();
    let y1 = cy - dy;
    let y2 = cy + dy;
    format!(
        "M0,{cy:.1} L{o:.1},{y1:.1} L{x:.1},{y1:.1} L{w:.1},{cy:.1} L{x:.1},{y2:.1} L{o:.1},{y2:.1} Z",
        o = o,
        x = w - o,
        w = w,
        cy = cy,
        y1 = y1,
        y2 = y2
    )
}
pub(super) fn trapezoid_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if let Some(path) = degenerate_path(w, h) {
        return path;
    }
    let ss = w.min(h);
    let max_adj = 50_000.0 * (w / ss);
    let a = finite(adj.get("adj").copied(), 25_000.0).clamp(0.0, max_adj);
    let o = scaled(ss, a);
    format!(
        "M0,{h:.1} L{o:.1},0 L{x:.1},0 L{w:.1},{h:.1} Z",
        o = o,
        x = w - o,
        w = w,
        h = h
    )
}
pub(super) fn pentagon_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let hf = finite(adj.get("hf").copied(), 105_146.0);
    let vf = finite(adj.get("vf").copied(), 110_557.0);
    let cx = w / 2.0;
    let scaled_w = scaled(cx, hf);
    let scaled_h = scaled(h / 2.0, vf);
    let scaled_center_y = scaled(h / 2.0, vf);
    let dx1 = scaled_w * 18.0_f64.to_radians().cos();
    let dx2 = scaled_w * 306.0_f64.to_radians().cos();
    let dy1 = scaled_h * 18.0_f64.to_radians().sin();
    let dy2 = scaled_h * 306.0_f64.to_radians().sin();
    format!(
        "M{x1:.1},{y1:.1} L{cx:.1},0 L{x4:.1},{y1:.1} L{x3:.1},{y2:.1} L{x2:.1},{y2:.1} Z",
        cx = cx,
        x1 = cx - dx1,
        x2 = cx - dx2,
        x3 = cx + dx2,
        x4 = cx + dx1,
        y1 = scaled_center_y - dy1,
        y2 = scaled_center_y - dy2
    )
}
pub(super) fn octagon_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let a = finite(adj.get("adj").copied(), 29_289.0).clamp(0.0, 50_000.0);
    let offset = scaled(w.min(h), a);
    format!(
        "M{ox:.1},0 L{x1:.1},0 L{w:.1},{oy:.1} L{w:.1},{y1:.1} L{x1:.1},{h:.1} L{ox:.1},{h:.1} L0,{y1:.1} L0,{oy:.1} Z",
        ox = offset,
        x1 = w - offset,
        w = w,
        oy = offset,
        y1 = h - offset,
        h = h
    )
}
// Ribbons
pub(super) fn ellipse_ribbon_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let a1 = finite(adj.get("adj1").copied(), 25_000.0).clamp(0.0, 100_000.0);
    let a2 = finite(adj.get("adj2").copied(), 50_000.0).clamp(25_000.0, 75_000.0);
    let min_a3 = (a1 - (100_000.0 - a1) / 2.0).max(0.0);
    let a3 = finite(adj.get("adj3").copied(), 12_500.0).clamp(min_a3, a1);
    let cy = scaled(h, 100_000.0 - a2 + a1);
    let bh = scaled(h, a3);
    format!(
        "M0,{cy:.1} Q{cx:.1},{h:.1} {w:.1},{cy:.1} L{w:.1},{bh:.1} Q{cx:.1},0 0,{bh:.1} Z",
        cx = w / 2.0,
        cy = cy,
        w = w,
        bh = bh,
        h = h
    )
}
pub(super) fn ellipse_ribbon2_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let a1 = finite(adj.get("adj1").copied(), 25_000.0).clamp(0.0, 100_000.0);
    let a2 = finite(adj.get("adj2").copied(), 50_000.0).clamp(25_000.0, 100_000.0);
    let min_a3 = (a1 - (100_000.0 - a1) / 2.0).max(0.0);
    let a3 = finite(adj.get("adj3").copied(), 12_500.0).clamp(min_a3, a1);
    let cy = scaled(h, a2 - a1);
    let bh = scaled(h, 100_000.0 - a3);
    format!(
        "M0,{cy:.1} Q{cx:.1},0 {w:.1},{cy:.1} L{w:.1},{bh:.1} Q{cx:.1},{h:.1} 0,{bh:.1} Z",
        cx = w / 2.0,
        cy = cy,
        w = w,
        bh = bh,
        h = h
    )
}
pub(super) fn non_isosceles_trapezoid_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if let Some(path) = degenerate_path(w, h) {
        return path;
    }
    let ss = w.min(h);
    let max_adj = 50_000.0 * (w / ss);
    let a1 = finite(adj.get("adj1").copied(), 25_000.0).clamp(0.0, max_adj);
    let a2 = finite(adj.get("adj2").copied(), 25_000.0).clamp(0.0, max_adj);
    let x1 = scaled(ss, a1);
    let x2 = w - scaled(ss, a2);
    format!(
        "M{a1:.1},0 L{x:.1},0 L{w:.1},{h:.1} L0,{h:.1} Z",
        a1 = x1,
        x = x2,
        w = w,
        h = h
    )
}
