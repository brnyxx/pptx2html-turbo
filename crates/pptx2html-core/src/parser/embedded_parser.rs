use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::io::{Cursor, Read, Seek};

use base64::Engine;
use quick_xml::Writer;
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;
use zip::ZipArchive;

use super::preserved_parser::part_diagnostic;
use super::preserved_parser::{read_text_entry, slide_index_from_part};
use super::relationships::{self, Relationship, TargetMode};
use crate::error::PptxResult;
use crate::model::embedded::{
    AlternateContentBranch, AlternateContentInventory, EmbeddedInventoryEntry,
    EmbeddedInventoryStore, EmbeddedPreview, EmbeddedRelationship, PREVIEW_BYTE_LIMIT,
    RAW_REFERENCE_LIMIT, inventory_key,
};
use crate::model::{
    ConversionDiagnostic, DiagnosticLocation, FallbackKind, FeatureFamily, Shape, ShapeType,
    SupportTier, UnresolvedType,
};

const OFFICE_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/";
const ACTIVEX_BINARY_REL: &str =
    "http://schemas.microsoft.com/office/2006/relationships/activeXControlBinary";
const MC: &[u8] = b"http://schemas.openxmlformats.org/markup-compatibility/2006";
const SUPPORTED_REQUIREMENT_NAMESPACES: &[&str] = &[
    "http://schemas.openxmlformats.org/presentationml/2006/main",
    "http://schemas.openxmlformats.org/drawingml/2006/main",
    "http://schemas.openxmlformats.org/drawingml/2006/diagram",
    "http://schemas.openxmlformats.org/officeDocument/2006/math",
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
];

pub(crate) struct UnknownPartInventory {
    part_names: BTreeSet<String>,
}

impl UnknownPartInventory {
    pub(crate) fn collect(names: &[String], diagnostics: &mut Vec<ConversionDiagnostic>) -> Self {
        const COUNT_LIMIT: usize = 128;
        const METADATA_LIMIT: usize = 32 * 1024;
        let mut part_names = BTreeSet::new();
        let mut emitted = 0usize;
        let mut metadata_bytes = 0usize;
        let mut omitted = 0usize;
        let mut truncated = false;
        for name in names {
            let Some(diagnostic) = unsupported_part_diagnostic(name) else {
                continue;
            };
            part_names.insert(name.clone());
            let entry_bytes = diagnostic
                .location
                .part_name
                .as_ref()
                .map_or(0, String::len)
                .saturating_add(diagnostic.raw_reference.as_ref().map_or(0, String::len));
            if !truncated
                && emitted < COUNT_LIMIT
                && metadata_bytes.saturating_add(entry_bytes) <= METADATA_LIMIT
            {
                emitted += 1;
                metadata_bytes += entry_bytes;
                diagnostics.push(diagnostic);
            } else {
                truncated = true;
                omitted += 1;
            }
        }
        if omitted != 0 {
            diagnostics.push(ConversionDiagnostic {
                code: "OOXML_PART_INVENTORY_TRUNCATED".to_owned(),
                family: FeatureFamily::Unsupported,
                support_tier: SupportTier::Unparsed,
                stage: None,
                location: DiagnosticLocation::default(),
                raw_reference: Some(format!("{{\"omitted_count\":{omitted}}}")),
                fallback_kind: FallbackKind::PreservedPart,
                reason: "Unknown package part inventory exceeded deterministic count or aggregate metadata limits".to_owned(),
            });
        }
        Self { part_names }
    }

    pub(crate) fn contains(&self, part_name: &str) -> bool {
        self.part_names.contains(part_name)
    }
}

fn unsupported_part_diagnostic(part_name: &str) -> Option<ConversionDiagnostic> {
    let reason = if part_name.starts_with("ppt/embeddings/") {
        "Embedded package content is preserved but never activated, executed, or emitted"
    } else if part_name.starts_with("ppt/extensions/") {
        "Unknown extension part is preserved and inventoried without exposing its bytes"
    } else if !known_package_part(part_name) {
        "Unknown package part is preserved and inventoried with bounded metadata without exposing its bytes"
    } else {
        return None;
    };
    Some(part_diagnostic(
        part_name,
        FeatureFamily::Unsupported,
        reason,
    ))
}

fn known_package_part(part_name: &str) -> bool {
    if matches!(part_name, "[Content_Types].xml" | "_rels/.rels")
        || part_name.ends_with(".rels")
        || matches!(
            part_name,
            "ppt/presentation.xml"
                | "ppt/presProps.xml"
                | "ppt/viewProps.xml"
                | "ppt/tableStyles.xml"
                | "ppt/commentAuthors.xml"
                | "ppt/additionalCharacteristics.xml"
        )
    {
        return true;
    }
    [
        "docProps/",
        "customXml/",
        "ppt/slides/",
        "ppt/slideLayouts/",
        "ppt/slideMasters/",
        "ppt/notesSlides/",
        "ppt/notesMasters/",
        "ppt/handoutMasters/",
        "ppt/slideUpdateInfo/",
        "ppt/theme/",
        "ppt/charts/",
        "ppt/diagrams/",
        "ppt/media/",
        "ppt/comments/",
        "ppt/bibliography/",
        "ppt/tags/",
        "ppt/customXml/",
    ]
    .iter()
    .any(|prefix| part_name.starts_with(prefix))
}

