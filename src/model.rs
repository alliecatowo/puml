use std::collections::BTreeSet;

use crate::ast::{ClassMember, DiagramKind, NoteKind};
use crate::diagnostic::Diagnostic;
use crate::scene::TextOverflowPolicy;
use crate::source::Span;
use crate::sprites::SpriteRegistry;
use crate::theme::{
    ActivityStyle, ChartStyle, ClassStyle, ComponentStyle, SequenceStyle, StateStyle, TimingStyle,
};

/// How to scale (or fix the size of) the output SVG.
#[derive(Debug, Clone, PartialEq)]
pub enum ScaleSpec {
    /// Multiply both width and height by this factor (e.g. `scale 1.5`).
    Factor(f64),
    /// Render to exactly this pixel size, preserving aspect via viewBox
    /// (e.g. `scale 800*600`).
    Fixed { width: u32, height: u32 },
    /// Cap the larger dimension at this pixel size (e.g. `scale max 800`).
    Max(u32),
}

/// Horizontal positioning of the legend box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LegendHAlign {
    Left,
    #[default]
    Center,
    Right,
}

/// Vertical positioning of the legend box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LegendVAlign {
    #[default]
    Bottom,
    Top,
}

// ─── State diagram model ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct StateDocument {
    pub kind: DiagramKind,
    pub nodes: Vec<StateNode>,
    pub transitions: Vec<StateTransition>,
    pub title: Option<String>,
    pub header: Option<String>,
    pub footer: Option<String>,
    pub caption: Option<String>,
    pub legend: Option<String>,
    pub state_style: StateStyle,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct StateNode {
    pub name: String,
    pub display: Option<String>,
    pub kind: StateNodeKind,
    pub stereotype: Option<String>,
    pub style: StateNodeStyle,
    pub internal_actions: Vec<StateInternalAction>,
    /// For composite states: children per region (concurrent → multiple vecs)
    pub regions: Vec<Vec<StateNode>>,
}

#[derive(Debug, Clone, Default)]
pub struct StateNodeStyle {
    pub fill_color: Option<String>,
    pub border_color: Option<String>,
    pub border_dashed: bool,
    pub border_thickness: Option<u8>,
    pub text_color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateNodeKind {
    /// A regular named state (rounded rectangle)
    Normal,
    /// `[*]` initial / final pseudo-state
    StartEnd,
    /// `[H]` shallow history
    HistoryShallow,
    /// `[H*]` deep history
    HistoryDeep,
    /// `<<fork>>` / `<<join>>` stereotype — thick horizontal bar
    Fork,
    Join,
    /// `<<choice>>` stereotype — diamond
    Choice,
    /// `<<end>>` stereotype — filled circle
    End,
    /// `<<entryPoint>>` / `<<exitPoint>>` boundary pseudo-states
    EntryPoint,
    ExitPoint,
    /// `<<inputPin>>` / `<<outputPin>>` pin pseudo-states
    InputPin,
    OutputPin,
    /// `<<expansionInput>>` / `<<expansionOutput>>` expansion port pseudo-states
    ExpansionInput,
    ExpansionOutput,
    /// Attached or floating state note
    Note,
    /// Inline `json $alias { ... }` projection in a state diagram
    JsonProjection,
}

#[derive(Debug, Clone)]
pub struct StateInternalAction {
    pub kind: String, // "entry", "exit", or event name
    pub action: String,
}

#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub line_color: Option<String>,
    pub dashed: bool,
    pub hidden: bool,
    pub thickness: Option<u8>,
    pub direction: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum NormalizedDocument {
    Sequence(SequenceDocument),
    Family(FamilyDocument),
    Timeline(TimelineDocument),
    State(StateDocument),
    Json(JsonDocument),
    Yaml(YamlDocument),
    Nwdiag(NwdiagDocument),
    Archimate(ArchimateDocument),
    Regex(RegexDocument),
    Ebnf(EbnfDocument),
    Math(MathDocument),
    Sdl(SdlDocument),
    Ditaa(DitaaDocument),
    Chart(ChartDocument),
}

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
    pub scale: Option<String>,
    pub scale_options: Vec<String>,
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
    pub is_deleted: bool,
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
}

#[derive(Debug, Clone)]
pub struct JsonProjection {
    pub alias: String,
    pub body: String,
    pub format: String,
}

/// Per-family style overrides carried through the model.
#[derive(Debug, Clone, PartialEq)]
pub enum FamilyStyle {
    Class(ClassStyle),
    State(StateStyle),
    Component(ComponentStyle),
    Activity(ActivityStyle),
    Timing(TimingStyle),
}

/// One page after a `newpage` directive in class/object/usecase diagrams.
#[derive(Debug, Clone)]
pub struct FamilyPage {
    pub title: Option<String>,
    pub nodes: Vec<FamilyNode>,
    pub relations: Vec<FamilyRelation>,
    pub groups: Vec<FamilyGroup>,
}

