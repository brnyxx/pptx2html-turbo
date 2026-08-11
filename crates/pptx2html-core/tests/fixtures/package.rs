use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use tempfile::{Builder as TempDirBuilder, TempDir};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

use super::parts::{
    FeaturePart, Relationship, content_types_xml, relationships_xml,
    resolve_internal_relationship_target,
};

const CONTENT_TYPES_PATH: &str = "[Content_Types].xml";
const PRESENTATION_PATH: &str = "ppt/presentation.xml";
const SLIDE_PATH: &str = "ppt/slides/slide1.xml";
const ROOT_RELS_PATH: &str = "_rels/.rels";
const PRESENTATION_RELS_PATH: &str = "ppt/_rels/presentation.xml.rels";
const SLIDE_RELS_PATH: &str = "ppt/slides/_rels/slide1.xml.rels";
const OFFICE_DOCUMENT_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
const SLIDE_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide";
const ADDITIONAL_RESERVED_PATHS: &[&str] = &[
    "docProps/core.xml",
    "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
    "ppt/slideLayouts/slideLayout1.xml",
    "ppt/slideMasters/_rels/slideMaster1.xml.rels",
    "ppt/slideMasters/slideMaster1.xml",
    "ppt/theme/theme1.xml",
];

#[derive(Debug)]
pub enum FixtureError {
    DanglingRelationship { target: String },
    DuplicatePartPath,
    ReservedPartPath,
    InvalidPartPath,
    InvalidXmlPart,
    DuplicateRelationshipId,
    InvalidRelationshipId,
    InvalidRelationshipTarget,
    Io(std::io::Error),
    Zip(zip::result::ZipError),
}

impl FixtureError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DanglingRelationship { .. } => "DANGLING_RELATIONSHIP",
            Self::DuplicatePartPath => "DUPLICATE_PART_PATH",
            Self::ReservedPartPath => "RESERVED_PART_PATH",
            Self::InvalidPartPath => "INVALID_PART_PATH",
            Self::InvalidXmlPart => "INVALID_XML_PART",
            Self::DuplicateRelationshipId => "DUPLICATE_RELATIONSHIP_ID",
            Self::InvalidRelationshipId => "INVALID_RELATIONSHIP_ID",
            Self::InvalidRelationshipTarget => "INVALID_RELATIONSHIP_TARGET",
            Self::Io(_) => "FIXTURE_IO_ERROR",
            Self::Zip(_) => "FIXTURE_ZIP_ERROR",
        }
    }

    pub fn target(&self) -> Option<&str> {
        if let Self::DanglingRelationship { target } = self {
            Some(target)
        } else {
            None
        }
    }
}

impl Display for FixtureError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => Display::fmt(error, formatter),
            Self::Zip(error) => Display::fmt(error, formatter),
            _ => formatter.write_str(self.code()),
        }
    }
}

impl Error for FixtureError {}

impl From<std::io::Error> for FixtureError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<zip::result::ZipError> for FixtureError {
    fn from(error: zip::result::ZipError) -> Self {
        Self::Zip(error)
    }
}

#[derive(Clone, Debug)]
pub struct PackageBuilder {
    slide_xml: String,
    slide_relationships: Vec<Relationship>,
    parts: Vec<FeaturePart>,
}

impl PackageBuilder {
    pub fn new(slide_xml: String) -> Self {
        Self {
            slide_xml,
            slide_relationships: Vec::new(),
            parts: Vec::new(),
        }
    }

    pub fn with_slide_relationship(mut self, relationship: Relationship) -> Self {
        self.slide_relationships.push(relationship);
        self
    }

    pub fn with_part(mut self, part: FeaturePart) -> Self {
        self.parts.push(part);
        self
    }

    pub fn validate(&self) -> Result<(), FixtureError> {
        let generated_entries = self.generated_entries();
        let mut part_paths = BTreeSet::new();
        for part in &self.parts {
            if generated_entries.contains_key(&part.path)
                || ADDITIONAL_RESERVED_PATHS.contains(&part.path.as_str())
            {
                return Err(FixtureError::ReservedPartPath);
            }
            if !valid_part_path(&part.path) {
                return Err(FixtureError::InvalidPartPath);
            }
            if !part_paths.insert(part.path.as_str()) {
                return Err(FixtureError::DuplicatePartPath);
            }
            if !part.has_valid_xml() {
                return Err(FixtureError::InvalidXmlPart);
            }
        }
        validate_relationship_ids(&self.slide_relationships)?;
        let entries = self.entries();
        for (source, target) in [
            ("", PRESENTATION_PATH),
            (PRESENTATION_PATH, "slides/slide1.xml"),
        ] {
            validate_relationship_target(&entries, source, target)?;
        }
        for relationship in &self.slide_relationships {
            if !relationship.external {
                validate_relationship_target(&entries, SLIDE_PATH, &relationship.target)?;
            }
        }
        Ok(())
    }

