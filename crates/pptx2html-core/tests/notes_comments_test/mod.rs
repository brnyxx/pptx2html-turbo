use super::fixtures::MinimalPptx;

const REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/";
const MODERN_REL: &str = "http://schemas.microsoft.com/office/2018/10/relationships/";
const PML: &str = "http://schemas.openxmlformats.org/presentationml/2006/main";
const DML: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const P188: &str = "http://schemas.microsoft.com/office/powerpoint/2018/8/main";

pub fn complete_package() -> Vec<u8> {
    base(classic_comments("0", "LEGACY_COMMENT"), false)
}

pub fn missing_author_package() -> Vec<u8> {
    base(classic_comments("404", "MISSING_AUTHOR_COMMENT"), false)
}

pub fn duplicate_relationship_package() -> Vec<u8> {
    let presentation_rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId2" Type="{REL}slide" Target="slides/slide1.xml"/>
<Relationship Id="rIdClassicAuthors" Type="{REL}commentAuthors" Target="commentAuthors.xml"/>
</Relationships>"#,
    );
    let slide_rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rIdDuplicate" Type="{REL}image" Target="../media/ignored.png"/>
<Relationship Id="rIdDuplicate" Type="{REL}comments" Target="../comments/comment1.xml"/>
<Relationship Id="rIdDuplicate" Type="{REL}comments" Target="../comments/comment2.xml"/>
</Relationships>"#,
    );
    MinimalPptx::new(&visible_shape())
        .with_presentation_rels(&presentation_rels)
        .with_slide_rels(&slide_rels)
        .with_extra_file(
            "ppt/comments/comment1.xml",
            classic_comments("0", "DUPLICATE_COMMENT_1").as_bytes(),
        )
        .with_extra_file(
            "ppt/comments/comment2.xml",
            classic_comments("0", "DUPLICATE_COMMENT_2").as_bytes(),
        )
        .with_extra_file("ppt/commentAuthors.xml", classic_authors().as_bytes())
        .build()
}

pub fn duplicate_authors_package() -> Vec<u8> {
    let presentation_rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId2" Type="{REL}slide" Target="slides/slide1.xml"/>
<Relationship Id="rIdClassicAuthors" Type="{REL}commentAuthors" Target="commentAuthors.xml"/>
</Relationships>"#,
    );
    let slide_rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rIdClassic" Type="{REL}comments" Target="../comments/comment1.xml"/>
</Relationships>"#,
    );
    let authors = format!(
        r#"<p:cmAuthorLst xmlns:p="{PML}">
<p:cmAuthor id="0" name="Duplicate Author One" initials="D1"/>
<p:cmAuthor id="0" name="Duplicate Author Two" initials="D2"/>
<p:cmAuthor id="9" name="Orphan Author" initials="OA"/>
</p:cmAuthorLst>"#,
    );
    MinimalPptx::new(&visible_shape())
        .with_presentation_rels(&presentation_rels)
        .with_slide_rels(&slide_rels)
        .with_extra_file(
            "ppt/comments/comment1.xml",
            classic_comments("0", "DUPLICATE_AUTHOR_COMMENT").as_bytes(),
        )
        .with_extra_file("ppt/commentAuthors.xml", authors.as_bytes())
        .build()
}

pub fn multiple_modern_extensions_package() -> Vec<u8> {
    base_with_modern(
        classic_comments("0", "LEGACY_COMMENT"),
        modern_comments_with_independent_extensions(),
        false,
    )
}

pub fn rich_annotation_text_package() -> Vec<u8> {
    base_with_parts(
        format!(
            r#"<p:cmLst xmlns:p="{PML}"><p:cm authorId="0" idx="1"><p:text>LEGACY_A&amp;B<![CDATA[<legacy>]]></p:text></p:cm></p:cmLst>"#,
        ),
        format!(
            r#"<p188:cmLst xmlns:p188="{P188}" xmlns:a="{DML}">
<p188:cm id="{{33333333-3333-3333-3333-333333333333}}" authorId="{{AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA}}">
<p188:txBody>
<a:p><a:r><a:t>COMMENT_A&amp;B</a:t></a:r><a:br/><a:fld id="{{44444444-4444-4444-4444-444444444444}}" type="datetime"><a:t>COMMENT_FIELD</a:t></a:fld></a:p>
<a:p><a:r><a:t><![CDATA[COMMENT_CDATA<ok>]]></a:t></a:r></a:p>
</p188:txBody>
</p188:cm>
</p188:cmLst>"#,
        ),
        format!(
            r#"<p:notes xmlns:p="{PML}" xmlns:a="{DML}"><p:cSld><p:spTree><p:sp><p:txBody>
<a:p><a:r><a:t>NOTE_A&amp;B</a:t></a:r><a:br/><a:fld id="{{55555555-5555-5555-5555-555555555555}}" type="datetime"><a:t>NOTE_FIELD</a:t></a:fld></a:p>
<a:p><a:r><a:t><![CDATA[NOTE_CDATA<ok>]]></a:t></a:r></a:p>
</p:txBody></p:sp></p:spTree></p:cSld></p:notes>"#,
        ),
        false,
    )
}

