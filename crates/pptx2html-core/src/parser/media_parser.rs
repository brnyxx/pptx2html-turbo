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
    if !valid_image_bytes(content_type, &data) {
        return None;
    }
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

fn valid_image_bytes(content_type: &str, data: &[u8]) -> bool {
    match content_type {
        "image/png" => valid_png(data),
        "image/jpeg" => valid_jpeg(data),
        "image/gif" => {
            data.len() >= 14
                && matches!(data.get(..6), Some(b"GIF87a" | b"GIF89a"))
                && u16::from_le_bytes([data[6], data[7]]) > 0
                && u16::from_le_bytes([data[8], data[9]]) > 0
                && data[10..data.len() - 1].contains(&0x2c)
                && data.last() == Some(&0x3b)
        }
        "image/webp" => {
            data.len() >= 20
                && data.starts_with(b"RIFF")
                && data.get(8..12) == Some(&b"WEBP"[..])
                && matches!(data.get(12..16), Some(b"VP8 " | b"VP8L" | b"VP8X"))
                && u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize + 8
                    == data.len()
                && u32::from_le_bytes([data[16], data[17], data[18], data[19]]) > 0
                && u32::from_le_bytes([data[16], data[17], data[18], data[19]]) as usize
                    <= data.len() - 20
        }
        _ => false,
    }
}

fn valid_png(data: &[u8]) -> bool {
    if !data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return false;
    }
    let mut offset = 8_usize;
    let mut first = true;
    let mut has_image_data = false;
    while offset.checked_add(12).is_some_and(|end| end <= data.len()) {
        let length = u32::from_be_bytes(match data[offset..offset + 4].try_into() {
            Ok(value) => value,
            Err(_) => return false,
        }) as usize;
        let Some(end) = offset
            .checked_add(12)
            .and_then(|base| base.checked_add(length))
        else {
            return false;
        };
        if end > data.len() {
            return false;
        }
        let kind = &data[offset + 4..offset + 8];
        if first {
            if kind != b"IHDR" || length != 13 {
                return false;
            }
            let width = u32::from_be_bytes(match data[offset + 8..offset + 12].try_into() {
                Ok(value) => value,
                Err(_) => return false,
            });
            let height = u32::from_be_bytes(match data[offset + 12..offset + 16].try_into() {
                Ok(value) => value,
                Err(_) => return false,
            });
            if width == 0 || height == 0 {
                return false;
            }
            first = false;
        } else if kind == b"IDAT" {
            has_image_data |= length > 0;
        } else if kind == b"IEND" {
            return length == 0 && has_image_data && end == data.len();
        }
        offset = end;
    }
    false
}

fn valid_jpeg(data: &[u8]) -> bool {
    if data.len() < 12 || !data.starts_with(&[0xff, 0xd8]) || !data.ends_with(&[0xff, 0xd9]) {
        return false;
    }
    let mut offset = 2_usize;
    let mut has_frame = false;
    while offset + 4 <= data.len() - 2 {
        if data[offset] != 0xff {
            return false;
        }
        let marker = data[offset + 1];
        offset += 2;
        if marker == 0xd8 || marker == 0xd9 || marker == 0x00 {
            return false;
        }
        let length = u16::from_be_bytes([data[offset], data[offset + 1]]) as usize;
        if length < 2 || offset + length > data.len() - 2 {
            return false;
        }
        if marker == 0xda {
            return has_frame && offset + length < data.len() - 2;
        }
        has_frame |= matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf);
        offset += length;
    }
    false
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
    if top_level
        .first()
        .is_none_or(|ftyp| ftyp.kind != *b"ftyp" || ftyp.payload.len() < 8)
    {
        return false;
    }
    let mdat_ranges = top_level
        .iter()
        .filter(|item| item.kind == *b"mdat" && !item.payload.is_empty())
        .map(|item| item.payload_start..item.end)
        .collect::<Vec<_>>();
    if mdat_ranges.is_empty() {
        return false;
    }
    top_level
        .iter()
        .filter(|item| item.kind == *b"moov")
        .flat_map(|moov| avc_tracks(moov.payload, moov.payload_start))
        .any(|track| valid_avc_samples(data, &mdat_ranges, &track))
}

#[derive(Clone, Copy)]
struct IsoBox<'a> {
    kind: [u8; 4],
    payload: &'a [u8],
    payload_start: usize,
    end: usize,
}

