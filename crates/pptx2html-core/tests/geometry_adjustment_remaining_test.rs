mod fixtures;

use std::path::Path;
use std::process::Command;

use fixtures::MinimalPptx;
use pptx2html_core::model::{CustomGuide, GuideFormulaError, ShapeType};
use pptx2html_core::parser::PptxParser;
use pptx2html_core::{convert_bytes, convert_bytes_with_metadata};

const UNKNOWN_FORMULA: &str = "unknownOp 1 2";
const REMAINING_PRESETS: &[&str] = &[
    "heptagon",
    "decagon",
    "dodecagon",
    "star4",
    "star5",
    "star6",
    "star7",
    "star8",
    "star10",
    "star12",
    "star16",
    "star24",
    "star32",
    "teardrop",
    "pieWedge",
    "pie",
    "blockArc",
    "donut",
    "noSmoking",
    "cube",
    "can",
    "lightningBolt",
    "heart",
    "sun",
    "moon",
    "smileyFace",
    "irregularSeal1",
    "irregularSeal2",
    "bevel",
    "frame",
    "chord",
    "arc",
    "cloud",
    "ribbon",
    "ribbon2",
    "leftRightRibbon",
    "wave",
    "doubleWave",
    "plus",
    "actionButtonBlank",
    "actionButtonHome",
    "actionButtonHelp",
    "actionButtonInformation",
    "actionButtonForwardNext",
    "actionButtonBackPrevious",
    "actionButtonEnd",
    "actionButtonBeginning",
    "actionButtonReturn",
    "actionButtonDocument",
    "actionButtonSound",
    "actionButtonMovie",
    "gear6",
    "gear9",
    "funnel",
    "mathPlus",
    "mathMinus",
    "mathMultiply",
    "mathDivide",
    "mathEqual",
    "mathNotEqual",
    "chartX",
    "chartStar",
    "chartPlus",
];

#[derive(Clone, Copy)]
struct AdjustmentCase<'a> {
    preset: &'a str,
    key: &'a str,
    default: f64,
    lower: f64,
    upper: f64,
}

#[test]
fn remaining_manifest_rows_are_consumed_by_official_rendering() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let output = Command::new("python3")
        .current_dir(repo_root)
        .args([
            "evaluate/check_preset_adjustments.py",
            "--repo-root",
            ".",
            "--bundle",
            "remaining",
        ])
        .output()
        .expect("run remaining checker");
    assert!(output.status.success(), "checker failed");
    let stdout = String::from_utf8(output.stdout).expect("checker UTF-8");
    assert!(stdout.contains("presets=63"), "{stdout}");
    assert!(
        stdout.contains("manifest_keys_never_consumed=0"),
        "{stdout}"
    );
}

#[test]
fn unknown_custom_formula_preserves_raw_value_in_structured_fallback() {
    let shape = format!(
        r#"<p:sp>
  <p:nvSpPr><p:cNvPr id="2" name="Unknown formula"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm>
    <a:custGeom><a:avLst/><a:gdLst><a:gd name="unknownGuide" fmla="{UNKNOWN_FORMULA}"/></a:gdLst>
      <a:pathLst><a:path w="21600" h="21600"><a:moveTo><a:pt x="unknownGuide" y="0"/></a:moveTo><a:lnTo><a:pt x="21600" y="21600"/></a:lnTo></a:path></a:pathLst>
    </a:custGeom>
  </p:spPr>
</p:sp>"#
    );
    let package = MinimalPptx::new(&shape).build();
    let result = convert_bytes_with_metadata(&package).expect("conversion remains non-fatal");
    let diagnostic = result
        .diagnostics()
        .iter()
        .find(|item| item.code == "DRAWINGML_CUSTOM_GEOMETRY_FALLBACK")
        .expect("typed custom geometry fallback diagnostic");
    assert_eq!(diagnostic.support_tier.as_str(), "fallback");
    assert_eq!(
        diagnostic.fallback_kind.as_str(),
        "custom-geometry-placeholder"
    );
    assert_eq!(diagnostic.raw_reference.as_deref(), Some(UNKNOWN_FORMULA));
    assert!(result.html.contains("data-type=\"custom-geometry\""));
    assert!(!result.html.contains("NaN") && !result.html.contains("Infinity"));
}

