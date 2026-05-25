use crate::ast::DiagramKind;
use crate::diagnostic::Diagnostic;
use crate::model::{LegendHAlign, LegendVAlign};
use crate::theme::ChartStyle;

#[derive(Debug, Clone)]
pub struct JsonDocument {
    pub raw: String,
    pub nodes: Vec<JsonTreeNode>,
    pub title: Option<String>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct JsonTreeNode {
    pub depth: usize,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct YamlDocument {
    pub raw: String,
    pub nodes: Vec<YamlTreeNode>,
    pub title: Option<String>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct YamlTreeNode {
    pub depth: usize,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct NwdiagDocument {
    pub networks: Vec<NwdiagNetwork>,
    pub groups: Vec<NwdiagGroup>,
    pub peer_links: Vec<(String, String)>,
    pub top_level_nodes: Vec<NwdiagNode>,
    pub title: Option<String>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct NwdiagNetwork {
    pub name: String,
    pub address: Option<String>,
    pub label: Option<String>,
    pub color: Option<String>,
    pub shape: Option<String>,
    pub style: Option<String>,
    pub width_full: bool,
    pub nodes: Vec<NwdiagNode>,
}

#[derive(Debug, Clone)]
pub struct NwdiagNode {
    pub name: String,
    pub address: Option<String>,
    pub addresses: Vec<String>,
    pub label: Option<String>,
    pub color: Option<String>,
    pub shape: Option<String>,
    pub style: Option<String>,
    pub width: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct NwdiagGroup {
    pub name: String,
    pub label: Option<String>,
    pub color: Option<String>,
    pub shape: Option<String>,
    pub style: Option<String>,
    pub nodes: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ArchimateDocument {
    pub elements: Vec<ArchimateElement>,
    pub relations: Vec<ArchimateRelation>,
    pub title: Option<String>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct ArchimateElement {
    pub name: String,
    pub alias: Option<String>,
    pub layer: String,
    pub kind: String,
    pub fill: Option<String>,
    pub stroke: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ArchimateRelation {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub label: Option<String>,
    pub direction: Option<String>,
    pub style: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RegexDocument {
    pub title: Option<String>,
    pub patterns: Vec<RegexPattern>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct RegexPattern {
    pub source: String,
    pub tokens: Vec<RegexToken>,
}

#[derive(Debug, Clone)]
pub enum RegexToken {
    Literal(String),
    CharClass(String),
    Group(Vec<RegexToken>),
    Alt(Vec<Vec<RegexToken>>),
    Repeat {
        inner: Box<RegexToken>,
        kind: RepeatKind,
    },
    Escape(char),
    AnyChar,
    Anchor(String),
    Unsupported(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepeatKind {
    ZeroOrOne,
    ZeroOrMore,
    OneOrMore,
    Exact(u32),
    Range { min: Option<u32>, max: Option<u32> },
}

#[derive(Debug, Clone)]
pub struct EbnfDocument {
    pub title: Option<String>,
    pub rules: Vec<EbnfRule>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct EbnfRule {
    pub name: String,
    pub body: String,
    pub tokens: Vec<EbnfToken>,
}

#[derive(Debug, Clone)]
pub enum EbnfToken {
    Terminal(String),
    NonTerminal(String),
    Alt(Vec<Vec<EbnfToken>>),
    Group(Vec<EbnfToken>),
    Optional(Vec<EbnfToken>),
    Repetition(Vec<EbnfToken>),
    Repeat {
        inner: Box<EbnfToken>,
        kind: RepeatKind,
    },
    Unsupported(String),
}

#[derive(Debug, Clone)]
pub struct MathDocument {
    pub title: Option<String>,
    pub body: String,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct SdlDocument {
    pub title: Option<String>,
    pub states: Vec<SdlState>,
    pub transitions: Vec<SdlTransition>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct SdlState {
    pub name: String,
    pub kind: SdlStateKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SdlStateKind {
    Start,
    Input,
    Output,
    Decision,
    State,
    Stop,
}

#[derive(Debug, Clone)]
pub struct SdlTransition {
    pub from: String,
    pub to: String,
    pub signal: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DitaaDocument {
    pub title: Option<String>,
    pub body: String,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct BoardDocument {
    pub title: Option<String>,
    pub columns: Vec<BoardColumn>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct BoardColumn {
    pub title: String,
    pub cards: Vec<BoardCard>,
}

#[derive(Debug, Clone)]
pub struct BoardCard {
    pub depth: usize,
    pub title: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FilesDocument {
    pub title: Option<String>,
    pub roots: Vec<FileTreeNode>,
    pub top_notes: Vec<String>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct FileTreeNode {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub notes: Vec<String>,
    pub children: Vec<FileTreeNode>,
}

#[derive(Debug, Clone)]
pub struct ChartDocument {
    pub title: Option<String>,
    pub caption: Option<String>,
    pub subtype: ChartSubtype,
    pub data: Vec<ChartPoint>,
    pub h_axis: Option<ChartAxis>,
    pub v_axis: Option<ChartAxis>,
    pub series: Vec<ChartSeries>,
    pub legend: ChartLegend,
    pub palette: Vec<String>,
    pub annotations: Vec<ChartAnnotation>,
    pub label_mode: ChartLabelMode,
    pub horizontal: bool,
    pub stacked: bool,
    pub style: ChartStyle,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChartLabelMode {
    #[default]
    Auto,
    Inside,
    Outside,
    None,
    Value,
    Percent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartSubtype {
    Bar,
    Line,
    Pie,
    Area,
    Scatter,
}

#[derive(Debug, Clone)]
pub struct ChartPoint {
    pub label: String,
    pub value: f64,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ChartAxis {
    pub label: Option<String>,
    pub categories: Vec<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub tick_step: Option<f64>,
    pub color: Option<String>,
    pub label_color: Option<String>,
    pub grid_color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChartSeries {
    pub name: String,
    pub values: Vec<f64>,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChartLegend {
    pub visible: bool,
    pub explicit: bool,
    pub h_align: LegendHAlign,
    pub v_align: LegendVAlign,
    pub background_color: Option<String>,
    pub border_color: Option<String>,
    pub text_color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChartAnnotation {
    pub target: String,
    pub text: String,
}

impl Default for ChartLegend {
    fn default() -> Self {
        Self {
            visible: false,
            explicit: false,
            h_align: LegendHAlign::Right,
            v_align: LegendVAlign::Top,
            background_color: None,
            border_color: None,
            text_color: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimelineDocument {
    pub kind: DiagramKind,
    pub tasks: Vec<TimelineTask>,
    pub milestones: Vec<TimelineMilestone>,
    pub separators: Vec<TimelineSeparator>,
    pub constraints: Vec<TimelineConstraint>,
    pub chronology_events: Vec<TimelineChronologyEvent>,
    pub closed_weekdays: Vec<String>,
    pub closed_ranges: Vec<TimelineClosedRange>,
    pub open_ranges: Vec<TimelineOpenRange>,
    pub day_markers: Vec<TimelineDayMarker>,
    pub resource_off_ranges: Vec<TimelineResourceOffRange>,
    pub named_dates: Vec<TimelineNamedDate>,
    pub scale: Option<String>,
    pub scale_options: Vec<String>,
    pub print_start: Option<String>,
    pub print_end: Option<String>,
    pub print_start_day: Option<u32>,
    pub print_end_day: Option<u32>,
    pub project_start: Option<String>,
    pub project_start_day: Option<u32>,
    pub title: Option<String>,
    pub header: Option<String>,
    pub footer: Option<String>,
    pub caption: Option<String>,
    pub legend: Option<String>,
    pub notes: Vec<TimelineNote>,
    pub hide_footbox: bool,
    pub hide_resource_names: bool,
    pub hide_resource_footbox: bool,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct TimelineTask {
    pub name: String,
    pub alias: Option<String>,
    pub start_day: u32,
    pub workload_days: u32,
    pub duration_days: u32,
    pub resources: Vec<String>,
    pub resource_allocations: Vec<TimelineResourceAllocation>,
    pub baseline_start_day: Option<u32>,
    pub baseline_duration_days: Option<u32>,
    pub is_critical: bool,
    pub fill_color: Option<String>,
    pub stroke_color: Option<String>,
    pub completion_percent: Option<u32>,
    pub hyperlink: Option<String>,
    pub is_deleted: bool,
    pub pause_weekdays: Vec<String>,
    pub pause_ranges: Vec<TimelineTaskPauseRange>,
}

#[derive(Debug, Clone)]
pub struct TimelineNamedDate {
    pub date: String,
    pub label: String,
    pub day: u32,
}

#[derive(Debug, Clone)]
pub struct TimelineMilestone {
    pub name: String,
    pub happens_on: Option<String>,
    pub is_critical: bool,
}

#[derive(Debug, Clone)]
pub struct TimelineSeparator {
    pub label: String,
    pub target: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TimelineConstraint {
    pub subject: String,
    pub kind: String,
    pub target: String,
}

#[derive(Debug, Clone)]
pub struct TimelineClosedRange {
    pub start_date: String,
    pub end_date: String,
    pub start_day: u32,
    pub end_day: u32,
}

#[derive(Debug, Clone)]
pub struct TimelineOpenRange {
    pub start_date: String,
    pub end_date: String,
    pub start_day: u32,
    pub end_day: u32,
}

#[derive(Debug, Clone)]
pub struct TimelineDayMarker {
    pub start_date: String,
    pub end_date: String,
    pub start_day: u32,
    pub end_day: u32,
    pub label: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TimelineResourceOffRange {
    pub resource: String,
    pub start_date: String,
    pub end_date: String,
    pub start_day: u32,
    pub end_day: u32,
}

#[derive(Debug, Clone)]
pub struct TimelineTaskPauseRange {
    pub start_date: String,
    pub end_date: String,
    pub start_day: u32,
    pub end_day: u32,
}

#[derive(Debug, Clone)]
pub struct TimelineNote {
    pub target: Option<String>,
    pub position: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct TimelineResourceAllocation {
    pub name: String,
    pub load_percent: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct TimelineChronologyEvent {
    pub subject: String,
    pub when: String,
    pub end: Option<String>,
    pub color: Option<String>,
    pub bracket: bool,
}
