use std::io::{Cursor, Write};

use zip::ZipWriter;
use zip::write::SimpleFileOptions;

const CUSTOM_STYLE: &str = "{11111111-1111-1111-1111-111111111111}";
pub const BUILT_IN_STYLE: &str = "{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}";
pub const INVALID_STYLE: &str = "{AAAAAAAA-AAAA-AAAA-AAAA-AAAAAAAAAAAA}";
pub const OTHER_BUILT_IN_STYLE: &str = "{00A15C55-8517-42AA-B614-E9B94910E393}";

pub fn package() -> Vec<u8> {
    package_with_slide(slide())
}

pub fn invalid_package() -> Vec<u8> {
    package_with_slide(document(&table(INVALID_STYLE, "1", "1", false)))
}

pub fn other_built_in_package() -> Vec<u8> {
    package_with_slide(document(&table(OTHER_BUILT_IN_STYLE, "1", "1", false)))
}

pub fn empty_id_package() -> Vec<u8> {
    let empty = table("", "1", "1", false)
        .replace("<a:tableStyleId></a:tableStyleId>", "<a:tableStyleId/>");
    package_with_slide(document(&empty))
}

pub fn corner_gate_package() -> Vec<u8> {
    let table = table(CUSTOM_STYLE, "0", "0", false)
        .replace("lastRow=\"1\"", "lastRow=\"0\"")
        .replace("lastCol=\"1\"", "lastCol=\"0\"");
    package_with_slide(document(&table))
}

pub fn merged_package() -> Vec<u8> {
    let cells = r#"<a:tc gridSpan="2"><a:txBody><a:bodyPr/><a:p><a:r><a:t>anchor</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc hMerge="1"><a:txBody><a:bodyPr/><a:p/></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>middle</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc><a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>logical-last</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>"#;
    let table = format!(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="merged"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr lastCol="1"><a:tableStyleId>{CUSTOM_STYLE}</a:tableStyleId></a:tblPr><a:tblGrid><a:gridCol w="1"/><a:gridCol w="1"/><a:gridCol w="1"/><a:gridCol w="1"/></a:tblGrid><a:tr h="1">{cells}</a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#
    );
    package_with_slide(document(&table))
}

pub fn spoof_namespace_package() -> Vec<u8> {
    let spoof = table(CUSTOM_STYLE, "1", "1", false)
        .replace(
            "<a:tableStyleId>",
            "<x:tableStyleId xmlns:x=\"urn:not-drawingml\">",
        )
        .replace("</a:tableStyleId>", "</x:tableStyleId>");
    package_with_slide(document(&spoof))
}

pub fn cdata_id_package() -> Vec<u8> {
    let cdata = table(CUSTOM_STYLE, "1", "1", false).replace(
        &format!("<a:tableStyleId>{CUSTOM_STYLE}</a:tableStyleId>"),
        &format!("<a:tableStyleId><![CDATA[{CUSTOM_STYLE}]]></a:tableStyleId>"),
    );
    package_with_slide(document(&cdata))
}

pub fn style_feature_package() -> Vec<u8> {
    let enhanced = styles().replacen(
        "<a:wholeTbl><a:tcStyle>",
        r#"<a:tblBg><a:fill><a:solidFill><a:srgbClr val="101010"/></a:solidFill></a:fill></a:tblBg><a:wholeTbl><a:tcTxStyle b="on" i="on"><a:fontRef idx="minor"/><a:schemeClr val="tx1"><a:tint val="50000"/></a:schemeClr></a:tcTxStyle><a:tcStyle><a:tcBdr><a:insideH><a:ln w="12700"><a:solidFill><a:srgbClr val="00AA00"/></a:solidFill></a:ln></a:insideH><a:insideV><a:ln w="12700"><a:solidFill><a:srgbClr val="0000AA"/></a:solidFill></a:ln></a:insideV></a:tcBdr>"#,
        1,
    );
    package_with_styles(slide(), &enhanced)
}

