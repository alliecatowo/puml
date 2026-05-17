use crate::source::Span;

#[derive(Debug, Clone)]
pub struct Document {
    pub kind: DiagramKind,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramKind {
    Sequence,
    Class,
    Object,
    UseCase,
    Salt,
    MindMap,
    Wbs,
    Gantt,
    Chronology,
    Component,
    Deployment,
    State,
    Activity,
    Timing,
    Json,
    Yaml,
    Nwdiag,
    Archimate,
    Regex,
    Ebnf,
    Math,
    Sdl,
    Ditaa,
    Chart,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Statement {
    pub span: Span,
    pub kind: StatementKind,
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    Participant(ParticipantDecl),
    Message(Message),
    ClassDecl(ClassDecl),
    ObjectDecl(ObjectDecl),
    UseCaseDecl(UseCaseDecl),
    FamilyRelation(FamilyRelation),
    StateDecl(StateDecl),
    StateTransition(StateTransition),
    StateInternalAction(StateInternalAction),
    StateRegionDivider,
    StateHistory {
        deep: bool,
    },
    GanttTaskDecl {
        name: String,
        start_date: Option<String>,
        duration_days: Option<u32>,
        depends_on: Vec<String>,
        resources: Vec<String>,
    },
    GanttMilestoneDecl {
        name: String,
        happens_on: Option<String>,
    },
    GanttConstraint {
        subject: String,
        kind: String,
        target: String,
    },
    GanttCalendarClosed {
        day: String,
    },
    GanttCalendarClosedDateRange {
        start_date: String,
        end_date: String,
    },
    ChronologyHappensOn {
        subject: String,
        when: String,
    },
    ComponentDecl {
        kind: ComponentNodeKind,
        name: String,
        alias: Option<String>,
        label: Option<String>,
        members: Vec<ClassMember>,
    },
    ActivityStep(ActivityStep),
    TimingDecl {
        kind: TimingDeclKind,
        name: String,
        label: Option<String>,
        controls: Vec<String>,
    },
    TimingEvent {
        time: String,
        signal: Option<String>,
        state: Option<String>,
        note: Option<String>,
    },
    Note(Note),
    Group(Group),
    Title(String),
    Header(String),
    Footer(String),
    Caption(String),
    Legend(String),
    SkinParam {
        key: String,
        value: String,
    },
    Theme(String),
    Pragma(String),
    Footbox(bool),
    Delay(Option<String>),
    Divider(Option<String>),
    Separator(Option<String>),
    Spacer,
    NewPage(Option<String>),
    IgnoreNewPage,
    Autonumber(Option<String>),
    Activate(String),
    Deactivate(String),
    Destroy(String),
    Create(String),
    Return(Option<String>),
    Include(String),
    Define {
        name: String,
        value: Option<String>,
    },
    Undef(String),
    RawBlockContent(String),
    RawBody(String),
    Scale(String),
    LegendPos(String),
    ClassGroup {
        kind: String,
        label: Option<String>,
        members: Vec<String>,
        relations: Vec<FamilyRelation>,
    },
    SetOption {
        key: String,
        value: String,
    },
    HideOption(String),
    HideUnlinked,
    /// `json $alias { ... }` inline block inside a `@startuml`/`@enduml` block.
    /// The body is the raw JSON text (everything between the outer braces).
    JsonProjection {
        alias: String,
        body: String,
    },
    /// `yaml $alias { ... }` inline block inside a `@startuml`/`@enduml` block.
    /// The body is the raw YAML-ish text (everything between the outer braces).
    YamlProjection {
        alias: String,
        body: String,
    },
    /// A row of cells in a `@startsalt` wireframe grid.
    SaltGridRow {
        cells: Vec<SaltCell>,
    },
    Unknown(String),
}

/// A single cell in a `@startsalt` wireframe grid row.
#[derive(Debug, Clone)]
pub enum SaltCell {
    /// Plain text label.
    Label(String),
    /// `"text"` — input field with placeholder text.
    Input(String),
    /// `[text]` — button.
    Button(String),
    /// `^text^` — combo box / dropdown.
    Combo(String),
    /// `[X] text` — checked checkbox.
    CheckboxChecked(String),
    /// `[ ] text` — unchecked checkbox.
    CheckboxUnchecked(String),
    /// `(X) text` — selected radio button.
    RadioOn(String),
    /// `( ) text` — unselected radio button.
    RadioOff(String),
}
/// A state declaration: `state Name` or `state Name { ... }` or `state Name <<stereotype>>`
#[derive(Debug, Clone)]
pub struct StateDecl {
    pub name: String,
    pub alias: Option<String>,
    pub stereotype: Option<String>,
    pub children: Vec<Statement>,
    pub region_dividers: Vec<usize>, // indices into children where `||` appeared
}

/// A state transition: `From --> To` or `From --> To : label`
#[derive(Debug, Clone)]
pub struct StateTransition {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
}

/// An internal action or entry/exit: `State : entry / action` or `State : exit / action`
/// or `State : event / action` (internal transition)
#[derive(Debug, Clone)]
pub struct StateInternalAction {
    pub state: String,
    pub kind: String, // "entry", "exit", or event name
    pub action: String,
}

/// Modifier on a class/object/usecase member, from `{field}`, `{method}`, `{abstract}`,
/// `{static}`, or `{class}` (alias for static).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum MemberModifier {
    Field,
    Method,
    Abstract,
    Static,
}

/// A single member line inside a class/object/usecase body block,
/// with an optional `{modifier}` annotation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ClassMember {
    /// The raw member text (visibility, name, type, etc.) without the modifier token.
    pub text: String,
    /// Optional modifier parsed from a trailing or leading `{field}`/`{method}`/
    /// `{abstract}`/`{static}`/`{class}` token, or from `<<abstract>>`/`<<static>>` stereotypes.
    pub modifier: Option<MemberModifier>,
}
#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub name: String,
    pub alias: Option<String>,
    pub members: Vec<ClassMember>,
}

