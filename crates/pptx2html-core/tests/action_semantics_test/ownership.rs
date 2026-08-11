use pptx2html_core::model::{ActionTarget, ShapeType};
use pptx2html_core::{convert_bytes_with_metadata, parser::PptxParser};

use super::fixtures::MinimalPptx;

const RELS: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rIdFrame" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://frame.example.test" TargetMode="External"/>
<Relationship Id="rIdCell" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="mailto:cell@example.test" TargetMode="External"/>
<Relationship Id="rIdGroup" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://group.example.test" TargetMode="External"/>
</Relationships>"#;

#[test]
fn table_frame_and_cell_run_keep_independent_actions_and_reachable_dom() {
    let body = r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="20" name="action table"><a:hlinkClick r:id="rIdFrame"/><a:hlinkMouseOver action="ppaction://program"/></p:cNvPr><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr/><a:tblGrid><a:gridCol w="2000000"/></a:tblGrid><a:tr h="500000"><a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr><a:hlinkClick r:id="rIdCell"/></a:rPr><a:t>TABLE_CELL_ACTION</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#;
    let bytes = MinimalPptx::new(body).with_slide_rels(RELS).build();
    let presentation = PptxParser::parse_bytes(&bytes).expect("table fixture parses");
    let table_shape = &presentation.slides[0].shapes[0];

    assert_eq!(
        (table_shape.id, table_shape.name.as_str()),
        (20, "action table")
    );
    assert!(matches!(
        table_shape.actions.click.as_ref().map(|action| &action.target),
        Some(ActionTarget::ExternalUri(uri)) if uri == "https://frame.example.test"
    ));
    assert!(matches!(
        table_shape.actions.hover.as_ref().map(|action| &action.target),
        Some(ActionTarget::Unsupported(raw)) if raw == "ppaction://program"
    ));
    let ShapeType::Table(table) = &table_shape.shape_type else {
        panic!("graphic frame is a table");
    };
    let run = &table.rows[0].cells[0]
        .text_body
        .as_ref()
        .expect("cell text")
        .paragraphs[0]
        .runs[0];
    assert!(matches!(
        run.actions.click.as_ref().map(|action| &action.target),
        Some(ActionTarget::ExternalUri(uri)) if uri == "mailto:cell@example.test"
    ));

    let result = convert_bytes_with_metadata(&bytes).expect("table fixture converts");
    let frame = result
        .html
        .find("href=\"https://frame.example.test\"")
        .expect("frame anchor");
    let table = result.html.find("<table").expect("table");
    let cell = result
        .html
        .find("href=\"mailto:cell@example.test\"")
        .expect("cell anchor");
    assert!(
        frame < table && table < cell,
        "anchors are siblings, not nested"
    );
    assert!(result.html.contains(".shape-action-surface~table"));
    assert!(result.html.contains(
        ".shape-action-surface~.text-body,.shape-action-surface~table{position:relative}"
    ));
    assert!(result.html.contains("pointer-events:none"));
    assert!(result.html.contains(".run[data-action]"));
    assert!(result.html.contains("pointer-events:auto"));
    assert!(result.diagnostics.iter().any(|item| {
        item.code == "ACTION_UNSUPPORTED" && item.reason.contains("identity=shape-20")
    }));
}

