use std::fs::{self, File};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use zip::{ZipArchive, ZipWriter};

use crate::config::NativeBackendConfig;
use crate::workspace::TemporaryWorkspace;
use crate::xlsx_workbook::freeze_workbook_calculation;
use crate::xlsx_zip::{declared_archive_entries, read_bounded_entry};
use crate::{NativeError, NativeResult};

const WORKBOOK_PART: &str = "xl/workbook.xml";

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
        let remaining = max_output_bytes.checked_sub(total_uncompressed).ok_or(
            NativeError::ResourceLimitExceeded {
                resource: "output",
                limit: max_output_bytes,
            },
        )?;
        let options = entry.options();
        let content = read_bounded_entry(&mut entry, remaining)?;
        let output = if name == WORKBOOK_PART {
            workbook_entries += 1;
            freeze_workbook_calculation(&content)?
        } else {
            content
        };
        let output_size =
            u64::try_from(output.len()).map_err(|_| NativeError::ResourceLimitExceeded {
                resource: "output",
                limit: max_output_bytes,
            })?;
        total_uncompressed = total_uncompressed.checked_add(output_size).ok_or(
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
        writer
            .start_file(name, options)
            .map_err(|_| malformed_error("could not create frozen XLSX entry"))?;
        writer.write_all(&output)?;
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
