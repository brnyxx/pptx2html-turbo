use std::collections::HashMap;
use std::f64::consts::PI;

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

fn point(cx: f64, cy: f64, rx: f64, ry: f64, angle: f64) -> (f64, f64) {
    let radians = angle / 60_000.0 * PI / 180.0;
    let (x, y) = (cx + rx * radians.cos(), cy + ry * radians.sin());
    (
        if x.is_finite() { x } else { 0.0 },
        if y.is_finite() { y } else { 0.0 },
    )
}

fn circular(w: f64, h: f64, adj: &HashMap<String, f64>, left: bool, double_head: bool) -> String {
    let (w, h) = (extent(w), extent(h));
    let ss = w.min(h);
    let a5 = finite(adj.get("adj5").copied(), 12_500.0).clamp(0.0, 25_000.0);
    let max_a1 = (50_000.0 - a5).max(0.0);
    let a1 = finite(adj.get("adj1").copied(), 12_500.0).clamp(0.0, max_a1);
    let default_a2 = if left { -1_142_319.0 } else { 1_142_319.0 };
    let a2 = finite(adj.get("adj2").copied(), default_a2).clamp(-21_599_999.0, 21_599_999.0);
    let a3 = finite(
        adj.get("adj3").copied(),
        if left { 1_142_319.0 } else { 20_457_681.0 },
    )
    .clamp(0.0, 21_599_999.0);
    let a4 = finite(
        adj.get("adj4").copied(),
        if double_head {
            11_942_319.0
        } else {
            10_800_000.0
        },
    )
    .clamp(0.0, 21_599_999.0);
    let direction = if left { -1.0 } else { 1.0 };
    let (cx, cy) = (w / 2.0, h / 2.0);
    let outer = (w / 2.0, h / 2.0);
    let thickness = ss * a1 / 100_000.0;
    let inner = (
        (outer.0 - thickness).max(0.0),
        (outer.1 - thickness).max(0.0),
    );
    let head = ss * a5 / 100_000.0;
    let start = a2 * direction;
    let end = (a3 + a4 / 10.0) * direction;
    let os = point(cx, cy, outer.0, outer.1, start);
    let oe = point(cx, cy, outer.0, outer.1, end);
    let ie = point(cx, cy, inner.0, inner.1, end - a4 / 20.0 * direction);
    let is = point(cx, cy, inner.0, inner.1, start);
    let tip = point(
        cx,
        cy,
        outer.0 + head,
        outer.1 + head,
        end + a4 / 30.0 * direction,
    );
    let sweep = if left { 0 } else { 1 };
    let mut path = format!(
        "M{:.1},{:.1} A{:.1},{:.1} 0 1 {sweep} {:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1} A{:.1},{:.1} 0 1 {} {:.1},{:.1} Z",
        os.0,
        os.1,
        outer.0,
        outer.1,
        oe.0,
        oe.1,
        tip.0,
        tip.1,
        ie.0,
        ie.1,
        inner.0,
        inner.1,
        1 - sweep,
        is.0,
        is.1
    );
    if double_head {
        let second = point(
            cx,
            cy,
            outer.0 + head,
            outer.1 + head,
            start - a4 / 30.0 * direction,
        );
        path.push_str(&format!(
            " M{:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1} Z",
            os.0, os.1, second.0, second.1, is.0, is.1
        ));
    }
    path
}

pub(super) fn circular_arrow_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    circular(w, h, a, false, false)
}
pub(super) fn left_circular_arrow_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    circular(w, h, a, true, false)
}
pub(super) fn left_right_circular_arrow_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    circular(w, h, a, false, true)
}