#[test]
fn group_owners_keep_identity_and_actions_without_leaking_to_children() {
    let body = r#"<p:grpSp><p:nvGrpSpPr><p:cNvPr id="30" name="outer group"><a:hlinkClick r:id="rIdGroup"/><a:hlinkMouseOver action="ppaction://hlinkshowjump?jump=lastslide"/></p:cNvPr><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:grpSp><p:nvGrpSpPr><p:cNvPr id="31" name="inner group"><a:hlinkClick action="ppaction://hlinkshowjump?jump=nextslide"/></p:cNvPr><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="32" name="leaf"><a:hlinkClick action="ppaction://hlinkshowjump?jump=previousslide"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:nvGrpSpPr><p:cNvPr id="999" name="fake group"><a:hlinkClick r:id="rIdGroup"/></p:cNvPr></p:nvGrpSpPr><p:spPr/></p:sp></p:grpSp></p:grpSp>"#;
    let bytes = MinimalPptx::new(body).with_slide_rels(RELS).build();
    let presentation = PptxParser::parse_bytes(&bytes).expect("group fixture parses");
    let outer = &presentation.slides[0].shapes[0];

    assert_eq!((outer.id, outer.name.as_str()), (30, "outer group"));
    assert!(matches!(
        outer.actions.click.as_ref().map(|action| &action.target),
        Some(ActionTarget::ExternalUri(_))
    ));
    assert!(matches!(
        outer.actions.hover.as_ref().map(|action| &action.target),
        Some(ActionTarget::Last)
    ));
    let ShapeType::Group(outer_children, _) = &outer.shape_type else {
        panic!("outer group");
    };
    let inner = &outer_children[0];
    assert_eq!((inner.id, inner.name.as_str()), (31, "inner group"));
    assert!(matches!(
        inner.actions.click.as_ref().map(|action| &action.target),
        Some(ActionTarget::Next)
    ));
    let ShapeType::Group(inner_children, _) = &inner.shape_type else {
        panic!("inner group");
    };
    assert_eq!(inner_children[0].id, 32);
    assert!(matches!(
        inner_children[0]
            .actions
            .click
            .as_ref()
            .map(|action| &action.target),
        Some(ActionTarget::Previous)
    ));

    let result = convert_bytes_with_metadata(&bytes).expect("group fixture converts");
    assert!(result.html.contains("aria-label=\"shape 30\""));
    assert!(result.html.contains("aria-label=\"shape 31\""));
    assert!(result.html.contains("aria-label=\"shape 32\""));
    assert!(result.html.contains(".shape-action-surface~.shape"));
    assert!(result.html.contains("pointer-events:none"));
}

#[test]
fn only_presentationml_c_nv_pr_in_an_allowed_parent_authorizes_actions() {
    let body = r#"<p:sp><p:nvSpPr><p:cNvPr id="50" name="valid start"><a:hlinkClick r:id="rIdGroup"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="51" name="valid empty"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp>
<p:sp xmlns:x="urn:evil"><p:nvSpPr><x:cNvPr id="666" name="evil start"><a:hlinkClick r:id="rIdGroup"/></x:cNvPr><x:cNvPr id="667" name="evil empty"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp>
<p:sp><p:nvSpPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><p:cNvPr id="668" name="wrong parent"><a:hlinkClick r:id="rIdGroup"/></p:cNvPr></p:spPr></p:sp>
<p:sp xmlns:x="urn:evil"><x:nvSpPr><p:cNvPr id="669" name="evil nv parent"><a:hlinkClick r:id="rIdGroup"/></p:cNvPr></x:nvSpPr><p:spPr/></p:sp>
<p:sp><p:nvGraphicFramePr><p:cNvPr id="670" name="wrong owner kind"><a:hlinkClick r:id="rIdGroup"/></p:cNvPr></p:nvGraphicFramePr><p:spPr/></p:sp>
<p:sp><p:spPr><p:nvSpPr><p:cNvPr id="671" name="nested lookalike"><a:hlinkClick r:id="rIdGroup"/></p:cNvPr></p:nvSpPr></p:spPr></p:sp>
<p:sp xmlns:x="urn:evil"><p:nvSpPr><p:cNvPr id="52" name="nested wrapper"><x:wrapper><a:hlinkClick r:id="rIdGroup"/></x:wrapper><x:cNvPr><a:hlinkClick r:id="rIdGroup"/></x:cNvPr></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp>"#;
    let bytes = MinimalPptx::new(body).with_slide_rels(RELS).build();
    let presentation = PptxParser::parse_bytes(&bytes).expect("namespace fixture parses");

    assert_eq!(presentation.slides[0].shapes[0].id, 50);
    assert_eq!(presentation.slides[0].shapes[1].id, 51);
    for shape in &presentation.slides[0].shapes[2..7] {
        assert_eq!(shape.id, 0);
        assert!(shape.name.is_empty());
        assert!(shape.actions.click.is_none());
    }
    assert_eq!(presentation.slides[0].shapes[7].id, 52);
    assert!(presentation.slides[0].shapes[7].actions.click.is_none());
    let result = convert_bytes_with_metadata(&bytes).expect("namespace fixture converts");
    assert_eq!(
        result
            .html
            .matches("href=\"https://group.example.test\"")
            .count(),
        1
    );
}

