// Auto-split from renderer/geometry.rs (mechanical move, no logic edits).
// Family: stars

use super::shared::scale_normalized_path;
use std::collections::HashMap;
pub(super) fn star4_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let ratio = adj.get("adj").copied().unwrap_or(18000.0) / 100_000.0;
    let (cx, cy) = (w / 2.0, h / 2.0);
    let (ix, iy) = (cx * (1.0 - ratio), cy * (1.0 - ratio));
    format!(
        "M{cx:.1},0 L{ix2:.1},{iy:.1} L{w:.1},{cy:.1} L{ix2:.1},{iy2:.1} L{cx:.1},{h:.1} L{ix:.1},{iy2:.1} L0,{cy:.1} L{ix:.1},{iy:.1} Z",
        cx = cx,
        cy = cy,
        w = w,
        h = h,
        ix = ix,
        ix2 = w - ix,
        iy = iy,
        iy2 = h - iy
    )
}
pub(super) fn star5_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if adj.is_empty() {
        return scale_normalized_path(
            "M 0.806338,0.991573 L 0.764085,0.974719 L 0.517606,0.783708 L 0.485915,0.783708 L 0.246479,0.969101 L 0.193662,0.991573 L 0.301056,0.629213 L 0.005282,0.382022 L 0.380282,0.373596 L 0.496479,0.008427 L 0.508803,0.016854 L 0.619718,0.373596 L 0.985915,0.373596 L 0.994718,0.382022 L 0.985915,0.401685 L 0.698944,0.629213 L 0.806338,0.991573 Z",
            w,
            h,
        );
    }

    let ratio = adj.get("adj").copied().unwrap_or(25000.0) / 100_000.0;
    let (cx, cy) = (w / 2.0, h / 2.0);
    let (ro_x, ro_y) = (
        cx * adj.get("hf").copied().unwrap_or(105_146.0) / 100_000.0,
        cy * adj.get("vf").copied().unwrap_or(110_557.0) / 100_000.0,
    );
    let (ri_x, ri_y) = (cx * ratio * 2.0, cy * ratio * 2.0);
    let n = 5;
    let t = n * 2;
    let st = -std::f64::consts::FRAC_PI_2;
    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(t as usize);
    for i in 0..t {
        let a = st + 2.0 * std::f64::consts::PI * (i as f64) / (t as f64);
        let (rx, ry) = if i % 2 == 0 {
            (ro_x, ro_y)
        } else {
            (ri_x, ri_y)
        };
        pts.push((cx + rx * a.cos(), cy + ry * a.sin()));
    }
    let mut s = format!("M{:.1},{:.1}", pts[0].0, pts[0].1);
    for &(x, y) in &pts[1..] {
        s.push_str(&format!(" L{x:.1},{y:.1}"));
    }
    s.push_str(" Z");
    s
}
pub(super) fn star6_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let ratio = adj.get("adj").copied().unwrap_or(28868.0) / 100_000.0;
    let (cx, cy) = (w / 2.0, h / 2.0);
    let (ro_x, ro_y) = (
        cx * adj.get("hf").copied().unwrap_or(115_470.0) / 100_000.0,
        cy,
    );
    let (ri_x, ri_y) = (cx * ratio * 2.0, cy * ratio * 2.0);
    let n = 6;
    let t = n * 2;
    let st = -std::f64::consts::FRAC_PI_2;
    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(t as usize);
    for i in 0..t {
        let a = st + 2.0 * std::f64::consts::PI * (i as f64) / (t as f64);
        let (rx, ry) = if i % 2 == 0 {
            (ro_x, ro_y)
        } else {
            (ri_x, ri_y)
        };
        pts.push((cx + rx * a.cos(), cy + ry * a.sin()));
    }
    let mut s = format!("M{:.1},{:.1}", pts[0].0, pts[0].1);
    for &(x, y) in &pts[1..] {
        s.push_str(&format!(" L{x:.1},{y:.1}"));
    }
    s.push_str(" Z");
    s
}
pub(super) fn star_n_path(
    w: f64,
    h: f64,
    n: u32,
    adj: &HashMap<String, f64>,
    default_adj: f64,
) -> String {
    let ratio = adj.get("adj").copied().unwrap_or(default_adj) / 100_000.0;
    star_n_scaled_path(w, h, n, ratio, 1.0, 1.0)
}

