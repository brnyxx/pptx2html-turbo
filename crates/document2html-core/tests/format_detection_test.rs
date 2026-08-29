use std::io::{Cursor, Write};

use document2html_core::{DocumentError, DocumentFormat, DocumentInput, detect_format};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

const CFBF_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

#[test]
fn detects_pdf_from_signature_without_filename() {
    // Given
    let pdf = minimal_pdf();
    let input = DocumentInput::detect(&pdf, None);

    // When
    let format = detect_format(&input).expect("PDF signature should be detected");

    // Then
    assert_eq!(format, DocumentFormat::Pdf);
}

#[test]
fn detects_each_ooxml_family_from_package_content() {
    for format in [
        DocumentFormat::Docx,
        DocumentFormat::Xlsx,
        DocumentFormat::Pptx,
    ] {
        // Given
        let package = build_ooxml_package(format);
        let input = DocumentInput::detect(&package, None);

        // When
        let detected = detect_format(&input).expect("OOXML family should be detected");

        // Then
        assert_eq!(detected, format);
    }
}

#[test]
fn rejects_doctype_in_unreferenced_ooxml_part() {
    let package = build_ooxml_package_with_extra(
        DocumentFormat::Docx,
        Some((
            "security/entity.xml",
            br#"<!DOCTYPE root [<!ENTITY x "expanded">]><root>&x;</root>"#,
        )),
    );

    let error = detect_format(&DocumentInput::detect(&package, None))
        .expect_err("OOXML document type declarations should fail");

    assert!(matches!(error, DocumentError::UnsupportedFormat));
}

#[test]
fn rejects_oversized_unreferenced_ooxml_part() {
    let oversized = vec![b' '; 16 * 1024 * 1024 + 1];
    let package = build_ooxml_package_with_extra(
        DocumentFormat::Xlsx,
        Some(("security/oversized.xml", &oversized)),
    );

    let error = detect_format(&DocumentInput::detect(&package, None))
        .expect_err("oversized OOXML should fail");

    assert!(matches!(
        error,
        DocumentError::PackageMetadataTooLarge { part, limit }
            if part == "security/oversized.xml" && limit == 16 * 1024 * 1024
    ));
}

#[test]
fn rejects_oversized_binary_ooxml_part() {
    let oversized = vec![0_u8; 16 * 1024 * 1024 + 1];
    let package = build_ooxml_package_with_extra(
        DocumentFormat::Docx,
        Some(("security/bomb.bin", &oversized)),
    );

    let error = detect_format(&DocumentInput::detect(&package, None))
        .expect_err("oversized binary OOXML part should fail");

    assert!(matches!(
        error,
        DocumentError::PackageMetadataTooLarge { part, limit }
            if part == "security/bomb.bin" && limit == 16 * 1024 * 1024
    ));
}

#[test]
fn rejects_filename_that_conflicts_with_conclusive_signature() {
    // Given
    let pdf = minimal_pdf();
    let input = DocumentInput::detect(&pdf, Some("report.docx"));

    // When
    let error = detect_format(&input).expect_err("conflicting extension should fail");

    // Then
    assert!(matches!(
        error,
        DocumentError::ConflictingFormatHint {
            detected: DocumentFormat::Pdf,
            hinted: DocumentFormat::Docx,
        }
    ));
}

#[test]
fn requires_a_hint_to_disambiguate_legacy_compound_files() {
    // Given
    let package = build_cfb_package();
    let input = DocumentInput::detect(&package, None);

    // When
    let error = detect_format(&input).expect_err("CFBF without a hint should be ambiguous");

    // Then
    assert!(matches!(error, DocumentError::AmbiguousFormat));
}

#[test]
fn legacy_filename_extension_disambiguates_compound_files() {
    for (source_name, expected) in [
        ("report.doc", DocumentFormat::Doc),
        ("report.xls", DocumentFormat::Xls),
        ("report.ppt", DocumentFormat::Ppt),
    ] {
        // Given
        let package = build_cfb_package();
        let input = DocumentInput::detect(&package, Some(source_name));

        // When
        let detected = detect_format(&input).expect("legacy extension should disambiguate CFBF");

        // Then
        assert_eq!(detected, expected);
    }
}

