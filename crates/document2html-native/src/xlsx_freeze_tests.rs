use std::fs;
use std::io::{Cursor, Read, Write};

use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use super::freeze_xlsx_archive;

const WORKBOOK_WITH_CALC_PR: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheets/><calcPr calcId="1" calcMode="auto"/></workbook>"#;
const WORKBOOK_WITHOUT_CALC_PR: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheets/></workbook>"#;
const CACHED_FORMULA: &[u8] = br#"<worksheet><sheetData><row r="1"><c r="A1"><f>NOW()</f><v>45292.5</v></c></row></sheetData></worksheet>"#;

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
fn freezes_calc_pr_with_a_different_prefix_for_the_workbook_namespace() {
    // Given
    let workbook = br#"<x:workbook xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:s="http:&#x2F;&#x2F;schemas.openxmlformats.org/spreadsheetml/2006/main"><x:sheets/><s:calcPr calcId="1" calcMode="auto"/></x:workbook>"#;
    let source = xlsx_archive(&[("xl/workbook.xml", workbook)]);

    // When
    let frozen = freeze_archive(&source).expect("freeze namespace-equivalent calcPr");

    // Then
    let frozen_workbook = archive_entry(&frozen, "xl/workbook.xml");
    assert_eq!(frozen_workbook.matches("calcPr").count(), 1);
    assert!(!frozen_workbook.contains("calcId=\"1\""));
    assert!(frozen_workbook.contains("<x:calcPr calcMode=\"manual\""));
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
    assert!(workbook.contains(
        "<calcPr calcMode=\"manual\" calcOnSave=\"0\" forceFullCalc=\"0\" fullCalcOnLoad=\"0\"/>"
    ));
}

