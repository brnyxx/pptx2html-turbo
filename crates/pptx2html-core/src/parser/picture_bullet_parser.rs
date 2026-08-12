use std::collections::HashMap;
use std::io::{Read, Seek};

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::reader::NsReader;
use zip::ZipArchive;

use super::graphic_frame_parser::{read_archive_bytes, resolve_relationship_path};
use super::relationships::{Relationship, TargetMode};
use super::xml_utils;
use crate::model::{
    Bullet, ListStyle, PictureBullet, PictureBulletFailure, PictureBulletImage,
    PictureBulletRelationshipMode, PictureBulletTargetMode, Shape, ShapeType, Slide, TextBody,
};

const IMAGE_RELATIONSHIP_SUFFIX: &str = "/image";

#[derive(Default)]
pub(crate) struct ContentTypes {
    defaults: HashMap<String, String>,
    overrides: HashMap<String, String>,
}

impl ContentTypes {
    pub(crate) fn parse(xml: &str) -> Self {
        let mut reader = NsReader::from_str(xml);
        let mut content_types = Self::default();
        let mut depth = 0_usize;
        let mut official_root = false;
        loop {
            match reader.read_resolved_event() {
                Ok((namespace, Event::Start(element))) => {
                    if depth == 0 {
                        official_root =
                            official_content_type_element(&namespace, &element, "Types");
                    }
                    depth += 1;
                }
                Ok((namespace, Event::Empty(element))) if depth == 1 && official_root => {
                    content_types.insert_element(&namespace, &element);
                }
                Ok((_, Event::End(_))) => depth = depth.saturating_sub(1),
                Ok((_, Event::Eof)) | Err(_) => break,
                _ => {}
            }
        }
        content_types
    }

    fn insert_element(
        &mut self,
        namespace: &ResolveResult<'_>,
        element: &quick_xml::events::BytesStart<'_>,
    ) {
        if official_content_type_element(namespace, element, "Default") {
            if let Some((extension, content_type)) =
                unqualified_content_type_attributes(element, "Extension")
            {
                self.defaults
                    .insert(extension.to_ascii_lowercase(), content_type);
            }
        } else if official_content_type_element(namespace, element, "Override")
            && let Some((part_name, content_type)) =
                unqualified_content_type_attributes(element, "PartName")
        {
            self.overrides
                .insert(part_name.trim_start_matches('/').to_owned(), content_type);
        }
    }

    pub(crate) fn for_part(&self, part_name: &str) -> Option<&str> {
        self.overrides
            .get(part_name)
            .map(String::as_str)
            .or_else(|| {
                let extension = part_name.rsplit_once('.')?.1.to_ascii_lowercase();
                self.defaults.get(&extension).map(String::as_str)
            })
    }
}

fn official_content_type_element(
    namespace: &ResolveResult<'_>,
    element: &quick_xml::events::BytesStart<'_>,
    local_name: &str,
) -> bool {
    matches!(namespace, ResolveResult::Bound(value) if value.as_ref() == b"http://schemas.openxmlformats.org/package/2006/content-types")
        && xml_utils::local_name(element.name().as_ref()) == local_name
}

fn unqualified_content_type_attributes(
    element: &quick_xml::events::BytesStart<'_>,
    key_name: &str,
) -> Option<(String, String)> {
    let mut key_value = None;
    let mut content_type = None;
    for attribute in element.attributes().flatten() {
        let key = std::str::from_utf8(attribute.key.as_ref()).ok()?;
        if key.contains(':') {
            return None;
        }
        let value = attribute.unescape_value().ok()?.into_owned();
        match key {
            key if key == key_name && key_value.is_none() => key_value = Some(value),
            "ContentType" if content_type.is_none() => content_type = Some(value),
            _ => {}
        }
    }
    Some((key_value?, content_type?))
}

pub(crate) fn resolve_slide<R: Read + Seek>(
    slide: &mut Slide,
    relationships: &[Relationship],
    content_types: &ContentTypes,
    archive: &mut ZipArchive<R>,
) {
    for shape in &mut slide.shapes {
        resolve_shape(shape, relationships, content_types, archive);
    }
}

fn resolve_shape<R: Read + Seek>(
    shape: &mut Shape,
    relationships: &[Relationship],
    content_types: &ContentTypes,
    archive: &mut ZipArchive<R>,
) {
    if let Some(text_body) = shape.text_body.as_mut() {
        resolve_text_body(text_body, relationships, content_types, archive);
    }
    if let ShapeType::Group(children, _) = &mut shape.shape_type {
        for child in children {
            resolve_shape(child, relationships, content_types, archive);
        }
    }
    if let ShapeType::Table(table) = &mut shape.shape_type {
        for cell in table.rows.iter_mut().flat_map(|row| &mut row.cells) {
            if let Some(text_body) = cell.text_body.as_mut() {
                resolve_text_body(text_body, relationships, content_types, archive);
            }
        }
    }
}