fn boxes(data: &[u8]) -> Option<Vec<IsoBox<'_>>> {
    boxes_at(data, 0)
}

fn boxes_at(mut data: &[u8], mut absolute: usize) -> Option<Vec<IsoBox<'_>>> {
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
        let payload_start = absolute.checked_add(header)?;
        let end = absolute.checked_add(size)?;
        result.push(IsoBox {
            kind,
            payload: &data[header..size],
            payload_start,
            end,
        });
        data = &data[size..];
        absolute = end;
    }
    Some(result)
}

#[derive(Clone)]
struct Sps {
    id: u32,
    width_mbs: u32,
    height_mbs: u32,
    picture_mbs: u32,
}

#[derive(Clone)]
struct Pps {
    id: u32,
    sps_id: u32,
}

#[derive(Clone)]
struct AvcConfig {
    sample_description_index: u32,
    length_size: usize,
    sps: Vec<Sps>,
    pps: Vec<Pps>,
}

#[derive(Clone)]
struct AvcTrack {
    config: AvcConfig,
    sample_ranges: Vec<std::ops::Range<usize>>,
}

fn avc_tracks(moov: &[u8], moov_start: usize) -> Vec<AvcTrack> {
    boxes_at(moov, moov_start)
        .unwrap_or_default()
        .into_iter()
        .filter(|item| item.kind == *b"trak")
        .filter_map(|trak| {
            child_boxes(trak.payload, trak.payload_start, b"mdia").find_map(|mdia| {
                child_boxes(mdia.payload, mdia.payload_start, b"minf").find_map(|minf| {
                    child_boxes(minf.payload, minf.payload_start, b"stbl").find_map(|stbl| {
                        let children = boxes_at(stbl.payload, stbl.payload_start)?;
                        let config = avc_config(single_box(&children, &[*b"stsd"])?.payload)?;
                        let sizes = sample_sizes(single_box(&children, &[*b"stsz"])?.payload)?;
                        let chunk_table = single_box(&children, &[*b"stco", *b"co64"])?;
                        let chunks = chunk_offsets(
                            chunk_table.payload,
                            if chunk_table.kind == *b"stco" { 4 } else { 8 },
                        )?;
                        let stsc = sample_to_chunk(single_box(&children, &[*b"stsc"])?.payload)?;
                        let sample_ranges = sample_ranges(&sizes, &chunks, &stsc, &config)?;
                        Some(AvcTrack {
                            config,
                            sample_ranges,
                        })
                    })
                })
            })
        })
        .collect()
}

fn single_box<'a>(items: &[IsoBox<'a>], kinds: &[[u8; 4]]) -> Option<IsoBox<'a>> {
    let mut matches = items
        .iter()
        .copied()
        .filter(|item| kinds.contains(&item.kind));
    let item = matches.next()?;
    matches.next().is_none().then_some(item)
}

fn sample_sizes(payload: &[u8]) -> Option<Vec<usize>> {
    if payload.len() < 12 {
        return None;
    }
    let sample_size = u32::from_be_bytes(payload[4..8].try_into().ok()?) as usize;
    let sample_count = u32::from_be_bytes(payload[8..12].try_into().ok()?) as usize;
    if sample_count == 0 || sample_count > 4096 {
        return None;
    }
    let sizes = if sample_size > 0 {
        vec![sample_size; sample_count]
    } else {
        let expected = 12_usize.checked_add(sample_count.checked_mul(4)?)?;
        if payload.len() != expected {
            return None;
        }
        payload[12..]
            .chunks_exact(4)
            .map(|raw| {
                usize::try_from(u32::from_be_bytes(raw.try_into().ok()?))
                    .ok()
                    .filter(|size| *size > 0)
            })
            .collect::<Option<Vec<_>>>()?
    };
    sizes
        .iter()
        .try_fold(0_usize, |total, size| total.checked_add(*size))
        .filter(|total| *total <= MAX_MEDIA_BYTES as usize)?;
    Some(sizes)
}

fn chunk_offsets(payload: &[u8], width: usize) -> Option<Vec<usize>> {
    if payload.len() < 8 {
        return None;
    }
    let count = u32::from_be_bytes(payload[4..8].try_into().ok()?) as usize;
    if count == 0
        || count > 4096
        || payload.len() != 8_usize.checked_add(count.checked_mul(width)?)?
    {
        return None;
    }
    payload[8..]
        .chunks_exact(width)
        .map(|raw| {
            let value = if width == 4 {
                u64::from(u32::from_be_bytes(raw.try_into().ok()?))
            } else {
                u64::from_be_bytes(raw.try_into().ok()?)
            };
            usize::try_from(value).ok()
        })
        .collect()
}

