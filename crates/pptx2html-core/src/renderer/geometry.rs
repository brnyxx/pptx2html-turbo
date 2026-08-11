//! Preset shape SVG path generation for all 187 OOXML preset geometries.
//! Split mechanically from the original monolith into family submodules.
//! Public API (needs_evenodd_fill, preset_shape_svg, preset_shape_multi_svg,
//! action_button_information_icon_paths, custom_geometry_svg, CustomGeomSvg,
//! CustomGeomPathSvg) is preserved at this path.

use std::collections::HashMap;

mod action_buttons;
mod arcs;
mod arrow_callouts;
mod arrows;
mod basic_shapes;
mod bent_u_arrows;
mod brackets_braces;
mod callouts;
mod chart_shapes;
mod circular_arrows;
mod connectors;
mod curved_arrows;
mod custom_geom;
mod flowchart;
mod math;
mod misc;
mod official_presets;
mod official_presets_formula;
mod official_presets_path;
#[cfg(test)]
mod official_presets_review_tests;
mod official_presets_schema;
mod official_presets_xml;
mod official_remaining_presets;
mod rects;
mod scrolls_tabs;
mod shared;
mod stars;
mod waves_polys;

#[cfg(test)]
mod tests;

// Public API re-exports. `CustomGeomPathSvg` is only reached via field access
// on `CustomGeomSvg.paths`, so rustc flags the direct import as unused even
// though the struct is part of the stable external API.
use crate::model::PathFill;
pub use action_buttons::action_button_information_icon_paths;
pub use custom_geom::custom_geometry_svg;
#[allow(unused_imports)]
pub use shared::{CustomGeomPathSvg, CustomGeomSvg};

/// Returns true if this preset shape needs `fill-rule="evenodd"` to render holes correctly.
/// Shapes with inner cutouts (donut, frame, noSmoking, blockArc, etc.) use two subpaths
/// where the inner subpath winds in the opposite direction to create a hole.
pub fn needs_evenodd_fill(name: &str) -> bool {
    matches!(
        name,
        "donut"
            | "frame"
            | "noSmoking"
            | "blockArc"
            | "bracePair"
            | "bracketPair"
            | "bevel"
            | "funnel"
    )
}