#[test]
fn explicit_format_hint_disambiguates_compound_files() {
    // Given
    let package = build_cfb_package();
    let input = DocumentInput::with_format(&package, None, DocumentFormat::Doc);

    // When
    let detected = detect_format(&input).expect("explicit CFBF hint should be accepted");

    // Then
    assert_eq!(detected, DocumentFormat::Doc);
}

#[test]
fn detects_compact_compound_file_only_with_end_of_chain_fat_tail() {
    let compact = compact_cfb_with_end_of_chain_fat_tail();

    let error = cfb::CompoundFile::open(Cursor::new(&compact.data))
        .expect_err("rust-cfb should reject the compact FAT");
    assert_eq!(
        error.to_string(),
        format!(
            "Malformed FAT (FAT has 128 entries, but file has only {} sectors)",
            compact.physical_sector_count
        )
    );

    let detected = detect_format(&DocumentInput::with_format(
        &compact.data,
        None,
        DocumentFormat::Doc,
    ))
    .expect("compact CFBF with an ENDOFCHAIN FAT tail should validate");
    assert_eq!(detected, DocumentFormat::Doc);

    let mut logical_only_directory = compact.data.clone();
    logical_only_directory[48..52]
        .copy_from_slice(&(compact.physical_sector_count as u32).to_le_bytes());
    let error = detect_format(&DocumentInput::with_format(
        &logical_only_directory,
        None,
        DocumentFormat::Doc,
    ))
    .expect_err("logical-only directory sector should fail physical reads");
    assert!(matches!(
        error,
        DocumentError::Io(ref error) if error.kind() == std::io::ErrorKind::UnexpectedEof
    ));

    let mut allocated_tail = compact.data.clone();
    allocated_tail[compact.first_trailing_fat_entry..compact.first_trailing_fat_entry + 4]
        .copy_from_slice(&0_u32.to_le_bytes());
    assert!(
        detect_format(&DocumentInput::with_format(
            &allocated_tail,
            None,
            DocumentFormat::Doc,
        ))
        .is_err()
    );

    let mut truncated_allocated_sector = compact.data;
    truncated_allocated_sector.truncate(truncated_allocated_sector.len() - compact.sector_size);
    assert!(
        detect_format(&DocumentInput::with_format(
            &truncated_allocated_sector,
            None,
            DocumentFormat::Doc,
        ))
        .is_err()
    );
}

#[test]
fn rejects_explicit_format_hint_that_conflicts_with_signature() {
    // Given
    let pdf = minimal_pdf();
    let input = DocumentInput::with_format(&pdf, Some("report.pdf"), DocumentFormat::Docx);

    // When
    let error = detect_format(&input).expect_err("conflicting explicit hint should fail");

    // Then
    assert!(matches!(
        error,
        DocumentError::ConflictingFormatHint {
            detected: DocumentFormat::Pdf,
            hinted: DocumentFormat::Docx,
        }
    ));
}

#[test]
fn rejects_unknown_bytes_instead_of_trusting_extension() {
    // Given
    let input = DocumentInput::detect(b"not a document", Some("report.docx"));

    // When
    let error = detect_format(&input).expect_err("extension-only OOXML detection should fail");

    // Then
    assert!(matches!(error, DocumentError::UnsupportedFormat));
}

#[test]
fn rejects_misdirected_pdf_xref_and_encryption() {
    let malformed = String::from_utf8(minimal_pdf())
        .expect("ASCII fixture")
        .replace("startxref\n45", "startxref\n46")
        .into_bytes();
    let mut encrypted = minimal_pdf();
    encrypted.extend_from_slice(b"/Encrypt 4 0 R");

    assert!(detect_format(&DocumentInput::detect(&malformed, None)).is_err());
    assert!(detect_format(&DocumentInput::detect(&encrypted, None)).is_err());
}

#[test]
fn rejects_truncated_compound_file_with_a_legacy_hint() {
    let input = DocumentInput::with_format(&CFBF_MAGIC, None, DocumentFormat::Doc);

    let error = detect_format(&input).expect_err("truncated CFBF should fail validation");

    assert!(matches!(error, DocumentError::Io(_)));
}

