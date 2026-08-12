use std::io::{Read, Seek};

use zip::ZipArchive;

use super::picture_bullet_parser::ContentTypes;
use super::preserved_parser::part_diagnostic;
use super::relationships::{Relationship, TargetMode};
use crate::model::{
    ConversionDiagnostic, FeatureFamily, Fill, ImageFill, MediaData, MediaFailure, MediaKind,
    Shape, ShapeType, Slide,
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
    if let ShapeType::Picture(picture) = &mut shape.shape_type
        && let Some(mut media) = MediaData::decode_marker(&picture.content_type)
    {
        let poster = resolve_poster(
            &picture.rel_id,
            owner_part,
            relationships,
            content_types,
            archive,
        );
        resolve_media(
            &mut media,
            owner_part,
            relationships,
            content_types,
            archive,
        );
        if let Some(poster) = poster {
            shape.fill = Fill::Image(poster);
        }
        picture.data = std::mem::take(&mut media.data);
        picture.content_type = media.encode_marker();
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
    let data = match read_bounded(archive, &path) {
        Ok(data) => data,
        Err(failure) => {
            media.failure = Some(failure);
            return;
        }
    };
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

fn resolve_poster<R: Read + Seek>(
    relationship_id: &str,
    owner_part: &str,
    relationships: &[Relationship],
    content_types: &ContentTypes,
    archive: &mut ZipArchive<R>,
) -> Option<ImageFill> {
    if relationship_id.is_empty() {
        return None;
    }
    let mut matches = relationships
        .iter()
        .filter(|relationship| relationship.id == relationship_id);
    let relationship = matches.next()?;
    if matches.next().is_some()
        || relationship.relationship_type
            != "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image"
        || !matches!(relationship.target_mode, TargetMode::Internal)
    {
        return None;
    }
    let path = resolve_owner_relative(owner_part, &relationship.target)?;
    let content_type = content_types.for_part(&path)?;
    if !matches!(
        content_type,
        "image/png" | "image/jpeg" | "image/gif" | "image/webp"
    ) {
        return None;
    }
    let data = read_bounded(archive, &path).ok()?;
    Some(ImageFill {
        rel_id: relationship_id.to_owned(),
        data,
        content_type: content_type.to_owned(),
    })
}

fn read_bounded<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    path: &str,
) -> Result<Vec<u8>, MediaFailure> {
    let entry = archive
        .by_name(path)
        .map_err(|_| MediaFailure::MissingPart)?;
    if entry.size() == 0 {
        return Err(MediaFailure::EmptyAsset);
    }
    if entry.size() > MAX_MEDIA_BYTES {
        return Err(MediaFailure::AssetTooLarge);
    }
    let mut data = Vec::with_capacity(entry.size() as usize);
    entry
        .take(MAX_MEDIA_BYTES + 1)
        .read_to_end(&mut data)
        .map_err(|_| MediaFailure::MissingPart)?;
    if data.len() as u64 > MAX_MEDIA_BYTES {
        return Err(MediaFailure::AssetTooLarge);
    }
    Ok(data)
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
    let Some(top_level) = boxes(data) else {
        return false;
    };
    let Some(ftyp) = top_level.first() else {
        return false;
    };
    if ftyp.kind != *b"ftyp" || ftyp.payload.len() < 8 {
        return false;
    }
    top_level
        .iter()
        .filter(|item| item.kind == *b"moov")
        .any(|moov| contains_avc_track(moov.payload))
}

#[derive(Clone, Copy)]
struct IsoBox<'a> {
    kind: [u8; 4],
    payload: &'a [u8],
}

