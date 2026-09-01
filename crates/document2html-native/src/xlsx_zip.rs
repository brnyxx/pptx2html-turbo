use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

use crate::{NativeError, NativeResult};

const CENTRAL_DIRECTORY_HEADER_SIZE: u64 = 46;
const CENTRAL_DIRECTORY_SIGNATURE: &[u8; 4] = b"PK\x01\x02";
const EOCD_HEADER_SIZE: usize = 22;
const MAX_ARCHIVE_ENTRIES: usize = 16_384;
const MAX_EOCD_SIZE: usize = EOCD_HEADER_SIZE + u16::MAX as usize;
const ZIP_EOCD_SIGNATURE: &[u8; 4] = b"PK\x05\x06";

pub(super) fn declared_archive_entries(source: &mut File, length: u64) -> NativeResult<usize> {
    let tail_length = length.min(MAX_EOCD_SIZE as u64);
    let tail_capacity = usize::try_from(tail_length)
        .map_err(|_| malformed_error("converted XLSX ZIP metadata is too large"))?;
    let mut tail = vec![0_u8; tail_capacity];
    source.seek(SeekFrom::Start(length - tail_length))?;
    source.read_exact(&mut tail)?;

    let Some(eocd) = tail
        .windows(ZIP_EOCD_SIGNATURE.len())
        .enumerate()
        .rev()
        .find_map(|(offset, signature)| {
            if signature != ZIP_EOCD_SIGNATURE
                || tail.len().saturating_sub(offset) < EOCD_HEADER_SIZE
            {
                return None;
            }
            let comment_length = u16::from_le_bytes([tail[offset + 20], tail[offset + 21]]);
            (offset + EOCD_HEADER_SIZE + usize::from(comment_length) == tail.len())
                .then_some(offset)
        })
    else {
        return malformed("converted XLSX has no valid ZIP end record");
    };

    let disk = u16::from_le_bytes([tail[eocd + 4], tail[eocd + 5]]);
    let directory_disk = u16::from_le_bytes([tail[eocd + 6], tail[eocd + 7]]);
    let disk_entries = u16::from_le_bytes([tail[eocd + 8], tail[eocd + 9]]);
    let total_entries = u16::from_le_bytes([tail[eocd + 10], tail[eocd + 11]]);
    if disk != 0 || directory_disk != 0 || disk_entries != total_entries {
        return malformed("converted XLSX uses an unsupported multi-disk ZIP archive");
    }
    if total_entries == u16::MAX {
        return malformed("converted XLSX ZIP64 entry count exceeds the supported limit");
    }
    let declared = usize::from(total_entries);
    if declared > MAX_ARCHIVE_ENTRIES {
        return malformed("converted XLSX has too many archive entries");
    }

    let directory_size = u64::from(u32::from_le_bytes([
        tail[eocd + 12],
        tail[eocd + 13],
        tail[eocd + 14],
        tail[eocd + 15],
    ]));
    let directory_start = u64::from(u32::from_le_bytes([
        tail[eocd + 16],
        tail[eocd + 17],
        tail[eocd + 18],
        tail[eocd + 19],
    ]));
    let eocd_start = length - tail_length
        + u64::try_from(eocd)
            .map_err(|_| malformed_error("converted XLSX ZIP metadata is too large"))?;
    let Some(directory_end) = directory_start.checked_add(directory_size) else {
        return malformed("converted XLSX central directory range is invalid");
    };
    if directory_end != eocd_start {
        return malformed("converted XLSX central directory range is invalid");
    }

    let actual = central_directory_entries(source, directory_start, directory_end)?;
    if actual != declared {
        return malformed("converted XLSX central directory entry count is invalid");
    }
    Ok(declared)
}

pub(super) fn read_bounded_entry(
    entry: &mut zip::read::ZipFile<'_>,
    limit: u64,
) -> NativeResult<Vec<u8>> {
    if entry.size() > limit {
        return Err(NativeError::ResourceLimitExceeded {
            resource: "output",
            limit,
        });
    }
    let capacity =
        usize::try_from(entry.size()).map_err(|_| NativeError::ResourceLimitExceeded {
            resource: "output",
            limit,
        })?;
    let mut content = Vec::with_capacity(capacity);
    let mut buffer = [0_u8; 8_192];
    while content.len() <= capacity {
        let remaining = capacity.saturating_add(1) - content.len();
        let read_size = remaining.min(buffer.len());
        let count = entry
            .read(&mut buffer[..read_size])
            .map_err(|_| malformed_error("converted XLSX has an unreadable archive entry"))?;
        if count == 0 {
            break;
        }
        content.extend_from_slice(&buffer[..count]);
    }
    if content.len() != capacity {
        return malformed("converted XLSX entry has an invalid declared size");
    }
    Ok(content)
}

fn central_directory_entries(source: &mut File, start: u64, end: u64) -> NativeResult<usize> {
    source.seek(SeekFrom::Start(start))?;
    let mut position = start;
    let mut count = 0_usize;
    let mut header = [0_u8; CENTRAL_DIRECTORY_HEADER_SIZE as usize];
    while position < end {
        if end - position < CENTRAL_DIRECTORY_HEADER_SIZE {
            return malformed("converted XLSX central directory is truncated");
        }
        source.read_exact(&mut header)?;
        if &header[..4] != CENTRAL_DIRECTORY_SIGNATURE {
            return malformed("converted XLSX central directory has an invalid entry");
        }
        let variable_size = u64::from(u16::from_le_bytes([header[28], header[29]]))
            + u64::from(u16::from_le_bytes([header[30], header[31]]))
            + u64::from(u16::from_le_bytes([header[32], header[33]]));
        let Some(next) = position
            .checked_add(CENTRAL_DIRECTORY_HEADER_SIZE)
            .and_then(|value| value.checked_add(variable_size))
        else {
            return malformed("converted XLSX central directory range is invalid");
        };
        if next > end {
            return malformed("converted XLSX central directory is truncated");
        }
        count += 1;
        if count > MAX_ARCHIVE_ENTRIES {
            return malformed("converted XLSX has too many archive entries");
        }
        source.seek(SeekFrom::Start(next))?;
        position = next;
    }
    Ok(count)
}

fn malformed<T>(reason: &str) -> NativeResult<T> {
    Err(malformed_error(reason))
}

fn malformed_error(reason: &str) -> NativeError {
    NativeError::MalformedBackendOutput {
        backend: "libreoffice",
        reason: reason.to_owned(),
    }
}
