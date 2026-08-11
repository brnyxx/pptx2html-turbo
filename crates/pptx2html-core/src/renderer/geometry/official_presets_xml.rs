use std::collections::HashMap;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

use super::official_presets_path::{PathCommandDefinition, PathDefinition, PointDefinition};
use super::official_presets_schema::{attribute, local_name, path_definition, validate_attributes};

const XML: &str = include_str!("official_arrow_presets.xml");

#[derive(Debug)]
pub(super) struct PresetDefinition {
    pub(super) adjustments: Vec<GuideDefinition>,
    pub(super) guides: Vec<GuideDefinition>,
    pub(super) paths: Vec<PathDefinition>,
}

#[derive(Debug)]
pub(super) struct GuideDefinition {
    pub(super) name: String,
    pub(super) formula: String,
}

pub(super) fn parse_definitions() -> Result<HashMap<String, PresetDefinition>, String> {
    parse_definitions_from(XML)
}

pub(super) fn parse_definitions_from(
    source: &str,
) -> Result<HashMap<String, PresetDefinition>, String> {
    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(true);
    let mut definitions = HashMap::new();
    let mut stack = Vec::new();
    let mut preset: Option<(String, PresetDefinition)> = None;
    let mut path: Option<PathDefinition> = None;
    let mut command: Option<(String, Vec<PointDefinition>)> = None;
    let mut declaration_seen = false;
    loop {
        match reader.read_event().map_err(|error| error.to_string())? {
            Event::Start(element) => {
                let tag = local_name(&element);
                handle_start(
                    &reader,
                    &element,
                    &tag,
                    &stack,
                    &mut preset,
                    &mut path,
                    &mut command,
                )?;
                stack.push(tag);
            }
            Event::Empty(element) => {
                let tag = local_name(&element);
                handle_empty(
                    &reader,
                    &element,
                    &tag,
                    &stack,
                    &mut preset,
                    &mut path,
                    &mut command,
                )?;
            }
            Event::End(element) => {
                let tag = String::from_utf8_lossy(element.local_name().as_ref()).into_owned();
                handle_end(&tag, &mut definitions, &mut preset, &mut path, &mut command)?;
                let actual = stack.pop().ok_or("unexpected XML end")?;
                if actual != tag {
                    return Err(format!("mismatched XML end: {actual}/{tag}"));
                }
            }
            Event::Text(text) => {
                let text = text.unescape().map_err(|error| error.to_string())?;
                if !text.trim().is_empty() {
                    return Err("unexpected official XML text".to_owned());
                }
            }
            Event::Comment(_) => {}
            Event::Decl(_) if stack.is_empty() && !declaration_seen => declaration_seen = true,
            Event::Decl(_) => return Err("unexpected official XML declaration".to_owned()),
            Event::CData(_) => return Err("unexpected official XML CDATA".to_owned()),
            Event::PI(_) => {
                return Err("unexpected official XML processing instruction".to_owned());
            }
            Event::DocType(_) => return Err("unexpected official XML doctype".to_owned()),
            Event::Eof => break,
        }
    }
    if definitions.len() != 55 {
        return Err(format!(
            "expected 55 official arrow presets, got {}",
            definitions.len()
        ));
    }
    Ok(definitions)
}

fn handle_start(
    reader: &Reader<&[u8]>,
    element: &BytesStart<'_>,
    tag: &str,
    stack: &[String],
    preset: &mut Option<(String, PresetDefinition)>,
    path: &mut Option<PathDefinition>,
    command: &mut Option<(String, Vec<PointDefinition>)>,
) -> Result<(), String> {
    if stack
        .last()
        .is_some_and(|parent| parent == "presetShapeDefinitions")
    {
        validate_attributes(reader, element, &[])?;
        *preset = Some((tag.to_owned(), empty_preset()));
    } else {
        match tag {
            "presetShapeDefinitions" => validate_attributes(reader, element, &[])?,
            "avLst" | "gdLst" | "ahLst" | "cxnLst" | "pathLst" => {
                validate_attributes(reader, element, &["xmlns"])?
            }
            "ahXY" => validate_attributes(
                reader,
                element,
                &["xmlns", "gdRefX", "gdRefY", "minX", "maxX", "minY", "maxY"],
            )?,
            "ahPolar" => validate_attributes(
                reader,
                element,
                &[
                    "xmlns", "gdRefAng", "gdRefR", "minAng", "maxAng", "minR", "maxR",
                ],
            )?,
            "cxn" => validate_attributes(reader, element, &["xmlns", "ang"])?,
            "path" => {
                validate_attributes(
                    reader,
                    element,
                    &["xmlns", "w", "h", "fill", "stroke", "extrusionOk"],
                )?;
                *path = Some(path_definition(reader, element)?);
            }
            "moveTo" | "lnTo" | "cubicBezTo" | "quadBezTo" => {
                validate_attributes(reader, element, &["xmlns"])?;
                *command = Some((tag.to_owned(), Vec::new()));
            }
            _ => return Err(format!("unknown official XML element: {tag}")),
        }
    }
    Ok(())
}

