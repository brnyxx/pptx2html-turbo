use std::fs;
use std::path::Path;

use crate::{NativeError, NativeResult};

#[derive(Clone, Copy)]
pub(super) enum OutputScan {
    Live,
    Final,
}

pub(super) fn enforce_live_output_limit(root: &Path, limit: u64) -> NativeResult<()> {
    enforce_output_limit(root, limit, OutputScan::Live)
}

pub(super) fn enforce_final_output_limit(root: &Path, limit: u64) -> NativeResult<()> {
    enforce_output_limit(root, limit, OutputScan::Final)
}

fn enforce_output_limit(root: &Path, limit: u64, scan: OutputScan) -> NativeResult<()> {
    if directory_size(root, scan)? <= limit {
        return Ok(());
    }
    Err(NativeError::ResourceLimitExceeded {
        resource: "output",
        limit,
    })
}

fn directory_size(root: &Path, scan: OutputScan) -> NativeResult<u64> {
    directory_size_inner(root, scan, |_| {})
}

#[cfg(test)]
pub(super) fn directory_size_with_hook<F>(
    root: &Path,
    scan: OutputScan,
    before_metadata: F,
) -> NativeResult<u64>
where
    F: FnMut(&Path),
{
    directory_size_inner(root, scan, before_metadata)
}

fn directory_size_inner<F>(
    root: &Path,
    scan: OutputScan,
    mut before_metadata: F,
) -> NativeResult<u64>
where
    F: FnMut(&Path),
{
    let mut total = 0_u64;
    let root_entries = fs::read_dir(root)?;
    let mut pending = vec![root_entries];
    while let Some(entries) = pending.pop() {
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) if allows_missing_descendant(scan, &error) => continue,
                Err(error) => return Err(error.into()),
            };
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) if allows_missing_descendant(scan, &error) => continue,
                Err(error) => return Err(error.into()),
            };
            if file_type.is_symlink() || (!file_type.is_dir() && !file_type.is_file()) {
                return Err(NativeError::UnsafeOutput(path));
            }
            before_metadata(&path);
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if allows_missing_descendant(scan, &error) => continue,
                Err(error) => return Err(error.into()),
            };
            if metadata.file_type().is_symlink() {
                return Err(NativeError::UnsafeOutput(path));
            }
            if metadata.is_dir() {
                match fs::read_dir(&path) {
                    Ok(entries) => pending.push(entries),
                    Err(error) if allows_missing_descendant(scan, &error) => {}
                    Err(error) => return Err(error.into()),
                }
            } else if metadata.is_file() {
                total = total.checked_add(metadata.len()).ok_or(
                    NativeError::ResourceLimitExceeded {
                        resource: "output",
                        limit: u64::MAX,
                    },
                )?;
            } else {
                return Err(NativeError::UnsafeOutput(path));
            }
        }
    }
    Ok(total)
}

fn allows_missing_descendant(scan: OutputScan, error: &std::io::Error) -> bool {
    matches!(scan, OutputScan::Live) && error.kind() == std::io::ErrorKind::NotFound
}
