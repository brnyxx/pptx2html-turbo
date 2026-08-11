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

fn horizontal(w: f64, h: f64, adj: &HashMap<String, f64>, left: bool) -> String {
    let (w, h) = (extent(w), extent(h));
    let ss = w.min(h);
    let a1 = finite(adj.get("adj1").copied(), 25_000.0).clamp(0.0, 50_000.0);
    let max_a2 = (50_000.0 - a1).max(0.0);
    let a2 = finite(adj.get("adj2").copied(), 25_000.0).clamp(0.0, max_a2);
    let max_a3 = if ss > 0.0 { 100_000.0 * w / ss } else { 0.0 };
    let a3 = finite(adj.get("adj3").copied(), 25_000.0).clamp(0.0, max_a3);
    let max_a4 = (100_000.0 - a3).max(0.0);
    let a4 = finite(adj.get("adj4").copied(), 64_977.0).clamp(0.0, max_a4);
    let (half, head, body, body_w) = (scaled(h, a1), scaled(h, a2), scaled(ss, a3), scaled(w, a4));
    let cy = h / 2.0;
    let main = if left {
        format!(
            "M{body_w:.1},0 L{w:.1},0 L{w:.1},{h:.1} L{body_w:.1},{h:.1} L{body_w:.1},{:.1} L{body:.1},{:.1} L0,{cy:.1} L{body:.1},{:.1} L{body_w:.1},{:.1} Z",
            cy + half + head,
            cy + half + head,
            cy - half - head,
            cy - half - head
        )
    } else {
        let edge = w - body_w;
        format!(
            "M0,0 L{edge:.1},0 L{edge:.1},{:.1} L{:.1},{:.1} L{w:.1},{cy:.1} L{:.1},{:.1} L{edge:.1},{:.1} L{edge:.1},{h:.1} L0,{h:.1} Z",
            cy - half - head,
            w - body,
            cy - half - head,
            w - body,
            cy + half + head,
            cy + half + head
        )
    };
    format!(
        "{main} M{body_w:.1},{:.1} L{body_w:.1},{:.1} Z",
        cy - half,
        cy + half
    )
}

fn vertical(w: f64, h: f64, adj: &HashMap<String, f64>, up: bool) -> String {
    let (w, h) = (extent(w), extent(h));
    let ss = w.min(h);
    let a1 = finite(adj.get("adj1").copied(), 25_000.0).clamp(0.0, 50_000.0);
    let max_a2 = (50_000.0 - a1).max(0.0);
    let a2 = finite(adj.get("adj2").copied(), 25_000.0).clamp(0.0, max_a2);
    let max_a3 = if ss > 0.0 { 100_000.0 * h / ss } else { 0.0 };
    let a3 = finite(adj.get("adj3").copied(), 25_000.0).clamp(0.0, max_a3);
    let max_a4 = (100_000.0 - a3).max(0.0);
    let a4 = finite(adj.get("adj4").copied(), 64_977.0).clamp(0.0, max_a4);
    let (half, head, body, body_h) = (scaled(w, a1), scaled(w, a2), scaled(ss, a3), scaled(h, a4));
    let cx = w / 2.0;
    let main = if up {
        let edge = h - body_h;
        format!(
            "M0,{edge:.1} L{:.1},{edge:.1} L{:.1},{body:.1} L{cx:.1},0 L{:.1},{body:.1} L{:.1},{edge:.1} L{w:.1},{edge:.1} L{w:.1},{h:.1} L0,{h:.1} Z",
            cx - half - head,
            cx - half - head,
            cx + half + head,
            cx + half + head
        )
    } else {
        format!(
            "M0,0 L{w:.1},0 L{w:.1},{body_h:.1} L{:.1},{body_h:.1} L{:.1},{:.1} L{cx:.1},{h:.1} L{:.1},{:.1} L{:.1},{body_h:.1} L0,{body_h:.1} Z",
            cx + half + head,
            cx + half + head,
            h - body,
            cx - half - head,
            h - body,
            cx - half - head
        )
    };
    format!(
        "{main} M{:.1},{body_h:.1} L{:.1},{body_h:.1} Z",
        cx - half,
        cx + half
    )
}

pub(super) fn down_arrow_callout_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    vertical(w, h, a, false)
}
pub(super) fn left_arrow_callout_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    horizontal(w, h, a, true)
}
pub(super) fn right_arrow_callout_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    horizontal(w, h, a, false)
}
pub(super) fn up_arrow_callout_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    vertical(w, h, a, true)
}