#[derive(Debug, Clone)]
pub struct ObjectDecl {
    pub name: String,
    pub alias: Option<String>,
    pub members: Vec<ClassMember>,
}

#[derive(Debug, Clone)]
pub struct UseCaseDecl {
    pub name: String,
    pub alias: Option<String>,
    pub members: Vec<ClassMember>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentNodeKind {
    Component,
    Interface,
    Port,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimingDeclKind {
    Concise,
    Robust,
    Clock,
    Binary,
}

#[derive(Debug, Clone)]
pub struct ActivityStep {
    pub kind: ActivityStepKind,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActivityStepKind {
    Start,
    Stop,
    End,
    Action,
    IfStart,
    Else,
    EndIf,
    RepeatStart,
    RepeatWhile,
    WhileStart,
    EndWhile,
    Fork,
    ForkAgain,
    EndFork,
    PartitionStart,
    PartitionEnd,
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
    pub left_lollipop: bool,
    pub right_lollipop: bool,
}

#[derive(Debug, Clone)]
pub struct ParticipantDecl {
    pub role: ParticipantRole,
    pub name: String,
    pub alias: Option<String>,
    pub display: Option<String>,
    pub order: Option<i32>,
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
pub struct Message {
    pub from: String,
    pub to: String,
    pub arrow: String,
    pub label: Option<String>,
    pub style: MessageStyle,
    pub from_virtual: Option<VirtualEndpoint>,
    pub to_virtual: Option<VirtualEndpoint>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MessageStyle {
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
}

#[derive(Debug, Clone)]
pub struct Note {
    pub kind: NoteKind,
    pub position: String,
    pub target: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    Folded,
    Hexagonal,
    Rectangle,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub kind: String,
    pub label: Option<String>,
}
