use std::io::Cursor;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use zip::ZipArchive;

use super::graphic_frame_parser::{read_archive_entry, relationships_path};
use super::relationships::{Relationship, TargetMode, parse_relationship_records};
use super::xml_utils;
use crate::error::{PptxError, PptxResult};
use crate::model::{
    CapabilityStage, ConversionDiagnostic, DiagnosticLocation, FallbackKind, FeatureFamily,
    SupportTier,
};

pub(crate) fn collect(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    part_name: &str,
    xml: &str,
    diagnostics: &mut Vec<ConversionDiagnostic>,
) -> PptxResult<()> {
    let Some(owner) = inheritance_owner(part_name) else {
        return Ok(());
    };
    let relationships = read_archive_entry(archive, &relationships_path(part_name))
        .ok()
        .and_then(|xml| parse_relationship_records(&xml).ok())
        .unwrap_or_default();
    let mut reader = Reader::from_str(xml);
    let mut in_picture_bullet = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(element)) => {
                let element_name = element.name();
                let local = xml_utils::local_name(element_name.as_ref());
                if local == "buBlip" {
                    in_picture_bullet = true;
                } else if local == "blip" && in_picture_bullet {
                    push(part_name, owner, &element, &relationships, diagnostics);
                }
            }
            Ok(Event::Empty(element))
                if xml_utils::local_name(element.name().as_ref()) == "blip"
                    && in_picture_bullet =>
            {
                push(part_name, owner, &element, &relationships, diagnostics);
            }
            Ok(Event::End(element))
                if xml_utils::local_name(element.name().as_ref()) == "buBlip" =>
            {
                in_picture_bullet = false;
            }
            Ok(Event::Eof) => return Ok(()),
            Err(error) => return Err(PptxError::Xml(error)),
            _ => {}
        }
    }
}

fn inheritance_owner(part_name: &str) -> Option<&'static str> {
    if part_name == "ppt/presentation.xml" {
        Some("presentation defaultTextStyle")
    } else if part_name.starts_with("ppt/slideMasters/") {
        Some("slide master text style")
    } else if part_name.starts_with("ppt/slideLayouts/") {
        Some("slide layout text style")
    } else {
        None
    }
}

fn push(
    part_name: &str,
    owner: &str,
    element: &BytesStart<'_>,
    relationships: &[Relationship],
    diagnostics: &mut Vec<ConversionDiagnostic>,
) {
    let relationship_id =
        xml_utils::attr_str(element, "embed").or_else(|| xml_utils::attr_str(element, "link"));
    let relationship = relationship_id.as_deref().and_then(|id| {
        relationships
            .iter()
            .find(|relationship| relationship.id == id)
    });
    let actual_mode = relationship
        .map(|relationship| target_mode_name(&relationship.target_mode))
        .unwrap_or("unavailable");
    diagnostics.push(ConversionDiagnostic {
        code: "PICTURE_BULLET_INHERITANCE_UNSUPPORTED".to_owned(),
        family: FeatureFamily::Images,
        support_tier: SupportTier::Fallback,
        stage: Some(CapabilityStage::Parsed),
        location: DiagnosticLocation {
            part_name: Some(part_name.to_owned()),
            relationship_id: relationship_id.clone(),
            relationship_type: relationship.map(|relationship| relationship.relationship_type.clone()),
            qualified_element_name: Some("a:buBlip".to_owned()),
            ..Default::default()
        },
        raw_reference: relationship_id,
        fallback_kind: FallbackKind::IgnoredRelationship,
        reason: format!(
            "Picture bullet inheritance from {owner} is unsupported; relationship mode: {actual_mode}"
        ),
    });
}

fn target_mode_name(mode: &TargetMode) -> &str {
    match mode {
        TargetMode::Internal => "Internal",
        TargetMode::External => "External",
        TargetMode::Other(value) => value,
    }
}
