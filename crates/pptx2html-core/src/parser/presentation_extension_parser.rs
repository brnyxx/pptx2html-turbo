use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;

use crate::model::{
    CapabilityStage, ConversionDiagnostic, DiagnosticLocation, FallbackKind, FeatureFamily,
    PresentationExtensionMetadata, SupportTier,
};

const PML: &[u8] = b"http://schemas.openxmlformats.org/presentationml/2006/main";
const RAW_LIMIT: usize = 16 * 1024;

pub(super) fn parse(xml: &str) -> Vec<PresentationExtensionMetadata> {
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();
    let mut stack = Vec::new();
    let mut extensions = Vec::new();
    let mut capture: Option<(usize, usize, String)> = None;
    loop {
        let event_start = reader.buffer_position() as usize;
        match reader.read_resolved_event_into(&mut buffer) {
            Ok((namespace, Event::Start(element))) => {
                let node = (
                    bound(&namespace, PML),
                    String::from_utf8_lossy(element.local_name().as_ref()).into_owned(),
                );
                if node == (true, "ext".to_owned())
                    && stack.as_slice()
                        == [
                            (true, "presentation".to_owned()),
                            (true, "extLst".to_owned()),
                        ]
                {
                    let uri = attribute(&element, "uri").unwrap_or_default();
                    capture = Some((stack.len(), event_start, uri));
                }
                stack.push(node);
            }
            Ok((namespace, Event::Empty(element))) => {
                if bound(&namespace, PML)
                    && element.local_name().as_ref() == b"ext"
                    && stack.as_slice()
                        == [
                            (true, "presentation".to_owned()),
                            (true, "extLst".to_owned()),
                        ]
                {
                    let uri = attribute(&element, "uri").unwrap_or_default();
                    let end = reader.buffer_position() as usize;
                    extensions.push(PresentationExtensionMetadata {
                        uri,
                        raw_xml: bounded(&xml[event_start..end]),
                    });
                }
            }
            Ok((namespace, Event::End(element))) => {
                if bound(&namespace, PML)
                    && element.local_name().as_ref() == b"ext"
                    && let Some((depth, start, uri)) = capture.take()
                    && stack.len() == depth + 1
                {
                    let end = reader.buffer_position() as usize;
                    extensions.push(PresentationExtensionMetadata {
                        uri,
                        raw_xml: bounded(&xml[start..end]),
                    });
                }
                stack.pop();
            }
            Ok((_, Event::Eof)) | Err(_) => break,
            _ => {}
        }
        buffer.clear();
    }
    extensions
}

pub(crate) fn diagnostics(
    extensions: &[PresentationExtensionMetadata],
) -> Vec<ConversionDiagnostic> {
    extensions
        .iter()
        .map(|extension| ConversionDiagnostic {
            code: "PRESENTATION_EXTENSION_METADATA".to_owned(),
            family: FeatureFamily::Unsupported,
            support_tier: SupportTier::Fallback,
            stage: Some(CapabilityStage::Parsed),
            location: DiagnosticLocation {
                part_name: Some("ppt/presentation.xml".to_owned()),
                qualified_element_name: Some("p:ext".to_owned()),
                ..Default::default()
            },
            raw_reference: Some(format!(
                "uri={}\nraw_xml={}",
                extension.uri, extension.raw_xml
            )),
            fallback_kind: FallbackKind::PreservedPart,
            reason: "Presentation extension payload was preserved as bounded metadata".to_owned(),
        })
        .collect()
}

fn bound(namespace: &ResolveResult<'_>, expected: &[u8]) -> bool {
    matches!(namespace, ResolveResult::Bound(value) if value.as_ref() == expected)
}

fn attribute(element: &quick_xml::events::BytesStart<'_>, name: &str) -> Option<String> {
    element
        .attributes()
        .flatten()
        .find(|attribute| attribute.key.as_ref() == name.as_bytes())
        .and_then(|attribute| attribute.unescape_value().ok())
        .map(|value| value.into_owned())
}

fn bounded(value: &str) -> String {
    if value.len() <= RAW_LIMIT {
        return value.to_owned();
    }
    let mut end = RAW_LIMIT;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}