fn cross(w: f64, h: f64, adj: &HashMap<String, f64>, mode: u8) -> String {
    let (w, h) = (extent(w), extent(h));
    let a2 = finite(
        adj.get("adj2").copied(),
        if mode == 2 { 18_515.0 } else { 25_000.0 },
    )
    .clamp(0.0, 50_000.0);
    let max_a1 = (50_000.0 - a2).max(0.0);
    let a1 = finite(
        adj.get("adj1").copied(),
        if mode == 2 { 18_515.0 } else { 25_000.0 },
    )
    .clamp(0.0, max_a1);
    let max_a3 = (100_000.0 - a2).max(0.0);
    let a3 = finite(
        adj.get("adj3").copied(),
        if mode == 2 { 18_515.0 } else { 25_000.0 },
    )
    .clamp(0.0, max_a3);
    let min_a4 = if mode == 2 { a1 } else { 0.0 };
    let a4 = finite(adj.get("adj4").copied(), 48_123.0).clamp(min_a4, 100_000.0 - a3);
    let (sx, sy, hx, hy) = (scaled(w, a1), scaled(h, a1), scaled(w, a2), scaled(h, a3));
    let (cx, cy) = (w / 2.0, h / 2.0);
    let body = scaled(if mode == 0 { w } else { h }, a4);
    if mode == 0 {
        let (left, right) = (body.min(w / 2.0), (w - body).max(w / 2.0));
        format!(
            "M0,{cy:.1} L{hx:.1},{top:.1} L{hx:.1},{inner_top:.1} L{left:.1},{inner_top:.1} L{left:.1},0 L{right:.1},0 L{right:.1},{inner_top:.1} L{neck:.1},{inner_top:.1} L{neck:.1},{top:.1} L{w:.1},{cy:.1} L{neck:.1},{bottom:.1} L{neck:.1},{inner_bottom:.1} L{right:.1},{inner_bottom:.1} L{right:.1},{h:.1} L{left:.1},{h:.1} L{left:.1},{inner_bottom:.1} L{hx:.1},{inner_bottom:.1} L{hx:.1},{bottom:.1} Z",
            top = cy - hy,
            inner_top = cy - sy,
            neck = w - hx,
            bottom = cy + hy,
            inner_bottom = cy + sy
        )
    } else if mode == 1 {
        let (top, bottom) = (body.min(h / 2.0), (h - body).max(h / 2.0));
        format!(
            "M0,{top:.1} L{left:.1},{top:.1} L{left:.1},{hy:.1} L{head_left:.1},{hy:.1} L{cx:.1},0 L{head_right:.1},{hy:.1} L{right:.1},{hy:.1} L{right:.1},{top:.1} L{w:.1},{top:.1} L{w:.1},{bottom:.1} L{right:.1},{bottom:.1} L{right:.1},{neck:.1} L{head_right:.1},{neck:.1} L{cx:.1},{h:.1} L{head_left:.1},{neck:.1} L{left:.1},{neck:.1} L{left:.1},{bottom:.1} L0,{bottom:.1} Z",
            left = cx - sx,
            right = cx + sx,
            head_left = cx - hx,
            head_right = cx + hx,
            neck = h - hy
        )
    } else {
        let (left, right, top, bottom) = (cx - sx, cx + sx, cy - sy, cy + sy);
        let main = format!(
            "M0,{cy:.1} L{hx:.1},{upper:.1} L{hx:.1},{top:.1} L{left:.1},{top:.1} L{left:.1},{hy:.1} L{head_left:.1},{hy:.1} L{cx:.1},0 L{head_right:.1},{hy:.1} L{right:.1},{hy:.1} L{right:.1},{top:.1} L{neck:.1},{top:.1} L{neck:.1},{upper:.1} L{w:.1},{cy:.1} L{neck:.1},{lower:.1} L{neck:.1},{bottom:.1} L{right:.1},{bottom:.1} L{right:.1},{down_neck:.1} L{head_right:.1},{down_neck:.1} L{cx:.1},{h:.1} L{head_left:.1},{down_neck:.1} L{left:.1},{down_neck:.1} L{left:.1},{bottom:.1} L{hx:.1},{bottom:.1} L{hx:.1},{lower:.1} Z",
            upper = cy - hy,
            lower = cy + hy,
            head_left = cx - hx,
            head_right = cx + hx,
            neck = w - hx,
            down_neck = h - hy
        );
        let (body_left, body_right) = (cx - body / 2.0, cx + body / 2.0);
        format!(
            "{main} M{body_left:.1},{top:.1} L{body_right:.1},{top:.1} L{body_right:.1},{bottom:.1} L{body_left:.1},{bottom:.1} Z"
        )
    }
}

pub(super) fn quad_arrow_callout_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    cross(w, h, a, 2)
}
pub(super) fn left_right_arrow_callout_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    cross(w, h, a, 0)
}
pub(super) fn up_down_arrow_callout_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    cross(w, h, a, 1)
}
