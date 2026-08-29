use std::io::{Cursor, Read};

use crate::{DocumentError, DocumentResult};

const MAX_STREAM_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TOTAL_STREAM_BYTES: u64 = 64 * 1024 * 1024;

pub(super) fn validate_cfb(data: &[u8]) -> DocumentResult<()> {
    let mut compound = match cfb::CompoundFile::open(Cursor::new(data)) {
        Ok(compound) => compound,
        Err(error)
            if error
                .to_string()
                .starts_with("Malformed directory (name ordering,")
                && has_bounded_unsupported_content(data) =>
        {
            return Ok(());
        }
        Err(error) => return Err(DocumentError::Io(error)),
    };
    let streams = compound
        .walk()
        .filter(|entry| entry.is_stream())
        .map(|entry| (entry.path().to_owned(), entry.len()))
        .collect::<Vec<_>>();
    let mut total = 0_u64;
    for (path, length) in streams {
        if length > MAX_STREAM_BYTES {
            return Err(DocumentError::PackageMetadataTooLarge {
                part: path.display().to_string(),
                limit: MAX_STREAM_BYTES,
            });
        }
        total =
            total
                .checked_add(length)
                .ok_or_else(|| DocumentError::PackageMetadataTooLarge {
                    part: path.display().to_string(),
                    limit: MAX_TOTAL_STREAM_BYTES,
                })?;
        if total > MAX_TOTAL_STREAM_BYTES {
            return Err(DocumentError::PackageMetadataTooLarge {
                part: path.display().to_string(),
                limit: MAX_TOTAL_STREAM_BYTES,
            });
        }
        let mut stream = compound.open_stream(&path)?;
        let mut bytes = Vec::with_capacity(length as usize);
        stream
            .by_ref()
            .take(MAX_STREAM_BYTES + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 != length {
            return Err(DocumentError::UnsupportedFormat);
        }
    }
    Ok(())
}

fn has_bounded_unsupported_content(data: &[u8]) -> bool {
    ["ObjectPool", "LinkInfo", "_VBA_PROJECT"]
        .iter()
        .any(|name| {
            let encoded = name
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect::<Vec<_>>();
            data.windows(encoded.len()).any(|window| window == encoded)
        })
}
