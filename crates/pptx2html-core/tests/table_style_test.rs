mod table_style_support;

use pptx2html_core::{
    convert_bytes_with_metadata,
    model::{ShapeType, TableStyle, TableStyleReference, TableStyleSourceKind},
    parser::PptxParser,
};
use table_style_support::{
    BUILT_IN_STYLE, INVALID_STYLE, OTHER_BUILT_IN_STYLE, cdata_id_package, corner_gate_package,
    duplicate_style_package, empty_id_package, invalid_bool_package, invalid_package,
    merged_package, other_built_in_package, package, spoof_namespace_package,
    style_feature_package, unsupported_primitive_package, wrong_style_namespace_package,
};

#[test]
fn parses_style_id_flags_and_office_region_precedence() {
    let bytes = package();
    let presentation = PptxParser::parse_bytes(&bytes).expect("table style fixture parses");
    let ShapeType::Table(table) = &presentation.slides[0].shapes[0].shape_type else {
        panic!("first shape is a table");
    };

    assert_eq!(
        table.style.as_ref().map(|style| style.id.as_str()),
        Some("{11111111-1111-1111-1111-111111111111}")
    );
    assert!(table.first_row && table.last_row && table.first_col && table.last_col);
    assert!(table.band_row && table.band_col);
    assert_eq!(presentation.slides[0].shapes.len(), 2);
    let ShapeType::Table(unavailable) = &presentation.slides[0].shapes[1].shape_type else {
        panic!("second shape is a table");
    };
    assert_eq!(
        unavailable
            .style
            .as_ref()
            .map(|style| style.source_kind.as_str()),
        Some("built_in")
    );

    let result = convert_bytes_with_metadata(&bytes).expect("table style fixture converts");
    for expected in [
        "background-color: #0D0D0D",
        "background-color: #0B0B0B",
        "background-color: #0C0C0C",
        "background-color: #070707",
        "background-color: #ABCDEF",
        "background-color: #0A0A0A",
        "background-color: #080808",
        "background-color: #090909",
    ] {
        assert!(
            result.html.contains(expected),
            "missing computed style {expected}"
        );
    }
    assert!(result.html.contains("border-top: 1.0pt solid #FF0000"));
    assert!(result.html.contains("background-color: transparent"));
}

#[test]
fn built_in_medium_style_two_renders_default_header_and_row_banding() {
    let result = convert_bytes_with_metadata(&table_style_support::built_in_default_package())
        .expect("built-in table style fixture converts");

    assert!(result.html.contains("background-color: #4F81BD"));
    assert!(result.html.contains("background-color: #D0D8E7"));
    assert!(result.html.contains("background-color: #E9ECF3"));
    assert!(result.html.contains("color: #FFFFFF"));
}

#[test]
fn unavailable_built_in_preserves_id_and_six_flags_in_one_diagnostic() {
    let result = convert_bytes_with_metadata(&package()).expect("table style fixture converts");
    let diagnostics = result
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "TABLE_STYLE_DEFINITION_UNAVAILABLE")
        .collect::<Vec<_>>();

    assert_eq!(
        diagnostics.len(),
        1,
        "all diagnostics: {:?}",
        result.diagnostics
    );
    let raw = diagnostics[0]
        .raw_reference
        .as_deref()
        .expect("style diagnostic carries machine-readable metadata");
    assert!(raw.contains(BUILT_IN_STYLE));
    assert!(raw.contains("source_kind=built_in"));
    for flag in [
        "firstRow=1",
        "lastRow=1",
        "firstCol=1",
        "lastCol=1",
        "bandRow=1",
        "bandCol=1",
    ] {
        assert!(raw.contains(flag), "missing preserved flag {flag}");
    }
    assert!(
        result
            .html
            .contains(&format!("data-table-style-id=\"{BUILT_IN_STYLE}\""))
    );
    assert!(!result.html.contains("OOXML_UNKNOWN_ELEMENT"));
}

#[test]
fn invalid_id_is_distinct_from_a_valid_built_in_without_a_definition() {
    let result = convert_bytes_with_metadata(&invalid_package()).expect("invalid style converts");
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "TABLE_STYLE_DEFINITION_UNAVAILABLE")
        .expect("invalid style is diagnosed");
    let raw = diagnostic.raw_reference.as_deref().expect("raw metadata");
    assert!(raw.contains(INVALID_STYLE));
    assert!(raw.contains("source_kind=invalid"));
    assert!(!raw.contains("source_kind=built_in"));
}

#[test]
fn every_official_built_in_id_uses_the_built_in_source_kind() {
    let result = convert_bytes_with_metadata(&other_built_in_package())
        .expect("other official built-in converts");
    let raw = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "TABLE_STYLE_DEFINITION_UNAVAILABLE")
        .and_then(|diagnostic| diagnostic.raw_reference.as_deref())
        .expect("unavailable built-in diagnostic");
    assert!(raw.contains(OTHER_BUILT_IN_STYLE));
    assert!(raw.contains("source_kind=built_in"));
}