pub fn wrong_style_namespace_package() -> Vec<u8> {
    let wrong = styles()
        .replace("xmlns:a=", "xmlns:x=")
        .replace(
            "http://schemas.openxmlformats.org/drawingml/2006/main",
            "urn:not-drawingml",
        )
        .replace("<a:", "<x:")
        .replace("</a:", "</x:");
    package_with_styles(document(&table(CUSTOM_STYLE, "1", "1", false)), &wrong)
}

pub fn unsupported_primitive_package() -> Vec<u8> {
    let unsupported = styles().replacen(
        "<a:wholeTbl><a:tcStyle>",
        "<a:wholeTbl><a:tcStyle><a:fill><a:gradFill/></a:fill>",
        1,
    );
    package_with_styles(
        document(&table(CUSTOM_STYLE, "1", "1", false)),
        &unsupported,
    )
}

pub fn duplicate_style_package() -> Vec<u8> {
    let duplicate = r#"<a:tblStyle styleId="{11111111-1111-1111-1111-111111111111}" styleName="Duplicate"><a:wholeTbl><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="FEED01"/></a:solidFill></a:fill></a:tcStyle></a:wholeTbl></a:tblStyle>"#;
    let duplicated = styles().replace("</a:tblStyleLst>", &format!("{duplicate}</a:tblStyleLst>"));
    package_with_styles(document(&table(CUSTOM_STYLE, "1", "1", false)), &duplicated)
}

pub fn invalid_bool_package() -> Vec<u8> {
    let invalid =
        table(CUSTOM_STYLE, "1", "1", false).replace("firstRow=\"1\"", "firstRow=\"maybe\"");
    package_with_slide(document(&invalid))
}

fn package_with_slide(slide_xml: String) -> Vec<u8> {
    package_with_styles(slide_xml, styles())
}

fn package_with_styles(slide_xml: String, styles_xml: &str) -> Vec<u8> {
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default();
    for (path, xml) in [
        ("[Content_Types].xml", content_types()),
        ("_rels/.rels", root_relationships()),
        ("ppt/presentation.xml", presentation()),
        (
            "ppt/_rels/presentation.xml.rels",
            presentation_relationships(),
        ),
        ("ppt/slides/slide1.xml", slide_xml.as_str()),
        ("ppt/slides/_rels/slide1.xml.rels", empty_relationships()),
        ("ppt/theme/theme1.xml", theme()),
        ("ppt/tableStyles.xml", styles_xml),
    ] {
        archive.start_file(path, options).expect("fixture entry");
        archive.write_all(xml.as_bytes()).expect("fixture XML");
    }
    archive.finish().expect("fixture archive").into_inner()
}

fn content_types() -> &'static str {
    r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/><Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/><Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/><Override PartName="/ppt/tableStyles.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml"/></Types>"#
}

fn root_relationships() -> &'static str {
    r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#
}

fn presentation() -> &'static str {
    r#"<?xml version="1.0"?><p:presentation xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><p:sldIdLst><p:sldId id="256" r:id="rIdSlide"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/></p:presentation>"#
}

fn presentation_relationships() -> &'static str {
    r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rIdSlide" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rIdTheme" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/><Relationship Id="rIdStyles" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/tableStyles" Target="tableStyles.xml"/></Relationships>"#
}

fn empty_relationships() -> &'static str {
    r#"<?xml version="1.0"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"/>"#
}

fn theme() -> &'static str {
    r#"<?xml version="1.0"?><a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="Fixture"><a:themeElements><a:clrScheme name="Fixture"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="222222"/></a:dk2><a:lt2><a:srgbClr val="EEEEEE"/></a:lt2><a:accent1><a:srgbClr val="111111"/></a:accent1><a:accent2><a:srgbClr val="222222"/></a:accent2><a:accent3><a:srgbClr val="333333"/></a:accent3><a:accent4><a:srgbClr val="444444"/></a:accent4><a:accent5><a:srgbClr val="555555"/></a:accent5><a:accent6><a:srgbClr val="666666"/></a:accent6><a:hlink><a:srgbClr val="777777"/></a:hlink><a:folHlink><a:srgbClr val="888888"/></a:folHlink></a:clrScheme><a:fontScheme name="Fixture"><a:majorFont><a:latin typeface="Major"/></a:majorFont><a:minorFont><a:latin typeface="Minor"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>"#
}

