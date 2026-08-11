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
    FeaturePart, PartValidationError, Relationship, content_types_xml, relationships_xml,
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

#[derive(Debug)]
pub enum FixtureError {
    DanglingRelationship { target: String },
    DuplicatePartPath,
    InvalidPartPath,
    InvalidXmlPart,
    InvalidRelationshipTarget,
    Io(std::io::Error),
    Zip(zip::result::ZipError),
}

impl FixtureError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DanglingRelationship { .. } => "DANGLING_RELATIONSHIP",
            Self::DuplicatePartPath => "DUPLICATE_PART_PATH",
            Self::InvalidPartPath => "INVALID_PART_PATH",
            Self::InvalidXmlPart => "INVALID_XML_PART",
            Self::InvalidRelationshipTarget => "INVALID_RELATIONSHIP_TARGET",
            Self::Io(_) => "FIXTURE_IO_ERROR",
            Self::Zip(_) => "FIXTURE_ZIP_ERROR",
        }
    }

    pub fn target(&self) -> Option<&str> {
        match self {
            Self::DanglingRelationship { target } => Some(target),
            Self::DuplicatePartPath
            | Self::InvalidPartPath
            | Self::InvalidXmlPart
            | Self::InvalidRelationshipTarget
            | Self::Io(_)
            | Self::Zip(_) => None,
        }
    }
}

impl Display for FixtureError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => Display::fmt(error, formatter),
            Self::Zip(error) => Display::fmt(error, formatter),
            Self::DanglingRelationship { .. }
            | Self::DuplicatePartPath
            | Self::InvalidPartPath
            | Self::InvalidXmlPart
            | Self::InvalidRelationshipTarget => write!(formatter, "{}", self.code()),
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
        let mut part_paths = BTreeSet::new();
        for part in &self.parts {
            if !part_paths.insert(part.path.as_str()) {
                return Err(FixtureError::DuplicatePartPath);
            }
            match part.validate() {
                Ok(()) => {}
                Err(PartValidationError::InvalidPath) => {
                    return Err(FixtureError::InvalidPartPath);
                }
                Err(PartValidationError::InvalidXml) => {
                    return Err(FixtureError::InvalidXmlPart);
                }
            }
        }
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
        for part in &self.parts {
            entries.insert(part.path.clone(), part.bytes.clone());
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
