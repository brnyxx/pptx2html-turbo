use std::fs;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use zip::{ZipArchive, ZipWriter};

use document2html_core::{DocumentFormat, DocumentInput};

use crate::config::NativeBackendConfig;
use crate::fonts::EastAsianFontPolicy;
use crate::process::CommandSpec;
use crate::runtime::NativeRuntimeInfo;
use crate::stage::run_stage;
use crate::workspace::TemporaryWorkspace;
use crate::xlsx_workbook::freeze_workbook_calculation;
use crate::{NativeError, NativeResult};

pub(crate) struct OfficeOutput {
    pub(crate) pdf: PathBuf,
    pub(crate) semantic_xlsx: Option<PathBuf>,
}

pub(crate) fn convert_office_to_pdf(
    input: &DocumentInput<'_>,
    format: DocumentFormat,
    config: &NativeBackendConfig,
    runtime: &NativeRuntimeInfo,
    workspace: &TemporaryWorkspace,
) -> NativeResult<OfficeOutput> {
    if input.data.len() as u64 > config.max_input_bytes {
        return Err(NativeError::ResourceLimitExceeded {
            resource: "input",
            limit: config.max_input_bytes,
        });
    }
    let input_path = workspace
        .root()
        .join("input")
        .join(format!("input.{}", format.extension()));
    fs::write(&input_path, input.data)?;
    match &runtime.east_asian_fonts {
        EastAsianFontPolicy::Pinned(substitute) => {
            workspace.seed_font_substitution(&substitute.family)?;
        }
        EastAsianFontPolicy::PlatformDefault { .. } => {}
    }
    let profile = file_uri(&workspace.root().join("profile"));
    let semantic_xlsx = if format == DocumentFormat::Xls {
        let converted = convert_xls_to_xlsx(&input_path, &profile, config, runtime, workspace)?;
        Some(freeze_xlsx_snapshot(&converted, config, workspace)?)
    } else {
        None
    };
    let pdf_input = semantic_xlsx.as_deref().unwrap_or(&input_path);
    let office_dir = workspace.root().join("office");
    let command = CommandSpec::new(&runtime.libreoffice.executable)
        .arg("--headless")
        .arg(format!("-env:UserInstallation={profile}"))
        .arg("--convert-to")
        .arg("pdf")
        .arg("--outdir")
        .arg(&office_dir)
        .arg(pdf_input)
        .working_directory(workspace.root());
    run_stage(command, config, workspace.root(), &office_dir)?;
    let pdf = validate_office_output(&office_dir, "pdf")?;
    Ok(OfficeOutput { pdf, semantic_xlsx })
}

fn convert_xls_to_xlsx(
    input_path: &Path,
    profile: &str,
    config: &NativeBackendConfig,
    runtime: &NativeRuntimeInfo,
    workspace: &TemporaryWorkspace,
) -> NativeResult<PathBuf> {
    let output_dir = workspace.root().join("spreadsheet");
    let command = CommandSpec::new(&runtime.libreoffice.executable)
        .arg("--headless")
        .arg(format!("-env:UserInstallation={profile}"))
        .arg("--convert-to")
        .arg("xlsx")
        .arg("--outdir")
        .arg(&output_dir)
        .arg(input_path)
        .working_directory(workspace.root());
    run_stage(command, config, workspace.root(), &output_dir)?;
    validate_office_output(&output_dir, "xlsx")
}

const MAX_ARCHIVE_ENTRIES: usize = 16_384;
const MAX_EOCD_SIZE: usize = 22 + u16::MAX as usize;
const WORKBOOK_PART: &str = "xl/workbook.xml";
const ZIP_EOCD_SIGNATURE: &[u8; 4] = b"PK\x05\x06";