/// Generate an SVG path string for a named preset shape.
/// Returns `None` if the shape name is not supported.
pub fn preset_shape_svg(
    name: &str,
    w: f64,
    h: f64,
    adjust_values: &HashMap<String, f64>,
) -> Option<String> {
    match official_presets::route(name, w, h, adjust_values) {
        official_presets::OfficialPresetRender::Rendered(svg)
        | official_presets::OfficialPresetRender::Invalid(svg) => {
            return Some(
                svg.paths
                    .into_iter()
                    .map(|path| path.d)
                    .collect::<Vec<_>>()
                    .join(" "),
            );
        }
        official_presets::OfficialPresetRender::NotOfficial => {}
    }
    match name {
        // Basic shapes
        "rect" => Some(basic_shapes::rect_path(w, h)),
        "roundRect" => Some(basic_shapes::round_rect_path(w, h, adjust_values)),
        "ellipse" => Some(basic_shapes::ellipse_path(w, h)),
        "triangle" | "isosTriangle" => Some(basic_shapes::triangle_path(w, h, adjust_values)),
        "rtTriangle" => Some(basic_shapes::rt_triangle_path(w, h)),
        "diamond" => Some(basic_shapes::diamond_path(w, h)),
        "parallelogram" => Some(basic_shapes::parallelogram_path(w, h, adjust_values)),
        "trapezoid" => Some(basic_shapes::trapezoid_path(w, h, adjust_values)),
        "pentagon" => Some(basic_shapes::pentagon_path(w, h, adjust_values)),
        "hexagon" => Some(basic_shapes::hexagon_path(w, h, adjust_values)),
        "octagon" => Some(basic_shapes::octagon_path(w, h, adjust_values)),
        // Snip/Fold/Round corner rectangles
        "snip1Rect" => Some(rects::snip1_rect_path(w, h, adjust_values)),
        "snip2SameRect" => Some(rects::snip2_same_rect_path(w, h, adjust_values)),
        "snip2DiagRect" => Some(rects::snip2_diag_rect_path(w, h, adjust_values)),
        "snipRoundRect" => Some(rects::snip_round_rect_path(w, h, adjust_values)),
        "round1Rect" => Some(rects::round1_rect_path(w, h, adjust_values)),
        "round2SameRect" => Some(rects::round2_same_rect_path(w, h, adjust_values)),
        "round2DiagRect" => Some(rects::round2_diag_rect_path(w, h, adjust_values)),
        "foldCorner" | "foldedCorner" => Some(rects::fold_corner_path(w, h, adjust_values)),
        // Rectangles & related
        "diagStripe" => Some(rects::diag_stripe_path(w, h, adjust_values)),
        "corner" => Some(rects::corner_path(w, h, adjust_values)),
        "plaque" => Some(rects::plaque_path(w, h, adjust_values)),
        "bracePair" => Some(brackets_braces::brace_pair_path(w, h, adjust_values)),
        "bracketPair" => Some(brackets_braces::bracket_pair_path(w, h, adjust_values)),
        "halfFrame" => Some(brackets_braces::half_frame_path(w, h, adjust_values)),
        "line" => Some(rects::line_path(w, h)),
        // Arrows
        "rightArrow" => Some(arrows::right_arrow_path(w, h, adjust_values)),
        "leftArrow" => Some(arrows::left_arrow_path(w, h, adjust_values)),
        "upArrow" => Some(arrows::up_arrow_path(w, h, adjust_values)),
        "downArrow" => Some(arrows::down_arrow_path(w, h, adjust_values)),
        "leftRightArrow" => Some(arrows::left_right_arrow_path(w, h, adjust_values)),
        "upDownArrow" => Some(arrows::up_down_arrow_path(w, h, adjust_values)),
        "bentArrow" => Some(arrows::bent_arrow_path(w, h, adjust_values)),
        "chevron" => Some(arrows::chevron_path(w, h, adjust_values)),
        "notchedRightArrow" => Some(arrows::notched_right_arrow_path(w, h, adjust_values)),
        "stripedRightArrow" => Some(arrows::striped_right_arrow_path(w, h, adjust_values)),
        "curvedRightArrow" => Some(curved_arrows::curved_right_arrow_path(w, h, adjust_values)),
        "curvedLeftArrow" => Some(curved_arrows::curved_left_arrow_path(w, h, adjust_values)),
        "curvedUpArrow" => Some(curved_arrows::curved_up_arrow_path(w, h, adjust_values)),
        "curvedDownArrow" => Some(curved_arrows::curved_down_arrow_path(w, h, adjust_values)),
        "circularArrow" => Some(circular_arrows::circular_arrow_path(w, h, adjust_values)),
        "bentUpArrow" => Some(bent_u_arrows::bent_up_arrow_path(w, h, adjust_values)),
        "uturnArrow" => Some(bent_u_arrows::uturn_arrow_path(w, h, adjust_values)),
        "leftRightUpArrow" => Some(bent_u_arrows::left_right_up_arrow_path(w, h, adjust_values)),
        "quadArrow" => Some(bent_u_arrows::quad_arrow_path(w, h, adjust_values)),
        "leftUpArrow" => Some(bent_u_arrows::left_up_arrow_path(w, h, adjust_values)),
        "homePlate" => Some(arrows::home_plate_path(w, h, adjust_values)),
        // Callouts
        "wedgeRoundRectCallout" => {
            Some(callouts::wedge_round_rect_callout_path(w, h, adjust_values))
        }
        "wedgeEllipseCallout" => Some(callouts::wedge_ellipse_callout_path(w, h, adjust_values)),
        "cloudCallout" => Some(callouts::cloud_callout_path(w, h, adjust_values)),
        "callout1" => Some(callouts::callout1_path(w, h, adjust_values)),
        "callout2" => Some(callouts::callout2_path(w, h, adjust_values)),
        "callout3" => Some(callouts::callout3_path(w, h, adjust_values)),
        "borderCallout1" => Some(callouts::border_callout1_path(w, h, adjust_values)),
        "borderCallout2" => Some(callouts::border_callout2_path(w, h, adjust_values)),
        "borderCallout3" => Some(callouts::border_callout3_path(w, h, adjust_values)),
        "accentCallout1" => Some(callouts::accent_callout1_path(w, h, adjust_values)),
        "accentCallout2" => Some(callouts::accent_callout2_path(w, h, adjust_values)),
        "accentCallout3" => Some(callouts::accent_callout3_path(w, h, adjust_values)),
        "accentBorderCallout1" => Some(callouts::accent_border_callout1_path(w, h, adjust_values)),
        "accentBorderCallout2" => Some(callouts::accent_border_callout2_path(w, h, adjust_values)),
        "accentBorderCallout3" => Some(callouts::accent_border_callout3_path(w, h, adjust_values)),
        "wedgeRectCallout" => Some(callouts::wedge_rect_callout_path(w, h, adjust_values)),
        // Flowchart
        "flowChartProcess" => Some(basic_shapes::rect_path(w, h)),
        "flowChartDecision" => Some(basic_shapes::diamond_path(w, h)),
        "flowChartTerminator" => Some(flowchart::flowchart_terminator_path(w, h)),
        "flowChartDocument" => Some(flowchart::flowchart_document_path(w, h, adjust_values)),
        "flowChartPredefinedProcess" => Some(flowchart::flowchart_predefined_process_path(w, h)),
        "flowChartAlternateProcess" => Some(flowchart::flowchart_alternate_process_path(
            w,
            h,
            adjust_values,
        )),
        "flowChartManualInput" => Some(flowchart::flowchart_manual_input_path(w, h, adjust_values)),
        "flowChartConnector" => Some(basic_shapes::ellipse_path(w, h)),
        "flowChartInputOutput" => Some(flowchart::flowchart_input_output_path(w, h, adjust_values)),
        "flowChartInternalStorage" => Some(flowchart::flowchart_internal_storage_path(w, h)),
        "flowChartMultidocument" => {
            Some(flowchart::flowchart_multidocument_path(w, h, adjust_values))
        }
        "flowChartPreparation" => Some(flowchart::flowchart_preparation_path(w, h)),
        "flowChartManualOperation" => Some(flowchart::flowchart_manual_operation_path(w, h)),
        "flowChartOffpageConnector" => Some(flowchart::flowchart_offpage_connector_path(w, h)),
        "flowChartPunchedCard" => Some(flowchart::flowchart_punched_card_path(w, h)),
        "flowChartPunchedTape" => Some(flowchart::flowchart_punched_tape_path(w, h)),
        "flowChartSummingJunction" => Some(flowchart::flowchart_summing_junction_path(w, h)),
        "flowChartOr" => Some(flowchart::flowchart_or_path(w, h)),
        "flowChartCollate" => Some(flowchart::flowchart_collate_path(w, h)),
        "flowChartSort" => Some(flowchart::flowchart_sort_path(w, h)),
        "flowChartExtract" => Some(flowchart::flowchart_extract_path(w, h)),
        "flowChartMerge" => Some(flowchart::flowchart_merge_path(w, h)),
        "flowChartOnlineStorage" => Some(flowchart::flowchart_online_storage_path(w, h)),
        "flowChartDelay" => Some(flowchart::flowchart_delay_path(w, h)),
        "flowChartMagneticTape" => Some(flowchart::flowchart_magnetic_tape_path(w, h)),
        "flowChartMagneticDisk" => Some(flowchart::flowchart_magnetic_disk_path(w, h)),
        "flowChartMagneticDrum" => Some(flowchart::flowchart_magnetic_drum_path(w, h)),
        "flowChartDisplay" => Some(flowchart::flowchart_display_path(w, h)),
        // Action buttons
        "actionButtonBlank" => Some(action_buttons::action_button_blank_path(w, h)),
        "actionButtonHome" => Some(action_buttons::action_button_icon_path(w, h, "home")),
        "actionButtonHelp" => Some(action_buttons::action_button_icon_path(w, h, "help")),
        "actionButtonInformation" => Some(action_buttons::action_button_icon_path(w, h, "info")),
        "actionButtonBackPrevious" => Some(action_buttons::action_button_icon_path(w, h, "back")),
        "actionButtonForwardNext" => Some(action_buttons::action_button_icon_path(w, h, "forward")),
        "actionButtonBeginning" => Some(action_buttons::action_button_icon_path(w, h, "beginning")),
        "actionButtonEnd" => Some(action_buttons::action_button_icon_path(w, h, "end")),
        "actionButtonReturn" => Some(action_buttons::action_button_icon_path(w, h, "return")),
        "actionButtonDocument" => Some(action_buttons::action_button_icon_path(w, h, "document")),
        "actionButtonSound" => Some(action_buttons::action_button_icon_path(w, h, "sound")),
        "actionButtonMovie" => Some(action_buttons::action_button_icon_path(w, h, "movie")),
        // Stars & seals
        "star4" => Some(stars::star4_path(w, h, adjust_values)),
        "star5" => Some(stars::star5_path(w, h, adjust_values)),
        "star6" => Some(stars::star6_path(w, h, adjust_values)),
        "star7" => Some(stars::star7_path(w, h, adjust_values)),
        "star8" => Some(stars::star_n_path(w, h, 8, adjust_values, 34601.0)),
        "star10" => Some(stars::star10_path(w, h, adjust_values)),
        "star12" => Some(stars::star_n_path(w, h, 12, adjust_values, 37500.0)),
        "star16" => Some(stars::star_n_path(w, h, 16, adjust_values, 37500.0)),
        "star24" => Some(stars::star_n_path(w, h, 24, adjust_values, 37500.0)),
        "star32" => Some(stars::star_n_path(w, h, 32, adjust_values, 37500.0)),
        "irregularSeal1" => Some(stars::irregular_seal1_path(w, h)),
        "irregularSeal2" => Some(stars::irregular_seal2_path(w, h)),
        // Math
        "mathEqual" => Some(math::math_equal_path(w, h, adjust_values)),
        "mathNotEqual" => Some(math::math_not_equal_path(w, h, adjust_values)),
        "mathMultiply" => Some(math::math_multiply_path(w, h, adjust_values)),
        "mathDivide" => Some(math::math_divide_path(w, h, adjust_values)),
        // Other
        "heart" => Some(misc::heart_path(w, h)),
        "plus" => Some(math::preset_plus_path(w, h, adjust_values)),
        "mathPlus" => Some(math::math_plus_path(w, h, adjust_values)),
        "mathMinus" => Some(math::math_minus_path(w, h, adjust_values)),
        "lightningBolt" => Some(misc::lightning_bolt_path(w, h)),
        "cloud" => Some(misc::cloud_path(w, h)),
        "frame" => Some(misc::frame_path(w, h, adjust_values)),
        "ribbon" => Some(misc::ribbon_path(w, h, adjust_values)),
        "ribbon2" => Some(misc::ribbon2_path(w, h, adjust_values)),
        "donut" => Some(misc::donut_path(w, h, adjust_values)),
        "noSmoking" => Some(misc::no_smoking_path(w, h, adjust_values)),
        "blockArc" => Some(misc::block_arc_path(w, h, adjust_values)),
        "smileyFace" => Some(misc::smiley_face_path(w, h, adjust_values)),
        "can" => Some(misc::can_path(w, h, adjust_values)),
        "cube" => Some(misc::cube_path(w, h, adjust_values)),
        "moon" => Some(misc::moon_path(w, h, adjust_values)),
        "sun" => Some(misc::sun_path(w, h, adjust_values)),
        "bevel" => Some(misc::bevel_path(w, h, adjust_values)),
        "gear6" => Some(misc::gear_path(w, h, 6, adjust_values)),
        "gear9" => Some(misc::gear_path(w, h, 9, adjust_values)),
        "pie" => Some(arcs::pie_path(w, h, adjust_values)),
        "pieWedge" => Some(arcs::pie_wedge_path(w, h)),
        "arc" => Some(arcs::arc_path(w, h, adjust_values)),
        "wave" => Some(waves_polys::wave_path(w, h, adjust_values)),
        "doubleWave" => Some(waves_polys::double_wave_path(w, h, adjust_values)),
        "decagon" => Some(waves_polys::decagon_path(w, h, adjust_values)),
        "dodecagon" => Some(waves_polys::regular_polygon_path(w, h, 12)),
        "funnel" => Some(waves_polys::funnel_path(w, h)),
        "teardrop" => Some(waves_polys::teardrop_path(w, h, adjust_values)),
        "heptagon" => Some(waves_polys::heptagon_path(w, h, adjust_values)),
        // Arrow callouts
        "downArrowCallout" => Some(arrow_callouts::down_arrow_callout_path(w, h, adjust_values)),
        "leftArrowCallout" => Some(arrow_callouts::left_arrow_callout_path(w, h, adjust_values)),
        "rightArrowCallout" => Some(arrow_callouts::right_arrow_callout_path(
            w,
            h,
            adjust_values,
        )),
        "upArrowCallout" => Some(arrow_callouts::up_arrow_callout_path(w, h, adjust_values)),
        "quadArrowCallout" => Some(arrow_callouts::quad_arrow_callout_path(w, h, adjust_values)),
        "leftRightArrowCallout" => Some(arrow_callouts::left_right_arrow_callout_path(
            w,
            h,
            adjust_values,
        )),
        "upDownArrowCallout" => Some(arrow_callouts::up_down_arrow_callout_path(
            w,
            h,
            adjust_values,
        )),
        // Brackets and braces
        "leftBrace" => Some(brackets_braces::left_brace_path(w, h, adjust_values)),
        "rightBrace" => Some(brackets_braces::right_brace_path(w, h, adjust_values)),
        "leftBracket" => Some(brackets_braces::left_bracket_path(w, h, adjust_values)),
        "rightBracket" => Some(brackets_braces::right_bracket_path(w, h, adjust_values)),
        // Chart shapes
        "chartPlus" => Some(chart_shapes::chart_plus_path(w, h)),
        "chartStar" => Some(chart_shapes::chart_star_path(w, h)),
        "chartX" => Some(chart_shapes::chart_x_path(w, h)),
        // Scrolls
        "horizontalScroll" => Some(scrolls_tabs::horizontal_scroll_path(w, h, adjust_values)),
        "verticalScroll" => Some(scrolls_tabs::vertical_scroll_path(w, h, adjust_values)),
        // Tabs
        "cornerTabs" => Some(scrolls_tabs::corner_tabs_path(w, h)),
        "plaqueTabs" => Some(scrolls_tabs::plaque_tabs_path(w, h)),
        "squareTabs" => Some(scrolls_tabs::square_tabs_path(w, h)),
        // Ribbons
        "ellipseRibbon" => Some(basic_shapes::ellipse_ribbon_path(w, h, adjust_values)),
        "ellipseRibbon2" => Some(basic_shapes::ellipse_ribbon2_path(w, h, adjust_values)),
        // Circular arrows
        "leftCircularArrow" => Some(circular_arrows::left_circular_arrow_path(
            w,
            h,
            adjust_values,
        )),
        "leftRightCircularArrow" => Some(circular_arrows::left_right_circular_arrow_path(
            w,
            h,
            adjust_values,
        )),
        // Misc
        "chord" => Some(arcs::chord_path(w, h, adjust_values)),
        "lineInv" => Some(rects::line_inv_path(w, h)),
        "nonIsoscelesTrapezoid" => Some(basic_shapes::non_isosceles_trapezoid_path(
            w,
            h,
            adjust_values,
        )),
        "swooshArrow" => Some(arrows::swoosh_arrow_path(w, h, adjust_values)),
        "leftRightRibbon" => Some(misc::left_right_ribbon_path(w, h, adjust_values)),
        // Additional ECMA-376 ST_ShapeType shapes
        "flowChartOfflineStorage" => Some(flowchart::flowchart_offline_storage_path(w, h)),
        "cross" => Some(math::preset_plus_path(w, h, adjust_values)),
        "straightConnector1" => Some(rects::line_path(w, h)),
        "curvedConnector2" => Some(connectors::curved_connector2_path(w, h)),
        "curvedConnector3" => Some(connectors::curved_connector3_path(w, h, adjust_values)),
        "curvedConnector4" => Some(connectors::curved_connector4_path(w, h, adjust_values)),
        "curvedConnector5" => Some(connectors::curved_connector5_path(w, h, adjust_values)),
        "bentConnector2" => Some(connectors::bent_connector2_path(w, h)),
        "bentConnector3" => Some(connectors::bent_connector3_path(w, h, adjust_values)),
        "bentConnector4" => Some(connectors::bent_connector4_path(w, h, adjust_values)),
        "bentConnector5" => Some(connectors::bent_connector5_path(w, h, adjust_values)),
        _ => None,
    }
}