fn handle_empty(
    reader: &Reader<&[u8]>,
    element: &BytesStart<'_>,
    tag: &str,
    stack: &[String],
    preset: &mut Option<(String, PresetDefinition)>,
    path: &mut Option<PathDefinition>,
    command: &mut Option<(String, Vec<PointDefinition>)>,
) -> Result<(), String> {
    match tag {
        "gd" => {
            validate_attributes(reader, element, &["xmlns", "name", "fmla"])?;
            let guide = GuideDefinition {
                name: attribute(reader, element, "name")?,
                formula: attribute(reader, element, "fmla")?,
            };
            let (_, definition) = preset.as_mut().ok_or("guide outside preset")?;
            if stack.iter().any(|entry| entry == "avLst") {
                definition.adjustments.push(guide);
            } else if stack.iter().any(|entry| entry == "gdLst") {
                definition.guides.push(guide);
            }
        }
        "pt" => {
            validate_attributes(reader, element, &["xmlns", "x", "y"])?;
            command
                .as_mut()
                .ok_or("point outside command")?
                .1
                .push(PointDefinition {
                    x: attribute(reader, element, "x")?,
                    y: attribute(reader, element, "y")?,
                })
        }
        "arcTo" => {
            validate_attributes(reader, element, &["xmlns", "wR", "hR", "stAng", "swAng"])?;
            path.as_mut()
                .ok_or("arc outside path")?
                .commands
                .push(PathCommandDefinition::Arc {
                    width_radius: attribute(reader, element, "wR")?,
                    height_radius: attribute(reader, element, "hR")?,
                    start_angle: attribute(reader, element, "stAng")?,
                    swing_angle: attribute(reader, element, "swAng")?,
                })
        }
        "close" => {
            validate_attributes(reader, element, &["xmlns"])?;
            path.as_mut()
                .ok_or("close outside path")?
                .commands
                .push(PathCommandDefinition::Close)
        }
        "pos" => validate_attributes(reader, element, &["xmlns", "x", "y"])?,
        "rect" => validate_attributes(reader, element, &["xmlns", "l", "t", "r", "b"])?,
        _ => return Err(format!("unknown empty official XML element: {tag}")),
    }
    Ok(())
}

fn handle_end(
    tag: &str,
    definitions: &mut HashMap<String, PresetDefinition>,
    preset: &mut Option<(String, PresetDefinition)>,
    path: &mut Option<PathDefinition>,
    command: &mut Option<(String, Vec<PointDefinition>)>,
) -> Result<(), String> {
    if matches!(tag, "moveTo" | "lnTo" | "cubicBezTo" | "quadBezTo") {
        let (kind, points) = command.take().ok_or("missing path command")?;
        let command = match kind.as_str() {
            "moveTo" => PathCommandDefinition::Move(points),
            "lnTo" => PathCommandDefinition::Line(points),
            "cubicBezTo" => PathCommandDefinition::Cubic(points),
            "quadBezTo" => PathCommandDefinition::Quad(points),
            _ => return Err(format!("unknown path command: {kind}")),
        };
        path.as_mut()
            .ok_or("command outside path")?
            .commands
            .push(command);
    } else if tag == "path" {
        preset
            .as_mut()
            .ok_or("path outside preset")?
            .1
            .paths
            .push(path.take().ok_or("missing path")?);
    } else if preset.as_ref().is_some_and(|(name, _)| name == tag) {
        let (name, definition) = preset.take().ok_or("missing preset")?;
        if definitions.insert(name.clone(), definition).is_some() {
            return Err(format!("duplicate official preset: {name}"));
        }
    }
    Ok(())
}

fn empty_preset() -> PresetDefinition {
    PresetDefinition {
        adjustments: Vec::new(),
        guides: Vec::new(),
        paths: Vec::new(),
    }
}

#[cfg(test)]
pub(super) fn source_xml() -> &'static str {
    XML
}
