mod fixtures;
#[path = "action_semantics_test/security.rs"]
mod security;

use pptx2html_core::model::{ActionTarget, ActionTrigger};
use pptx2html_core::{convert_bytes_with_metadata, parser::PptxParser};

use fixtures::MinimalPptx;

const SHAPES: &str = r#"
<p:sp><p:nvSpPr><p:cNvPr id="2" name="safe shape"><a:hlinkClick r:id="rIdSafe" anchor="named-anchor" tooltip="Safe &amp; sound"/><a:hlinkMouseOver action="ppaction://hlinkshowjump?jump=lastslide"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="500000"/></a:xfrm></p:spPr></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="3" name="unsafe shape"><a:hlinkClick r:id="rIdUnsafe"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:p><a:r><a:rPr><a:hlinkClick action="ppaction://hlinkshowjump?jump=nextslide"/></a:rPr><a:t>RUN_NAV</a:t></a:r><a:r><a:rPr><a:hlinkClick action="ppaction://program"/></a:rPr><a:t>RUN_CUSTOM</a:t></a:r></a:p></p:txBody></p:sp>
<p:pic><p:nvPicPr><p:cNvPr id="4" name="media"><a:hlinkClick r:id="" action="ppaction://media"/></p:cNvPr><p:cNvPicPr/><p:nvPr/></p:nvPicPr><p:blipFill/><p:spPr/></p:pic>
<p:sp><p:nvSpPr><p:cNvPr id="5" name="spoof"><x:hlinkClick xmlns:x="urn:evil" action="ppaction://program"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:hlinkClick action="ppaction://program"/></p:spPr></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="6" name="file"><a:hlinkClick r:id="rIdSafe" action="ppaction://hlinkfile"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp>
<p:sp><p:nvSpPr><p:cNvPr id="7" name="alias"><a:hlinkClick xmlns:rel="http://schemas.openxmlformats.org/officeDocument/2006/relationships" rel:id="rIdSafe"/></p:cNvPr><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/></p:sp>"#;

const RELS: &str = r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rIdSafe" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.test/?a=1&amp;b=2" TargetMode="External"/>
<Relationship Id="rIdUnsafe" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="javascript:secret0" TargetMode="External"/>
</Relationships>"#;

#[test]
fn typed_actions_are_preserved_by_trigger_when_package_is_parsed() {
    let pptx = MinimalPptx::new(SHAPES).with_slide_rels(RELS).build();

    let presentation = PptxParser::parse_bytes(&pptx).expect("fixture parses");

    let safe = &presentation.slides[0].shapes[0].actions;
    assert_eq!(
        safe.click.as_ref().map(|action| action.trigger),
        Some(ActionTrigger::Click)
    );
    assert!(
        matches!(safe.click.as_ref().map(|action| &action.target), Some(ActionTarget::ExternalUri(uri)) if uri == "https://example.test/?a=1&b=2")
    );
    assert_eq!(
        safe.click
            .as_ref()
            .and_then(|action| action.anchor.as_deref()),
        Some("named-anchor")
    );
    assert_eq!(
        safe.click
            .as_ref()
            .and_then(|action| action.tooltip.as_deref()),
        Some("Safe & sound")
    );
    assert!(matches!(
        safe.hover.as_ref().map(|action| &action.target),
        Some(ActionTarget::Last)
    ));
    let runs = &presentation.slides[0].shapes[1]
        .text_body
        .as_ref()
        .expect("text body")
        .paragraphs[0]
        .runs;
    assert!(matches!(
        runs[0].actions.click.as_ref().map(|action| &action.target),
        Some(ActionTarget::Next)
    ));
    assert!(
        matches!(runs[1].actions.click.as_ref().map(|action| &action.target), Some(ActionTarget::Unsupported(raw)) if raw == "ppaction://program")
    );
    assert!(matches!(
        presentation.slides[0].shapes[2]
            .actions
            .click
            .as_ref()
            .map(|action| &action.target),
        Some(ActionTarget::MediaPlay)
    ));
    assert!(presentation.slides[0].shapes[3].actions.click.is_none());
    let file = presentation.slides[0].shapes[4]
        .actions
        .click
        .as_ref()
        .expect("file action");
    assert!(
        matches!(file.target, ActionTarget::Unsupported(ref raw) if raw == "ppaction://hlinkfile")
    );
    assert_eq!(
        (
            file.relationship_type.as_deref(),
            file.relationship_mode.as_deref()
        ),
        (
            Some("http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink"),
            Some("External")
        )
    );
    assert!(matches!(
        presentation.slides[0].shapes[5]
            .actions
            .click
            .as_ref()
            .map(|action| &action.target),
        Some(ActionTarget::ExternalUri(_))
    ));
}

#[test]
fn rendered_actions_execute_only_allowlisted_contracts() {
    let pptx = MinimalPptx::new(SHAPES).with_slide_rels(RELS).build();

    let result = convert_bytes_with_metadata(&pptx).expect("fixture converts");

    assert!(result.html.contains("id=\"slide-1\""));
    assert!(
        result
            .html
            .contains("href=\"https://example.test/?a=1&amp;b=2\"")
    );
    assert!(result.html.contains("rel=\"noopener noreferrer\""));
    assert!(result.html.contains("data-action=\"next\""));
    assert!(result.html.contains("data-hover-action=\"last\""));
    assert!(result.html.contains("RUN_NAV"));
    assert!(result.html.contains("RUN_CUSTOM"));
    assert!(!result.html.contains("href=\"javascript:"));
    assert!(!result.html.contains("secret0"));
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|item| item.code == "ACTION_UNSUPPORTED")
            .count(),
        2
    );
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|item| item.code == "ACTION_UNSAFE_URI")
            .count(),
        1
    );
    assert!(
        result
            .diagnostics
            .iter()
            .filter(|item| item.code == "OOXML_ELEMENT_UNSUPPORTED")
            .all(|item| {
                !item
                    .location
                    .qualified_element_name
                    .as_deref()
                    .is_some_and(|name| matches!(name, "a:hlinkClick" | "a:hlinkMouseOver"))
            })
    );
}

#[test]
fn no_action_document_does_not_receive_action_runtime_or_css() {
    let pptx = MinimalPptx::new("").build();

    let result = convert_bytes_with_metadata(&pptx).expect("fixture converts");

    assert!(!result.html.contains("shape-action-surface"));
    assert!(
        !result
            .html
            .contains("document.addEventListener('click',go)")
    );
}