#[test]
fn injects_calc_pr_before_schema_following_workbook_children() {
    // Given
    let workbook = br#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheets/><extLst><ext uri="urn:example"/></extLst></workbook>"#;
    let source = xlsx_archive(&[("xl/workbook.xml", workbook)]);

    // When
    let frozen = freeze_archive(&source).expect("freeze workbook with extension list");

    // Then
    let frozen_workbook = archive_entry(&frozen, "xl/workbook.xml");
    let calc = frozen_workbook.find("<calcPr").expect("find calcPr");
    let extensions = frozen_workbook.find("<extLst").expect("find extLst");
    assert!(calc < extensions);
    assert!(frozen_workbook.contains("<ext uri=\"urn:example\"/>"));
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
fn rejects_foreign_workbook_namespace() {
    // Given
    let workbook =
        br#"<x:workbook xmlns:x="urn:foreign"><x:sheets/><x:calcPr calcMode="auto"/></x:workbook>"#;
    let archive = xlsx_archive(&[("xl/workbook.xml", workbook)]);

    // When
    let error = freeze_archive(&archive).expect_err("foreign workbook namespace must fail");

    // Then
    assert_malformed_reason(
        error,
        "xl/workbook.xml has an unsupported workbook namespace",
    );
}

#[test]
fn rejects_unbound_workbook_prefix() {
    // Given
    let workbook = br#"<x:workbook><x:sheets/></x:workbook>"#;
    let archive = xlsx_archive(&[("xl/workbook.xml", workbook)]);

    // When
    let error = freeze_archive(&archive).expect_err("unbound workbook prefix must fail");

    // Then
    assert_malformed_reason(error, "xl/workbook.xml has an unbound element prefix");
}

#[test]
fn rejects_missing_workbook_namespace() {
    // Given
    let workbook = br#"<workbook><sheets/></workbook>"#;
    let archive = xlsx_archive(&[("xl/workbook.xml", workbook)]);

    // When
    let error = freeze_archive(&archive).expect_err("missing workbook namespace must fail");

    // Then
    assert_malformed_reason(
        error,
        "xl/workbook.xml has no SpreadsheetML workbook namespace",
    );
}

#[test]
fn rejects_duplicate_xml_attributes() {
    // Given
    let workbook = br#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheets/><calcPr calcMode="auto" calcMode="manual"/></workbook>"#;
    let archive = xlsx_archive(&[("xl/workbook.xml", workbook)]);

    // When
    let error = freeze_archive(&archive).expect_err("duplicate XML attribute must fail");

    // Then
    assert_malformed_reason(error, "xl/workbook.xml has a duplicate attribute");
}

#[test]
fn rejects_duplicate_expanded_xml_attributes() {
    // Given
    let workbook = br#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:a="urn:duplicate" xmlns:b="urn:duplicate" a:value="1" b:value="2"><sheets/></workbook>"#;
    let archive = xlsx_archive(&[("xl/workbook.xml", workbook)]);

    // When
    let error = freeze_archive(&archive).expect_err("duplicate expanded attribute must fail");

    // Then
    assert_malformed_reason(error, "xl/workbook.xml has a duplicate attribute");
}

#[test]
fn rejects_duplicate_expanded_attributes_with_inherited_namespace() {
    // Given
    let workbook = br#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:a="urn:duplicate"><sheets xmlns:b="urn:duplicate" a:value="1" b:value="2"/></workbook>"#;
    let archive = xlsx_archive(&[("xl/workbook.xml", workbook)]);

    // When
    let error = freeze_archive(&archive).expect_err("inherited duplicate attribute must fail");

    // Then
    assert_malformed_reason(error, "xl/workbook.xml has a duplicate attribute");
}

#[test]
fn rejects_unbound_xml_attribute_prefix() {
    // Given
    let workbook = br#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheets rogue:value="1"/></workbook>"#;
    let archive = xlsx_archive(&[("xl/workbook.xml", workbook)]);

    // When
    let error = freeze_archive(&archive).expect_err("unbound attribute prefix must fail");

    // Then
    assert_malformed_reason(error, "xl/workbook.xml has an unbound attribute prefix");
}

#[test]
fn rejects_unbound_xml_element_prefix() {
    // Given
    let workbook = br#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><rogue:unknown/></workbook>"#;
    let archive = xlsx_archive(&[("xl/workbook.xml", workbook)]);

    // When
    let error = freeze_archive(&archive).expect_err("unbound element prefix must fail");

    // Then
    assert_malformed_reason(error, "xl/workbook.xml has an unbound element prefix");
}

#[test]
fn rejects_forbidden_raw_xml_characters() {
    // Given
    let workbook = b"<workbook xmlns=\"urn:example:\x01\"><sheets/></workbook>";
    let archive = xlsx_archive(&[("xl/workbook.xml", workbook)]);

    // When
    let error = freeze_archive(&archive).expect_err("forbidden XML character must fail");

    // Then
    assert_malformed_reason(error, "xl/workbook.xml has an invalid XML character");
}

#[test]
fn rejects_forbidden_xml_characters_in_comments() {
    // Given
    let workbook = b"<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><!--\x01--><sheets/></workbook>";
    let archive = xlsx_archive(&[("xl/workbook.xml", workbook)]);

    // When
    let error = freeze_archive(&archive).expect_err("forbidden comment character must fail");

    // Then
    assert_malformed_reason(error, "xl/workbook.xml has an invalid XML character");
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
fn rejects_eocd_count_smaller_than_central_directory() {
    // Given
    let mut archive = xlsx_archive(&[
        ("xl/workbook.xml", WORKBOOK_WITHOUT_CALC_PR),
        ("xl/extra.bin", b"unlisted central directory entry"),
    ]);
    let eocd = archive
        .windows(4)
        .rposition(|signature| signature == b"PK\x05\x06")
        .expect("find ZIP end record");
    archive[eocd + 8..eocd + 10].copy_from_slice(&1_u16.to_le_bytes());
    archive[eocd + 10..eocd + 12].copy_from_slice(&1_u16.to_le_bytes());

    // When
    let error = freeze_archive(&archive).expect_err("truncated entry count must fail");

    // Then
    assert_malformed_reason(
        error,
        "converted XLSX central directory entry count is invalid",
    );
}

#[test]
fn rejects_non_workbook_entry_with_invalid_crc() {
    // Given
    let content = vec![b'x'; 8_192];
    let mut archive = xlsx_archive(&[
        ("xl/workbook.xml", WORKBOOK_WITHOUT_CALC_PR),
        ("xl/extra.bin", &content),
    ]);
    corrupt_archive_entry_crc(&mut archive, b"xl/extra.bin");

    // When
    let error = freeze_archive(&archive).expect_err("invalid entry CRC must fail");

    // Then
    assert_malformed_reason(error, "converted XLSX has an unreadable archive entry");
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

fn corrupt_archive_entry_crc(archive: &mut [u8], name: &[u8]) {
    let offsets = archive
        .windows(name.len())
        .enumerate()
        .filter_map(|(offset, candidate)| (candidate == name).then_some(offset))
        .collect::<Vec<_>>();
    assert_eq!(offsets.len(), 2);
    for name_offset in offsets {
        let crc_offset = if archive[name_offset - 30..name_offset - 26] == *b"PK\x03\x04" {
            name_offset - 16
        } else if archive[name_offset - 46..name_offset - 42] == *b"PK\x01\x02" {
            name_offset - 30
        } else {
            panic!("archive entry name does not follow a ZIP header");
        };
        archive[crc_offset] ^= 0xff;
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
