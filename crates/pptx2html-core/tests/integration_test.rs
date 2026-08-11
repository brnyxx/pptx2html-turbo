//! Integration tests — minimal PPTX generation → parsing → HTML rendering verification

mod fixtures;

use pptx2html_core::model::*;

fn parse_pptx(data: &[u8]) -> pptx2html_core::model::Presentation {
    pptx2html_core::parser::PptxParser::parse_bytes(data).expect("PPTX parsing failed")
}

fn render_html(data: &[u8]) -> String {
    let pres = parse_pptx(data);
    pptx2html_core::renderer::HtmlRenderer::render(&pres).expect("HTML rendering failed")
}

fn build_background_image_pptx() -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:blipFill>
          <a:blip r:embed="rId2"/>
          <a:stretch><a:fillRect/></a:stretch>
        </a:blipFill>
        <a:effectLst/>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/>
</Relationships>"#;

    let png_data: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="png" ContentType="image/png"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    )
    .unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/media/image1.png", opts).unwrap();
    zip.write_all(&png_data).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T">
  <a:themeElements>
    <a:clrScheme name="O">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_column_chart_pptx() -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="Chart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let chart_xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:barChart>
        <c:barDir val="col"/>
        <c:grouping val="clustered"/>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue</c:v></c:tx>
          <c:cat>
            <c:strLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>Q1</c:v></c:pt>
              <c:pt idx="1"><c:v>Q2</c:v></c:pt>
              <c:pt idx="2"><c:v>Q3</c:v></c:pt>
            </c:strLit>
          </c:cat>
          <c:val>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>10</c:v></c:pt>
              <c:pt idx="1"><c:v>20</c:v></c:pt>
              <c:pt idx="2"><c:v>30</c:v></c:pt>
            </c:numLit>
          </c:val>
        </c:ser>
        <c:axId val="123"/>
        <c:axId val="456"/>
      </c:barChart>
      <c:catAx>
        <c:axId val="123"/>
        <c:crossAx val="456"/>
      </c:catAx>
      <c:valAx>
        <c:axId val="456"/>
        <c:crossAx val="123"/>
      </c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#,
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    )
    .unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T">
  <a:themeElements>
    <a:clrScheme name="O">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_chart_with_axis_titles_pptx() -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="Chart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let chart_xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:barChart>
        <c:barDir val="col"/>
        <c:grouping val="clustered"/>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue</c:v></c:tx>
          <c:cat>
            <c:strLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>Q1</c:v></c:pt>
              <c:pt idx="1"><c:v>Q2</c:v></c:pt>
              <c:pt idx="2"><c:v>Q3</c:v></c:pt>
            </c:strLit>
          </c:cat>
          <c:val>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>10</c:v></c:pt>
              <c:pt idx="1"><c:v>20</c:v></c:pt>
              <c:pt idx="2"><c:v>30</c:v></c:pt>
            </c:numLit>
          </c:val>
        </c:ser>
        <c:axId val="123"/>
        <c:axId val="456"/>
      </c:barChart>
      <c:catAx>
        <c:axId val="123"/>
        <c:title>
          <c:tx>
            <c:rich>
              <a:bodyPr/>
              <a:lstStyle/>
              <a:p><a:r><a:t>Quarter</a:t></a:r></a:p>
            </c:rich>
          </c:tx>
        </c:title>
        <c:crossAx val="456"/>
      </c:catAx>
      <c:valAx>
        <c:axId val="456"/>
        <c:title>
          <c:tx>
            <c:rich>
              <a:bodyPr/>
              <a:lstStyle/>
              <a:p><a:r><a:t>Revenue ($M)</a:t></a:r></a:p>
            </c:rich>
          </c:tx>
        </c:title>
        <c:crossAx val="123"/>
      </c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#,
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    )
    .unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T">
  <a:themeElements>
    <a:clrScheme name="O">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_bar_chart_pptx() -> Vec<u8> {
    build_chart_pptx("bar", 1)
}

fn build_multi_series_column_chart_pptx() -> Vec<u8> {
    build_chart_pptx("col", 2)
}

fn build_line_chart_pptx(series_count: usize) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="LineChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let mut series_xml = String::new();
    for idx in 0..series_count {
        let name = if idx == 0 { "Revenue" } else { "Profit" };
        let v1 = if idx == 0 { "10" } else { "5" };
        let v2 = if idx == 0 { "20" } else { "7" };
        let v3 = if idx == 0 { "30" } else { "9" };
        series_xml.push_str(&format!(
            r#"<c:ser>
          <c:idx val="{idx}"/>
          <c:order val="{idx}"/>
          <c:tx><c:v>{name}</c:v></c:tx>
          <c:cat>
            <c:strLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>Q1</c:v></c:pt>
              <c:pt idx="1"><c:v>Q2</c:v></c:pt>
              <c:pt idx="2"><c:v>Q3</c:v></c:pt>
            </c:strLit>
          </c:cat>
          <c:val>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>{v1}</c:v></c:pt>
              <c:pt idx="1"><c:v>{v2}</c:v></c:pt>
              <c:pt idx="2"><c:v>{v3}</c:v></c:pt>
            </c:numLit>
          </c:val>
        </c:ser>"#
        ));
    }

    let chart_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:lineChart>
        <c:grouping val="standard"/>
        {series_xml}
        <c:axId val="123"/>
        <c:axId val="456"/>
      </c:lineChart>
      <c:catAx>
        <c:axId val="123"/>
        <c:crossAx val="456"/>
      </c:catAx>
      <c:valAx>
        <c:axId val="456"/>
        <c:crossAx val="123"/>
      </c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    )
    .unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T">
  <a:themeElements>
    <a:clrScheme name="O">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_line_chart_with_value_labels_pptx() -> Vec<u8> {
    build_line_chart_with_label_flags_pptx(true, false, false, None)
}

fn build_line_chart_with_label_flags_pptx(
    show_value: bool,
    show_category_name: bool,
    show_series_name: bool,
    label_position: Option<&str>,
) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="LineChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let show_value = if show_value { 1 } else { 0 };
    let show_category_name = if show_category_name { 1 } else { 0 };
    let show_series_name = if show_series_name { 1 } else { 0 };
    let label_position_xml = label_position
        .map(|value| format!("<c:dLblPos val=\"{value}\"/>"))
        .unwrap_or_default();
    let chart_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:lineChart>
        <c:grouping val="standard"/>
        <c:dLbls><c:showVal val="{show_value}"/><c:showCatName val="{show_category_name}"/><c:showSerName val="{show_series_name}"/>{label_position_xml}</c:dLbls>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue</c:v></c:tx>
          <c:cat><c:strLit><c:ptCount val="3"/><c:pt idx="0"><c:v>Q1</c:v></c:pt><c:pt idx="1"><c:v>Q2</c:v></c:pt><c:pt idx="2"><c:v>Q3</c:v></c:pt></c:strLit></c:cat>
          <c:val><c:numLit><c:ptCount val="3"/><c:pt idx="0"><c:v>10</c:v></c:pt><c:pt idx="1"><c:v>20</c:v></c:pt><c:pt idx="2"><c:v>30</c:v></c:pt></c:numLit></c:val>
        </c:ser>
        <c:axId val="123"/>
        <c:axId val="456"/>
      </c:lineChart>
      <c:catAx><c:axId val="123"/><c:crossAx val="456"/></c:catAx>
      <c:valAx><c:axId val="456"/><c:crossAx val="123"/></c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;
    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/></p:presentation>"#).unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T"><a:themeElements><a:clrScheme name="O"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme><a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>"#).unwrap();
    zip.finish().unwrap().into_inner()
}

fn build_area_chart_with_value_labels_pptx() -> Vec<u8> {
    build_area_chart_with_label_flags_pptx(true, false, false, None)
}

fn build_area_chart_with_label_flags_pptx(
    show_value: bool,
    show_category_name: bool,
    show_series_name: bool,
    label_position: Option<&str>,
) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="AreaChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let show_value = if show_value { 1 } else { 0 };
    let show_category_name = if show_category_name { 1 } else { 0 };
    let show_series_name = if show_series_name { 1 } else { 0 };
    let label_position_xml = label_position
        .map(|value| format!("<c:dLblPos val=\"{value}\"/>"))
        .unwrap_or_default();
    let chart_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:areaChart>
        <c:grouping val="standard"/>
        <c:dLbls><c:showVal val="{show_value}"/><c:showCatName val="{show_category_name}"/><c:showSerName val="{show_series_name}"/>{label_position_xml}</c:dLbls>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue</c:v></c:tx>
          <c:cat><c:strLit><c:ptCount val="3"/><c:pt idx="0"><c:v>Q1</c:v></c:pt><c:pt idx="1"><c:v>Q2</c:v></c:pt><c:pt idx="2"><c:v>Q3</c:v></c:pt></c:strLit></c:cat>
          <c:val><c:numLit><c:ptCount val="3"/><c:pt idx="0"><c:v>10</c:v></c:pt><c:pt idx="1"><c:v>20</c:v></c:pt><c:pt idx="2"><c:v>30</c:v></c:pt></c:numLit></c:val>
        </c:ser>
        <c:axId val="123"/>
        <c:axId val="456"/>
      </c:areaChart>
      <c:catAx><c:axId val="123"/><c:crossAx val="456"/></c:catAx>
      <c:valAx><c:axId val="456"/><c:crossAx val="123"/></c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;
    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/></p:presentation>"#).unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T"><a:themeElements><a:clrScheme name="O"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme><a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>"#).unwrap();
    zip.finish().unwrap().into_inner()
}

fn build_area_chart_pptx() -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="AreaChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let chart_xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:areaChart>
        <c:grouping val="standard"/>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue</c:v></c:tx>
          <c:cat>
            <c:strLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>Q1</c:v></c:pt>
              <c:pt idx="1"><c:v>Q2</c:v></c:pt>
              <c:pt idx="2"><c:v>Q3</c:v></c:pt>
            </c:strLit>
          </c:cat>
          <c:val>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>10</c:v></c:pt>
              <c:pt idx="1"><c:v>20</c:v></c:pt>
              <c:pt idx="2"><c:v>30</c:v></c:pt>
            </c:numLit>
          </c:val>
        </c:ser>
        <c:axId val="123"/>
        <c:axId val="456"/>
      </c:areaChart>
      <c:catAx><c:axId val="123"/><c:crossAx val="456"/></c:catAx>
      <c:valAx><c:axId val="456"/><c:crossAx val="123"/></c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#,
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    )
    .unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T">
  <a:themeElements>
    <a:clrScheme name="O">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_area3d_chart_pptx() -> Vec<u8> {
    build_area_chart_variant_pptx("area3DChart", "standard")
}

fn build_area3d_chart_with_grouping_pptx(grouping: &str) -> Vec<u8> {
    build_area_chart_variant_pptx("area3DChart", grouping)
}

fn build_area_chart_variant_pptx(chart_tag: &str, grouping: &str) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="Area3DChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let chart_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:{chart_tag}>
        <c:grouping val="{grouping}"/>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue</c:v></c:tx>
          <c:cat>
            <c:strLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>Q1</c:v></c:pt>
              <c:pt idx="1"><c:v>Q2</c:v></c:pt>
              <c:pt idx="2"><c:v>Q3</c:v></c:pt>
            </c:strLit>
          </c:cat>
          <c:val>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>10</c:v></c:pt>
              <c:pt idx="1"><c:v>20</c:v></c:pt>
              <c:pt idx="2"><c:v>30</c:v></c:pt>
            </c:numLit>
          </c:val>
        </c:ser>
        <c:axId val="123"/>
        <c:axId val="456"/>
      </c:{chart_tag}>
      <c:catAx><c:axId val="123"/><c:crossAx val="456"/></c:catAx>
      <c:valAx><c:axId val="456"/><c:crossAx val="123"/></c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/></p:presentation>"#).unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T"><a:themeElements><a:clrScheme name="O"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme><a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>"#).unwrap();
    zip.finish().unwrap().into_inner()
}

fn build_scatter_chart_pptx() -> Vec<u8> {
    build_scatter_chart_with_style_pptx("marker")
}

fn build_scatter_chart_with_style_pptx(scatter_style: &str) -> Vec<u8> {
    build_scatter_chart_with_label_flags_pptx(scatter_style, false, false, false, None)
}

fn build_scatter_chart_with_flags_pptx(
    scatter_style: &str,
    show_value_labels: bool,
    show_category_name: bool,
    show_series_name: bool,
) -> Vec<u8> {
    build_scatter_chart_with_label_flags_pptx(
        scatter_style,
        show_value_labels,
        show_category_name,
        show_series_name,
        None,
    )
}

fn build_scatter_chart_with_label_flags_pptx(
    scatter_style: &str,
    show_value_labels: bool,
    show_category_name: bool,
    show_series_name: bool,
    label_position: Option<&str>,
) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="ScatterChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let dlabels_xml = if show_value_labels || show_category_name || show_series_name {
        let show_value = if show_value_labels { 1 } else { 0 };
        let show_category_name = if show_category_name { 1 } else { 0 };
        let show_series_name = if show_series_name { 1 } else { 0 };
        let label_position_xml = label_position
            .map(|value| format!("<c:dLblPos val=\"{value}\"/>"))
            .unwrap_or_default();
        format!(
            "<c:dLbls><c:showVal val=\"{show_value}\"/><c:showCatName val=\"{show_category_name}\"/><c:showSerName val=\"{show_series_name}\"/>{label_position_xml}</c:dLbls>"
        )
    } else {
        String::new()
    };
    let chart_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:scatterChart>
        <c:scatterStyle val="{scatter_style}"/>
        {dlabels_xml}
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue</c:v></c:tx>
          <c:marker>
            <c:symbol val="diamond"/>
            <c:size val="9"/>
          </c:marker>
          <c:xVal>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>1</c:v></c:pt>
              <c:pt idx="1"><c:v>2</c:v></c:pt>
              <c:pt idx="2"><c:v>3</c:v></c:pt>
            </c:numLit>
          </c:xVal>
          <c:yVal>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>10</c:v></c:pt>
              <c:pt idx="1"><c:v>20</c:v></c:pt>
              <c:pt idx="2"><c:v>15</c:v></c:pt>
            </c:numLit>
          </c:yVal>
        </c:ser>
        <c:axId val="123"/>
        <c:axId val="456"/>
      </c:scatterChart>
      <c:valAx><c:axId val="123"/><c:crossAx val="456"/></c:valAx>
      <c:valAx><c:axId val="456"/><c:crossAx val="123"/></c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    )
    .unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T">
  <a:themeElements>
    <a:clrScheme name="O">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_line_chart_with_marker_pptx(symbol: &str, size: Option<i32>) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="LineChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let marker_size_xml = size
        .map(|value| format!("<c:size val=\"{value}\"/>"))
        .unwrap_or_default();
    let chart_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:lineChart>
        <c:grouping val="standard"/>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue</c:v></c:tx>
          <c:marker>
            <c:symbol val="{symbol}"/>
            {marker_size_xml}
          </c:marker>
          <c:cat>
            <c:strLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>Q1</c:v></c:pt>
              <c:pt idx="1"><c:v>Q2</c:v></c:pt>
              <c:pt idx="2"><c:v>Q3</c:v></c:pt>
            </c:strLit>
          </c:cat>
          <c:val>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>10</c:v></c:pt>
              <c:pt idx="1"><c:v>20</c:v></c:pt>
              <c:pt idx="2"><c:v>30</c:v></c:pt>
            </c:numLit>
          </c:val>
        </c:ser>
        <c:axId val="123"/>
        <c:axId val="456"/>
      </c:lineChart>
      <c:catAx><c:axId val="123"/><c:crossAx val="456"/></c:catAx>
      <c:valAx><c:axId val="456"/><c:crossAx val="123"/></c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    )
    .unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T">
  <a:themeElements>
    <a:clrScheme name="O">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_pie_chart_pptx() -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="PieChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let chart_xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:pieChart>
        <c:varyColors val="1"/>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue Share</c:v></c:tx>
          <c:cat>
            <c:strLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>North</c:v></c:pt>
              <c:pt idx="1"><c:v>South</c:v></c:pt>
              <c:pt idx="2"><c:v>West</c:v></c:pt>
            </c:strLit>
          </c:cat>
          <c:val>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>30</c:v></c:pt>
              <c:pt idx="1"><c:v>20</c:v></c:pt>
              <c:pt idx="2"><c:v>50</c:v></c:pt>
            </c:numLit>
          </c:val>
        </c:ser>
        <c:firstSliceAng val="45"/>
      </c:pieChart>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#,
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    )
    .unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T">
  <a:themeElements>
    <a:clrScheme name="O">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_pie_chart_with_dlabels_pptx(
    show_val: bool,
    show_cat_name: bool,
    show_percent: bool,
    label_position: Option<&str>,
) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="PieChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let show_val = if show_val { 1 } else { 0 };
    let show_cat_name = if show_cat_name { 1 } else { 0 };
    let show_percent = if show_percent { 1 } else { 0 };
    let label_position_xml = label_position
        .map(|value| format!("<c:dLblPos val=\"{value}\"/>"))
        .unwrap_or_default();
    let chart_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:pieChart>
        <c:varyColors val="1"/>
        <c:dLbls>
          <c:showVal val="{show_val}"/>
          <c:showCatName val="{show_cat_name}"/>
          <c:showPercent val="{show_percent}"/>
          {label_position_xml}
        </c:dLbls>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue Share</c:v></c:tx>
          <c:cat>
            <c:strLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>North</c:v></c:pt>
              <c:pt idx="1"><c:v>South</c:v></c:pt>
              <c:pt idx="2"><c:v>West</c:v></c:pt>
            </c:strLit>
          </c:cat>
          <c:val>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>30</c:v></c:pt>
              <c:pt idx="1"><c:v>20</c:v></c:pt>
              <c:pt idx="2"><c:v>50</c:v></c:pt>
            </c:numLit>
          </c:val>
        </c:ser>
      </c:pieChart>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    )
    .unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T">
  <a:themeElements>
    <a:clrScheme name="O">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_doughnut_chart_with_dlabels_pptx(
    show_val: bool,
    show_cat_name: bool,
    show_percent: bool,
    label_position: Option<&str>,
) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="DoughnutChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let show_val = if show_val { 1 } else { 0 };
    let show_cat_name = if show_cat_name { 1 } else { 0 };
    let show_percent = if show_percent { 1 } else { 0 };
    let label_position_xml = label_position
        .map(|value| format!("<c:dLblPos val=\"{value}\"/>"))
        .unwrap_or_default();
    let chart_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:doughnutChart>
        <c:varyColors val="1"/>
        <c:dLbls>
          <c:showVal val="{show_val}"/>
          <c:showCatName val="{show_cat_name}"/>
          <c:showPercent val="{show_percent}"/>
          {label_position_xml}
        </c:dLbls>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue Mix</c:v></c:tx>
          <c:cat>
            <c:strLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>North</c:v></c:pt>
              <c:pt idx="1"><c:v>South</c:v></c:pt>
              <c:pt idx="2"><c:v>West</c:v></c:pt>
            </c:strLit>
          </c:cat>
          <c:val>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>30</c:v></c:pt>
              <c:pt idx="1"><c:v>20</c:v></c:pt>
              <c:pt idx="2"><c:v>50</c:v></c:pt>
            </c:numLit>
          </c:val>
        </c:ser>
        <c:holeSize val="60"/>
      </c:doughnutChart>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    )
    .unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T">
  <a:themeElements>
    <a:clrScheme name="O">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_doughnut_chart_pptx() -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="DoughnutChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let chart_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:doughnutChart>
        <c:varyColors val="1"/>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue Mix</c:v></c:tx>
          <c:cat>
            <c:strLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>North</c:v></c:pt>
              <c:pt idx="1"><c:v>South</c:v></c:pt>
              <c:pt idx="2"><c:v>West</c:v></c:pt>
            </c:strLit>
          </c:cat>
          <c:val>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>30</c:v></c:pt>
              <c:pt idx="1"><c:v>20</c:v></c:pt>
              <c:pt idx="2"><c:v>50</c:v></c:pt>
            </c:numLit>
          </c:val>
        </c:ser>
        <c:holeSize val="60"/>
        <c:firstSliceAng val="30"/>
      </c:doughnutChart>
    </c:plotArea>
    <c:legend><c:legendPos val="r"/></c:legend>
  </c:chart>