pub(crate) fn collect_relationship_diagnostics(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    name: &str,
    diagnostics: &mut Vec<ConversionDiagnostic>,
) -> PptxResult<()> {
    let xml = read_text_entry(archive, name)?;
    let source_part = relationship_source_part(name);
    let records = relationships::parse_relationship_records(&xml)?;
    for relationship in &records {
        if !known_relationship_type(&relationship.relationship_type) {
            diagnostics.push(relationship_diagnostic(
                "OOXML_RELATIONSHIP_UNSUPPORTED",
                &source_part,
                relationship,
                "Relationship type is not supported; its target was not followed or exposed",
            ));
            continue;
        }
        if let Some(kind) = embedded_relationship_kind(&relationship.relationship_type) {
            let resolved = matches!(relationship.target_mode, TargetMode::Internal)
                .then(|| resolve_target(&source_part, &relationship.target))
                .flatten();
            let valid = resolved
                .as_deref()
                .is_some_and(|part_name| valid_embedded_part(archive, kind, part_name));
            if !valid {
                diagnostics.push(relationship_diagnostic(
                    "OOXML_EMBEDDED_RELATIONSHIP_INVALID",
                    &source_part,
                    relationship,
                    "Embedded relationship must be internal, package-root bounded, present, and match its official content type and namespace",
                ));
                continue;
            }
            if kind == "package"
                && let Some(part_name) = resolved.as_deref()
                && let Some(diagnostic) =
                    embedded_package_diagnostic(archive, &source_part, part_name, relationship)
            {
                diagnostics.push(diagnostic);
            }
        }
        if relationship.relationship_type == format!("{OFFICE_REL}control")
            && let Some(diagnostic) =
                embedded_control_diagnostic(archive, &source_part, relationship)
        {
            diagnostics.push(diagnostic);
        }
        if relationship.relationship_type == format!("{OFFICE_REL}tags")
            && let Some(diagnostic) =
                user_defined_tags_diagnostic(archive, &source_part, relationship)
        {
            diagnostics.push(diagnostic);
        }
    }

    // The package diagnostic collector already visits every .rels part. Slide-owned
    // relationships provide a deterministic hook to inventory original MC branches.
    if source_part.starts_with("ppt/slides/")
        && source_part.ends_with(".xml")
        && let Ok(slide_xml) = read_text_entry(archive, &source_part)
    {
        let (_, inventories) = select_alternate_content(&slide_xml, &source_part)?;
        for inventory in inventories {
            diagnostics.push(ConversionDiagnostic {
                code: "OOXML_ALTERNATE_CONTENT_PRESERVED".to_owned(),
                family: FeatureFamily::Unsupported,
                support_tier: SupportTier::Fallback,
                stage: None,
                location: DiagnosticLocation {
                    slide_index: slide_index_from_part(&source_part),
                    part_name: Some(source_part.clone()),
                    qualified_element_name: Some("mc:AlternateContent".to_owned()),
                    relationship_id: Some(inventory.source_identity.clone()),
                    ..Default::default()
                },
                raw_reference: Some(inventory.to_json()),
                fallback_kind: FallbackKind::PreservedPart,
                reason: "All Markup Compatibility branches and Requires tokens were preserved; exactly one supported branch was selected".to_owned(),
            });
        }
    }
    Ok(())
}

fn user_defined_tags_diagnostic(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    source_part: &str,
    relationship: &Relationship,
) -> Option<ConversionDiagnostic> {
    const CONTENT_TYPE: &str =
        "application/vnd.openxmlformats-officedocument.presentationml.tags+xml";
    if !matches!(relationship.target_mode, TargetMode::Internal) {
        return None;
    }
    let part_name = resolve_target(source_part, &relationship.target)?;
    if declared_content_type(archive, &part_name)?.as_str() != CONTENT_TYPE {
        return None;
    }
    let tags_xml = read_text_entry(archive, &part_name).ok()?;
    let tags = parse_user_defined_tags(&tags_xml)?;
    let mut raw_reference = format!(
        "owner={source_part}\npart={part_name}\nrelationship_id={}",
        relationship.id
    );
    for (name, value) in tags {
        if raw_reference.len() + name.len() + value.len() + 2 > RAW_REFERENCE_LIMIT {
            break;
        }
        raw_reference.push('\n');
        raw_reference.push_str(&name);
        raw_reference.push('=');
        raw_reference.push_str(&value);
    }
    Some(ConversionDiagnostic {
        code: "USER_DEFINED_TAGS_METADATA".to_owned(),
        family: FeatureFamily::Unsupported,
        support_tier: SupportTier::Fallback,
        stage: Some(crate::model::CapabilityStage::Parsed),
        location: DiagnosticLocation {
            slide_index: slide_index_from_part(source_part),
            part_name: Some(part_name),
            relationship_id: Some(relationship.id.clone()),
            relationship_type: Some(relationship.relationship_type.clone()),
            qualified_element_name: Some("p:tagLst".to_owned()),
            ..Default::default()
        },
        raw_reference: Some(raw_reference),
        fallback_kind: FallbackKind::PreservedPart,
        reason: "User-defined presentation tags were preserved as metadata".to_owned(),
    })
}

fn parse_user_defined_tags(xml: &str) -> Option<Vec<(String, String)>> {
    const PML: &[u8] = b"http://schemas.openxmlformats.org/presentationml/2006/main";
    let mut reader = NsReader::from_str(xml);
    let mut saw_root = false;
    let mut tags = Vec::new();
    loop {
        match reader.read_resolved_event() {
            Ok((
                ResolveResult::Bound(namespace),
                Event::Start(ref element) | Event::Empty(ref element),
            )) if namespace.as_ref() == PML => match element.local_name().as_ref() {
                b"tagLst" if !saw_root => saw_root = true,
                b"tag" if saw_root => {
                    let name = attribute_value(element, "name")?;
                    let value = attribute_value(element, "val")?;
                    tags.push((name, value));
                }
                _ => {}
            },
            Ok((_, Event::Eof)) => {
                tags.sort();
                tags.dedup();
                return saw_root.then_some(tags);
            }
            Err(_) => return None,
            _ => {}
        }
    }
}

