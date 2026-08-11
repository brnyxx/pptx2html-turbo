// Auto-split from renderer/geometry.rs (mechanical move, no logic edits).
// Family: waves_polys

use super::shared::{scale_normalized_path, scale_unit_point};
use std::collections::HashMap;
pub(super) const WAVE_ADJ_LIGHT_NORMALIZED_PATH: &str = r#"M -0.000086,0.099880 C 0.333191,-0.233396 0.666467,0.433156 0.999914,0.099880 L 0.999914,0.899777 C 0.666467,1.233225 0.333191,0.566501 -0.000086,0.899777 L -0.000086,0.099880 Z"#;
pub(super) const WAVE_ADJ_SHIFT_NORMALIZED_PATH: &str = r#"M -0.000086,0.124872 C 0.266433,-0.291595 0.533122,0.541510 0.799812,0.124872 L 0.999914,0.874786 C 0.733225,1.291424 0.466535,0.458148 0.199846,0.874786 L -0.000086,0.124872 Z"#;
pub(super) const WAVE_ADJ_DEEP_NORMALIZED_PATH: &str = r#"M -0.000086,0.199846 C 0.333191,-0.466707 0.666467,0.866570 0.999914,0.199846 L 0.999914,0.799812 C 0.666467,1.466535 0.333191,0.133088 -0.000086,0.799812 L -0.000086,0.199846 Z"#;
pub(super) const WAVE_ADJ_DEEP_SHIFT_NORMALIZED_PATH: &str = r#"M -0.000086,0.199846 C 0.266433,-0.466707 0.533122,0.866570 0.799812,0.199846 L 0.999914,0.799812 C 0.733225,1.466535 0.466535,0.133088 0.199846,0.799812 L -0.000086,0.199846 Z"#;
pub(super) fn wave_adjust_anchor(adj: &HashMap<String, f64>) -> &'static str {
    let adj1 = adj.get("adj1").copied().unwrap_or(12_500.0);
    let adj2 = adj.get("adj2").copied().unwrap_or(0.0);
    let anchors = [
        (10_000.0, 0.0, WAVE_ADJ_LIGHT_NORMALIZED_PATH),
        (12_500.0, 40_000.0, WAVE_ADJ_SHIFT_NORMALIZED_PATH),
        (30_000.0, 0.0, WAVE_ADJ_DEEP_NORMALIZED_PATH),
        (30_000.0, 40_000.0, WAVE_ADJ_DEEP_SHIFT_NORMALIZED_PATH),
    ];

    anchors
        .into_iter()
        .min_by(|(a1x, a2x, _), (a1y, a2y, _)| {
            let dx1 = (adj1 - *a1x) / 20_000.0;
            let dx2 = (adj2 - *a2x) / 40_000.0;
            let dy1 = (adj1 - *a1y) / 20_000.0;
            let dy2 = (adj2 - *a2y) / 40_000.0;
            (dx1 * dx1 + dx2 * dx2)
                .partial_cmp(&(dy1 * dy1 + dy2 * dy2))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, _, path)| path)
        .unwrap_or(WAVE_ADJ_LIGHT_NORMALIZED_PATH)
}
pub(super) fn wave_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    scale_normalized_path(wave_adjust_anchor(adj), w, h)
}
pub(super) const DOUBLE_WAVE_ADJ_LIGHT_NORMALIZED_PATH: &str = r#"M -0.000086,0.099880 C 0.166467,-0.233396 0.333191,0.433156 0.499914,0.099880 0.666467,-0.233396 0.833191,0.433156 0.999914,0.099880 L 0.999914,0.899777 C 0.833191,1.233225 0.666467,0.566501 0.499914,0.899777 0.333191,1.233225 0.166467,0.566501 -0.000086,0.899777 L -0.000086,0.099880 Z"#;
pub(super) const DOUBLE_WAVE_ADJ_SHIFT_NORMALIZED_PATH: &str = r#"M -0.000086,0.124872 C 0.133088,-0.291595 0.266433,0.541510 0.399777,0.124872 0.533122,-0.291595 0.666467,0.541510 0.799812,0.124872 L 0.999914,0.874786 C 0.866570,1.291424 0.733225,0.458148 0.599880,0.874786 0.466535,1.291424 0.333191,0.458148 0.199846,0.874786 L -0.000086,0.124872 Z"#;
pub(super) const DOUBLE_WAVE_ADJ_DEEP_NORMALIZED_PATH: &str = r#"M -0.000086,0.124872 C 0.166467,-0.291595 0.333191,0.541510 0.499914,0.124872 0.666467,-0.291595 0.833191,0.541510 0.999914,0.124872 L 0.999914,0.874786 C 0.833191,1.291424 0.666467,0.458148 0.499914,0.874786 0.333191,1.291424 0.166467,0.458148 -0.000086,0.874786 L -0.000086,0.124872 Z"#;
pub(super) const DOUBLE_WAVE_ADJ_DEEP_SHIFT_NORMALIZED_PATH: &str = r#"M -0.000086,0.124872 C 0.133088,-0.291595 0.266433,0.541510 0.399777,0.124872 0.533122,-0.291595 0.666467,0.541510 0.799812,0.124872 L 0.999914,0.874786 C 0.866570,1.291424 0.733225,0.458148 0.599880,0.874786 0.466535,1.291424 0.333191,0.458148 0.199846,0.874786 L -0.000086,0.124872 Z"#;
pub(super) fn double_wave_adjust_anchor(adj: &HashMap<String, f64>) -> &'static str {
    let adj1 = adj.get("adj1").copied().unwrap_or(6_250.0);
    let adj2 = adj.get("adj2").copied().unwrap_or(0.0);
    let anchors = [
        (10_000.0, 0.0, DOUBLE_WAVE_ADJ_LIGHT_NORMALIZED_PATH),
        (12_500.0, 40_000.0, DOUBLE_WAVE_ADJ_SHIFT_NORMALIZED_PATH),
        (30_000.0, 0.0, DOUBLE_WAVE_ADJ_DEEP_NORMALIZED_PATH),
        (
            30_000.0,
            40_000.0,
            DOUBLE_WAVE_ADJ_DEEP_SHIFT_NORMALIZED_PATH,
        ),
    ];

    anchors
        .into_iter()
        .min_by(|(a1x, a2x, _), (a1y, a2y, _)| {
            let dx1 = (adj1 - *a1x) / 20_000.0;
            let dx2 = (adj2 - *a2x) / 40_000.0;
            let dy1 = (adj1 - *a1y) / 20_000.0;
            let dy2 = (adj2 - *a2y) / 40_000.0;
            (dx1 * dx1 + dx2 * dx2)
                .partial_cmp(&(dy1 * dy1 + dy2 * dy2))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, _, path)| path)
        .unwrap_or(DOUBLE_WAVE_ADJ_LIGHT_NORMALIZED_PATH)
}
pub(super) fn double_wave_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    scale_normalized_path(double_wave_adjust_anchor(adj), w, h)
}
pub(super) fn regular_polygon_path(w: f64, h: f64, sides: u32) -> String {
    scaled_polygon_path(w, h, sides, 1.0, 1.0)
}

