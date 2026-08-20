use document2html_core::DocumentDiagnostic;

pub(crate) fn diagnostics_json(diagnostics: &[DocumentDiagnostic]) -> String {
    let entries = diagnostics
        .iter()
        .map(diagnostic_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{entries}]")
}

pub(crate) fn report_diagnostics(diagnostics: &[DocumentDiagnostic]) -> bool {
    let mut has_fallback = false;
    for diagnostic in diagnostics {
        log::warn!(
            "diagnostic code={} family={} support={} reason={}",
            diagnostic.code,
            diagnostic.family,
            diagnostic.support_tier,
            diagnostic.reason
        );
        has_fallback |= diagnostic.support_tier == "fallback";
    }
    has_fallback
}

fn diagnostic_json(diagnostic: &DocumentDiagnostic) -> String {
    format!(
        concat!(
            "{{",
            "\"code\":{},",
            "\"family\":{},",
            "\"support_tier\":{},",
            "\"stage\":{},",
            "\"raw_reference\":{},",
            "\"fallback_kind\":{},",
            "\"reason\":{}",
            "}}"
        ),
        json_string(&diagnostic.code),
        json_string(&diagnostic.family),
        json_string(&diagnostic.support_tier),
        optional_json_string(diagnostic.stage.as_deref()),
        optional_json_string(diagnostic.raw_reference.as_deref()),
        json_string(&diagnostic.fallback_kind),
        json_string(&diagnostic.reason),
    )
}

fn optional_json_string(value: Option<&str>) -> String {
    value.map(json_string).unwrap_or_else(|| "null".to_owned())
}

pub(crate) fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character <= '\u{001f}' => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped.push('"');
    escaped
}
