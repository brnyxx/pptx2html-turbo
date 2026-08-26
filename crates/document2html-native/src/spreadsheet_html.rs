//! Attaches workbook cell coordinates to rendered spreadsheet HTML.
//!
//! Matching is evidence-first: a coordinate is emitted only when it is proven
//! unique. Tokenization lives in [`text`], the indexed scan in [`matching`],
//! and refusal reporting in [`diagnostics`].

use std::collections::{BTreeMap, HashMap};

use document2html_core::{DocumentDiagnostic, SpreadsheetCell, SpreadsheetSemantics};

mod diagnostics;
mod matching;
mod text;

use diagnostics::{
    ambiguous_diagnostic, missing_body_diagnostic, overlap_diagnostic, truncated_diagnostic,
    unreproducible_format_diagnostic,
};
use matching::scan_occurrences;
use text::{body_range, normalize};

struct Annotation {
    start: usize,
    end: usize,
    cell: usize,
}

pub(crate) struct AnnotatedSpreadsheetHtml {
    pub(crate) html: String,
    pub(crate) diagnostics: Vec<DocumentDiagnostic>,
    /// Phrase lookups performed while matching. Used only by tests to pin the
    /// matching cost model; conversion output does not depend on it.
    #[cfg(test)]
    pub(crate) probes: usize,
}

/// Wraps rendered cell text in coordinate-bearing spans.
///
/// A cell is annotated only when its displayed value is unambiguous: exactly
/// one cell carries the value and exactly one rendered occurrence matches it.
/// Duplicate values are never zipped positionally onto duplicate occurrences,
/// because the converter does not prove that HTML order matches workbook
/// order. Every refusal is reported as a diagnostic instead of being guessed.
pub(crate) fn annotate_spreadsheet_html(
    html: &str,
    semantics: &SpreadsheetSemantics,
) -> AnnotatedSpreadsheetHtml {
    let Some(content) = body_range(html) else {
        return AnnotatedSpreadsheetHtml {
            html: html.to_owned(),
            diagnostics: vec![missing_body_diagnostic()],
            #[cfg(test)]
            probes: 0,
        };
    };

    // Group cells by normalized value so duplicate values are refused as a set
    // rather than matched one at a time. Cells whose displayed text could not
    // be reproduced are counted but never matched, so no coordinate is claimed
    // for text that may differ from the rendering.
    let mut cells_by_value: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut unreproducible = 0usize;
    for (index, cell) in semantics.cells.iter().enumerate() {
        if !cell.attributable {
            unreproducible += 1;
            continue;
        }
        let value = normalize(&cell.displayed_value);
        if !value.is_empty() {
            cells_by_value.entry(value).or_default().push(index);
        }
    }
    let wanted: HashMap<&str, usize> = cells_by_value
        .iter()
        .filter(|(_, cells)| cells.len() == 1)
        .map(|(value, cells)| (value.as_str(), cells[0]))
        .collect();

    let scan = scan_occurrences(html, &content, &wanted);
    let mut annotations = Vec::new();
    let mut ambiguous = 0usize;
    for (value, cells) in &cells_by_value {
        if cells.len() > 1 {
            ambiguous += 1;
            continue;
        }
        match scan.occurrences.get(value.as_str()) {
            Some(matches) if matches.len() == 1 => annotations.push(Annotation {
                start: matches[0].0,
                end: matches[0].1,
                cell: cells[0],
            }),
            Some(_) => ambiguous += 1,
            None => {}
        }
    }

    annotations.sort_by_key(|annotation| annotation.start);
    // Adjacent-pair comparison suffices on sorted, per-value-unique spans and
    // avoids the quadratic all-pairs conflict scan.
    if annotations
        .windows(2)
        .any(|pair| pair[0].end > pair[1].start)
    {
        return AnnotatedSpreadsheetHtml {
            html: html.to_owned(),
            diagnostics: vec![overlap_diagnostic()],
            #[cfg(test)]
            probes: scan.probes,
        };
    }

    let mut result = Vec::new();
    if ambiguous > 0 {
        result.push(ambiguous_diagnostic(ambiguous));
    }
    if unreproducible > 0 {
        result.push(unreproducible_format_diagnostic(unreproducible));
    }
    if scan.truncated {
        result.push(truncated_diagnostic());
    }

    let mut output = html.to_owned();
    for annotation in annotations.into_iter().rev() {
        let cell = &semantics.cells[annotation.cell];
        output.insert_str(annotation.end, "</span>");
        output.insert_str(annotation.start, &opening_span(cell));
    }
    AnnotatedSpreadsheetHtml {
        html: output,
        diagnostics: result,
        #[cfg(test)]
        probes: scan.probes,
    }
}

fn opening_span(cell: &SpreadsheetCell) -> String {
    format!(
        "<span data-cell-coordinate=\"{}\" data-worksheet=\"{}\">",
        escape_attribute(&cell.coordinate),
        escape_attribute(&cell.worksheet)
    )
}

fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
#[path = "spreadsheet_html_tests.rs"]
mod tests;
