use crate::ast::DiagramKind;
use crate::diagnostic::Diagnostic;
use crate::theme::StateStyle;

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
    pub hide_empty_description: bool,
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