#[test]
fn empty_table_style_id_is_preserved_without_inventing_a_style() {
    let presentation = PptxParser::parse_bytes(&empty_id_package()).expect("empty id parses");
    let ShapeType::Table(table) = &presentation.slides[0].shapes[0].shape_type else {
        panic!("shape is table");
    };
    assert_eq!(
        table.style.as_ref().map(|style| style.id.as_str()),
        Some("")
    );
}

#[test]
fn corner_regions_require_both_corresponding_row_and_column_flags() {
    let result =
        convert_bytes_with_metadata(&corner_gate_package()).expect("corner fixture converts");
    assert!(
        result
            .html
            .contains("data-table-cell=\"r0c0\" data-table-style-region=\"firstRow\"")
    );
    assert!(
        !result
            .html
            .contains("data-table-cell=\"r0c0\" data-table-style-region=\"nwCell\"")
    );
}

#[test]
fn logical_grid_skips_horizontal_merge_continuations_and_uses_grid_span() {
    let result = convert_bytes_with_metadata(&merged_package()).expect("merged fixture converts");
    assert_eq!(result.html.matches("<td").count(), 3);
    assert!(
        result
            .html
            .contains("data-table-cell=\"r0c3\" data-table-style-region=\"lastCol\"")
    );
    assert!(
        !result
            .html
            .contains("data-table-cell=\"r0c2\" data-table-style-region=\"lastCol\"")
    );
}

#[test]
fn adjacent_spoof_namespace_remains_an_unsupported_element_diagnostic() {
    let result =
        convert_bytes_with_metadata(&spoof_namespace_package()).expect("spoof fixture converts");
    assert!(!result.html.contains("data-table-style-id"));
    assert!(!result.html.contains("background-color: #0D0D0D"));
    assert!(result.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "OOXML_ELEMENT_UNSUPPORTED"
            && diagnostic.location.qualified_element_name.as_deref() == Some("x:tableStyleId")
    }));
}

#[test]
fn cdata_style_id_and_supported_table_primitives_reach_public_html() {
    let cdata = PptxParser::parse_bytes(&cdata_id_package()).expect("CDATA style ID parses");
    let ShapeType::Table(table) = &cdata.slides[0].shapes[0].shape_type else {
        panic!("table");
    };
    assert_eq!(
        table.style.as_ref().map(|style| style.id.as_str()),
        Some("{11111111-1111-1111-1111-111111111111}")
    );

    let result =
        convert_bytes_with_metadata(&style_feature_package()).expect("style primitives convert");
    assert!(result.html.contains("background-color: #101010"));
    assert!(result.html.contains("font-weight: bold"));
    assert!(result.html.contains("font-style: italic"));
    assert!(result.html.contains("color: #808080"));
    assert!(result.html.contains("font-family: 'Minor'"));
    assert!(result.html.contains("border-left: 1.0pt solid #0000AA"));
    assert!(result.html.contains("border-bottom: 1.0pt solid #00AA00"));
}

#[test]
fn wrong_namespace_style_definition_is_never_applied() {
    let result = convert_bytes_with_metadata(&wrong_style_namespace_package())
        .expect("wrong namespace converts");
    assert!(!result.html.contains("background-color: #0D0D0D"));
    assert!(result.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "TABLE_STYLE_XML_INVALID"
            && diagnostic.location.part_name.as_deref() == Some("ppt/tableStyles.xml")
    }));
}

#[test]
fn unsupported_style_primitive_is_a_stable_typed_diagnostic() {
    let result = convert_bytes_with_metadata(&unsupported_primitive_package())
        .expect("unsupported primitive converts");
    assert!(result.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "TABLE_STYLE_PRIMITIVE_UNSUPPORTED"
            && diagnostic.location.qualified_element_name.as_deref() == Some("a:gradFill")
    }));
}

#[test]
fn duplicate_style_ids_use_the_first_definition_and_emit_one_diagnostic() {
    let result = convert_bytes_with_metadata(&duplicate_style_package())
        .expect("duplicate style fixture converts");
    assert!(result.html.contains("background-color: #0D0D0D"));
    assert!(!result.html.contains("background-color: #FEED01"));
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "TABLE_STYLE_DUPLICATE_ID")
            .count(),
        1
    );
}

#[test]
fn invalid_table_boolean_falls_back_false_and_emits_typed_diagnostic() {
    let result =
        convert_bytes_with_metadata(&invalid_bool_package()).expect("invalid bool converts");
    assert!(!result.html.contains("data-table-style-region=\"firstRow\""));
    let diagnostic = result
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.code == "TABLE_STYLE_INVALID_BOOLEAN")
        .expect("invalid boolean diagnostic");
    let raw = diagnostic.raw_reference.as_deref().unwrap_or_default();
    assert!(raw.contains("table_id=2"));
    assert!(raw.contains("firstRow=maybe"));
}

#[test]
fn table_style_metadata_keeps_shape_type_below_large_variant_threshold() {
    assert!(std::mem::size_of::<ShapeType>() <= 256);
}

#[test]
fn public_table_style_definition_uses_indirect_storage() {
    let reference = TableStyleReference {
        id: "{11111111-1111-1111-1111-111111111111}".to_owned(),
        source_kind: TableStyleSourceKind::Package,
        definition: Some(Box::new(TableStyle::default())),
        issues: Vec::new(),
    };

    assert!(reference.definition.is_some());
}
