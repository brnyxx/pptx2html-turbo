/// Chart data (fallback rendering for embedded charts)
#[derive(Debug, Clone, Default)]
pub struct ChartData {
    pub rel_id: String,
    pub preview_image: Option<Vec<u8>>,
    pub preview_mime: Option<String>,
    pub direct_spec: Option<ChartSpec>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChartType {
    #[default]
    Column,
    Bar,
    Line,
    Scatter,
    Bubble,
    Area,
    Radar,
    OfPie,
    Pie,
    Doughnut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChartOfPieType {
    #[default]
    Pie,
    Bar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChartSplitType {
    #[default]
    Auto,
    Pos,
    Percent,
    Value,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChartRadarStyle {
    #[default]
    Standard,
    Marker,
    Filled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChartGrouping {
    #[default]
    Clustered,
    Stacked,
    PercentStacked,
    Standard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChartScatterStyle {
    None,
    Line,
    LineMarker,
    Smooth,
    SmoothMarker,
    #[default]
    Marker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChartBubbleSizeRepresents {
    #[default]
    Area,
    Width,
}

#[derive(Debug, Clone, Default)]
pub struct ChartMarkerSpec {
    pub symbol: Option<String>,
    pub size: Option<i32>,
}

#[derive(Debug, Clone, Default)]
pub struct ChartDataLabelSettings {
    pub show_value: bool,
    pub show_category_name: bool,
    pub show_series_name: bool,
    pub show_percent: bool,
    pub position: Option<ChartDataLabelPosition>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChartDataLabelPosition {
    Center,
    InEnd,
    #[default]
    OutEnd,
}

#[derive(Debug, Clone, Default)]
pub struct ChartSeries {
    pub name: Option<String>,
    pub categories: Vec<String>,
    pub x_values: Vec<f64>,
    pub values: Vec<f64>,
    pub bubble_sizes: Vec<f64>,
    pub marker: Option<ChartMarkerSpec>,
}

#[derive(Debug, Clone, Default)]
pub struct ChartSpec {
    pub chart_type: ChartType,
    pub grouping: ChartGrouping,
    pub of_pie_type: Option<ChartOfPieType>,
    pub split_type: Option<ChartSplitType>,
    pub split_pos: Option<f64>,
    pub second_pie_size: Option<i32>,
    pub scatter_style: Option<ChartScatterStyle>,
    pub bubble_scale: Option<f64>,
    pub bubble_size_represents: Option<ChartBubbleSizeRepresents>,
    pub show_neg_bubbles: Option<bool>,
    pub radar_style: Option<ChartRadarStyle>,
    pub gap_width: Option<i32>,
    pub overlap: Option<i32>,
    pub hole_size: Option<i32>,
    pub category_axis_title: Option<String>,
    pub value_axis_title: Option<String>,
    pub data_labels: Option<ChartDataLabelSettings>,
    pub series: Vec<ChartSeries>,
}