fn embedded_control_diagnostic(
    archive: &mut ZipArchive<Cursor<&[u8]>>,
    source_part: &str,
    relationship: &Relationship,
) -> Option<ConversionDiagnostic> {
    if !matches!(relationship.target_mode, TargetMode::Internal) {
        return None;
    }
    let part_name = resolve_target(source_part, &relationship.target)?;
    if declared_content_type(archive, &part_name)?.as_str()
        != "application/vnd.ms-office.activeX+xml"
    {
        return None;
    }
    let control_xml = read_text_entry(archive, &part_name).ok()?;
    let (class_id, persistence, binary_relationship_id) = parse_active_x_root(&control_xml)?;
    let (directory, file) = part_name.rsplit_once('/')?;
    let rels_path = format!("{directory}/_rels/{file}.rels");
    let relationships_xml = read_text_entry(archive, &rels_path).ok()?;
    let binary_relationship = relationships::parse_relationship_records(&relationships_xml)
        .ok()?
        .into_iter()
        .find(|candidate| {
            candidate.id == binary_relationship_id
                && candidate.relationship_type == ACTIVEX_BINARY_REL
                && matches!(candidate.target_mode, TargetMode::Internal)
        })?;
    let binary_part = resolve_target(&part_name, &binary_relationship.target)?;
    if declared_content_type(archive, &binary_part)?.as_str() != "application/vnd.ms-office.activeX"
    {
        return None;
    }
    let binary_byte_length = archive.by_name(&binary_part).ok()?.size();
    Some(ConversionDiagnostic {
        code: "EMBEDDED_CONTROL_PERSISTENCE_METADATA".to_owned(),
        family: FeatureFamily::Unsupported,
        support_tier: SupportTier::Fallback,
        stage: Some(crate::model::CapabilityStage::Parsed),
        location: DiagnosticLocation {
            slide_index: slide_index_from_part(source_part),
            part_name: Some(part_name.clone()),
            relationship_id: Some(relationship.id.clone()),
            relationship_type: Some(relationship.relationship_type.clone()),
            qualified_element_name: Some("ax:ocx".to_owned()),
            ..Default::default()
        },
        raw_reference: Some(format!(
            "owner={source_part}\npart={part_name}\nrelationship_id={}\nclass_id={class_id}\npersistence={persistence}\nbinary_relationship_id={binary_relationship_id}\nbinary_part={binary_part}\nbinary_byte_length={binary_byte_length}",
            relationship.id
        )),
        fallback_kind: FallbackKind::PreservedPart,
        reason: "Embedded control persistence metadata was preserved without activating or exposing its payload".to_owned(),
    })
}

fn parse_active_x_root(xml: &str) -> Option<(String, String, String)> {
    const ACTIVEX: &[u8] = b"http://schemas.microsoft.com/office/2006/activeX";
    const REL: &[u8] = b"http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let mut reader = NsReader::from_str(xml);
    loop {
        match reader.read_resolved_event() {
            Ok((
                ResolveResult::Bound(namespace),
                Event::Start(ref element) | Event::Empty(ref element),
            )) if namespace.as_ref() == ACTIVEX && element.local_name().as_ref() == b"ocx" => {
                let mut class_id = None;
                let mut persistence = None;
                let mut relationship_id = None;
                for attribute in element.attributes().flatten() {
                    let local_name = attribute.key.local_name();
                    let value = attribute.unescape_value().ok()?.into_owned();
                    match local_name.as_ref() {
                        b"classid" => class_id = Some(value),
                        b"persistence" => persistence = Some(value),
                        b"id"
                            if matches!(
                                reader.resolve_attribute(attribute.key).0,
                                ResolveResult::Bound(value) if value.as_ref() == REL
                            ) =>
                        {
                            relationship_id = Some(value)
                        }
                        _ => {}
                    }
                }
                return Some((class_id?, persistence?, relationship_id?));
            }
            Ok((_, Event::Eof)) | Err(_) => return None,
            _ => {}
        }
    }
}

fn embedded_package_diagnostic<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    source_part: &str,
    part_name: &str,
    relationship: &Relationship,
) -> Option<ConversionDiagnostic> {
    let content_type = declared_content_type(archive, part_name)?;
    let byte_length = archive.by_name(part_name).ok()?.size();
    Some(ConversionDiagnostic {
        code: "EMBEDDED_PACKAGE_METADATA".to_owned(),
        family: FeatureFamily::Unsupported,
        support_tier: SupportTier::Fallback,
        stage: Some(crate::model::CapabilityStage::Parsed),
        location: DiagnosticLocation {
            slide_index: slide_index_from_part(source_part),
            part_name: Some(part_name.to_owned()),
            relationship_id: Some(relationship.id.clone()),
            relationship_type: Some(relationship.relationship_type.clone()),
            qualified_element_name: Some("p:oleObj".to_owned()),
            ..Default::default()
        },
        raw_reference: Some(format!(
            "owner={source_part}\npart={part_name}\nrelationship_id={}\ncontent_type={content_type}\nbyte_length={byte_length}",
            relationship.id
        )),
        fallback_kind: FallbackKind::PreservedPart,
        reason:
            "Embedded package metadata was preserved without activating or exposing its payload"
                .to_owned(),
    })
}

