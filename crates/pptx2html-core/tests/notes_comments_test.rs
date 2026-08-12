mod fixtures;
#[path = "notes_comments_test/mod.rs"]
mod support;

use fixtures::MinimalPptx;
use pptx2html_core::model::{ClrMap, Presentation, Size};
use pptx2html_core::{
    ConversionOptions, convert_bytes_with_metadata, convert_bytes_with_options_metadata,
};
use support::{
    complete_package, duplicate_authors_package, duplicate_relationship_package,
    empty_second_root_annotation_package, empty_single_root_annotation_package,
    invalid_notes_master_package, missing_author_package, missing_notes_master_part_package,
    missing_notes_master_relationship_package, missing_notes_master_relationship_part_package,
    missing_required_comment_attributes_package, modern_replies_package,
    multiple_modern_extensions_package, multiple_root_annotation_package,
    multiple_root_authors_package, multiple_root_notes_master_package,
    rich_annotation_text_package, selected_slides_package, spoof_package,
    spoof_relationship_package,
};

fn raw_for<'a>(result: &'a pptx2html_core::ConversionResult, code: &str) -> Vec<&'a str> {
    result
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == code)
        .filter_map(|diagnostic| diagnostic.raw_reference.as_deref())
        .collect()
}

fn code_count(result: &pptx2html_core::ConversionResult, code: &str) -> usize {
    result
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == code)
        .count()
}

fn raw_contains(result: &pptx2html_core::ConversionResult, sentinel: &str) -> bool {
    result
        .diagnostics()
        .iter()
        .filter_map(|diagnostic| diagnostic.raw_reference.as_deref())
        .any(|raw| raw.contains(sentinel))
}

#[test]
#[ignore = "manual QA fixture export"]
fn export_complete_package_for_manual_qa() {
    let path = std::env::var_os("PPTX2HTML_TASK16_QA_PATH").expect("QA output path");
    std::fs::write(path, complete_package()).expect("write QA fixture");
}

fn tiers_for<'a>(result: &'a pptx2html_core::ConversionResult, code: &str) -> Vec<&'a str> {
    result
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.code == code)
        .map(|diagnostic| diagnostic.support_tier.as_str())
        .collect()
}

#[test]
fn preserves_notes_master_and_classic_and_modern_comments_off_canvas() {
    let result = convert_bytes_with_metadata(&complete_package()).expect("fixture converts");

    assert!(result.html.contains("VISIBLE_BODY"));
    let slide_end = result
        .html
        .find("</div>\n</div>\n</div>\n<script")
        .expect("visible slide subtree ends before metadata");
    let visible = &result.html[..slide_end];
    for sentinel in ["NOTES_SENTINEL", "LEGACY_COMMENT", "MODERN_COMMENT"] {
        assert!(
            !visible.contains(sentinel),
            "{sentinel} leaked onto the canvas"
        );
        assert!(
            result.html.contains(sentinel),
            "{sentinel} missing from metadata"
        );
    }

    let notes = raw_for(&result, "NOTES_SLIDE_METADATA");
    assert_eq!(notes.len(), 1);
    assert!(notes[0].contains("slide_number=1\n"));
    assert!(notes[0].contains("part=ppt/notesSlides/notesSlide1.xml\n"));
    assert!(notes[0].contains("notes_master=ppt/notesMasters/notesMaster1.xml\n"));
    assert!(notes[0].ends_with("text=NOTES_SENTINEL"));

    let classic = raw_for(&result, "LEGACY_COMMENT_METADATA");
    assert_eq!(classic.len(), 1);
    assert!(classic[0].contains("author=Classic Author\n"));
    assert!(classic[0].contains("created=2026-01-01T00:00:00Z\n"));
    assert!(classic[0].ends_with("text=LEGACY_COMMENT"));

    let modern = raw_for(&result, "MODERN_COMMENT_METADATA");
    assert_eq!(modern.len(), 1);
    assert!(modern[0].contains("author=Modern Author\n"));
    assert!(modern[0].contains("id={5C22544A-7EE6-4342-B048-85BDC9FD1C3A}\n"));
    assert!(modern[0].ends_with("text=MODERN_COMMENT"));
    assert_eq!(tiers_for(&result, "NOTES_SLIDE_METADATA"), vec!["fallback"]);
    assert_eq!(
        tiers_for(&result, "LEGACY_COMMENT_METADATA"),
        vec!["fallback"],
    );
    assert_eq!(
        tiers_for(&result, "MODERN_COMMENT_METADATA"),
        vec!["fallback"],
    );
}

