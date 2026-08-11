use super::fill::Fill;
#[cfg(test)]
use super::geometry::CustomGeometry;
use super::hierarchy::ClrMapOverride;

pub use super::bullet::{Bullet, BulletAutoNum, BulletChar};
pub use super::chart::{
    ChartBubbleSizeRepresents, ChartData, ChartDataLabelPosition, ChartDataLabelSettings,
    ChartGrouping, ChartMarkerSpec, ChartOfPieType, ChartRadarStyle, ChartScatterStyle,
    ChartSeries, ChartSpec, ChartSplitType, ChartType,
};
pub use super::preserved::{UnresolvedElement, UnresolvedType, UnsupportedData};
pub use super::shape::{ConnectionRef, CropRect, GroupData, PictureData, Shape, ShapeType};
pub use super::table::{TableCell, TableData, TableRow};
pub use super::text::{
    AutoFit, ParagraphDefRPr, TextBody, TextMargins, TextParagraph, TextRun, VerticalAlign,
};

/// Slide
#[derive(Debug, Clone)]
pub struct Slide {
    pub layout_idx: Option<usize>,
    pub shapes: Vec<Shape>,
    pub background: Option<Fill>,
    pub clr_map_ovr: Option<ClrMapOverride>,
    pub show_master_sp: bool,
    pub hidden: bool,
}

