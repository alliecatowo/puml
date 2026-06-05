//! Typed AST surface for `<style>` blocks (Phase A of #1404).
//!
//! This module defines the full style-block AST that the recursive-descent
//! parser in `src/parser/style_block.rs` produces.  The cascade resolver
//! (Phase B) consumes these types.  The legacy `StatementKind::StyleParam`
//! flat-triple compat shim was removed in Phase E (#1417).

use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// SName — catalogue of upstream selector tag names (mirrors SName.java)
// ---------------------------------------------------------------------------

/// Identifier-level selector tags, mirroring upstream `SName.java`.
///
/// All variants that collide with Rust keywords are suffixed with `_`
/// (e.g. `Class_`, `Package_`, `Interface_`).  The `retrieve` function maps
/// case-insensitive raw strings to the correct variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SName {
    Action,
    ActivationBox,
    Activity,
    ActivityBar,
    ActivityDiagram,
    Actor,
    Agent,
    Analog,
    Archimate,
    Arrow,
    Artifact,
    Binary,
    Body,
    Boundary,
    Box,
    Boxless,
    Business,
    Caption,
    Card,
    Cardinality,
    Circle,
    ClassDiagram,
    Class_,
    Clickable,
    Cloud,
    Closed,
    Collection,
    Collections,
    Component,
    Composite,
    Robust,
    ChenAttribute,
    ChenEerDiagram,
    ChenEntity,
    ChenRelationship,
    Concise,
    Clock,
    ComponentDiagram,
    ConstraintArrow,
    Control,
    Database,
    Day,
    Delay,
    Description,
    Destroy,
    Diamond,
    Document,
    Ebnf,
    Element,
    Entity,
    End,
    Start,
    Stop,
    File,
    FilesDiagram,
    Folder,
    Footer,
    Frame,
    GanttDiagram,
    Generic,
    Goto_,
    Group,
    GroupHeader,
    Header,
    Hexagon,
    Highlight,
    Hnote,
    Interface_,
    Json,
    JsonDiagram,
    GitDiagram,
    Label,
    LeafNode,
    Legend,
    LifeLine,
    Mainframe,
    Map,
    Milestone,
    MindmapDiagram,
    Month,
    Name,
    Network,
    Newpage,
    Node,
    Note,
    NwdiagDiagram,
    PacketdiagDiagram,
    ObjectDiagram,
    Object,
    Package_,
    Participant,
    Partition,
    Person,
    Port,
    Process,
    Qualified,
    Queue,
    Rectangle,
    Reference,
    ReferenceHeader,
    Regex,
    Requirement,
    Rnote,
    Root,
    RootNode,
    SaltDiagram,
    Separator,
    SequenceDiagram,
    Server,
    Stack,
    StateDiagram,
    State,
    StateBody,
    Stereotype,
    Storage,
    Swimlane,
    Task,
    Timegrid,
    Timeline,
    TimingDiagram,
    Title,
    Undone,
    Unstarted,
    Usecase,
    VerticalSeparator,
    Year,
    // Visibility icons
    VisibilityIcon,
    Private_,
    Protected_,
    Public_,
    IeMandatory,
    Spot,
    SpotAnnotation,
    SpotInterface,
    SpotEnum,
    SpotProtocol,
    SpotStruct,
    SpotEntity,
    SpotException,
    SpotClass,
    SpotAbstractClass,
    SpotMetaClass,
    SpotStereotype,
    SpotDataClass,
    SpotRecord,
    // Diagram types
    WbsDiagram,
    YamlDiagram,
    ChartDiagram,
    // Chart elements
    Bar,
    Line,
    Area,
    Scatter,
    Axis,
    HAxis,
    VAxis,
    Grid,
    Annotation,
}