pub fn missing_required_comment_attributes_package() -> Vec<u8> {
    base_with_parts(
        format!(
            r#"<p:cmLst xmlns:p="{PML}"><p:cm authorId="0"><p:text>INVALID_LEGACY_COMMENT</p:text></p:cm></p:cmLst>"#,
        ),
        format!(
            r#"<p188:cmLst xmlns:p188="{P188}" xmlns:a="{DML}">
<p188:cm id="{{66666666-6666-6666-6666-666666666666}}">
<p188:txBody><a:p><a:r><a:t>INVALID_MODERN_COMMENT</a:t></a:r></a:p></p188:txBody>
</p188:cm>
</p188:cmLst>"#,
        ),
        notes_slide(),
        false,
    )
}

pub fn missing_notes_master_relationship_part_package() -> Vec<u8> {
    notes_master_contract_package(None, None)
}

pub fn missing_notes_master_relationship_package() -> Vec<u8> {
    notes_master_contract_package(
        Some(&format!(
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rIdImage" Type="{REL}image" Target="../media/ignored.png"/>
</Relationships>"#,
        )),
        None,
    )
}

pub fn missing_notes_master_part_package() -> Vec<u8> {
    notes_master_contract_package(Some(&notes_rels()), None)
}

pub fn invalid_notes_master_package() -> Vec<u8> {
    notes_master_contract_package(
        Some(&notes_rels()),
        Some(&format!(r#"<p:notes xmlns:p="{PML}"><p:cSld/></p:notes>"#)),
    )
}

pub fn selected_slides_package() -> Vec<u8> {
    let presentation = format!(
        r#"<p:presentation xmlns:r="{REL}" xmlns:p="{PML}">
<p:sldIdLst>
<p:sldId id="256" r:id="rIdSlide1"/>
<p:sldId id="257" r:id="rIdSlide2" show="0"/>
</p:sldIdLst>
<p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    );
    let presentation_rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rIdSlide1" Type="{REL}slide" Target="slides/slide1.xml"/>
<Relationship Id="rIdSlide2" Type="{REL}slide" Target="slides/slide2.xml"/>
<Relationship Id="rIdClassicAuthors" Type="{REL}commentAuthors" Target="commentAuthors.xml"/>
</Relationships>"#,
    );
    MinimalPptx::new(&visible_shape())
        .with_presentation_xml(&presentation)
        .with_presentation_rels(&presentation_rels)
        .with_slide_rels(&slide_annotation_rels(1))
        .with_extra_file(
            "ppt/slides/slide2.xml",
            slide_document("VISIBLE_BODY_2", true).as_bytes(),
        )
        .with_extra_file(
            "ppt/slides/_rels/slide2.xml.rels",
            slide_annotation_rels(2).as_bytes(),
        )
        .with_extra_file(
            "ppt/notesSlides/notesSlide1.xml",
            notes_slide_with_text("NOTES_SLIDE_1").as_bytes(),
        )
        .with_extra_file(
            "ppt/notesSlides/notesSlide2.xml",
            notes_slide_with_text("NOTES_SLIDE_2").as_bytes(),
        )
        .with_extra_file(
            "ppt/notesSlides/_rels/notesSlide1.xml.rels",
            notes_rels().as_bytes(),
        )
        .with_extra_file(
            "ppt/notesSlides/_rels/notesSlide2.xml.rels",
            notes_rels().as_bytes(),
        )
        .with_extra_file(
            "ppt/notesMasters/notesMaster1.xml",
            format!(r#"<p:notesMaster xmlns:p="{PML}"><p:cSld/></p:notesMaster>"#).as_bytes(),
        )
        .with_extra_file(
            "ppt/comments/comment1.xml",
            classic_comments("0", "COMMENT_SLIDE_1").as_bytes(),
        )
        .with_extra_file(
            "ppt/comments/comment2.xml",
            classic_comments("0", "COMMENT_SLIDE_2").as_bytes(),
        )
        .with_extra_file("ppt/commentAuthors.xml", classic_authors().as_bytes())
        .build()
}

pub fn spoof_package() -> Vec<u8> {
    let slide_rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rIdNotes" Type="{REL}notesSlide" Target="https://example.invalid/notes.xml" TargetMode="External"/>
<Relationship Id="rIdComments" Type="{REL}comments" Target="../comments/comment1.xml" TargetMode="External"/>
<Relationship Id="rIdSpoof" Type="{REL}comments" Target="../comments/spoof.xml"/>
</Relationships>"#,
    );
    MinimalPptx::new(&visible_shape())
        .with_slide_rels(&slide_rels)
        .with_extra_file(
            "ppt/comments/comment1.xml",
            br#"<x:cmLst xmlns:x="urn:spoof"><x:cm authorId="0"><x:text>SPOOF_SECRET</x:text></x:cm></x:cmLst>"#,
        )
        .with_extra_file(
            "ppt/comments/spoof.xml",
            br#"<x:cmLst xmlns:x="urn:spoof"><x:cm authorId="0"><x:text>SPOOF_SECRET</x:text></x:cm></x:cmLst>"#,
        )
        .build()
}

pub fn spoof_relationship_package() -> Vec<u8> {
    let slide_rels = format!(
        r#"<x:Relationships xmlns:x="urn:spoof"><x:Relationship Id="rIdComments" Type="{REL}comments" Target="../comments/comment1.xml"/></x:Relationships>"#,
    );
    MinimalPptx::new(&visible_shape())
        .with_slide_rels(&slide_rels)
        .with_extra_file(
            "ppt/comments/comment1.xml",
            classic_comments("0", "SPOOF_RELATIONSHIP_SECRET").as_bytes(),
        )
        .build()
}

fn base(classic: String, unsafe_target: bool) -> Vec<u8> {
    base_with_parts(classic, modern_comments(), notes_slide(), unsafe_target)
}

fn base_with_modern(classic: String, modern: String, unsafe_target: bool) -> Vec<u8> {
    base_with_parts(classic, modern, notes_slide(), unsafe_target)
}

fn base_with_parts(classic: String, modern: String, notes: String, unsafe_target: bool) -> Vec<u8> {
    let presentation_rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId2" Type="{REL}slide" Target="slides/slide1.xml"/>
<Relationship Id="rIdClassicAuthors" Type="{REL}commentAuthors" Target="commentAuthors.xml"/>
<Relationship Id="rIdModernAuthors" Type="{MODERN_REL}authors" Target="authors/author1.xml"/>
</Relationships>"#,
    );
    let notes_target = if unsafe_target {
        "https://example.invalid/notes.xml"
    } else {
        "../notesSlides/notesSlide1.xml"
    };
    let slide_rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rIdNotes" Type="{REL}notesSlide" Target="{notes_target}"/>
<Relationship Id="rIdClassic" Type="{REL}comments" Target="../comments/comment1.xml"/>
<Relationship Id="rIdModern" Type="{MODERN_REL}comments" Target="../comments/modernComment1.xml"/>
</Relationships>"#,
    );
    MinimalPptx::new(&visible_shape())
        .with_presentation_rels(&presentation_rels)
        .with_slide_rels(&slide_rels)
        .with_extra_file("ppt/notesSlides/notesSlide1.xml", notes.as_bytes())
        .with_extra_file(
            "ppt/notesSlides/_rels/notesSlide1.xml.rels",
            notes_rels().as_bytes(),
        )
        .with_extra_file(
            "ppt/notesMasters/notesMaster1.xml",
            format!(r#"<p:notesMaster xmlns:p="{PML}"><p:cSld/></p:notesMaster>"#).as_bytes(),
        )
        .with_extra_file("ppt/comments/comment1.xml", classic.as_bytes())
        .with_extra_file("ppt/commentAuthors.xml", classic_authors().as_bytes())
        .with_extra_file("ppt/comments/modernComment1.xml", modern.as_bytes())
        .with_extra_file("ppt/authors/author1.xml", modern_authors().as_bytes())
        .build()
}

