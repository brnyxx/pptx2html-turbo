use super::preserved_parser::part_diagnostic;
use crate::model::{ConversionDiagnostic, FeatureFamily};

pub(crate) fn collect_part_diagnostics(
    part_name: &str,
    diagnostics: &mut Vec<ConversionDiagnostic>,
) {
    if part_name.starts_with("ppt/notesSlides/") || part_name.starts_with("ppt/comments/") {
        diagnostics.push(part_diagnostic(
            part_name,
            FeatureFamily::Unsupported,
            "Off-slide annotations are preserved but not rendered",
        ));
    }
}
