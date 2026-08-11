use quick_xml::events::BytesStart;
use std::collections::HashMap;

use super::text_parser::RunBuilder;

pub(crate) fn hyperlink_rel_id(element: &BytesStart<'_>) -> Option<String> {
    element.attributes().flatten().find_map(|attribute| {
        let key = std::str::from_utf8(attribute.key.as_ref()).unwrap_or("");
        (key.ends_with("id") && key.contains(':'))
            .then(|| String::from_utf8_lossy(&attribute.value).to_string())
    })
}

pub(crate) fn assign_hyperlink(
    element: &BytesStart<'_>,
    relationships: &HashMap<String, String>,
    shape_run: &mut Option<RunBuilder>,
    cell_run: &mut Option<RunBuilder>,
) {
    let target = hyperlink_rel_id(element).and_then(|id| relationships.get(&id).cloned());
    if let Some(run) = shape_run.as_mut() {
        run.hyperlink = target.clone();
    }
    if let Some(run) = cell_run.as_mut() {
        run.hyperlink = target;
    }
}