fn notes_slide() -> String {
    notes_slide_with_text("NOTES_SENTINEL")
}

fn visible_shape() -> String {
    r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="visible"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:p><a:r><a:t>VISIBLE_BODY</a:t></a:r></a:p></p:txBody></p:sp>"#.to_owned()
}

fn slide_document(text: &str, hidden: bool) -> String {
    let show = if hidden { r#" show="0""# } else { "" };
    format!(
        r#"<p:sld xmlns:a="{DML}" xmlns:r="{REL}" xmlns:p="{PML}"{show}>
<p:cSld><p:spTree>
<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
<p:grpSpPr/>
<p:sp><p:nvSpPr><p:cNvPr id="2" name="visible"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:p><a:r><a:t>{text}</a:t></a:r></a:p></p:txBody></p:sp>
</p:spTree></p:cSld>
</p:sld>"#,
    )
}

fn slide_annotation_rels(index: usize) -> String {
    format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rIdNotes" Type="{REL}notesSlide" Target="../notesSlides/notesSlide{index}.xml"/>
<Relationship Id="rIdClassic" Type="{REL}comments" Target="../comments/comment{index}.xml"/>
</Relationships>"#,
    )
}

fn notes_slide_with_text(text: &str) -> String {
    format!(
        r#"<p:notes xmlns:p="{PML}" xmlns:a="{DML}"><p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>{text}</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>"#,
    )
}

