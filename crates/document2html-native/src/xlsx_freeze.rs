use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use zip::{ZipArchive, ZipWriter};

use crate::config::NativeBackendConfig;
use crate::workspace::TemporaryWorkspace;
use crate::xlsx_workbook::freeze_workbook_calculation;
use crate::{NativeError, NativeResult};

const MAX_ARCHIVE_ENTRIES: usize = 16_384;
const MAX_EOCD_SIZE: usize = 22 + u16::MAX as usize;
const WORKBOOK_PART: &str = "xl/workbook.xml";
const ZIP_EOCD_SIGNATURE: &[u8; 4] = b"PK\x05\x06";

pub(crate) fn freeze_xlsx_snapshot(
    converted: &Path,
    config: &NativeBackendConfig,
    workspace: &TemporaryWorkspace,
) -> NativeResult<PathBuf> {
    let frozen_dir = workspace.root().join("frozen");
    fs::create_dir(&frozen_dir)?;
    let frozen = frozen_dir.join("input.xlsx");
    freeze_xlsx_archive(converted, &frozen, config.max_output_bytes)?;
    Ok(frozen)
}

fn freeze_xlsx_archive(
    source: &Path,
    destination: &Path,
    max_output_bytes: u64,
) -> NativeResult<()> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(NativeError::UnsafeOutput(source.to_owned()));
    }
    if metadata.len() > max_output_bytes {
        return Err(NativeError::ResourceLimitExceeded {
            resource: "output",
            limit: max_output_bytes,
        });
    }

    let mut source = File::open(source)?;
    let declared_entries = declared_archive_entries(&mut source, metadata.len())?;
    source.seek(SeekFrom::Start(0))?;
    let mut archive = ZipArchive::new(source)
        .map_err(|_| malformed_error("converted XLSX is not a valid ZIP archive"))?;
    if declared_entries > MAX_ARCHIVE_ENTRIES {
        return malformed("converted XLSX has too many archive entries");
    }
    if declared_entries != archive.len() {
        return malformed("converted XLSX has duplicate archive entries");
    }

    let destination_file = File::create_new(destination)?;
    let mut writer = ZipWriter::new(destination_file);
    let mut names = std::collections::HashSet::with_capacity(archive.len());
    let mut total_uncompressed = 0_u64;
    let mut workbook_entries = 0_usize;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|_| malformed_error("converted XLSX has an unreadable archive entry"))?;
        let name = entry.name().to_owned();
        if !safe_archive_path(&name) || !names.insert(name.clone()) {
            return malformed("converted XLSX has an unsafe archive entry");
        }
        if !entry.is_file() || entry.is_symlink() {
            return malformed("converted XLSX has a non-regular archive entry");
        }
        total_uncompressed = total_uncompressed.checked_add(entry.size()).ok_or(
            NativeError::ResourceLimitExceeded {
                resource: "output",
                limit: max_output_bytes,
            },
        )?;
        if total_uncompressed > max_output_bytes {
            return Err(NativeError::ResourceLimitExceeded {
                resource: "output",
                limit: max_output_bytes,
            });
        }

        if name == WORKBOOK_PART {
            workbook_entries += 1;
            let workbook = read_bounded_entry(&mut entry, max_output_bytes)?;
            let frozen_workbook = freeze_workbook_calculation(&workbook)?;
            let frozen_size = u64::try_from(frozen_workbook.len()).map_err(|_| {
                NativeError::ResourceLimitExceeded {
                    resource: "output",
                    limit: max_output_bytes,
                }
            })?;
            total_uncompressed = total_uncompressed
                .checked_sub(entry.size())
                .and_then(|total| total.checked_add(frozen_size))
                .ok_or(NativeError::ResourceLimitExceeded {
                    resource: "output",
                    limit: max_output_bytes,
                })?;
            if total_uncompressed > max_output_bytes {
                return Err(NativeError::ResourceLimitExceeded {
                    resource: "output",
                    limit: max_output_bytes,
                });
            }
            writer
                .start_file(name, entry.options())
                .map_err(|_| malformed_error("could not create frozen workbook part"))?;
            writer.write_all(&frozen_workbook)?;
        } else {
            writer
                .raw_copy_file(entry)
                .map_err(|_| malformed_error("could not preserve converted XLSX entry"))?;
        }
    }

    if workbook_entries != 1 {
        return malformed("converted XLSX must contain exactly one xl/workbook.xml part");
    }
    let destination = writer
        .finish()
        .map_err(|_| malformed_error("could not finish frozen XLSX archive"))?;
    if destination.metadata()?.len() > max_output_bytes {
        return Err(NativeError::ResourceLimitExceeded {
            resource: "output",
            limit: max_output_bytes,
        });
    }
    Ok(())
}

fn declared_archive_entries(source: &mut File, length: u64) -> NativeResult<usize> {
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
            if signature != ZIP_EOCD_SIGNATURE || tail.len().saturating_sub(offset) < 22 {
                return None;
            }
            let comment_length = u16::from_le_bytes([tail[offset + 20], tail[offset + 21]]);
            (offset + 22 + usize::from(comment_length) == tail.len()).then_some(offset)
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
    Ok(usize::from(total_entries))
}

fn read_bounded_entry(entry: &mut zip::read::ZipFile<'_>, limit: u64) -> NativeResult<Vec<u8>> {
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
    entry
        .take(entry.size().saturating_add(1))
        .read_to_end(&mut content)?;
    if content.len() != capacity {
        return malformed("converted XLSX entry has an invalid declared size");
    }
    Ok(content)
}

fn safe_archive_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && path
            .split('/')
            .all(|component| !component.is_empty() && component != "." && component != "..")
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

#[cfg(test)]
#[path = "xlsx_freeze_tests.rs"]
mod tests;
