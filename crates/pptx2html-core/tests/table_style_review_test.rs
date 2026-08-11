#[path = "table_style_support/review.rs"]
mod review;

use pptx2html_core::{convert_bytes_with_metadata, model::ShapeType, parser::PptxParser};

fn convert(data: &[u8]) -> pptx2html_core::ConversionResult {
    convert_bytes_with_metadata(data).expect("fixture converts")
}

fn has_diagnostic(result: &pptx2html_core::ConversionResult, code: &str) -> bool {
    result.diagnostics.iter().any(|item| item.code == code)
}

#[test]
fn hostile_relationship_order_never_preempts_valid_internal_style_part() {
    let hostile = review::table_styles_relationship(
        "rIdHostile",
        "https://user:secret@example.test/x",
        Some("External"),
    );
    let valid = review::table_styles_relationship("rIdValid", "tableStyles.xml", None);
    for relationships in [format!("{hostile}{valid}"), format!("{valid}{hostile}")] {
        let result = convert(&review::relationship_package(
            &relationships,
            Some(review::valid_styles()),
        ));
        assert!(result.html.contains("background-color: #0D0D0D"));
        let rejected: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|item| item.code == "TABLE_STYLE_RELATIONSHIP_REJECTED")
            .collect();
        assert_eq!(rejected.len(), 1);
        let raw = rejected[0].raw_reference.as_deref().unwrap_or_default();
        assert!(raw.contains("relationship_id=rIdHostile"));
        assert!(!raw.contains("secret"));
        assert!(!raw.contains("example.test"));
    }
}

#[test]
fn unsafe_external_and_spoof_relationships_never_select_a_zip_entry() {
    for target in [
        "",
        "/ppt/tableStyles.xml",
        "../tableStyles.xml",
        "./tableStyles.xml",
        "tables//styles.xml",
        "tables\\styles.xml",
        "https://example.test/styles.xml",
        "%2e%2e/tableStyles.xml",
    ] {
        let hostile = review::table_styles_relationship("rIdUnsafe", target, None);
        let valid = review::table_styles_relationship("rIdValid", "tableStyles.xml", None);
        let result = convert(&review::relationship_package(
            &format!("{hostile}{valid}"),
            Some(review::valid_styles()),
        ));
        assert!(result.html.contains("background-color: #0D0D0D"));
        assert!(has_diagnostic(&result, "TABLE_STYLE_RELATIONSHIP_REJECTED"));
    }

    let result = convert(&review::relationship_package(
        &review::spoof_relationship("rIdSpoof", "tableStyles.xml"),
        Some(review::valid_styles()),
    ));
    assert!(!result.html.contains("background-color: #0D0D0D"));
}

#[test]
fn duplicate_missing_and_malformed_style_parts_have_typed_diagnostics() {
    let duplicates = format!(
        "{}{}",
        review::table_styles_relationship("rIdB", "tableStyles.xml", None),
        review::table_styles_relationship("rIdA", "tableStyles.xml", None)
    );
    let duplicate_result = convert(&review::relationship_package(
        &duplicates,
        Some(review::valid_styles()),
    ));
    assert!(has_diagnostic(
        &duplicate_result,
        "TABLE_STYLE_RELATIONSHIP_DUPLICATE"
    ));

    let relationship = review::table_styles_relationship("rIdStyles", "missing.xml", None);
    let missing_result = convert(&review::relationship_package(&relationship, None));
    assert!(has_diagnostic(&missing_result, "TABLE_STYLE_PART_MISSING"));

    let malformed_result = convert(&review::strict_style_package("<a:tblStyleLst"));
    assert!(has_diagnostic(
        &malformed_result,
        "TABLE_STYLE_PART_MALFORMED"
    ));
}

#[test]
fn active_xml_tokens_are_rejected_by_table_style_parser() {
    for active in [
        "unexpected",
        "<![CDATA[unexpected]]>",
        "<?review active?>",
        "<!DOCTYPE tblStyleLst>",
    ] {
        let xml = review::valid_styles().replacen('>', &format!(">{active}"), 1);
        let result = convert(&review::strict_style_package(&xml));
        assert!(!result.html.contains("background-color: #0D0D0D"));
        assert!(has_diagnostic(&result, "TABLE_STYLE_XML_INVALID"));
    }
}

#[test]
fn wrong_namespace_and_out_of_context_primitives_are_not_applied() {
    for injected in [
        r#"<x:wholeTbl xmlns:x="urn:spoof"><x:tcStyle><x:fill><x:solidFill><x:srgbClr val="ABCDEF"/></x:solidFill></x:fill></x:tcStyle></x:wholeTbl>"#,
        r#"<a:tblBg><x:effectRef xmlns:x="urn:spoof" idx="1"><x:schemeClr val="accent1"/></x:effectRef></a:tblBg>"#,
        r#"<a:wholeTbl><a:tcStyle><a:tcBdr><a:left><x:lnRef xmlns:x="urn:spoof" idx="1"><x:schemeClr val="accent1"/></x:lnRef></a:left></a:tcBdr></a:tcStyle></a:wholeTbl>"#,
        r#"<a:wholeTbl><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="ABCDEF"/></a:solidFill></a:fill></a:tcStyle></a:wholeTbl>"#,
        r#"<a:srgbClr val="ABCDEF"/>"#,
        r#"<a:effectRef idx="1"><a:schemeClr val="accent1"/></a:effectRef>"#,
        r#"<a:lnRef idx="1"><a:schemeClr val="accent1"/></a:lnRef>"#,
    ] {
        let xml =
            review::valid_styles().replace("<a:tblStyle ", &format!("{injected}<a:tblStyle "));
        let result = convert(&review::strict_style_package(&xml));
        assert!(!result.html.contains("#ABCDEF"));
        assert!(has_diagnostic(&result, "TABLE_STYLE_XML_INVALID"));
    }
}

