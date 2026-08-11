use log::warn;
use quick_xml::events::{BytesEnd, BytesStart};

use super::slide_parser::ShapeBuilder;
use crate::model::slide::{UnresolvedType, UnsupportedData};

pub(crate) struct UnsupportedGraphic {
    pub(crate) label: &'static str,
    pub(crate) element_type: UnresolvedType,
}

pub(crate) fn classify_unsupported_graphic(uri: &str) -> Option<UnsupportedGraphic> {
    if uri.contains("diagram") || uri.contains("dgm") {
        warn!("SmartArt diagram detected — rendering placeholder");
        Some(UnsupportedGraphic {
            label: "SmartArt",
            element_type: UnresolvedType::SmartArt,
        })
    } else if uri.contains("oleObject") {
        warn!("OLE object detected — rendering placeholder");
        Some(UnsupportedGraphic {
            label: "OLE Object",
            element_type: UnresolvedType::OleObject,
        })
    } else if uri.contains("math") || uri.contains("omml") {
        warn!("Math equation detected — rendering placeholder");
        Some(UnsupportedGraphic {
            label: "Math Equation",
            element_type: UnresolvedType::MathEquation,
        })
    } else {
        None
    }
}

pub(crate) fn unsupported_data(
    label: String,
    element_type: Option<UnresolvedType>,
    raw_xml: Option<String>,
) -> UnsupportedData {
    UnsupportedData {
        label,
        element_type: element_type.unwrap_or(UnresolvedType::SmartArt),
        raw_xml,
    }
}

pub(crate) fn finish_raw_capture(shape: &mut Option<ShapeBuilder>, raw_xml: &mut String) {
    if let Some(shape) = shape.as_mut()
        && !raw_xml.is_empty()
    {
        shape.raw_xml_capture = Some(raw_xml.clone());
    }
    raw_xml.clear();
}

pub(crate) fn append_start_element(
    element: &BytesStart<'_>,
    fallback_name: &str,
    output: &mut String,
) {
    output.push('<');
    append_element_name(element.name().as_ref(), fallback_name, output);
    append_attributes(element, output);
    output.push('>');
}

pub(crate) fn append_empty_element(
    element: &BytesStart<'_>,
    fallback_name: &str,
    output: &mut String,
) {
    output.push('<');
    append_element_name(element.name().as_ref(), fallback_name, output);
    append_attributes(element, output);
    output.push_str("/>");
}

pub(crate) fn append_end_element(element: &BytesEnd<'_>, fallback_name: &str, output: &mut String) {
    output.push_str("</");
    append_element_name(element.name().as_ref(), fallback_name, output);
    output.push('>');
}

fn append_element_name(name: &[u8], fallback_name: &str, output: &mut String) {
    output.push_str(std::str::from_utf8(name).unwrap_or(fallback_name));
}

fn append_attributes(element: &BytesStart<'_>, output: &mut String) {
    for attribute in element.attributes().flatten() {
        let key = std::str::from_utf8(attribute.key.as_ref()).unwrap_or("");
        let value = std::str::from_utf8(&attribute.value).unwrap_or("");
        output.push(' ');
        output.push_str(key);
        output.push_str("=\"");
        output.push_str(value);
        output.push('"');
    }
}
