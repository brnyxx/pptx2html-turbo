use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

use super::table_style_values::{handle_empty, handle_start};
use super::xml_utils;
use crate::error::{PptxError, PptxResult};
use crate::model::{TableCellStyle, TableStyle, TableStyleRegion};

pub(crate) fn parse_table_styles(xml: &str) -> PptxResult<Vec<TableStyle>> {
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut styles = Vec::new();
    let mut style: Option<TableStyle> = None;
    let mut region: Option<TableStyleRegion> = None;
    let mut cell_style = TableCellStyle::default();
    let mut border_side: Option<String> = None;
    let mut in_fill = false;
    let mut in_text_style = false;

    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((namespace, Event::Start(element))) if drawingml(&namespace) => {
                let local = xml_utils::local_name(element.name().as_ref()).to_owned();
                if local == "tblStyle" {
                    style = Some(TableStyle {
                        id: xml_utils::attr_str(&element, "styleId").unwrap_or_default(),
                        name: xml_utils::attr_str(&element, "styleName"),
                        ..Default::default()
                    });
                } else if local == "tblBg" {
                    cell_style = TableCellStyle::default();
                } else if let Some(parsed) = TableStyleRegion::from_ooxml(&local) {
                    region = Some(parsed);
                    cell_style = TableCellStyle::default();
                } else {
                    if matches!(local.as_str(), "gradFill" | "blipFill" | "pattFill")
                        && let Some(style) = style.as_mut()
                    {
                        style.unsupported_primitives.push(local.clone());
                    }
                    handle_start(
                        &local,
                        &element,
                        &mut cell_style,
                        &mut border_side,
                        &mut in_fill,
                        &mut in_text_style,
                    );
                }
            }
            Ok((namespace, Event::Empty(element))) if drawingml(&namespace) => {
                let local = xml_utils::local_name(element.name().as_ref()).to_owned();
                if matches!(local.as_str(), "gradFill" | "blipFill" | "pattFill")
                    && let Some(style) = style.as_mut()
                {
                    style.unsupported_primitives.push(local.clone());
                }
                handle_empty(
                    &local,
                    &element,
                    &mut cell_style,
                    border_side.as_deref(),
                    in_fill,
                    in_text_style,
                );
            }
            Ok((namespace, Event::End(element))) if drawingml(&namespace) => {
                let name = element.name();
                let local = xml_utils::local_name(name.as_ref());
                if local == "tblBg" {
                    if let Some(style) = style.as_mut() {
                        style.table_background = cell_style.fill.take();
                    }
                } else if let Some(parsed) = TableStyleRegion::from_ooxml(local) {
                    if region == Some(parsed)
                        && let Some(style) = style.as_mut()
                    {
                        style
                            .regions
                            .push((parsed, std::mem::take(&mut cell_style)));
                    }
                    region = None;
                } else {
                    match local {
                        "tblStyle" => {
                            if let Some(completed) = style.take() {
                                styles.push(completed);
                            }
                        }
                        "fill" | "fillRef" => in_fill = false,
                        "tcTxStyle" => in_text_style = false,
                        "left" | "right" | "top" | "bottom" | "insideH" | "insideV" => {
                            border_side = None;
                        }
                        _ => {}
                    }
                }
            }
            Ok((_, Event::Eof)) => break,
            Err(error) => return Err(PptxError::Xml(error)),
            _ => {}
        }
        buffer.clear();
    }
    Ok(styles)
}

fn drawingml(namespace: &ResolveResult<'_>) -> bool {
    matches!(namespace, ResolveResult::Bound(uri) if uri.as_ref() == b"http://schemas.openxmlformats.org/drawingml/2006/main")
}