pub(crate) fn select_alternate_content(
    xml: &str,
    owner_part: &str,
) -> PptxResult<(String, Vec<AlternateContentInventory>)> {
    select_alternate_content_with_namespaces(xml, owner_part, BTreeMap::new())
}

fn select_alternate_content_with_namespaces(
    xml: &str,
    owner_part: &str,
    inherited_namespaces: BTreeMap<String, String>,
) -> PptxResult<(String, Vec<AlternateContentInventory>)> {
    let mut reader = NsReader::from_str(xml);
    let mut output = Writer::new(Vec::new());
    let mut inventories = Vec::new();
    let mut active: Option<AlternateCapture> = None;
    let mut ordinal = 0usize;
    let mut namespace_scopes = vec![inherited_namespaces];

    loop {
        let (namespace, event) = reader.read_resolved_event()?;
        if let Event::Start(element) = &event {
            let mut scope = namespace_scopes.last().cloned().unwrap_or_default();
            scope.extend(namespace_bindings(element));
            namespace_scopes.push(scope);
        }
        let closes_scope = matches!(event, Event::End(_));
        let current_scope = namespace_scopes.last().expect("namespace scope");
        let event_is_mc = is_mc_event(&namespace, &event, current_scope);
        if active.is_none()
            && event_is_mc
            && event_local(&event).as_deref() == Some("AlternateContent")
            && matches!(event, Event::Start(_))
        {
            active = Some(AlternateCapture::new(format!(
                "{owner_part}#alternate-{ordinal}"
            )));
            ordinal += 1;
            continue;
        }
        if let Some(capture) = active.as_mut() {
            let branch_scope = (capture.depth == 0
                && event_is_mc
                && matches!(event_local(&event).as_deref(), Some("Choice" | "Fallback"))
                && matches!(event, Event::Start(_)))
            .then(|| current_scope.clone());
            if capture.handle(branch_scope, event_is_mc, event)? {
                let capture = active.take().expect("active AlternateContent capture");
                let (selected, selected_scope, inventory) = capture.finish();
                let (selected, mut nested) = select_alternate_content_with_namespaces(
                    &selected,
                    owner_part,
                    selected_scope,
                )?;
                output.get_mut().extend_from_slice(selected.as_bytes());
                inventories.push(inventory);
                inventories.append(&mut nested);
            }
        } else {
            let eof = matches!(event, Event::Eof);
            output.write_event(event.into_owned())?;
            if eof {
                break;
            }
        }
        if closes_scope {
            namespace_scopes.pop();
        }
    }
    let bytes = output.into_inner();
    let selected = String::from_utf8_lossy(&bytes).replace(
        "http://schemas.openxmlformats.org/presentationml/2006/ole\"",
        "http://schemas.openxmlformats.org/presentationml/2006/oleObject\"",
    );
    Ok((selected, inventories))
}

struct AlternateCapture {
    source_identity: String,
    depth: usize,
    current: Option<CapturedBranch>,
    branches: Vec<CapturedBranch>,
}

struct CapturedBranch {
    kind: &'static str,
    requires: Vec<String>,
    supported: bool,
    namespaces: BTreeMap<String, String>,
    writer: Writer<Vec<u8>>,
}

impl AlternateCapture {
    fn new(source_identity: String) -> Self {
        Self {
            source_identity,
            depth: 0,
            current: None,
            branches: Vec::new(),
        }
    }

    fn handle(
        &mut self,
        branch_scope: Option<BTreeMap<String, String>>,
        event_is_mc: bool,
        event: Event<'_>,
    ) -> PptxResult<bool> {
        let local = event_local(&event);
        if self.depth == 0 && event_is_mc && matches!(local.as_deref(), Some("Choice" | "Fallback"))
        {
            let (kind, requires, supported) = match &event {
                Event::Start(element) => {
                    let kind = if local.as_deref() == Some("Choice") {
                        "choice"
                    } else {
                        "fallback"
                    };
                    let requires: Vec<String> = attribute_value(element, "Requires")
                        .map(|value| value.split_whitespace().map(str::to_owned).collect())
                        .unwrap_or_default();
                    let namespaces = branch_scope.as_ref().expect("branch namespace scope");
                    let supported =
                        kind == "fallback" || requirements_supported(namespaces, &requires);
                    (kind, requires, supported)
                }
                _ => return Ok(false),
            };
            let mut branch = CapturedBranch {
                kind,
                requires,
                supported,
                namespaces: branch_scope.expect("branch namespace scope"),
                writer: Writer::new(Vec::new()),
            };
            branch.writer.write_event(event.into_owned())?;
            self.current = Some(branch);
            self.depth = 1;
            return Ok(false);
        }
        if let Some(branch) = self.current.as_mut() {
            let closes_branch = self.depth == 1
                && event_is_mc
                && matches!(local.as_deref(), Some("Choice" | "Fallback"))
                && matches!(event, Event::End(_));
            branch.writer.write_event(event.clone().into_owned())?;
            match event {
                Event::Start(_) => self.depth += 1,
                Event::End(_) => self.depth = self.depth.saturating_sub(1),
                _ => {}
            }
            if closes_branch {
                self.branches
                    .push(self.current.take().expect("captured branch"));
            }
            return Ok(false);
        }
        if event_is_mc
            && local.as_deref() == Some("AlternateContent")
            && matches!(event, Event::End(_))
        {
            return Ok(true);
        }
        Ok(false)
    }