fn boxes(mut data: &[u8]) -> Option<Vec<IsoBox<'_>>> {
    let mut result = Vec::new();
    while !data.is_empty() {
        if data.len() < 8 || result.len() >= 4096 {
            return None;
        }
        let size32 = u32::from_be_bytes(data[..4].try_into().ok()?) as u64;
        let kind = data[4..8].try_into().ok()?;
        let (header, size) = if size32 == 1 {
            if data.len() < 16 {
                return None;
            }
            (16_usize, u64::from_be_bytes(data[8..16].try_into().ok()?))
        } else if size32 == 0 {
            (8_usize, data.len() as u64)
        } else {
            (8_usize, size32)
        };
        let size = usize::try_from(size).ok()?;
        if size < header || size > data.len() {
            return None;
        }
        result.push(IsoBox {
            kind,
            payload: &data[header..size],
        });
        data = &data[size..];
    }
    Some(result)
}

fn contains_avc_track(moov: &[u8]) -> bool {
    boxes(moov).is_some_and(|tracks| {
        tracks
            .iter()
            .filter(|item| item.kind == *b"trak")
            .any(|trak| {
                child_boxes(trak.payload, b"mdia").any(|mdia| {
                    child_boxes(mdia.payload, b"minf").any(|minf| {
                        child_boxes(minf.payload, b"stbl").any(|stbl| {
                            child_boxes(stbl.payload, b"stsd")
                                .any(|stsd| valid_avc_stsd(stsd.payload))
                        })
                    })
                })
            })
    })
}

fn child_boxes<'a>(data: &'a [u8], kind: &'static [u8; 4]) -> impl Iterator<Item = IsoBox<'a>> {
    boxes(data)
        .unwrap_or_default()
        .into_iter()
        .filter(move |item| item.kind == *kind)
}

fn valid_avc_stsd(payload: &[u8]) -> bool {
    if payload.len() < 8 {
        return false;
    }
    let entry_count = u32::from_be_bytes(match payload[4..8].try_into() {
        Ok(value) => value,
        Err(_) => return false,
    }) as usize;
    let Some(entries) = boxes(&payload[8..]) else {
        return false;
    };
    entry_count == entries.len()
        && entries.iter().any(|entry| {
            matches!(
                entry.kind,
                [b'a', b'v', b'c', b'1'] | [b'a', b'v', b'c', b'3']
            ) && entry.payload.len() >= 78
                && boxes(&entry.payload[78..]).is_some_and(|children| {
                    children
                        .iter()
                        .any(|child| child.kind == *b"avcC" && !child.payload.is_empty())
                })
        })
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
        assert!(!is_avc_mp4(b"\0\0\0\x18ftypavc1avcCxxxx"));
    }

    #[test]
    fn iso_box_parser_requires_avc_sample_entry_with_avcc_child() {
        fn iso_box(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
            let mut data = Vec::new();
            data.extend_from_slice(&((payload.len() + 8) as u32).to_be_bytes());
            data.extend_from_slice(kind);
            data.extend_from_slice(payload);
            data
        }
        let avcc = iso_box(b"avcC", &[1, 2, 3]);
        let mut sample = vec![0_u8; 78];
        sample.extend_from_slice(&avcc);
        let entry = iso_box(b"avc1", &sample);
        let mut stsd_payload = vec![0_u8; 4];
        stsd_payload.extend_from_slice(&1_u32.to_be_bytes());
        stsd_payload.extend_from_slice(&entry);
        let stsd = iso_box(b"stsd", &stsd_payload);
        let stbl = iso_box(b"stbl", &stsd);
        let minf = iso_box(b"minf", &stbl);
        let mdia = iso_box(b"mdia", &minf);
        let trak = iso_box(b"trak", &mdia);
        let moov = iso_box(b"moov", &trak);
        let mut ftyp_payload = Vec::from(&b"isom\0\0\0\0"[..]);
        ftyp_payload.extend_from_slice(b"avc1");
        let mut file = iso_box(b"ftyp", &ftyp_payload);
        file.extend_from_slice(&moov);
        assert!(is_avc_mp4(&file));

        let mut wrong_nesting = iso_box(b"ftyp", &ftyp_payload);
        wrong_nesting.extend_from_slice(&iso_box(b"moov", &stsd));
        assert!(!is_avc_mp4(&wrong_nesting));
    }
}