#[test]
fn run_diagnostic_identity_is_owner_derived_and_exact_duplicates_collapse() {
    fn render(body: &str) -> pptx2html_core::ConversionResult {
        convert_bytes_with_metadata(&MinimalPptx::new(body).build()).expect("fixture converts")
    }
    let action_shape = r#"<p:sp><p:nvSpPr><p:cNvPr id="42" name="owned run"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:p><a:r><a:rPr><a:hlinkClick action="ppaction://program"/></a:rPr><a:t>OWNED_ACTION</a:t></a:r></a:p></p:txBody></p:sp>"#;
    let unrelated = r#"<p:sp><p:nvSpPr><p:cNvPr id="41" name="unrelated"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:p><a:r><a:t>PRIOR_RUN</a:t></a:r></a:p></p:txBody></p:sp>"#;
    let base = render(action_shape);
    let inserted = render(&format!("{unrelated}{action_shape}"));
    let reason = |result: &pptx2html_core::ConversionResult| {
        result
            .diagnostics
            .iter()
            .find(|item| item.code == "ACTION_UNSUPPORTED")
            .expect("action diagnostic")
            .reason
            .clone()
    };
    assert_eq!(reason(&base), reason(&inserted));
    assert!(reason(&base).contains("slide-1/shape-42/paragraph-0/run-0"));

    let bytes = MinimalPptx::new(action_shape).build();
    let mut presentation = PptxParser::parse_bytes(&bytes).expect("fixture parses");
    let actions = &mut presentation.slides[0].shapes[0]
        .text_body
        .as_mut()
        .expect("text")
        .paragraphs[0]
        .runs[0]
        .actions;
    actions.hover = actions.click.clone();
    let duplicate = pptx2html_core::renderer::HtmlRenderer::render_with_options_metadata(
        &presentation,
        &pptx2html_core::ConversionOptions::default(),
    )
    .expect("model renders");
    assert_eq!(
        duplicate
            .diagnostics
            .iter()
            .filter(|item| item.code == "ACTION_UNSUPPORTED")
            .count(),
        1
    );
}

#[test]
fn identical_table_cell_actions_keep_distinct_cell_identities() {
    let cell = |label: &str| {
        format!(
            r#"<a:tc><a:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:rPr><a:hlinkClick action="ppaction://program"/></a:rPr><a:t>{label}</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>"#
        )
    };
    let body = format!(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="60" name="two cells"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr/><a:tblGrid><a:gridCol w="1000000"/><a:gridCol w="1000000"/></a:tblGrid><a:tr h="500000">{}{}</a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#,
        cell("CELL_A"),
        cell("CELL_B")
    );
    let result = convert_bytes_with_metadata(&MinimalPptx::new(&body).build())
        .expect("two-cell fixture converts");
    let reasons: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|item| item.code == "ACTION_UNSUPPORTED")
        .map(|item| item.reason.as_str())
        .collect();

    assert_eq!(reasons.len(), 2);
    assert_ne!(reasons[0], reasons[1]);
    assert!(
        reasons
            .iter()
            .any(|reason| reason.contains("table-cell-r0c0"))
    );
    assert!(
        reasons
            .iter()
            .any(|reason| reason.contains("table-cell-r0c1"))
    );
}
