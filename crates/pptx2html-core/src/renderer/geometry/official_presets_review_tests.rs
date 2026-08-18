use std::collections::HashMap;

use super::official_presets::{OfficialPresetRender, route_with_xml};
use super::official_presets_xml::source_xml;

fn assert_invalid(render: OfficialPresetRender) {
    let OfficialPresetRender::Invalid(svg) = render else {
        panic!("official fault must return Invalid")
    };
    assert_eq!(
        svg.paths[0].d,
        "M0.00,0.00 L160.00,0.00 L160.00,100.00 L0.00,100.00 Z"
    );
}

fn assert_rendered(render: OfficialPresetRender) {
    assert!(matches!(render, OfficialPresetRender::Rendered(_)));
}

fn render_source(source: &str) -> OfficialPresetRender {
    route_with_xml("rightArrow", 160.0, 100.0, &HashMap::new(), source)
}

#[test]
fn overflowing_official_adjustment_is_invalid_not_a_zero_anchor() {
    let adjustments = HashMap::from([("adj1".to_owned(), f64::MAX)]);
    assert_invalid(route_with_xml(
        "wedgeRoundRectCallout",
        160.0,
        100.0,
        &adjustments,
        source_xml(),
    ));
}

#[test]
fn non_whitespace_xml_text_is_invalid() {
    let malformed = source_xml().replacen("<rightArrow>", "<rightArrow>unexpected", 1);
    assert_invalid(render_source(&malformed));
}

#[test]
fn xml_cdata_is_invalid() {
    let malformed = source_xml().replacen("<rightArrow>", "<rightArrow><![CDATA[x]]>", 1);
    assert_invalid(render_source(&malformed));
}

#[test]
fn xml_processing_instruction_is_invalid() {
    let malformed = source_xml().replacen("<rightArrow>", "<rightArrow><?task x?>", 1);
    assert_invalid(render_source(&malformed));
}

#[test]
fn xml_doctype_is_invalid() {
    let malformed = source_xml().replacen(
        "<presetShapeDefinitions>",
        "<!DOCTYPE presetShapeDefinitions><presetShapeDefinitions>",
        1,
    );
    assert_invalid(render_source(&malformed));
}

#[test]
fn xml_declaration_after_whitespace_text_is_invalid() {
    let malformed = format!(" \n{}", source_xml());
    assert_invalid(render_source(&malformed));
}

#[test]
fn xml_declaration_after_comment_is_invalid() {
    let malformed = format!("<!--before declaration-->{}", source_xml());
    assert_invalid(render_source(&malformed));
}

#[test]
fn repeated_xml_declaration_is_invalid() {
    let malformed = source_xml().replacen("?>", "?><?xml version=\"1.0\" encoding=\"utf-8\"?>", 1);
    assert_invalid(render_source(&malformed));
}

#[test]
fn exactly_leading_xml_declaration_is_rendered() {
    assert_rendered(render_source(source_xml()));
}

#[test]
fn source_without_xml_declaration_is_rendered() {
    let source = source_xml().replacen("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n", "", 1);
    assert_rendered(render_source(&source));
}

#[test]
fn normal_whitespace_and_comments_elsewhere_are_rendered() {
    let source = source_xml().replacen(
        "<rightArrow>",
        "\n  <!--permitted body comment-->\n  <rightArrow>",
        1,
    );
    assert_rendered(render_source(&source));
}

#[test]
fn ellipse_ribbons_use_official_multi_path_geometry() {
    const PRESETS: &[&str] = &["ellipseRibbon", "ellipseRibbon2"];

    super::official_ellipse_ribbon_presets::definitions()
        .expect("official ellipse ribbon preset XML should parse");
    for preset in PRESETS {
        assert!(
            super::official_ellipse_ribbon_presets::contains(preset),
            "missing official ellipse ribbon preset: {preset}"
        );
        assert!(
            matches!(
                super::official_presets::route(preset, 160.0, 100.0, &HashMap::new()),
                OfficialPresetRender::Rendered(_)
            ),
            "official ellipse ribbon preset did not render: {preset}"
        );
    }
}

#[test]
fn curved_right_arrow_arc_reaches_official_guide_endpoint() {
    let OfficialPresetRender::Rendered(svg) =
        super::official_presets::route("curvedRightArrow", 235.2, 103.68, &HashMap::new())
    else {
        panic!("default curvedRightArrow should render")
    };

    assert!(
        svg.paths[0]
            .d
            .contains("A235.20,32.40 0 0,0 209.28,64.60 L209.28,51.64"),
        "{}",
        svg.paths[0].d
    );
}
