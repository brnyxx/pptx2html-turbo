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