#[derive(Debug, Clone)]
pub struct FamilyDocument {
    pub kind: DiagramKind,
    pub nodes: Vec<FamilyNode>,
    pub relations: Vec<FamilyRelation>,
    pub groups: Vec<FamilyGroup>,
    pub pages: Vec<FamilyPage>,
    pub json_projections: Vec<JsonProjection>,
    pub hide_options: BTreeSet<String>,
    pub namespace_separator: Option<String>,
    pub title: Option<String>,
    pub header: Option<String>,
    pub footer: Option<String>,
    pub caption: Option<String>,
    pub legend: Option<String>,
    pub orientation: FamilyOrientation,
    pub style: SequenceStyle,
    /// Family-specific style overrides (class/state/component/activity).
    pub family_style: Option<FamilyStyle>,
    pub text_overflow_policy: TextOverflowPolicy,
    /// MindMap/WBS: auto word-wrap node labels at this pixel width (`skinparam MaximumWidth`).
    pub maximum_width: Option<i32>,
    pub sprites: SpriteRegistry,
    pub list_sprites: bool,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilyOrientation {
    TopToBottom,
    LeftToRight,
    BottomToTop,
    RightToLeft,
}

impl FamilyOrientation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TopToBottom => "TopToBottom",
            Self::LeftToRight => "LeftToRight",
            Self::BottomToTop => "BottomToTop",
            Self::RightToLeft => "RightToLeft",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FamilyGroup {
    pub kind: String,
    pub label: Option<String>,
    pub member_ids: Vec<String>,
}

/// Side placement for MindMap nodes (left/right of root).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MindMapSide {
    /// Determined by context / `left side` keyword (default = right)
    #[default]
    Right,
    Left,
}

/// Checkbox / progress annotation for WBS nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WbsCheckbox {
    /// `[x]` — checked
    Checked,
    /// `[ ]` — unchecked
    Unchecked,
    /// `[%NN]` — progress percentage (0–100)
    Progress(u8),
}