#[derive(Clone, Copy)]
struct StscEntry {
    first_chunk: usize,
    samples_per_chunk: usize,
    sample_description_index: u32,
}

fn sample_to_chunk(payload: &[u8]) -> Option<Vec<StscEntry>> {
    if payload.len() < 8 {
        return None;
    }
    let count = u32::from_be_bytes(payload[4..8].try_into().ok()?) as usize;
    if count == 0 || count > 4096 || payload.len() != 8_usize.checked_add(count.checked_mul(12)?)? {
        return None;
    }
    let entries = payload[8..]
        .chunks_exact(12)
        .map(|raw| {
            Some(StscEntry {
                first_chunk: usize::try_from(u32::from_be_bytes(raw[0..4].try_into().ok()?))
                    .ok()?,
                samples_per_chunk: usize::try_from(u32::from_be_bytes(raw[4..8].try_into().ok()?))
                    .ok()?,
                sample_description_index: u32::from_be_bytes(raw[8..12].try_into().ok()?),
            })
        })
        .collect::<Option<Vec<_>>>()?;
    if entries.first()?.first_chunk != 1
        || entries
            .iter()
            .any(|entry| entry.samples_per_chunk == 0 || entry.sample_description_index == 0)
        || entries
            .windows(2)
            .any(|pair| pair[0].first_chunk >= pair[1].first_chunk)
    {
        return None;
    }
    Some(entries)
}

fn sample_ranges(
    sizes: &[usize],
    chunks: &[usize],
    stsc: &[StscEntry],
    config: &AvcConfig,
) -> Option<Vec<std::ops::Range<usize>>> {
    if chunks.is_empty() || stsc.last()?.first_chunk > chunks.len() {
        return None;
    }
    let mut ranges = Vec::with_capacity(sizes.len());
    let mut sample_index = 0_usize;
    let mut stsc_index = 0_usize;
    for (chunk_index, chunk_offset) in chunks.iter().copied().enumerate() {
        let chunk_number = chunk_index + 1;
        while stsc_index + 1 < stsc.len() && stsc[stsc_index + 1].first_chunk <= chunk_number {
            stsc_index += 1;
        }
        let entry = stsc.get(stsc_index)?;
        if entry.sample_description_index != config.sample_description_index {
            return None;
        }
        let mut offset = chunk_offset;
        for _ in 0..entry.samples_per_chunk {
            let size = *sizes.get(sample_index)?;
            let end = offset.checked_add(size)?;
            ranges.push(offset..end);
            offset = end;
            sample_index += 1;
        }
    }
    (sample_index == sizes.len()).then_some(ranges)
}

fn valid_avc_samples(
    data: &[u8],
    mdat_ranges: &[std::ops::Range<usize>],
    track: &AvcTrack,
) -> bool {
    let Some(range) = track
        .sample_ranges
        .first()
        .filter(|_| track.sample_ranges.len() == 1)
    else {
        return false;
    };
    if !mdat_ranges
        .iter()
        .any(|mdat| range.start >= mdat.start && range.end <= mdat.end)
    {
        return false;
    }
    let Some(sample) = data.get(range.clone()) else {
        return false;
    };
    let Some(length_bytes) = sample.get(..track.config.length_size) else {
        return false;
    };
    let nal_size = length_bytes
        .iter()
        .fold(0_usize, |value, byte| (value << 8) | usize::from(*byte));
    let Some(nal) = sample.get(track.config.length_size..) else {
        return false;
    };
    nal_size == nal.len()
        && nal.first().is_some_and(|header| *header == 0x65)
        && valid_slice(nal, &track.config)
}

fn child_boxes<'a>(
    data: &'a [u8],
    absolute: usize,
    kind: &'static [u8; 4],
) -> impl Iterator<Item = IsoBox<'a>> {
    boxes_at(data, absolute)
        .unwrap_or_default()
        .into_iter()
        .filter(move |item| item.kind == *kind)
}