impl SName {
    /// Retrieve an `SName` from a raw string (case-insensitive, underscore-stripped).
    /// Returns `None` if the name is not recognised (becomes `SelectorSegment::Unknown`).
    pub fn retrieve(s: &str) -> Option<Self> {
        // Normalise: lower-case and strip underscores used in Java enum keyword escaping.
        let lower = s.to_ascii_lowercase();
        let normalised: String = lower.chars().filter(|&c| c != '_').collect();
        match normalised.as_str() {
            "action" => Some(Self::Action),
            "activationbox" => Some(Self::ActivationBox),
            "activity" => Some(Self::Activity),
            "activitybar" => Some(Self::ActivityBar),
            "activitydiagram" => Some(Self::ActivityDiagram),
            "actor" => Some(Self::Actor),
            "agent" => Some(Self::Agent),
            "analog" => Some(Self::Analog),
            "archimate" => Some(Self::Archimate),
            "arrow" => Some(Self::Arrow),
            "artifact" => Some(Self::Artifact),
            "binary" => Some(Self::Binary),
            "body" => Some(Self::Body),
            "boundary" => Some(Self::Boundary),
            "box" => Some(Self::Box),
            "boxless" => Some(Self::Boxless),
            "business" => Some(Self::Business),
            "caption" => Some(Self::Caption),
            "card" => Some(Self::Card),
            "cardinality" => Some(Self::Cardinality),
            "circle" => Some(Self::Circle),
            "classdiagram" => Some(Self::ClassDiagram),
            "class" => Some(Self::Class_),
            "clickable" => Some(Self::Clickable),
            "cloud" => Some(Self::Cloud),
            "closed" => Some(Self::Closed),
            "collection" => Some(Self::Collection),
            "collections" => Some(Self::Collections),
            "component" => Some(Self::Component),
            "composite" => Some(Self::Composite),
            "robust" => Some(Self::Robust),
            "chenattribute" => Some(Self::ChenAttribute),
            "cheneerdiagram" => Some(Self::ChenEerDiagram),
            "chenentity" => Some(Self::ChenEntity),
            "chenrelationship" => Some(Self::ChenRelationship),
            "concise" => Some(Self::Concise),
            "clock" => Some(Self::Clock),
            "componentdiagram" => Some(Self::ComponentDiagram),
            "constraintarrow" => Some(Self::ConstraintArrow),
            "control" => Some(Self::Control),
            "database" => Some(Self::Database),
            "day" => Some(Self::Day),
            "delay" => Some(Self::Delay),
            "description" => Some(Self::Description),
            "destroy" => Some(Self::Destroy),
            "diamond" => Some(Self::Diamond),
            "document" => Some(Self::Document),
            "ebnf" => Some(Self::Ebnf),
            "element" => Some(Self::Element),
            "entity" => Some(Self::Entity),
            "end" => Some(Self::End),
            "start" => Some(Self::Start),
            "stop" => Some(Self::Stop),
            "file" => Some(Self::File),
            "filesdiagram" => Some(Self::FilesDiagram),
            "folder" => Some(Self::Folder),
            "footer" => Some(Self::Footer),
            "frame" => Some(Self::Frame),
            "ganttdiagram" => Some(Self::GanttDiagram),
            "generic" => Some(Self::Generic),
            "goto" => Some(Self::Goto_),
            "group" => Some(Self::Group),
            "groupheader" => Some(Self::GroupHeader),
            "header" => Some(Self::Header),
            "hexagon" => Some(Self::Hexagon),
            "highlight" => Some(Self::Highlight),
            "hnote" => Some(Self::Hnote),
            "interface" => Some(Self::Interface_),
            "json" => Some(Self::Json),
            "jsondiagram" => Some(Self::JsonDiagram),
            "gitdiagram" => Some(Self::GitDiagram),
            "label" => Some(Self::Label),
            "leafnode" => Some(Self::LeafNode),
            "legend" => Some(Self::Legend),
            "lifeline" => Some(Self::LifeLine),
            "mainframe" => Some(Self::Mainframe),
            "map" => Some(Self::Map),
            "milestone" => Some(Self::Milestone),
            "mindmapdiagram" => Some(Self::MindmapDiagram),
            "month" => Some(Self::Month),
            "name" => Some(Self::Name),
            "network" => Some(Self::Network),
            "newpage" => Some(Self::Newpage),
            "node" => Some(Self::Node),
            "note" => Some(Self::Note),
            "nwdiagdiagram" => Some(Self::NwdiagDiagram),
            "packetdiagdiagram" => Some(Self::PacketdiagDiagram),
            "objectdiagram" => Some(Self::ObjectDiagram),
            "object" => Some(Self::Object),
            "package" => Some(Self::Package_),
            "participant" => Some(Self::Participant),
            "partition" => Some(Self::Partition),
            "person" => Some(Self::Person),
            "port" => Some(Self::Port),
            "process" => Some(Self::Process),
            "qualified" => Some(Self::Qualified),
            "queue" => Some(Self::Queue),
            "rectangle" => Some(Self::Rectangle),
            "reference" => Some(Self::Reference),
            "referenceheader" => Some(Self::ReferenceHeader),
            "regex" => Some(Self::Regex),
            "requirement" => Some(Self::Requirement),
            "rnote" => Some(Self::Rnote),
            "root" => Some(Self::Root),
            "rootnode" => Some(Self::RootNode),
            "saltdiagram" => Some(Self::SaltDiagram),
            "separator" => Some(Self::Separator),
            "sequencediagram" => Some(Self::SequenceDiagram),
            "server" => Some(Self::Server),
            "stack" => Some(Self::Stack),
            "statediagram" => Some(Self::StateDiagram),
            "state" => Some(Self::State),
            "statebody" => Some(Self::StateBody),
            "stereotype" => Some(Self::Stereotype),
            "storage" => Some(Self::Storage),
            "swimlane" => Some(Self::Swimlane),
            "task" => Some(Self::Task),
            "timegrid" => Some(Self::Timegrid),
            "timeline" => Some(Self::Timeline),
            "timingdiagram" => Some(Self::TimingDiagram),
            "title" => Some(Self::Title),
            "undone" => Some(Self::Undone),
            "unstarted" => Some(Self::Unstarted),
            "usecase" => Some(Self::Usecase),
            "verticalseparator" => Some(Self::VerticalSeparator),
            "year" => Some(Self::Year),
            "visibilityicon" => Some(Self::VisibilityIcon),
            "private" => Some(Self::Private_),
            "protected" => Some(Self::Protected_),
            "public" => Some(Self::Public_),
            "iemandatory" => Some(Self::IeMandatory),
            "spot" => Some(Self::Spot),
            "spotannotation" => Some(Self::SpotAnnotation),
            "spotinterface" => Some(Self::SpotInterface),
            "spotenum" => Some(Self::SpotEnum),
            "spotprotocol" => Some(Self::SpotProtocol),
            "spotstruct" => Some(Self::SpotStruct),
            "spotentity" => Some(Self::SpotEntity),
            "spotexception" => Some(Self::SpotException),
            "spotclass" => Some(Self::SpotClass),
            "spotabstractclass" => Some(Self::SpotAbstractClass),
            "spotmetaclass" => Some(Self::SpotMetaClass),
            "spotstereotype" => Some(Self::SpotStereotype),
            "spotdataclass" => Some(Self::SpotDataClass),
            "spotrecord" => Some(Self::SpotRecord),
            "wbsdiagram" => Some(Self::WbsDiagram),
            "yamldiagram" => Some(Self::YamlDiagram),
            "chartdiagram" => Some(Self::ChartDiagram),
            "bar" => Some(Self::Bar),
            "line" => Some(Self::Line),
            "area" => Some(Self::Area),
            "scatter" => Some(Self::Scatter),
            "axis" => Some(Self::Axis),
            "haxis" => Some(Self::HAxis),
            "vaxis" => Some(Self::VAxis),
            "grid" => Some(Self::Grid),
            "annotation" => Some(Self::Annotation),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// PName — catalogue of upstream property names (mirrors PName.java)
// ---------------------------------------------------------------------------

/// CSS-like property names, mirroring upstream `PName.java`.
///
/// Variant names follow Rust PascalCase; lookup is case-insensitive via
/// `from_name`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PName {
    Shadowing,
    FontName,
    FontColor,
    FontSize,
    FontStyle,
    FontWeight,
    BackgroundColor,
    RoundCorner,
    LineThickness,
    DiagonalCorner,
    HyperLinkColor,
    HyperlinkUnderlineStyle,
    HyperlinkUnderlineThickness,
    HeadColor,
    LineColor,
    LineStyle,
    Padding,
    Margin,
    MaximumWidth,
    MinimumWidth,
    ExportedName,
    Image,
    HorizontalAlignment,
    ShowStereotype,
    ImagePosition,
    // Chart-specific
    MarkerShape,
    MarkerSize,
    MarkerColor,
    BarWidth,
    Width,
}

impl PName {
    /// Case-insensitive lookup by the raw string from a style block.
    /// Returns `None` for unknown properties (stored in `StyleRule::unknown_properties`).
    pub fn from_name(s: &str) -> Option<Self> {
        let lower = s.to_ascii_lowercase();
        match lower.as_str() {
            "shadowing" => Some(Self::Shadowing),
            "fontname" => Some(Self::FontName),
            "fontcolor" => Some(Self::FontColor),
            "fontsize" => Some(Self::FontSize),
            "fontstyle" => Some(Self::FontStyle),
            "fontweight" => Some(Self::FontWeight),
            // upstream spells it "BackGroundColor" with capital G
            "backgroundcolor" | "backgroudcolor" => Some(Self::BackgroundColor),
            "roundcorner" => Some(Self::RoundCorner),
            "linethickness" | "borderthickness" => Some(Self::LineThickness),
            "diagonalcorner" => Some(Self::DiagonalCorner),
            "hyperlinkcolor" => Some(Self::HyperLinkColor),
            "hyperlinkunderlinestyle" => Some(Self::HyperlinkUnderlineStyle),
            "hyperlinkunderlinethickness" => Some(Self::HyperlinkUnderlineThickness),
            "headcolor" => Some(Self::HeadColor),
            // accept both "linecolor" and "bordercolor" (common alias)
            "linecolor" | "bordercolor" => Some(Self::LineColor),
            "linestyle" => Some(Self::LineStyle),
            "padding" => Some(Self::Padding),
            "margin" => Some(Self::Margin),
            "maximumwidth" => Some(Self::MaximumWidth),
            "minimumwidth" => Some(Self::MinimumWidth),
            "exportedname" => Some(Self::ExportedName),
            "image" => Some(Self::Image),
            "horizontalalignment" => Some(Self::HorizontalAlignment),
            "showstereotype" => Some(Self::ShowStereotype),
            "imageposition" => Some(Self::ImagePosition),
            "markershape" => Some(Self::MarkerShape),
            "markersize" => Some(Self::MarkerSize),
            "markercolor" => Some(Self::MarkerColor),
            "barwidth" => Some(Self::BarWidth),
            "width" => Some(Self::Width),
            // alias: upstream also accepts "borderroundcorner" in theme files
            "borderroundcorner" => Some(Self::RoundCorner),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// StyleValue — typed property values
// ---------------------------------------------------------------------------

/// A parsed property value from a `<style>` block.
///
/// Matches the Value.java type hierarchy.  `Raw` catches anything that the
/// value parser does not fully interpret (e.g. gradient specs like `#a-#b`,
/// variable references like `$FOO`, or compound spacing `"5 40"`).
#[derive(Debug, Clone, PartialEq)]
pub enum StyleValue {
    /// A CSS colour: `#RGB`, `#RRGGBB`, or a named PlantUML colour.
    Color(String),
    /// A plain numeric value (font size, thickness, …).
    Number(f64),
    /// A keyword value: `bold`, `italic`, `transparent`, `left`, etc.
    Keyword(String),
    /// A raw / unparsed value stored verbatim for later resolution.
    Raw(String),
}

impl StyleValue {
    /// Render as an owned string (used by the compat shim).
    pub fn to_display_string(&self) -> String {
        match self {
            Self::Color(s) | Self::Keyword(s) | Self::Raw(s) => s.clone(),
            Self::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Selector types
// ---------------------------------------------------------------------------

/// A single segment in a selector path.  Paths are built up by nesting:
/// `activityDiagram { partition { ... } }` produces a rule with
/// `[Tag(ActivityDiagram), Tag(Partition)]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectorSegment {
    /// A known tag from the SName catalogue.
    Tag(SName),
    /// A stereotype selector: `.Apache`, `.entity`.
    Stereotype(String),
    /// The wildcard `*`.
    Wildcard,
    /// A `:depth(N)` pseudo-selector.
    Depth(u32),
    /// A tag/selector string that is not in the known SName catalogue.
    Unknown(String),
}

/// A comma-expanded chain of segments.  Each `SelectorChain` represents one
/// of the comma-separated selectors in a rule group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorChain {
    pub segments: Vec<SelectorSegment>,
}

// ---------------------------------------------------------------------------
// StyleScheme
// ---------------------------------------------------------------------------

/// Whether the block (or sub-block) applies to the regular or dark scheme.
/// Dark rules come from `@media dark { ... }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StyleScheme {
    #[default]
    Regular,
    Dark,
}

// ---------------------------------------------------------------------------
// StyleRule
// ---------------------------------------------------------------------------

/// A single rule inside a `<style>` block, after comma-expansion.
///
/// `selector_path` is the full ancestry from the block root to this rule
/// (e.g. `[Tag(NwdiagDiagram), Tag(Group)]` for a nested `group { }` inside
/// `nwdiagDiagram { }`).
#[derive(Debug, Clone)]
pub struct StyleRule {
    /// Full selector path from the block root.  Each element is a
    /// `SelectorChain` (one per comma-expanded selector at that nesting depth).
    pub selector_path: Vec<SelectorChain>,
    /// Recognised properties for this rule.
    pub properties: BTreeMap<PName, StyleValue>,
    /// Properties whose name was not recognised.  Stored verbatim for
    /// diagnostic emission in Phase E.
    pub unknown_properties: BTreeMap<String, String>,
    /// Source insertion order (for stable cascade specificity within a block).
    pub source_order: u32,
    /// Which scheme this rule belongs to.
    pub scheme: StyleScheme,
}

// ---------------------------------------------------------------------------
// StyleBlock
// ---------------------------------------------------------------------------

/// The typed AST node for a complete `<style> … </style>` block.
///
/// This is the canonical `StatementKind::StyleBlock` variant.  The legacy
/// flat-triple compat shim (`StyleParam`) was removed in Phase E (#1417).
#[derive(Debug, Clone)]
pub struct StyleBlock {
    /// All rules produced by the parser, in source order.
    pub rules: Vec<StyleRule>,
    /// CSS-variable bindings: `--name: value` entries.
    pub variables: BTreeMap<String, String>,
}