    fn finish(self) -> (String, BTreeMap<String, String>, AlternateContentInventory) {
        let selected = self
            .branches
            .iter()
            .position(|branch| branch.kind == "choice" && branch.supported)
            .or_else(|| {
                self.branches
                    .iter()
                    .position(|branch| branch.kind == "fallback")
            })
            .unwrap_or(0);
        let selected_xml = self
            .branches
            .get(selected)
            .map(|branch| String::from_utf8_lossy(branch.writer.get_ref()).into_owned())
            .unwrap_or_default();
        let selected_namespaces = self
            .branches
            .get(selected)
            .map(|branch| branch.namespaces.clone())
            .unwrap_or_default();
        let inventory = AlternateContentInventory {
            source_identity: self.source_identity,
            selected_branch: selected,
            branches: self
                .branches
                .into_iter()
                .map(|branch| AlternateContentBranch {
                    kind: branch.kind,
                    requires: branch.requires,
                    supported: branch.supported,
                    raw_xml: String::from_utf8_lossy(&branch.writer.into_inner()).into_owned(),
                })
                .collect(),
        };
        (selected_xml, selected_namespaces, inventory)
    }
}

fn requirements_supported(namespaces: &BTreeMap<String, String>, requirements: &[String]) -> bool {
    requirements.iter().all(|token| {
        namespaces
            .get(token)
            .is_some_and(|uri| SUPPORTED_REQUIREMENT_NAMESPACES.contains(&uri.as_str()))
    })
}

fn namespace_bindings(element: &BytesStart<'_>) -> BTreeMap<String, String> {
    element
        .attributes()
        .flatten()
        .filter_map(|attribute| {
            let key = std::str::from_utf8(attribute.key.as_ref()).ok()?;
            let prefix = key.strip_prefix("xmlns:")?;
            Some((
                prefix.to_owned(),
                String::from_utf8_lossy(&attribute.value).into_owned(),
            ))
        })
        .collect()
}

pub(crate) fn resolve_slide<R: Read + Seek>(
    slide: &mut crate::model::Slide,
    relationships: &[Relationship],
    owner_part: &str,
    archive: &mut ZipArchive<R>,
    store: &mut EmbeddedInventoryStore,
) {
    let mut occurrences = BTreeMap::new();
    for shape in &mut slide.shapes {
        resolve_shape(
            shape,
            relationships,
            owner_part,
            archive,
            &mut occurrences,
            store,
        );
    }
}

fn resolve_shape<R: Read + Seek>(
    shape: &mut Shape,
    relationships: &[Relationship],
    owner_part: &str,
    archive: &mut ZipArchive<R>,
    occurrences: &mut BTreeMap<(u32, &'static str), usize>,
    store: &mut EmbeddedInventoryStore,
) {
    if let ShapeType::Group(children, _) = &mut shape.shape_type {
        for child in children {
            resolve_shape(
                child,
                relationships,
                owner_part,
                archive,
                occurrences,
                store,
            );
        }
        return;
    }
    let ShapeType::Unsupported(data) = &mut shape.shape_type else {
        return;
    };
    if data.element_type == UnresolvedType::CustomGeometry {
        return;
    }
    let raw = data.raw_xml.as_deref().unwrap_or_default();
    let ids = relationship_ids(raw);
    let expected = match data.element_type {
        UnresolvedType::SmartArt => Some(
            &[
                "diagramData",
                "diagramLayout",
                "diagramQuickStyle",
                "diagramColors",
            ][..],
        ),
        UnresolvedType::OleObject => Some(&["oleObject", "package"][..]),
        _ => None,
    };
    let domain = domain_name(&data.element_type);
    let occurrence = occurrences.entry((shape.id, domain)).or_default();
    let source_identity = format!(
        "{owner_part}#shape-{}-{domain}-occurrence-{occurrence}",
        shape.id
    );
    *occurrence += 1;
    let mut inventory = EmbeddedInventoryEntry {
        source_identity,
        ..Default::default()
    };
    let mut seen = BTreeSet::new();
    for id in ids {
        let Some(relationship) = relationships
            .iter()
            .find(|relationship| relationship.id == id)
        else {
            continue;
        };
        if !matches!(relationship.target_mode, TargetMode::Internal) {
            continue;
        }
        let Some(part_name) = resolve_target(owner_part, &relationship.target) else {
            continue;
        };
        let Some(kind) = relationship.relationship_type.strip_prefix(OFFICE_REL) else {
            continue;
        };
        if kind == "image" {
            if inventory.preview.is_none() {
                inventory.preview = read_preview(archive, &relationship.id, &part_name);
            }
        } else if expected.is_some_and(|kinds| kinds.contains(&kind))
            && valid_embedded_part(archive, kind, &part_name)
            && seen.insert((relationship.id.clone(), part_name.clone()))
        {
            inventory.relationships.push(EmbeddedRelationship {
                id: relationship.id.clone(),
                relationship_type: relationship.relationship_type.clone(),
                part_name,
            });
        }
    }
    if data.element_type == UnresolvedType::SmartArt {
        close_part_relationships(archive, &mut inventory, &mut seen);
    }
    inventory.relationships.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| left.part_name.cmp(&right.part_name))
    });
    let key = inventory_key(owner_part, shape.id, domain, raw);
    store.register(key, inventory);
}

const SMARTART_MAX_DEPTH: usize = 8;
const SMARTART_MAX_RELATIONSHIPS: usize = 32;
const SMARTART_MAX_RELS_BYTES: usize = 128 * 1024;