pub(super) fn star7_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let ratio = adj.get("adj").copied().unwrap_or(34_601.0) / 100_000.0;
    let horizontal = adj.get("hf").copied().unwrap_or(102_572.0) / 100_000.0;
    let vertical = adj.get("vf").copied().unwrap_or(105_210.0) / 100_000.0;
    star_n_scaled_path(w, h, 7, ratio, horizontal, vertical)
}

pub(super) fn star10_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let ratio = adj.get("adj").copied().unwrap_or(42_533.0) / 100_000.0;
    let horizontal = adj.get("hf").copied().unwrap_or(105_146.0) / 100_000.0;
    star_n_scaled_path(w, h, 10, ratio, horizontal, 1.0)
}

fn star_n_scaled_path(
    w: f64,
    h: f64,
    n: u32,
    ratio: f64,
    horizontal: f64,
    vertical: f64,
) -> String {
    let (cx, cy) = (w / 2.0, h / 2.0);
    let ro = cx * horizontal;
    let ryo = cy * vertical;
    let (ri, ryi) = (ro * ratio * 2.0, ryo * ratio * 2.0);
    let t = n * 2;
    let st = -std::f64::consts::FRAC_PI_2;
    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(t as usize);
    for i in 0..t {
        let a = st + 2.0 * std::f64::consts::PI * (i as f64) / (t as f64);
        let (rx, ry) = if i % 2 == 0 { (ro, ryo) } else { (ri, ryi) };
        pts.push((cx + rx * a.cos(), cy + ry * a.sin()));
    }
    let mut s = format!("M{:.1},{:.1}", pts[0].0, pts[0].1);
    for &(x, y) in &pts[1..] {
        s.push_str(&format!(" L{x:.1},{y:.1}"));
    }
    s.push_str(" Z");
    s
}
pub(super) fn irregular_seal1_path(w: f64, h: f64) -> String {
    scale_normalized_path(
        "M 0.399293,0.991573 L 0.383392,0.966292 L 0.353357,0.750000 L 0.226148,0.817416 L 0.250883,0.665730 L 0.008834,0.668539 L 0.153710,0.544944 L 0.005300,0.398876 L 0.196113,0.348315 L 0.024735,0.132022 L 0.024735,0.109551 L 0.332155,0.283708 L 0.392226,0.109551 L 0.498233,0.261236 L 0.671378,0.008427 L 0.660777,0.238764 L 0.844523,0.205056 L 0.787986,0.334270 L 0.973498,0.382022 L 0.835689,0.488764 L 0.989399,0.598315 L 0.992933,0.620787 L 0.789753,0.623596 L 0.837456,0.834270 L 0.650177,0.699438 L 0.607774,0.912921 L 0.501767,0.727528 L 0.484099,0.727528 L 0.399293,0.991573 Z",
        w,
        h,
    )
}
pub(super) fn irregular_seal2_path(w: f64, h: f64) -> String {
    scale_normalized_path(
        "M 0.238372,0.969203 L 0.235465,0.844203 L 0.209302,0.835145 L 0.078488,0.818841 L 0.165698,0.717391 L 0.165698,0.706522 L 0.020349,0.601449 L 0.186047,0.545290 L 0.194767,0.532609 L 0.072674,0.387681 L 0.258721,0.365942 L 0.229651,0.221014 L 0.232558,0.193841 L 0.369186,0.289855 L 0.401163,0.302536 L 0.415698,0.278986 L 0.450581,0.123188 L 0.462209,0.119565 L 0.514535,0.199275 L 0.529070,0.208333 L 0.540698,0.204710 L 0.668605,0.038043 L 0.659884,0.268116 L 0.680233,0.273551 L 0.796512,0.186594 L 0.747093,0.304348 L 0.755814,0.309783 L 0.968023,0.315217 L 0.776163,0.431159 L 0.828488,0.521739 L 0.755814,0.559783 L 0.747093,0.572464 L 0.845930,0.702899 L 0.674419,0.657609 L 0.665698,0.681159 L 0.677326,0.782609 L 0.563953,0.730072 L 0.549419,0.742754 L 0.531977,0.851449 L 0.479651,0.807971 L 0.453488,0.798913 L 0.438953,0.811594 L 0.401163,0.893116 L 0.363372,0.840580 L 0.337209,0.835145 L 0.238372,0.969203 Z",
        w,
        h,
    )
}
