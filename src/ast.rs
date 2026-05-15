use crate::source::Span;

#[derive(Debug, Clone)]
pub struct Document {
    pub kind: DiagramKind,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramKind {
    Sequence,
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
    Note(Note),
    Group(Group),
    Title(String),
    Header(String),
    Footer(String),
    Caption(String),
    Legend(String),
    SkinParam { key: String, value: String },
    Theme(String),
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
    Define { name: String, value: Option<String> },
    Undef(String),
    Unknown(String),
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
