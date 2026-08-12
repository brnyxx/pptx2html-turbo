from __future__ import annotations

if __package__:
    from .completion_deck_common import theme_xml
    from .completion_deck_package import (
        NS,
        REL,
        ContentType,
        Part,
        Relationship,
        relationships_xml,
    )
else:
    from completion_deck_common import theme_xml
    from completion_deck_package import (
        NS,
        REL,
        ContentType,
        Part,
        Relationship,
        relationships_xml,
    )


MODERN_COMMENTS = "http://schemas.microsoft.com/office/2018/10/relationships/comments"
MODERN_AUTHORS = "http://schemas.microsoft.com/office/2018/10/relationships/authors"


def presentation_relationships() -> tuple[Relationship, ...]:
    return (
        ("rIdClassicAuthors", REL + "commentAuthors", "commentAuthors.xml", None),
        ("rIdModernAuthors", MODERN_AUTHORS, "authors/author1.xml", None),
        ("rIdNotesMaster", REL + "notesMaster", "notesMasters/notesMaster1.xml", None),
    )


def slide_relationships() -> tuple[Relationship, ...]:
    return (
        ("rIdNotes", REL + "notesSlide", "../notesSlides/notesSlide1.xml", None),
        ("rIdComments", REL + "comments", "../comments/comment1.xml", None),
        ("rIdModernComments", MODERN_COMMENTS, "../comments/modernComment1.xml", None),
    )


def parts() -> tuple[Part, ...]:
    return (
        ("ppt/notesSlides/notesSlide1.xml", _notes_slide()),
        (
            "ppt/notesSlides/_rels/notesSlide1.xml.rels",
            relationships_xml(
                (
                    ("rIdSlide", REL + "slide", "../slides/slide1.xml", None),
                    (
                        "rIdNotesMaster",
                        REL + "notesMaster",
                        "../notesMasters/notesMaster1.xml",
                        None,
                    ),
                )
            ),
        ),
        ("ppt/notesMasters/notesMaster1.xml", _notes_master()),
        (
            "ppt/notesMasters/_rels/notesMaster1.xml.rels",
            relationships_xml(
                (("rIdTheme", REL + "theme", "../theme/notesTheme1.xml", None),)
            ),
        ),
        ("ppt/theme/notesTheme1.xml", theme_xml("Notes Completion")),
        ("ppt/comments/comment1.xml", _classic_comments()),
        ("ppt/commentAuthors.xml", _classic_authors()),
        ("ppt/comments/modernComment1.xml", _modern_comments()),
        ("ppt/authors/author1.xml", _modern_authors()),
    )


def content_types() -> tuple[ContentType, ...]:
    presentation = "application/vnd.openxmlformats-officedocument.presentationml."
    return (
        ("/ppt/notesSlides/notesSlide1.xml", presentation + "notesSlide+xml"),
        ("/ppt/notesMasters/notesMaster1.xml", presentation + "notesMaster+xml"),
        (
            "/ppt/theme/notesTheme1.xml",
            "application/vnd.openxmlformats-officedocument.theme+xml",
        ),
        ("/ppt/comments/comment1.xml", presentation + "comments+xml"),
        ("/ppt/commentAuthors.xml", presentation + "commentAuthors+xml"),
        (
            "/ppt/comments/modernComment1.xml",
            "application/vnd.ms-powerpoint.comments+xml",
        ),
        ("/ppt/authors/author1.xml", "application/vnd.ms-powerpoint.authors+xml"),
    )


def _notes_slide() -> bytes:
    return f'<?xml version="1.0"?><p:notes {NS}><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="notes"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>NOTES_SENTINEL</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld><p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr></p:notes>'.encode()


def _notes_master() -> bytes:
    return f'<?xml version="1.0"?><p:notesMaster {NS}><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><p:clrMap accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" bg1="lt1" bg2="lt2" folHlink="folHlink" hlink="hlink" tx1="dk1" tx2="dk2"/><p:hf dt="0" hdr="0" ftr="0" sldNum="0"/><p:notesStyle/></p:notesMaster>'.encode()


def _classic_comments() -> bytes:
    return f'<?xml version="1.0"?><p:cmLst {NS}><p:cm authorId="0" dt="2026-01-01T00:00:00Z" idx="1"><p:pos x="0" y="0"/><p:text>LEGACY_COMMENT</p:text></p:cm><p:cm authorId="404" dt="2026-01-01T00:00:00Z" idx="2"><p:pos x="1" y="1"/><p:text>MISSING_AUTHOR_COMMENT</p:text></p:cm></p:cmLst>'.encode()


def _classic_authors() -> bytes:
    return f'<?xml version="1.0"?><p:cmAuthorLst {NS}><p:cmAuthor id="0" name="Fixture" initials="F" lastIdx="2" clrIdx="0"/></p:cmAuthorLst>'.encode()


def _modern_comments() -> bytes:
    return f'<?xml version="1.0"?><p188:cmLst {NS}><p188:cm id="{{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}}" authorId="{{AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA}}" created="2026-01-01T00:00:00Z"><p188:unknownAnchor/><p188:txBody><a:bodyPr/><a:lstStyle/><a:p><a:r><a:t>MODERN_COMMENT</a:t></a:r></a:p></p188:txBody><p188:extLst><p:ext uri="fixture-modern-extension"><future:payload xmlns:future="urn:pptx2html:fixture:future">MODERN_EXTENSION_SENTINEL&lt;/script&gt;</future:payload></p:ext></p188:extLst></p188:cm></p188:cmLst>'.encode()


def _modern_authors() -> bytes:
    return f'<?xml version="1.0"?><p188:authorLst {NS}><p188:author id="{{AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA}}" name="Fixture" initials="F" userId="fixture@example.invalid" providerId=""/></p188:authorLst>'.encode()