fn build_ooxml_package(format: DocumentFormat) -> Vec<u8> {
    build_ooxml_package_with_extra(format, None)
}

fn build_cfb_package() -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let compound = cfb::CompoundFile::create(cursor).expect("create compound file");
    compound.into_inner().into_inner()
}

struct CompactCfbFixture {
    data: Vec<u8>,
    sector_size: usize,
    physical_sector_count: usize,
    first_trailing_fat_entry: usize,
}

fn compact_cfb_with_end_of_chain_fat_tail() -> CompactCfbFixture {
    let cursor = Cursor::new(Vec::new());
    let mut compound = cfb::CompoundFile::create_with_version(cfb::Version::V3, cursor)
        .expect("create compound file");
    let mut stream = compound
        .create_stream("/WordDocument")
        .expect("create WordDocument stream");
    stream
        .write_all(&vec![0; 4097])
        .expect("write WordDocument stream");
    drop(stream);
    let mut full = compound.into_inner().into_inner();
    let sector_shift = u16::from_le_bytes(full[30..32].try_into().expect("sector shift"));
    let sector_size = 1_usize << sector_shift;
    let full_sector_count = full.len() / sector_size - 1;
    let fat_sector =
        u32::from_le_bytes(full[76..80].try_into().expect("FAT sector entry")) as usize;
    let fat_offset = (fat_sector + 1) * sector_size;
    let fat_entry_capacity = sector_size / 4;
    let physical_sector_count = (0..full_sector_count)
        .filter(|entry| {
            u32::from_le_bytes(
                full[fat_offset + entry * 4..fat_offset + (entry + 1) * 4]
                    .try_into()
                    .expect("FAT entry"),
            ) != u32::MAX
        })
        .max()
        .expect("CFBF must contain allocated sectors")
        + 1;
    for entry in physical_sector_count..fat_entry_capacity {
        full[fat_offset + entry * 4..fat_offset + (entry + 1) * 4]
            .copy_from_slice(&0xffff_fffe_u32.to_le_bytes());
    }
    full.truncate((physical_sector_count + 1) * sector_size);

    CompactCfbFixture {
        data: full,
        sector_size,
        physical_sector_count,
        first_trailing_fat_entry: fat_offset + physical_sector_count * 4,
    }
}

fn minimal_pdf() -> Vec<u8> {
    b"%PDF-1.7
1 0 obj
<< /Type /Catalog >>
endobj
xref
0 2
0000000000 65535 f\x20
0000000009 00000 n\x20
trailer
<< /Size 2 /Root 1 0 R >>
startxref
45
%%EOF
"
    .to_vec()
}

fn build_ooxml_package_with_extra(format: DocumentFormat, extra: Option<(&str, &[u8])>) -> Vec<u8> {
    let (part_name, content_type, target) = match format {
        DocumentFormat::Docx => (
            "/word/document.xml",
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml",
            "word/document.xml",
        ),
        DocumentFormat::Xlsx => (
            "/xl/workbook.xml",
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml",
            "xl/workbook.xml",
        ),
        DocumentFormat::Pptx => (
            "/ppt/presentation.xml",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml",
            "ppt/presentation.xml",
        ),
        DocumentFormat::Doc | DocumentFormat::Xls | DocumentFormat::Ppt | DocumentFormat::Pdf => {
            panic!("test helper requires an OOXML format")
        }
    };

    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    let options = SimpleFileOptions::default();
    zip.start_file("[Content_Types].xml", options)
        .expect("start content types");
    write!(
        zip,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="{part_name}" ContentType="{content_type}"/>
</Types>"#
    )
    .expect("write content types");
    zip.start_file("_rels/.rels", options)
        .expect("start root relationships");
    write!(
        zip,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="{target}"/>
</Relationships>"#
    )
    .expect("write root relationships");
    zip.start_file(target, options).expect("start main part");
    zip.write_all(b"<root/>").expect("write main part");
    if let Some((name, data)) = extra {
        zip.start_file(name, options).expect("start extra part");
        zip.write_all(data).expect("write extra part");
    }
    zip.finish().expect("finish OOXML package").into_inner()
}