</c:chartSpace>"#;

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    )
    .unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T">
  <a:themeElements>
    <a:clrScheme name="O">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_multi_series_pie_chart_pptx() -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="PieChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let chart_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:pieChart>
        <c:varyColors val="1"/>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue Share</c:v></c:tx>
          <c:cat><c:strLit><c:ptCount val="2"/><c:pt idx="0"><c:v>North</c:v></c:pt><c:pt idx="1"><c:v>South</c:v></c:pt></c:strLit></c:cat>
          <c:val><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>30</c:v></c:pt><c:pt idx="1"><c:v>70</c:v></c:pt></c:numLit></c:val>
        </c:ser>
        <c:ser>
          <c:idx val="1"/>
          <c:order val="1"/>
          <c:tx><c:v>Profit Share</c:v></c:tx>
          <c:cat><c:strLit><c:ptCount val="2"/><c:pt idx="0"><c:v>North</c:v></c:pt><c:pt idx="1"><c:v>South</c:v></c:pt></c:strLit></c:cat>
          <c:val><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>20</c:v></c:pt><c:pt idx="1"><c:v>80</c:v></c:pt></c:numLit></c:val>
        </c:ser>
      </c:pieChart>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#;

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    )
    .unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T">
  <a:themeElements>
    <a:clrScheme name="O">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_pie3d_chart_pptx() -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="Pie3DChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let chart_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:pie3DChart>
        <c:varyColors val="1"/>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue Share</c:v></c:tx>
          <c:cat><c:strLit><c:ptCount val="2"/><c:pt idx="0"><c:v>North</c:v></c:pt><c:pt idx="1"><c:v>South</c:v></c:pt></c:strLit></c:cat>
          <c:val><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>30</c:v></c:pt><c:pt idx="1"><c:v>70</c:v></c:pt></c:numLit></c:val>
        </c:ser>
      </c:pie3DChart>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#;

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;
    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/></p:presentation>"#).unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T"><a:themeElements><a:clrScheme name="O"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme><a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>"#).unwrap();
    zip.finish().unwrap().into_inner()
}

fn build_of_pie_chart_pptx() -> Vec<u8> {
    build_of_pie_chart_variant_pptx("pie", "pos", Some("2"), false)
}

fn build_of_pie_chart_variant_pptx(
    of_pie_type: &str,
    split_type: &str,
    split_pos: Option<&str>,
    with_data_labels: bool,
) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="OfPieChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let split_pos_xml = split_pos
        .map(|val| format!("<c:splitPos val=\"{val}\"/>"))
        .unwrap_or_default();
    let d_lbls_xml = if with_data_labels {
        "<c:dLbls><c:showVal val=\"1\"/></c:dLbls>"
    } else {
        ""
    };

    let chart_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:ofPieChart>
        <c:ofPieType val="{of_pie_type}"/>
        <c:varyColors val="1"/>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue Share</c:v></c:tx>
          <c:cat>
            <c:strLit>
              <c:ptCount val="4"/>
              <c:pt idx="0"><c:v>North</c:v></c:pt>
              <c:pt idx="1"><c:v>South</c:v></c:pt>
              <c:pt idx="2"><c:v>East</c:v></c:pt>
              <c:pt idx="3"><c:v>West</c:v></c:pt>
            </c:strLit>
          </c:cat>
          <c:val>
            <c:numLit>
              <c:ptCount val="4"/>
              <c:pt idx="0"><c:v>40</c:v></c:pt>
              <c:pt idx="1"><c:v>30</c:v></c:pt>
              <c:pt idx="2"><c:v>20</c:v></c:pt>
              <c:pt idx="3"><c:v>10</c:v></c:pt>
            </c:numLit>
          </c:val>
        </c:ser>
        <c:gapWidth val="120"/>
        <c:splitType val="{split_type}"/>
        {split_pos_xml}
        {d_lbls_xml}
        <c:secondPieSize val="70"/>
      </c:ofPieChart>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/></p:presentation>"#).unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T"><a:themeElements><a:clrScheme name="O"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme><a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_chart_preview_fallback_pptx() -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="RadarChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let chart_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:radarChart>
        <c:radarStyle val="standard"/>
      </c:radarChart>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#;

    let chart_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rIdPreview" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/chart-preview.png"/>
</Relationships>"#;

    let png_data: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE2, 0x21, 0xBC, 0x33, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Default Extension="png" ContentType="image/png"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/></p:presentation>"#).unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/charts/_rels/chart1.xml.rels", opts)
        .unwrap();
    zip.write_all(chart_rels.as_bytes()).unwrap();
    zip.start_file("ppt/media/chart-preview.png", opts).unwrap();
    zip.write_all(&png_data).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T"><a:themeElements><a:clrScheme name="O"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme><a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_radar_chart_pptx(style: &str) -> Vec<u8> {
    build_radar_chart_variant_pptx(style, 1, false, None, None)
}

fn build_multi_series_radar_chart_pptx(style: &str) -> Vec<u8> {
    build_radar_chart_variant_pptx(style, 2, false, None, None)
}

fn build_radar_chart_with_value_labels_pptx(style: &str) -> Vec<u8> {
    build_radar_chart_variant_pptx(style, 1, true, None, None)
}

fn build_radar_chart_with_marker_pptx(symbol: &str, size: Option<i32>) -> Vec<u8> {
    build_radar_chart_variant_pptx("marker", 1, false, Some(symbol), size)
}

fn build_radar_chart_variant_pptx(
    style: &str,
    series_count: usize,
    show_value_labels: bool,
    marker_symbol: Option<&str>,
    marker_size: Option<i32>,
) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="RadarChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let mut series_xml = String::new();
    for idx in 0..series_count {
        let (name, values) = if idx == 0 {
            ("Coverage", [10, 25, 15, 30])
        } else {
            ("Forecast", [20, 15, 28, 18])
        };
        let marker_xml = marker_symbol
            .map(|symbol| {
                let marker_size_xml = marker_size
                    .map(|value| format!("<c:size val=\"{value}\"/>"))
                    .unwrap_or_default();
                format!("<c:marker><c:symbol val=\"{symbol}\"/>{marker_size_xml}</c:marker>")
            })
            .unwrap_or_default();
        series_xml.push_str(&format!(
            r#"
        <c:ser>
          <c:idx val="{idx}"/>
          <c:order val="{idx}"/>
          <c:tx><c:v>{name}</c:v></c:tx>
          {marker_xml}
          <c:cat>
            <c:strLit>
              <c:ptCount val="4"/>
              <c:pt idx="0"><c:v>Q1</c:v></c:pt>
              <c:pt idx="1"><c:v>Q2</c:v></c:pt>
              <c:pt idx="2"><c:v>Q3</c:v></c:pt>
              <c:pt idx="3"><c:v>Q4</c:v></c:pt>
            </c:strLit>
          </c:cat>
          <c:val>
            <c:numLit>
              <c:ptCount val="4"/>
              <c:pt idx="0"><c:v>{}</c:v></c:pt>
              <c:pt idx="1"><c:v>{}</c:v></c:pt>
              <c:pt idx="2"><c:v>{}</c:v></c:pt>
              <c:pt idx="3"><c:v>{}</c:v></c:pt>
            </c:numLit>
          </c:val>
        </c:ser>"#,
            values[0], values[1], values[2], values[3]
        ));
    }
    let d_lbls_xml = if show_value_labels {
        "<c:dLbls><c:showVal val=\"1\"/></c:dLbls>"
    } else {
        ""
    };

    let chart_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:radarChart>
        <c:radarStyle val="{style}"/>
        {series_xml}
        {d_lbls_xml}
        <c:axId val="123"/>
        <c:axId val="456"/>
      </c:radarChart>
      <c:catAx>
        <c:axId val="123"/>
        <c:crossAx val="456"/>
      </c:catAx>
      <c:valAx>
        <c:axId val="456"/>
        <c:crossAx val="123"/>
      </c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/></p:presentation>"#).unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T"><a:themeElements><a:clrScheme name="O"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme><a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_bubble_chart_pptx() -> Vec<u8> {
    build_bubble_chart_semantics_pptx(1, false, Some("100"), Some("area"), Some("0"), None)
}

fn build_multi_series_bubble_chart_pptx() -> Vec<u8> {
    build_bubble_chart_semantics_pptx(2, false, Some("100"), Some("area"), Some("0"), None)
}

fn build_bubble_chart_with_value_labels_pptx() -> Vec<u8> {
    build_bubble_chart_semantics_pptx(1, true, Some("100"), Some("area"), Some("0"), None)
}

fn build_bubble_chart_with_scale_pptx(scale: &str) -> Vec<u8> {
    build_bubble_chart_semantics_pptx(1, false, Some(scale), Some("area"), Some("0"), None)
}

fn build_bubble_chart_with_width_semantics_pptx() -> Vec<u8> {
    build_bubble_chart_semantics_pptx(1, false, Some("100"), Some("w"), Some("0"), None)
}

fn build_bubble_chart_with_negative_sizes_pptx() -> Vec<u8> {
    build_bubble_chart_semantics_pptx(
        1,
        false,
        Some("100"),
        Some("area"),
        Some("0"),
        Some(&[[-6.0, 14.0, 10.0]]),
    )
}

fn build_bubble_chart_with_show_neg_bubbles_pptx(show_neg_bubbles: &str) -> Vec<u8> {
    build_bubble_chart_semantics_pptx(
        1,
        false,
        Some("100"),
        Some("area"),
        Some(show_neg_bubbles),
        None,
    )
}

fn build_bubble_chart_semantics_pptx(
    series_count: usize,
    show_value_labels: bool,
    bubble_scale: Option<&str>,
    size_represents: Option<&str>,
    show_neg_bubbles: Option<&str>,
    bubble_size_sets: Option<&[[f64; 3]]>,
) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="BubbleChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let mut series_xml = String::new();
    for idx in 0..series_count {
        let bubble_sizes = bubble_size_sets
            .and_then(|sets| sets.get(idx).copied())
            .unwrap_or(if idx == 0 {
                [6.0, 14.0, 10.0]
            } else {
                [8.0, 12.0, 9.0]
            });
        let (name, x_values, y_values) = if idx == 0 {
            ("Pipeline", [10, 20, 35], [15, 28, 12])
        } else {
            ("Forecast", [14, 26, 32], [18, 20, 26])
        };
        series_xml.push_str(&format!(
            r#"
        <c:ser>
          <c:idx val="{idx}"/>
          <c:order val="{idx}"/>
          <c:tx><c:v>{name}</c:v></c:tx>
          <c:xVal>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>{}</c:v></c:pt>
              <c:pt idx="1"><c:v>{}</c:v></c:pt>
              <c:pt idx="2"><c:v>{}</c:v></c:pt>
            </c:numLit>
          </c:xVal>
          <c:yVal>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>{}</c:v></c:pt>
              <c:pt idx="1"><c:v>{}</c:v></c:pt>
              <c:pt idx="2"><c:v>{}</c:v></c:pt>
            </c:numLit>
          </c:yVal>
          <c:bubbleSize>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>{}</c:v></c:pt>
              <c:pt idx="1"><c:v>{}</c:v></c:pt>
              <c:pt idx="2"><c:v>{}</c:v></c:pt>
            </c:numLit>
          </c:bubbleSize>
        </c:ser>"#,
            x_values[0],
            x_values[1],
            x_values[2],
            y_values[0],
            y_values[1],
            y_values[2],
            bubble_sizes[0],
            bubble_sizes[1],
            bubble_sizes[2]
        ));
    }
    let d_lbls_xml = if show_value_labels {
        "<c:dLbls><c:showVal val=\"1\"/></c:dLbls>"
    } else {
        ""
    };
    let bubble_scale_xml = bubble_scale
        .map(|value| format!("<c:bubbleScale val=\"{value}\"/>"))
        .unwrap_or_default();
    let size_represents_xml = size_represents
        .map(|value| format!("<c:sizeRepresents val=\"{value}\"/>"))
        .unwrap_or_default();
    let show_neg_bubbles_xml = show_neg_bubbles
        .map(|value| format!("<c:showNegBubbles val=\"{value}\"/>"))
        .unwrap_or_default();

    let chart_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:bubbleChart>
        <c:varyColors val="0"/>
        {bubble_scale_xml}
        {show_neg_bubbles_xml}
        {size_represents_xml}
        {series_xml}
        {d_lbls_xml}
        <c:axId val="123"/>
        <c:axId val="456"/>
      </c:bubbleChart>
      <c:catAx><c:axId val="123"/><c:crossAx val="456"/></c:catAx>
      <c:valAx><c:axId val="456"/><c:crossAx val="123"/></c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/></p:presentation>"#).unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T"><a:themeElements><a:clrScheme name="O"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme><a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_chart_pptx(bar_dir: &str, series_count: usize) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="Chart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let mut series_xml = String::new();
    for idx in 0..series_count {
        let name = if idx == 0 { "Revenue" } else { "Profit" };
        let v1 = if idx == 0 { "10" } else { "5" };
        let v2 = if idx == 0 { "20" } else { "7" };
        let v3 = if idx == 0 { "30" } else { "9" };
        series_xml.push_str(&format!(
            r#"<c:ser>
          <c:idx val="{idx}"/>
          <c:order val="{idx}"/>
          <c:tx><c:v>{name}</c:v></c:tx>
          <c:cat>
            <c:strLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>Q1</c:v></c:pt>
              <c:pt idx="1"><c:v>Q2</c:v></c:pt>
              <c:pt idx="2"><c:v>Q3</c:v></c:pt>
            </c:strLit>
          </c:cat>
          <c:val>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>{v1}</c:v></c:pt>
              <c:pt idx="1"><c:v>{v2}</c:v></c:pt>
              <c:pt idx="2"><c:v>{v3}</c:v></c:pt>
            </c:numLit>
          </c:val>
        </c:ser>"#
        ));
    }

    let chart_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:barChart>
        <c:barDir val="{bar_dir}"/>
        <c:grouping val="clustered"/>
        {series_xml}
        <c:axId val="123"/>
        <c:axId val="456"/>
      </c:barChart>
      <c:catAx>
        <c:axId val="123"/>
        <c:crossAx val="456"/>
      </c:catAx>
      <c:valAx>
        <c:axId val="456"/>
        <c:crossAx val="123"/>
      </c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    )
    .unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T">
  <a:themeElements>
    <a:clrScheme name="O">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_chart_with_value_labels_pptx(
    bar_dir: &str,
    show_value: bool,
    show_category_name: bool,
) -> Vec<u8> {
    build_chart_with_label_flags_pptx(bar_dir, show_value, show_category_name, false, None)
}

