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

fn horizontal_arrow(w: f64, h: f64, adj: &HashMap<String, f64>, left: bool) -> String {
    let (w, h) = (extent(w), extent(h));
    let ss = w.min(h);
    let a1 = finite(adj.get("adj1").copied(), 50_000.0).clamp(0.0, 100_000.0);
    let max_a2 = if ss > 0.0 { 100_000.0 * w / ss } else { 0.0 };
    let a2 = finite(adj.get("adj2").copied(), 50_000.0).clamp(0.0, max_a2);
    let half_shaft = scaled(h, a1) / 2.0;
    let head = scaled(ss, a2);
    let cy = h / 2.0;
    let (top, bottom) = (cy - half_shaft, cy + half_shaft);
    if left {
        format!(
            "M{w:.1},{top:.1} L{head:.1},{top:.1} L{head:.1},0 L0,{cy:.1} L{head:.1},{h:.1} L{head:.1},{bottom:.1} L{w:.1},{bottom:.1} Z"
        )
    } else {
        let neck = w - head;
        format!(
            "M0,{top:.1} L{neck:.1},{top:.1} L{neck:.1},0 L{w:.1},{cy:.1} L{neck:.1},{h:.1} L{neck:.1},{bottom:.1} L0,{bottom:.1} Z"
        )
    }
}

fn vertical_arrow(w: f64, h: f64, adj: &HashMap<String, f64>, up: bool) -> String {
    let (w, h) = (extent(w), extent(h));
    let ss = w.min(h);
    let a1 = finite(adj.get("adj1").copied(), 50_000.0).clamp(0.0, 100_000.0);
    let max_a2 = if ss > 0.0 { 100_000.0 * h / ss } else { 0.0 };
    let a2 = finite(adj.get("adj2").copied(), 50_000.0).clamp(0.0, max_a2);
    let half_shaft = scaled(w, a1) / 2.0;
    let head = scaled(ss, a2);
    let cx = w / 2.0;
    let (left, right) = (cx - half_shaft, cx + half_shaft);
    if up {
        format!(
            "M{left:.1},{h:.1} L{left:.1},{head:.1} L0,{head:.1} L{cx:.1},0 L{w:.1},{head:.1} L{right:.1},{head:.1} L{right:.1},{h:.1} Z"
        )
    } else {
        let neck = h - head;
        format!(
            "M{left:.1},0 L{right:.1},0 L{right:.1},{neck:.1} L{w:.1},{neck:.1} L{cx:.1},{h:.1} L0,{neck:.1} L{left:.1},{neck:.1} Z"
        )
    }
}

pub(super) fn right_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    horizontal_arrow(w, h, adj, false)
}
pub(super) fn left_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    horizontal_arrow(w, h, adj, true)
}
pub(super) fn up_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    vertical_arrow(w, h, adj, true)
}
pub(super) fn down_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    vertical_arrow(w, h, adj, false)
}

pub(super) fn left_right_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let ss = w.min(h);
    let a1 = finite(adj.get("adj1").copied(), 50_000.0).clamp(0.0, 100_000.0);
    let max_a2 = if ss > 0.0 { 50_000.0 * w / ss } else { 0.0 };
    let a2 = finite(adj.get("adj2").copied(), 50_000.0).clamp(0.0, max_a2);
    let dy = scaled(h, a1) / 2.0;
    let head = scaled(ss, a2);
    let cy = h / 2.0;
    format!(
        "M0,{cy:.1} L{head:.1},0 L{head:.1},{:.1} L{:.1},{:.1} L{:.1},0 L{w:.1},{cy:.1} L{:.1},{h:.1} L{:.1},{:.1} L{head:.1},{:.1} L{head:.1},{h:.1} Z",
        cy - dy,
        w - head,
        cy - dy,
        w - head,
        w - head,
        w - head,
        cy + dy,
        cy + dy
    )
}

pub(super) fn up_down_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let ss = w.min(h);
    let a1 = finite(adj.get("adj1").copied(), 50_000.0).clamp(0.0, 100_000.0);
    let max_a2 = if ss > 0.0 { 50_000.0 * h / ss } else { 0.0 };
    let a2 = finite(adj.get("adj2").copied(), 50_000.0).clamp(0.0, max_a2);
    let dx = scaled(w, a1) / 2.0;
    let head = scaled(ss, a2);
    let cx = w / 2.0;
    format!(
        "M{cx:.1},0 L{w:.1},{head:.1} L{:.1},{head:.1} L{:.1},{:.1} L{w:.1},{:.1} L{cx:.1},{h:.1} L0,{:.1} L{:.1},{:.1} L{:.1},{head:.1} L0,{head:.1} Z",
        cx + dx,
        cx + dx,
        h - head,
        h - head,
        h - head,
        cx - dx,
        h - head,
        cx - dx
    )
}

