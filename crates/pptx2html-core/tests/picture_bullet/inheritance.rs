use super::{
    FeaturePart, IMAGE_RELATIONSHIP, MinimalPptx, PNG, PackageBuilder, Relationship,
    convert_bytes_with_options_metadata, options, replace_package_entry, shape, slide,
};

#[test]
fn table_cell_picture_bullets_share_image_and_fallback_semantics() {
    // Given
    let cell = |relationship_id: &str, text: &str| {
        format!(
            r#"<a:tc><a:txBody><a:bodyPr/><a:p><a:pPr><a:buSzTx/><a:buBlip><a:blip r:embed="{relationship_id}"/></a:buBlip></a:pPr><a:r><a:rPr sz="2000"/><a:t>{text}</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>"#
        )
    };
    let table = format!(
        r#"<p:graphicFrame>
  <p:nvGraphicFramePr><p:cNvPr id="3" name="Picture bullet table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr>
  <p:xfrm><a:off x="0" y="0"/><a:ext cx="4000000" cy="1000000"/></p:xfrm>
  <a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table">
    <a:tbl><a:tblPr/><a:tblGrid><a:gridCol w="2000000"/><a:gridCol w="2000000"/></a:tblGrid>
      <a:tr h="1000000">{}{}</a:tr>
    </a:tbl>
  </a:graphicData></a:graphic>
</p:graphicFrame>"#,
        cell("rIdBullet", "Table image"),
        cell("rIdMissing", "Table fallback"),
    );
    let package = PackageBuilder::new(slide(&table))
        .with_slide_relationship(Relationship::internal(
            "rIdBullet",
            IMAGE_RELATIONSHIP,
            "../media/bullet.png",
        ))
        .with_part(FeaturePart::media("bullet.png", "image/png", PNG))
        .build()
        .expect("table picture bullet fixture builds");

    // When
    let result = convert_bytes_with_options_metadata(&package, &options(false))
        .expect("table picture bullet conversion succeeds");

    // Then
    assert_eq!(result.html.matches("class=\"picture-bullet\"").count(), 1);
    assert_eq!(result.html.matches("picture-bullet-missing").count(), 1);
    assert!(result.html.contains("Table image"));
    assert!(result.html.contains("Table fallback"));
    assert_eq!(result.external_assets.len(), 1);
    assert_eq!(result.external_assets[0].data, PNG);
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "PICTURE_BULLET_IMAGE_MISSING")
            .count(),
        1
    );
}

#[test]
fn table_cell_list_style_picture_bullets_are_inherited_at_level_zero() {
    // Given
    let cell = |relationship_id: &str, text: &str| {
        format!(
            r#"<a:tc><a:txBody><a:bodyPr/><a:lstStyle><a:lvl1pPr><a:buSzTx/><a:buBlip><a:blip r:embed="{relationship_id}"/></a:buBlip></a:lvl1pPr></a:lstStyle><a:p><a:pPr lvl="0"/><a:r><a:rPr sz="2000"/><a:t>{text}</a:t></a:r></a:p></a:txBody><a:tcPr/></a:tc>"#
        )
    };
    let table = format!(
        r#"<p:graphicFrame><p:nvGraphicFramePr><p:cNvPr id="3" name="List style table"/><p:cNvGraphicFramePr/><p:nvPr/></p:nvGraphicFramePr><p:xfrm><a:off x="0" y="0"/><a:ext cx="4000000" cy="1000000"/></p:xfrm><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/table"><a:tbl><a:tblPr/><a:tblGrid><a:gridCol w="2000000"/><a:gridCol w="2000000"/></a:tblGrid><a:tr h="1000000">{}{}</a:tr></a:tbl></a:graphicData></a:graphic></p:graphicFrame>"#,
        cell("rIdBullet", "Inherited table image"),
        cell("rIdMissing", "Inherited table fallback"),
    );
    let package = PackageBuilder::new(slide(&table))
        .with_slide_relationship(Relationship::internal(
            "rIdBullet",
            IMAGE_RELATIONSHIP,
            "../media/bullet.png",
        ))
        .with_part(FeaturePart::media("bullet.png", "image/png", PNG))
        .build()
        .expect("table list style fixture builds");

    // When
    let result = convert_bytes_with_options_metadata(&package, &options(false))
        .expect("table list style conversion succeeds");

    // Then
    assert_eq!(result.html.matches("class=\"picture-bullet\"").count(), 1);
    assert_eq!(result.html.matches("picture-bullet-missing").count(), 1);
    assert!(result.html.contains("Inherited table image"));
    assert!(result.html.contains("Inherited table fallback"));
    assert_eq!(result.external_assets.len(), 1);
    assert_eq!(result.external_assets[0].data, PNG);
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "PICTURE_BULLET_IMAGE_MISSING")
            .count(),
        1
    );
}