fn build_multi_series_chart_with_label_flags_pptx(
    bar_dir: &str,
    show_value: bool,
    show_series_name: bool,
) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="Chart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let show_value = if show_value { 1 } else { 0 };
    let show_series_name = if show_series_name { 1 } else { 0 };
    let chart_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:barChart>
        <c:barDir val="{bar_dir}"/>
        <c:grouping val="clustered"/>
        <c:dLbls>
          <c:showVal val="{show_value}"/>
          <c:showSerName val="{show_series_name}"/>
        </c:dLbls>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue</c:v></c:tx>
          <c:cat><c:strLit><c:ptCount val="3"/><c:pt idx="0"><c:v>Q1</c:v></c:pt><c:pt idx="1"><c:v>Q2</c:v></c:pt><c:pt idx="2"><c:v>Q3</c:v></c:pt></c:strLit></c:cat>
          <c:val><c:numLit><c:ptCount val="3"/><c:pt idx="0"><c:v>10</c:v></c:pt><c:pt idx="1"><c:v>20</c:v></c:pt><c:pt idx="2"><c:v>30</c:v></c:pt></c:numLit></c:val>
        </c:ser>
        <c:ser>
          <c:idx val="1"/>
          <c:order val="1"/>
          <c:tx><c:v>Profit</c:v></c:tx>
          <c:cat><c:strLit><c:ptCount val="3"/><c:pt idx="0"><c:v>Q1</c:v></c:pt><c:pt idx="1"><c:v>Q2</c:v></c:pt><c:pt idx="2"><c:v>Q3</c:v></c:pt></c:strLit></c:cat>
          <c:val><c:numLit><c:ptCount val="3"/><c:pt idx="0"><c:v>5</c:v></c:pt><c:pt idx="1"><c:v>7</c:v></c:pt><c:pt idx="2"><c:v>9</c:v></c:pt></c:numLit></c:val>
        </c:ser>
        <c:axId val="123"/>
        <c:axId val="456"/>
      </c:barChart>
      <c:catAx><c:axId val="123"/><c:crossAx val="456"/></c:catAx>
      <c:valAx><c:axId val="456"/><c:crossAx val="123"/></c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;
    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/></p:presentation>"#).unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T"><a:themeElements><a:clrScheme name="O"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme><a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>"#).unwrap();
    zip.finish().unwrap().into_inner()
}

fn build_chart_with_label_flags_pptx(
    bar_dir: &str,
    show_value: bool,
    show_category_name: bool,
    show_percent: bool,
    label_position: Option<&str>,
) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="Chart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let show_value = if show_value { 1 } else { 0 };
    let show_category_name = if show_category_name { 1 } else { 0 };
    let show_percent = if show_percent { 1 } else { 0 };
    let label_position_xml = label_position
        .map(|pos| format!("<c:dLblPos val=\"{pos}\"/>"))
        .unwrap_or_default();
    let chart_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:barChart>
        <c:barDir val="{bar_dir}"/>
        <c:grouping val="clustered"/>
        <c:dLbls>
          <c:showVal val="{show_value}"/>
          <c:showCatName val="{show_category_name}"/>
          <c:showPercent val="{show_percent}"/>
          {label_position_xml}
        </c:dLbls>
        <c:ser>
          <c:idx val="0"/>
          <c:order val="0"/>
          <c:tx><c:v>Revenue</c:v></c:tx>
          <c:cat>
            <c:strLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>Q1</c:v></c:pt>
              <c:pt idx="1"><c:v>Q2</c:v></c:pt>
              <c:pt idx="2"><c:v>Q3</c:v></c:pt>
            </c:strLit>
          </c:cat>
          <c:val>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>10</c:v></c:pt>
              <c:pt idx="1"><c:v>20</c:v></c:pt>
              <c:pt idx="2"><c:v>30</c:v></c:pt>
            </c:numLit>
          </c:val>
        </c:ser>
        <c:axId val="123"/>
        <c:axId val="456"/>
      </c:barChart>
      <c:catAx><c:axId val="123"/><c:crossAx val="456"/></c:catAx>
      <c:valAx><c:axId val="456"/><c:crossAx val="123"/></c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    )
    .unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T">
  <a:themeElements>
    <a:clrScheme name="O">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_chart_spacing_pptx(
    bar_dir: &str,
    series_count: usize,
    gap_width: i32,
    overlap: i32,
) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="Chart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let mut series_xml = String::new();
    for idx in 0..series_count {
        let name = if idx == 0 { "Revenue" } else { "Profit" };
        let v1 = if idx == 0 { "10" } else { "5" };
        let v2 = if idx == 0 { "20" } else { "7" };
        let v3 = if idx == 0 { "30" } else { "9" };
        series_xml.push_str(&format!(
            r#"<c:ser>
          <c:idx val="{idx}"/>
          <c:order val="{idx}"/>
          <c:tx><c:v>{name}</c:v></c:tx>
          <c:cat>
            <c:strLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>Q1</c:v></c:pt>
              <c:pt idx="1"><c:v>Q2</c:v></c:pt>
              <c:pt idx="2"><c:v>Q3</c:v></c:pt>
            </c:strLit>
          </c:cat>
          <c:val>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>{v1}</c:v></c:pt>
              <c:pt idx="1"><c:v>{v2}</c:v></c:pt>
              <c:pt idx="2"><c:v>{v3}</c:v></c:pt>
            </c:numLit>
          </c:val>
        </c:ser>"#
        ));
    }

    let chart_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:barChart>
        <c:barDir val="{bar_dir}"/>
        <c:grouping val="clustered"/>
        <c:gapWidth val="{gap_width}"/>
        <c:overlap val="{overlap}"/>
        {series_xml}
        <c:axId val="123"/>
        <c:axId val="456"/>
      </c:barChart>
      <c:catAx>
        <c:axId val="123"/>
        <c:crossAx val="456"/>
      </c:catAx>
      <c:valAx>
        <c:axId val="456"/>
        <c:crossAx val="123"/>
      </c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;

    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
                xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
                xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst>
  <p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst>
  <p:sldSz cx="9144000" cy="6858000"/>
</p:presentation>"#,
    )
    .unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/>
  <Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>
</Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T">
  <a:themeElements>
    <a:clrScheme name="O">
      <a:dk1><a:srgbClr val="000000"/></a:dk1>
      <a:lt1><a:srgbClr val="FFFFFF"/></a:lt1>
      <a:dk2><a:srgbClr val="44546A"/></a:dk2>
      <a:lt2><a:srgbClr val="E7E6E6"/></a:lt2>
      <a:accent1><a:srgbClr val="4472C4"/></a:accent1>
      <a:accent2><a:srgbClr val="ED7D31"/></a:accent2>
      <a:accent3><a:srgbClr val="A5A5A5"/></a:accent3>
      <a:accent4><a:srgbClr val="FFC000"/></a:accent4>
      <a:accent5><a:srgbClr val="5B9BD5"/></a:accent5>
      <a:accent6><a:srgbClr val="70AD47"/></a:accent6>
      <a:hlink><a:srgbClr val="0563C1"/></a:hlink>
      <a:folHlink><a:srgbClr val="954F72"/></a:folHlink>
    </a:clrScheme>
    <a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme>
  </a:themeElements>
</a:theme>"#).unwrap();

    zip.finish().unwrap().into_inner()
}

fn build_stacked_chart_pptx(bar_dir: &str, grouping: &str, series_count: usize) -> Vec<u8> {
    build_stacked_chart_with_label_flags_pptx(bar_dir, grouping, series_count, false, false, false)
}

fn build_stacked_chart_with_value_labels_pptx(
    bar_dir: &str,
    grouping: &str,
    series_count: usize,
    show_value_labels: bool,
) -> Vec<u8> {
    build_stacked_chart_with_label_flags_pptx(
        bar_dir,
        grouping,
        series_count,
        show_value_labels,
        false,
        false,
    )
}

fn build_stacked_chart_with_label_flags_pptx(
    bar_dir: &str,
    grouping: &str,
    series_count: usize,
    show_value_labels: bool,
    show_category_name: bool,
    show_percent: bool,
) -> Vec<u8> {
    use std::io::{Cursor, Write};
    use zip::ZipWriter;
    use zip::write::SimpleFileOptions;

    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"
       xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">
  <p:cSld>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
      <p:graphicFrame>
        <p:nvGraphicFramePr><p:cNvPr id="2" name="StackedChart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
        <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
        <a:graphic>
          <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
            <c:chart r:id="rId2"/>
          </a:graphicData>
        </a:graphic>
      </p:graphicFrame>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let slide_rels = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/>
</Relationships>"#;

    let mut series_xml = String::new();
    for idx in 0..series_count {
        let name = if idx == 0 { "Revenue" } else { "Profit" };
        let v1 = if idx == 0 { "10" } else { "5" };
        let v2 = if idx == 0 { "20" } else { "15" };
        let v3 = if idx == 0 { "30" } else { "10" };
        series_xml.push_str(&format!(
            r#"<c:ser>
          <c:idx val="{idx}"/>
          <c:order val="{idx}"/>
          <c:tx><c:v>{name}</c:v></c:tx>
          <c:cat>
            <c:strLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>Q1</c:v></c:pt>
              <c:pt idx="1"><c:v>Q2</c:v></c:pt>
              <c:pt idx="2"><c:v>Q3</c:v></c:pt>
            </c:strLit>
          </c:cat>
          <c:val>
            <c:numLit>
              <c:ptCount val="3"/>
              <c:pt idx="0"><c:v>{v1}</c:v></c:pt>
              <c:pt idx="1"><c:v>{v2}</c:v></c:pt>
              <c:pt idx="2"><c:v>{v3}</c:v></c:pt>
            </c:numLit>
          </c:val>
        </c:ser>"#
        ));
    }

    let dlabels_xml = if show_value_labels || show_category_name || show_percent {
        let show_value = if show_value_labels { 1 } else { 0 };
        let show_category_name = if show_category_name { 1 } else { 0 };
        let show_percent = if show_percent { 1 } else { 0 };
        format!(
            "<c:dLbls><c:showVal val=\"{show_value}\"/><c:showCatName val=\"{show_category_name}\"/><c:showPercent val=\"{show_percent}\"/></c:dLbls>"
        )
    } else {
        String::new()
    };

    let chart_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"
              xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
              xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <c:chart>
    <c:plotArea>
      <c:layout/>
      <c:barChart>
        <c:barDir val="{bar_dir}"/>
        <c:grouping val="{grouping}"/>
        {dlabels_xml}
        {series_xml}
        <c:axId val="123"/>
        <c:axId val="456"/>
      </c:barChart>
      <c:catAx><c:axId val="123"/><c:crossAx val="456"/></c:catAx>
      <c:valAx><c:axId val="456"/><c:crossAx val="123"/></c:valAx>
    </c:plotArea>
  </c:chart>
</c:chartSpace>"#
    );

    let buf = Vec::new();
    let cursor = Cursor::new(buf);
    let mut zip = ZipWriter::new(cursor);
    let opts = SimpleFileOptions::default();
    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/ppt/presentation.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml"/>
  <Override PartName="/ppt/slides/slide1.xml" ContentType="application/vnd.openxmlformats-officedocument.presentationml.slide+xml"/>
  <Override PartName="/ppt/charts/chart1.xml" ContentType="application/vnd.openxmlformats-officedocument.drawingml.chart+xml"/>
  <Override PartName="/ppt/theme/theme1.xml" ContentType="application/vnd.openxmlformats-officedocument.theme+xml"/>
</Types>"#;
    zip.start_file("[Content_Types].xml", opts).unwrap();
    zip.write_all(content_types.as_bytes()).unwrap();
    zip.start_file("_rels/.rels", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="ppt/presentation.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/presentation.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/></p:presentation>"#).unwrap();
    zip.start_file("ppt/_rels/presentation.xml.rels", opts)
        .unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/></Relationships>"#).unwrap();
    zip.start_file("ppt/slides/slide1.xml", opts).unwrap();
    zip.write_all(slide_xml.as_bytes()).unwrap();
    zip.start_file("ppt/slides/_rels/slide1.xml.rels", opts)
        .unwrap();
    zip.write_all(slide_rels.as_bytes()).unwrap();
    zip.start_file("ppt/charts/chart1.xml", opts).unwrap();
    zip.write_all(chart_xml.as_bytes()).unwrap();
    zip.start_file("ppt/theme/theme1.xml", opts).unwrap();
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<a:theme xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" name="T"><a:themeElements><a:clrScheme name="O"><a:dk1><a:srgbClr val="000000"/></a:dk1><a:lt1><a:srgbClr val="FFFFFF"/></a:lt1><a:dk2><a:srgbClr val="44546A"/></a:dk2><a:lt2><a:srgbClr val="E7E6E6"/></a:lt2><a:accent1><a:srgbClr val="4472C4"/></a:accent1><a:accent2><a:srgbClr val="ED7D31"/></a:accent2><a:accent3><a:srgbClr val="A5A5A5"/></a:accent3><a:accent4><a:srgbClr val="FFC000"/></a:accent4><a:accent5><a:srgbClr val="5B9BD5"/></a:accent5><a:accent6><a:srgbClr val="70AD47"/></a:accent6><a:hlink><a:srgbClr val="0563C1"/></a:hlink><a:folHlink><a:srgbClr val="954F72"/></a:folHlink></a:clrScheme><a:fontScheme name="O"><a:majorFont><a:latin typeface="Calibri"/></a:majorFont><a:minorFont><a:latin typeface="Calibri"/></a:minorFont></a:fontScheme></a:themeElements></a:theme>"#).unwrap();
    zip.finish().unwrap().into_inner()
}

// ── Basic parsing tests ──

#[test]
fn test_empty_slide() {
    let pptx = fixtures::MinimalPptx::new("").build();
    let pres = parse_pptx(&pptx);
    assert_eq!(pres.slides.len(), 1);
    assert!(pres.slides[0].shapes.is_empty());
}

#[test]
fn test_slide_size() {
    let pptx = fixtures::MinimalPptx::new("").build();
    let pres = parse_pptx(&pptx);
    // 9144000 EMU = 10 inches = 960px at 96dpi
    assert!((pres.slide_size.width.to_px() - 960.0).abs() < 0.1);
    assert!((pres.slide_size.height.to_px() - 720.0).abs() < 0.1);
}

// ── Theme color tests ──

#[test]
fn test_theme_parsing() {
    let pptx = fixtures::MinimalPptx::new("").build();
    let pres = parse_pptx(&pptx);
    let theme = pres.primary_theme().expect("Theme not found");
    assert_eq!(theme.color_scheme.accent1, "4472C4");
    assert_eq!(theme.color_scheme.dk1, "000000");
    assert_eq!(theme.color_scheme.lt1, "FFFFFF");
}

#[test]
fn test_theme_color_in_text() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="TextBox 1"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:r>
            <a:rPr lang="en-US" sz="2400">
              <a:solidFill><a:schemeClr val="accent1"/></a:solidFill>
            </a:rPr>
            <a:t>Theme Color Text</a:t>
          </a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    // accent1 = #4472C4
    assert!(
        html.contains("#4472C4"),
        "Theme accent1 color not found in HTML: {html}"
    );
}

// ── ClrMap tests ──

#[test]
fn test_clr_map_parsing() {
    let pptx = fixtures::MinimalPptx::new("")
        .with_clr_map(r#"bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink""#)
        .build();
    let pres = parse_pptx(&pptx);
    assert!(!pres.clr_map.is_empty());
    assert_eq!(pres.clr_map.get("tx1"), Some(&"dk1".to_string()));
    assert_eq!(pres.clr_map.get("bg1"), Some(&"lt1".to_string()));
}

#[test]
fn test_clr_map_color_resolution() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:r>
            <a:rPr sz="1800">
              <a:solidFill><a:schemeClr val="tx1"/></a:solidFill>
            </a:rPr>
            <a:t>Mapped Color</a:t>
          </a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide)
        .with_clr_map(r#"bg1="lt1" tx1="dk1" bg2="lt2" tx2="dk2" accent1="accent1" accent2="accent2" accent3="accent3" accent4="accent4" accent5="accent5" accent6="accent6" hlink="hlink" folHlink="folHlink""#)
        .build();
    let html = render_html(&pptx);
    // tx1 → dk1 → "000000"
    assert!(
        html.contains("#000000"),
        "ClrMap tx1→dk1 color not found: {html}"
    );
}

// ── SolidFill tests ──

#[test]
fn test_solid_fill_rgb() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="500000" y="500000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:solidFill><a:srgbClr val="FF5733"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    match &shape.fill {
        Fill::Solid(sf) => {
            assert_eq!(sf.color.kind, color::ColorKind::Rgb("FF5733".to_string()));
        }
        other => panic!("Expected SolidFill: {other:?}"),
    }
}

#[test]
fn test_solid_fill_theme() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="500000" y="500000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:solidFill><a:schemeClr val="accent2"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    // accent2 = ED7D31
    assert!(
        html.contains("#ED7D31"),
        "accent2 fill color not found: {html}"
    );
}

// ── Color modifier tests ──

#[test]
fn test_color_modifier_tint() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:solidFill>
          <a:schemeClr val="accent1">
            <a:tint val="50000"/>
          </a:schemeClr>
        </a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    match &shape.fill {
        Fill::Solid(sf) => {
            assert_eq!(sf.color.modifiers.len(), 1);
            assert_eq!(sf.color.modifiers[0], color::ColorModifier::Tint(50000));
        }
        other => panic!("Expected SolidFill: {other:?}"),
    }
}

#[test]
fn test_color_modifier_lum_mod_off() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:solidFill>
          <a:schemeClr val="accent1">
            <a:lumMod val="75000"/>
            <a:lumOff val="25000"/>
          </a:schemeClr>
        </a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    match &shape.fill {
        Fill::Solid(sf) => {
            assert_eq!(sf.color.modifiers.len(), 2);
            assert_eq!(sf.color.modifiers[0], color::ColorModifier::LumMod(75000));
            assert_eq!(sf.color.modifiers[1], color::ColorModifier::LumOff(25000));
        }
        other => panic!("Expected SolidFill: {other:?}"),
    }
}

// ── GradientFill tests ──

#[test]
fn test_gradient_fill() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:gradFill>
          <a:gsLst>
            <a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs>
            <a:gs pos="50000"><a:srgbClr val="00FF00"/></a:gs>
            <a:gs pos="100000"><a:srgbClr val="0000FF"/></a:gs>
          </a:gsLst>
          <a:lin ang="5400000" scaled="1"/>
        </a:gradFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    match &shape.fill {
        Fill::Gradient(gf) => {
            assert_eq!(gf.stops.len(), 3);
            assert!((gf.stops[0].position - 0.0).abs() < 0.01);
            assert!((gf.stops[1].position - 0.5).abs() < 0.01);
            assert!((gf.stops[2].position - 1.0).abs() < 0.01);
            assert!((gf.angle - 90.0).abs() < 0.1); // 5400000/60000 = 90
        }
        other => panic!("Expected GradientFill: {other:?}"),
    }
}

