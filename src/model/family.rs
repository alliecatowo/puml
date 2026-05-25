use std::collections::BTreeSet;

use crate::ast::{ClassMember, DiagramKind};
use crate::diagnostic::Diagnostic;
use crate::model::ScaleSpec;
use crate::scene::TextOverflowPolicy;
use crate::sprites::SpriteRegistry;
use crate::theme::{
    ActivityStyle, ClassStyle, ComponentStyle, MindMapStyle, SaltStyle, SequenceStyle, StateStyle,
    TimingStyle,
};

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
    MindMap(MindMapStyle),
    Salt(Box<SaltStyle>),
}

#[derive(Debug, Clone)]
pub struct FamilyDocument {
    pub kind: DiagramKind,
    pub nodes: Vec<FamilyNode>,
    pub relations: Vec<FamilyRelation>,
    pub groups: Vec<FamilyGroup>,
    pub json_projections: Vec<JsonProjection>,
    pub hide_options: BTreeSet<String>,
    pub namespace_separator: Option<String>,
    pub title: Option<String>,
    pub header: Option<String>,
    pub footer: Option<String>,
    pub caption: Option<String>,
    pub legend: Option<String>,
    /// Optional mainframe title (`mainframe <text>` common command).
    pub mainframe: Option<String>,
    /// Optional output scaling (`scale <factor>`, `scale <w>*<h>`, etc.).
    pub scale: Option<ScaleSpec>,
    pub orientation: FamilyOrientation,
    pub style: SequenceStyle,
    /// Family-specific style overrides (class/component/activity/tree families).
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
    Map,
    Diamond,
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
