use std::io::Cursor;

use quick_xml::events::BytesStart;
use zip::ZipArchive;

use super::preserved_parser::part_diagnostic;
use super::preserved_parser::{read_text_entry, slide_index_from_part};
use super::relationships;
use crate::error::PptxResult;
use crate::model::{
    ConversionDiagnostic, DiagnosticLocation, FallbackKind, FeatureFamily, SupportTier,
};

pub(crate) fn collect_part_diagnostics(
    part_name: &str,
    diagnostics: &mut Vec<ConversionDiagnostic>,
) {
    if part_name.starts_with("ppt/embeddings/") {
        diagnostics.push(part_diagnostic(
            part_name,
            FeatureFamily::Unsupported,
            "Embedded package content is preserved but not rendered",
        ));
    }
}

pub(crate) fn safe_external_target(target: &str) -> String {
    let without_fragment = target.split('#').next().unwrap_or("");
    let without_query = without_fragment.split('?').next().unwrap_or("");
    let Some((scheme, remainder)) = without_query.split_once("://") else {
        return "external-target".to_owned();
    };
    let (authority, path) = remainder
        .split_once('/')
        .map_or((remainder, ""), |(authority, path)| (authority, path));
    let safe_authority = authority.rsplit('@').next().unwrap_or("");
    if path.is_empty() {
        format!("{scheme}://{safe_authority}")
    } else {
        format!("{scheme}://{safe_authority}/{path}")
    }
}

pub(crate) fn collect_relationship_diagnostics(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
    diagnostics: &mut Vec<ConversionDiagnostic>,
) -> PptxResult<()> {
    let xml = read_text_entry(archive, name)?;
    let source_part = relationship_source_part(name);
    for relationship in relationships::parse_relationship_records(&xml)? {
        if known_relationship_type(&relationship.relationship_type) {
            continue;
        }
        let raw_reference = match relationship.target_mode {
            relationships::TargetMode::External => safe_external_target(&relationship.target),
            relationships::TargetMode::Internal | relationships::TargetMode::Other(_) => {
                relationship.target
            }
        };
        diagnostics.push(ConversionDiagnostic {
            code: "OOXML_RELATIONSHIP_UNSUPPORTED".to_owned(),
            family: FeatureFamily::Unsupported,
            support_tier: SupportTier::Unparsed,
            stage: None,
            location: DiagnosticLocation {
                slide_index: slide_index_from_part(&source_part),
                part_name: Some(source_part.clone()),
                relationship_id: Some(relationship.id),
                relationship_type: Some(relationship.relationship_type),
                ..Default::default()
            },
            raw_reference: Some(raw_reference),
            fallback_kind: FallbackKind::IgnoredRelationship,
            reason: "Relationship type is not supported; conversion continued without following it"
                .to_owned(),
        });
    }
    Ok(())
}

fn relationship_source_part(name: &str) -> String {
    let Some((directory, file)) = name.rsplit_once("/_rels/") else {
        return name.to_owned();
    };
    file.strip_suffix(".rels")
        .map(|file| format!("{directory}/{file}"))
        .unwrap_or_else(|| name.to_owned())
}

pub(crate) fn attribute_value(element: &BytesStart<'_>, local_name: &str) -> Option<String> {
    element.attributes().flatten().find_map(|attribute| {
        let name = String::from_utf8_lossy(attribute.key.as_ref());
        (name == local_name || name.ends_with(&format!(":{local_name}")))
            .then(|| String::from_utf8_lossy(&attribute.value).into_owned())
    })
}

pub(crate) fn known_relationship_type(value: &str) -> bool {
    matches!(
        value.rsplit('/').next(),
        Some(
            "officeDocument"
                | "slide"
                | "slideMaster"
                | "slideLayout"
                | "theme"
                | "image"
                | "chart"
                | "hyperlink"
                | "notesSlide"
                | "comments"
                | "oleObject"
                | "package"
                | "audio"
                | "video"
                | "media"
                | "diagramData"
                | "diagramLayout"
                | "diagramStyle"
                | "diagramColors"
                | "core-properties"
                | "extended-properties"
        )
    )
}