#[test]
fn test_gradient_fill_html() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:gradFill>
          <a:gsLst>
            <a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs>
            <a:gs pos="100000"><a:srgbClr val="0000FF"/></a:gs>
          </a:gsLst>
          <a:lin ang="5400000"/>
        </a:gradFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("linear-gradient"),
        "Gradient not found in HTML: {html}"
    );
    assert!(html.contains("#FF0000"), "Start color not found");
    assert!(html.contains("#0000FF"), "End color not found");
}

// ── Border tests ──

#[test]
fn test_border_parsing() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:ln w="25400">
          <a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>
          <a:prstDash val="dash"/>
        </a:ln>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    // 25400 EMU = 2pt
    assert!((shape.border.width - 2.0).abs() < 0.1);
    assert!(matches!(shape.border.style, BorderStyle::Dashed));
    assert_eq!(
        shape.border.color.kind,
        color::ColorKind::Rgb("FF0000".to_string())
    );
}

#[test]
fn test_border_html() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:ln w="12700">
          <a:solidFill><a:srgbClr val="0000FF"/></a:solidFill>
        </a:ln>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(html.contains("border:"), "border not found: {html}");
    assert!(html.contains("#0000FF"), "border color not found: {html}");
}

// ── bodyPr tests ──

#[test]
fn test_body_pr_vertical_align() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr anchor="ctr" lIns="91440" tIns="45720" rIns="91440" bIns="45720"/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>Centered</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    let tb = shape.text_body.as_ref().expect("TextBody not found");
    assert!(matches!(tb.vertical_align, VerticalAlign::Middle));
    // 91440 EMU = 7.2pt
    assert!((tb.margins.left - 7.2).abs() < 0.1);
    assert!((tb.margins.top - 3.6).abs() < 0.1);
}

#[test]
fn test_body_pr_html_rendering() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Box"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="2000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
      <p:txBody>
        <a:bodyPr anchor="b"/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>Bottom</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("v-bottom"),
        "v-bottom class not found: {html}"
    );
}

// ── NoFill tests ──

#[test]
fn test_no_fill() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:noFill/>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    assert!(matches!(shape.fill, Fill::NoFill));
}

// ── E2E HTML rendering tests ──

#[test]
fn test_full_html_structure() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Title"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="500000" y="300000"/><a:ext cx="8000000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:solidFill><a:schemeClr val="accent1"/></a:solidFill>
      </p:spPr>
      <p:txBody>
        <a:bodyPr anchor="ctr"/>
        <a:p>
          <a:pPr algn="ctr"/>
          <a:r>
            <a:rPr sz="3600" b="1">
              <a:solidFill><a:srgbClr val="FFFFFF"/></a:solidFill>
              <a:latin typeface="Arial"/>
            </a:rPr>
            <a:t>Hello World</a:t>
          </a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);

    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("pptx-container"));
    assert!(html.contains("Hello World"));
    assert!(html.contains("font-weight: bold"));
    assert!(html.contains("text-align: center"));
    assert!(html.contains("#FFFFFF"));
    assert!(html.contains("#4472C4")); // accent1 fill
    assert!(html.contains("v-middle"));
    assert!(html.contains("Arial"));
}

// ── Composite test: multiple shapes + various attributes ──

#[test]
fn test_multiple_shapes() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Shape1"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="ellipse"/>
        <a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>
      </p:spPr>
    </p:sp>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="3" name="Shape2"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="3000000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="roundRect"/>
        <a:solidFill><a:srgbClr val="00FF00"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    assert_eq!(pres.slides[0].shapes.len(), 2);

    let html = render_html(&pptx);
    assert!(html.contains("#FF0000"));
    assert!(html.contains("#00FF00"));
    // Ellipse and roundRect are now rendered as SVG paths
    assert!(
        html.contains("shape-svg"),
        "Expected SVG rendering for preset shapes"
    );
    assert!(html.contains("<path d="), "Expected SVG path element");
}

// ── Month 4: Preset Shape SVG tests ──

#[test]
fn test_preset_shape_svg_diamond() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Diamond"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="diamond"/>
        <a:solidFill><a:srgbClr val="4472C4"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(html.contains("shape-svg"), "Diamond should be SVG rendered");
    assert!(html.contains("<path d="), "Should contain SVG path");
    assert!(html.contains("#4472C4"), "Should contain fill color");
}

#[test]
fn test_preset_shape_svg_right_arrow() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Arrow"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rightArrow"/>
        <a:solidFill><a:srgbClr val="FF5733"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    assert!(
        matches!(&pres.slides[0].shapes[0].shape_type, ShapeType::Custom(name) if name == "rightArrow"),
        "Should be Custom(rightArrow)"
    );
    let html = render_html(&pptx);
    assert!(html.contains("shape-svg"));
    assert!(
        html.contains(
            "M0,26.2 L140.0,26.2 L140.0,0 L210.0,52.5 L140.0,105.0 L140.0,78.7 L0,78.7 Z"
        ),
        "rightArrow should use the narrower default head length seen in Office decks: {html}"
    );
}

#[test]
fn test_preset_shape_svg_left_arrow() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Arrow"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="leftArrow"/>
        <a:solidFill><a:srgbClr val="FF5733"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    assert!(
        matches!(&pres.slides[0].shapes[0].shape_type, ShapeType::Custom(name) if name == "leftArrow"),
        "Should be Custom(leftArrow)"
    );
    let html = render_html(&pptx);
    assert!(html.contains("shape-svg"));
    assert!(
        html.contains(
            "M210.0,26.2 L70.0,26.2 L70.0,0 L0,52.5 L70.0,105.0 L70.0,78.7 L210.0,78.7 Z"
        ),
        "leftArrow should use the narrower default head length seen in Office decks: {html}"
    );
}

#[test]
fn test_preset_shape_svg_up_arrow() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Arrow"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="upArrow"/>
        <a:solidFill><a:srgbClr val="FF5733"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains(
            "M26.2,105.0 L26.2,52.5 L0,52.5 L105.0,0 L210.0,52.5 L183.7,52.5 L183.7,105.0 Z"
        ),
        "upArrow should keep the wider default shaft from Office decks: {html}"
    );
}

#[test]
fn test_preset_shape_svg_down_arrow() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Arrow"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="downArrow"/>
        <a:solidFill><a:srgbClr val="FF5733"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("M26.2,0 L183.7,0 L183.7,52.5 L210.0,52.5 L105.0,105.0 L0,52.5 L26.2,52.5 Z"),
        "downArrow should keep the wider default shaft from Office decks: {html}"
    );
}

#[test]
fn test_preset_shape_with_adjust_values() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="RRect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="roundRect">
          <a:avLst>
            <a:gd name="adj" fmla="val 25000"/>
          </a:avLst>
        </a:prstGeom>
        <a:solidFill><a:srgbClr val="00FF00"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    assert!(shape.adjust_values.is_some(), "Should have adjust values");
    let adj = shape.adjust_values.as_ref().unwrap();
    assert_eq!(
        *adj.get("adj").unwrap() as i64,
        25000,
        "adj should be 25000"
    );
}

#[test]
fn test_preset_shape_star4_uses_office_default_body_width() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Star4"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="star4"/>
        <a:solidFill><a:srgbClr val="FFD700"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(html.contains("shape-svg"));
    assert!(
        html.contains(
            "M52.5,0 L61.9,43.0 L105.0,52.5 L61.9,61.9 L52.5,105.0 L43.0,61.9 L0,52.5 L43.0,43.0 Z"
        ),
        "star4 should keep the fuller Office default body width: {html}"
    );
}

#[test]
fn test_preset_shape_star5() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Star"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="star5"/>
        <a:solidFill><a:srgbClr val="FFD700"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(html.contains("shape-svg"));
    assert!(html.contains("#FFD700"));
    assert!(
        html.contains(
            "M 84.7,104.1 L 80.2,102.3 L 54.3,82.3 L 51.0,82.3 L 25.9,101.7 L 20.3,104.1 L 31.6,66.1 L 0.6,40.1 L 39.9,39.2 L 52.1,0.9 L 53.4,1.8 L 65.1,39.2 L 103.5,39.2 L 104.4,40.1 L 103.5,42.2 L 73.4,66.1 L 84.7,104.1 Z"
        ),
        "star5 should keep the fuller Office default body width: {html}"
    );
}

#[test]
fn test_rect_shape_no_svg() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:solidFill><a:srgbClr val="0000FF"/></a:solidFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    // rect is ShapeType::Rectangle (rendered without SVG path)
    // Note: .shape-svg class exists in CSS, so we check for actual SVG element usage
    assert!(
        !html.contains("<svg viewBox="),
        "Rect should not generate SVG viewBox"
    );
    assert!(html.contains("#0000FF"));
}

// ── Month 4: Text break (<a:br>) test ──

#[test]
fn test_text_break_renders_as_br() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="TextBox"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="2000000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:r><a:t>Line 1</a:t></a:r>
          <a:br/>
          <a:r><a:t>Line 2</a:t></a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let para = &pres.slides[0].shapes[0]
        .text_body
        .as_ref()
        .unwrap()
        .paragraphs[0];
    assert_eq!(para.runs.len(), 3, "Should have 3 runs (text, break, text)");
    assert!(para.runs[1].is_break, "Second run should be a break");

    let html = render_html(&pptx);
    assert!(html.contains("<br/>"), "Should render line break");
    assert!(html.contains("Line 1"), "Should contain first line text");
    assert!(html.contains("Line 2"), "Should contain second line text");
}

// ── Month 4: Vertical text test ──

#[test]
fn test_vertical_text_rendering() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="VertText"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1000000" cy="3000000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr vert="vert"/>
        <a:p><a:r><a:t>Vertical</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    assert_eq!(shape.vertical_text.as_deref(), Some("vert"));

    let html = render_html(&pptx);
    assert!(
        html.contains("writing-mode: vertical-rl"),
        "Should contain vertical writing mode"
    );
}

#[test]
fn test_vertical_text_270() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Vert270"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1000000" cy="3000000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr vert="vert270"/>
        <a:p><a:r><a:t>Rotated</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(html.contains("writing-mode: vertical-lr"));
}

// ── Month 4: Text highlight test ──

#[test]
fn test_text_highlight() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Highlight"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="2000000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:r>
            <a:rPr>
              <a:highlight><a:srgbClr val="FFFF00"/></a:highlight>
            </a:rPr>
            <a:t>Highlighted text</a:t>
          </a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let run = &pres.slides[0].shapes[0]
        .text_body
        .as_ref()
        .unwrap()
        .paragraphs[0]
        .runs[0];
    assert!(run.style.highlight.is_some(), "Should have highlight color");

    let html = render_html(&pptx);
    assert!(
        html.contains("background-color: #FFFF00"),
        "Should render highlight as background-color"
    );
}

// ── Month 4: Text shadow test ──

#[test]
fn test_text_shadow() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Shadow"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="2000000"/></a:xfrm>
      </p:spPr>
      <p:txBody>
        <a:bodyPr/>
        <a:p>
          <a:r>
            <a:rPr>
              <a:effectLst>
                <a:outerShdw blurRad="38100" dist="25400" dir="2700000">
                  <a:srgbClr val="000000"/>
                </a:outerShdw>
              </a:effectLst>
            </a:rPr>
            <a:t>Shadow text</a:t>
          </a:r>
        </a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let run = &pres.slides[0].shapes[0]
        .text_body
        .as_ref()
        .unwrap()
        .paragraphs[0]
        .runs[0];
    assert!(run.style.shadow.is_some(), "Should have text shadow");
    let shadow = run.style.shadow.as_ref().unwrap();
    assert!(shadow.blur_rad > 0.0, "Blur radius should be positive");

    let html = render_html(&pptx);
    assert!(
        html.contains("text-shadow:"),
        "Should render text-shadow CSS"
    );
}

// ── Month 4: Image crop test ──

#[test]
fn test_image_crop_parsing() {
    let slide = r#"
    <p:pic>
      <p:nvPicPr><p:cNvPr id="2" name="Pic"/><p:cNvPicPr/><p:nvPr/></p:nvPicPr>
      <p:blipFill>
        <a:blip r:embed="rId1"/>
        <a:srcRect l="10000" t="20000" r="15000" b="5000"/>
      </p:blipFill>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1500000"/></a:xfrm>
      </p:spPr>
    </p:pic>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    if let ShapeType::Picture(pic) = &shape.shape_type {
        assert!(pic.crop.is_some(), "Should have crop rect");
        let crop = pic.crop.as_ref().unwrap();
        assert!((crop.left - 0.1).abs() < 0.01, "Left crop should be ~0.1");
        assert!((crop.top - 0.2).abs() < 0.01, "Top crop should be ~0.2");
        assert!(
            (crop.right - 0.15).abs() < 0.01,
            "Right crop should be ~0.15"
        );
        assert!(
            (crop.bottom - 0.05).abs() < 0.01,
            "Bottom crop should be ~0.05"
        );
    } else {
        panic!("Expected Picture shape type");
    }
}

// ── Month 4: Chart detection test ──

#[test]
fn test_chart_detection() {
    let slide = r#"
    <p:graphicFrame>
      <p:nvGraphicFramePr><p:cNvPr id="2" name="Chart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
      <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
      <a:graphic>
        <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
          <c:chart r:id="rId2" xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"/>
        </a:graphicData>
      </a:graphic>
    </p:graphicFrame>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    assert!(
        matches!(&shape.shape_type, ShapeType::Chart(_)),
        "Should detect chart in graphicFrame"
    );
}

#[test]
fn test_chart_renders_placeholder() {
    let slide = r#"
    <p:graphicFrame>
      <p:nvGraphicFramePr><p:cNvPr id="2" name="Chart"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
      <p:xfrm><a:off x="100000" y="100000"/><a:ext cx="5000000" cy="3000000"/></p:xfrm>
      <a:graphic>
        <a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart">
          <c:chart r:id="rId2" xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"/>
        </a:graphicData>
      </a:graphic>
    </p:graphicFrame>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("chart-placeholder"),
        "Should render chart placeholder"
    );
    assert!(html.contains("Chart"), "Should show Chart label");
}

#[test]
fn test_chart_uses_preview_image_fallback_when_chart_part_exposes_image() {
    let pptx = build_chart_preview_fallback_pptx();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            assert!(
                chart.direct_spec.is_none(),
                "Unsupported radar chart fixture should stay off the direct renderer path"
            );
            assert!(
                chart
                    .preview_image
                    .as_ref()
                    .is_some_and(|bytes| !bytes.is_empty()),
                "Chart preview image bytes should be captured from chart part relationships"
            );
            assert_eq!(chart.preview_mime.as_deref(), Some("image/png"));
        }
        _ => panic!("Expected Chart shape type"),
    }

    let html = render_html(&pptx);
    assert!(
        html.contains("<img class=\"shape-image\""),
        "Chart preview fallback should render as an image when available: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "Chart preview fallback should replace the generic placeholder when preview bytes exist: {html}"
    );
}

#[test]
fn test_chart_parses_direct_column_spec() {
    let pptx = build_column_chart_pptx();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::Column);
            assert_eq!(spec.grouping, ChartGrouping::Clustered);
            assert_eq!(spec.series.len(), 1);
            assert_eq!(spec.series[0].name.as_deref(), Some("Revenue"));
            assert_eq!(spec.series[0].categories, vec!["Q1", "Q2", "Q3"]);
            assert_eq!(spec.series[0].values, vec![10.0, 20.0, 30.0]);
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_stacked_chart_parses_grouping() {
    let pptx = build_stacked_chart_pptx("col", "stacked", 2);
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::Column);
            assert_eq!(spec.grouping, ChartGrouping::Stacked);
            assert_eq!(spec.series.len(), 2);
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_chart_renders_direct_column_chart() {
    let pptx = build_column_chart_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("chart-direct"),
        "Should render direct chart container: {html}"
    );
    assert!(
        html.contains("Revenue"),
        "Should render series label: {html}"
    );
    assert!(
        html.contains("chart-bar"),
        "Should render bar elements: {html}"
    );
    assert!(
        html.contains(">Q1<") && html.contains(">Q2<") && html.contains(">Q3<"),
        "Should render category labels: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "Direct-renderable chart should not fall back to placeholder markup: {html}"
    );
}

#[test]
fn test_chart_renders_direct_bar_chart_horizontally() {
    let pptx = build_bar_chart_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("chart-direct"),
        "Should render direct chart container: {html}"
    );
    assert!(
        html.contains("chart-bar-horizontal"),
        "Horizontal bar charts should emit horizontal bar class: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "Renderable bar chart should not use placeholder: {html}"
    );
}

