use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use document2html_core::{AssetMode, DocumentAsset};

use crate::{NativeError, NativeResult};

pub(crate) struct NormalizedHtml {
    pub(crate) html: String,
    pub(crate) assets: Vec<DocumentAsset>,
    pub(crate) page_count: usize,
}

pub(crate) fn normalize_poppler_html(
    output_dir: &Path,
    html_path: &Path,
    asset_mode: AssetMode,
    expected_pages: usize,
    workspace_root: &Path,
) -> NativeResult<NormalizedHtml> {
    let mut files = read_output_inventory(output_dir, html_path)?;
    let html_bytes = fs::read(html_path)?;
    let html = String::from_utf8(html_bytes).map_err(|_| malformed("HTML is not UTF-8"))?;
    let mut html = normalize_generated_metadata(&html, workspace_root)?;
    validate_safe_html(&html, workspace_root)?;
    let page_count = count_page_containers(&html)?;
    if page_count != expected_pages {
        return Err(malformed("HTML and PDF page counts differ"));
    }
    files.sort_by(|left, right| left.file_name().cmp(&right.file_name()));
    let mut assets = Vec::new();
    for (index, source) in files.into_iter().enumerate() {
        let original_name = file_name(&source)?;
        if !html.contains(&original_name) {
            return Err(malformed("Poppler emitted an unreferenced asset"));
        }
        let extension = extension(&source)?;
        let content_type = content_type(extension)?;
        let data = fs::read(&source)?;
        let replacement = match asset_mode {
            AssetMode::Embed => format!(
                "data:{content_type};base64,{}",
                base64::engine::general_purpose::STANDARD.encode(&data)
            ),
            AssetMode::External => format!("assets/asset-{:04}.{extension}", index + 1),
        };
        html = html.replace(&original_name, &replacement);
        if asset_mode == AssetMode::External {
            assets.push(DocumentAsset {
                relative_path: replacement,
                content_type: content_type.to_owned(),
                data,
            });
        }
    }
    Ok(NormalizedHtml {
        html: canonicalize(&html),
        assets,
        page_count,
    })
}

fn read_output_inventory(output_dir: &Path, html_path: &Path) -> NativeResult<Vec<PathBuf>> {
    let mut assets = Vec::new();
    let mut found_html = false;
    for entry in fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(NativeError::UnsafeOutput(path));
        }
        if path == html_path {
            if found_html {
                return Err(malformed("duplicate HTML output"));
            }
            found_html = true;
            continue;
        }
        extension(&path)?;
        assets.push(path);
    }
    if !found_html {
        return Err(malformed("missing output.html"));
    }
    Ok(assets)
}

fn validate_safe_html(html: &str, workspace_root: &Path) -> NativeResult<()> {
    let lowercase = html.to_ascii_lowercase();
    for forbidden in [
        "<script",
        "<form",
        "<object",
        "<embed",
        "<iframe",
        "<link",
        "@import",
        "@font-face",
        "javascript:",
        "file:",
        "src=\"http:",
        "src=\"https:",
        "src='http:",
        "src='https:",
    ] {
        if lowercase.contains(forbidden) {
            return Err(malformed(
                "HTML contains forbidden active or remote content",
            ));
        }
    }
    let workspace = workspace_root.to_string_lossy();
    if !workspace.is_empty() && html.contains(workspace.as_ref()) {
        return Err(malformed("HTML contains a temporary absolute path"));
    }
    Ok(())
}

fn normalize_generated_metadata(html: &str, workspace_root: &Path) -> NativeResult<String> {
    let workspace = workspace_root.to_string_lossy();
    let generated_title = format!("<title>{workspace}/poppler/output</title>");
    let mut normalized = if html.contains(&generated_title) {
        html.replacen(&generated_title, "<title>document</title>", 1)
    } else {
        html.to_owned()
    };
    let date_prefix = r#"<meta name="date" content=""#;
    if let Some(start) = normalized.find(date_prefix) {
        let Some(relative_end) = normalized[start..].find("/>") else {
            return Err(malformed("malformed Poppler date metadata"));
        };
        let mut end = start + relative_end + 2;
        if normalized.as_bytes().get(end) == Some(&b'\n') {
            end += 1;
        }
        normalized.replace_range(start..end, "");
    }
    if normalized.contains(date_prefix) {
        return Err(malformed("duplicate Poppler date metadata"));
    }
    Ok(normalized)
}

fn count_page_containers(html: &str) -> NativeResult<usize> {
    let mut remaining = html;
    let mut expected = 1_usize;
    while let Some(position) = remaining.find("id=\"page") {
        remaining = &remaining[position + 8..];
        let Some(end) = remaining.find("-div\"") else {
            return Err(malformed("malformed Poppler page id"));
        };
        let page = remaining[..end]
            .parse::<usize>()
            .map_err(|_| malformed("non-numeric Poppler page id"))?;
        if page != expected {
            return Err(malformed("Poppler page ids are not strictly sequential"));
        }
        expected += 1;
        remaining = &remaining[end + 5..];
    }
    if expected == 1 {
        return Err(malformed("HTML contains no Poppler page containers"));
    }
    Ok(expected - 1)
}

fn canonicalize(html: &str) -> String {
    let normalized = html.replace("\r\n", "\n").replace('\r', "\n");
    let mut output = normalized
        .lines()
        .map(|line| line.trim_end_matches(' '))
        .collect::<Vec<_>>()
        .join("\n");
    output.push('\n');
    output
}

fn extension(path: &Path) -> NativeResult<&str> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .ok_or_else(|| malformed("asset has no UTF-8 extension"))?;
    if extension.eq_ignore_ascii_case("png") {
        Ok("png")
    } else if extension.eq_ignore_ascii_case("jpg") || extension.eq_ignore_ascii_case("jpeg") {
        Ok("jpg")
    } else {
        Err(malformed("asset has an unsupported extension"))
    }
}

fn content_type(extension: &str) -> NativeResult<&'static str> {
    match extension {
        "png" => Ok("image/png"),
        "jpg" => Ok("image/jpeg"),
        _ => Err(malformed("asset has an unsupported content type")),
    }
}

fn file_name(path: &Path) -> NativeResult<String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::to_owned)
        .ok_or_else(|| malformed("asset filename is not UTF-8"))
}

fn malformed(reason: &str) -> NativeError {
    NativeError::MalformedBackendOutput {
        backend: "pdftohtml",
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
#[path = "html_tests.rs"]
mod tests;