#[test]
fn declarations_comments_and_whitespace_remain_valid() {
    let xml = review::valid_styles().replace("<a:tblStyle ", "\n<!-- passive -->\n<a:tblStyle ");
    let result = convert(&review::strict_style_package(&xml));
    assert!(result.html.contains("background-color: #0D0D0D"));
    assert!(!has_diagnostic(&result, "TABLE_STYLE_XML_INVALID"));
}

#[test]
fn unavailable_non_solid_fill_ref_is_preserved_without_inventing_a_solid_fill() {
    let package = review::fill_ref_package();
    let presentation = PptxParser::parse_bytes(&package).expect("fillRef fixture parses");
    let ShapeType::Table(table) = &presentation.slides[0].shapes[0].shape_type else {
        panic!("shape is a table");
    };
    let definition = table
        .style
        .as_ref()
        .and_then(|style| style.definition.as_ref())
        .expect("package style definition");
    let fill_ref = definition.regions[0]
        .1
        .fill_ref
        .as_ref()
        .expect("typed fillRef");
    assert_eq!(fill_ref.idx, 2);
    assert_eq!(fill_ref.color.modifiers.len(), 1);
    assert_eq!(presentation.themes[0].fmt_scheme.fill_style_lst.len(), 1);
    let result = convert(&package);
    assert!(result.html.contains(">cell</span>"));
    assert!(!result.html.contains("linear-gradient("));
    assert!(!result.html.contains("background-color: #222222"));
    let diagnostics: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|item| item.code == "TABLE_STYLE_PRIMITIVE_UNSUPPORTED")
        .collect();
    assert_eq!(diagnostics.len(), 1);
    let raw = diagnostics[0].raw_reference.as_deref().unwrap_or_default();
    assert!(raw.contains("table_id=2"));
    assert!(raw.contains("fill_ref_idx=2"));
    assert!(raw.contains("Theme(\"accent2\")"));
    assert!(raw.contains("Tint(20000)"));
    assert!(!result.html.contains("secret"));
    assert!(!result.html.contains("NaN"));
    assert!(!result.html.contains("Infinity"));
}

#[test]
fn diagnostics_keep_each_table_identity_and_slide_position() {
    let package = review::diagnostic_identity_package();
    let presentation = PptxParser::parse_bytes(&package).expect("identity fixture parses");
    assert_eq!(presentation.slides[0].shapes[0].id, 20);
    assert_eq!(presentation.slides[0].shapes[1].id, 21);
    assert_eq!(presentation.slides[1].shapes[0].id, 30);
    let result = convert(&package);
    let diagnostics: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|item| item.code == "TABLE_STYLE_DEFINITION_UNAVAILABLE")
        .collect();
    assert_eq!(diagnostics.len(), 3);
    let identities: Vec<_> = diagnostics
        .iter()
        .map(|item| item.raw_reference.as_deref().unwrap_or_default())
        .collect();
    assert!(identities[0].contains("table_id=20"));
    assert!(identities[1].contains("table_id=21"));
    assert!(identities[2].contains("table_id=30"));
    assert_eq!(diagnostics[0].location.slide_index, Some(0));
    assert_eq!(diagnostics[1].location.slide_index, Some(0));
    assert_eq!(diagnostics[2].location.slide_index, Some(1));
    assert!(
        diagnostics
            .iter()
            .all(|item| item.location.relationship_id.is_none())
    );
}

#[test]
fn vertical_merge_zero_grid_and_explicit_text_formatting_are_stable() {
    let result = convert(&review::boundary_package());
    assert!(result.html.contains("rowspan=\"2\""));
    assert!(!result.html.contains("data-table-cell=\"r1c0\""));
    assert!(result.html.contains(
        "data-table-cell=\"r1c1\" data-table-style-region=\"seCell\" style=\"background-color: #090909"
    ));
    assert!(result.html.contains("color: #FF0000"));
    assert!(!result.html.contains("NaN"));
    assert!(!result.html.contains("Infinity"));
}

#[test]
fn hostile_fill_ref_and_multiplicity_manual_qa_package_is_convertible() {
    let package = review::manual_qa_package();
    let result = convert(&package);
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == "TABLE_STYLE_DEFINITION_UNAVAILABLE")
            .count(),
        3
    );
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == "TABLE_STYLE_RELATIONSHIP_REJECTED")
            .count(),
        1
    );
    assert!(result.html.contains("background-color: #0D0D0D"));
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.code == "TABLE_STYLE_PRIMITIVE_UNSUPPORTED")
            .count(),
        3
    );
    review::assert_reference_diagnostics(&result);
    assert!(!has_diagnostic(&result, "TABLE_STYLE_XML_INVALID"));
    assert!(!result.diagnostics.iter().any(|d| {
        d.code.starts_with("OOXML_")
            && d.location.part_name.as_deref() == Some("ppt/tableStyles.xml")
    }));
}