fn resolve_text_body<R: Read + Seek>(
    text_body: &mut TextBody,
    relationships: &[Relationship],
    content_types: &ContentTypes,
    archive: &mut ZipArchive<R>,
) {
    for paragraph in &mut text_body.paragraphs {
        if let Some(Bullet::Picture(picture)) = paragraph.bullet.as_mut() {
            resolve_picture(picture, relationships, content_types, archive);
        }
    }
    if let Some(list_style) = text_body.list_style.as_mut() {
        resolve_list_style(list_style, relationships, content_types, archive);
    }
}

fn resolve_list_style<R: Read + Seek>(
    list_style: &mut ListStyle,
    relationships: &[Relationship],
    content_types: &ContentTypes,
    archive: &mut ZipArchive<R>,
) {
    for defaults in list_style.levels.iter_mut().flatten() {
        if let Some(Bullet::Picture(picture)) = defaults.bullet.as_mut() {
            resolve_picture(picture, relationships, content_types, archive);
        }
    }
}

fn resolve_picture<R: Read + Seek>(
    picture: &mut PictureBullet,
    relationships: &[Relationship],
    content_types: &ContentTypes,
    archive: &mut ZipArchive<R>,
) {
    let Some(relationship) = relationships
        .iter()
        .find(|relationship| relationship.id == picture.relationship_id)
    else {
        return;
    };
    picture.relationship_type = Some(relationship.relationship_type.clone());
    picture.target_mode = Some(target_mode(&relationship.target_mode));
    if !relationship
        .relationship_type
        .ends_with(IMAGE_RELATIONSHIP_SUFFIX)
    {
        picture.failure = Some(PictureBulletFailure::WrongRelationshipKind);
        return;
    }
    if !mode_matches(picture.relationship_mode, &relationship.target_mode) {
        picture.failure = Some(PictureBulletFailure::WrongTargetMode);
        return;
    }
    if picture.relationship_mode == Some(PictureBulletRelationshipMode::Link) {
        picture.failure = Some(PictureBulletFailure::LinkedExternal);
        return;
    }
    let path = resolve_relationship_path("ppt/slides", &relationship.target);
    let Some(content_type) = content_types.for_part(&path) else {
        picture.failure = Some(PictureBulletFailure::MissingContentType);
        return;
    };
    if !supported_mime(content_type) {
        picture.failure = Some(PictureBulletFailure::UnsupportedContentType);
        return;
    }
    let Ok(data) = read_archive_bytes(archive, &path) else {
        picture.failure = Some(PictureBulletFailure::MissingPart);
        return;
    };
    if data.is_empty() {
        picture.failure = Some(PictureBulletFailure::EmptyImage);
        return;
    }
    picture.image = Some(PictureBulletImage {
        data,
        content_type: content_type.to_owned(),
    });
    picture.failure = None;
}

fn target_mode(mode: &TargetMode) -> PictureBulletTargetMode {
    match mode {
        TargetMode::Internal => PictureBulletTargetMode::Internal,
        TargetMode::External => PictureBulletTargetMode::External,
        TargetMode::Other(value) => PictureBulletTargetMode::Other(value.clone()),
    }
}

fn mode_matches(mode: Option<PictureBulletRelationshipMode>, target_mode: &TargetMode) -> bool {
    matches!(
        (mode, target_mode),
        (
            Some(PictureBulletRelationshipMode::Embed),
            TargetMode::Internal
        ) | (
            Some(PictureBulletRelationshipMode::Link),
            TargetMode::External
        )
    )
}

fn supported_mime(mime: &str) -> bool {
    matches!(
        mime,
        "image/png" | "image/jpeg" | "image/gif" | "image/webp"
    )
}

#[cfg(test)]
mod content_type_tests {
    use super::ContentTypes;

    #[test]
    fn requires_official_elements_and_unqualified_attributes() {
        let foreign_element = ContentTypes::parse(
            r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types" xmlns:x="urn:foreign"><x:Default Extension="wav" ContentType="audio/wav"/></Types>"#,
        );
        assert!(foreign_element.for_part("ppt/media/a.wav").is_none());

        let foreign_attribute = ContentTypes::parse(
            r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types" xmlns:x="urn:foreign"><Default x:Extension="wav" ContentType="audio/wav"/></Types>"#,
        );
        assert!(foreign_attribute.for_part("ppt/media/a.wav").is_none());

        let nested = ContentTypes::parse(
            r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><wrapper><Default Extension="wav" ContentType="audio/wav"/></wrapper></Types>"#,
        );
        assert!(nested.for_part("ppt/media/a.wav").is_none());

        let foreign_root = ContentTypes::parse(
            r#"<x:Types xmlns:x="urn:foreign" xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="wav" ContentType="audio/wav"/></x:Types>"#,
        );
        assert!(foreign_root.for_part("ppt/media/a.wav").is_none());

        let official = ContentTypes::parse(
            r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="wav" ContentType="audio/wav"/></Types>"#,
        );
        assert_eq!(official.for_part("ppt/media/a.wav"), Some("audio/wav"));
    }
}
