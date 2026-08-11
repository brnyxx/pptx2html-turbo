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
        let raw_reference = format!("{source_part}#{}", relationship.id);
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
    if name == "_rels/.rels" {
        return "/".to_owned();
    }
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
    const OFFICE_RELATIONSHIP_NAMESPACE: &str =
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/";
    const CORE_PROPERTIES_RELATIONSHIP: &str =
        "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties";

    let Some(kind) = value.strip_prefix(OFFICE_RELATIONSHIP_NAMESPACE) else {
        return value == CORE_PROPERTIES_RELATIONSHIP;
    };
    matches!(
        kind,
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
            | "extended-properties"
    )
}

#[cfg(test)]
mod tests {
    use super::relationship_source_part;

    #[test]
    fn root_relationship_source_is_the_package_root() {
        assert_eq!(relationship_source_part("_rels/.rels"), "/");
    }
}