fn freeze_xlsx_snapshot(
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

fn validate_office_output(office_dir: &Path, extension: &str) -> NativeResult<PathBuf> {
    let expected = office_dir.join(format!("input.{extension}"));
    let mut entries = fs::read_dir(office_dir)?;
    let Some(entry) = entries.next().transpose()? else {
        return malformed("LibreOffice did not create the expected output");
    };
    if entries.next().transpose()?.is_some() || entry.path() != expected {
        return malformed("LibreOffice emitted an unexpected output inventory");
    }
    let metadata = fs::symlink_metadata(&expected)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(NativeError::UnsafeOutput(expected));
    }
    Ok(expected)
}

fn file_uri(path: &Path) -> String {
    let value = path.to_string_lossy();
    let escaped = value
        .replace('%', "%25")
        .replace(' ', "%20")
        .replace('#', "%23");
    if escaped.starts_with('/') {
        format!("file://{escaped}")
    } else {
        format!("file:///{escaped}")
    }
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
mod tests {
    use std::fs;
    use std::io::{Cursor, Read, Write};

    use zip::write::SimpleFileOptions;
    use zip::{ZipArchive, ZipWriter};

    use super::{file_uri, freeze_xlsx_archive};

    const WORKBOOK_WITH_CALC_PR: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheets/><calcPr calcId="1" calcMode="auto"/></workbook>"#;
    const WORKBOOK_WITHOUT_CALC_PR: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheets/></workbook>"#;
    const CACHED_FORMULA: &[u8] = br#"<worksheet><sheetData><row r="1"><c r="A1"><f>NOW()</f><v>45292.5</v></c></row></sheetData></worksheet>"#;

    #[test]
    fn profile_uri_escapes_path_delimiters() {
        // Given
        let path = std::path::Path::new("/tmp/profile #1");

        // When
        let uri = file_uri(path);

        // Then
        assert_eq!(uri, "file:///tmp/profile%20%231");
    }

    #[test]
    fn freezes_existing_calc_pr_without_changing_cached_formula_values_or_other_entries() {
        // Given
        let source = xlsx_archive(&[
            ("xl/workbook.xml", WORKBOOK_WITH_CALC_PR),
            ("xl/worksheets/sheet1.xml", CACHED_FORMULA),
            ("xl/media/image.bin", b"unchanged binary entry"),
        ]);

        // When
        let frozen = freeze_archive(&source).expect("freeze XLSX");

        // Then
        let workbook = archive_entry(&frozen, "xl/workbook.xml");
        assert!(workbook.contains("calcMode=\"manual\""));
        assert!(workbook.contains("calcOnSave=\"0\""));
        assert!(workbook.contains("forceFullCalc=\"0\""));
        assert!(workbook.contains("fullCalcOnLoad=\"0\""));
        assert!(!workbook.contains("calcId=\"1\""));
        assert_eq!(
            archive_entry_bytes(&frozen, "xl/worksheets/sheet1.xml"),
            CACHED_FORMULA
        );
        assert_eq!(
            archive_entry_bytes(&frozen, "xl/media/image.bin"),
            b"unchanged binary entry"
        );
    }

    #[test]
    fn injects_calc_pr_when_workbook_has_none() {
        // Given
        let source = xlsx_archive(&[("xl/workbook.xml", WORKBOOK_WITHOUT_CALC_PR)]);

        // When
        let frozen = freeze_archive(&source).expect("freeze XLSX");

        // Then
        let workbook = archive_entry(&frozen, "xl/workbook.xml");
        assert_eq!(workbook.matches("calcPr").count(), 1);
        assert!(workbook.contains("<calcPr calcMode=\"manual\" calcOnSave=\"0\" forceFullCalc=\"0\" fullCalcOnLoad=\"0\"/>"));
    }

    #[test]
    fn preserves_valid_xml_entities_when_freezing_workbook_calculation() {
        // Given
        let workbook = br#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:ext="urn:example:x&amp;y"><sheets/></workbook>"#;
        let source = xlsx_archive(&[("xl/workbook.xml", workbook)]);

        // When
        let frozen = freeze_archive(&source).expect("freeze XLSX with valid XML entity");

        // Then
        assert!(archive_entry(&frozen, "xl/workbook.xml").contains("urn:example:x&amp;y"));
    }

    #[test]
    fn rejects_malformed_workbook() {
        // Given
        let malformed = xlsx_archive(&[("xl/workbook.xml", b"<workbook><sheets></workbook>")]);

        // When
        let malformed_error = freeze_archive(&malformed).expect_err("malformed workbook must fail");

        // Then
        assert!(matches!(
            malformed_error,
            crate::NativeError::MalformedBackendOutput { .. }
        ));
    }

    #[test]
    fn rejects_duplicate_archive_parts() {
        // Given
        let mut duplicate = xlsx_archive(&[
            ("xl/workbook.xml", WORKBOOK_WITHOUT_CALC_PR),
            ("xl/workbook.dup", WORKBOOK_WITHOUT_CALC_PR),
        ]);
        replace_archive_name(&mut duplicate, b"xl/workbook.dup", b"xl/workbook.xml");

        // When
        let duplicate_error = freeze_archive(&duplicate).expect_err("duplicate workbook must fail");

        // Then
        assert_malformed_reason(
            duplicate_error,
            "converted XLSX has duplicate archive entries",
        );
    }

    #[test]
    fn rejects_zip64_entry_count_sentinel() {
        // Given
        let mut archive = xlsx_archive(&[("xl/workbook.xml", WORKBOOK_WITHOUT_CALC_PR)]);
        let eocd = archive
            .windows(4)
            .rposition(|signature| signature == b"PK\x05\x06")
            .expect("find ZIP end record");
        archive[eocd + 8..eocd + 12].fill(0xff);

        // When
        let error = freeze_archive(&archive).expect_err("ZIP64 entry count must fail");

        // Then
        assert_malformed_reason(
            error,
            "converted XLSX ZIP64 entry count exceeds the supported limit",
        );
    }

    fn xlsx_archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, content) in entries {
            writer
                .start_file(name, SimpleFileOptions::default())
                .expect("start archive entry");
            writer.write_all(content).expect("write archive entry");
        }
        writer.finish().expect("finish source archive").into_inner()
    }

    fn replace_archive_name(archive: &mut [u8], original: &[u8], replacement: &[u8]) {
        assert_eq!(original.len(), replacement.len());
        let offsets = archive
            .windows(original.len())
            .enumerate()
            .filter_map(|(offset, name)| (name == original).then_some(offset))
            .collect::<Vec<_>>();
        assert_eq!(offsets.len(), 2);
        for offset in offsets {
            archive[offset..offset + replacement.len()].copy_from_slice(replacement);
        }
    }

    fn freeze_archive(source: &[u8]) -> crate::NativeResult<Vec<u8>> {
        let directory = tempfile::tempdir().expect("create temporary directory");
        let source_path = directory.path().join("source.xlsx");
        let frozen_path = directory.path().join("frozen.xlsx");
        fs::write(&source_path, source).expect("write source archive");
        freeze_xlsx_archive(&source_path, &frozen_path, 1024 * 1024)?;
        Ok(fs::read(frozen_path).expect("read frozen archive"))
    }

    fn archive_entry(archive: &[u8], name: &str) -> String {
        String::from_utf8(archive_entry_bytes(archive, name)).expect("UTF-8 XML entry")
    }

    fn archive_entry_bytes(archive: &[u8], name: &str) -> Vec<u8> {
        let mut archive = ZipArchive::new(Cursor::new(archive)).expect("open archive");
        let mut entry = archive.by_name(name).expect("find archive entry");
        let mut content = Vec::new();
        entry.read_to_end(&mut content).expect("read archive entry");
        content
    }

    fn assert_malformed_reason(error: crate::NativeError, expected: &str) {
        let crate::NativeError::MalformedBackendOutput { backend, reason } = error else {
            panic!("expected malformed backend output");
        };
        assert_eq!(backend, "libreoffice");
        assert_eq!(reason, expected);
    }
}
