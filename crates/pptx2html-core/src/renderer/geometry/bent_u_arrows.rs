use super::shared::polygon_path;
use std::collections::HashMap;

fn finite(value: Option<f64>, default: f64) -> f64 {
    value.filter(|value| value.is_finite()).unwrap_or(default)
}

fn extent(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn scaled(base: f64, adjustment: f64) -> f64 {
    let value = base * (adjustment / 100_000.0);
    if value.is_finite() { value } else { 0.0 }
}

pub(super) fn bent_up_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    if w == 0.0 && h == 0.0 {
        return polygon_path(&[(0.0, 0.0)]);
    }
    let a1 = finite(adj.get("adj1").copied(), 25_000.0).clamp(0.0, 50_000.0);
    let a2 = finite(adj.get("adj2").copied(), 25_000.0).clamp(0.0, 50_000.0);
    let a3 = finite(adj.get("adj3").copied(), 25_000.0).clamp(0.0, 50_000.0);
    let (shaft, head, turn) = (scaled(w, a1), scaled(w, a2), scaled(h, a3));
    format!(
        "M0,{h:.1} L0,{base:.1} L{neck:.1},{base:.1} L{neck:.1},{head:.1} L{tip_left:.1},{head:.1} L{tip:.1},0 L{w:.1},{head:.1} L{right:.1},{head:.1} L{right:.1},{h:.1} Z",
        base = h - shaft,
        neck = w - head,
        tip_left = w - head * 2.0,
        tip = w - head,
        right = w - turn
    )
}

pub(super) fn uturn_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let a2 = finite(adj.get("adj2").copied(), 25_000.0).clamp(0.0, 25_000.0);
    let max_a1 = (50_000.0 - a2).max(0.0);
    let a1 = finite(adj.get("adj1").copied(), 25_000.0).clamp(0.0, max_a1);
    let a3 = finite(adj.get("adj3").copied(), 25_000.0).clamp(0.0, 50_000.0);
    let max_a4 = (100_000.0 - a3).max(0.0);
    let a4 = finite(adj.get("adj4").copied(), 43_750.0).clamp(0.0, max_a4);
    let min_a5 = a3 + a4;
    let a5 = finite(adj.get("adj5").copied(), 75_000.0).clamp(min_a5, 100_000.0);
    let (shaft, gap, head, head_y, depth) = (
        scaled(w, a1),
        scaled(w, a2),
        scaled(w, a3),
        scaled(h, a4),
        scaled(h, a5),
    );
    let outer = w - gap;
    format!(
        "M0,{h:.1} L0,{:.1} Q0,0 {outer:.1},0 Q{w:.1},0 {w:.1},{:.1} L{w:.1},{depth:.1} L{:.1},{depth:.1} L{:.1},{:.1} L{:.1},{depth:.1} L{outer:.1},{head_y:.1} Q{outer:.1},{shaft:.1} {shaft:.1},{shaft:.1} L{shaft:.1},{h:.1} Z",
        shaft,
        head_y,
        w - head,
        w - head,
        depth + head,
        w - head * 2.0
    )
}

pub(super) fn left_right_up_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let a2 = finite(adj.get("adj2").copied(), 25_000.0).clamp(0.0, 50_000.0);
    let max_a1 = (50_000.0 - a2).max(0.0);
    let a1 = finite(adj.get("adj1").copied(), 25_000.0).clamp(0.0, max_a1);
    let max_a3 = (100_000.0 - a2).max(0.0);
    let a3 = finite(adj.get("adj3").copied(), 25_000.0).clamp(0.0, max_a3);
    let (shaft, head, y) = (scaled(w, a1), scaled(w, a2), scaled(h, a3));
    let cx = w / 2.0;
    let (shaft_left, shaft_right) = (cx - shaft / 2.0, cx + shaft / 2.0);
    format!(
        "M0,{base:.1} L{head:.1},{base:.1} L{head:.1},{y:.1} L{shaft_left:.1},{y:.1} L{shaft_left:.1},{head:.1} L{up_left:.1},{head:.1} L{cx:.1},0 L{up_right:.1},{head:.1} L{shaft_right:.1},{head:.1} L{shaft_right:.1},{y:.1} L{right_neck:.1},{y:.1} L{right_neck:.1},{base:.1} L{w:.1},{h:.1} L{right_neck:.1},{h:.1} L{right_neck:.1},{h:.1} L{head:.1},{h:.1} Z",
        base = h - head,
        up_left = cx - head,
        up_right = cx + head,
        right_neck = w - head
    )
}