#[test]
fn missing_author_preserves_comment_and_emits_stable_code() {
    let result = convert_bytes_with_metadata(&missing_author_package()).expect("fixture converts");
    let unresolved = raw_for(&result, "COMMENT_AUTHOR_UNRESOLVED");

    assert_eq!(unresolved.len(), 1);
    assert!(unresolved[0].contains("author_id=404\n"));
    assert!(unresolved[0].ends_with("text=MISSING_AUTHOR_COMMENT"));
    assert!(result.html.contains("MISSING_AUTHOR_COMMENT"));
}

#[test]
fn modern_extension_is_raw_fallback_and_output_is_deterministic_and_safe() {
    let bytes = complete_package();
    let first = convert_bytes_with_metadata(&bytes).expect("first conversion");
    let second = convert_bytes_with_metadata(&bytes).expect("second conversion");

    assert_eq!(first.html, second.html);
    let raw = raw_for(&first, "MODERN_COMMENT_EXTENSION_FALLBACK");
    assert_eq!(raw.len(), 1);
    assert!(raw[0].contains("<future:payload secret=\"SECRET_SENTINEL\""));
    assert!(raw[0].contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(!first.html.contains("<script>alert(1)</script>"));
    assert!(!first.html.contains("NaN"));
    assert!(!first.html.contains("Infinity"));
}

#[test]
fn spoofed_namespace_and_external_targets_do_not_expose_unrelated_parts() {
    let result = convert_bytes_with_metadata(&spoof_package()).expect("fixture converts");

    assert!(code_count(&result, "ANNOTATION_RELATIONSHIP_UNSAFE") >= 2);
    assert!(code_count(&result, "ANNOTATION_ELEMENT_NAMESPACE_INVALID") >= 1);
    assert!(!result.html.contains("SPOOF_SECRET"));
}

#[test]
fn spoofed_relationship_namespace_cannot_select_comment_part() {
    let result =
        convert_bytes_with_metadata(&spoof_relationship_package()).expect("fixture converts");

    assert_eq!(
        code_count(&result, "ANNOTATION_ELEMENT_NAMESPACE_INVALID"),
        1
    );
    assert!(!result.html.contains("SPOOF_RELATIONSHIP_SECRET"));
}

#[test]
fn duplicate_relationship_ids_reject_every_annotation_candidate() {
    let result = convert_bytes_with_metadata(&duplicate_relationship_package())
        .expect("duplicate relationship fixture converts");

    assert_eq!(code_count(&result, "ANNOTATION_RELATIONSHIP_DUPLICATE"), 1);
    assert!(!raw_contains(&result, "DUPLICATE_COMMENT_1"));
    assert!(!raw_contains(&result, "DUPLICATE_COMMENT_2"));
}

#[test]
fn duplicate_and_orphan_authors_are_preserved_without_ambiguous_resolution() {
    let result = convert_bytes_with_metadata(&duplicate_authors_package())
        .expect("duplicate author fixture converts");

    assert_eq!(code_count(&result, "COMMENT_AUTHOR_METADATA"), 3);
    assert_eq!(code_count(&result, "COMMENT_AUTHOR_DUPLICATE"), 1);
    assert_eq!(code_count(&result, "COMMENT_AUTHOR_UNRESOLVED"), 1);
    assert!(raw_contains(&result, "Duplicate Author One"));
    assert!(raw_contains(&result, "Duplicate Author Two"));
    assert!(raw_contains(&result, "Orphan Author"));
    assert!(raw_contains(&result, "DUPLICATE_AUTHOR_COMMENT"));
}

#[test]
fn modern_extensions_are_isolated_to_their_own_comments() {
    let result = convert_bytes_with_metadata(&multiple_modern_extensions_package())
        .expect("multiple modern comments convert");
    let extensions = raw_for(&result, "MODERN_COMMENT_EXTENSION_FALLBACK");

    assert_eq!(extensions.len(), 2);
    assert!(
        extensions
            .iter()
            .all(|raw| { raw.starts_with("<p188:extLst") && !raw.contains("<p188:cmLst") })
    );
    assert!(
        extensions
            .iter()
            .any(|raw| { raw.contains("EXTENSION_ONE") && !raw.contains("EXTENSION_TWO") })
    );
    assert!(
        extensions
            .iter()
            .any(|raw| { raw.contains("EXTENSION_TWO") && !raw.contains("EXTENSION_ONE") })
    );
}

#[test]
fn modern_replies_preserve_independent_identity_author_time_and_text() {
    let result =
        convert_bytes_with_metadata(&modern_replies_package()).expect("modern replies convert");
    let comments = raw_for(&result, "MODERN_COMMENT_METADATA");

    assert_eq!(comments.len(), 2);
    assert!(comments[0].ends_with("text=PARENT_COMMENT"));
    assert!(comments[1].ends_with("text=REPLY_COMMENT"));
    assert!(comments.iter().any(|raw| {
        raw.contains("id={FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF}\n")
            && raw.contains("parent_id=\n")
            && raw.ends_with("text=PARENT_COMMENT")
            && !raw.contains("REPLY_COMMENT")
    }));
    assert!(comments.iter().any(|raw| {
        raw.contains("id={00000000-0000-0000-0000-000000000000}\n")
            && raw.contains("parent_id={FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF}\n")
            && raw.contains("author_id={BBBBBBBB-BBBB-BBBB-BBBB-BBBBBBBBBBBB}\n")
            && raw.contains("created=2026-01-01T00:00:01Z\n")
            && raw.ends_with("text=REPLY_COMMENT")
    }));
    assert_eq!(code_count(&result, "COMMENT_AUTHOR_UNRESOLVED"), 1);
    assert!(raw_for(&result, "COMMENT_AUTHOR_UNRESOLVED")[0].ends_with("text=REPLY_COMMENT"));
}

#[test]
fn author_xml_rejects_multiple_document_roots() {
    let result = convert_bytes_with_metadata(&multiple_root_authors_package())
        .expect("multiple-root author parts degrade");

    assert_eq!(code_count(&result, "ANNOTATION_PART_MALFORMED"), 2);
    assert_eq!(code_count(&result, "COMMENT_AUTHOR_METADATA"), 0);
    assert_eq!(code_count(&result, "COMMENT_AUTHOR_UNRESOLVED"), 2);
}

#[test]
fn annotation_xml_rejects_multiple_document_roots() {
    let result = convert_bytes_with_metadata(&multiple_root_annotation_package())
        .expect("multiple-root package degrades");

    assert_eq!(code_count(&result, "ANNOTATION_PART_MALFORMED"), 2);
    assert!(!raw_contains(&result, "FIRST_COMMENT_ROOT"));
    assert!(!raw_contains(&result, "SECOND_COMMENT_ROOT"));
    assert!(!raw_contains(&result, "FIRST_NOTES_ROOT"));
    assert!(!raw_contains(&result, "SECOND_NOTES_ROOT"));

    let notes_master = convert_bytes_with_metadata(&multiple_root_notes_master_package())
        .expect("multiple-root notes master degrades");
    assert_eq!(code_count(&notes_master, "ANNOTATION_PART_MALFORMED"), 1,);
    assert!(raw_for(&notes_master, "NOTES_SLIDE_METADATA")[0].contains("notes_master=\n"),);
}

#[test]
fn annotation_xml_rejects_empty_second_document_roots() {
    let result = convert_bytes_with_metadata(&empty_second_root_annotation_package())
        .expect("empty second roots degrade");

    assert_eq!(code_count(&result, "ANNOTATION_PART_MALFORMED"), 2);
    assert_eq!(code_count(&result, "LEGACY_COMMENT_METADATA"), 0);
    assert_eq!(code_count(&result, "NOTES_SLIDE_METADATA"), 1);
    assert!(!raw_contains(&result, "FIRST_COMMENT_ROOT"));
    assert!(!raw_contains(&result, "FIRST_NOTES_ROOT"));
}

#[test]
fn annotation_xml_accepts_empty_single_document_roots() {
    let result = convert_bytes_with_metadata(&empty_single_root_annotation_package())
        .expect("empty annotation roots convert");

    assert_eq!(code_count(&result, "ANNOTATION_PART_MALFORMED"), 0);
    assert_eq!(code_count(&result, "LEGACY_COMMENT_METADATA"), 0);
    assert_eq!(code_count(&result, "NOTES_SLIDE_METADATA"), 1);
}

#[test]
fn annotation_text_preserves_paragraphs_breaks_fields_entities_and_cdata() {
    let result = convert_bytes_with_metadata(&rich_annotation_text_package())
        .expect("rich annotation text converts");

    let notes = raw_for(&result, "NOTES_SLIDE_METADATA");
    assert_eq!(notes.len(), 1);
    assert!(notes[0].contains("text=NOTE_A&B\nNOTE_FIELD\nNOTE_CDATA<ok>"));

    let legacy = raw_for(&result, "LEGACY_COMMENT_METADATA");
    assert_eq!(legacy.len(), 1);
    assert!(legacy[0].contains("text=LEGACY_A&B<legacy>"));

    let modern = raw_for(&result, "MODERN_COMMENT_METADATA");
    assert_eq!(modern.len(), 1);
    assert!(modern[0].contains("text=COMMENT_A&B\nCOMMENT_FIELD\nCOMMENT_CDATA<ok>",));
}

#[test]
fn comments_missing_required_identity_attributes_are_rejected() {
    let result = convert_bytes_with_metadata(&missing_required_comment_attributes_package())
        .expect("invalid comment identity fixture converts");

    assert_eq!(code_count(&result, "ANNOTATION_PART_MALFORMED"), 2);
    assert_eq!(code_count(&result, "LEGACY_COMMENT_METADATA"), 0);
    assert_eq!(code_count(&result, "MODERN_COMMENT_METADATA"), 0);
    assert!(!raw_contains(&result, "INVALID_LEGACY_COMMENT"));
    assert!(!raw_contains(&result, "INVALID_MODERN_COMMENT"));
}

#[test]
fn notes_master_relationship_and_target_contract_is_validated() {
    let complete =
        convert_bytes_with_metadata(&complete_package()).expect("complete fixture converts");
    let notes = raw_for(&complete, "NOTES_SLIDE_METADATA");
    assert_eq!(notes.len(), 1);
    assert!(notes[0].contains("notes_master_relationship_id=rIdMaster"));

    let missing_relationship_part =
        convert_bytes_with_metadata(&missing_notes_master_relationship_part_package())
            .expect("missing relationships part converts");
    assert_eq!(
        code_count(&missing_relationship_part, "ANNOTATION_PART_MISSING"),
        1,
    );

    let missing_relationship =
        convert_bytes_with_metadata(&missing_notes_master_relationship_package())
            .expect("missing relationship converts");
    assert_eq!(
        code_count(&missing_relationship, "ANNOTATION_PART_MISSING"),
        1,
    );

    let missing_part = convert_bytes_with_metadata(&missing_notes_master_part_package())
        .expect("missing notes master part converts");
    assert_eq!(code_count(&missing_part, "ANNOTATION_PART_MISSING"), 1);
    assert!(raw_for(&missing_part, "NOTES_SLIDE_METADATA")[0].contains("notes_master=\n"));

    let invalid_root = convert_bytes_with_metadata(&invalid_notes_master_package())
        .expect("invalid notes master converts");
    assert_eq!(code_count(&invalid_root, "ANNOTATION_PART_MALFORMED"), 1);
    assert!(raw_for(&invalid_root, "NOTES_SLIDE_METADATA")[0].contains("notes_master=\n"));
}

#[test]
fn plain_fixture_remains_annotation_free() {
    let result = convert_bytes_with_metadata(&MinimalPptx::new(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="visible"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:p><a:r><a:t>VISIBLE_BODY</a:t></a:r></a:p></p:txBody></p:sp>"#,
    ).build())
        .expect("plain fixture converts");
    assert!(result.diagnostics().iter().all(
        |diagnostic| !diagnostic.code.contains("COMMENT") && !diagnostic.code.contains("NOTES")
    ));
}

#[test]
fn annotations_follow_slide_selection_and_hidden_slide_policy() {
    let bytes = selected_slides_package();

    let visible = convert_bytes_with_options_metadata(&bytes, &ConversionOptions::default())
        .expect("visible slide converts");
    assert_eq!(visible.slide_count, 1);
    assert!(raw_contains(&visible, "NOTES_SLIDE_1"));
    assert!(raw_contains(&visible, "COMMENT_SLIDE_1"));
    assert!(!raw_contains(&visible, "NOTES_SLIDE_2"));
    assert!(!raw_contains(&visible, "COMMENT_SLIDE_2"));

    let hidden_excluded = convert_bytes_with_options_metadata(
        &bytes,
        &ConversionOptions {
            slide_indices: Some(vec![2]),
            ..Default::default()
        },
    )
    .expect("hidden slide selection converts");
    assert_eq!(hidden_excluded.slide_count, 0);
    assert!(!raw_contains(&hidden_excluded, "NOTES_SLIDE_1"));
    assert!(!raw_contains(&hidden_excluded, "COMMENT_SLIDE_1"));
    assert!(!raw_contains(&hidden_excluded, "NOTES_SLIDE_2"));
    assert!(!raw_contains(&hidden_excluded, "COMMENT_SLIDE_2"));
    assert!(!raw_contains(&hidden_excluded, "Classic Author"));

    let hidden_included = convert_bytes_with_options_metadata(
        &bytes,
        &ConversionOptions {
            include_hidden: true,
            slide_indices: Some(vec![2]),
            ..Default::default()
        },
    )
    .expect("included hidden slide converts");
    assert_eq!(hidden_included.slide_count, 1);
    assert!(!raw_contains(&hidden_included, "NOTES_SLIDE_1"));
    assert!(!raw_contains(&hidden_included, "COMMENT_SLIDE_1"));
    assert!(raw_contains(&hidden_included, "NOTES_SLIDE_2"));
    assert!(raw_contains(&hidden_included, "COMMENT_SLIDE_2"));
}

#[test]
fn public_presentation_literal_remains_source_compatible() {
    let presentation = Presentation {
        slides: Vec::new(),
        slide_size: Size::default(),
        title: None,
        themes: Vec::new(),
        masters: Vec::new(),
        layouts: Vec::new(),
        default_text_style: None,
        clr_map: ClrMap::default(),
    };

    assert!(presentation.slides.is_empty());
}
