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

fn coordinate(base: f64, adjustment: f64) -> f64 {
    let value = base * (adjustment / 100_000.0);
    if value.is_finite() { value } else { 0.0 }
}

fn rect(w: f64, h: f64) -> String {
    format!("M0,0 L{w:.1},0 L{w:.1},{h:.1} L0,{h:.1} Z")
}

fn wedge(w: f64, h: f64, x: f64, y: f64, radius: f64) -> String {
    let notch = radius.min(w / 4.0).max(0.0);
    format!(
        "M{notch:.1},0 L{:.1},0 Q{w:.1},0 {w:.1},{notch:.1} L{w:.1},{:.1} Q{w:.1},{h:.1} {:.1},{h:.1} L{:.1},{h:.1} L{x:.1},{y:.1} L{notch:.1},{h:.1} Q0,{h:.1} 0,{:.1} L0,{notch:.1} Q0,0 {notch:.1},0 Z",
        w - notch,
        h - notch,
        w - notch,
        w * 0.65,
        h - notch
    )
}

pub(super) fn wedge_round_rect_callout_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let a1 = finite(adj.get("adj1").copied(), -20_833.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let a2 = finite(adj.get("adj2").copied(), 62_500.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let a3 = finite(adj.get("adj3").copied(), 16_667.0);
    wedge(
        w,
        h,
        w / 2.0 + coordinate(w, a1),
        coordinate(h, a2),
        coordinate(w.min(h), a3),
    )
}

pub(super) fn wedge_ellipse_callout_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let a1 = finite(adj.get("adj1").copied(), -20_833.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let a2 = finite(adj.get("adj2").copied(), 62_500.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let (x, y) = (w / 2.0 + coordinate(w, a1), coordinate(h, a2));
    format!(
        "M0,{:.1} A{:.1},{:.1} 0 1 1 {w:.1},{:.1} A{:.1},{:.1} 0 1 1 0,{:.1} Z M{:.1},{:.1} L{x:.1},{y:.1} Z",
        h / 2.0,
        w / 2.0,
        h / 2.0,
        h / 2.0,
        w / 2.0,
        h / 2.0,
        h / 2.0,
        w * 0.35,
        h * 0.8
    )
}

pub(super) fn cloud_callout_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let a1 = finite(adj.get("adj1").copied(), -20_833.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let a2 = finite(adj.get("adj2").copied(), 62_500.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let (x, y) = (w / 2.0 + coordinate(w, a1), coordinate(h, a2));
    format!(
        "M0,{left_y:.1} C0,0 {left_x:.1},0 {mid_x:.1},{top_y:.1} C{right_x:.1},0 {w:.1},{right_y:.1} {w:.1},{mid_y:.1} C{w:.1},{h:.1} {right_x:.1},{h:.1} {mid_x:.1},{bottom_y:.1} C{left_x:.1},{h:.1} 0,{h:.1} 0,{left_y:.1} Z M{tail_x:.1},{tail_y:.1} L{x:.1},{y:.1} Z",
        left_y = h * 0.6,
        left_x = w * 0.15,
        mid_x = w * 0.5,
        top_y = h * 0.2,
        right_x = w * 0.75,
        right_y = h * 0.15,
        mid_y = h * 0.55,
        bottom_y = h * 0.85,
        tail_x = w * 0.35,
        tail_y = h * 0.8
    )
}

fn callout1(w: f64, h: f64, adj: &HashMap<String, f64>, accent: bool) -> String {
    let a1 = finite(adj.get("adj1").copied(), 18_750.0);
    let a2 = finite(adj.get("adj2").copied(), -8_333.0);
    let a3 = finite(adj.get("adj3").copied(), 112_500.0);
    let a4 = finite(adj.get("adj4").copied(), -38_333.0);
    let (p1x, p1y) = (coordinate(w, a2), coordinate(h, a1));
    let (p2x, p2y) = (coordinate(w, a4), coordinate(h, a3));
    format!(
        "{} {}M{p1x:.1},{p1y:.1} L{p2x:.1},{p2y:.1}",
        rect(w, h),
        if accent {
            format!("M{p1x:.1},0 L{p1x:.1},{h:.1} ")
        } else {
            String::new()
        }
    )
}

fn callout2(w: f64, h: f64, adj: &HashMap<String, f64>, accent: bool) -> String {
    let p1 = (
        coordinate(w, finite(adj.get("adj2").copied(), -8_333.0)),
        coordinate(h, finite(adj.get("adj1").copied(), 18_750.0)),
    );
    let p2 = (
        coordinate(w, finite(adj.get("adj4").copied(), -16_667.0)),
        coordinate(h, finite(adj.get("adj3").copied(), 18_750.0)),
    );
    let p3 = (
        coordinate(w, finite(adj.get("adj6").copied(), -46_667.0)),
        coordinate(h, finite(adj.get("adj5").copied(), 112_500.0)),
    );
    format!(
        "{} {}M{:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1}",
        rect(w, h),
        if accent {
            format!("M{:.1},0 L{:.1},{h:.1} ", p1.0, p1.0)
        } else {
            String::new()
        },
        p1.0,
        p1.1,
        p2.0,
        p2.1,
        p3.0,
        p3.1
    )
}

fn callout3(w: f64, h: f64, adj: &HashMap<String, f64>, accent: bool) -> String {
    let p1 = (
        coordinate(w, finite(adj.get("adj2").copied(), -8_333.0)),
        coordinate(h, finite(adj.get("adj1").copied(), 18_750.0)),
    );
    let p2 = (
        coordinate(w, finite(adj.get("adj4").copied(), -16_667.0)),
        coordinate(h, finite(adj.get("adj3").copied(), 18_750.0)),
    );
    let p3 = (
        coordinate(w, finite(adj.get("adj6").copied(), -16_667.0)),
        coordinate(h, finite(adj.get("adj5").copied(), 100_000.0)),
    );
    let p4 = (
        coordinate(w, finite(adj.get("adj8").copied(), -8_333.0)),
        coordinate(h, finite(adj.get("adj7").copied(), 112_963.0)),
    );
    format!(
        "{} {}M{:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1} L{:.1},{:.1}",
        rect(w, h),
        if accent {
            format!("M{:.1},0 L{:.1},{h:.1} ", p1.0, p1.0)
        } else {
            String::new()
        },
        p1.0,
        p1.1,
        p2.0,
        p2.1,
        p3.0,
        p3.1,
        p4.0,
        p4.1
    )
}

pub(super) fn callout1_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    callout1(extent(w), extent(h), a, false)
}
pub(super) fn callout2_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    callout2(extent(w), extent(h), a, false)
}
pub(super) fn callout3_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    callout3(extent(w), extent(h), a, false)
}
pub(super) fn border_callout1_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    callout1(extent(w), extent(h), a, false)
}
pub(super) fn border_callout2_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    callout2(extent(w), extent(h), a, false)
}
pub(super) fn border_callout3_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    callout3(extent(w), extent(h), a, false)
}
pub(super) fn accent_callout1_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    callout1(extent(w), extent(h), a, true)
}
pub(super) fn accent_callout2_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    callout2(extent(w), extent(h), a, true)
}
pub(super) fn accent_callout3_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    callout3(extent(w), extent(h), a, true)
}
pub(super) fn accent_border_callout1_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    callout1(extent(w), extent(h), a, true)
}
pub(super) fn accent_border_callout2_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    callout2(extent(w), extent(h), a, true)
}
pub(super) fn accent_border_callout3_path(w: f64, h: f64, a: &HashMap<String, f64>) -> String {
    callout3(extent(w), extent(h), a, true)
}

pub(super) fn wedge_rect_callout_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let a1 = finite(adj.get("adj1").copied(), -20_833.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let a2 = finite(adj.get("adj2").copied(), 62_500.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    wedge(w, h, w / 2.0 + coordinate(w, a1), coordinate(h, a2), 0.0)
}