pub fn preset_shape_multi_svg(
    name: &str,
    w: f64,
    h: f64,
    adjust_values: &HashMap<String, f64>,
) -> Option<CustomGeomSvg> {
    match official_presets::route(name, w, h, adjust_values) {
        official_presets::OfficialPresetRender::Rendered(svg)
        | official_presets::OfficialPresetRender::Invalid(svg) => return Some(svg),
        official_presets::OfficialPresetRender::NotOfficial => {}
    }
    match name {
        "curvedLeftArrow"
            if shared::matches_curved_arrow_profile(
                adjust_values,
                12_000.0,
                70_000.0,
                18_000.0,
            ) =>
        {
            Some(shared::curved_arrow_multi_svg(
                [
                    (
                        shared::CURVED_LEFT_ARROW_MULTI_TIGHT_MAIN_PATH,
                        PathFill::Norm,
                        false,
                    ),
                    (
                        shared::CURVED_LEFT_ARROW_MULTI_TIGHT_SHADE_PATH,
                        PathFill::DarkenLess,
                        false,
                    ),
                    (
                        shared::CURVED_LEFT_ARROW_MULTI_TIGHT_OUTLINE_PATH,
                        PathFill::None,
                        true,
                    ),
                ],
                w,
                h,
            ))
        }
        "curvedLeftArrow"
            if shared::matches_curved_arrow_profile(
                adjust_values,
                42_000.0,
                30_000.0,
                42_000.0,
            ) =>
        {
            Some(shared::curved_arrow_multi_svg(
                [
                    (
                        shared::CURVED_LEFT_ARROW_MULTI_WIDE_MAIN_PATH,
                        PathFill::Norm,
                        false,
                    ),
                    (
                        shared::CURVED_LEFT_ARROW_MULTI_WIDE_SHADE_PATH,
                        PathFill::DarkenLess,
                        false,
                    ),
                    (
                        shared::CURVED_LEFT_ARROW_MULTI_WIDE_OUTLINE_PATH,
                        PathFill::None,
                        true,
                    ),
                ],
                w,
                h,
            ))
        }
        "curvedUpArrow"
            if shared::matches_curved_arrow_profile(
                adjust_values,
                12_000.0,
                70_000.0,
                18_000.0,
            ) =>
        {
            Some(shared::curved_arrow_multi_svg(
                [
                    (
                        shared::CURVED_UP_ARROW_MULTI_TIGHT_MAIN_PATH,
                        PathFill::Norm,
                        false,
                    ),
                    (
                        shared::CURVED_UP_ARROW_MULTI_TIGHT_SHADE_PATH,
                        PathFill::DarkenLess,
                        false,
                    ),
                    (
                        shared::CURVED_UP_ARROW_MULTI_TIGHT_OUTLINE_PATH,
                        PathFill::None,
                        true,
                    ),
                ],
                w,
                h,
            ))
        }
        "curvedUpArrow"
            if shared::matches_curved_arrow_profile(
                adjust_values,
                42_000.0,
                30_000.0,
                42_000.0,
            ) =>
        {
            Some(shared::curved_arrow_multi_svg(
                [
                    (
                        shared::CURVED_UP_ARROW_MULTI_WIDE_MAIN_PATH,
                        PathFill::Norm,
                        false,
                    ),
                    (
                        shared::CURVED_UP_ARROW_MULTI_WIDE_SHADE_PATH,
                        PathFill::DarkenLess,
                        false,
                    ),
                    (
                        shared::CURVED_UP_ARROW_MULTI_WIDE_OUTLINE_PATH,
                        PathFill::None,
                        true,
                    ),
                ],
                w,
                h,
            ))
        }
        "curvedDownArrow"
            if shared::matches_curved_arrow_profile(
                adjust_values,
                12_000.0,
                70_000.0,
                18_000.0,
            ) =>
        {
            Some(shared::curved_arrow_multi_svg(
                [
                    (
                        shared::CURVED_DOWN_ARROW_MULTI_TIGHT_MAIN_PATH,
                        PathFill::Norm,
                        false,
                    ),
                    (
                        shared::CURVED_DOWN_ARROW_MULTI_TIGHT_SHADE_PATH,
                        PathFill::DarkenLess,
                        false,
                    ),
                    (
                        shared::CURVED_DOWN_ARROW_MULTI_TIGHT_OUTLINE_PATH,
                        PathFill::None,
                        true,
                    ),
                ],
                w,
                h,
            ))
        }
        "curvedDownArrow"
            if shared::matches_curved_arrow_profile(
                adjust_values,
                42_000.0,
                30_000.0,
                42_000.0,
            ) =>
        {
            Some(shared::curved_arrow_multi_svg(
                [
                    (
                        shared::CURVED_DOWN_ARROW_MULTI_WIDE_MAIN_PATH,
                        PathFill::Norm,
                        false,
                    ),
                    (
                        shared::CURVED_DOWN_ARROW_MULTI_WIDE_SHADE_PATH,
                        PathFill::DarkenLess,
                        false,
                    ),
                    (
                        shared::CURVED_DOWN_ARROW_MULTI_WIDE_OUTLINE_PATH,
                        PathFill::None,
                        true,
                    ),
                ],
                w,
                h,
            ))
        }
        _ => None,
    }
}