fn close_part_relationships<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    inventory: &mut EmbeddedInventoryEntry,
    seen: &mut BTreeSet<(String, String)>,
) {
    let mut queue = inventory
        .relationships
        .iter()
        .map(|relationship| (relationship.part_name.clone(), 0usize))
        .collect::<VecDeque<_>>();
    let mut visited = BTreeSet::new();
    let mut relationship_bytes = 0usize;
    while let Some((owner, depth)) = queue.pop_front() {
        if depth >= SMARTART_MAX_DEPTH
            || inventory.relationships.len() >= SMARTART_MAX_RELATIONSHIPS
            || !visited.insert(owner.clone())
        {
            continue;
        }
        let Some((directory, file)) = owner.rsplit_once('/') else {
            continue;
        };
        let rels_path = format!("{directory}/_rels/{file}.rels");
        let Some((entry_size, mut records)) = (|| {
            let mut entry = archive.by_name(&rels_path).ok()?;
            let entry_size = usize::try_from(entry.size()).ok()?;
            if entry_size > RAW_REFERENCE_LIMIT
                || relationship_bytes.saturating_add(entry_size) > SMARTART_MAX_RELS_BYTES
            {
                return None;
            }
            let mut xml = String::new();
            entry.read_to_string(&mut xml).ok()?;
            let records = relationships::parse_relationship_records(&xml).ok()?;
            Some((entry_size, records))
        })() else {
            continue;
        };
        relationship_bytes += entry_size;
        records.sort_by(|left, right| {
            left.id
                .cmp(&right.id)
                .then_with(|| left.relationship_type.cmp(&right.relationship_type))
                .then_with(|| left.target.cmp(&right.target))
        });
        for relationship in records {
            if inventory.relationships.len() >= SMARTART_MAX_RELATIONSHIPS
                || !matches!(relationship.target_mode, TargetMode::Internal)
            {
                break;
            }
            let Some(part_name) = resolve_target(&owner, &relationship.target) else {
                continue;
            };
            let Some(kind) = relationship.relationship_type.strip_prefix(OFFICE_REL) else {
                continue;
            };
            if kind == "image" {
                if inventory.preview.is_none() {
                    inventory.preview = read_preview(archive, &relationship.id, &part_name);
                }
                continue;
            }
            if !matches!(
                kind,
                "diagramData" | "diagramLayout" | "diagramQuickStyle" | "diagramColors"
            ) || !valid_embedded_part(archive, kind, &part_name)
            {
                continue;
            }
            let identity = format!("{owner}#{}", relationship.id);
            if !seen.insert((identity.clone(), part_name.clone())) {
                continue;
            }
            inventory.relationships.push(EmbeddedRelationship {
                id: identity,
                relationship_type: relationship.relationship_type,
                part_name: part_name.clone(),
            });
            queue.push_back((part_name, depth + 1));
        }
    }
}

fn read_preview<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    relationship_id: &str,
    part_name: &str,
) -> Option<EmbeddedPreview> {
    let mime_type = preview_mime(part_name)?;
    if declared_content_type(archive, part_name).as_deref() != Some(mime_type) {
        return None;
    }
    let mut file = archive.by_name(part_name).ok()?;
    if file.size() as usize > PREVIEW_BYTE_LIMIT {
        return None;
    }
    let mut bytes = Vec::with_capacity(file.size() as usize);
    file.read_to_end(&mut bytes).ok()?;
    if !preview_magic(mime_type, &bytes) {
        return None;
    }
    Some(EmbeddedPreview {
        relationship_id: relationship_id.to_owned(),
        mime_type: mime_type.to_owned(),
        base64: base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}

fn declared_content_type<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    part_name: &str,
) -> Option<String> {
    let mut file = archive.by_name("[Content_Types].xml").ok()?;
    let mut xml = String::new();
    file.read_to_string(&mut xml).ok()?;
    let mut reader = quick_xml::Reader::from_str(&xml);
    let expected = format!("/{part_name}");
    let extension = part_name.rsplit_once('.')?.1;
    let mut default = None;
    loop {
        match reader.read_event().ok()? {
            Event::Start(element) | Event::Empty(element)
                if element.name().local_name().as_ref() == b"Override" =>
            {
                if attribute_value(&element, "PartName").as_deref() == Some(&expected)
                    && let Some(content_type) = attribute_value(&element, "ContentType")
                {
                    return Some(content_type);
                }
            }
            Event::Start(element) | Event::Empty(element)
                if element.name().local_name().as_ref() == b"Default"
                    && attribute_value(&element, "Extension")
                        .is_some_and(|value| value.eq_ignore_ascii_case(extension)) =>
            {
                default = attribute_value(&element, "ContentType");
            }
            Event::Eof => return default,
            _ => {}
        }
    }
}

fn valid_embedded_part<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    kind: &str,
    part_name: &str,
) -> bool {
    let Some(content_type) = declared_content_type(archive, part_name) else {
        return false;
    };
    let diagram = match kind {
        "diagramData" => Some((
            "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml",
            "dataModel",
        )),
        "diagramLayout" => Some((
            "application/vnd.openxmlformats-officedocument.drawingml.diagramLayout+xml",
            "layoutDef",
        )),
        "diagramQuickStyle" => Some((
            "application/vnd.openxmlformats-officedocument.drawingml.diagramStyle+xml",
            "styleDef",
        )),
        "diagramColors" => Some((
            "application/vnd.openxmlformats-officedocument.drawingml.diagramColors+xml",
            "colorsDef",
        )),
        _ => None,
    };
    if let Some((expected_content_type, expected_root)) = diagram {
        if content_type != expected_content_type {
            return false;
        }
        return valid_diagram_root(archive, part_name, expected_root);
    }
    matches!(kind, "oleObject" | "package")
        && content_type.starts_with("application/vnd.")
        && !content_type.contains("html")
        && archive.by_name(part_name).is_ok()
}

