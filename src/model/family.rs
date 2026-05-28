use std::collections::BTreeSet;
use std::fmt;
use std::ops::Deref;

use crate::ast::{ClassMember, DiagramKind};
use crate::diagnostic::Diagnostic;
use crate::model::{MetadataHAlign, ScaleSpec};
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
    /// Horizontal alignment for the `header` text (`left header ...` / `center header ...`).
    pub header_align: MetadataHAlign,
    pub footer: Option<String>,
    /// Horizontal alignment for the `footer` text (`right footer ...` etc.).
    pub footer_align: MetadataHAlign,
    pub caption: Option<String>,
    pub legend: Option<String>,
    /// Horizontal alignment of the legend block (`legend left|center|right`).
    pub legend_halign: crate::model::LegendHAlign,
    /// Vertical alignment of the legend block (`legend top|bottom`).
    pub legend_valign: crate::model::LegendVAlign,
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
    pub arrow: FamilyRelationArrow,
    pub label: Option<String>,
    pub stereotype: Option<String>,
    pub left_cardinality: Option<String>,
    pub right_cardinality: Option<String>,
    pub left_role: Option<String>,
    pub right_role: Option<String>,
    pub line_color: Option<FamilyRelationColor>,
    pub dashed: bool,
    pub hidden: bool,
    pub thickness: Option<u8>,
    pub direction: Option<FamilyRelationDirection>,
    pub left_lollipop: bool,
    pub right_lollipop: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FamilyRelationDirection {
    Left,
    Right,
    Up,
    Down,
}

impl FamilyRelationDirection {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Up => "up",
            Self::Down => "down",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "left" | "l" => Some(Self::Left),
            "right" | "r" => Some(Self::Right),
            "up" | "u" => Some(Self::Up),
            "down" | "d" => Some(Self::Down),
            _ => None,
        }
    }
}

impl fmt::Display for FamilyRelationDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Deref for FamilyRelationDirection {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FamilyRelationColor(String);

impl FamilyRelationColor {
    pub fn parse(value: &str) -> Result<Self, String> {
        crate::theme::color::parse_relation_color_token(value)
            .map(Self)
            .ok_or_else(|| format!("invalid relation line color `{value}`"))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FamilyRelationColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Deref for FamilyRelationColor {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FamilyRelationEndpointMarker {
    Open,
    DoubleOpen,
    Triangle,
    DiamondFilled,
    DiamondOpen,
    CircleOpen,
    CircleFilled,
    TriangleFilled,
    BoxFilled,
    Plus,
    Slash,
    /// `x--` crossed/dead-end arrowhead.
    Cross,
    /// `}--` bracket-open arrowhead.
    BracketOpen,
    IeZeroMany,
    IeOneMany,
    IeZeroOne,
    IeOne,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FamilyRelationLineKind {
    Solid,
    Dashed,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FamilyRelationArrow {
    raw: String,
    line: FamilyRelationLineKind,
    start_marker: Option<FamilyRelationEndpointMarker>,
    end_marker: Option<FamilyRelationEndpointMarker>,
}

impl FamilyRelationArrow {
    pub fn parse(value: &str) -> Result<Self, String> {
        let raw = value.trim();
        if raw.len() < 2 || (!raw.contains('-') && !raw.contains('.')) {
            return Err(format!("invalid relation arrow `{value}`"));
        }
        Ok(Self {
            raw: raw.to_string(),
            line: if raw.contains("..") {
                FamilyRelationLineKind::Dashed
            } else {
                FamilyRelationLineKind::Solid
            },
            start_marker: relation_start_marker(raw),
            end_marker: relation_end_marker(raw),
        })
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }

    pub fn line_kind(&self) -> FamilyRelationLineKind {
        self.line
    }

    pub fn is_dashed(&self) -> bool {
        self.line == FamilyRelationLineKind::Dashed
    }

    pub fn start_marker(&self) -> Option<FamilyRelationEndpointMarker> {
        self.start_marker
    }

    pub fn end_marker(&self) -> Option<FamilyRelationEndpointMarker> {
        self.end_marker
    }

    pub fn with_endpoint_markers(&self, start: &str, end: &str) -> Result<Self, String> {
        Self::parse(&format!("{start}{}{end}", self.raw))
    }
}

impl fmt::Display for FamilyRelationArrow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Deref for FamilyRelationArrow {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl PartialEq<&str> for FamilyRelationArrow {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<FamilyRelationArrow> for &str {
    fn eq(&self, other: &FamilyRelationArrow) -> bool {
        *self == other.as_str()
    }
}

fn relation_start_marker(raw: &str) -> Option<FamilyRelationEndpointMarker> {
    if raw.starts_with("}o") || raw.starts_with("o{") {
        Some(FamilyRelationEndpointMarker::IeZeroMany)
    } else if raw.starts_with("}|") || raw.starts_with("|{") {
        Some(FamilyRelationEndpointMarker::IeOneMany)
    } else if raw.starts_with("|o") || raw.starts_with("o|") {
        Some(FamilyRelationEndpointMarker::IeZeroOne)
    } else if raw.starts_with("||") {
        Some(FamilyRelationEndpointMarker::IeOne)
    } else if raw.starts_with("<<") {
        Some(FamilyRelationEndpointMarker::DoubleOpen)
    } else if raw.starts_with("<|") {
        Some(FamilyRelationEndpointMarker::Triangle)
    } else {
        raw.chars().next().and_then(marker_from_endpoint_char)
    }
}

fn relation_end_marker(raw: &str) -> Option<FamilyRelationEndpointMarker> {
    if raw.ends_with("o{") || raw.ends_with("}o") {
        Some(FamilyRelationEndpointMarker::IeZeroMany)
    } else if raw.ends_with("|{") || raw.ends_with("}|") {
        Some(FamilyRelationEndpointMarker::IeOneMany)
    } else if raw.ends_with("o|") || raw.ends_with("|o") {
        Some(FamilyRelationEndpointMarker::IeZeroOne)
    } else if raw.ends_with("||") {
        Some(FamilyRelationEndpointMarker::IeOne)
    } else if raw.ends_with(">>") {
        Some(FamilyRelationEndpointMarker::DoubleOpen)
    } else if raw.ends_with("|>") {
        Some(FamilyRelationEndpointMarker::Triangle)
    } else {
        raw.chars().last().and_then(marker_from_endpoint_char)
    }
}

fn marker_from_endpoint_char(ch: char) -> Option<FamilyRelationEndpointMarker> {
    match ch {
        '<' | '>' => Some(FamilyRelationEndpointMarker::Open),
        '*' => Some(FamilyRelationEndpointMarker::DiamondFilled),
        'o' => Some(FamilyRelationEndpointMarker::DiamondOpen),
        '0' | '(' | ')' => Some(FamilyRelationEndpointMarker::CircleOpen),
        '@' => Some(FamilyRelationEndpointMarker::CircleFilled),
        '^' => Some(FamilyRelationEndpointMarker::TriangleFilled),
        '#' => Some(FamilyRelationEndpointMarker::BoxFilled),
        '+' => Some(FamilyRelationEndpointMarker::Plus),
        '\\' | '/' => Some(FamilyRelationEndpointMarker::Slash),
        // `x--` crossed/dead-end arrowhead (IE two-char patterns are checked first).
        'x' => Some(FamilyRelationEndpointMarker::Cross),
        // `}--` bracket-open arrowhead (IE two-char patterns `}o`, `}|` checked first).
        '}' => Some(FamilyRelationEndpointMarker::BracketOpen),
        _ => None,
    }
}
