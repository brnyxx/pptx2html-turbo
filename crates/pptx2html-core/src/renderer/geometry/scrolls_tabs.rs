// Auto-split from renderer/geometry.rs (mechanical move, no logic edits).
// Family: scrolls_tabs

use super::shared::scale_normalized_path;
use std::collections::HashMap;

fn finite(value: Option<f64>, default: f64) -> f64 {
    value.filter(|value| value.is_finite()).unwrap_or(default)
}

// Scrolls
pub(super) fn horizontal_scroll_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let a = finite(adj.get("adj").copied(), 12_500.0).clamp(0.0, 25_000.0);
    let r = w.min(h) * a / 100_000.0;
    let r2 = r / 2.0;
    let (x, y1, y2) = (w - r, h - r, h - r2);
    format!(
        "M{r:.1},{r:.1} L{x:.1},{r:.1} A{r2:.1},{r2:.1} 0 0,1 {x:.1},{r2:.1} A{r2:.1},{r2:.1} 0 0,1 {w:.1},{r:.1} L{w:.1},{y1:.1} A{r2:.1},{r2:.1} 0 0,1 {w:.1},{y2:.1} A{r2:.1},{r2:.1} 0 0,1 {x:.1},{y1:.1} L{r:.1},{y1:.1} A{r2:.1},{r2:.1} 0 0,1 {r:.1},{y2:.1} A{r2:.1},{r2:.1} 0 0,1 0,{y1:.1} L0,{r:.1} A{r2:.1},{r2:.1} 0 0,1 0,{r2:.1} A{r2:.1},{r2:.1} 0 0,1 {r:.1},{r:.1} Z",
        r = r,
        r2 = r2,
        x = x,
        w = w,
        y1 = y1,
        y2 = y2
    )
}
pub(super) fn vertical_scroll_path(w: f64, h: f64, adj: &HashMap<String, f64>) -> String {
    let a = finite(adj.get("adj").copied(), 12_500.0).clamp(0.0, 25_000.0);
    let r = w.min(h) * a / 100_000.0;
    let body_left = r * 0.8;
    let body_top = r * 1.2;
    let body_right = w - r * 0.4;
    let body_bottom = h - r * 0.2;
    let body_radius = r * 0.45;
    let lip_left = r * 0.35;
    let lip_right = w;
    let lip_height = r * 1.1;
    let curl_r = r * 0.45;
    format!(
        "M{body_left:.1},{body_top:.1} L{body_right_minus:.1},{body_top:.1} Q{body_right:.1},{body_top:.1} {body_right:.1},{body_top_plus:.1} L{body_right:.1},{body_bottom_minus:.1} Q{body_right:.1},{body_bottom:.1} {body_right_minus:.1},{body_bottom:.1} L{body_left:.1},{body_bottom:.1} Q0,{body_bottom:.1} 0,{body_bottom_minus:.1} L0,{body_top_plus:.1} Q0,{body_top:.1} {body_left:.1},{body_top:.1} Z \
         M{lip_left_plus:.1},0 L{lip_right_minus:.1},0 Q{lip_right:.1},0 {lip_right:.1},{curl_r:.1} Q{lip_right:.1},{lip_height:.1} {lip_right_minus:.1},{lip_height:.1} L{lip_left_plus:.1},{lip_height:.1} Q{lip_left:.1},{lip_height:.1} {lip_left:.1},{curl_r:.1} Q{lip_left:.1},0 {lip_left_plus:.1},0 Z \
         M{curl_center:.1},{curl_r:.1} A{curl_r:.1},{curl_r:.1} 0 1,1 {curl_center:.1},{lip_height_minus:.1} A{curl_r:.1},{curl_r:.1} 0 1,1 {curl_center:.1},{curl_r:.1} Z \
         M{curl_center_bottom:.1},{body_bottom_minus_curl:.1} A{curl_r:.1},{curl_r:.1} 0 1,1 {curl_center_bottom:.1},{body_bottom_plus_curl:.1} A{curl_r:.1},{curl_r:.1} 0 1,1 {curl_center_bottom:.1},{body_bottom_minus_curl:.1} Z",
        body_left = body_left,
        body_right = body_right,
        body_right_minus = body_right - body_radius,
        body_top = body_top,
        body_top_plus = body_top + body_radius,
        body_bottom = body_bottom,
        body_bottom_minus = body_bottom - body_radius,
        lip_left = lip_left,
        lip_left_plus = lip_left + curl_r,
        lip_right = lip_right,
        lip_right_minus = lip_right - curl_r,
        lip_height = lip_height,
        lip_height_minus = lip_height - curl_r,
        curl_r = curl_r,
        curl_center = lip_left + curl_r,
        curl_center_bottom = curl_r,
        body_bottom_minus_curl = body_bottom - curl_r * 2.0,
        body_bottom_plus_curl = body_bottom
    )
}
// Tabs
pub(super) const CORNER_TABS_DEFAULT_NORMALIZED_PATH: &str = r#"M 0.099415,0.000000 L 0.000000,0.061594 L 0.000000,0.000000 Z M 0.953216,0.032609 L 0.906433,0.000000 L 1.000000,0.000000 L 1.000000,0.061594 Z M 0.070175,0.985507 L 0.099415,1.000000 L 0.000000,1.000000 L 0.000000,0.942029 Z M 0.988304,0.952899 L 1.000000,0.942029 L 1.000000,1.000000 L 0.906433,1.000000 Z"#;
pub(super) fn corner_tabs_path(w: f64, h: f64) -> String {
    scale_normalized_path(CORNER_TABS_DEFAULT_NORMALIZED_PATH, w, h)
}
pub(super) const PLAQUE_TABS_DEFAULT_NORMALIZED_PATH: &str = r#"M 0.000000,0.077586 L 0.000000,0.000000 L 0.077586,0.000000 L 0.077586,0.021552 L 0.068966,0.030172 L 0.068966,0.043103 L 0.043103,0.068966 Z M 1.000000,0.077586 L 0.987069,0.077586 L 0.987069,0.073276 L 0.974138,0.073276 L 0.974138,0.068966 L 0.956897,0.064655 L 0.931034,0.030172 L 0.926724,0.000000 L 1.000000,0.000000 Z M 0.000000,0.926724 L 0.043103,0.935345 L 0.068966,0.961207 L 0.068966,0.974138 L 0.077586,0.982759 L 0.077586,1.000000 L 0.000000,1.000000 Z M 0.987069,0.931034 L 0.987069,0.926724 L 1.000000,0.926724 L 1.000000,1.000000 L 0.926724,1.000000 L 0.931034,0.974138 L 0.935345,0.974138 L 0.939655,0.956897 L 0.956897,0.939655 L 0.965517,0.939655 L 0.974138,0.931034 Z"#;
pub(super) fn plaque_tabs_path(w: f64, h: f64) -> String {
    scale_normalized_path(PLAQUE_TABS_DEFAULT_NORMALIZED_PATH, w, h)
}
pub(super) const SQUARE_TABS_DEFAULT_NORMALIZED_PATH: &str = r#"M 0.077586,0.073276 L 0.073276,0.077586 L 0.000000,0.077586 L 0.000000,0.000000 L 0.077586,0.000000 Z M 1.000000,0.077586 L 0.931034,0.077586 L 0.926724,0.073276 L 0.926724,0.000000 L 1.000000,0.000000 Z M 0.000000,0.926724 L 0.073276,0.926724 L 0.077586,0.931034 L 0.077586,1.000000 L 0.000000,1.000000 Z M 0.926724,1.000000 L 0.926724,0.931034 L 0.931034,0.926724 L 1.000000,0.926724 L 1.000000,1.000000 Z"#;
pub(super) fn square_tabs_path(w: f64, h: f64) -> String {
    scale_normalized_path(SQUARE_TABS_DEFAULT_NORMALIZED_PATH, w, h)
}
