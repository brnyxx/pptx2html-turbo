use quick_xml::events::BytesStart;

pub(crate) fn hyperlink_rel_id(element: &BytesStart<'_>) -> Option<String> {
    element.attributes().flatten().find_map(|attribute| {
        let key = std::str::from_utf8(attribute.key.as_ref()).unwrap_or("");
        (key.ends_with("id") && key.contains(':'))
            .then(|| String::from_utf8_lossy(&attribute.value).to_string())
    })
}
