use std::io::{Read, Seek};

use zip::ZipArchive;

use crate::error::{PptxError, PptxResult};

pub(crate) fn read_archive_entry<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> PptxResult<String> {
    let mut file = archive
        .by_name(name)
        .map_err(|_| PptxError::MissingFile(name.to_owned()))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

pub(crate) fn read_archive_bytes<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> PptxResult<Vec<u8>> {
    let mut file = archive
        .by_name(name)
        .map_err(|_| PptxError::MissingFile(name.to_owned()))?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    Ok(contents)
}

pub(crate) fn relationships_path(part_path: &str) -> String {
    let (directory, file) = part_path.rsplit_once('/').unwrap_or(("", part_path));
    if directory.is_empty() {
        format!("_rels/{file}.rels")
    } else {
        format!("{directory}/_rels/{file}.rels")
    }
}

pub(crate) fn resolve_relative_file_path(base_file: &str, target: &str) -> String {
    let base_directory = base_file
        .rsplit_once('/')
        .map(|(directory, _)| directory)
        .unwrap_or("");
    resolve_relationship_path(base_directory, target)
}

pub(crate) fn resolve_relationship_path(base_directory: &str, target: &str) -> String {
    if !target.contains("../") {
        return format!("{base_directory}/{target}");
    }
    let mut parts = base_directory.split('/').collect::<Vec<_>>();
    for segment in target.split('/') {
        match segment {
            ".." => {
                parts.pop();
            }
            "" | "." => {}
            _ => parts.push(segment),
        }
    }
    parts.join("/")
}

pub(crate) fn mime_from_extension(path: &str) -> String {
    let extension = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "svg" => "image/svg+xml",
        "emf" => "image/x-emf",
        "wmf" => "image/x-wmf",
        _ => "image/png",
    }
    .to_owned()
}