fn avc_config(payload: &[u8]) -> Option<AvcConfig> {
    if payload.len() < 8 {
        return None;
    }
    let entry_count = u32::from_be_bytes(payload[4..8].try_into().ok()?) as usize;
    let entries = boxes_at(&payload[8..], 0)?;
    if entry_count != entries.len() {
        return None;
    }
    entries.into_iter().enumerate().find_map(|(index, entry)| {
        if !matches!(
            entry.kind,
            [b'a', b'v', b'c', b'1'] | [b'a', b'v', b'c', b'3']
        ) || entry.payload.len() < 78
        {
            return None;
        }
        boxes_at(&entry.payload[78..], 0)?
            .into_iter()
            .find(|child| child.kind == *b"avcC")
            .and_then(|child| {
                let width = u16::from_be_bytes(entry.payload[24..26].try_into().ok()?);
                let height = u16::from_be_bytes(entry.payload[26..28].try_into().ok()?);
                valid_avcc(child.payload, u32::try_from(index + 1).ok()?, width, height)
            })
    })
}

fn valid_avcc(
    data: &[u8],
    sample_description_index: u32,
    width: u16,
    height: u16,
) -> Option<AvcConfig> {
    if data.len() < 7
        || data[0] != 1
        || data[1] != 66
        || data[2] & 0xc3 != 0xc0
        || data[3] != 30
        || data[4] != 0xff
        || data[5] != 0xe1
    {
        return None;
    }
    let length_size = usize::from((data[4] & 0x03) + 1);
    if length_size == 3 {
        return None;
    }
    let mut offset = 6_usize;
    let mut sps = Vec::with_capacity(1);
    for _ in 0..1 {
        let (nal, end) = parameter_set(data, offset, 7)?;
        let parsed = parse_sps(nal)?;
        if nal.get(1).copied()? != data[1]
            || nal.get(2).copied()? != data[2]
            || nal.get(3).copied()? != data[3]
            || sps.iter().any(|known: &Sps| known.id == parsed.id)
        {
            return None;
        }
        if u32::from(width) != parsed.width_mbs.checked_mul(16)?
            || u32::from(height) != parsed.height_mbs.checked_mul(16)?
        {
            return None;
        }
        sps.push(parsed);
        offset = end;
    }
    if *data.get(offset)? != 1 {
        return None;
    }
    offset += 1;
    let mut pps = Vec::with_capacity(1);
    for _ in 0..1 {
        let (nal, end) = parameter_set(data, offset, 8)?;
        let parsed = parse_pps(nal, &sps)?;
        if pps.iter().any(|known: &Pps| known.id == parsed.id) {
            return None;
        }
        pps.push(parsed);
        offset = end;
    }
    (offset == data.len()).then_some(AvcConfig {
        sample_description_index,
        length_size,
        sps,
        pps,
    })
}

fn parameter_set(data: &[u8], offset: usize, expected_nal_type: u8) -> Option<(&[u8], usize)> {
    let length_end = offset.checked_add(2)?;
    let length = usize::from(u16::from_be_bytes(
        data.get(offset..length_end)?.try_into().ok()?,
    ));
    let end = length_end.checked_add(length).filter(|_| length > 1)?;
    let nal = data.get(length_end..end)?;
    (nal[0] & 0x80 == 0 && (nal[0] & 0x1f) == expected_nal_type).then_some((nal, end))
}

fn nal_rbsp(nal: &[u8]) -> Option<Vec<u8>> {
    if nal.len() < 2 || nal.len() > 65_535 {
        return None;
    }
    let mut rbsp = Vec::with_capacity(nal.len() - 1);
    let payload = &nal[1..];
    let mut zeros = 0_u8;
    let mut index = 0_usize;
    while index < payload.len() {
        let byte = payload[index];
        if zeros >= 2 {
            if byte == 3 {
                if payload.get(index + 1).is_none_or(|next| *next > 3) {
                    return None;
                }
                zeros = 0;
                index += 1;
                continue;
            }
            if byte <= 2 {
                return None;
            }
        }
        rbsp.push(byte);
        zeros = if byte == 0 {
            zeros.saturating_add(1)
        } else {
            0
        };
        index += 1;
    }
    (!rbsp.is_empty() && rbsp.last().copied() != Some(0)).then_some(rbsp)
}