pub(super) fn bent_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let a1 = finite(adj.get("adj1").copied(), 25_000.0).clamp(0.0, 50_000.0);
    let a2 = finite(adj.get("adj2").copied(), 25_000.0).clamp(0.0, 50_000.0);
    let a3 = finite(adj.get("adj3").copied(), 25_000.0).clamp(0.0, 50_000.0);
    let max_a4 = (100_000.0 - a3).max(0.0);
    let a4 = finite(adj.get("adj4").copied(), 43_750.0).clamp(0.0, max_a4);
    let (shaft, head, bend, neck) = (
        scaled(h, a1),
        scaled(w.min(h), a2),
        scaled(w, a3),
        scaled(h, a4),
    );
    format!(
        "M0,{h:.1} L0,{neck:.1} Q0,{bend:.1} {bend:.1},{bend:.1} L{:.1},{bend:.1} L{:.1},0 L{w:.1},{head:.1} L{:.1},{:.1} L{:.1},{:.1} Q{shaft:.1},{shaft:.1} {shaft:.1},{neck:.1} L{shaft:.1},{h:.1} Z",
        w - head,
        w - head,
        w - head,
        head * 2.0,
        bend,
        head * 2.0
    )
}

pub(super) fn chevron_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let max_adj = if h > 0.0 { 100_000.0 * w / h } else { 0.0 };
    let a = finite(adj.get("adj").copied(), 50_000.0).clamp(0.0, max_adj);
    let p = scaled(h, a);
    format!(
        "M0,0 L{:.1},0 L{w:.1},{:.1} L{:.1},{h:.1} L0,{h:.1} L{p:.1},{:.1} Z",
        w - p,
        h / 2.0,
        w - p,
        h / 2.0
    )
}

pub(super) fn notched_right_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let base = right_arrow_path(w, h, adj);
    let a2 = finite(adj.get("adj2").copied(), 50_000.0);
    let notch = scaled(extent(w).min(extent(h)), a2) / 2.0;
    base.trim_end_matches('Z').to_owned() + &format!(" L{notch:.1},{:.1} Z", extent(h) / 2.0)
}

pub(super) fn striped_right_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let body = right_arrow_path(w, h, adj);
    let a1 = finite(adj.get("adj1").copied(), 50_000.0).clamp(0.0, 100_000.0);
    let stripe = scaled(extent(w), a1) / 10.0;
    format!(
        "M0,0 L{stripe:.1},0 L{stripe:.1},{:.1} L0,{:.1} Z M{:.1},0 L{:.1},0 L{:.1},{:.1} L{:.1},{:.1} Z {body}",
        extent(h),
        extent(h),
        stripe * 2.0,
        stripe * 3.0,
        stripe * 3.0,
        extent(h),
        stripe * 2.0,
        extent(h)
    )
}

pub(super) fn home_plate_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let max_adj = if h > 0.0 { 100_000.0 * w / h } else { 0.0 };
    let a = finite(adj.get("adj").copied(), 50_000.0).clamp(0.0, max_adj);
    let point = scaled(h, a);
    format!(
        "M0,0 L{:.1},0 L{w:.1},{:.1} L{:.1},{h:.1} L0,{h:.1} Z",
        w - point,
        h / 2.0,
        w - point
    )
}

pub(super) fn swoosh_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let a1 = finite(adj.get("adj1").copied(), 25_000.0).clamp(1.0, 75_000.0);
    let max_a2 = (70_000.0 - a1 / 2.0).max(0.0);
    let a2 = finite(adj.get("adj2").copied(), 16_667.0).clamp(0.0, max_a2);
    let (rise, head) = (scaled(h, a1), scaled(w, a2));
    format!(
        "M0,{h:.1} C{:.1},{:.1} {:.1},{rise:.1} {:.1},{rise:.1} L{:.1},0 L{w:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1} C{:.1},{:.1} {:.1},{h:.1} 0,{h:.1} Z",
        w / 3.0,
        h - rise,
        w * 0.6,
        w - head,
        w - head,
        h / 2.0,
        w - head,
        rise * 2.0,
        w - head,
        rise,
        w * 0.5,
        rise * 2.0,
        w / 4.0
    )
}
