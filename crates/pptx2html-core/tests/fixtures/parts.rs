use quick_xml::{events::Event, name::ResolveResult, reader::NsReader};

const RELATIONSHIPS_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/package/2006/relationships";
const PRESENTATIONML_NAMESPACE: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
const CHART_NAMESPACE: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
const DRAWINGML_NAMESPACE: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const RELATIONSHIP_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const MARKUP_COMPATIBILITY_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/markup-compatibility/2006";

#[derive(Clone, Copy)]
struct XmlPartSpec {
    path: &'static str,
    content_type: &'static str,
    root: &'static str,
}

const NOTES_PART: XmlPartSpec = XmlPartSpec {
    path: "ppt/notesSlides/notesSlide1.xml",
    content_type: "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml",
    root: "p:notes",
};
const COMMENTS_PART: XmlPartSpec = XmlPartSpec {
    path: "ppt/comments/comment1.xml",
    content_type: "application/vnd.openxmlformats-officedocument.presentationml.comments+xml",
    root: "p:cmLst",
};
const CHART_PART: XmlPartSpec = XmlPartSpec {
    path: "ppt/charts/chart1.xml",
    content_type: "application/vnd.openxmlformats-officedocument.drawingml.chart+xml",
    root: "c:chartSpace",
};

#[derive(Clone, Debug)]
pub struct Relationship {
    pub(super) id: String,
    pub(super) relationship_type: String,
    pub(super) target: String,
    pub(super) external: bool,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PartValidationError {
    InvalidPath,
    InvalidXml,
}

#[derive(Clone, Debug)]
pub struct FeaturePart {
    pub(super) path: String,
    pub(super) content_type: String,
    pub(super) bytes: Vec<u8>,
    xml: bool,
}

impl FeaturePart {
    pub fn notes(xml: &str) -> Self {
        Self::xml(NOTES_PART, xml)
    }

    pub fn comments(xml: &str) -> Self {
        Self::xml(COMMENTS_PART, xml)
    }

    pub fn media(name: &str, content_type: &str, bytes: &[u8]) -> Self {
        Self::extra(&format!("ppt/media/{name}"), content_type, bytes)
    }

    pub fn chart(xml: &str) -> Self {
        Self::xml(CHART_PART, xml)
    }

    pub fn extra(path: &str, content_type: &str, bytes: &[u8]) -> Self {
        Self {
            path: path.to_owned(),
            content_type: content_type.to_owned(),
            bytes: bytes.to_vec(),
            xml: false,
        }
    }

    pub(super) fn validate(&self) -> Result<(), PartValidationError> {
        if !valid_part_path(&self.path) {
            return Err(PartValidationError::InvalidPath);
        }
        if self.xml && !valid_xml_document(&self.bytes) {
            return Err(PartValidationError::InvalidXml);
        }
        Ok(())
    }

    fn xml(spec: XmlPartSpec, content: &str) -> Self {
        Self {
            path: spec.path.to_owned(),
            content_type: spec.content_type.to_owned(),
            bytes: standalone_xml(spec.root, content).into_bytes(),
            xml: true,
        }
    }
}

pub(super) fn content_types_xml(parts: &[FeaturePart]) -> String {
    let mut content_types = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/><Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>"#,
    );
    let mut ordered_parts = parts.iter().collect::<Vec<_>>();
    ordered_parts.sort_unstable_by(|left, right| left.path.cmp(&right.path));
    for part in ordered_parts {
        content_types.push_str(&format!(
            "\n  <Override PartName=\"/{}\" ContentType=\"{}\"/>",
            xml_escape(&part.path),
            xml_escape(&part.content_type),
        ));
    }
    content_types.push_str("\n</Types>");
    content_types
}

pub(super) fn relationships_xml(relationships: &[Relationship]) -> String {
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

pub(super) fn resolve_internal_relationship_target(
    source: &str,
    target: &str,
) -> Result<String, ()> {
    if target.is_empty()
        || matches!(target, "." | "..")
        || target.starts_with('/')
        || target.contains('\\')
    {
        return Err(());
    }

    let mut resolved = source.split('/').collect::<Vec<_>>();
    resolved.pop();
    for segment in target.split('/') {
        match segment {
            "" | "." => return Err(()),
            ".." => {
                resolved.pop().ok_or(())?;
            }
            value => resolved.push(value),
        }
    }
    if resolved.is_empty() {
        return Err(());
    }
    Ok(resolved.join("/"))
}

fn standalone_xml(root: &str, content: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><{root} xmlns:a="{DRAWINGML_NAMESPACE}" xmlns:c="{CHART_NAMESPACE}" xmlns:mc="{MARKUP_COMPATIBILITY_NAMESPACE}" xmlns:p="{PRESENTATIONML_NAMESPACE}" xmlns:r="{RELATIONSHIP_NAMESPACE}">{content}</{root}>"#
    )
}

fn valid_part_path(path: &str) -> bool {
    path.starts_with("ppt/")
        && !path.contains('\\')
        && path
            .split('/')
            .all(|segment| !matches!(segment, "" | "." | ".."))
}

fn valid_xml_document(bytes: &[u8]) -> bool {
    let Ok(xml) = std::str::from_utf8(bytes) else {
        return false;
    };
    let mut reader = NsReader::from_str(xml);
    let mut buffer = Vec::new();

    loop {
        let Ok((namespace, event)) = reader.read_resolved_event_into(&mut buffer) else {
            return false;
        };
        match &event {
            Event::Start(element) | Event::Empty(element) => {
                if invalid_namespace_resolution(namespace)
                    || element.attributes().any(|attribute| match attribute {
                        Ok(attribute) => {
                            invalid_namespace_resolution(reader.resolve_attribute(attribute.key).0)
                        }
                        Err(_) => true,
                    })
                {
                    return false;
                }
            }
            _ => {}
        }
        if matches!(event, Event::Eof) {
            return true;
        }
        buffer.clear();
    }
}

fn invalid_namespace_resolution(resolution: ResolveResult<'_>) -> bool {
    matches!(
        resolution,
        ResolveResult::Unbound | ResolveResult::Unknown(_)
    )
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[derive(Clone, Debug)]
pub struct SlideXml {
    body: String,
    alternate_content: Option<String>,
}

impl SlideXml {
    pub fn from_body(body: &str) -> Self {
        Self {
            body: body.to_owned(),
            alternate_content: None,
        }
    }

    pub fn with_alternate_content(mut self, alternate_content: &str) -> Self {
        self.alternate_content = Some(alternate_content.to_owned());
        self
    }

    pub fn build(self) -> String {
        let alternate_content = self.alternate_content.unwrap_or_default();
        format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
    {body}
    {alternate_content}
  </p:spTree></p:cSld>
</p:sld>"#,
            body = self.body,
        )
    }
}
