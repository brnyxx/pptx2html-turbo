use std::collections::BTreeSet;

use document2html_core::{DocumentConversionOptions, DocumentFormat, DocumentInput};
use document2html_native::{NativeBackendConfig, NativeDocumentConverter};

#[path = "support/cjk.rs"]
mod cjk_support;
use cjk_support::{CJK_TEXT, UNRESOLVABLE_CJK_FAMILY, build_cjk_docx};

const REPEATS: usize = 6;

#[test]
#[ignore = "requires LibreOffice and Poppler executables"]
fn repeated_conversion_of_an_unresolvable_cjk_family_is_byte_identical() {
    // Given
    let data = build_cjk_docx(UNRESOLVABLE_CJK_FAMILY, CJK_TEXT);
    let input = DocumentInput::detect(&data, Some("cjk.docx"));
    let converter =
        NativeDocumentConverter::new(NativeBackendConfig::default()).expect("probe native runtime");

    // When
    let outputs = (0..REPEATS)
        .map(|_| {
            converter
                .convert(&input, &DocumentConversionOptions::default())
                .expect("convert CJK DOCX")
                .html
        })
        .collect::<BTreeSet<_>>();

    // Then
    assert_eq!(
        outputs.len(),
        1,
        "{REPEATS} conversions produced {} distinct HTML outputs",
        outputs.len()
    );
}

#[test]
#[ignore = "requires LibreOffice and Poppler executables"]
fn an_unresolvable_cjk_family_still_renders_east_asian_text() {
    // Given
    let data = build_cjk_docx(UNRESOLVABLE_CJK_FAMILY, CJK_TEXT);
    let input = DocumentInput::detect(&data, Some("cjk.docx"));
    let converter =
        NativeDocumentConverter::new(NativeBackendConfig::default()).expect("probe native runtime");

    // When
    let result = converter
        .convert(&input, &DocumentConversionOptions::default())
        .expect("convert CJK DOCX");

    // Then
    assert_eq!(result.format, DocumentFormat::Docx);
    for expected in ["한국어", "日本語", "简体字", "International"] {
        assert!(
            result.html.contains(expected),
            "converted HTML lost {expected}"
        );
    }
}
