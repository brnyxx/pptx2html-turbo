use std::io::{self, Cursor, Read, Seek, SeekFrom};

use crate::{DocumentError, DocumentResult};

const MAX_STREAM_BYTES: u64 = 16 * 1024 * 1024;
const MAX_TOTAL_STREAM_BYTES: u64 = 64 * 1024 * 1024;
const MAX_COMPACT_VALIDATION_BYTES: usize = 64 * 1024 * 1024;

const CFBF_HEADER_LEN: usize = 512;
const DIFAT_OFFSET: usize = 76;
const DIFAT_ENTRIES: usize = 109;
const FREESECT: u32 = 0xffff_ffff;
const ENDOFCHAIN: u32 = 0xffff_fffe;
const FATSECT: u32 = 0xffff_fffd;

pub(super) fn validate_cfb(data: &[u8]) -> DocumentResult<()> {
    match cfb::CompoundFile::open(Cursor::new(data)) {
        Ok(mut compound) => validate_streams(&mut compound),
        Err(error) if is_compact_fat_error(&error, data) => {
            let padded = compact_fat_validation_reader(data).ok_or(DocumentError::Io(error))?;
            let mut compound = cfb::CompoundFile::open(padded)?;
            validate_streams(&mut compound)
        }
        Err(error)
            if error
                .to_string()
                .starts_with("Malformed directory (name ordering,")
                && has_bounded_unsupported_content(data) =>
        {
            Ok(())
        }
        Err(error) => Err(DocumentError::Io(error)),
    }
}

fn validate_streams<F: Read + std::io::Seek>(
    compound: &mut cfb::CompoundFile<F>,
) -> DocumentResult<()> {
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

fn is_compact_fat_error(error: &std::io::Error, data: &[u8]) -> bool {
    let Some(header) = compact_fat_header(data) else {
        return false;
    };
    error.to_string()
        == format!(
            "Malformed FAT (FAT has {} entries, but file has only {} sectors)",
            header.fat_entry_capacity, header.physical_sector_count
        )
}

fn compact_fat_validation_reader(data: &[u8]) -> Option<CompactFatReader> {
    let header = compact_fat_header(data)?;
    let padded_len = header
        .sector_size
        .checked_mul(header.fat_entry_capacity.checked_add(1)?)?;
    if padded_len > MAX_COMPACT_VALIDATION_BYTES {
        return None;
    }
    Some(CompactFatReader {
        data: data.to_vec(),
        position: 0,
        logical_len: u64::try_from(padded_len).ok()?,
    })
}

struct CompactFatReader {
    data: Vec<u8>,
    position: u64,
    logical_len: u64,
}

impl Read for CompactFatReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let position = usize::try_from(self.position)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "position exceeds usize"))?;
        if position >= self.data.len() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "compact CFBF references a physically absent sector",
            ));
        }
        let count = buffer.len().min(self.data.len() - position);
        buffer[..count].copy_from_slice(&self.data[position..position + count]);
        self.position += u64::try_from(count).expect("read count fits u64");
        Ok(count)
    }
}

impl Seek for CompactFatReader {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let target = match position {
            SeekFrom::Start(value) => i128::from(value),
            SeekFrom::End(value) => i128::from(self.logical_len) + i128::from(value),
            SeekFrom::Current(value) => i128::from(self.position) + i128::from(value),
        };
        if target < 0 || target > i128::from(self.logical_len) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "seek escapes compact CFBF logical bounds",
            ));
        }
        self.position = u64::try_from(target).expect("nonnegative target fits u64");
        Ok(self.position)
    }
}

struct CompactFatHeader {
    sector_size: usize,
    physical_sector_count: usize,
    fat_entry_capacity: usize,
    fat_sector_ids: Vec<usize>,
}

fn compact_fat_header(data: &[u8]) -> Option<CompactFatHeader> {
    let header = compact_fat_header_base(data)?;
    if compact_tail_value(data, &header)? != ENDOFCHAIN {
        return None;
    }
    Some(header)
}

