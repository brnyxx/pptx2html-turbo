use std::io::{Cursor, Write};

use zip::ZipWriter;
use zip::write::SimpleFileOptions;

const CUSTOM_STYLE: &str = "{11111111-1111-1111-1111-111111111111}";
const TABLE_STYLES_REL: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/tableStyles";

pub fn relationship_package(style_relationships: &str, style_part: Option<&str>) -> Vec<u8> {
    let rels = format!(
        r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdSlide" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rIdTheme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>{style_relationships}</Relationships>"#
    );
    package(
        &[document(&table(CUSTOM_STYLE, "1", "1", false))],
        &rels,
        style_part,
        theme(),
    )
}

pub fn relationship(id: &str, target: &str, mode: Option<&str>, kind: &str) -> String {
    let mode = mode
        .map(|value| format!(r#" TargetMode="{value}""#))
        .unwrap_or_default();
    format!(r#"<Relationship Id="{id}" Type="{kind}" Target="{target}"{mode}/>"#)
}

pub fn table_styles_relationship(id: &str, target: &str, mode: Option<&str>) -> String {
    relationship(id, target, mode, TABLE_STYLES_REL)
}

pub fn spoof_relationship(id: &str, target: &str) -> String {
    relationship(id, target, None, "urn:spoof/tableStyles")
}

pub fn strict_style_package(style_xml: &str) -> Vec<u8> {
    relationship_package(
        &table_styles_relationship("rIdStyles", "tableStyles.xml", None),
        Some(style_xml),
    )
}

pub fn fill_ref_package() -> Vec<u8> {
    let style_xml = format!(
        r#"<?xml version="1.0"?><a:tblStyleLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:tblStyle styleId="{CUSTOM_STYLE}" styleName="fillRef"><a:wholeTbl><a:tcStyle><a:fillRef idx="2"><a:schemeClr val="accent2"><a:tint val="20000"/></a:schemeClr></a:fillRef></a:tcStyle></a:wholeTbl></a:tblStyle></a:tblStyleLst>"#
    );
    let themed = theme().replace(
        "</a:themeElements>",
        r#"<a:fmtScheme name="Fixture"><a:fillStyleLst><a:solidFill><a:schemeClr val="accent1"/></a:solidFill><a:gradFill><a:gsLst><a:gs pos="0"><a:schemeClr val="phClr"/></a:gs><a:gs pos="100000"><a:schemeClr val="accent3"/></a:gs></a:gsLst><a:lin ang="5400000"/></a:gradFill></a:fillStyleLst><a:lnStyleLst/><a:effectStyleLst/><a:bgFillStyleLst/></a:fmtScheme></a:themeElements>"#,
    );
    let rels = format!(
        r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdSlide" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rIdTheme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>{}</Relationships>"#,
        table_styles_relationship("rIdStyles", "tableStyles.xml", None)
    );
    package(
        &[document(&table(CUSTOM_STYLE, "1", "1", false))],
        &rels,
        Some(&style_xml),
        &themed,
    )
}

pub fn diagnostic_identity_package() -> Vec<u8> {
    let first = document(&format!(
        "{}{}",
        table_with_id("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}", 20),
        table_with_id("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}", 21)
    ));
    let second = document(&table_with_id("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}", 30));
    let rels = r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdSlide1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rIdSlide2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/><Relationship Id="rIdTheme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/></Relationships>"#;
    package(&[first, second], rels, None, theme())
}

pub fn manual_qa_package() -> Vec<u8> {
    let first = document(&format!(
        "{}{}{}",
        table_with_id(CUSTOM_STYLE, 2),
        table_with_id("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}", 20),
        table_with_id("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}", 21),
    ));
    let second = document(&table_with_id("{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}", 30));
    let hostile = table_styles_relationship(
        "rIdHostile",
        "https://user:secret@example.test/styles.xml",
        Some("External"),
    );
    let valid = table_styles_relationship("rIdStyles", "tableStyles.xml", None);
    let rels = format!(
        r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdSlide1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rIdSlide2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide2.xml"/><Relationship Id="rIdTheme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>{hostile}{valid}</Relationships>"#
    );
    let style = format!(
        r#"<?xml version="1.0"?><a:tblStyleLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:tblStyle styleId="{CUSTOM_STYLE}" styleName="manual"><a:wholeTbl><a:tcStyle><a:fillRef idx="2"><a:schemeClr val="accent2"><a:tint val="20000"/></a:schemeClr></a:fillRef></a:tcStyle></a:wholeTbl></a:tblStyle></a:tblStyleLst>"#
    );
    package(&[first, second], &rels, Some(&style), theme())
}

pub fn boundary_package() -> Vec<u8> {
    let merged = format!(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="40" name="vertical merge"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr lastRow="1" lastCol="1"><a:tableStyleId>{CUSTOM_STYLE}</a:tableStyleId></a:tblPr><a:tblGrid><a:gridCol w="1"/><a:gridCol w="1"/></a:tblGrid><a:tr h="1"><a:tc rowSpan="2"><a:txBody><a:bodyPr/><a:p><a:r><a:t>anchor</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>{}</a:tr><a:tr h="1"><a:tc vMerge="1"><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc>{}</a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#,
        cell("top-right", ""),
        cell("bottom-right", ""),
    );
    let zero = format!(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="41" name="zero"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr><a:tableStyleId>{CUSTOM_STYLE}</a:tableStyleId></a:tblPr><a:tblGrid/></a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#
    );
    let text = format!(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="42" name="text"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr><a:tableStyleId>{CUSTOM_STYLE}</a:tableStyleId></a:tblPr><a:tblGrid><a:gridCol w="1"/></a:tblGrid><a:tr h="1">{}</a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#,
        cell(
            "explicit",
            r#"<a:rPr b="0"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:rPr>"#,
        ),
    );
    let styled = valid_styles().replace(
        "<a:wholeTbl><a:tcStyle>",
        r#"<a:wholeTbl><a:tcTxStyle b="1"><a:srgbClr val="00AA00"/></a:tcTxStyle><a:tcStyle>"#,
    ).replace(
        "</a:tblStyle></a:tblStyleLst>",
        r#"<a:seCell><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="090909"/></a:solidFill></a:fill></a:tcStyle></a:seCell></a:tblStyle></a:tblStyleLst>"#,
    );
    let relationships = format!(
        r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdSlide" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rIdTheme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>{}</Relationships>"#,
        table_styles_relationship("rIdStyles", "tableStyles.xml", None),
    );
    package(
        &[document(&format!("{merged}{zero}{text}"))],
        &relationships,
        Some(&styled),
        theme(),
    )
}

fn cell(text: &str, run_properties: &str) -> String {
    format!(
        r#"<a:tc><a:txBody><a:bodyPr/><a:p><a:r>{run_properties}<a:t>{text}</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>"#
    )
}

fn table_with_id(style_id: &str, id: u32) -> String {
    table(style_id, "1", "1", false).replacen("id=\"2\"", &format!("id=\"{id}\""), 1)
}

fn package(slides: &[String], rels: &str, style_part: Option<&str>, theme_xml: &str) -> Vec<u8> {
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    let presentation = if slides.len() == 1 {
        r#"<?xml version="1.0"?><p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:sldIdLst><p:sldId id="256" r:id="rIdSlide"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/></p:presentation>"#.to_owned()
    } else {
        r#"<?xml version="1.0"?><p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:sldIdLst><p:sldId id="256" r:id="rIdSlide1"/><p:sldId id="257" r:id="rIdSlide2"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/></p:presentation>"#.to_owned()
    };
    let types = content_types().replace(
        "</Types>",
        if slides.len() > 1 {
            r#"<Override PartName="/ppt/slides/slide2.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/></Types>"#
        } else {
            "</Types>"
        },
    );
    let fixed = [
        ("[Content_Types].xml", types.as_str()),
        ("_rels/.rels", root_relationships()),
        ("ppt/presentation.xml", presentation.as_str()),
        ("ppt/_rels/presentation.xml.rels", rels),
        ("ppt/theme/theme1.xml", theme_xml),
    ];
    for (path, xml) in fixed {
        archive.start_file(path, options).expect("fixture entry");
        archive.write_all(xml.as_bytes()).expect("fixture XML");
    }
    for (index, slide_xml) in slides.iter().enumerate() {
        archive
            .start_file(format!("ppt/slides/slide{}.xml", index + 1), options)
            .expect("slide entry");
        archive.write_all(slide_xml.as_bytes()).expect("slide XML");
        archive
            .start_file(
                format!("ppt/slides/_rels/slide{}.xml.rels", index + 1),
                options,
            )
            .expect("slide rels");
        archive
            .write_all(br#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#)
            .expect("slide rels XML");
    }
    if let Some(xml) = style_part {
        archive
            .start_file("ppt/tableStyles.xml", options)
            .expect("style entry");
        archive.write_all(xml.as_bytes()).expect("style XML");
    }
    archive.finish().expect("fixture archive").into_inner()
}

pub fn valid_styles() -> &'static str {
    r#"<?xml version="1.0"?><a:tblStyleLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><a:tblStyle styleId="{11111111-1111-1111-1111-111111111111}" styleName="Review"><a:wholeTbl><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="0D0D0D"/></a:solidFill></a:fill></a:tcStyle></a:wholeTbl></a:tblStyle></a:tblStyleLst>"#
}

fn content_types() -> &'static str {
    r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/><Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/><Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/><Override PartName="/ppt/tableStyles.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml"/></Types>"#
}

fn root_relationships() -> &'static str {
    r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#
}

fn theme() -> &'static str {
    r#"<?xml version="1.0"?><a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Review"><a:themeElements><a:clrScheme name="Review"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="111111"/></a:dk2><a:lt2><a:srgbClr val="EEEEEE"/></a:lt2><a:accent1><a:srgbClr val="111111"/></a:accent1><a:accent2><a:srgbClr val="222222"/></a:accent2><a:accent3><a:srgbClr val="333333"/></a:accent3><a:accent4><a:srgbClr val="444444"/></a:accent4><a:accent5><a:srgbClr val="555555"/></a:accent5><a:accent6><a:srgbClr val="666666"/></a:accent6><a:hlink><a:srgbClr val="777777"/></a:hlink><a:folHlink><a:srgbClr val="888888"/></a:folHlink></a:clrScheme><a:fontScheme name="Review"><a:majorFont><a:latin typeface="Major"/></a:majorFont><a:minorFont><a:latin typeface="Minor"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>"#
}

fn document(body: &str) -> String {
    format!(
        r#"<?xml version="1.0"?><p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{body}</p:spTree></p:cSld></p:sld>"#
    )
}

fn table(style_id: &str, first_col: &str, band_col: &str, _explicit: bool) -> String {
    format!(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="1000000" cy="500000"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr firstCol="{first_col}" bandCol="{band_col}"><a:tableStyleId>{style_id}</a:tableStyleId></a:tblPr><a:tblGrid><a:gridCol w="1000000"/></a:tblGrid><a:tr h="500000"><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>cell</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc></a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#
    )
}