fn valid_diagram_root<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    part_name: &str,
    expected_root: &str,
) -> bool {
    let Ok(mut part) = archive.by_name(part_name) else {
        return false;
    };
    if part.size() as usize > RAW_REFERENCE_LIMIT {
        return false;
    }
    let mut xml = String::new();
    if part.read_to_string(&mut xml).is_err() {
        return false;
    }
    let mut reader = NsReader::from_str(&xml);
    loop {
        match reader.read_resolved_event() {
            Ok((
                ResolveResult::Bound(namespace),
                Event::Start(element) | Event::Empty(element),
            )) => {
                return namespace.as_ref()
                    == b"http://schemas.openxmlformats.org/drawingml/2006/diagram"
                    && element.name().local_name().as_ref() == expected_root.as_bytes();
            }
            Ok((_, Event::Eof)) | Err(_) => return false,
            _ => {}
        }
    }
}

fn preview_mime(path: &str) -> Option<&'static str> {
    (path.rsplit_once('.')?.1.eq_ignore_ascii_case("png")).then_some("image/png")
}

fn preview_magic(mime: &str, bytes: &[u8]) -> bool {
    mime == "image/png" && valid_safe_png(bytes)
}

// Safe preview subset: PNG with only IHDR/IDAT/IEND, 8-bit non-interlaced
// RGBA, bounded dimensions, valid chunk CRCs, and zlib stored blocks whose
// decoded scanlines use filter 0. Other image families and PNG forms fall
// back to a placeholder rather than being emitted.
fn valid_safe_png(bytes: &[u8]) -> bool {
    const MAX_DIMENSION: usize = 4_096;
    const MAX_DECOMPRESSED: usize = 64 * 1024 * 1024;
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return false;
    }
    let mut offset = 8usize;
    let mut width = 0usize;
    let mut height = 0usize;
    let mut idat = Vec::new();
    let mut state = 0u8;
    loop {
        let Some(header) = bytes.get(offset..offset + 8) else {
            return false;
        };
        let length = u32::from_be_bytes(header[..4].try_into().unwrap_or_default()) as usize;
        let kind = &header[4..8];
        let Some(data_end) = offset
            .checked_add(8)
            .and_then(|value| value.checked_add(length))
        else {
            return false;
        };
        let Some(chunk_end) = data_end.checked_add(4) else {
            return false;
        };
        let (Some(data), Some(crc_bytes)) = (
            bytes.get(offset + 8..data_end),
            bytes.get(data_end..chunk_end),
        ) else {
            return false;
        };
        let expected_crc = u32::from_be_bytes(crc_bytes.try_into().unwrap_or_default());
        if png_crc(kind, data) != expected_crc {
            return false;
        }
        match kind {
            b"IHDR" if state == 0 && length == 13 => {
                width = u32::from_be_bytes(data[..4].try_into().unwrap_or_default()) as usize;
                height = u32::from_be_bytes(data[4..8].try_into().unwrap_or_default()) as usize;
                if width == 0
                    || height == 0
                    || width > MAX_DIMENSION
                    || height > MAX_DIMENSION
                    || data[8..] != [8, 6, 0, 0, 0]
                {
                    return false;
                }
                state = 1;
            }
            b"IDAT" if matches!(state, 1 | 2) => {
                if idat.len().saturating_add(data.len()) > PREVIEW_BYTE_LIMIT {
                    return false;
                }
                idat.extend_from_slice(data);
                state = 2;
            }
            b"IEND" if state == 2 && data.is_empty() && chunk_end == bytes.len() => {
                let Some(row_bytes) = width.checked_mul(4).and_then(|value| value.checked_add(1))
                else {
                    return false;
                };
                let Some(expected_size) = row_bytes.checked_mul(height) else {
                    return false;
                };
                if expected_size > MAX_DECOMPRESSED {
                    return false;
                }
                let Some(decoded) = decode_stored_zlib(&idat, expected_size) else {
                    return false;
                };
                return decoded
                    .chunks_exact(row_bytes)
                    .all(|scanline| scanline.first() == Some(&0));
            }
            _ => return false,
        }
        offset = chunk_end;
    }
}