struct BitReader<'a> {
    data: &'a [u8],
    bit: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, bit: 0 }
    }

    fn read(&mut self, count: usize) -> Option<u32> {
        if count > 32 || self.bit.checked_add(count)? > self.data.len().checked_mul(8)? {
            return None;
        }
        let mut value = 0_u32;
        for _ in 0..count {
            value = (value << 1) | u32::from((self.data[self.bit / 8] >> (7 - self.bit % 8)) & 1);
            self.bit += 1;
        }
        Some(value)
    }

    fn ue(&mut self) -> Option<u32> {
        let mut zeros = 0_usize;
        while self.read(1)? == 0 {
            zeros += 1;
            if zeros > 31 {
                return None;
            }
        }
        let suffix = self.read(zeros)?;
        ((1_u32.checked_shl(u32::try_from(zeros).ok()?)?).checked_sub(1)?).checked_add(suffix)
    }

    fn se(&mut self) -> Option<i32> {
        let code = self.ue()?;
        let magnitude = i32::try_from(code.div_ceil(2)).ok()?;
        Some(if code & 1 == 0 { -magnitude } else { magnitude })
    }

    fn remaining(&self) -> usize {
        self.data.len().saturating_mul(8).saturating_sub(self.bit)
    }

    fn trailing_bits(&mut self) -> Option<()> {
        (self.read(1)? == 1).then_some(())?;
        while !self.bit.is_multiple_of(8) {
            (self.read(1)? == 0).then_some(())?;
        }
        (self.remaining() == 0).then_some(())
    }
}

fn parse_sps(nal: &[u8]) -> Option<Sps> {
    let rbsp = nal_rbsp(nal)?;
    let mut bits = BitReader::new(&rbsp);
    let profile = bits.read(8)? as u8;
    let constraints = bits.read(8)? as u8;
    let level = bits.read(8)? as u8;
    if profile != 66 || constraints != 0xc0 || level != 30 {
        return None;
    }
    let id = bits.ue()?;
    if id != 0 || bits.ue()? != 0 || bits.ue()? != 0 || bits.ue()? != 0 || bits.ue()? != 1 {
        return None;
    }
    if bits.read(1)? != 0 {
        return None;
    }
    let width_mbs = bits.ue()?.filter_max(15)?.checked_add(1)?;
    let height_mbs = bits.ue()?.filter_max(15)?.checked_add(1)?;
    if bits.read(1)? != 1 || bits.read(1)? != 1 {
        return None;
    }
    if bits.read(1)? != 0 || bits.read(1)? != 0 {
        return None;
    }
    bits.trailing_bits()?;
    let picture_mbs = width_mbs.checked_mul(height_mbs)?;
    Some(Sps {
        id,
        width_mbs,
        height_mbs,
        picture_mbs,
    })
}

fn parse_pps(nal: &[u8], sps: &[Sps]) -> Option<Pps> {
    let rbsp = nal_rbsp(nal)?;
    let mut bits = BitReader::new(&rbsp);
    let id = bits.ue()?;
    let sps_id = bits.ue()?;
    if id != 0 || sps_id != 0 || sps.iter().all(|item| item.id != sps_id) {
        return None;
    }
    if bits.read(1)? != 0
        || bits.read(1)? != 0
        || bits.ue()? != 0
        || bits.ue()? != 0
        || bits.ue()? != 0
        || bits.read(1)? != 0
        || bits.read(2)? != 0
        || bits.se()? != 0
        || bits.se()? != 0
        || bits.se()? != 0
        || bits.read(1)? != 0
        || bits.read(1)? != 0
        || bits.read(1)? != 0
    {
        return None;
    }
    bits.trailing_bits()?;
    Some(Pps { id, sps_id })
}

fn valid_slice(nal: &[u8], config: &AvcConfig) -> bool {
    let Some(rbsp) = nal_rbsp(nal) else {
        return false;
    };
    let mut bits = BitReader::new(&rbsp);
    let result = (|| {
        if nal[0] & 0x1f != 5 || nal[0] >> 5 == 0 || bits.ue()? != 0 {
            return None;
        }
        if bits.ue()? != 7 {
            return None;
        }
        let pps_id = bits.ue()?;
        let pps = config.pps.iter().find(|item| item.id == pps_id)?;
        let sps = config.sps.iter().find(|item| item.id == pps.sps_id)?;
        if bits.read(4)? != 0
            || bits.ue()? != 0
            || bits.read(4)? != 0
            || bits.read(1)? != 0
            || bits.read(1)? != 0
            || bits.se()? != 0
        {
            return None;
        }
        for _ in 0..sps.picture_mbs {
            if bits.ue()? != 25 {
                return None;
            }
            while !bits.bit.is_multiple_of(8) {
                if bits.read(1)? != 0 {
                    return None;
                }
            }
            for _ in 0..384 {
                bits.read(8)?;
            }
        }
        bits.trailing_bits()
    })();
    result.is_some()
}

