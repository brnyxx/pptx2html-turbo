use super::shared::{
    CURVED_DOWN_ARROW_ADJ_TIGHT_NORMALIZED_PATH, CURVED_DOWN_ARROW_ADJ_WIDE_NORMALIZED_PATH,
    CURVED_LEFT_ARROW_ADJ_TIGHT_NORMALIZED_PATH, CURVED_LEFT_ARROW_ADJ_WIDE_NORMALIZED_PATH,
    CURVED_RIGHT_ARROW_ADJ_TIGHT_NORMALIZED_PATH, CURVED_RIGHT_ARROW_ADJ_WIDE_NORMALIZED_PATH,
    CURVED_UP_ARROW_ADJ_TIGHT_NORMALIZED_PATH, CURVED_UP_ARROW_ADJ_WIDE_NORMALIZED_PATH,
    curved_arrow_adjust_profile, interpolate_normalized_paths,
};
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

fn degenerate(w: f64, h: f64, start: &str, end: &str) -> String {
    let defaults = HashMap::new();
    interpolate_normalized_paths(start, end, curved_arrow_adjust_profile(&defaults), w, h)
}

fn values(w: f64, h: f64, adj: &HashMap<String, f64>, vertical: bool) -> (f64, f64, f64) {
    let a2 = finite(adj.get("adj2").copied(), 50_000.0).clamp(0.0, 100_000.0);
    let a1 = finite(adj.get("adj1").copied(), 25_000.0).clamp(0.0, a2);
    let max_a3 = 100_000.0 - a2 / 2.0;
    let a3 = finite(adj.get("adj3").copied(), 25_000.0).clamp(0.0, max_a3);
    let cross = if vertical { w } else { h };
    (
        scaled(cross, a1),
        scaled(cross, a2),
        scaled(if vertical { h } else { w }, a3),
    )
}

pub(super) fn curved_right_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let (inner, outer, head) = values(w, h, adj, false);
    let cy = h / 2.0;
    if w == 0.0 || h == 0.0 {
        return degenerate(
            w,
            h,
            CURVED_RIGHT_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
            CURVED_RIGHT_ARROW_ADJ_WIDE_NORMALIZED_PATH,
        );
    }
    format!(
        "M0,{:.1} C{:.1},0 {:.1},0 {:.1},{:.1} L{:.1},{:.1} L{w:.1},{cy:.1} L{:.1},{:.1} L{:.1},{:.1} C{:.1},{h:.1} {:.1},{h:.1} 0,{:.1} Z",
        cy - outer,
        w * 0.35,
        w * 0.65,
        w - head,
        cy - outer,
        w - head,
        cy - outer * 1.5,
        w - head,
        cy + outer * 1.5,
        w - head,
        cy + inner,
        w * 0.65,
        w * 0.35,
        cy - inner
    )
}

pub(super) fn curved_left_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let (inner, outer, head) = values(w, h, adj, false);
    let cy = h / 2.0;
    if w == 0.0 || h == 0.0 {
        return degenerate(
            w,
            h,
            CURVED_LEFT_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
            CURVED_LEFT_ARROW_ADJ_WIDE_NORMALIZED_PATH,
        );
    }
    format!(
        "M{w:.1},{:.1} C{:.1},0 {:.1},0 {head:.1},{:.1} L{head:.1},{:.1} L0,{cy:.1} L{head:.1},{:.1} L{head:.1},{:.1} C{:.1},{h:.1} {:.1},{h:.1} {w:.1},{:.1} Z",
        cy - outer,
        w * 0.65,
        w * 0.35,
        cy - outer,
        cy - outer * 1.5,
        cy + outer * 1.5,
        cy + inner,
        w * 0.35,
        w * 0.65,
        cy - inner
    )
}

pub(super) fn curved_up_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let (inner, outer, head) = values(w, h, adj, true);
    let cx = w / 2.0;
    if w == 0.0 || h == 0.0 {
        return degenerate(
            w,
            h,
            CURVED_UP_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
            CURVED_UP_ARROW_ADJ_WIDE_NORMALIZED_PATH,
        );
    }
    format!(
        "M{:.1},{h:.1} C0,{:.1} 0,{:.1} {:.1},{head:.1} L{:.1},{head:.1} L{cx:.1},0 L{:.1},{head:.1} L{:.1},{head:.1} C{w:.1},{:.1} {w:.1},{:.1} {:.1},{h:.1} Z",
        cx - outer,
        h * 0.65,
        h * 0.35,
        cx - outer,
        cx - outer * 1.5,
        cx + outer * 1.5,
        cx + inner,
        h * 0.35,
        h * 0.65,
        cx - inner
    )
}

pub(super) fn curved_down_arrow_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let (w, h) = (extent(w), extent(h));
    let (inner, outer, head) = values(w, h, adj, true);
    let cx = w / 2.0;
    if w == 0.0 || h == 0.0 {
        return degenerate(
            w,
            h,
            CURVED_DOWN_ARROW_ADJ_TIGHT_NORMALIZED_PATH,
            CURVED_DOWN_ARROW_ADJ_WIDE_NORMALIZED_PATH,
        );
    }
    format!(
        "M{start:.1},0 C0,{c1:.1} 0,{c2:.1} {inner_left:.1},{neck:.1} L{outer_left:.1},{neck:.1} L{cx:.1},{h:.1} L{outer_right:.1},{neck:.1} L{inner_right:.1},{neck:.1} C{w:.1},{c2:.1} {w:.1},{c1:.1} {end:.1},0 Z",
        start = cx - outer,
        c1 = h * 0.35,
        c2 = h * 0.65,
        inner_left = cx - inner,
        neck = h - head,
        outer_left = cx - outer * 1.5,
        outer_right = cx + outer * 1.5,
        inner_right = cx + inner,
        end = cx + outer
    )
}
