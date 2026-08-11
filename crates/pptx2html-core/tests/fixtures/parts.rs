#[derive(Clone, Debug)]
pub struct FeaturePart {
    pub(super) path: String,
    pub(super) content_type: String,
    pub(super) bytes: Vec<u8>,
}

impl FeaturePart {
    pub fn notes(xml: &str) -> Self {
        Self::xml(
            "ppt/notesSlides/notesSlide1.xml",
            "application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml",
            xml,
        )
    }

    pub fn comments(xml: &str) -> Self {
        Self::xml(
            "ppt/comments/comment1.xml",
            "application/vnd.openxmlformats-officedocument.presentationml.comments+xml",
            xml,
        )
    }

    pub fn media(name: &str, content_type: &str, bytes: &[u8]) -> Self {
        Self::extra(&format!("ppt/media/{name}"), content_type, bytes)
    }

    pub fn chart(xml: &str) -> Self {
        Self::xml(
            "ppt/charts/chart1.xml",
            "application/vnd.openxmlformats-officedocument.drawingml.chart+xml",
            xml,
        )
    }

    pub fn extra(path: &str, content_type: &str, bytes: &[u8]) -> Self {
        Self {
            path: path.to_owned(),
            content_type: content_type.to_owned(),
            bytes: bytes.to_vec(),
        }
    }

    fn xml(path: &str, content_type: &str, xml: &str) -> Self {
        Self {
            path: path.to_owned(),
            content_type: content_type.to_owned(),
            bytes: xml.as_bytes().to_vec(),
        }
    }
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