    pub fn build(&self) -> Result<Vec<u8>, FixtureError> {
        self.validate()?;
        let cursor = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(DateTime::default());

        for (path, bytes) in self.entries() {
            zip.start_file(path, options)?;
            zip.write_all(&bytes)?;
        }

        Ok(zip.finish()?.into_inner())
    }

    pub fn write_to_temp(&self, namespace: &str) -> Result<TemporaryPackage, FixtureError> {
        let directory = TempDirBuilder::new()
            .prefix(&format!("pptx2html-core-{namespace}-"))
            .tempdir()?;
        let path = directory.path().join("fixture.pptx");
        fs::write(&path, self.build()?)?;
        Ok(TemporaryPackage {
            _directory: directory,
            path,
        })
    }

    fn entries(&self) -> BTreeMap<String, Vec<u8>> {
        let mut entries = self.generated_entries();
        for part in &self.parts {
            entries.insert(part.path.clone(), part.bytes.clone());
        }
        entries
    }

    fn generated_entries(&self) -> BTreeMap<String, Vec<u8>> {
        let mut entries = BTreeMap::new();
        for (path, bytes) in [
            (
                CONTENT_TYPES_PATH,
                content_types_xml(&self.parts).into_bytes(),
            ),
            (ROOT_RELS_PATH, root_relationships_xml().into_bytes()),
            (PRESENTATION_PATH, presentation_xml().into_bytes()),
            (
                PRESENTATION_RELS_PATH,
                presentation_relationships_xml().into_bytes(),
            ),
            (SLIDE_PATH, self.slide_xml.as_bytes().to_vec()),
            (
                SLIDE_RELS_PATH,
                relationships_xml(&self.slide_relationships).into_bytes(),
            ),
        ] {
            entries.insert(path.to_owned(), bytes);
        }
        entries
    }
}

pub struct TemporaryPackage {
    _directory: TempDir,
    path: PathBuf,
}

impl TemporaryPackage {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn root_relationships_xml() -> String {
    relationships_xml(&[Relationship::internal(
        "rId1",
        OFFICE_DOCUMENT_RELATIONSHIP,
        PRESENTATION_PATH,
    )])
}

fn presentation_relationships_xml() -> String {
    relationships_xml(&[Relationship::internal(
        "rId1",
        SLIDE_RELATIONSHIP,
        "slides/slide1.xml",
    )])
}

fn presentation_xml() -> String {
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldIdLst><p:sldId id="256" r:id="rId1"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#
        .to_owned()
}

fn validate_relationship_target(
    entries: &BTreeMap<String, Vec<u8>>,
    source: &str,
    target: &str,
) -> Result<(), FixtureError> {
    let resolved_target = resolve_internal_relationship_target(source, target)
        .map_err(|()| FixtureError::InvalidRelationshipTarget)?;
    if entries.contains_key(&resolved_target) {
        return Ok(());
    }
    Err(FixtureError::DanglingRelationship {
        target: resolved_target,
    })
}

fn validate_relationship_ids(relationships: &[Relationship]) -> Result<(), FixtureError> {
    let mut ids = BTreeSet::new();
    for relationship in relationships {
        if !valid_ncname(&relationship.id) {
            return Err(FixtureError::InvalidRelationshipId);
        }
        if !ids.insert(relationship.id.as_str()) {
            return Err(FixtureError::DuplicateRelationshipId);
        }
    }
    Ok(())
}

fn valid_part_path(path: &str) -> bool {
    path.starts_with("ppt/")
        && !path.contains('\\')
        && path
            .split('/')
            .all(|segment| !matches!(segment, "" | "." | ".."))
}

fn valid_ncname(value: &str) -> bool {
    let mut characters = value.chars();
    characters.next().is_some_and(is_ncname_start) && characters.all(is_ncname_character)
}

fn is_ncname_start(character: char) -> bool {
    matches!(
        character,
        'A'..='Z'
            | '_'
            | 'a'..='z'
            | '\u{C0}'..='\u{D6}'
            | '\u{D8}'..='\u{F6}'
            | '\u{F8}'..='\u{2FF}'
            | '\u{370}'..='\u{37D}'
            | '\u{37F}'..='\u{1FFF}'
            | '\u{200C}'..='\u{200D}'
            | '\u{2070}'..='\u{218F}'
            | '\u{2C00}'..='\u{2FEF}'
            | '\u{3001}'..='\u{D7FF}'
            | '\u{F900}'..='\u{FDCF}'
            | '\u{FDF0}'..='\u{FFFD}'
            | '\u{10000}'..='\u{EFFFF}'
    )
}

fn is_ncname_character(character: char) -> bool {
    is_ncname_start(character)
        || matches!(
            character,
            '-' | '.' | '0'..='9' | '\u{B7}' | '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}'
        )
}