#[test]
fn test_chart_parses_gap_width_and_overlap() {
    let pptx = build_chart_spacing_pptx("col", 2, 50, 100);
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.gap_width, Some(50));
            assert_eq!(spec.overlap, Some(100));
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_bar_chart_parses_value_label_flag() {
    let pptx = build_chart_with_value_labels_pptx("col", true, false);
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            let labels = spec.data_labels.as_ref().expect("data label settings");
            assert!(labels.show_value);
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_column_chart_renders_value_labels() {
    let pptx = build_chart_with_value_labels_pptx("col", true, false);
    let html = render_html(&pptx);

    assert!(
        html.contains("<text class=\"chart-data-label\""),
        "Column chart should render value labels: {html}"
    );
    assert!(
        html.contains(">10<"),
        "Column chart value label should include the first value: {html}"
    );
}

#[test]
fn test_horizontal_bar_chart_renders_value_labels() {
    let pptx = build_chart_with_value_labels_pptx("bar", true, false);
    let html = render_html(&pptx);

    assert!(
        html.contains("<text class=\"chart-data-label\""),
        "Horizontal bar chart should render value labels: {html}"
    );
    assert!(
        html.contains(">20<"),
        "Horizontal bar chart value label should include bar values: {html}"
    );
}

#[test]
fn test_column_chart_renders_category_and_value_labels() {
    let pptx = build_chart_with_value_labels_pptx("col", true, true);
    let html = render_html(&pptx);

    assert!(
        html.contains(">Q1: 10<"),
        "Column chart should combine category and value label text when showCatName and showVal are enabled: {html}"
    );
}

#[test]
fn test_horizontal_bar_chart_renders_category_and_value_labels() {
    let pptx = build_chart_with_value_labels_pptx("bar", true, true);
    let html = render_html(&pptx);

    assert!(
        html.contains(">Q2: 20<"),
        "Horizontal bar chart should combine category and value label text when showCatName and showVal are enabled: {html}"
    );
}

#[test]
fn test_bar_chart_parses_label_position() {
    let pptx = build_chart_with_label_flags_pptx("col", true, false, false, Some("ctr"));
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            let labels = spec.data_labels.as_ref().expect("data label settings");
            assert_eq!(labels.position, Some(ChartDataLabelPosition::Center));
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_column_chart_renders_centered_value_labels() {
    let pptx = build_chart_with_label_flags_pptx("col", true, false, false, Some("ctr"));
    let html = render_html(&pptx);

    assert!(
        html.contains("data-label-position=\"ctr\""),
        "Centered column labels should expose ctr label position: {html}"
    );
    assert!(
        html.contains("y=\"220.5\">10</text>"),
        "Centered column label should move into the bar center: {html}"
    );
}

#[test]
fn test_horizontal_bar_chart_renders_explicit_out_end_labels() {
    let pptx = build_chart_with_label_flags_pptx("bar", true, false, false, Some("outEnd"));
    let html = render_html(&pptx);

    assert!(
        html.contains("data-label-position=\"outEnd\""),
        "Explicit outEnd bar labels should expose outEnd label position: {html}"
    );
}

#[test]
fn test_bar_chart_parses_series_name_label_flag() {
    let pptx = build_multi_series_chart_with_label_flags_pptx("col", true, true);
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            let labels = spec.data_labels.as_ref().expect("data label settings");
            assert!(labels.show_series_name);
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_multi_series_column_chart_renders_series_name_and_value_labels() {
    let pptx = build_multi_series_chart_with_label_flags_pptx("col", true, true);
    let html = render_html(&pptx);

    assert!(
        html.contains(">Revenue: 10<"),
        "Column chart should include series name in direct data labels: {html}"
    );
    assert!(
        html.contains(">Profit: 5<"),
        "Column chart should include each series name in direct data labels: {html}"
    );
}

#[test]
fn test_multi_series_bar_chart_renders_series_name_and_value_labels() {
    let pptx = build_multi_series_chart_with_label_flags_pptx("bar", true, true);
    let html = render_html(&pptx);

    assert!(
        html.contains(">Revenue: 20<") || html.contains(">Revenue: 10<"),
        "Bar chart should include series name in direct data labels: {html}"
    );
    assert!(
        html.contains(">Profit: 7<") || html.contains(">Profit: 5<"),
        "Bar chart should include each series name in direct data labels: {html}"
    );
}

#[test]
fn test_column_chart_renders_in_end_value_labels() {
    let pptx = build_chart_with_label_flags_pptx("col", true, false, false, Some("inEnd"));
    let html = render_html(&pptx);

    assert!(
        html.contains("data-label-position=\"inEnd\""),
        "In-end column labels should expose inEnd label position: {html}"
    );
    assert!(
        html.contains("y=\"190.0\">10</text>"),
        "In-end column label should move inside the bar near its end: {html}"
    );
}

#[test]
fn test_horizontal_bar_chart_renders_in_end_value_labels() {
    let pptx = build_chart_with_label_flags_pptx("bar", true, false, false, Some("inEnd"));
    let html = render_html(&pptx);

    assert!(
        html.contains("data-label-position=\"inEnd\""),
        "In-end horizontal bar labels should expose inEnd label position: {html}"
    );
    assert!(
        html.contains("x=\"162.3\"") || html.contains("x=\"162.2\""),
        "In-end horizontal bar label should move inside the bar near its end: {html}"
    );
}

#[test]
fn test_chart_parses_axis_titles() {
    let pptx = build_chart_with_axis_titles_pptx();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.category_axis_title.as_deref(), Some("Quarter"));
            assert_eq!(spec.value_axis_title.as_deref(), Some("Revenue ($M)"));
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_chart_renders_axis_titles() {
    let pptx = build_chart_with_axis_titles_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("chart-axis-title-x"),
        "X axis title should render with dedicated class: {html}"
    );
    assert!(
        html.contains("chart-axis-title-y"),
        "Y axis title should render with dedicated class: {html}"
    );
    assert!(
        html.contains("Quarter"),
        "X axis title text should appear in HTML: {html}"
    );
    assert!(
        html.contains("Revenue ($M)"),
        "Y axis title text should appear in HTML: {html}"
    );
}

#[test]
fn test_chart_renders_gap_width_and_overlap_attributes() {
    let pptx = build_chart_spacing_pptx("col", 2, 50, 100);
    let html = render_html(&pptx);

    assert!(
        html.contains("data-chart-gap-width=\"50\""),
        "Custom gap width should be exposed in chart markup: {html}"
    );
    assert!(
        html.contains("data-chart-overlap=\"100\""),
        "Custom overlap should be exposed in chart markup: {html}"
    );
}

#[test]
fn test_multi_series_column_chart_renders_grouped_directly() {
    let pptx = build_multi_series_column_chart_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "Multi-series chart should render directly once grouped rendering is supported: {html}"
    );
    assert!(
        html.contains("chart-bar"),
        "Multi-series column chart should render bar elements: {html}"
    );
    assert!(
        html.contains("Revenue") && html.contains("Profit"),
        "Multi-series legend/labels should include both series names: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "Multi-series chart should not fall back once grouped rendering is supported: {html}"
    );
}

#[test]
fn test_multi_series_bar_chart_renders_grouped_horizontal_bars() {
    let pptx = build_chart_pptx("bar", 2);
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "Multi-series bar chart should render directly: {html}"
    );
    assert!(
        html.contains("chart-bar-horizontal"),
        "Multi-series bar chart should render horizontal bars: {html}"
    );
    assert!(
        html.contains("Revenue") && html.contains("Profit"),
        "Multi-series legend/labels should include both series names: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "Multi-series bar chart should not use placeholder once grouped rendering is supported: {html}"
    );
}

#[test]
fn test_line_chart_parses_direct_spec() {
    let pptx = build_line_chart_pptx(1);
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::Line);
            assert_eq!(spec.series.len(), 1);
            assert_eq!(spec.series[0].name.as_deref(), Some("Revenue"));
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_line_chart_renders_directly() {
    let pptx = build_line_chart_pptx(1);
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "Line chart should render directly: {html}"
    );
    assert!(
        html.contains("chart-line"),
        "Line chart should render a line path/polyline: {html}"
    );
    assert!(
        html.contains("chart-point"),
        "Line chart should render point markers: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "Line chart should not use placeholder: {html}"
    );
}

#[test]
fn test_line_chart_parses_marker_spec() {
    let pptx = build_line_chart_with_marker_pptx("diamond", Some(9));
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            let marker = spec.series[0].marker.as_ref().expect("marker spec");
            assert_eq!(marker.symbol.as_deref(), Some("diamond"));
            assert_eq!(marker.size, Some(9));
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_line_chart_parses_value_label_flag() {
    let pptx = build_line_chart_with_value_labels_pptx();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            let labels = spec.data_labels.as_ref().expect("data label settings");
            assert!(labels.show_value);
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_line_chart_renders_value_labels() {
    let pptx = build_line_chart_with_value_labels_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("<text class=\"chart-data-label\""),
        "Line chart should render value labels: {html}"
    );
    assert!(
        html.contains(">20<"),
        "Line chart should render point values as labels: {html}"
    );
}

#[test]
fn test_area_chart_renders_value_labels() {
    let pptx = build_area_chart_with_value_labels_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("<text class=\"chart-data-label\""),
        "Area chart should render value labels: {html}"
    );
    assert!(
        html.contains(">30<"),
        "Area chart should render point values as labels: {html}"
    );
}

#[test]
fn test_line_chart_renders_category_and_value_labels() {
    let pptx = build_line_chart_with_label_flags_pptx(true, true, false, None);
    let html = render_html(&pptx);

    assert!(
        html.contains(">Q2: 20<"),
        "Line chart should combine category and value label text when showCatName and showVal are enabled: {html}"
    );
}

#[test]
fn test_area_chart_renders_series_and_value_labels() {
    let pptx = build_area_chart_with_label_flags_pptx(true, false, true, None);
    let html = render_html(&pptx);

    assert!(
        html.contains(">Revenue: 30<"),
        "Area chart should combine series name and value label text when showSerName and showVal are enabled: {html}"
    );
}

#[test]
fn test_line_chart_renders_centered_value_labels() {
    let pptx = build_line_chart_with_label_flags_pptx(true, false, false, Some("ctr"));
    let html = render_html(&pptx);

    assert!(
        html.contains("data-label-position=\"ctr\""),
        "Centered line labels should expose ctr label position: {html}"
    );
    assert!(
        html.contains("y=\"93.0\">20</text>"),
        "Centered line label should sit on the point center: {html}"
    );
}

#[test]
fn test_line_chart_renders_in_end_value_labels() {
    let pptx = build_line_chart_with_label_flags_pptx(true, false, false, Some("inEnd"));
    let html = render_html(&pptx);

    assert!(
        html.contains("data-label-position=\"inEnd\""),
        "Line inEnd labels should expose inEnd label position: {html}"
    );
    assert!(
        html.contains("y=\"103.0\">20</text>"),
        "Line inEnd label should move below the point: {html}"
    );
}

#[test]
fn test_area_chart_renders_explicit_out_end_labels() {
    let pptx = build_area_chart_with_label_flags_pptx(true, false, false, Some("outEnd"));
    let html = render_html(&pptx);

    assert!(
        html.contains("data-label-position=\"outEnd\""),
        "Area outEnd labels should expose outEnd label position: {html}"
    );
}

#[test]
fn test_area_chart_renders_in_end_value_labels() {
    let pptx = build_area_chart_with_label_flags_pptx(true, false, false, Some("inEnd"));
    let html = render_html(&pptx);

    assert!(
        html.contains("data-label-position=\"inEnd\""),
        "Area inEnd labels should expose inEnd label position: {html}"
    );
    assert!(
        html.contains("y=\"103.0\">20</text>"),
        "Area inEnd label should move below the point: {html}"
    );
}

#[test]
fn test_scatter_chart_parses_direct_spec() {
    let pptx = build_scatter_chart_pptx();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::Scatter);
            assert_eq!(spec.series.len(), 1);
            assert_eq!(spec.series[0].x_values, vec![1.0, 2.0, 3.0]);
            assert_eq!(spec.series[0].values, vec![10.0, 20.0, 15.0]);
            let marker = spec.series[0].marker.as_ref().expect("marker spec");
            assert_eq!(marker.symbol.as_deref(), Some("diamond"));
            assert_eq!(marker.size, Some(9));
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_scatter_chart_renders_directly() {
    let pptx = build_scatter_chart_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "Scatter chart should render directly: {html}"
    );
    assert!(
        html.contains("<circle class=\"chart-point\""),
        "Scatter chart should render points: {html}"
    );
    assert!(
        !html.contains("<polyline class=\"chart-line\""),
        "Scatter chart should not render a line path in marker-only mode: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "Scatter chart should not use placeholder: {html}"
    );
}

#[test]
fn test_scatter_chart_parses_value_label_flag() {
    let pptx = build_scatter_chart_with_flags_pptx("marker", true, false, false);
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            let labels = spec.data_labels.as_ref().expect("data label settings");
            assert!(labels.show_value);
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_scatter_chart_renders_value_labels() {
    let pptx = build_scatter_chart_with_flags_pptx("marker", true, false, false);
    let html = render_html(&pptx);

    assert!(
        html.contains("<text class=\"chart-data-label\""),
        "Scatter chart should render value labels: {html}"
    );
    assert!(
        html.contains(">20<"),
        "Scatter chart should render point values as labels: {html}"
    );
}

#[test]
fn test_scatter_chart_renders_category_and_value_labels() {
    let pptx = build_scatter_chart_with_flags_pptx("marker", true, true, false);
    let html = render_html(&pptx);

    assert!(
        html.contains(">2: 20<"),
        "Scatter chart should combine x-value category text and point value labels: {html}"
    );
}

#[test]
fn test_scatter_chart_renders_series_and_value_labels() {
    let pptx = build_scatter_chart_with_flags_pptx("marker", true, false, true);
    let html = render_html(&pptx);

    assert!(
        html.contains(">Revenue: 20<"),
        "Scatter chart should combine series name and value labels: {html}"
    );
}

#[test]
fn test_scatter_chart_renders_centered_value_labels() {
    let pptx = build_scatter_chart_with_label_flags_pptx("marker", true, false, false, Some("ctr"));
    let html = render_html(&pptx);

    assert!(
        html.contains("data-label-position=\"ctr\""),
        "Centered scatter labels should expose ctr label position: {html}"
    );
    assert!(
        html.contains("y=\"8.0\">20</text>"),
        "Centered scatter label should sit on the point center: {html}"
    );
}

#[test]
fn test_scatter_chart_renders_explicit_out_end_labels() {
    let pptx =
        build_scatter_chart_with_label_flags_pptx("marker", true, false, false, Some("outEnd"));
    let html = render_html(&pptx);

    assert!(
        html.contains("data-label-position=\"outEnd\""),
        "Scatter outEnd labels should expose outEnd label position: {html}"
    );
}

#[test]
fn test_scatter_chart_renders_in_end_value_labels() {
    let pptx =
        build_scatter_chart_with_label_flags_pptx("marker", true, false, false, Some("inEnd"));
    let html = render_html(&pptx);

    assert!(
        html.contains("data-label-position=\"inEnd\""),
        "Scatter inEnd labels should expose inEnd label position: {html}"
    );
    assert!(
        html.contains("y=\"18.0\">20</text>"),
        "Scatter inEnd label should move below the point: {html}"
    );
}

#[test]
fn test_scatter_chart_parses_scatter_style() {
    let pptx = build_scatter_chart_with_style_pptx("lineMarker");
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.scatter_style, Some(ChartScatterStyle::LineMarker));
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_scatter_line_marker_renders_line_and_points() {
    let pptx = build_scatter_chart_with_style_pptx("lineMarker");
    let html = render_html(&pptx);

    assert!(
        html.contains("<polyline class=\"chart-line\""),
        "Scatter lineMarker should render a line path: {html}"
    );
    assert!(
        html.contains("<circle class=\"chart-point\""),
        "Scatter lineMarker should render points: {html}"
    );
}

#[test]
fn test_scatter_line_style_suppresses_points() {
    let pptx = build_scatter_chart_with_style_pptx("line");
    let html = render_html(&pptx);

    assert!(
        html.contains("<polyline class=\"chart-line\""),
        "Scatter line style should render a line path: {html}"
    );
    assert!(
        !html.contains("<circle class=\"chart-point\""),
        "Scatter line style should suppress point markers: {html}"
    );
}

#[test]
fn test_area_chart_parses_direct_spec() {
    let pptx = build_area_chart_pptx();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::Area);
            assert_eq!(spec.grouping, ChartGrouping::Standard);
            assert_eq!(spec.series.len(), 1);
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_area3d_chart_parses_direct_spec() {
    let pptx = build_area3d_chart_pptx();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::Area);
            assert_eq!(spec.grouping, ChartGrouping::Standard);
            assert_eq!(spec.series.len(), 1);
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_bubble_chart_parses_direct_spec() {
    let pptx = build_bubble_chart_pptx();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::Bubble);
            assert_eq!(spec.series.len(), 1);
            assert_eq!(spec.series[0].x_values, vec![10.0, 20.0, 35.0]);
            assert_eq!(spec.series[0].values, vec![15.0, 28.0, 12.0]);
            assert_eq!(spec.series[0].bubble_sizes, vec![6.0, 14.0, 10.0]);
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_bubble_chart_parses_percent_bubble_scale() {
    let pptx = build_bubble_chart_with_scale_pptx("200%");
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::Bubble);
            assert_eq!(spec.bubble_scale, Some(200.0));
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_bubble_chart_parses_show_neg_bubbles_true() {
    let pptx = build_bubble_chart_with_show_neg_bubbles_pptx("1");
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::Bubble);
            assert_eq!(spec.show_neg_bubbles, Some(true));
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_bubble_chart_parses_show_neg_bubbles_true_literal() {
    let pptx = build_bubble_chart_with_show_neg_bubbles_pptx("true");
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::Bubble);
            assert_eq!(spec.show_neg_bubbles, Some(true));
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_bubble_chart_parses_show_neg_bubbles_false() {
    let pptx = build_bubble_chart_with_show_neg_bubbles_pptx("0");
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::Bubble);
            assert_eq!(spec.show_neg_bubbles, Some(false));
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_bubble_chart_parses_show_neg_bubbles_false_literal() {
    let pptx = build_bubble_chart_with_show_neg_bubbles_pptx("false");
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::Bubble);
            assert_eq!(spec.show_neg_bubbles, Some(false));
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_bubble_chart_clamps_bubble_scale_to_upper_bound() {
    let pptx = build_bubble_chart_with_scale_pptx("500");
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::Bubble);
            assert_eq!(spec.bubble_scale, Some(300.0));
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_bubble_chart_renders_directly() {
    let pptx = build_bubble_chart_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "Bubble chart should render directly once bubble support is available: {html}"
    );
    assert!(
        html.contains("class=\"chart-bubble\""),
        "Bubble chart should emit bubble circles rather than fallback markers: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "Bubble chart should not use the generic placeholder once supported: {html}"
    );
}

#[test]
fn test_positive_bubble_chart_with_show_neg_bubbles_true_still_renders_directly() {
    let html = render_html(&build_bubble_chart_with_show_neg_bubbles_pptx("1"));

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "showNegBubbles on positive-only bubble data should not disable direct rendering: {html}"
    );
    assert!(
        html.contains("class=\"chart-bubble\""),
        "showNegBubbles on positive-only bubble data should still render bubbles directly: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "showNegBubbles on positive-only bubble data should not force fallback: {html}"
    );
}

