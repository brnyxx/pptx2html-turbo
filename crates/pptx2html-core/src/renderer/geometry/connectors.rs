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

pub(super) fn curved_connector2_path(w: f64, h: f64) -> String {
    let (w, h) = (extent(w), extent(h));
    format!("M0,0 C{:.1},0 {:.1},{h:.1} {w:.1},{h:.1}", w / 2.0, w / 2.0)
}

pub(super) fn curved_connector3_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let a1 = finite(adj.get("adj1").copied(), 50_000.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let x = coordinate(w, a1);
    format!(
        "M0,0 C{:.1},0 {x:.1},0 {x:.1},{:.1} C{x:.1},{h:.1} {:.1},{h:.1} {w:.1},{h:.1}",
        x / 2.0,
        h / 2.0,
        x + (w - x) / 2.0
    )
}

pub(super) fn curved_connector4_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let a1 = finite(adj.get("adj1").copied(), 50_000.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let a2 = finite(adj.get("adj2").copied(), 50_000.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let (x, y) = (coordinate(w, a1), coordinate(h, a2));
    format!(
        "M0,0 C{:.1},0 {x:.1},0 {x:.1},{y:.1} C{x:.1},{y:.1} {:.1},{y:.1} {:.1},{:.1} C{:.1},{h:.1} {:.1},{h:.1} {w:.1},{h:.1}",
        x / 2.0,
        (x + w) / 2.0,
        (x + w) / 2.0,
        (y + h) / 2.0,
        (x + w) / 2.0,
        (x + w * 3.0) / 4.0
    )
}

pub(super) fn curved_connector5_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let a1 = finite(adj.get("adj1").copied(), 50_000.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let a2 = finite(adj.get("adj2").copied(), 50_000.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let a3 = finite(adj.get("adj3").copied(), 50_000.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let (x1, y, x2) = (coordinate(w, a1), coordinate(h, a2), coordinate(w, a3));
    format!(
        "M0,0 C{:.1},0 {x1:.1},0 {x1:.1},{y:.1} C{x1:.1},{y:.1} {x2:.1},{y:.1} {x2:.1},{y:.1} C{x2:.1},{y:.1} {x2:.1},{h:.1} {w:.1},{h:.1}",
        x1 / 2.0
    )
}

pub(super) fn bent_connector2_path(w: f64, h: f64) -> String {
    let (w, h) = (extent(w), extent(h));
    format!("M0,0 L{w:.1},0 L{w:.1},{h:.1}")
}
pub(super) fn bent_connector3_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let a1 = finite(adj.get("adj1").copied(), 50_000.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let x = coordinate(w, a1);
    format!("M0,0 L{x:.1},0 L{x:.1},{h:.1} L{w:.1},{h:.1}")
}
pub(super) fn bent_connector4_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let a1 = finite(adj.get("adj1").copied(), 50_000.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let a2 = finite(adj.get("adj2").copied(), 50_000.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let (x, y) = (coordinate(w, a1), coordinate(h, a2));
    format!("M0,0 L{x:.1},0 L{x:.1},{y:.1} L{w:.1},{y:.1} L{w:.1},{h:.1}")
}
pub(super) fn bent_connector5_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let a1 = finite(adj.get("adj1").copied(), 50_000.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let a2 = finite(adj.get("adj2").copied(), 50_000.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let a3 = finite(adj.get("adj3").copied(), 50_000.0).clamp(-2_147_483_647.0, 2_147_483_647.0);
    let (x1, y, x2) = (coordinate(w, a1), coordinate(h, a2), coordinate(w, a3));
    format!("M0,0 L{x1:.1},0 L{x1:.1},{y:.1} L{x2:.1},{y:.1} L{x2:.1},{h:.1} L{w:.1},{h:.1}")
}