#[test]
fn shape_list_style_picture_bullet_is_inherited_by_paragraph() {
    // Given
    let body = r#"<p:sp>
  <p:nvSpPr><p:cNvPr id="4" name="Inherited picture bullet"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="3000000" cy="1000000"/></a:xfrm><a:prstGeom prst="rect"/></p:spPr>
  <p:txBody><a:bodyPr/><a:lstStyle><a:lvl1pPr><a:buSzTx/><a:buBlip><a:blip r:embed="rIdBullet"/></a:buBlip></a:lvl1pPr></a:lstStyle>
    <a:p><a:pPr lvl="0"/><a:r><a:rPr sz="2000"/><a:t>Inherited visible</a:t></a:r></a:p>
  </p:txBody>
</p:sp>"#;
    let package = PackageBuilder::new(slide(body))
        .with_slide_relationship(Relationship::internal(
            "rIdBullet",
            IMAGE_RELATIONSHIP,
            "../media/bullet.png",
        ))
        .with_part(FeaturePart::media("bullet.png", "image/png", PNG))
        .build()
        .expect("inherited picture bullet fixture builds");

    // When
    let result = convert_bytes_with_options_metadata(&package, &options(true))
        .expect("inherited picture bullet conversion succeeds");

    // Then
    assert_eq!(result.html.matches("class=\"picture-bullet\"").count(), 1);
    assert!(result.html.contains("height: 20.0pt"));
    assert!(result.html.contains("Inherited visible"));
    assert!(
        result
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "PICTURE_BULLET_IMAGE_MISSING")
    );
}

#[test]
fn owner_inherited_picture_bullets_emit_explicit_fallback_diagnostics() {
    // Given
    let visible = shape(r#"<a:p><a:r><a:rPr sz="2000"/><a:t>Owner visible</a:t></a:r></a:p>"#);
    let relationship = |required: &str| {
        format!(
            r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{required}<Relationship Id="rIdPB" Type="{IMAGE_RELATIONSHIP}" Target="https://user:password@example.invalid/bullet.png?token=secret" TargetMode="External"/></Relationships>"#
        )
    };
    let bullet_style =
        r#"<a:lvl1pPr><a:buSzTx/><a:buBlip><a:blip r:link="rIdPB"/></a:buBlip></a:lvl1pPr>"#;
    let presentation = format!(
        r#"<p:presentation xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:sldMasterIdLst><p:sldMasterId r:id="rId1"/></p:sldMasterIdLst><p:sldIdLst><p:sldId id="256" r:id="rId2"/></p:sldIdLst><p:sldSz cx="9144000" cy="6858000"/><p:defaultTextStyle>{bullet_style}</p:defaultTextStyle></p:presentation>"#
    );
    let master = format!(
        r#"<p:sldMaster xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/></p:spTree></p:cSld><p:txStyles><p:titleStyle/><p:bodyStyle>{bullet_style}</p:bodyStyle><p:otherStyle/></p:txStyles><p:clrMap/></p:sldMaster>"#
    );
    let layout = format!(
        r#"<p:sldLayout xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main"><p:cSld><p:spTree><p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr><p:grpSpPr/><p:sp><p:nvSpPr><p:cNvPr id="2" name="layout style"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr/><p:txBody><a:bodyPr/><a:lstStyle>{bullet_style}</a:lstStyle></p:txBody></p:sp></p:spTree></p:cSld></p:sldLayout>"#
    );
    let standard_presentation_rels = r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="slideMasters/slideMaster1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide" Target="slides/slide1.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/>"#;
    let standard_master_rels = r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="../theme/theme1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout" Target="../slideLayouts/slideLayout1.xml"/>"#;
    let standard_layout_rels = r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster" Target="../slideMasters/slideMaster1.xml"/>"#;
    let presentation_package = MinimalPptx::new(&visible)
        .with_presentation_xml(&presentation)
        .build();
    let presentation_package = replace_package_entry(
        &presentation_package,
        "ppt/_rels/presentation.xml.rels",
        relationship(standard_presentation_rels).as_bytes(),
    );
    let master_package = MinimalPptx::new(&visible).with_full_master(&master).build();
    let master_package = replace_package_entry(
        &master_package,
        "ppt/slideMasters/_rels/slideMaster1.xml.rels",
        relationship(standard_master_rels).as_bytes(),
    );
    let cases = [
        (presentation_package, "ppt/presentation.xml"),
        (master_package, "ppt/slideMasters/slideMaster1.xml"),
        (
            MinimalPptx::new(&visible)
                .with_full_master(&master.replace(bullet_style, ""))
                .with_layout(&layout)
                .with_layout_rels(&relationship(standard_layout_rels))
                .with_slide_layout_rel()
                .build(),
            "ppt/slideLayouts/slideLayout1.xml",
        ),
    ];

    for (package, owner_part) in cases {
        // When
        let result = convert_bytes_with_options_metadata(&package, &options(true))
            .expect("owner inheritance fallback conversion succeeds");

        // Then
        assert!(result.html.contains("Owner visible"));
        let diagnostics = result
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "PICTURE_BULLET_INHERITANCE_UNSUPPORTED")
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 1, "owner {owner_part}");
        assert_eq!(
            diagnostics[0].location.part_name.as_deref(),
            Some(owner_part)
        );
        assert_eq!(
            diagnostics[0].location.relationship_id.as_deref(),
            Some("rIdPB")
        );
        assert_eq!(
            diagnostics[0].location.relationship_type.as_deref(),
            Some(IMAGE_RELATIONSHIP)
        );
        assert_eq!(
            diagnostics[0].location.qualified_element_name.as_deref(),
            Some("a:buBlip")
        );
        assert!(diagnostics[0].reason.contains("External"));
        assert!(!result.html.contains("password"));
        assert!(!result.html.contains("token=secret"));
    }
}