fn notes_rels() -> String {
    format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdMaster" Type="{REL}notesMaster" Target="../notesMasters/notesMaster1.xml"/></Relationships>"#,
    )
}

fn notes_master_contract_package(
    notes_relationships: Option<&str>,
    notes_master: Option<&str>,
) -> Vec<u8> {
    let presentation_rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId2" Type="{REL}slide" Target="slides/slide1.xml"/>
</Relationships>"#,
    );
    let slide_rels = format!(
        r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rIdNotes" Type="{REL}notesSlide" Target="../notesSlides/notesSlide1.xml"/>
</Relationships>"#,
    );
    let mut package = MinimalPptx::new(&visible_shape())
        .with_presentation_rels(&presentation_rels)
        .with_slide_rels(&slide_rels)
        .with_extra_file("ppt/notesSlides/notesSlide1.xml", notes_slide().as_bytes());
    if let Some(relationships) = notes_relationships {
        package = package.with_extra_file(
            "ppt/notesSlides/_rels/notesSlide1.xml.rels",
            relationships.as_bytes(),
        );
    }
    if let Some(master) = notes_master {
        package = package.with_extra_file("ppt/notesMasters/notesMaster1.xml", master.as_bytes());
    }
    package.build()
}

fn classic_comments(author_id: &str, text: &str) -> String {
    format!(
        r#"<p:cmLst xmlns:p="{PML}"><p:cm authorId="{author_id}" dt="2026-01-01T00:00:00Z" idx="1"><p:pos x="0" y="0"/><p:text>{text}</p:text></p:cm></p:cmLst>"#,
    )
}

fn classic_authors() -> String {
    format!(
        r#"<p:cmAuthorLst xmlns:p="{PML}"><p:cmAuthor id="0" name="Classic Author" initials="CA" lastIdx="1" clrIdx="0"/></p:cmAuthorLst>"#,
    )
}

fn modern_comments() -> String {
    format!(
        r#"<p188:cmLst xmlns:p188="{P188}" xmlns:a="{DML}" xmlns:p="{PML}" xmlns:future="urn:future"><p188:cm id="{{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}}" authorId="{{AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA}}" created="2026-01-01T00:00:00Z"><p188:unknownAnchor/><p188:txBody><a:p><a:r><a:t>MODERN_COMMENT</a:t></a:r></a:p></p188:txBody><p188:extLst><p:ext uri="future"><future:payload secret="SECRET_SENTINEL">&lt;script&gt;alert(1)&lt;/script&gt;</future:payload></p:ext></p188:extLst></p188:cm></p188:cmLst>"#,
    )
}

fn modern_comments_with_independent_extensions() -> String {
    format!(
        r#"<p188:cmLst xmlns:p188="{P188}" xmlns:a="{DML}" xmlns:p="{PML}" xmlns:future="urn:future">
<p188:cm id="{{11111111-1111-1111-1111-111111111111}}" authorId="{{AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA}}" created="2026-01-01T00:00:00Z">
<p188:txBody><a:p><a:r><a:t>MODERN_COMMENT_ONE</a:t></a:r></a:p></p188:txBody>
<p188:extLst><p:ext uri="one"><future:payload>EXTENSION_ONE</future:payload></p:ext></p188:extLst>
</p188:cm>
<p188:cm id="{{22222222-2222-2222-2222-222222222222}}" authorId="{{AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA}}" created="2026-01-01T00:00:01Z">
<p188:txBody><a:p><a:r><a:t>MODERN_COMMENT_TWO</a:t></a:r></a:p></p188:txBody>
<p188:extLst><p:ext uri="two"><future:payload>EXTENSION_TWO</future:payload></p:ext></p188:extLst>
</p188:cm>
</p188:cmLst>"#,
    )
}

fn modern_authors() -> String {
    format!(
        r#"<p188:authorLst xmlns:p188="{P188}"><p188:author id="{{AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA}}" name="Modern Author" initials="MA" userId="modern@example.invalid" providerId=""/></p188:authorLst>"#,
    )
}
