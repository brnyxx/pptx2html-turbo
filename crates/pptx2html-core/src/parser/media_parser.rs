use super::preserved_parser::part_diagnostic;
use crate::model::{ConversionDiagnostic, FeatureFamily};

pub(crate) fn collect_part_diagnostics(
    part_name: &str,
    diagnostics: &mut Vec<ConversionDiagnostic>,
) {
    let extension = part_name.rsplit('.').next().unwrap_or("");
    if part_name.starts_with("ppt/media/")
        && matches!(extension, "wav" | "mp3" | "mp4" | "m4a" | "avi" | "mov")
    {
        diagnostics.push(part_diagnostic(
            part_name,
            FeatureFamily::Media,
            "Timed media content is preserved but not rendered",
        ));
    }
}