#[test]
fn test_negative_bubble_chart_with_show_neg_bubbles_true_still_falls_back() {
    let html = render_html(&build_bubble_chart_semantics_pptx(
        1,
        false,
        Some("100"),
        Some("area"),
        Some("1"),
        Some(&[[-6.0, 14.0, 10.0]]),
    ));

    assert!(
        html.contains("<div class=\"chart-placeholder\">"),
        "showNegBubbles should not bypass the current negative-size fallback contract: {html}"
    );
    assert!(
        !html.contains("class=\"chart-bubble\""),
        "negative-size bubbles should remain off the direct renderer even when showNegBubbles is true: {html}"
    );
}

#[test]
fn test_bubble_scale_changes_rendered_radius() {
    let default_html = render_html(&build_bubble_chart_with_scale_pptx("100"));
    let scaled_html = render_html(&build_bubble_chart_with_scale_pptx("200"));

    assert!(
        default_html.contains("class=\"chart-bubble\""),
        "Default-scale bubble chart should render directly: {default_html}"
    );
    assert!(
        scaled_html.contains("class=\"chart-bubble\""),
        "Scaled bubble chart should render directly: {scaled_html}"
    );
    assert!(
        default_html.contains("r=\"10.0\""),
        "Default bubble scale should preserve the baseline radius for the smallest bubble: {default_html}"
    );
    assert!(
        scaled_html.contains("r=\"20.0\""),
        "bubbleScale=200 should enlarge the same bubble radius relative to the baseline chart: {scaled_html}"
    );
}

#[test]
fn test_bubble_chart_with_width_semantics_falls_back_to_placeholder() {
    let pptx = build_bubble_chart_with_width_semantics_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-placeholder\">"),
        "Width-semantics bubble charts should stay on fallback until width-based sizing is implemented: {html}"
    );
    assert!(
        !html.contains("class=\"chart-bubble\""),
        "Width-semantics bubble charts should not partially direct render yet: {html}"
    );
}

#[test]
fn test_bubble_chart_with_negative_sizes_falls_back_to_placeholder() {
    let pptx = build_bubble_chart_with_negative_sizes_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-placeholder\">"),
        "Negative-size bubble charts should stay on fallback until negative-bubble semantics are implemented: {html}"
    );
    assert!(
        !html.contains("class=\"chart-bubble\""),
        "Negative-size bubble charts should not partially direct render yet: {html}"
    );
}

#[test]
fn test_multi_series_bubble_chart_falls_back_to_placeholder() {
    let pptx = build_multi_series_bubble_chart_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-placeholder\">"),
        "Multi-series bubble should stay on the safe fallback path for now: {html}"
    );
    assert!(
        !html.contains("class=\"chart-bubble\""),
        "Multi-series bubble should not partially direct render yet: {html}"
    );
}

#[test]
fn test_bubble_chart_with_value_labels_falls_back_to_placeholder() {
    let pptx = build_bubble_chart_with_value_labels_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-placeholder\">"),
        "Bubble charts with data labels should stay on fallback until label rendering is implemented: {html}"
    );
    assert!(
        !html.contains("class=\"chart-bubble\""),
        "Bubble charts with data labels should not partially direct render yet: {html}"
    );
}

#[test]
fn test_area_chart_renders_directly() {
    let pptx = build_area_chart_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "Area chart should render directly: {html}"
    );
    assert!(
        html.contains("chart-area"),
        "Area chart should render a filled area path: {html}"
    );
    assert!(
        html.contains("chart-line"),
        "Area chart should render an outline line path: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "Area chart should not use placeholder: {html}"
    );
}

#[test]
fn test_area3d_chart_renders_directly_as_flat_area() {
    let pptx = build_area3d_chart_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "3D area chart should reuse the direct chart renderer when the series shape matches a flat area: {html}"
    );
    assert!(
        html.contains("chart-area"),
        "3D area chart should emit a flat area polygon through the existing area renderer: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "3D area chart should not use placeholder once flattening is supported: {html}"
    );
}

#[test]
fn test_stacked_area3d_chart_falls_back_to_placeholder() {
    let pptx = build_area3d_chart_with_grouping_pptx("stacked");
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-placeholder\">"),
        "Stacked area3D should stay on fallback until stacked flattening is explicitly supported: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-direct\">"),
        "Stacked area3D should not enter the direct chart path yet: {html}"
    );
}

#[test]
fn test_radar_chart_renders_directly() {
    let pptx = build_radar_chart_pptx("standard");
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "Radar chart should render directly once radar support is available: {html}"
    );
    assert!(
        html.contains("chart-radar-line"),
        "Radar chart should emit a radar line path: {html}"
    );
    assert!(
        html.contains("Q1") && html.contains("Q2") && html.contains("Q3") && html.contains("Q4"),
        "Radar chart should render category labels: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "Radar chart should no longer use the generic placeholder: {html}"
    );
}

#[test]
fn test_radar_chart_parses_marker_spec() {
    let pptx = build_radar_chart_with_marker_pptx("diamond", Some(12));
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::Radar);
            let marker = spec.series[0].marker.as_ref().expect("marker spec");
            assert_eq!(marker.symbol.as_deref(), Some("diamond"));
            assert_eq!(marker.size, Some(12));
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_radar_marker_style_uses_series_marker_symbol_and_size() {
    let pptx = build_radar_chart_with_marker_pptx("diamond", Some(12));
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "Marker-style radar should still render directly: {html}"
    );
    assert!(
        html.contains("data-marker-symbol=\"diamond\""),
        "Radar marker style should expose the parsed marker symbol on rendered points: {html}"
    );
    assert!(
        html.contains("r=\"6.0\""),
        "Radar marker style should scale point radius from the parsed marker size: {html}"
    );
}

#[test]
fn test_radar_marker_symbol_none_suppresses_points() {
    let pptx = build_radar_chart_with_marker_pptx("none", None);
    let html = render_html(&pptx);

    assert!(
        html.contains("chart-radar-line"),
        "Radar marker style with symbol none should still render the radar path: {html}"
    );
    assert!(
        !html.contains("<circle class=\"chart-point\""),
        "Radar marker symbol none should suppress point rendering: {html}"
    );
}

#[test]
fn test_radar_filled_style_renders_fill_without_markers() {
    let pptx = build_radar_chart_pptx("filled");
    let html = render_html(&pptx);

    assert!(
        html.contains("chart-radar-fill"),
        "Filled radar should emit a filled radar polygon: {html}"
    );
    assert!(
        !html.contains("<circle class=\"chart-point\""),
        "Filled radar should not render marker points: {html}"
    );
}

#[test]
fn test_multi_series_radar_chart_renders_directly() {
    let pptx = build_multi_series_radar_chart_pptx("standard");
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "Multi-series radar should render directly once bounded multi-series radar support is enabled: {html}"
    );
    assert!(
        html.matches("chart-radar-line").count() >= 2,
        "Multi-series radar should emit one radar line per series: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "Multi-series radar should no longer use the generic placeholder when the bounded multi-series slice is supported: {html}"
    );
}

#[test]
fn test_multi_series_radar_chart_with_value_labels_falls_back_to_placeholder() {
    let pptx = build_radar_chart_variant_pptx("standard", 2, true, None, None);
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-placeholder\">"),
        "Multi-series radar with data labels should stay on fallback until bounded radar label support is implemented: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-direct\">"),
        "Multi-series radar with data labels should not enter the direct chart path yet: {html}"
    );
}

#[test]
fn test_multi_series_radar_marker_style_renders_markers_for_each_series() {
    let pptx = build_radar_chart_variant_pptx("marker", 2, false, Some("diamond"), Some(12));
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "Multi-series marker radar should render directly: {html}"
    );
    assert!(
        html.matches("chart-radar-line").count() >= 2,
        "Multi-series marker radar should emit one radar line per series: {html}"
    );
    assert!(
        html.matches("data-marker-symbol=\"diamond\"").count() >= 8,
        "Multi-series marker radar should render markers for each point in each series: {html}"
    );
}

#[test]
fn test_multi_series_radar_filled_style_renders_fill_for_each_series() {
    let pptx = build_radar_chart_variant_pptx("filled", 2, false, None, None);
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "Multi-series filled radar should render directly: {html}"
    );
    assert!(
        html.matches("chart-radar-fill").count() >= 2,
        "Multi-series filled radar should emit one fill polygon per series: {html}"
    );
    assert!(
        !html.contains("<circle class=\"chart-point\""),
        "Filled multi-series radar should not render marker points: {html}"
    );
}

#[test]
fn test_radar_chart_with_value_labels_falls_back_to_placeholder() {
    let pptx = build_radar_chart_with_value_labels_pptx("standard");
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-placeholder\">"),
        "Radar charts with data labels should stay on fallback until label rendering is implemented: {html}"
    );
    assert!(
        !html.contains("chart-radar-line"),
        "Radar charts with data labels should not partially direct render yet: {html}"
    );
}

#[test]
fn test_line_chart_marker_none_suppresses_points() {
    let pptx = build_line_chart_with_marker_pptx("none", None);
    let html = render_html(&pptx);

    assert!(
        html.contains("chart-line"),
        "Line chart should still render the line path: {html}"
    );
    assert!(
        !html.contains("<circle class=\"chart-point\""),
        "Marker symbol none should suppress point rendering: {html}"
    );
}

#[test]
fn test_multi_series_line_chart_renders_directly() {
    let pptx = build_line_chart_pptx(2);
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "Multi-series line chart should render directly: {html}"
    );
    assert!(
        html.contains("Revenue") && html.contains("Profit"),
        "Multi-series line chart should render legend labels: {html}"
    );
    assert!(
        html.matches("chart-line").count() >= 2,
        "Multi-series line chart should render one line per series: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "Multi-series line chart should not use placeholder: {html}"
    );
}

#[test]
fn test_stacked_column_chart_renders_directly() {
    let pptx = build_stacked_chart_pptx("col", "stacked", 2);
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "Stacked column chart should render directly: {html}"
    );
    assert!(
        html.contains("Revenue") && html.contains("Profit"),
        "Stacked chart should render legend labels: {html}"
    );
    assert!(
        html.contains("chart-bar-stacked"),
        "Stacked column chart should emit stacked bar segments: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "Stacked column chart should not use placeholder: {html}"
    );
}

#[test]
fn test_percent_stacked_column_chart_normalizes_to_full_height() {
    let pptx = build_stacked_chart_pptx("col", "percentStacked", 2);
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "100% stacked chart should render directly: {html}"
    );
    assert!(
        html.contains("chart-bar-stacked"),
        "100% stacked chart should render stacked bars: {html}"
    );
    assert!(
        html.contains("data-chart-grouping=\"percent-stacked\""),
        "100% stacked chart should expose percent-stacked grouping marker: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "100% stacked chart should not use placeholder: {html}"
    );
}

#[test]
fn test_stacked_bar_chart_renders_directly() {
    let pptx = build_stacked_chart_pptx("bar", "stacked", 2);
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "Stacked bar chart should render directly: {html}"
    );
    assert!(
        html.contains("Revenue") && html.contains("Profit"),
        "Stacked bar chart should render legend labels: {html}"
    );
    assert!(
        html.contains("chart-bar-horizontal"),
        "Stacked bar chart should emit horizontal bar segments: {html}"
    );
    assert!(
        html.contains("data-chart-grouping=\"stacked\""),
        "Stacked bar chart should expose stacked grouping marker: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "Stacked bar chart should not use placeholder: {html}"
    );
}

#[test]
fn test_percent_stacked_bar_chart_normalizes_to_full_width() {
    let pptx = build_stacked_chart_pptx("bar", "percentStacked", 2);
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "100% stacked bar chart should render directly: {html}"
    );
    assert!(
        html.contains("chart-bar-horizontal"),
        "100% stacked bar chart should render horizontal bar segments: {html}"
    );
    assert!(
        html.contains("data-chart-grouping=\"percent-stacked\""),
        "100% stacked bar chart should expose percent-stacked grouping marker: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "100% stacked bar chart should not use placeholder: {html}"
    );
}

#[test]
fn test_stacked_column_chart_renders_value_labels() {
    let pptx = build_stacked_chart_with_value_labels_pptx("col", "stacked", 2, true);
    let html = render_html(&pptx);

    assert!(
        html.contains("<text class=\"chart-data-label\""),
        "Stacked column chart should render value labels: {html}"
    );
    assert!(
        html.contains(">15<") || html.contains(">20<"),
        "Stacked column chart should expose stacked segment values as labels: {html}"
    );
}

#[test]
fn test_percent_stacked_bar_chart_renders_value_labels() {
    let pptx = build_stacked_chart_with_value_labels_pptx("bar", "percentStacked", 2, true);
    let html = render_html(&pptx);

    assert!(
        html.contains("<text class=\"chart-data-label\""),
        "Percent-stacked bar chart should render value labels: {html}"
    );
    assert!(
        html.contains(">5<") || html.contains(">10<"),
        "Percent-stacked bar chart should keep raw point values in labels: {html}"
    );
}

#[test]
fn test_percent_stacked_column_renders_percent_labels() {
    let pptx =
        build_stacked_chart_with_label_flags_pptx("col", "percentStacked", 2, false, false, true);
    let html = render_html(&pptx);

    assert!(
        html.contains("<text class=\"chart-data-label\""),
        "Percent-stacked column chart should render percent labels: {html}"
    );
    assert!(
        html.contains("67%") || html.contains("57%"),
        "Percent-stacked column chart should render rounded percentage labels: {html}"
    );
    assert!(
        !html.contains(">10<"),
        "Percent-only labels should not render raw values: {html}"
    );
}

#[test]
fn test_percent_stacked_bar_renders_category_and_percent_labels() {
    let pptx =
        build_stacked_chart_with_label_flags_pptx("bar", "percentStacked", 2, false, true, true);
    let html = render_html(&pptx);

    assert!(
        html.contains("Q1: 67%") || html.contains("Q2: 57%"),
        "Percent-stacked bar chart should combine category and percent labels: {html}"
    );
}

#[test]
fn test_pie_chart_parses_direct_spec() {
    let pptx = build_pie_chart_pptx();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::Pie);
            assert_eq!(spec.series.len(), 1);
            assert_eq!(spec.series[0].name.as_deref(), Some("Revenue Share"));
            assert_eq!(spec.series[0].categories, vec!["North", "South", "West"]);
            assert_eq!(spec.series[0].values, vec![30.0, 20.0, 50.0]);
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_doughnut_chart_parses_direct_spec() {
    let pptx = build_doughnut_chart_pptx();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::Doughnut);
            assert_eq!(spec.hole_size, Some(60));
            assert_eq!(spec.series.len(), 1);
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_pie_chart_parses_data_label_flags() {
    let pptx = build_pie_chart_with_dlabels_pptx(true, true, false, None);
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            let labels = spec.data_labels.as_ref().expect("data label settings");
            assert!(labels.show_value);
            assert!(labels.show_category_name);
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_pie_chart_renders_value_and_category_labels() {
    let pptx = build_pie_chart_with_dlabels_pptx(true, true, false, None);
    let html = render_html(&pptx);

    assert!(
        html.contains("chart-data-label"),
        "Pie chart should render data labels when enabled: {html}"
    );
    assert!(
        html.contains("North: 30"),
        "Pie chart should render category and value label text: {html}"
    );
}

#[test]
fn test_doughnut_chart_renders_value_only_labels() {
    let pptx = build_doughnut_chart_with_dlabels_pptx(true, false, false, None);
    let html = render_html(&pptx);

    assert!(
        html.contains("chart-data-label"),
        "Doughnut chart should render data labels when enabled: {html}"
    );
    assert!(
        html.contains(">30<"),
        "Doughnut chart should render value-only label text: {html}"
    );
    assert!(
        !html.contains("North: 30"),
        "Value-only doughnut labels should not include category text: {html}"
    );
}

#[test]
fn test_pie_chart_parses_percent_data_label_flag() {
    let pptx = build_pie_chart_with_dlabels_pptx(false, false, true, None);
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            let labels = spec.data_labels.as_ref().expect("data label settings");
            assert!(labels.show_percent);
            assert!(!labels.show_value);
            assert!(!labels.show_category_name);
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_pie_chart_renders_percent_only_labels() {
    let pptx = build_pie_chart_with_dlabels_pptx(false, false, true, None);
    let html = render_html(&pptx);

    assert!(
        html.contains("30%"),
        "Pie chart should render percent-only label text: {html}"
    );
    assert!(
        !html.contains(">30<"),
        "Percent-only labels should not render raw values: {html}"
    );
    assert!(
        !html.contains("North: 30"),
        "Percent-only labels should not render category/value text: {html}"
    );
}

#[test]
fn test_doughnut_chart_renders_value_and_percent_labels() {
    let pptx = build_doughnut_chart_with_dlabels_pptx(true, false, true, None);
    let html = render_html(&pptx);

    assert!(
        html.contains("30: 30%"),
        "Doughnut labels should combine value and percent when both are enabled: {html}"
    );
}

#[test]
fn test_pie_chart_renders_centered_labels() {
    let pptx = build_pie_chart_with_dlabels_pptx(true, false, false, Some("ctr"));
    let html = render_html(&pptx);

    assert!(
        html.contains("data-label-position=\"ctr\""),
        "Centered pie labels should expose ctr label position: {html}"
    );
}

#[test]
fn test_doughnut_chart_renders_out_end_labels() {
    let pptx = build_doughnut_chart_with_dlabels_pptx(true, false, false, Some("outEnd"));
    let html = render_html(&pptx);

    assert!(
        html.contains("data-label-position=\"outEnd\""),
        "Doughnut outEnd labels should expose outEnd label position: {html}"
    );
}

#[test]
fn test_pie_chart_without_dlabels_does_not_render_data_labels() {
    let pptx = build_pie_chart_pptx();
    let html = render_html(&pptx);

    assert!(
        !html.contains("<text class=\"chart-data-label\""),
        "Pie chart without dLbls should not render data labels: {html}"
    );
}

#[test]
fn test_doughnut_chart_renders_directly() {
    let pptx = build_doughnut_chart_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "Doughnut chart should render directly: {html}"
    );
    assert!(
        html.contains("chart-pie-slice"),
        "Doughnut chart should render slice paths: {html}"
    );
    assert!(
        html.contains("data-chart-hole-size=\"60\""),
        "Doughnut chart should expose hole size metadata: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "Doughnut chart should not use placeholder: {html}"
    );
}

#[test]
fn test_pie_chart_renders_directly() {
    let pptx = build_pie_chart_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "Pie chart should render directly: {html}"
    );
    assert!(
        html.contains("chart-pie-slice"),
        "Pie chart should render slice paths: {html}"
    );
    assert!(
        html.contains("North") && html.contains("South") && html.contains("West"),
        "Pie chart should render category legend labels: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "Pie chart should not use placeholder: {html}"
    );
}