pub(super) fn heptagon_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let horizontal = adj.get("hf").copied().unwrap_or(102_572.0) / 100_000.0;
    let vertical = adj.get("vf").copied().unwrap_or(105_210.0) / 100_000.0;
    scaled_polygon_path(w, h, 7, horizontal, vertical)
}

pub(super) fn decagon_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let vertical = adj.get("vf").copied().unwrap_or(105_146.0) / 100_000.0;
    scaled_polygon_path(w, h, 10, 1.0, vertical)
}

fn scaled_polygon_path(w: f64, h: f64, sides: u32, horizontal: f64, vertical: f64) -> String {
    let (cx, cy) = (w / 2.0, h / 2.0);
    let radius_x = cx * horizontal;
    let radius_y = cy * vertical;
    let st = -std::f64::consts::FRAC_PI_2;
    let mut pts: Vec<(f64, f64)> = Vec::with_capacity(sides as usize);
    for i in 0..sides {
        let a = st + 2.0 * std::f64::consts::PI * (i as f64) / (sides as f64);
        pts.push((cx + radius_x * a.cos(), cy + radius_y * a.sin()));
    }
    let mut p = format!("M{:.1},{:.1}", pts[0].0, pts[0].1);
    for &(x, y) in &pts[1..] {
        p.push_str(&format!(" L{x:.1},{y:.1}"));
    }
    p.push_str(" Z");
    p
}
pub(super) fn funnel_path(w: f64, h: f64) -> String {
    let (sx, sy) = scale_unit_point(w, h, 0.004682, 0.284492);
    let (c1x, c1y) = scale_unit_point(w, h, 0.001490, 0.272852);
    let (c2x, c2y) = scale_unit_point(w, h, -0.000213, 0.261212);
    let (x1, y1) = scale_unit_point(w, h, -0.000213, 0.249572);
    let (c3x, c3y) = scale_unit_point(w, h, 0.022984, 0.205751);
    let (c4x, c4y) = scale_unit_point(w, h, 0.066823, 0.162616);
    let (x2, y2) = scale_unit_point(w, h, 0.110662, 0.124615);
    let (c5x, c5y) = scale_unit_point(w, h, 0.173867, 0.086614);
    let (c6x, c6y) = scale_unit_point(w, h, 0.249840, 0.055118);
    let (x3, y3) = scale_unit_point(w, h, 0.325814, 0.033208);
    let (c7x, c7y) = scale_unit_point(w, h, 0.412003, 0.011298);
    let (c8x, c8y) = scale_unit_point(w, h, 0.499894, -0.000342);
    let (x4, y4) = scale_unit_point(w, h, 0.587572, -0.000342);
    let (c9x, c9y) = scale_unit_point(w, h, 0.673760, 0.011298);
    let (c10x, c10y) = scale_unit_point(w, h, 0.749734, 0.033208);
    let (x5, y5) = scale_unit_point(w, h, 0.825920, 0.055118);
    let (c11x, c11y) = scale_unit_point(w, h, 0.888913, 0.086614);
    let (c12x, c12y) = scale_unit_point(w, h, 0.932752, 0.124615);
    let (x6, y6) = scale_unit_point(w, h, 0.976804, 0.162616);
    let (c13x, c13y) = scale_unit_point(w, h, 0.999787, 0.205751);
    let (c14x, c14y) = scale_unit_point(w, h, 0.999787, 0.249572);
    let (x7, y7) = scale_unit_point(w, h, 0.999787, 0.261212);
    let (c15x, c15y) = scale_unit_point(w, h, 0.998085, 0.272852);
    let (c16x, c16y) = scale_unit_point(w, h, 0.994893, 0.284492);
    let (x8, y8) = scale_unit_point(w, h, 0.623750, 0.945909);
    let (c17x, c17y) = scale_unit_point(w, h, 0.621622, 0.953783);
    let (c18x, c18y) = scale_unit_point(w, h, 0.616301, 0.961657);
    let (x9, y9) = scale_unit_point(w, h, 0.608215, 0.968504);
    let (c19x, c19y) = scale_unit_point(w, h, 0.597148, 0.978090);
    let (c20x, c20y) = scale_unit_point(w, h, 0.581400, 0.985964);
    let (x10, y10) = scale_unit_point(w, h, 0.562460, 0.991441);
    let (c21x, c21y) = scale_unit_point(w, h, 0.543520, 0.996919);
    let (c22x, c22y) = scale_unit_point(w, h, 0.521813, 0.999658);
    let (x11, y11) = scale_unit_point(w, h, 0.499894, 0.999658);
    let (c23x, c23y) = scale_unit_point(w, h, 0.477974, 0.999658);
    let (c24x, c24y) = scale_unit_point(w, h, 0.456480, 0.996919);
    let (x12, y12) = scale_unit_point(w, h, 0.437540, 0.991441);
    let (c25x, c25y) = scale_unit_point(w, h, 0.418387, 0.985964);
    let (c26x, c26y) = scale_unit_point(w, h, 0.402639, 0.978090);
    let (x13, y13) = scale_unit_point(w, h, 0.391785, 0.968504);
    let (c27x, c27y) = scale_unit_point(w, h, 0.383699, 0.961657);
    let (c28x, c28y) = scale_unit_point(w, h, 0.378378, 0.953783);
    let (x14, y14) = scale_unit_point(w, h, 0.376250, 0.945909);
    let outer = format!(
        "M{sx:.1},{sy:.1} C{c1x:.1},{c1y:.1} {c2x:.1},{c2y:.1} {x1:.1},{y1:.1} C{c3x:.1},{c3y:.1} {c4x:.1},{c4y:.1} {x2:.1},{y2:.1} C{c5x:.1},{c5y:.1} {c6x:.1},{c6y:.1} {x3:.1},{y3:.1} C{c7x:.1},{c7y:.1} {c8x:.1},{c8y:.1} {x4:.1},{y4:.1} C{c9x:.1},{c9y:.1} {c10x:.1},{c10y:.1} {x5:.1},{y5:.1} C{c11x:.1},{c11y:.1} {c12x:.1},{c12y:.1} {x6:.1},{y6:.1} C{c13x:.1},{c13y:.1} {c14x:.1},{c14y:.1} {x7:.1},{y7:.1} C{c15x:.1},{c15y:.1} {c16x:.1},{c16y:.1} {x8:.1},{y8:.1} C{c17x:.1},{c17y:.1} {c18x:.1},{c18y:.1} {x9:.1},{y9:.1} C{c19x:.1},{c19y:.1} {c20x:.1},{c20y:.1} {x10:.1},{y10:.1} C{c21x:.1},{c21y:.1} {c22x:.1},{c22y:.1} {x11:.1},{y11:.1} C{c23x:.1},{c23y:.1} {c24x:.1},{c24y:.1} {x12:.1},{y12:.1} C{c25x:.1},{c25y:.1} {c26x:.1},{c26y:.1} {x13:.1},{y13:.1} C{c27x:.1},{c27y:.1} {c28x:.1},{c28y:.1} {x14:.1},{y14:.1} L{sx:.1},{sy:.1} Z"
    );
    let hole_cx = w * 0.49955;
    let hole_cy = h * 0.26229;
    let hole_rx = w * 0.45225;
    let hole_ry = h * 0.17101;
    let hole = format!(
        "M{left:.1},{cy:.1} A{rx:.1},{ry:.1} 0 1,0 {right:.1},{cy:.1} A{rx:.1},{ry:.1} 0 1,0 {left:.1},{cy:.1} Z",
        left = hole_cx - hole_rx,
        right = hole_cx + hole_rx,
        cy = hole_cy,
        rx = hole_rx,
        ry = hole_ry
    );
    format!("{outer} {hole}")
}
pub(super) const TEARDROP_ADJ_LIGHT_NORMALIZED_PATH: &str = r#"M -0.000086,0.499914 L -0.000086,0.499914 C -0.000086,0.412102 0.023023,0.326001 0.066844,0.250000 0.110835,0.173827 0.173827,0.110835 0.249829,0.066844 0.326001,0.023023 0.412102,-0.000086 0.499914,-0.000086 0.533293,-0.000086 0.566501,0.133259 0.599880,0.399949 0.866570,0.433328 0.999914,0.466535 0.999914,0.499914 L 0.999914,0.499914 C 0.999914,0.587727 0.976806,0.673827 0.932985,0.750000 0.888993,0.826001 0.826001,0.888993 0.750000,0.932985 0.673827,0.976806 0.587727,0.999914 0.499914,0.999914 L 0.499914,0.999914 C 0.412102,0.999914 0.326001,0.976806 0.249829,0.932985 0.173827,0.888993 0.110835,0.826001 0.066844,0.750000 0.023023,0.673827 -0.000086,0.587727 -0.000086,0.499914 L -0.000086,0.499914 Z"#;
pub(super) const TEARDROP_ADJ_DEFAULT_NORMALIZED_PATH: &str = r#"M -0.000086,0.499914 L -0.000086,0.499914 C -0.000086,0.412102 0.023023,0.326001 0.066844,0.250000 0.110835,0.173827 0.173827,0.110835 0.250000,0.066844 0.326001,0.023023 0.412102,-0.000086 0.499914,-0.000086 0.583276,-0.000086 0.666638,0.083276 0.750000,0.250000 0.916553,0.333191 0.999914,0.416553 0.999914,0.499914 L 0.999914,0.499914 C 0.999914,0.587727 0.976806,0.673827 0.932985,0.750000 0.888993,0.826001 0.826001,0.888993 0.750000,0.932985 0.673827,0.976806 0.587727,0.999914 0.499914,0.999914 L 0.499914,0.999914 C 0.412102,0.999914 0.326001,0.976806 0.250000,0.932985 0.173827,0.888993 0.110835,0.826001 0.066844,0.750000 0.023023,0.673827 -0.000086,0.587727 -0.000086,0.499914 L -0.000086,0.499914 Z"#;
pub(super) const TEARDROP_ADJ_DEEP_NORMALIZED_PATH: &str = r#"M -0.000086,0.499914 L -0.000086,0.499914 C -0.000086,0.412102 0.023023,0.326001 0.066844,0.250000 0.110835,0.173827 0.173827,0.110835 0.249829,0.066844 0.326001,0.023023 0.412102,-0.000086 0.499914,-0.000086 0.633259,-0.000086 0.766604,0.033293 0.899949,0.099880 0.966535,0.233225 0.999914,0.366570 0.999914,0.499914 L 0.999914,0.499914 C 0.999914,0.587727 0.976806,0.673827 0.932985,0.750000 0.888993,0.826001 0.826001,0.888993 0.750000,0.932985 0.673827,0.976806 0.587727,0.999914 0.499914,0.999914 L 0.499914,0.999914 C 0.412102,0.999914 0.326001,0.976806 0.249829,0.932985 0.173827,0.888993 0.110835,0.826001 0.066844,0.750000 0.023023,0.673827 -0.000086,0.587727 -0.000086,0.499914 L -0.000086,0.499914 Z"#;
pub(super) const TEARDROP_ADJ_SHARP_NORMALIZED_PATH: &str = r#"M -0.000086,0.499914 L -0.000086,0.499914 C -0.000086,0.412102 0.023023,0.326001 0.066844,0.250000 0.110835,0.173827 0.173827,0.110835 0.250000,0.066844 0.326001,0.023023 0.412102,-0.000086 0.499914,-0.000086 0.666638,-0.000086 0.833191,-0.000086 0.999914,-0.000086 0.999914,0.166638 0.999914,0.333191 0.999914,0.499914 L 0.999914,0.499914 C 0.999914,0.587727 0.976806,0.673827 0.932985,0.750000 0.888993,0.826001 0.826001,0.888993 0.750000,0.932985 0.673827,0.976806 0.587727,0.999914 0.499914,0.999914 L 0.499914,0.999914 C 0.412102,0.999914 0.326001,0.976806 0.250000,0.932985 0.173827,0.888993 0.110835,0.826001 0.066844,0.750000 0.023023,0.673827 -0.000086,0.587727 -0.000086,0.499914 L -0.000086,0.499914 Z"#;
pub(super) fn teardrop_adjust_anchor(adj: &HashMap<String, f64>) -> &'static str {
    let ratio = adj.get("adj").copied().unwrap_or(100_000.0);
    let anchors = [
        (20_000.0, TEARDROP_ADJ_LIGHT_NORMALIZED_PATH),
        (50_000.0, TEARDROP_ADJ_DEFAULT_NORMALIZED_PATH),
        (80_000.0, TEARDROP_ADJ_DEEP_NORMALIZED_PATH),
        (100_000.0, TEARDROP_ADJ_SHARP_NORMALIZED_PATH),
    ];

    anchors
        .into_iter()
        .min_by(|(ax, _), (ay, _)| {
            let dx = (ratio - *ax) / 80_000.0;
            let dy = (ratio - *ay) / 80_000.0;
            (dx * dx)
                .partial_cmp(&(dy * dy))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(_, path)| path)
        .unwrap_or(TEARDROP_ADJ_DEFAULT_NORMALIZED_PATH)
}
pub(super) fn teardrop_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    if adj.is_empty() {
        let ratio = adj.get("adj").copied().unwrap_or(100000.0) / 100_000.0;
        let (rx, ry) = (w / 2.0, h / 2.0);
        let tip_x = rx + rx * ratio;
        return format!(
            "M{tip_x:.1},0 L{tip_x:.1},{ry:.1} A{rx:.1},{ry:.1} 0 1,1 {rx:.1},0 Z",
            tip_x = tip_x.min(w),
            rx = rx,
            ry = ry
        );
    }

    scale_normalized_path(teardrop_adjust_anchor(adj), w, h)
}
