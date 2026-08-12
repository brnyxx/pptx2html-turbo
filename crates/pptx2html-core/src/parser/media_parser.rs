use std::io::{Read, Seek};

use zip::ZipArchive;

use super::picture_bullet_parser::ContentTypes;
use super::preserved_parser::part_diagnostic;
use super::relationships::{Relationship, TargetMode};
use crate::model::{
    ConversionDiagnostic, FeatureFamily, MediaData, MediaFailure, MediaKind, Shape, ShapeType,
    Slide,
};

const AUDIO_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/audio";
const VIDEO_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/video";
pub(crate) const MAX_MEDIA_BYTES: u64 = 16 * 1024 * 1024;

pub(crate) fn resolve_slide<R: Read + Seek>(
    slide: &mut Slide,
    owner_part: &str,
    relationships: &[Relationship],
    content_types: &ContentTypes,
    archive: &mut ZipArchive<R>,
) {
    for shape in &mut slide.shapes {
        resolve_shape(shape, owner_part, relationships, content_types, archive);
    }
}

fn resolve_shape<R: Read + Seek>(
    shape: &mut Shape,
    owner_part: &str,
    relationships: &[Relationship],
    content_types: &ContentTypes,
    archive: &mut ZipArchive<R>,
) {
    if let Some(media) = shape.media.as_mut() {
        resolve_media(media, owner_part, relationships, content_types, archive);
    }
    if let ShapeType::Group(children, _) = &mut shape.shape_type {
        for child in children {
            resolve_shape(child, owner_part, relationships, content_types, archive);
        }
    }
}

fn resolve_media<R: Read + Seek>(
    media: &mut MediaData,
    owner_part: &str,
    relationships: &[Relationship],
    content_types: &ContentTypes,
    archive: &mut ZipArchive<R>,
) {
    let matches = relationships
        .iter()
        .filter(|relationship| relationship.id == media.relationship_id)
        .collect::<Vec<_>>();
    let Some(relationship) = matches.first().copied() else {
        media.failure = Some(MediaFailure::MissingRelationship);
        return;
    };
    if matches.len() != 1 {
        media.failure = Some(MediaFailure::DuplicateRelationship);
        return;
    }
    media.relationship_type = Some(relationship.relationship_type.clone());
    let expected_relationship = match media.kind {
        MediaKind::Audio => AUDIO_RELATIONSHIP,
        MediaKind::Video => VIDEO_RELATIONSHIP,
    };
    if relationship.relationship_type != expected_relationship {
        media.failure = Some(MediaFailure::WrongRelationshipType);
        return;
    }
    if !matches!(relationship.target_mode, TargetMode::Internal) {
        media.failure = Some(MediaFailure::ExternalTarget);
        return;
    }
    let Some(path) = resolve_owner_relative(owner_part, &relationship.target) else {
        media.failure = Some(MediaFailure::UnsafeTarget);
        return;
    };
    let Some(content_type) = content_types.for_part(&path) else {
        media.failure = Some(MediaFailure::MissingContentType);
        return;
    };
    media.content_type = Some(content_type.to_owned());
    let expected_content_type = match media.kind {
        MediaKind::Audio => "audio/wav",
        MediaKind::Video => "video/mp4",
    };
    if content_type != expected_content_type {
        media.failure = Some(MediaFailure::UnsupportedContentType);
        return;
    }
    let Ok(entry) = archive.by_name(&path) else {
        media.failure = Some(MediaFailure::MissingPart);
        return;
    };
    if entry.size() == 0 {
        media.failure = Some(MediaFailure::EmptyAsset);
        return;
    }
    if entry.size() > MAX_MEDIA_BYTES {
        media.failure = Some(MediaFailure::AssetTooLarge);
        return;
    }
    let mut data = Vec::with_capacity(entry.size() as usize);
    if entry
        .take(MAX_MEDIA_BYTES + 1)
        .read_to_end(&mut data)
        .is_err()
        || data.len() as u64 > MAX_MEDIA_BYTES
    {
        media.failure = Some(MediaFailure::AssetTooLarge);
        return;
    }
    let supported = match media.kind {
        MediaKind::Audio => is_pcm_wav(&data),
        MediaKind::Video => is_avc_mp4(&data),
    };
    if !supported {
        media.failure = Some(MediaFailure::UnsupportedCodec);
        return;
    }
    media.data = data;
    media.failure = None;
}

fn resolve_owner_relative(owner_part: &str, target: &str) -> Option<String> {
    if target.is_empty()
        || target.starts_with('/')
        || target.contains('\\')
        || target.contains('%')
        || target.contains(':')
    {
        return None;
    }
    let mut segments = owner_part
        .rsplit_once('/')?
        .0
        .split('/')
        .collect::<Vec<_>>();
    for segment in target.split('/') {
        match segment {
            "" | "." => return None,
            ".." => {
                segments.pop()?;
            }
            value => segments.push(value),
        }
    }
    let path = segments.join("/");
    path.starts_with("ppt/media/").then_some(path)
}

fn is_pcm_wav(data: &[u8]) -> bool {
    if data.len() < 44 || &data[..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return false;
    }
    let mut offset = 12_usize;
    let mut pcm = false;
    let mut samples = false;
    while offset.checked_add(8).is_some_and(|end| end <= data.len()) {
        let size = u32::from_le_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ]) as usize;
        let start = offset + 8;
        let Some(end) = start.checked_add(size) else {
            return false;
        };
        if end > data.len() {
            return false;
        }
        if &data[offset..offset + 4] == b"fmt " && size >= 16 {
            pcm = u16::from_le_bytes([data[start], data[start + 1]]) == 1;
        } else if &data[offset..offset + 4] == b"data" {
            samples = size > 0;
        }
        offset = end + (size & 1);
    }
    pcm && samples
}

fn is_avc_mp4(data: &[u8]) -> bool {
    if data.len() < 16 || &data[4..8] != b"ftyp" {
        return false;
    }
    data.windows(4)
        .any(|window| matches!(window, b"avc1" | b"avc3"))
        && data.windows(4).any(|window| window == b"avcC")
}

pub(crate) fn collect_part_diagnostics(
    part_name: &str,
    diagnostics: &mut Vec<ConversionDiagnostic>,
) {
    let extension = part_name.rsplit('.').next().unwrap_or("");
    if part_name.starts_with("ppt/media/") && matches!(extension, "mp3" | "m4a" | "avi" | "mov") {
        diagnostics.push(part_diagnostic(
            part_name,
            FeatureFamily::Media,
            "Media asset is outside the bounded PCM WAV and AVC MP4 subset",
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::{is_avc_mp4, is_pcm_wav, resolve_owner_relative};

    #[test]
    fn owner_relative_resolution_is_package_and_media_bounded() {
        assert_eq!(
            resolve_owner_relative("ppt/slides/slide1.xml", "../media/a.wav"),
            Some("ppt/media/a.wav".to_owned())
        );
        assert!(resolve_owner_relative("ppt/slides/slide1.xml", "../../../a.wav").is_none());
        assert!(resolve_owner_relative("ppt/slides/slide1.xml", "https://x/a.wav").is_none());
    }

    #[test]
    fn codec_sniffers_reject_labels_without_bounded_signatures() {
        assert!(!is_pcm_wav(b"RIFF fake WAVE"));
        assert!(!is_avc_mp4(b"....ftyp....mp4v...."));
    }
}