#[test]
fn test_pie_chart_renders_category_legend_items() {
    let pptx = build_pie_chart_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("chart-legend-item"),
        "Pie chart should render legend items: {html}"
    );
    assert!(
        html.matches("chart-legend-item").count() >= 3,
        "Pie chart should render one legend item per category: {html}"
    );
    assert!(
        html.contains("North") && html.contains("South") && html.contains("West"),
        "Pie legend should use category labels: {html}"
    );
}

#[test]
fn test_multi_series_pie_chart_falls_back_to_placeholder() {
    let pptx = build_multi_series_pie_chart_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-placeholder\">"),
        "Multi-series pie should stay on safe fallback path: {html}"
    );
    assert!(
        !html.contains("<path class=\"chart-pie-slice\""),
        "Multi-series pie should not partially direct-render slices: {html}"
    );
}

#[test]
fn test_pie3d_chart_renders_directly_as_flat_pie() {
    let pptx = build_pie3d_chart_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "3D pie chart should reuse the direct chart renderer when the series shape matches a flat pie: {html}"
    );
    assert!(
        html.contains("<path class=\"chart-pie-slice\""),
        "3D pie chart should emit pie slices through the existing flat pie renderer: {html}"
    );
    assert!(
        html.contains("North") && html.contains("South"),
        "3D pie chart should preserve category labels in the flat direct render path: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "3D pie chart should no longer fall back to the generic chart placeholder when a flat pie render is possible: {html}"
    );
}

#[test]
fn test_of_pie_chart_renders_directly() {
    let pptx = build_of_pie_chart_pptx();
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-direct\">"),
        "ofPie chart should render directly once bounded ofPie support is available: {html}"
    );
    assert!(
        html.contains("chart-of-pie-primary"),
        "ofPie chart should render a primary pie cluster: {html}"
    );
    assert!(
        html.contains("chart-of-pie-secondary"),
        "ofPie chart should render a secondary pie cluster: {html}"
    );
    assert!(
        html.contains("North") && html.contains("West"),
        "ofPie chart should preserve category labels across both clusters: {html}"
    );
    assert!(
        !html.contains("<div class=\"chart-placeholder\">"),
        "ofPie chart should not use the generic placeholder once the bounded slice is supported: {html}"
    );
}

#[test]
fn test_of_pie_chart_parses_direct_spec() {
    let pptx = build_of_pie_chart_pptx();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    match &shape.shape_type {
        ShapeType::Chart(chart) => {
            let spec = chart.direct_spec.as_ref().expect("direct chart spec");
            assert_eq!(spec.chart_type, ChartType::OfPie);
            assert_eq!(spec.of_pie_type, Some(ChartOfPieType::Pie));
            assert_eq!(spec.split_type, Some(ChartSplitType::Pos));
            assert_eq!(spec.split_pos, Some(2.0));
            assert_eq!(spec.second_pie_size, Some(70));
        }
        _ => panic!("Expected Chart shape type"),
    }
}

#[test]
fn test_of_pie_chart_with_bar_type_falls_back_to_placeholder() {
    let pptx = build_of_pie_chart_variant_pptx("bar", "pos", Some("2"), false);
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-placeholder\">"),
        "Bar-of-pie should stay on fallback until bar secondary rendering is implemented: {html}"
    );
    assert!(
        !html.contains("chart-of-pie-primary"),
        "Bar-of-pie should not partially direct render yet: {html}"
    );
}

#[test]
fn test_of_pie_chart_with_value_split_falls_back_to_placeholder() {
    let pptx = build_of_pie_chart_variant_pptx("pie", "val", Some("15"), false);
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-placeholder\">"),
        "Value-split ofPie should stay on fallback until value-based partitioning is implemented: {html}"
    );
    assert!(
        !html.contains("chart-of-pie-primary"),
        "Value-split ofPie should not partially direct render yet: {html}"
    );
}

#[test]
fn test_of_pie_chart_with_data_labels_falls_back_to_placeholder() {
    let pptx = build_of_pie_chart_variant_pptx("pie", "pos", Some("2"), true);
    let html = render_html(&pptx);

    assert!(
        html.contains("<div class=\"chart-placeholder\">"),
        "ofPie with data labels should stay on fallback until bounded label support is implemented: {html}"
    );
    assert!(
        !html.contains("chart-of-pie-primary"),
        "ofPie with data labels should not partially direct render yet: {html}"
    );
}

// ── Shape effect tests (outerShdw / glow) ──

#[test]
fn test_shape_outer_shadow_parsing() {
    // outerShdw: blurRad=50800 EMU (4pt), dist=38100 EMU (3pt), dir=2700000 (45deg)
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:solidFill><a:srgbClr val="4472C4"/></a:solidFill>
        <a:effectLst>
          <a:outerShdw blurRad="50800" dist="38100" dir="2700000">
            <a:srgbClr val="000000"><a:alpha val="40000"/></a:srgbClr>
          </a:outerShdw>
        </a:effectLst>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    let shadow = shape
        .effects
        .outer_shadow
        .as_ref()
        .expect("outer_shadow should be parsed");
    // blurRad: 50800 EMU / 12700 = 4pt
    assert!(
        (shadow.blur_radius - 4.0).abs() < 0.1,
        "blur_radius should be ~4pt, got {}",
        shadow.blur_radius
    );
    // dist: 38100 EMU / 12700 = 3pt
    assert!(
        (shadow.distance - 3.0).abs() < 0.1,
        "distance should be ~3pt, got {}",
        shadow.distance
    );
    // dir: 2700000 / 60000 = 45 deg
    assert!(
        (shadow.direction - 45.0).abs() < 0.1,
        "direction should be 45deg, got {}",
        shadow.direction
    );
}

#[test]
fn test_shape_outer_shadow_css_rendering() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:effectLst>
          <a:outerShdw blurRad="50800" dist="38100" dir="2700000">
            <a:srgbClr val="000000"/>
          </a:outerShdw>
        </a:effectLst>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("box-shadow:"),
        "HTML should contain box-shadow for outerShdw: {html}"
    );
}

#[test]
fn test_shape_glow_parsing() {
    // glow: rad=63500 EMU (5pt)
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:effectLst>
          <a:glow rad="63500">
            <a:srgbClr val="FFC000"><a:alpha val="60000"/></a:srgbClr>
          </a:glow>
        </a:effectLst>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    let glow = shape.effects.glow.as_ref().expect("glow should be parsed");
    // rad: 63500 EMU / 12700 = 5pt
    assert!(
        (glow.radius - 5.0).abs() < 0.1,
        "glow radius should be ~5pt, got {}",
        glow.radius
    );
}

#[test]
fn test_shape_glow_css_rendering() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:effectLst>
          <a:glow rad="63500">
            <a:srgbClr val="FFC000"/>
          </a:glow>
        </a:effectLst>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("box-shadow:"),
        "HTML should contain box-shadow for glow: {html}"
    );
    // Glow uses spread with zero offset
    assert!(
        html.contains("0 0"),
        "Glow box-shadow should have zero offsets: {html}"
    );
}

#[test]
fn test_shape_combined_shadow_and_glow() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:effectLst>
          <a:outerShdw blurRad="50800" dist="38100" dir="2700000">
            <a:srgbClr val="000000"/>
          </a:outerShdw>
          <a:glow rad="63500">
            <a:srgbClr val="FFC000"/>
          </a:glow>
        </a:effectLst>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    assert!(
        shape.effects.outer_shadow.is_some(),
        "outer_shadow should be parsed"
    );
    assert!(shape.effects.glow.is_some(), "glow should be parsed");

    let html = render_html(&pptx);
    // Combined box-shadow should have comma-separated values
    assert!(
        html.contains("box-shadow:"),
        "HTML should contain box-shadow: {html}"
    );
}

#[test]
fn test_shape_no_effects_no_box_shadow() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    // Extract the shape div's style attribute (not global CSS which has slide box-shadow)
    let shape_div_start = html
        .find("<div class=\"shape\"")
        .expect("shape div should exist");
    let shape_section =
        &html[shape_div_start..shape_div_start + 300.min(html.len() - shape_div_start)];
    assert!(
        !shape_section.contains("box-shadow"),
        "No effects means no box-shadow on shape div: {shape_section}"
    );
}

#[test]
fn test_svg_shape_effects_use_svg_filter_not_box_shadow() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Arrow"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rightArrow"/>
        <a:solidFill><a:srgbClr val="336699"/></a:solidFill>
        <a:effectLst>
          <a:outerShdw blurRad="50800" dist="38100" dir="2700000">
            <a:srgbClr val="000000"/>
          </a:outerShdw>
          <a:glow rad="63500">
            <a:srgbClr val="FFC000"/>
          </a:glow>
        </a:effectLst>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    let shape_div_start = html
        .find("<div class=\"shape\"")
        .expect("shape div should exist");
    let shape_section =
        &html[shape_div_start..shape_div_start + 800.min(html.len() - shape_div_start)];

    assert!(
        !shape_section.contains("box-shadow"),
        "SVG shapes should not use rectangular box-shadow on the outer div: {shape_section}"
    );
    assert!(
        shape_section.contains("filter: drop-shadow("),
        "SVG shapes should render effects through SVG filters: {shape_section}"
    );
}

#[test]
fn test_shape_shadow_with_scheme_color() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1500000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:effectLst>
          <a:outerShdw blurRad="50800" dist="38100" dir="5400000">
            <a:schemeClr val="dk1"/>
          </a:outerShdw>
        </a:effectLst>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    let shadow = shape
        .effects
        .outer_shadow
        .as_ref()
        .expect("should parse scheme color shadow");
    // dir: 5400000 / 60000 = 90 deg (straight down)
    assert!(
        (shadow.direction - 90.0).abs() < 0.1,
        "direction should be 90deg"
    );
}

// ── Month 4: CSS global classes test ──

#[test]
fn test_global_css_contains_svg_styles() {
    let pptx = fixtures::MinimalPptx::new("").build();
    let html = render_html(&pptx);
    assert!(
        html.contains(".shape-svg"),
        "CSS should contain .shape-svg class"
    );
    assert!(
        html.contains(".chart-placeholder"),
        "CSS should contain .chart-placeholder class"
    );
}

// ── Connector line color tests ──

#[test]
fn test_ln_defaults_and_extended_line_properties_are_parsed() {
    let slide = r#"
    <p:cxnSp>
      <p:nvCxnSpPr><p:cNvPr id="2" name="Connector"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="0"/></a:xfrm>
        <a:prstGeom prst="line"><a:avLst/></a:prstGeom>
        <a:ln cmpd="dbl" algn="in">
          <a:solidFill><a:srgbClr val="C00000"/></a:solidFill>
          <a:miter lim="400000"/>
        </a:ln>
      </p:spPr>
    </p:cxnSp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    assert_eq!(
        shape.border.width, 0.0,
        "Default ln width should remain zero"
    );
    assert!(matches!(shape.border.cap, LineCap::Square));
    assert!(matches!(shape.border.compound, CompoundLine::Double));
    assert!(matches!(shape.border.alignment, LineAlignment::Inset));
    assert_eq!(shape.border.miter_limit, Some(4.0));
}

#[test]
fn test_default_square_cap_and_miter_limit_render_to_svg() {
    let slide = r#"
    <p:cxnSp>
      <p:nvCxnSpPr><p:cNvPr id="2" name="Connector"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="0"/></a:xfrm>
        <a:prstGeom prst="line"><a:avLst/></a:prstGeom>
        <a:ln w="9525">
          <a:solidFill><a:srgbClr val="C00000"/></a:solidFill>
          <a:miter lim="400000"/>
        </a:ln>
      </p:spPr>
    </p:cxnSp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);

    assert!(
        html.contains("stroke-linecap=\"square\""),
        "Default square line cap should render to SVG: {html}"
    );
    assert!(
        html.contains("stroke-miterlimit=\"4.0\""),
        "Miter limit should render to SVG: {html}"
    );
}

#[test]
fn test_custgeom_rect_insets_text_body_padding() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="RectInsetText"/><p:cNvSpPr txBox="1"/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="1270000" cy="1270000"/></a:xfrm>
        <a:custGeom>
          <a:rect l="5400" t="2160" r="16200" b="19440"/>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="0"/></a:lnTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
              <a:lnTo><a:pt x="0" y="21600"/></a:lnTo>
              <a:close/>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
      <p:txBody>
        <a:bodyPr lIns="0" tIns="0" rIns="0" bIns="0"/>
        <a:p><a:r><a:rPr sz="1800"/><a:t>Inset text</a:t></a:r></a:p>
      </p:txBody>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);

    assert!(
        html.contains("padding: 10.0pt 25.0pt 10.0pt 25.0pt"),
        "custom geometry rect should inset text body padding: {html}"
    );
}

#[test]
fn test_straight_connector_anchors_to_custom_geometry_connection_sites() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Source"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="0" y="0"/><a:ext cx="1270000" cy="1270000"/></a:xfrm>
        <a:custGeom>
          <a:cxnLst>
            <a:cxn ang="0"><a:pos x="21600" y="10800"/></a:cxn>
          </a:cxnLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="0"/></a:lnTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
              <a:lnTo><a:pt x="0" y="21600"/></a:lnTo>
              <a:close/>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="3" name="Target"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="2540000" y="0"/><a:ext cx="1270000" cy="1270000"/></a:xfrm>
        <a:custGeom>
          <a:cxnLst>
            <a:cxn ang="18000000"><a:pos x="0" y="10800"/></a:cxn>
          </a:cxnLst>
          <a:pathLst>
            <a:path w="21600" h="21600">
              <a:moveTo><a:pt x="0" y="0"/></a:moveTo>
              <a:lnTo><a:pt x="21600" y="0"/></a:lnTo>
              <a:lnTo><a:pt x="21600" y="21600"/></a:lnTo>
              <a:lnTo><a:pt x="0" y="21600"/></a:lnTo>
              <a:close/>
            </a:path>
          </a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>
    <p:cxnSp>
      <p:nvCxnSpPr>
        <p:cNvPr id="4" name="Connector"/>
        <p:cNvCxnSpPr>
          <a:stCxn id="2" idx="0"/>
          <a:endCxn id="3" idx="0"/>
        </p:cNvCxnSpPr>
        <p:nvPr/>
      </p:nvCxnSpPr>
      <p:spPr>
        <a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/></a:xfrm>
        <a:prstGeom prst="straightConnector1"><a:avLst/></a:prstGeom>
        <a:ln w="9525"><a:solidFill><a:srgbClr val="C00000"/></a:solidFill></a:ln>
      </p:spPr>
    </p:cxnSp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);

    assert!(
        html.contains("left: 133.3px; top: 65.7px; width: 133.3px; height: 2px"),
        "anchored connector should span between connection sites: {html}"
    );
}

#[test]
fn test_connector_parses_shape_ids_and_connection_refs() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Source"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1270000" cy="1270000"/></a:xfrm><a:custGeom><a:cxnLst><a:cxn ang="0"><a:pos x="21600" y="10800"/></a:cxn></a:cxnLst><a:pathLst><a:path w="21600" h="21600"><a:moveTo><a:pt x="0" y="0"/></a:moveTo><a:lnTo><a:pt x="21600" y="21600"/></a:lnTo></a:path></a:pathLst></a:custGeom></p:spPr>
    </p:sp>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="3" name="Target"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr><a:xfrm><a:off x="2540000" y="0"/><a:ext cx="1270000" cy="1270000"/></a:xfrm><a:custGeom><a:cxnLst><a:cxn ang="18000000"><a:pos x="0" y="10800"/></a:cxn></a:cxnLst><a:pathLst><a:path w="21600" h="21600"><a:moveTo><a:pt x="0" y="0"/></a:moveTo><a:lnTo><a:pt x="21600" y="21600"/></a:lnTo></a:path></a:pathLst></a:custGeom></p:spPr>
    </p:sp>
    <p:cxnSp>
      <p:nvCxnSpPr>
        <p:cNvPr id="4" name="Connector"/>
        <p:cNvCxnSpPr><a:stCxn id="2" idx="0"/><a:endCxn id="3" idx="0"/></p:cNvCxnSpPr>
        <p:nvPr/>
      </p:nvCxnSpPr>
      <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/></a:xfrm><a:prstGeom prst="straightConnector1"><a:avLst/></a:prstGeom></p:spPr>
    </p:cxnSp>"#;

    let pres = parse_pptx(&fixtures::MinimalPptx::new(slide).build());
    assert_eq!(pres.slides[0].shapes[0].id, 2);
    assert_eq!(pres.slides[0].shapes[1].id, 3);
    let connector = &pres.slides[0].shapes[2];
    assert_eq!(connector.id, 4);
    assert_eq!(
        connector.start_connection.as_ref().map(|c| c.shape_id),
        Some(2)
    );
    assert_eq!(
        connector.start_connection.as_ref().map(|c| c.site_idx),
        Some(0)
    );
    assert_eq!(
        connector.end_connection.as_ref().map(|c| c.shape_id),
        Some(3)
    );
    assert_eq!(
        connector.end_connection.as_ref().map(|c| c.site_idx),
        Some(0)
    );
}