impl Default for Slide {
    fn default() -> Self {
        Self {
            layout_idx: None,
            shapes: Vec::new(),
            background: None,
            clr_map_ovr: None,
            show_master_sp: true,
            hidden: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AutoFit, Bullet, BulletAutoNum, BulletChar, ChartBubbleSizeRepresents, ChartData,
        ChartDataLabelPosition, ChartGrouping, ChartOfPieType, ChartRadarStyle, ChartScatterStyle,
        ChartSplitType, ChartType, CustomGeometry, GroupData, PictureData, Shape, ShapeType, Slide,
        TableCell, TableData, TextBody, TextMargins, UnresolvedType, UnsupportedData,
        VerticalAlign,
    };
    use crate::model::Color;
    use crate::model::style::Fill;

    fn approx_eq(left: f64, right: f64) {
        assert!((left - right).abs() < 1e-6, "expected {left} ≈ {right}");
    }

    #[test]
    fn table_cell_default_uses_ooxml_margin_defaults() {
        let cell = TableCell::default();

        assert!(cell.text_body.is_none());
        assert!(matches!(cell.fill, Fill::None));
        assert_eq!(cell.col_span, 0);
        assert_eq!(cell.row_span, 0);
        assert!(!cell.v_merge);
        approx_eq(cell.margin_left, 7.2);
        approx_eq(cell.margin_right, 7.2);
        approx_eq(cell.margin_top, 3.6);
        approx_eq(cell.margin_bottom, 3.6);
        assert!(matches!(cell.vertical_align, VerticalAlign::Top));
    }

    #[test]
    fn vertical_align_from_ooxml_maps_known_values_and_defaults_top() {
        assert!(matches!(
            VerticalAlign::from_ooxml("ctr"),
            VerticalAlign::Middle
        ));
        assert!(matches!(
            VerticalAlign::from_ooxml("b"),
            VerticalAlign::Bottom
        ));
        assert!(matches!(
            VerticalAlign::from_ooxml("unexpected"),
            VerticalAlign::Top
        ));
    }

    #[test]
    fn text_margins_default_match_ooxml_defaults() {
        let margins = TextMargins::default();

        approx_eq(margins.top, 3.6);
        approx_eq(margins.bottom, 3.6);
        approx_eq(margins.left, 7.2);
        approx_eq(margins.right, 7.2);
    }

    #[test]
    fn slide_and_text_body_defaults_match_expected_values() {
        let slide = Slide::default();
        assert!(slide.layout_idx.is_none());
        assert!(slide.shapes.is_empty());
        assert!(slide.background.is_none());
        assert!(slide.clr_map_ovr.is_none());
        assert!(slide.show_master_sp);
        assert!(!slide.hidden);

        let body = TextBody::default();
        assert!(body.paragraphs.is_empty());
        assert!(body.list_style.is_none());
        assert!(matches!(body.vertical_align, VerticalAlign::Top));
        assert!(!body.vertical_align_explicit);
        assert!(!body.anchor_center);
        approx_eq(body.text_rotation_deg, 0.0);
        assert!(body.word_wrap);
        assert!(!body.word_wrap_explicit);
        assert!(matches!(body.auto_fit, AutoFit::None));
    }

    #[test]
    fn slide_model_variants_can_be_constructed() {
        let _shape_types = vec![
            ShapeType::Rectangle,
            ShapeType::RoundedRectangle,
            ShapeType::Ellipse,
            ShapeType::Triangle,
            ShapeType::Arrow,
            ShapeType::Line,
            ShapeType::TextBox,
            ShapeType::Picture(PictureData::default()),
            ShapeType::Table(TableData::default()),
            ShapeType::Group(vec![Shape::default()], GroupData::default()),
            ShapeType::Chart(ChartData::default()),
            ShapeType::Custom("chevron".to_string()),
            ShapeType::CustomGeom(CustomGeometry {
                paths: Vec::new(),
                text_rect: None,
                adjust_handles: Vec::new(),
                connection_sites: Vec::new(),
            }),
            ShapeType::Unsupported(UnsupportedData {
                label: "SmartArt".to_string(),
                element_type: UnresolvedType::SmartArt,
                raw_xml: Some("<dgm/>".to_string()),
            }),
        ];

        let _chart_types = [
            ChartType::Column,
            ChartType::Bar,
            ChartType::Line,
            ChartType::Scatter,
            ChartType::Bubble,
            ChartType::Area,
            ChartType::Radar,
            ChartType::OfPie,
            ChartType::Pie,
            ChartType::Doughnut,
        ];
        let _of_pie_types = [ChartOfPieType::Pie, ChartOfPieType::Bar];
        let _split_types = [
            ChartSplitType::Auto,
            ChartSplitType::Pos,
            ChartSplitType::Percent,
            ChartSplitType::Value,
            ChartSplitType::Custom,
        ];
        let _radar_styles = [
            ChartRadarStyle::Standard,
            ChartRadarStyle::Marker,
            ChartRadarStyle::Filled,
        ];
        let _groupings = [
            ChartGrouping::Clustered,
            ChartGrouping::Stacked,
            ChartGrouping::PercentStacked,
            ChartGrouping::Standard,
        ];
        let _scatter_styles = [
            ChartScatterStyle::None,
            ChartScatterStyle::Line,
            ChartScatterStyle::LineMarker,
            ChartScatterStyle::Smooth,
            ChartScatterStyle::SmoothMarker,
            ChartScatterStyle::Marker,
        ];
        let _bubble_size_semantics = [
            ChartBubbleSizeRepresents::Area,
            ChartBubbleSizeRepresents::Width,
        ];
        let _label_positions = [
            ChartDataLabelPosition::Center,
            ChartDataLabelPosition::InEnd,
            ChartDataLabelPosition::OutEnd,
        ];
        let _auto_fit_modes = [
            AutoFit::None,
            AutoFit::NoAutoFit,
            AutoFit::Normal {
                font_scale: Some(0.8),
                line_spacing_reduction: Some(0.2),
            },
            AutoFit::Shrink,
        ];
        let _unresolved_types = [
            UnresolvedType::SmartArt,
            UnresolvedType::OleObject,
            UnresolvedType::MathEquation,
            UnresolvedType::CustomGeometry,
        ];
        let _bullets = [
            Bullet::Char(BulletChar {
                char: "•".to_string(),
                font: Some("Calibri".to_string()),
                size_pct: Some(1.0),
                color: Some(Color::rgb("FF0000")),
            }),
            Bullet::AutoNum(BulletAutoNum {
                num_type: "arabicPeriod".to_string(),
                start_at: Some(3),
                font: Some("Calibri".to_string()),
                size_pct: Some(0.8),
                color: Some(Color::theme("accent1")),
            }),
            Bullet::None,
        ];
    }
}
