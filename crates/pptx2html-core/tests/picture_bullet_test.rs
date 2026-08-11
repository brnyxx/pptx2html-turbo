mod fixtures;

use std::io::{Cursor, Read, Write};

use fixtures::{FeaturePart, MinimalPptx, PackageBuilder, Relationship, SlideXml};
use pptx2html_core::{ConversionOptions, convert_bytes_with_options_metadata};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

const IMAGE_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
const CHART_RELATIONSHIP: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart";
const PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 8, 215, 99, 248, 207, 192, 240, 31, 0, 5,
    0, 1, 255, 137, 153, 61, 29, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

fn shape(paragraphs: &str) -> String {
    format!(
        r#"<p:sp>
  <p:nvSpPr><p:cNvPr id="2" name="Picture bullets"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="5000000" cy="3000000"/></a:xfrm><a:prstGeom prst="rect"><a:avLst/></a:prstGeom></p:spPr>
  <p:txBody><a:bodyPr/><a:lstStyle/>{paragraphs}</p:txBody>
</p:sp>"#
    )
}

fn paragraph(reference: &str, size: &str, text: &str) -> String {
    format!(
        r#"<a:p><a:pPr>{size}<a:buBlip><a:blip {reference}/></a:buBlip></a:pPr><a:r><a:rPr sz="2000"/><a:t>{text}</a:t></a:r></a:p>"#
    )
}

fn slide(body: &str) -> String {
    SlideXml::from_body(body).build().replacen(
        "xmlns:mc=",
        "xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" xmlns:mc=",
        1,
    )
}

fn options(embed_images: bool) -> ConversionOptions {
    ConversionOptions {
        embed_images,
        ..Default::default()
    }
}

fn replace_package_entry(package: &[u8], entry_name: &str, replacement: &[u8]) -> Vec<u8> {
    let mut archive = ZipArchive::new(Cursor::new(package)).expect("fixture archive opens");
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).expect("fixture entry opens");
        let name = entry.name().to_owned();
        writer
            .start_file(&name, SimpleFileOptions::default())
            .expect("fixture replacement entry starts");
        if name == entry_name {
            writer
                .write_all(replacement)
                .expect("replacement content types write");
        } else {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).expect("fixture entry reads");
            writer.write_all(&bytes).expect("fixture entry writes");
        }
    }
    writer
        .finish()
        .expect("replacement archive finishes")
        .into_inner()
}

#[path = "picture_bullet/assets.rs"]
mod assets;
#[path = "picture_bullet/inheritance.rs"]
mod inheritance;
#[path = "picture_bullet/safety.rs"]
mod safety;
#[path = "picture_bullet/sizing.rs"]
mod sizing;