fn slide() -> String {
    document(&format!(
        "{}{}",
        table(CUSTOM_STYLE, "1", "1", true),
        table(BUILT_IN_STYLE, "1", "1", false),
    ))
}

fn document(body: &str) -> String {
    format!(
        r#"<?xml version="1.0"?><p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/>{body}</p:spTree></p:cSld></p:sld>"#
    )
}

fn table(style_id: &str, first_col: &str, band_col: &str, explicit: bool) -> String {
    let mut rows = String::new();
    for row in 0..3 {
        let mut cells = String::new();
        for col in 0..3 {
            let properties = if explicit && row == 1 && col == 1 {
                r#"<a:tcPr><a:solidFill><a:srgbClr val="ABCDEF"/></a:solidFill><a:lnT w="12700"><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></a:lnT></a:tcPr>"#
            } else if explicit && row == 1 && col == 2 {
                "<a:tcPr><a:noFill/></a:tcPr>"
            } else {
                "<a:tcPr/>"
            };
            cells.push_str(&format!(r#"<a:tc><a:txBody><a:bodyPr/><a:p><a:r><a:t>r{row}c{col}</a:t></a:r></a:p></a:txBody>{properties}</a:tc>"#));
        }
        rows.push_str(&format!(r#"<a:tr h="500000">{cells}</a:tr>"#));
    }
    format!(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="2" name="table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="3000000" cy="1500000"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr firstRow="1" lastRow="1" firstCol="{first_col}" lastCol="1" bandRow="1" bandCol="{band_col}"><a:tableStyleId>{style_id}</a:tableStyleId></a:tblPr><a:tblGrid><a:gridCol w="1000000"/><a:gridCol w="1000000"/><a:gridCol w="1000000"/></a:tblGrid>{rows}</a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#
    )
}

fn styles() -> &'static str {
    r#"<?xml version="1.0"?><a:tblStyleLst xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" def="{11111111-1111-1111-1111-111111111111}"><a:tblStyle styleId="{11111111-1111-1111-1111-111111111111}" styleName="Regions"><a:wholeTbl><a:tcStyle><a:fill><a:solidFill><a:schemeClr val="accent1"/></a:solidFill></a:fill></a:tcStyle></a:wholeTbl><a:band1H><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="020202"/></a:solidFill></a:fill></a:tcStyle></a:band1H><a:band2H><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="030303"/></a:solidFill></a:fill></a:tcStyle></a:band2H><a:band1V><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="040404"/></a:solidFill></a:fill></a:tcStyle></a:band1V><a:band2V><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="050505"/></a:solidFill></a:fill></a:tcStyle></a:band2V><a:lastCol><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="060606"/></a:solidFill></a:fill></a:tcStyle></a:lastCol><a:firstCol><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="070707"/></a:solidFill></a:fill></a:tcStyle></a:firstCol><a:lastRow><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="080808"/></a:solidFill></a:fill></a:tcStyle></a:lastRow><a:seCell><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="090909"/></a:solidFill></a:fill></a:tcStyle></a:seCell><a:swCell><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="0A0A0A"/></a:solidFill></a:fill></a:tcStyle></a:swCell><a:firstRow><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="0B0B0B"/></a:solidFill></a:fill></a:tcStyle></a:firstRow><a:neCell><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="0C0C0C"/></a:solidFill></a:fill></a:tcStyle></a:neCell><a:nwCell><a:tcStyle><a:fill><a:solidFill><a:srgbClr val="0D0D0D"/></a:solidFill></a:fill></a:tcStyle></a:nwCell></a:tblStyle></a:tblStyleLst>"#
}
