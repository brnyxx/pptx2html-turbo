use std::fmt::Write;

use super::escape_html;

pub(super) fn render_run_wrapper(
    hyperlink: Option<&str>,
    run_style: &str,
    segment_html: &str,
    html: &mut String,
) {
    if let Some(href) = hyperlink {
        let _ = write!(
            html,
            "<a class=\"run\" href=\"{}\" style=\"{run_style}\">{segment_html}</a>",
            escape_html(href)
        );
    } else {
        let _ = write!(
            html,
            "<span class=\"run\" style=\"{run_style}\">{segment_html}</span>"
        );
    }
}