trait FilterMax: Sized + PartialOrd {
    fn filter_max(self, maximum: Self) -> Option<Self> {
        (self <= maximum).then_some(self)
    }
}
impl FilterMax for u32 {}

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
    use super::{chunk_offsets, is_avc_mp4, is_pcm_wav, resolve_owner_relative};

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
    fn chunk_offset_tables_accept_bounded_stco_and_co64_entries() {
        let mut stco = vec![0_u8; 4];
        stco.extend_from_slice(&2_u32.to_be_bytes());
        stco.extend_from_slice(&100_u32.to_be_bytes());
        stco.extend_from_slice(&200_u32.to_be_bytes());
        assert_eq!(chunk_offsets(&stco, 4), Some(vec![100, 200]));

        let mut co64 = vec![0_u8; 4];
        co64.extend_from_slice(&2_u32.to_be_bytes());
        co64.extend_from_slice(&100_u64.to_be_bytes());
        co64.extend_from_slice(&200_u64.to_be_bytes());
        assert_eq!(chunk_offsets(&co64, 8), Some(vec![100, 200]));
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
        let avcc = iso_box(
            b"avcC",
            &[
                1, 66, 0, 10, 0xff, 0xe1, 0, 2, 0x67, 0x01, 1, 0, 2, 0x68, 0x01,
            ],
        );
        let mut sample = vec![0_u8; 78];
        sample.extend_from_slice(&avcc);
        let entry = iso_box(b"avc1", &sample);
        let mut stsd_payload = vec![0_u8; 4];
        stsd_payload.extend_from_slice(&1_u32.to_be_bytes());
        stsd_payload.extend_from_slice(&entry);
        let stsd = iso_box(b"stsd", &stsd_payload);
        let mut stsz_payload = vec![0_u8; 4];
        stsz_payload.extend_from_slice(&0_u32.to_be_bytes());
        stsz_payload.extend_from_slice(&1_u32.to_be_bytes());
        stsz_payload.extend_from_slice(&5_u32.to_be_bytes());
        let stsz = iso_box(b"stsz", &stsz_payload);
        let mut stbl_payload = stsd.clone();
        stbl_payload.extend_from_slice(&stsz);
        let stbl = iso_box(b"stbl", &stbl_payload);
        let minf = iso_box(b"minf", &stbl);
        let mdia = iso_box(b"mdia", &minf);
        let trak = iso_box(b"trak", &mdia);
        let moov = iso_box(b"moov", &trak);
        let mut ftyp_payload = Vec::from(&b"isom\0\0\0\0"[..]);
        ftyp_payload.extend_from_slice(b"avc1");

        let mut metadata_only = iso_box(b"ftyp", &ftyp_payload);
        metadata_only.extend_from_slice(&moov);
        assert!(!is_avc_mp4(&metadata_only));

        let mut arbitrary_sample = metadata_only.clone();
        arbitrary_sample.extend_from_slice(&iso_box(b"mdat", b"NOPE!"));
        assert!(!is_avc_mp4(&arbitrary_sample));

        let mut forbidden_bit_sample = metadata_only.clone();
        forbidden_bit_sample.extend_from_slice(&iso_box(b"mdat", &[0, 0, 0, 1, 0xe5]));
        assert!(!is_avc_mp4(&forbidden_bit_sample));

        let mut missing_sample_tables = metadata_only;
        missing_sample_tables.extend_from_slice(&iso_box(b"mdat", &[0, 0, 0, 1, 0x65]));
        assert!(!is_avc_mp4(&missing_sample_tables));

        let mut wrong_nesting = iso_box(b"ftyp", &ftyp_payload);
        wrong_nesting.extend_from_slice(&iso_box(b"moov", &stsd));
        wrong_nesting.extend_from_slice(&iso_box(b"mdat", &[0, 0, 0, 1, 0x65]));
        assert!(!is_avc_mp4(&wrong_nesting));
    }
}
