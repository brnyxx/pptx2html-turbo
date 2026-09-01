//! Diagnostics emitted when cell coordinates are deliberately withheld.
//!
//! Annotation fails closed: whenever a coordinate cannot be proven, the
//! rendered HTML is left untouched and the reason is reported here instead of
//! being guessed.

use document2html_core::DocumentDiagnostic;

use super::matching::max_match_tokens;

fn spreadsheet_diagnostic(code: &str, reason: String) -> DocumentDiagnostic {
    DocumentDiagnostic {
        code: code.to_owned(),
        family: "spreadsheet-semantics".to_owned(),
        support_tier: "approximate".to_owned(),
        stage: Some("render".to_owned()),
        raw_reference: None,
        fallback_kind: "cell-coordinate-omitted".to_owned(),
        reason,
    }
}

pub(super) fn missing_body_diagnostic() -> DocumentDiagnostic {
    spreadsheet_diagnostic(
        "SPREADSHEET_CELLS_UNMAPPED",
        "The rendered HTML has no body element, so no cell coordinates were attached".to_owned(),
    )
}

pub(super) fn overlap_diagnostic() -> DocumentDiagnostic {
    spreadsheet_diagnostic(
        "SPREADSHEET_CELLS_UNMAPPED",
        "Rendered cell values overlap, so no cell coordinates were attached".to_owned(),
    )
}

pub(super) fn ambiguous_diagnostic(count: usize) -> DocumentDiagnostic {
    spreadsheet_diagnostic(
        "SPREADSHEET_CELL_AMBIGUOUS",
        format!(
            "{count} cell value(s) matched no unique rendered occurrence, \
             so their coordinates were omitted"
        ),
    )
}

/// A cell converted normally but its number format could not be reproduced,
/// so it is excluded from attribution while the HTML is left intact.
pub(super) fn unreproducible_format_diagnostic(count: usize) -> DocumentDiagnostic {
    spreadsheet_diagnostic(
        "SPREADSHEET_CELL_FORMAT_UNREPRODUCED",
        format!(
            "{count} cell(s) use a number format whose displayed text is not \
             reproduced, so their coordinates were omitted"
        ),
    )
}

pub(super) fn truncated_diagnostic() -> DocumentDiagnostic {
    spreadsheet_diagnostic(
        "SPREADSHEET_CELL_SCAN_TRUNCATED",
        format!(
            "Rendered text exceeded the {} token matching bound, \
             so later cell coordinates were omitted",
            max_match_tokens()
        ),
    )
}
