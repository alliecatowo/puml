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
    Gantt,
    Chronology,
    MindMap,
    Wbs,
    Component,
    Deployment,
    State,
    Activity,
    Timing,
    Json,
    Yaml,
    Nwdiag,
    Archimate,
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
    },
    GanttMilestoneDecl {
        name: String,
    },
    GanttConstraint {
        subject: String,
        kind: String,
        target: String,
    },
    ChronologyHappensOn {
        subject: String,
        when: String,
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
    /// Raw body line emitted verbatim from @startjson / @startyaml / @startnwdiag / @startarchimate.
    RawBody(String),
    /// A single row of a Salt wireframe grid.
    SaltGridRow {
        cells: Vec<SaltCell>,
    },
    Unknown(String),
}

/// A single cell within a Salt wireframe `{ ... }` grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaltCell {
    /// Bare text label.
    Label(String),
    /// `"text"` — an input field with placeholder text.
    Input(String),
    /// `[text]` — a button.
    Button(String),
    /// `^text^` — a combo-box / drop-down.
    Combo(String),
    /// `[X] text` — a checked checkbox.
    CheckboxChecked(String),
    /// `[ ] text` — an unchecked checkbox.
    CheckboxUnchecked(String),
    /// `(X) text` — a selected radio button.
    RadioSelected(String),
    /// `( ) text` — an unselected radio button.
    RadioUnselected(String),
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

#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub name: String,
    pub alias: Option<String>,
    pub members: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ObjectDecl {
    pub name: String,
    pub alias: Option<String>,
    pub members: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct UseCaseDecl {
    pub name: String,
    pub alias: Option<String>,
    pub members: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FamilyRelation {
    pub from: String,
    pub to: String,
    pub arrow: String,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParticipantDecl {
    pub role: ParticipantRole,
    pub name: String,
    pub alias: Option<String>,
    pub display: Option<String>,
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
    pub from_virtual: Option<VirtualEndpoint>,
    pub to_virtual: Option<VirtualEndpoint>,
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
    pub position: String,
    pub target: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub kind: String,
    pub label: Option<String>,
}
