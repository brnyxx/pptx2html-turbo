use std::path::{Component, Path, PathBuf};

use document2html_core::DocumentAsset;

pub(crate) fn write_text(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    std::fs::write(path, content)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub(crate) fn write_assets(base: &Path, assets: &[DocumentAsset]) -> Result<(), String> {
    for asset in assets {
        let relative = Path::new(&asset.relative_path);
        validate_asset_path(relative)?;
        let path = base.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        std::fs::write(&path, &asset.data)
            .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    }
    Ok(())
}

pub(crate) fn validate_asset_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().is_empty() || path.is_absolute() {
        return Err(format!("unsafe asset path: {}", path.display()));
    }
    if path
        .components()
        .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(format!("unsafe asset path: {}", path.display()));
    }
    Ok(())
}

pub(crate) fn ensure_distinct_paths(
    left: &Path,
    right: &Path,
    left_label: &str,
    right_label: &str,
) -> Result<(), String> {
    let same = if left.exists() && right.exists() {
        same_file::is_same_file(left, right)
            .map_err(|error| format!("failed to compare paths: {error}"))?
    } else {
        normalized_absolute(left)? == normalized_absolute(right)?
    };
    if same {
        return Err(format!(
            "{left_label} and {right_label} resolve to {}",
            left.display()
        ));
    }
    Ok(())
}

fn normalized_absolute(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("failed to resolve current directory: {error}"))?
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized)
}
