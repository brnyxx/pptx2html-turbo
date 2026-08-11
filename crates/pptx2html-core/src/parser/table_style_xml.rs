use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

use super::xml_utils;
use crate::error::{PptxError, PptxResult};

const DRAWINGML: &[u8] = b"http://schemas.openxmlformats.org/drawingml/2006/main";

pub(super) fn validate(xml: &str) -> PptxResult<()> {
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut stack = Vec::<String>::new();
    let mut roots = 0usize;
    loop {
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((namespace, Event::Start(element))) => {
                require_drawingml(&namespace)?;
                let name = element.name();
                let local = xml_utils::local_name(name.as_ref());
                validate_context(local, stack.last().map(String::as_str))?;
                if stack.is_empty() {
                    roots += 1;
                }
                stack.push(local.to_owned());
            }
            Ok((namespace, Event::Empty(element))) => {
                require_drawingml(&namespace)?;
                let name = element.name();
                let local = xml_utils::local_name(name.as_ref());
                validate_context(local, stack.last().map(String::as_str))?;
                if stack.is_empty() {
                    roots += 1;
                }
            }
            Ok((namespace, Event::End(element))) => {
                require_drawingml(&namespace)?;
                let name = element.name();
                let local = xml_utils::local_name(name.as_ref());
                if stack.pop().as_deref() != Some(local) {
                    return invalid();
                }
            }
            Ok((_, Event::Text(text))) => {
                if !text.as_ref().iter().all(u8::is_ascii_whitespace) {
                    return invalid();
                }
            }
            Ok((_, Event::CData(_) | Event::PI(_) | Event::DocType(_))) => return invalid(),
            Ok((_, Event::Decl(_) | Event::Comment(_))) => {}
            Ok((_, Event::Eof)) => break,
            Err(error) => return Err(PptxError::Xml(error)),
        }
        buffer.clear();
    }
    if roots != 1 || !stack.is_empty() {
        return invalid();
    }
    Ok(())
}

fn require_drawingml(namespace: &ResolveResult<'_>) -> PptxResult<()> {
    if matches!(namespace, ResolveResult::Bound(uri) if uri.as_ref() == DRAWINGML) {
        Ok(())
    } else {
        invalid()
    }
}

fn validate_context(local: &str, parent: Option<&str>) -> PptxResult<()> {
    let valid = match local {
        "tblStyleLst" => parent.is_none(),
        "tblStyle" => parent == Some("tblStyleLst"),
        "tblBg" => parent == Some("tblStyle"),
        "wholeTbl" | "band1H" | "band2H" | "band1V" | "band2V" | "lastCol" | "firstCol"
        | "lastRow" | "seCell" | "swCell" | "firstRow" | "neCell" | "nwCell" => {
            parent == Some("tblStyle")
        }
        "tcStyle" | "tcTxStyle" => parent.is_some_and(is_region),
        "fill" => matches!(parent, Some("tcStyle" | "tblBg")),
        "fillRef" => matches!(parent, Some("tcStyle" | "tblBg")),
        "tcBdr" => parent == Some("tcStyle"),
        "left" | "right" | "top" | "bottom" | "insideH" | "insideV" => parent == Some("tcBdr"),
        "ln" => matches!(
            parent,
            Some("left" | "right" | "top" | "bottom" | "insideH" | "insideV")
        ),
        "solidFill" | "noFill" | "gradFill" | "blipFill" | "pattFill" => {
            matches!(parent, Some("fill" | "ln"))
        }
        "srgbClr" | "schemeClr" | "sysClr" | "prstClr" => matches!(
            parent,
            Some("solidFill" | "ln" | "fillRef" | "tcTxStyle" | "fontRef" | "gs")
        ),
        "alpha" | "alphaMod" | "alphaOff" | "blue" | "blueMod" | "blueOff" | "gamma" | "gray"
        | "green" | "greenMod" | "greenOff" | "hue" | "hueMod" | "hueOff" | "inv" | "invGamma"
        | "lum" | "lumMod" | "lumOff" | "red" | "redMod" | "redOff" | "sat" | "satMod"
        | "satOff" | "shade" | "tint" => {
            matches!(parent, Some("srgbClr" | "schemeClr" | "sysClr" | "prstClr"))
        }
        "fontRef" => parent == Some("tcTxStyle"),
        "gsLst" => parent == Some("gradFill"),
        "gs" => parent == Some("gsLst"),
        "lin" => parent == Some("gradFill"),
        _ => false,
    };
    if valid { Ok(()) } else { invalid() }
}

fn is_region(parent: &str) -> bool {
    matches!(
        parent,
        "wholeTbl"
            | "band1H"
            | "band2H"
            | "band1V"
            | "band2V"
            | "lastCol"
            | "firstCol"
            | "lastRow"
            | "seCell"
            | "swCell"
            | "firstRow"
            | "neCell"
            | "nwCell"
    )
}

fn invalid<T>() -> PptxResult<T> {
    Err(PptxError::UnsupportedFormat(
        "invalid DrawingML table styles XML".to_owned(),
    ))
}