fn png_crc(kind: &[u8], data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in kind.iter().chain(data) {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

fn decode_stored_zlib(bytes: &[u8], expected_size: usize) -> Option<Vec<u8>> {
    let header = bytes.get(..2)?;
    let header_value = u16::from_be_bytes(header.try_into().ok()?);
    if header[0] & 0x0f != 8
        || header[0] >> 4 > 7
        || header_value % 31 != 0
        || header[1] & 0x20 != 0
    {
        return None;
    }
    let mut offset = 2usize;
    let mut output = Vec::with_capacity(expected_size);
    loop {
        let block_header = *bytes.get(offset)?;
        offset += 1;
        let final_block = block_header & 1 != 0;
        if block_header & 0xfe != 0 {
            return None;
        }
        let length = u16::from_le_bytes(bytes.get(offset..offset + 2)?.try_into().ok()?) as usize;
        let complement = u16::from_le_bytes(bytes.get(offset + 2..offset + 4)?.try_into().ok()?);
        offset += 4;
        if (length as u16) != !complement || output.len().saturating_add(length) > expected_size {
            return None;
        }
        output.extend_from_slice(bytes.get(offset..offset + length)?);
        offset += length;
        if final_block {
            break;
        }
    }
    let expected_adler = u32::from_be_bytes(bytes.get(offset..offset + 4)?.try_into().ok()?);
    offset += 4;
    (offset == bytes.len() && output.len() == expected_size && adler32(&output) == expected_adler)
        .then_some(output)
}

fn adler32(bytes: &[u8]) -> u32 {
    let mut first = 1u32;
    let mut second = 0u32;
    for byte in bytes {
        first = (first + u32::from(*byte)) % 65_521;
        second = (second + first) % 65_521;
    }
    (second << 16) | first
}
fn domain_name(value: &UnresolvedType) -> &'static str {
    match value {
        UnresolvedType::SmartArt => "smartart",
        UnresolvedType::OleObject => "ole",
        UnresolvedType::MathEquation => "math",
        UnresolvedType::CustomGeometry => "custom-geometry",
    }
}

fn relationship_ids(xml: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for marker in [
        "r:id=\"",
        "r:dm=\"",
        "r:lo=\"",
        "r:qs=\"",
        "r:cs=\"",
        "r:embed=\"",
    ] {
        let mut tail = xml;
        while let Some((_, rest)) = tail.split_once(marker) {
            if let Some((id, remaining)) = rest.split_once('"') {
                ids.push(id.to_owned());
                tail = remaining;
            } else {
                break;
            }
        }
    }
    ids.sort();
    ids.dedup();
    ids
}

fn resolve_target(owner_part: &str, target: &str) -> Option<String> {
    if target.is_empty() || target.starts_with('/') || target.contains(['\\', ':', '?', '#', '%']) {
        return None;
    }
    let mut path = owner_part
        .rsplit_once('/')
        .map(|(parent, _)| parent.split('/').collect::<Vec<_>>())
        .unwrap_or_default();
    for segment in target.split('/') {
        match segment {
            "" | "." => return None,
            ".." => {
                path.pop()?;
            }
            value => path.push(value),
        }
    }
    (!path.is_empty()).then(|| path.join("/"))
}

fn relationship_diagnostic(
    code: &str,
    source_part: &str,
    relationship: &Relationship,
    reason: &str,
) -> ConversionDiagnostic {
    ConversionDiagnostic {
        code: code.to_owned(),
        family: FeatureFamily::Unsupported,
        support_tier: SupportTier::Unparsed,
        stage: None,
        location: DiagnosticLocation {
            slide_index: slide_index_from_part(source_part),
            part_name: Some(source_part.to_owned()),
            relationship_id: Some(relationship.id.clone()),
            relationship_type: Some(relationship.relationship_type.clone()),
            ..Default::default()
        },
        raw_reference: Some(format!("{source_part}#{}", relationship.id)),
        fallback_kind: FallbackKind::IgnoredRelationship,
        reason: reason.to_owned(),
    }
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
fn embedded_relationship_kind(value: &str) -> Option<&str> {
    let kind = value.strip_prefix(OFFICE_REL)?;
    matches!(
        kind,
        "oleObject"
            | "package"
            | "diagramData"
            | "diagramLayout"
            | "diagramQuickStyle"
            | "diagramColors"
    )
    .then_some(kind)
}
pub(crate) fn known_relationship_type(value: &str) -> bool {
    const CORE: &str =
        "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties";
    const THUMBNAIL: &str =
        "http://schemas.openxmlformats.org/package/2006/relationships/metadata/thumbnail";
    const CHARTEX: &str = "http://schemas.microsoft.com/office/2014/relationships/chartEx";
    let Some(kind) = value.strip_prefix(OFFICE_REL) else {
        return matches!(value, CORE | THUMBNAIL | CHARTEX | ACTIVEX_BINARY_REL);
    };
    matches!(
        kind,
        "officeDocument"
            | "slide"
            | "slideMaster"
            | "slideLayout"
            | "slideUpdateInfo"
            | "customXml"
            | "control"
            | "tags"
            | "theme"
            | "themeOverride"
            | "image"
            | "chart"
            | "hyperlink"
            | "notesSlide"
            | "comments"
            | "commentAuthors"
            | "oleObject"
            | "package"
            | "audio"
            | "video"
            | "media"
            | "diagramData"
            | "diagramLayout"
            | "diagramQuickStyle"
            | "diagramColors"
            | "extended-properties"
            | "tableStyles"
    )
}
fn is_mc_event(
    namespace: &ResolveResult<'_>,
    event: &Event<'_>,
    namespaces: &BTreeMap<String, String>,
) -> bool {
    if matches!(namespace, ResolveResult::Bound(value) if value.as_ref() == MC) {
        return true;
    }
    let qualified = match event {
        Event::Start(element) | Event::Empty(element) => element.name(),
        Event::End(element) => element.name(),
        _ => return false,
    };
    let qualified = String::from_utf8_lossy(qualified.as_ref());
    let Some((prefix, _)) = qualified.split_once(':') else {
        return false;
    };
    namespaces
        .get(prefix)
        .is_some_and(|uri| uri.as_bytes() == MC)
}
fn event_local(event: &Event<'_>) -> Option<String> {
    let name = match event {
        Event::Start(element) | Event::Empty(element) => element.name(),
        Event::End(element) => element.name(),
        _ => return None,
    };
    Some(String::from_utf8_lossy(name.local_name().as_ref()).into_owned())
}

#[cfg(test)]
mod tests {
    use super::{relationship_source_part, resolve_target};
    #[test]
    fn root_relationship_source_is_the_package_root() {
        assert_eq!(relationship_source_part("_rels/.rels"), "/");
    }
    #[test]
    fn package_target_normalization_is_bounded() {
        assert_eq!(
            resolve_target("ppt/slides/slide1.xml", "../media/a.png").as_deref(),
            Some("ppt/media/a.png")
        );
        assert!(resolve_target("ppt/slides/slide1.xml", "../../../secret").is_none());
    }
}