#[test]
fn known_custom_operator_matrix_preserves_raw_formulas_and_finite_results() {
    let formulas = [
        ("base", "val 4000"),
        ("sum", "+- base 5000 1000"),
        ("product", "*/ sum 3 2"),
        ("average", "+/ product 2000 2"),
        ("pinned", "pin 1000 average 12000"),
        ("minimum", "min pinned 9000"),
        ("maximum", "max minimum 7000"),
        ("choice", "?: maximum 8000 3000"),
        ("absolute", "abs -4500"),
        ("root", "sqrt 8100"),
        ("length", "mod 3 4 12"),
        ("sine", "sin 10000 5400000"),
        ("cosine", "cos 10000 0"),
        ("cat", "cat2 100 3 4"),
        ("sat", "sat2 100 3 4"),
        ("angle", "at2 3 4"),
        ("tangent", "tan 10000 2700000"),
    ];
    let guide_xml = formulas
        .iter()
        .map(|(name, formula)| format!(r#"<a:gd name="{name}" fmla="{formula}"/>"#))
        .collect::<String>();
    let shape = format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Known formulas"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
<p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm>
<a:custGeom><a:avLst/><a:gdLst>{guide_xml}</a:gdLst><a:pathLst><a:path w="21600" h="21600">
<a:moveTo><a:pt x="choice" y="root"/></a:moveTo><a:lnTo><a:pt x="sine" y="cosine"/></a:lnTo>
</a:path></a:pathLst></a:custGeom></p:spPr></p:sp>"#
    );
    let package = MinimalPptx::new(&shape).build();
    let presentation = PptxParser::parse_bytes(&package).expect("parse known formulas");
    let ShapeType::CustomGeom(geometry) = &presentation.slides[0].shapes[0].shape_type else {
        panic!("known formulas must remain custom geometry")
    };
    assert_eq!(geometry.guides.len(), formulas.len());
    for (guide, (name, formula)) in geometry.guides.iter().zip(formulas) {
        assert_eq!(guide.name, name);
        assert_eq!(guide.raw_formula, formula);
        assert!(
            guide
                .evaluation
                .as_ref()
                .is_ok_and(|value| value.is_finite())
        );
    }
    let result = convert_bytes_with_metadata(&package).expect("render known formulas");
    assert!(
        result
            .diagnostics()
            .iter()
            .all(|item| item.code != "DRAWINGML_CUSTOM_GEOMETRY_FALLBACK")
    );
    assert!(!result.html.contains("NaN") && !result.html.contains("Infinity"));
}

#[test]
fn predefined_guides_and_coupled_references_render_at_shape_extent_scale() {
    let formulas = [
        ("half_w", "*/ w 1 2", 457_200.0),
        ("shape_h", "val h", 457_200.0),
        ("center_x", "val hc", 457_200.0),
        ("center_y", "val vc", 228_600.0),
        ("short", "val ss", 457_200.0),
        ("long", "val ls", 914_400.0),
        ("half_h", "val hd2", 228_600.0),
        ("quarter_w", "val wd4", 228_600.0),
        ("tenth_w", "val wd10", 91_440.0),
        ("eighth_short", "val ssd8", 57_150.0),
        ("quarter_turn", "val cd4", 5_400_000.0),
        ("coupled", "*/ half_w 1 2", 228_600.0),
    ];
    let guide_xml = formulas
        .iter()
        .map(|(name, formula, _)| format!(r#"<a:gd name="{name}" fmla="{formula}"/>"#))
        .collect::<String>();
    let shape = format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Predefined guides"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
<p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm>
<a:custGeom><a:avLst/><a:gdLst>{guide_xml}</a:gdLst><a:pathLst><a:path>
<a:moveTo><a:pt x="coupled" y="center_y"/></a:moveTo><a:lnTo><a:pt x="r" y="b"/></a:lnTo>
</a:path></a:pathLst></a:custGeom></p:spPr></p:sp>"#
    );
    let package = MinimalPptx::new(&shape).build();
    let presentation = PptxParser::parse_bytes(&package).expect("parse predefined guides");
    let ShapeType::CustomGeom(geometry) = &presentation.slides[0].shapes[0].shape_type else {
        panic!("official predefined guides must render directly")
    };
    for (guide, (name, raw_formula, expected)) in geometry.guides.iter().zip(formulas) {
        assert_eq!(guide.name, name);
        assert_eq!(guide.raw_formula, raw_formula);
        let actual = guide.evaluation.as_ref().expect("finite guide result");
        assert!((actual - expected).abs() < 1e-6, "{name}: {actual}");
    }
    let result = convert_bytes_with_metadata(&package).expect("render predefined guides");
    assert!(result.diagnostics().is_empty());
    assert!(result.html.contains("d=\"M24.00,24.00 L96.00,48.00\""));
}

#[test]
fn signed_angles_and_multi_turn_trigonometry_follow_drawingml_units() {
    let shape = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Signed angles"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
<p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm>
<a:custGeom><a:avLst/><a:gdLst>
<a:gd name="negative_quadrant" fmla="at2 -1 -1"/>
<a:gd name="sin_negative" fmla="sin 10 -5400000"/>
<a:gd name="sin_multi_turn" fmla="sin 10 16200000"/>
<a:gd name="cos_multi_turn" fmla="cos 10 21600000"/>
</a:gdLst><a:pathLst><a:path w="10" h="10"><a:moveTo><a:pt x="0" y="0"/></a:moveTo></a:path></a:pathLst>
</a:custGeom></p:spPr></p:sp>"#;
    let package = MinimalPptx::new(shape).build();
    let presentation = PptxParser::parse_bytes(&package).expect("parse signed angles");
    let ShapeType::CustomGeom(geometry) = &presentation.slides[0].shapes[0].shape_type else {
        panic!("signed angles must render directly")
    };
    let values = geometry
        .guides
        .iter()
        .map(|guide| *guide.evaluation.as_ref().expect("finite angle result"))
        .collect::<Vec<_>>();
    assert!((values[0] - -8_100_000.0).abs() < 1e-6);
    assert!((values[1] - -10.0).abs() < 1e-6);
    assert!((values[2] - -10.0).abs() < 1e-6);
    assert!((values[3] - 10.0).abs() < 1e-6);
}

#[test]
fn unresolved_custom_geometry_tokens_preserve_typed_locations_and_fallback() {
    let cases = [
        (
            "a:pt",
            "x",
            "missingPoint",
            r#"<a:pathLst><a:path w="10" h="10"><a:moveTo><a:pt x="missingPoint" y="0"/></a:moveTo></a:path></a:pathLst>"#,
        ),
        (
            "a:rect",
            "l",
            "missingRect",
            r#"<a:rect l="missingRect" t="0" r="10" b="10"/><a:pathLst><a:path w="10" h="10"/></a:pathLst>"#,
        ),
        (
            "a:arcTo",
            "wR",
            "missingArcStart",
            r#"<a:pathLst><a:path w="10" h="10"><a:arcTo wR="missingArcStart" hR="1" stAng="0" swAng="5400000"></a:arcTo></a:path></a:pathLst>"#,
        ),
        (
            "a:arcTo",
            "wR",
            "missingArcEmpty",
            r#"<a:pathLst><a:path w="10" h="10"><a:arcTo wR="missingArcEmpty" hR="1" stAng="0" swAng="5400000"/></a:path></a:pathLst>"#,
        ),
        (
            "a:ahXY",
            "minX",
            "missingHandle",
            r#"<a:ahLst><a:ahXY minX="missingHandle"><a:pos x="0" y="0"/></a:ahXY></a:ahLst><a:pathLst><a:path w="10" h="10"/></a:pathLst>"#,
        ),
        (
            "a:cxn",
            "ang",
            "missingConnection",
            r#"<a:cxnLst><a:cxn ang="missingConnection"><a:pos x="0" y="0"/></a:cxn></a:cxnLst><a:pathLst><a:path w="10" h="10"/></a:pathLst>"#,
        ),
        (
            "a:pos",
            "x",
            "missingPosition",
            r#"<a:ahLst><a:ahXY><a:pos x="missingPosition" y="0"/></a:ahXY></a:ahLst><a:pathLst><a:path w="10" h="10"/></a:pathLst>"#,
        ),
        (
            "a:ahPolar",
            "minR",
            "missingPolar",
            r#"<a:ahLst><a:ahPolar minR="missingPolar"><a:pos x="0" y="0"/></a:ahPolar></a:ahLst><a:pathLst><a:path w="10" h="10"/></a:pathLst>"#,
        ),
    ];
    for (index, (element, attribute, token, body)) in cases.iter().enumerate() {
        let shape = format!(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="{}" name="{token}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:custGeom><a:avLst/><a:gdLst/>{body}</a:custGeom></p:spPr></p:sp>"#,
            index + 2
        );
        let package = MinimalPptx::new(&shape).build();
        let presentation = PptxParser::parse_bytes(&package).expect("parse unresolved token");
        let ShapeType::Unsupported(data) = &presentation.slides[0].shapes[0].shape_type else {
            panic!("provided unresolved token must force typed fallback")
        };
        let geometry = data
            .custom_geometry
            .as_ref()
            .expect("typed custom geometry");
        assert_eq!(geometry.issues.len(), 1);
        let issue = &geometry.issues[0];
        assert_eq!(issue.element, *element);
        assert_eq!(issue.attribute, *attribute);
        assert_eq!(issue.token, *token);
        assert_eq!(
            issue.error,
            GuideFormulaError::UnresolvedToken((*token).to_owned())
        );
        let result = convert_bytes_with_metadata(&package).expect("render typed fallback");
        let diagnostic = result
            .diagnostics()
            .iter()
            .find(|item| item.code == "DRAWINGML_CUSTOM_GEOMETRY_FALLBACK")
            .expect("custom geometry diagnostic");
        assert!(
            diagnostic
                .raw_reference
                .as_deref()
                .is_some_and(|raw| raw.contains(token))
        );
        let unresolved = result
            .unresolved_elements
            .iter()
            .find(|item| {
                matches!(
                    item.element_type,
                    pptx2html_core::model::UnresolvedType::CustomGeometry
                )
            })
            .expect("custom geometry unresolved element");
        assert!(unresolved.data_model.as_deref().is_some_and(|json| {
            json.contains(element) && json.contains(attribute) && json.contains(token)
        }));
        assert!(!result.html.contains("NaN") && !result.html.contains("Infinity"));
    }
}

#[test]
fn nonstandard_predefined_names_remain_typed_unresolved_tokens() {
    for token in ["hd10", "ssd3"] {
        let formula = format!("val {token}");
        let shape = format!(
            r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="{token}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="457200"/></a:xfrm><a:custGeom><a:avLst/><a:gdLst><a:gd name="bad" fmla="{formula}"/></a:gdLst><a:pathLst><a:path w="10" h="10"/></a:pathLst></a:custGeom></p:spPr></p:sp>"#
        );
        let package = MinimalPptx::new(&shape).build();
        let presentation = PptxParser::parse_bytes(&package).expect("parse nonstandard guide");
        let ShapeType::Unsupported(data) = &presentation.slides[0].shapes[0].shape_type else {
            panic!("nonstandard predefined name must fall back")
        };
        let guide = &data
            .custom_geometry
            .as_ref()
            .expect("typed geometry")
            .guides[0];
        assert_eq!(guide.raw_formula, formula);
        assert_eq!(
            guide.evaluation,
            Err(GuideFormulaError::UnresolvedToken(token.to_owned()))
        );
    }
}

#[test]
fn negative_sqrt_is_a_typed_domain_fallback_with_exact_formula() {
    const FORMULA: &str = "sqrt -1";
    let shape = format!(
        r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Negative sqrt"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:custGeom><a:avLst/><a:gdLst><a:gd name="badRoot" fmla="{FORMULA}"/></a:gdLst><a:pathLst><a:path w="10" h="10"><a:moveTo><a:pt x="badRoot" y="0"/></a:moveTo></a:path></a:pathLst></a:custGeom></p:spPr></p:sp>"#
    );
    let package = MinimalPptx::new(&shape).build();
    let presentation = PptxParser::parse_bytes(&package).expect("parse negative sqrt");
    let ShapeType::Unsupported(data) = &presentation.slides[0].shapes[0].shape_type else {
        panic!("negative sqrt must force fallback")
    };
    let guide = &data
        .custom_geometry
        .as_ref()
        .expect("typed geometry")
        .guides[0];
    assert_eq!(guide.name, "badRoot");
    assert_eq!(guide.raw_formula, FORMULA);
    assert_eq!(
        guide.evaluation,
        Err(GuideFormulaError::DomainError {
            operator: "sqrt".to_owned(),
            operand: "-1".to_owned(),
        })
    );
    let result = convert_bytes_with_metadata(&package).expect("render negative sqrt fallback");
    assert_eq!(
        result.diagnostics()[0].raw_reference.as_deref(),
        Some(FORMULA)
    );
}

#[test]
fn multiple_invalid_guides_preserve_order_and_typed_metadata() {
    let shape = r#"<p:sp><p:nvSpPr><p:cNvPr id="2" name="Two invalid guides"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr><p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="914400" cy="914400"/></a:xfrm><a:custGeom><a:avLst/><a:gdLst><a:gd name="first" fmla="unknownOp 1 2"/><a:gd name="second" fmla="sqrt -1"/></a:gdLst><a:pathLst><a:path w="10" h="10"/></a:pathLst></a:custGeom></p:spPr></p:sp>"#;
    let package = MinimalPptx::new(shape).build();
    let presentation = PptxParser::parse_bytes(&package).expect("parse two invalid guides");
    let ShapeType::Unsupported(data) = &presentation.slides[0].shapes[0].shape_type else {
        panic!("invalid guides must force fallback")
    };
    let guides = &data
        .custom_geometry
        .as_ref()
        .expect("typed geometry")
        .guides;
    assert_eq!(guides.len(), 2);
    assert_eq!(
        (&guides[0].name, &guides[0].raw_formula),
        (&"first".to_owned(), &"unknownOp 1 2".to_owned())
    );
    assert_eq!(
        (&guides[1].name, &guides[1].raw_formula),
        (&"second".to_owned(), &"sqrt -1".to_owned())
    );
    assert!(matches!(
        guides[0].evaluation,
        Err(GuideFormulaError::UnknownOperator(_))
    ));
    assert!(matches!(
        guides[1].evaluation,
        Err(GuideFormulaError::DomainError { .. })
    ));
    let result = convert_bytes_with_metadata(&package).expect("render two-guide fallback");
    let raw = result.diagnostics()[0]
        .raw_reference
        .as_deref()
        .expect("ordered reference");
    assert!(raw.find("first").expect("first guide") < raw.find("second").expect("second guide"));
    assert!(raw.contains("unknownOp 1 2") && raw.contains("sqrt -1"));
    let model = result.unresolved_elements[0]
        .data_model
        .as_deref()
        .expect("typed data model");
    assert!(model.contains("UnknownOperator") && model.contains("DomainError"));
}

#[test]
fn public_custom_guide_types_expose_typed_failure_contract() {
    let guide = CustomGuide {
        name: "unknownGuide".to_owned(),
        raw_formula: UNKNOWN_FORMULA.to_owned(),
        evaluation: Err(GuideFormulaError::UnknownOperator("unknownOp".to_owned())),
    };
    assert_eq!(guide.raw_formula, UNKNOWN_FORMULA);
    assert_eq!(
        guide.evaluation,
        Err(GuideFormulaError::UnknownOperator("unknownOp".to_owned()))
    );
}

#[test]
fn every_remaining_official_default_renders_finite_svg() {
    assert_eq!(REMAINING_PRESETS.len(), 63);
    for preset in REMAINING_PRESETS {
        let html = render_preset(preset, "");
        assert!(html.contains("<path "), "{preset}: no SVG path");
        assert!(!html.contains("NaN"), "{preset}: NaN");
        assert!(!html.contains("Infinity"), "{preset}: Infinity");
    }
}

#[test]
fn every_remaining_adjustment_covers_default_bounds_representative_and_hostile_values() {
    let cases = adjustment_cases();
    assert_eq!(cases.len(), 65);
    assert_eq!(
        cases
            .iter()
            .map(|case| (case.preset, case.key))
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        65,
        "duplicate preset-key"
    );
    for case in cases {
        let absent = render_preset(case.preset, "");
        let explicit = render_adjustment(case.preset, case.key, case.default);
        assert_eq!(
            path_data(&absent),
            path_data(&explicit),
            "{}.{} default",
            case.preset,
            case.key
        );
        let values = [
            case.lower,
            (case.lower + case.upper) / 2.0,
            case.upper,
            case.default * 0.75 + case.upper * 0.25,
        ];
        let variants = values.map(|value| render_adjustment(case.preset, case.key, value));
        for html in &variants {
            assert!(
                !html.contains("NaN") && !html.contains("Infinity"),
                "{}.{}",
                case.preset,
                case.key
            );
            assert!(
                !path_data(html).is_empty(),
                "{}.{}: no path",
                case.preset,
                case.key
            );
        }
        assert!(
            variants
                .windows(2)
                .any(|pair| path_data(&pair[0]) != path_data(&pair[1])),
            "{}.{}: adjustment ignored",
            case.preset,
            case.key
        );
        for hostile in [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MAX,
            -f64::MAX,
        ] {
            let html = render_adjustment(case.preset, case.key, hostile);
            assert!(
                !html.contains("NaN") && !html.contains("Infinity"),
                "{}.{}",
                case.preset,
                case.key
            );
        }
    }
}

#[test]
fn layered_remaining_presets_preserve_path_fill_and_stroke_roles() {
    for preset in [
        "actionButtonHome",
        "actionButtonInformation",
        "chartPlus",
        "chartStar",
        "chartX",
    ] {
        let html = render_preset(preset, "");
        assert!(
            html.matches("<path ").count() >= 2,
            "{preset}: layered paths"
        );
        assert!(html.contains("fill=\"none\""), "{preset}: no-fill role");
        assert!(html.contains("stroke=\""), "{preset}: stroke role");
    }
}

#[test]
fn plus_aliases_and_math_plus_keep_adjustment_keys_isolated() {
    for preset in ["plus", "cross"] {
        let default = render_preset(preset, "");
        let foreign = render_preset(preset, r#"<a:gd name="adj1" fmla="val 10000"/>"#);
        assert_eq!(path_data(&default), path_data(&foreign), "{preset}.adj1");
    }
    let default = render_preset("mathPlus", "");
    let adjusted = render_preset("mathPlus", r#"<a:gd name="adj1" fmla="val 10000"/>"#);
    assert_ne!(path_data(&default), path_data(&adjusted));
}

fn render_preset(preset: &str, adjustment_xml: &str) -> String {
    let shape = format!(
        r#"<p:sp>
  <p:nvSpPr><p:cNvPr id="2" name="{preset}"/><p:cNvSpPr/><p:nvPr/></p:nvSpPr>
  <p:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="1524000" cy="952500"/></a:xfrm>
    <a:prstGeom prst="{preset}"><a:avLst>{adjustment_xml}</a:avLst></a:prstGeom>
    <a:solidFill><a:srgbClr val="336699"/></a:solidFill>
    <a:ln><a:solidFill><a:srgbClr val="112233"/></a:solidFill></a:ln>
  </p:spPr>
</p:sp>"#
    );
    convert_bytes(&MinimalPptx::new(&shape).build()).expect("convert remaining preset")
}

fn render_adjustment(preset: &str, key: &str, value: f64) -> String {
    render_preset(
        preset,
        &format!(r#"<a:gd name="{key}" fmla="val {value}"/>"#),
    )
}

fn adjustment_cases() -> Vec<AdjustmentCase<'static>> {
    include_str!("fixtures/remaining_adjustment_contract.tsv")
        .lines()
        .filter(|line| !line.starts_with('#') && !line.starts_with("preset\t"))
        .map(|line| {
            let columns = line.split('\t').collect::<Vec<_>>();
            assert_eq!(columns.len(), 5, "contract row: {line}");
            AdjustmentCase {
                preset: columns[0],
                key: columns[1],
                default: columns[2].parse().expect("default"),
                lower: columns[3].parse().expect("lower"),
                upper: columns[4].parse().expect("upper"),
            }
        })
        .collect()
}

fn path_data(html: &str) -> Vec<&str> {
    html.match_indices("<path ")
        .map(|(start, _)| {
            let element = &html[start..html[start..].find('>').expect("path end") + start];
            let value_start = element.find("d=\"").expect("path d") + 3;
            let value_end = element[value_start..].find('"').expect("path d end") + value_start;
            &element[value_start..value_end]
        })
        .collect()
}