#[derive(Debug, Clone)]
pub struct FamilyNode {
    pub kind: FamilyNodeKind,
    pub name: String,
    pub alias: Option<String>,
    pub members: Vec<ClassMember>,
    pub depth: usize,
    pub label: Option<String>,
    /// MindMap side (only meaningful for MindMap kind)
    pub mindmap_side: MindMapSide,
    /// WBS checkbox annotation (only meaningful for Wbs kind)
    pub wbs_checkbox: Option<WbsCheckbox>,
    /// Optional per-node fill color for MindMap/WBS color tags.
    pub fill_color: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilyNodeKind {
    Class,
    Object,
    Diamond,
    Map,
    UseCase,
    Salt,
    MindMap,
    Wbs,
    // Component-family
    Component,
    Interface,
    Port,
    // Deployment-family
    Action,
    Agent,
    Node,
    Artifact,
    Boundary,
    Cloud,
    Circle,
    Collections,
    Frame,
    Storage,
    Container,
    Control,
    Database,
    Entity,
    Package,
    Rectangle,
    Folder,
    File,
    Card,
    Actor,
    BusinessActor,
    BusinessUseCase,
    Hexagon,
    Label,
    Person,
    Process,
    Queue,
    Stack,
    UseCaseDeployment,
    // State family
    State,
    StateInitial,
    StateFinal,
    StateHistory,
    // Activity family
    ActivityStart,
    ActivityStop,
    ActivityAction,
    ActivityDecision,
    ActivityFork,
    ActivityForkEnd,
    ActivityMerge,
    ActivityPartition,
    Note,
    // Timing family
    TimingConcise,
    TimingRobust,
    TimingClock,
    TimingBinary,
    TimingEvent,
    // C4 family
    C4Person,
    C4PersonExt,
    C4System,
    C4SystemExt,
    C4SystemDb,
    C4SystemQueue,
    C4Container,
    C4ContainerExt,
    C4ContainerDb,
    C4ContainerQueue,
    C4Component,
    C4ComponentExt,
    C4ComponentDb,
    C4ComponentQueue,
    C4Boundary,
}

#[derive(Debug, Clone)]
pub struct FamilyRelation {
    pub from: String,
    pub to: String,
    pub arrow: String,
    pub label: Option<String>,
    pub stereotype: Option<String>,
    pub left_cardinality: Option<String>,
    pub right_cardinality: Option<String>,
    pub left_role: Option<String>,
    pub right_role: Option<String>,
    pub line_color: Option<String>,
    pub dashed: bool,
    pub hidden: bool,
    pub thickness: Option<u8>,
    pub direction: Option<String>,
    pub left_lollipop: bool,
    pub right_lollipop: bool,
}

#[derive(Debug, Clone)]
pub struct SequenceDocument {
    pub participants: Vec<Participant>,
    pub participant_groups: Vec<SequenceParticipantGroup>,
    pub events: Vec<SequenceEvent>,
    pub teoz: bool,
    pub title: Option<String>,
    pub header: Option<String>,
    pub footer: Option<String>,
    pub caption: Option<String>,
    pub legend: Option<String>,
    pub skinparams: Vec<(String, String)>,
    pub style: SequenceStyle,
    pub footbox_visible: bool,
    pub scale: Option<ScaleSpec>,
    pub legend_halign: LegendHAlign,
    pub legend_valign: LegendVAlign,
    pub warnings: Vec<Diagnostic>,
    /// Whether `hide unlinked` was active for this document.
    pub hide_unlinked: bool,
    /// IDs of participants that were removed by the `hide unlinked` filter.
    pub hidden_participants: Vec<String>,
    pub sprites: SpriteRegistry,
    pub list_sprites: bool,
    /// Optional mainframe title (`mainframe <text>` keyword — feature 1.43).
    pub mainframe: Option<String>,
}

impl Default for SequenceDocument {
    fn default() -> Self {
        Self {
            participants: Vec::new(),
            participant_groups: Vec::new(),
            events: Vec::new(),
            teoz: false,
            title: None,
            header: None,
            footer: None,
            caption: None,
            legend: None,
            skinparams: Vec::new(),
            style: SequenceStyle::default(),
            footbox_visible: true,
            scale: None,
            legend_halign: LegendHAlign::default(),
            legend_valign: LegendVAlign::default(),
            warnings: Vec::new(),
            hide_unlinked: false,
            hidden_participants: Vec::new(),
            sprites: SpriteRegistry::new(),
            list_sprites: false,
            mainframe: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SequencePage {
    pub participants: Vec<Participant>,
    pub participant_groups: Vec<SequenceParticipantGroup>,
    pub events: Vec<SequenceEvent>,
    pub teoz: bool,
    pub title: Option<String>,
    pub header: Option<String>,
    pub footer: Option<String>,
    pub caption: Option<String>,
    pub legend: Option<String>,
    pub skinparams: Vec<(String, String)>,
    pub style: SequenceStyle,
    pub footbox_visible: bool,
    pub scale: Option<ScaleSpec>,
    pub legend_halign: LegendHAlign,
    pub legend_valign: LegendVAlign,
    pub warnings: Vec<Diagnostic>,
    /// Whether `hide unlinked` was active for this page.
    pub hide_unlinked: bool,
    /// IDs of participants that were removed by the `hide unlinked` filter.
    pub hidden_participants: Vec<String>,
    pub sprites: SpriteRegistry,
    pub list_sprites: bool,
    /// Optional mainframe title (`mainframe <text>` keyword — feature 1.43).
    pub mainframe: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Participant {
    pub id: String,
    pub display: String,
    pub role: ParticipantRole,
    pub explicit: bool,
}

#[derive(Debug, Clone)]
pub struct SequenceParticipantGroup {
    pub label: Option<String>,
    pub color: Option<String>,
    pub participant_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipantRole {
    Participant,
    Actor,
    Boundary,
    Control,
    Entity,
    Database,
    Collections,
    Queue,
}

#[derive(Debug, Clone)]
pub struct SequenceEvent {
    pub span: Span,
    pub kind: SequenceEventKind,
}

#[derive(Debug, Clone)]
pub enum SequenceEventKind {
    Message {
        from: String,
        to: String,
        arrow: String,
        label: Option<String>,
        style: SequenceMessageStyle,
        from_virtual: Option<VirtualEndpoint>,
        to_virtual: Option<VirtualEndpoint>,
    },
    Note {
        kind: NoteKind,
        position: String,
        target: Option<String>,
        text: String,
        /// When `true`, align this note at the same y level as the preceding note.
        aligned: bool,
    },
    GroupStart {
        kind: String,
        label: Option<String>,
    },
    GroupEnd,
    Delay(Option<String>),
    Divider(Option<String>),
    Separator(Option<String>),
    Spacer(Option<i32>),
    NewPage(Option<String>),
    Autonumber(Option<String>),
    Activate(String),
    Deactivate(String),
    Destroy(String),
    Create(String),
    Return {
        label: Option<String>,
        from: Option<String>,
        to: Option<String>,
    },
    IncludePlaceholder(String),
    DefinePlaceholder {
        name: String,
        value: Option<String>,
    },
    UndefPlaceholder(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SequenceMessageStyle {
    pub color: Option<String>,
    pub hidden: bool,
    pub dashed: bool,
    pub dotted: bool,
    pub thickness: Option<u8>,
    pub parallel: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualEndpoint {
    pub side: VirtualEndpointSide,
    pub kind: VirtualEndpointKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualEndpointSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualEndpointKind {
    Plain,
    Circle,
    Cross,
    Filled,
    /// Short arrow (`?->` / `->?`) — stub from the diagram edge (feature 1.30).
    Short,
}