pub(super) fn quad_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let a2 = finite(adj.get("adj2").copied(), 22_500.0).clamp(0.0, 50_000.0);
    let max_a1 = (50_000.0 - a2).max(0.0);
    let a1 = finite(adj.get("adj1").copied(), 22_500.0).clamp(0.0, max_a1);
    let max_a3 = (100_000.0 - a2).max(0.0);
    let a3 = finite(adj.get("adj3").copied(), 22_500.0).clamp(0.0, max_a3);
    let (shaft, head_x, head_y) = (scaled(w.min(h), a1), scaled(w, a2), scaled(h, a3));
    let (cx, cy) = (w / 2.0, h / 2.0);
    format!(
        "M0,{cy:.1} L{head_x:.1},{:.1} L{head_x:.1},{:.1} L{:.1},{:.1} L{:.1},{head_y:.1} L{:.1},{head_y:.1} L{cx:.1},0 L{:.1},{head_y:.1} L{:.1},{head_y:.1} L{:.1},{:.1} L{:.1},{:.1} L{w:.1},{cy:.1} L{:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1} L{cx:.1},{h:.1} L{:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1} L{head_x:.1},{:.1} L{head_x:.1},{:.1} Z",
        cy - head_y,
        cy - shaft / 2.0,
        cx - shaft / 2.0,
        cy - shaft / 2.0,
        cx - shaft / 2.0,
        cx + shaft / 2.0,
        cx + shaft / 2.0,
        cy - shaft / 2.0,
        w - head_x,
        cy - shaft / 2.0,
        w - head_x,
        cy - head_y,
        w - head_x,
        cy + head_y,
        w - head_x,
        cy + shaft / 2.0,
        cx + shaft / 2.0,
        cy + shaft / 2.0,
        cx + shaft / 2.0,
        h - head_y,
        cx - shaft / 2.0,
        h - head_y,
        cx - shaft / 2.0,
        cy + shaft / 2.0,
        head_x,
        cy + shaft / 2.0,
        head_x,
        cy + head_y
    )
}

pub(super) fn left_up_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let a2 = finite(adj.get("adj2").copied(), 25_000.0).clamp(0.0, 50_000.0);
    let max_a1 = (100_000.0 - a2).max(0.0);
    let a1 = finite(adj.get("adj1").copied(), 25_000.0).clamp(0.0, max_a1);
    let max_a3 = (100_000.0 - a2).max(0.0);
    let a3 = finite(adj.get("adj3").copied(), 25_000.0).clamp(0.0, max_a3);
    let (shaft, head, reach) = (scaled(w.min(h), a1), scaled(w.min(h), a2), scaled(h, a3));
    format!(
        "M0,{:.1} L{head:.1},{:.1} L{head:.1},{:.1} L{:.1},{:.1} L{:.1},{head:.1} L{:.1},{head:.1} L{:.1},0 L{w:.1},{head:.1} L{:.1},{head:.1} L{:.1},{h:.1} L{head:.1},{h:.1} L{head:.1},{:.1} Z",
        h - head,
        h - head * 2.0,
        h - reach,
        w - shaft,
        h - reach,
        w - shaft,
        w - head,
        w - shaft,
        w - shaft,
        h - shaft,
        h - head
    )
}
