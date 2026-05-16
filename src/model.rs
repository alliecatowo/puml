use crate::ast::DiagramKind;
use crate::diagnostic::Diagnostic;
use crate::source::Span;
use crate::theme::SequenceStyle;

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
    Salt(SaltDocument),
    Json(JsonDocument),
    Yaml(YamlDocument),
    Nwdiag(NwdiagDocument),
    Archimate(ArchimateDocument),
}

// ─── Salt wireframe model ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct SaltDocument {
    pub rows: Vec<SaltRow>,
    pub title: Option<String>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct SaltRow {
    pub cells: Vec<SaltCell>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaltCell {
    Label(String),
    Input(String),
    Button(String),
    Combo(String),
    CheckboxChecked(String),
    CheckboxUnchecked(String),
    RadioSelected(String),
    RadioUnselected(String),
}

// ─── JSON tree model ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct JsonDocument {
    pub nodes: Vec<JsonNode>,
    pub title: Option<String>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct JsonNode {
    pub depth: usize,
    pub key: Option<String>,
    pub value_type: JsonValueType,
    pub display: String,
    pub has_children: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonValueType {
    String,
    Number,
    Bool,
    Null,
    Object,
    Array,
}

// ─── YAML tree model ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct YamlDocument {
    pub nodes: Vec<YamlNode>,
    pub title: Option<String>,
    pub warnings: Vec<Diagnostic>,
}

#[derive(Debug, Clone)]
pub struct YamlNode {
    pub depth: usize,
    pub label: String,
    pub value_type: YamlValueType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YamlValueType {
    Key,
    StringValue,
    NumberValue,
    BoolValue,
    NullValue,
    Unknown,
}

// ─── nwdiag model ────────────────────────────────────────────────────────────

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

// ─── Archimate model ─────────────────────────────────────────────────────────

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
    pub layer: ArchimateLayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchimateLayer {
    Motivation,
    Strategy,
    Business,
    Application,
    Technology,
    Physical,
    Unknown,
}

impl ArchimateLayer {
    pub fn parse_layer(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "motivation" => Self::Motivation,
            "strategy" => Self::Strategy,
            "business"
            | "business-actor"
            | "business-role"
            | "business-process"
            | "business-function"
            | "business-service"
            | "business-object"
            | "business-interface"
            | "business-event"
            | "business-interaction"
            | "business-collaboration" => Self::Business,
            "application"
            | "application-component"
            | "application-service"
            | "application-function"
            | "application-interface"
            | "application-process"
            | "application-interaction"
            | "application-event"
            | "application-collaboration"
            | "application-data-object" => Self::Application,
            "technology"
            | "technology-service"
            | "technology-function"
            | "technology-interface"
            | "technology-process"
            | "technology-interaction"
            | "technology-event"
            | "technology-collaboration"
            | "node"
            | "device"
            | "system-software"
            | "network"
            | "communication-path"
            | "path"
            | "artifact" => Self::Technology,
            "physical"
            | "physical-equipment"
            | "physical-facility"
            | "physical-distribution-network"
            | "material" => Self::Physical,
            _ => Self::Business, // default to business layer for unknown stereotypes
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Motivation => "Motivation",
            Self::Strategy => "Strategy",
            Self::Business => "Business",
            Self::Application => "Application",
            Self::Technology => "Technology",
            Self::Physical => "Physical",
            Self::Unknown => "Unknown",
        }
    }

    pub fn bg_color(self) -> &'static str {
        match self {
            Self::Motivation => "#ede9fe",
            Self::Strategy => "#fee2e2",
            Self::Business => "#fef3c7",
            Self::Application => "#dbeafe",
            Self::Technology => "#dcfce7",
            Self::Physical => "#fce7f3",
            Self::Unknown => "#f1f5f9",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ArchimateRelation {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub label: Option<String>,
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
    pub warnings: Vec<Diagnostic>,
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
