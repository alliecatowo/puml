use crate::ast::DiagramKind;
use crate::diagnostic::Diagnostic;
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

#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
pub enum NormalizedDocument {
    Sequence(SequenceDocument),
    Family(FamilyDocument),
    Timeline(TimelineDocument),
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
    pub title: Option<String>,
    pub header: Option<String>,
    pub footer: Option<String>,
    pub caption: Option<String>,
    pub legend: Option<String>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct FamilyNode {
    pub kind: FamilyNodeKind,
    pub name: String,
    pub alias: Option<String>,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FamilyNodeKind {
    Class,
    Object,
    UseCase,
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
