use std::fs;
use std::path::Path;

use crate::{NativeError, NativeResult};

pub(super) fn enforce_output_limit(root: &Path, limit: u64) -> NativeResult<()> {
    if directory_size(root)? <= limit {
        return Ok(());
    }
    Err(NativeError::ResourceLimitExceeded {
        resource: "output",
        limit,
    })
}

fn directory_size(root: &Path) -> NativeResult<u64> {
    let mut total = 0_u64;
    let mut pending = vec![root.to_owned()];
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let metadata = fs::symlink_metadata(entry.path())?;
            if metadata.file_type().is_symlink() {
                return Err(NativeError::UnsafeOutput(entry.path()));
            }
            if metadata.is_dir() {
                pending.push(entry.path());
            } else if metadata.is_file() {
                total = total.checked_add(metadata.len()).ok_or(
                    NativeError::ResourceLimitExceeded {
                        resource: "output",
                        limit: u64::MAX,
                    },
                )?;
            } else {
                return Err(NativeError::UnsafeOutput(entry.path()));
            }
        }
    }
    Ok(total)
}
