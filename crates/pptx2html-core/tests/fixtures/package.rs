use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use tempfile::{Builder as TempDirBuilder, TempDir};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipWriter};

use super::parts::FeaturePart;

const CONTENT_TYPES_PATH: &str = "[Content_Types].xml";
const PRESENTATION_PATH: &str = "ppt/presentation.xml";
const SLIDE_PATH: &str = "ppt/slides/slide1.xml";
const ROOT_RELS_PATH: &str = "_rels/.rels";
const PRESENTATION_RELS_PATH: &str = "ppt/_rels/presentation.xml.rels";
const SLIDE_RELS_PATH: &str = "ppt/slides/_rels/slide1.xml.rels";
const RELATIONSHIPS_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships";
const OFFICE_DOCUMENT_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";
const SLIDE_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide";

#[derive(Clone, Debug)]
pub struct Relationship {
    id: String,
    relationship_type: String,
    target: String,
    external: bool,
}

impl Relationship {
    pub fn internal(id: &str, relationship_type: &str, target: &str) -> Self {
        Self {
            id: id.to_owned(),
            relationship_type: relationship_type.to_owned(),
            target: target.to_owned(),
            external: false,
        }
    }

    pub fn external(id: &str, relationship_type: &str, target: &str) -> Self {
        Self {
            id: id.to_owned(),
            relationship_type: relationship_type.to_owned(),
            target: target.to_owned(),
            external: true,
        }
    }
}

#[derive(Debug)]
pub enum FixtureError {
    DanglingRelationship {
        source: String,
        relationship_id: String,
        target: String,
    },
    Io(std::io::Error),
    Zip(zip::result::ZipError),
}

impl FixtureError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::DanglingRelationship { .. } => "DANGLING_RELATIONSHIP",
            Self::Io(_) => "FIXTURE_IO_ERROR",
            Self::Zip(_) => "FIXTURE_ZIP_ERROR",
        }
    }

    pub fn target(&self) -> Option<&str> {
        match self {
            Self::DanglingRelationship { target, .. } => Some(target),
            Self::Io(_) | Self::Zip(_) => None,
        }
    }
}

impl Display for FixtureError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DanglingRelationship {
                source,
                relationship_id,
                target,
            } => write!(
                formatter,
                "DANGLING_RELATIONSHIP source={source} id={relationship_id} target={target}"
            ),
            Self::Io(error) => Display::fmt(error, formatter),
            Self::Zip(error) => Display::fmt(error, formatter),
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
    parts: BTreeMap<String, FeaturePart>,
}

impl PackageBuilder {
    pub fn new(slide_xml: String) -> Self {
        Self {
            slide_xml,
            slide_relationships: Vec::new(),
            parts: BTreeMap::new(),
        }
    }

    pub fn with_slide_relationship(mut self, relationship: Relationship) -> Self {
        self.slide_relationships.push(relationship);
        self
    }

    pub fn with_part(mut self, part: FeaturePart) -> Self {
        self.parts.insert(part.path.clone(), part);
        self
    }

    pub fn validate(&self) -> Result<(), FixtureError> {
        let entries = self.entries();
        for (source, relationship_id, target) in [
            ("", "rId1", PRESENTATION_PATH),
            (PRESENTATION_PATH, "rId1", "slides/slide1.xml"),
        ] {
            validate_relationship_target(&entries, source, relationship_id, target)?;
        }
        for relationship in &self.slide_relationships {
            if !relationship.external {
                validate_relationship_target(
                    &entries,
                    SLIDE_PATH,
                    &relationship.id,
                    &relationship.target,
                )?;
            }
        }
        Ok(())
    }

    pub fn build(&self) -> Result<Vec<u8>, FixtureError> {
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
        entries.insert(
            CONTENT_TYPES_PATH.to_owned(),
            self.content_types().into_bytes(),
        );
        entries.insert(
            ROOT_RELS_PATH.to_owned(),
            root_relationships_xml().into_bytes(),
        );
        entries.insert(
            PRESENTATION_PATH.to_owned(),
            presentation_xml().into_bytes(),
        );
        entries.insert(
            PRESENTATION_RELS_PATH.to_owned(),
            presentation_relationships_xml().into_bytes(),
        );
        entries.insert(SLIDE_PATH.to_owned(), self.slide_xml.as_bytes().to_vec());
        entries.insert(
            SLIDE_RELS_PATH.to_owned(),
            relationships_xml(&self.slide_relationships).into_bytes(),
        );
        for (path, part) in &self.parts {
            entries.insert(path.clone(), part.bytes.clone());
        }
        entries
    }

    fn content_types(&self) -> String {
        let mut content_types = String::from(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#,
        );
        for part in self.parts.values() {
            content_types.push_str(&format!(
                "\n  <Override PartName=\"/{}\" ContentType=\"{}\"/>",
                xml_escape(&part.path),
                xml_escape(&part.content_type),
            ));
        }
        content_types.push_str("\n</Types>");
        content_types
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

fn relationships_xml(relationships: &[Relationship]) -> String {
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"{RELATIONSHIPS_NAMESPACE}\">"
    );
    for relationship in relationships {
        let target_mode = if relationship.external {
            " TargetMode=\"External\""
        } else {
            ""
        };
        xml.push_str(&format!(
            "\n  <Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"{target_mode}/>",
            xml_escape(&relationship.id),
            xml_escape(&relationship.relationship_type),
            xml_escape(&relationship.target),
        ));
    }
    xml.push_str("\n</Relationships>");
    xml
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

fn resolve_target(source: &str, target: &str) -> String {
    let mut resolved = if target.starts_with('/') {
        Vec::new()
    } else {
        source.split('/').collect::<Vec<_>>()
    };
    if !target.starts_with('/') {
        resolved.pop();
    }
    for segment in target.trim_start_matches('/').split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                resolved.pop();
            }
            value => resolved.push(value),
        }
    }
    resolved.join("/")
}

fn validate_relationship_target(
    entries: &BTreeMap<String, Vec<u8>>,
    source: &str,
    relationship_id: &str,
    target: &str,
) -> Result<(), FixtureError> {
    let resolved_target = resolve_target(source, target);
    if entries.contains_key(&resolved_target) {
        return Ok(());
    }
    Err(FixtureError::DanglingRelationship {
        source: source.to_owned(),
        relationship_id: relationship_id.to_owned(),
        target: resolved_target,
    })
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