fn compact_fat_header_base(data: &[u8]) -> Option<CompactFatHeader> {
    if data.len() < CFBF_HEADER_LEN || data.get(..8)? != cfbf_magic() {
        return None;
    }
    let version = read_u16(data, 26)?;
    let sector_size = match (version, read_u16(data, 30)?) {
        (3, 9) => 512,
        (4, 12) => 4096,
        _ => return None,
    };
    if read_u16(data, 28)? != 0xfffe
        || read_u16(data, 32)? != 6
        || read_u32(data, 56)? != 4096
        || (version == 3 && read_u32(data, 40)? != 0)
        || !data.len().is_multiple_of(sector_size)
        || data.len() < sector_size
    {
        return None;
    }

    let physical_sector_count = data.len() / sector_size - 1;
    let num_fat_sectors = usize::try_from(read_u32(data, 44)?).ok()?;
    if num_fat_sectors == 0 || num_fat_sectors > DIFAT_ENTRIES {
        return None;
    }
    let first_difat_sector = read_u32(data, 68)?;
    if read_u32(data, 72)? != 0
        || (first_difat_sector != ENDOFCHAIN && first_difat_sector != FREESECT)
    {
        return None;
    }

    let mut fat_sector_ids = Vec::with_capacity(num_fat_sectors);
    for index in 0..DIFAT_ENTRIES {
        let sector_id = read_u32(data, DIFAT_OFFSET + index * 4)?;
        if index < num_fat_sectors {
            let sector_id = usize::try_from(sector_id).ok()?;
            if sector_id >= physical_sector_count || fat_sector_ids.contains(&sector_id) {
                return None;
            }
            fat_sector_ids.push(sector_id);
        } else if sector_id != FREESECT {
            return None;
        }
    }

    let entries_per_fat_sector = sector_size / 4;
    let fat_entry_capacity = num_fat_sectors.checked_mul(entries_per_fat_sector)?;
    if physical_sector_count >= fat_entry_capacity {
        return None;
    }
    for fat_entry_index in 0..physical_sector_count {
        let fat_sector_id = fat_sector_ids[fat_entry_index / entries_per_fat_sector];
        let value = fat_entry(
            data,
            sector_size,
            fat_sector_id,
            fat_entry_index % entries_per_fat_sector,
        )?;
        if value <= 0xffff_fffa && usize::try_from(value).ok()? >= physical_sector_count {
            return None;
        }
    }
    for &fat_sector_id in &fat_sector_ids {
        if fat_entry(data, sector_size, fat_sector_id, fat_sector_id)? != FATSECT {
            return None;
        }
    }

    Some(CompactFatHeader {
        sector_size,
        physical_sector_count,
        fat_entry_capacity,
        fat_sector_ids,
    })
}

fn compact_tail_value(data: &[u8], header: &CompactFatHeader) -> Option<u32> {
    let entries_per_fat_sector = header.sector_size / 4;
    let first = header.physical_sector_count;
    let first_fat_sector = header.fat_sector_ids[first / entries_per_fat_sector];
    let expected = fat_entry(
        data,
        header.sector_size,
        first_fat_sector,
        first % entries_per_fat_sector,
    )?;
    for fat_entry_index in first + 1..header.fat_entry_capacity {
        let fat_sector_id = header.fat_sector_ids[fat_entry_index / entries_per_fat_sector];
        if fat_entry(
            data,
            header.sector_size,
            fat_sector_id,
            fat_entry_index % entries_per_fat_sector,
        )? != expected
        {
            return None;
        }
    }
    Some(expected)
}

fn fat_entry(
    data: &[u8],
    sector_size: usize,
    fat_sector_id: usize,
    entry_index: usize,
) -> Option<u32> {
    let sector_offset = fat_sector_id.checked_add(1)?.checked_mul(sector_size)?;
    read_u32(
        data,
        sector_offset.checked_add(entry_index.checked_mul(4)?)?,
    )
}

fn read_u16(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        data.get(offset..offset.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        data.get(offset..offset.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn cfbf_magic() -> &'static [u8] {
    &[0xd0, 0xcf, 0x11, 0xe0, 0xa1, 0xb1, 0x1a, 0xe1]
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
