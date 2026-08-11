use quick_xml::Reader;
use quick_xml::events::BytesStart;

use super::official_presets_path::PathDefinition;
use crate::model::PathFill;

pub(super) fn path_definition(
    reader: &Reader<&[u8]>,
    element: &BytesStart<'_>,
) -> Result<PathDefinition, String> {
    let fill = match optional_attribute(reader, element, "fill")?.as_deref() {
        None | Some("norm") => PathFill::Norm,
        Some("none") => PathFill::None,
        Some("lighten") => PathFill::Lighten,
        Some("lightenLess") => PathFill::LightenLess,
        Some("darken") => PathFill::Darken,
        Some("darkenLess") => PathFill::DarkenLess,
        Some(value) => return Err(format!("unknown path fill: {value}")),
    };
    Ok(PathDefinition {
        width: optional_attribute(reader, element, "w")?,
        height: optional_attribute(reader, element, "h")?,
        fill,
        stroke: optional_attribute(reader, element, "stroke")?.as_deref() != Some("false"),
        commands: Vec::new(),
    })
}

pub(super) fn attribute(
    reader: &Reader<&[u8]>,
    element: &BytesStart<'_>,
    name: &str,
) -> Result<String, String> {
    optional_attribute(reader, element, name)?.ok_or_else(|| format!("missing {name}"))
}

pub(super) fn validate_attributes(
    reader: &Reader<&[u8]>,
    element: &BytesStart<'_>,
    allowed: &[&str],
) -> Result<(), String> {
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| error.to_string())?;
        let name = String::from_utf8_lossy(attribute.key.as_ref());
        if !allowed.iter().any(|allowed| *allowed == name) {
            return Err(format!(
                "unknown {tag} attribute: {name}",
                tag = local_name(element)
            ));
        }
        attribute
            .decode_and_unescape_value(reader.decoder())
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn optional_attribute(
    reader: &Reader<&[u8]>,
    element: &BytesStart<'_>,
    name: &str,
) -> Result<Option<String>, String> {
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| error.to_string())?;
        if attribute.key.as_ref() == name.as_bytes() {
            return attribute
                .decode_and_unescape_value(reader.decoder())
                .map(|value| Some(value.into_owned()))
                .map_err(|error| error.to_string());
        }
    }
    Ok(None)
}

pub(super) fn local_name(element: &BytesStart<'_>) -> String {
    String::from_utf8_lossy(element.local_name().as_ref()).into_owned()
}