#[test]
fn test_bent_connector_anchors_to_custom_geometry_connection_sites() {
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Source"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="0" y="0"/><a:ext cx="1270000" cy="1270000"/></a:xfrm>
        <a:custGeom>
          <a:cxnLst><a:cxn ang="0"><a:pos x="21600" y="10800"/></a:cxn></a:cxnLst>
          <a:pathLst><a:path w="21600" h="21600"><a:moveTo><a:pt x="0" y="0"/></a:moveTo><a:lnTo><a:pt x="21600" y="21600"/></a:lnTo></a:path></a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>
    <p:sp>
      <p:nvSpPr><p:cNvPr id="3" name="Target"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="2540000" y="1270000"/><a:ext cx="1270000" cy="1270000"/></a:xfrm>
        <a:custGeom>
          <a:cxnLst><a:cxn ang="18000000"><a:pos x="0" y="10800"/></a:cxn></a:cxnLst>
          <a:pathLst><a:path w="21600" h="21600"><a:moveTo><a:pt x="0" y="0"/></a:moveTo><a:lnTo><a:pt x="21600" y="21600"/></a:lnTo></a:path></a:pathLst>
        </a:custGeom>
      </p:spPr>
    </p:sp>
    <p:cxnSp>
      <p:nvCxnSpPr>
        <p:cNvPr id="4" name="Bent Connector"/>
        <p:cNvCxnSpPr><a:stCxn id="2" idx="0"/><a:endCxn id="3" idx="0"/></p:cNvCxnSpPr>
        <p:nvPr/>
      </p:nvCxnSpPr>
      <p:spPr>
        <a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/></a:xfrm>
        <a:prstGeom prst="bentConnector2"><a:avLst/></a:prstGeom>
        <a:ln w="9525"><a:solidFill><a:srgbClr val="C00000"/></a:solidFill></a:ln>
      </p:spPr>
    </p:cxnSp>"#;

    let html = render_html(&fixtures::MinimalPptx::new(slide).build());

    assert!(
        html.contains("left: 133.3px; top: 66.7px; width: 133.3px; height: 133.3px"),
        "bent connector should use anchored bounding box: {html}"
    );
    assert!(
        html.contains("M0,0 L0,133.3 L133.3,133.3")
            || html.contains("M0.0,0.0 L0.0,133.3 L133.3,133.3"),
        "bent connector should render anchored bent path: {html}"
    );
}

#[test]
fn test_bent_connector5_adjust_values_are_parsed_and_rendered() {
    let slide = r#"
    <p:cxnSp>
      <p:nvCxnSpPr><p:cNvPr id="2" name="Bent Connector 5"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="1200000"/></a:xfrm>
        <a:prstGeom prst="bentConnector5">
          <a:avLst>
            <a:gd name="adj1" fmla="val 20000"/>
            <a:gd name="adj2" fmla="val 35000"/>
            <a:gd name="adj3" fmla="val 80000"/>
          </a:avLst>
        </a:prstGeom>
        <a:ln w="9525"><a:solidFill><a:srgbClr val="4472C4"/></a:solidFill></a:ln>
      </p:spPr>
    </p:cxnSp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];

    assert!(matches!(
        shape.shape_type,
        ShapeType::Custom(ref name) if name == "bentConnector5"
    ));
    assert_eq!(
        shape
            .adjust_values
            .as_ref()
            .and_then(|values| values.get("adj1"))
            .copied(),
        Some(20_000.0)
    );
    assert_eq!(
        shape
            .adjust_values
            .as_ref()
            .and_then(|values| values.get("adj2"))
            .copied(),
        Some(35_000.0)
    );
    assert_eq!(
        shape
            .adjust_values
            .as_ref()
            .and_then(|values| values.get("adj3"))
            .copied(),
        Some(80_000.0)
    );

    let html = render_html(&pptx);
    assert!(
        html.contains(
            "M0,0 L42.0,0 L42.0,44.1 L105.0,44.1 L105.0,72.4 L168.0,72.4 L168.0,126.0 L210.0,126.0"
        ),
        "bentConnector5 should render the adjusted multi-bend path: {html}"
    );
    assert!(
        html.contains("stroke=\"#4472C4\""),
        "bentConnector5 should keep its inline stroke color: {html}"
    );
}

#[test]
fn test_connector_border_color_srgb() {
    // Connector with inline srgbClr in <a:ln> — must parse border color
    let slide = r#"
    <p:cxnSp>
      <p:nvCxnSpPr><p:cNvPr id="2" name="Connector"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="0"/></a:xfrm>
        <a:prstGeom prst="line"><a:avLst/></a:prstGeom>
        <a:ln w="9525">
          <a:solidFill><a:srgbClr val="C00000"/></a:solidFill>
        </a:ln>
      </p:spPr>
    </p:cxnSp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    assert!(
        shape.border.width > 0.0,
        "border width should be > 0, got {}",
        shape.border.width
    );
    assert_eq!(
        shape.border.color.kind,
        color::ColorKind::Rgb("C00000".to_string()),
        "border color should be C00000, got {:?}",
        shape.border.color
    );
}

#[test]
fn test_connector_with_style_and_inline_color() {
    // Connector with both inline <a:ln> color AND <p:style> (real-world pattern)
    let slide = r#"
    <p:cxnSp>
      <p:nvCxnSpPr><p:cNvPr id="2" name="Connector"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="0"/></a:xfrm>
        <a:prstGeom prst="line"><a:avLst/></a:prstGeom>
        <a:ln w="9525" cap="flat" cmpd="sng" algn="ctr">
          <a:solidFill><a:srgbClr val="C00000"/></a:solidFill>
          <a:prstDash val="dash"/>
          <a:round/>
          <a:headEnd type="none" w="med" len="med"/>
          <a:tailEnd type="none" w="med" len="med"/>
        </a:ln>
      </p:spPr>
      <p:style>
        <a:lnRef idx="0"><a:scrgbClr r="0" g="0" b="0"/></a:lnRef>
        <a:fillRef idx="0"><a:scrgbClr r="0" g="0" b="0"/></a:fillRef>
        <a:effectRef idx="0"><a:scrgbClr r="0" g="0" b="0"/></a:effectRef>
        <a:fontRef idx="minor"><a:schemeClr val="tx1"/></a:fontRef>
      </p:style>
    </p:cxnSp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    assert!(
        shape.border.width > 0.0,
        "border width should be > 0, got {}",
        shape.border.width
    );
    assert_eq!(
        shape.border.color.kind,
        color::ColorKind::Rgb("C00000".to_string()),
        "border color should be C00000 with style element present, got {:?}",
        shape.border.color
    );
}

#[test]
fn test_connector_border_color_srgb_html() {
    // Connector rendered as SVG should use the parsed stroke color, not black
    let slide = r#"
    <p:cxnSp>
      <p:nvCxnSpPr><p:cNvPr id="2" name="Connector"/><p:cNvCxnSpPr/><p:nvPr/></p:nvCxnSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="2000000" cy="0"/></a:xfrm>
        <a:prstGeom prst="line"><a:avLst/></a:prstGeom>
        <a:ln w="9525">
          <a:solidFill><a:srgbClr val="C00000"/></a:solidFill>
        </a:ln>
      </p:spPr>
    </p:cxnSp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("#C00000"),
        "HTML should contain connector stroke color #C00000, got: {html}"
    );
}

// ── noFill inside <a:ln> suppresses border ──

#[test]
fn test_ln_nofill_suppresses_border() {
    // Shape with <a:ln><a:noFill/></a:ln> must have zero-width border
    // even though <a:ln> is present (regression: was defaulting to 1pt black)
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Donut"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="200000" cy="200000"/></a:xfrm>
        <a:prstGeom prst="donut"><a:avLst/></a:prstGeom>
        <a:solidFill><a:srgbClr val="C00000"/></a:solidFill>
        <a:ln><a:noFill/></a:ln>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    assert_eq!(
        shape.border.width, 0.0,
        "noFill inside <a:ln> should yield border width 0, got {}",
        shape.border.width
    );
    assert!(
        shape.border.no_fill,
        "noFill inside <a:ln> should set no_fill flag"
    );
}

#[test]
fn test_ln_nofill_no_black_stroke_in_svg() {
    // SVG output for shape with <a:ln><a:noFill/></a:ln> must NOT have #000 stroke
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Donut"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="200000" cy="200000"/></a:xfrm>
        <a:prstGeom prst="donut"><a:avLst/></a:prstGeom>
        <a:solidFill><a:srgbClr val="C00000"/></a:solidFill>
        <a:ln><a:noFill/></a:ln>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        !html.contains("stroke=\"#000\""),
        "noFill inside <a:ln> must not produce #000 stroke: {html}"
    );
}

// ── Background gradient fill tests ──

#[test]
fn test_background_gradient_fill_parsing() {
    // Slide with background gradient fill using self-closing color tags (Empty events)
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:gradFill>
          <a:gsLst>
            <a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs>
            <a:gs pos="50000"><a:srgbClr val="00FF00"/></a:gs>
            <a:gs pos="100000"><a:srgbClr val="0000FF"/></a:gs>
          </a:gsLst>
          <a:lin ang="5400000" scaled="0"/>
        </a:gradFill>
        <a:effectLst/>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_raw_slide(slide_xml)
        .build();
    let pres = parse_pptx(&pptx);
    let slide = &pres.slides[0];
    match &slide.background {
        Some(Fill::Gradient(gf)) => {
            assert_eq!(gf.stops.len(), 3, "Expected 3 gradient stops");
            assert!((gf.stops[0].position - 0.0).abs() < 0.01);
            assert!((gf.stops[1].position - 0.5).abs() < 0.01);
            assert!((gf.stops[2].position - 1.0).abs() < 0.01);
            assert!((gf.angle - 90.0).abs() < 0.1);
        }
        other => panic!("Expected Fill::Gradient for background, got: {other:?}"),
    }
}

#[test]
fn test_background_gradient_fill_html() {
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:gradFill>
          <a:gsLst>
            <a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs>
            <a:gs pos="100000"><a:srgbClr val="0000FF"/></a:gs>
          </a:gsLst>
          <a:lin ang="5400000"/>
        </a:gradFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_raw_slide(slide_xml)
        .build();
    let html = render_html(&pptx);
    assert!(
        html.contains("linear-gradient"),
        "Background gradient should produce linear-gradient CSS: {html}"
    );
    assert!(
        html.contains("#FF0000"),
        "Start color not found in gradient CSS"
    );
    assert!(
        html.contains("#0000FF"),
        "End color not found in gradient CSS"
    );
}

#[test]
fn test_background_gradient_with_scheme_colors() {
    // Background gradient using schemeClr (self-closing Empty event)
    let slide_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<p:sld xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
       xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
       xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
  <p:cSld>
    <p:bg>
      <p:bgPr>
        <a:gradFill>
          <a:gsLst>
            <a:gs pos="0"><a:schemeClr val="accent1"/></a:gs>
            <a:gs pos="100000"><a:schemeClr val="accent2"/></a:gs>
          </a:gsLst>
          <a:lin ang="0"/>
        </a:gradFill>
      </p:bgPr>
    </p:bg>
    <p:spTree>
      <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
      <p:grpSpPr/>
    </p:spTree>
  </p:cSld>
</p:sld>"#;

    let pptx = fixtures::MinimalPptx::new("")
        .with_raw_slide(slide_xml)
        .build();
    let pres = parse_pptx(&pptx);
    let slide = &pres.slides[0];
    match &slide.background {
        Some(Fill::Gradient(gf)) => {
            assert_eq!(gf.stops.len(), 2, "Expected 2 gradient stops");
        }
        other => panic!("Expected Fill::Gradient for scheme color background, got: {other:?}"),
    }
}

#[test]
fn test_background_image_fill() {
    let data = build_background_image_pptx();

    // Verify parsing produces Fill::Image
    let pres = parse_pptx(&data);
    let slide = &pres.slides[0];
    match &slide.background {
        Some(Fill::Image(img)) => {
            assert!(!img.data.is_empty(), "Image data should not be empty");
            assert_eq!(img.content_type, "image/png");
        }
        other => panic!("Expected Fill::Image for background, got: {other:?}"),
    }

    // Verify HTML rendering produces background-image
    let html = render_html(&data);
    assert!(
        html.contains("background-image"),
        "Background image should produce background-image CSS"
    );
    assert!(
        html.contains("data:image/png;base64,"),
        "Background image should be embedded as base64"
    );
}

#[test]
fn test_external_assets_collected_when_embedding_disabled() {
    let data = build_background_image_pptx();
    let opts = pptx2html_core::ConversionOptions {
        embed_images: false,
        ..Default::default()
    };

    let result = pptx2html_core::convert_bytes_with_options_metadata(&data, &opts)
        .expect("conversion should succeed");

    assert!(
        result.html.contains("images/slide-1/background-0.png"),
        "Expected deterministic background asset path in HTML: {}",
        result.html
    );
    assert_eq!(
        result.external_assets.len(),
        1,
        "Expected one external asset"
    );
    let asset = &result.external_assets[0];
    assert_eq!(asset.relative_path, "images/slide-1/background-0.png");
    assert_eq!(asset.content_type, "image/png");
    assert!(
        !asset.data.is_empty(),
        "External asset bytes should be preserved"
    );
}

// ── Gradient fill with color modifiers (Start+End events) ──

#[test]
fn test_gradient_fill_with_modifiers() {
    // Real-world gradient: colors have child modifiers (tint, satMod),
    // making them Start+End events instead of Empty events.
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="roundRect"/>
        <a:gradFill>
          <a:gsLst>
            <a:gs pos="0">
              <a:srgbClr val="FF0000">
                <a:tint val="50000"/>
              </a:srgbClr>
            </a:gs>
            <a:gs pos="100000">
              <a:srgbClr val="0000FF">
                <a:shade val="80000"/>
              </a:srgbClr>
            </a:gs>
          </a:gsLst>
          <a:lin ang="5400000" scaled="1"/>
        </a:gradFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    match &shape.fill {
        Fill::Gradient(gf) => {
            assert_eq!(
                gf.stops.len(),
                2,
                "Expected 2 gradient stops, got {}",
                gf.stops.len()
            );
            assert!((gf.stops[0].position - 0.0).abs() < 0.01);
            assert!((gf.stops[1].position - 1.0).abs() < 0.01);
            assert!((gf.angle - 90.0).abs() < 0.1);
        }
        other => panic!("Expected Fill::Gradient with modifiers, got: {other:?}"),
    }
}

#[test]
fn test_gradient_fill_with_modifiers_html() {
    // Gradient with color modifiers should produce SVG linearGradient for SVG shapes
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="roundRect"/>
        <a:gradFill>
          <a:gsLst>
            <a:gs pos="0">
              <a:srgbClr val="FF0000">
                <a:tint val="50000"/>
              </a:srgbClr>
            </a:gs>
            <a:gs pos="100000">
              <a:srgbClr val="0000FF">
                <a:shade val="80000"/>
              </a:srgbClr>
            </a:gs>
          </a:gsLst>
          <a:lin ang="5400000"/>
        </a:gradFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    // SVG shapes (roundRect) use SVG <linearGradient> + url(#gradN)
    assert!(
        html.contains("linearGradient"),
        "SVG gradient def not found in HTML: {html}"
    );
    assert!(
        html.contains("url(#grad"),
        "SVG gradient url reference not found in HTML: {html}"
    );
}

#[test]
fn test_gradient_fill_rect_css() {
    // Non-SVG rect shapes should use CSS linear-gradient
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="rect"/>
        <a:gradFill>
          <a:gsLst>
            <a:gs pos="0">
              <a:srgbClr val="FF0000">
                <a:tint val="50000"/>
              </a:srgbClr>
            </a:gs>
            <a:gs pos="100000">
              <a:srgbClr val="0000FF">
                <a:shade val="80000"/>
              </a:srgbClr>
            </a:gs>
          </a:gsLst>
          <a:lin ang="5400000"/>
        </a:gradFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let html = render_html(&pptx);
    assert!(
        html.contains("linear-gradient"),
        "CSS gradient not found in HTML for rect shape: {html}"
    );
}

#[test]
fn test_gradient_fill_scheme_colors_with_modifiers() {
    // Real-world pattern: schemeClr with tint/satMod modifiers in shape gradient
    let slide = r#"
    <p:sp>
      <p:nvSpPr><p:cNvPr id="2" name="Rect"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
      <p:spPr>
        <a:xfrm><a:off x="100000" y="100000"/><a:ext cx="3000000" cy="1000000"/></a:xfrm>
        <a:prstGeom prst="roundRect"/>
        <a:gradFill>
          <a:gsLst>
            <a:gs pos="0">
              <a:schemeClr val="accent1">
                <a:tint val="66000"/>
                <a:satMod val="160000"/>
              </a:schemeClr>
            </a:gs>
            <a:gs pos="50000">
              <a:schemeClr val="accent1">
                <a:tint val="44500"/>
                <a:satMod val="160000"/>
              </a:schemeClr>
            </a:gs>
            <a:gs pos="100000">
              <a:schemeClr val="accent1">
                <a:tint val="23500"/>
                <a:satMod val="160000"/>
              </a:schemeClr>
            </a:gs>
          </a:gsLst>
          <a:lin ang="5400000" scaled="0"/>
        </a:gradFill>
      </p:spPr>
    </p:sp>"#;

    let pptx = fixtures::MinimalPptx::new(slide).build();
    let pres = parse_pptx(&pptx);
    let shape = &pres.slides[0].shapes[0];
    match &shape.fill {
        Fill::Gradient(gf) => {
            assert_eq!(
                gf.stops.len(),
                3,
                "Expected 3 gradient stops, got {}",
                gf.stops.len()
            );
            assert!((gf.stops[0].position - 0.0).abs() < 0.01);
            assert!((gf.stops[1].position - 0.5).abs() < 0.01);
            assert!((gf.stops[2].position - 1.0).abs() < 0.01);
        }
        other => panic!("Expected Fill::Gradient with scheme colors, got: {other:?}"),
    }
}
