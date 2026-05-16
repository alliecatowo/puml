use std::collections::BTreeSet;

use crate::ast::DiagramKind;
use crate::diagnostic::Diagnostic;
use crate::scene::TextOverflowPolicy;
use crate::source::Span;
use crate::theme::SequenceStyle;

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
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct StateNode {
    pub name: String,
    pub display: Option<String>,
    pub kind: StateNodeKind,
    pub internal_actions: Vec<StateInternalAction>,
    /// For composite states: children per region (concurrent → multiple vecs)
    pub regions: Vec<Vec<StateNode>>,
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
    pub title: Option<String>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct NwdiagNetwork {
    pub name: String,
    pub address: Option<String>,
    pub nodes: Vec<NwdiagNode>,
}

#[derive(Debug, Clone)]
pub struct NwdiagNode {
    pub name: String,
    pub address: Option<String>,
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
}

#[derive(Debug, Clone)]
pub struct ArchimateRelation {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub label: Option<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatKind {
    ZeroOrOne,
    ZeroOrMore,
    OneOrMore,
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
    pub subtype: ChartSubtype,
    pub data: Vec<ChartPoint>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartSubtype {
    Bar,
    Line,
    Pie,
}

#[derive(Debug, Clone)]
pub struct ChartPoint {
    pub label: String,
    pub value: f64,
}

#[derive(Debug, Clone)]
pub struct TimelineDocument {
    pub kind: DiagramKind,
    pub tasks: Vec<TimelineTask>,
    pub milestones: Vec<TimelineMilestone>,
    pub constraints: Vec<TimelineConstraint>,
    pub chronology_events: Vec<TimelineChronologyEvent>,
    pub title: Option<String>,
    pub header: Option<String>,
    pub footer: Option<String>,
    pub caption: Option<String>,
    pub legend: Option<String>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct TimelineTask {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct TimelineMilestone {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct TimelineConstraint {
    pub subject: String,
    pub kind: String,
    pub target: String,
}

#[derive(Debug, Clone)]
pub struct TimelineChronologyEvent {
    pub subject: String,
    pub when: String,
}

#[derive(Debug, Clone)]
pub struct FamilyDocument {
    pub kind: DiagramKind,
    pub nodes: Vec<FamilyNode>,
    pub relations: Vec<FamilyRelation>,
    pub groups: Vec<FamilyGroup>,
    pub hide_options: BTreeSet<String>,
    pub namespace_separator: Option<String>,
    pub title: Option<String>,
    pub header: Option<String>,
    pub footer: Option<String>,
    pub caption: Option<String>,
    pub legend: Option<String>,
    pub orientation: FamilyOrientation,
    pub style: SequenceStyle,
    pub text_overflow_policy: TextOverflowPolicy,
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

#[derive(Debug, Clone)]
pub struct FamilyNode {
    pub kind: FamilyNodeKind,
    pub name: String,
    pub alias: Option<String>,
    pub members: Vec<String>,
    pub depth: usize,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilyNodeKind {
    Class,
    Object,
    UseCase,
    Salt,
    MindMap,
    Wbs,
    // Component-family
    Component,
    Interface,
    Port,
    // Deployment-family
    Node,
    Artifact,
    Cloud,
    Frame,
    Storage,
    Database,
    Package,
    Rectangle,
    Folder,
    File,
    Card,
    Actor,
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
    // Timing family
    TimingConcise,
    TimingRobust,
    TimingClock,
    TimingBinary,
    TimingEvent,
}

#[derive(Debug, Clone)]
pub struct FamilyRelation {
    pub from: String,
    pub to: String,
    pub arrow: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SequenceDocument {
    pub participants: Vec<Participant>,
    pub events: Vec<SequenceEvent>,
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
}

impl Default for SequenceDocument {
    fn default() -> Self {
        Self {
            participants: Vec::new(),
            events: Vec::new(),
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
        }
    }
}

#[derive(Debug, Clone)]
pub struct SequencePage {
    pub participants: Vec<Participant>,
    pub events: Vec<SequenceEvent>,
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
}

#[derive(Debug, Clone)]
pub struct Participant {
    pub id: String,
    pub display: String,
    pub role: ParticipantRole,
    pub explicit: bool,
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
        from_virtual: Option<VirtualEndpoint>,
        to_virtual: Option<VirtualEndpoint>,
    },
    Note {
        position: String,
        target: Option<String>,
        text: String,
    },
    GroupStart {
        kind: String,
        label: Option<String>,
    },
    GroupEnd,
    Delay(Option<String>),
    Divider(Option<String>),
    Separator(Option<String>),
    Spacer,
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
}
