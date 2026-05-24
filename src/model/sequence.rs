use crate::ast::NoteKind;
use crate::diagnostic::Diagnostic;
use crate::model::{LegendHAlign, LegendVAlign, MetadataHAlign, ScaleSpec};
use crate::source::Span;
use crate::sprites::SpriteRegistry;
use crate::theme::SequenceStyle;

#[derive(Debug, Clone)]
pub struct SequenceDocument {
    pub participants: Vec<Participant>,
    pub participant_groups: Vec<SequenceParticipantGroup>,
    pub events: Vec<SequenceEvent>,
    pub teoz: bool,
    pub title: Option<String>,
    pub header: Option<String>,
    pub header_align: MetadataHAlign,
    pub footer: Option<String>,
    pub footer_align: MetadataHAlign,
    pub caption: Option<String>,
    pub legend: Option<String>,
    pub skinparams: Vec<(String, String)>,
    pub style: SequenceStyle,
    pub footbox_visible: bool,
    pub scale: Option<ScaleSpec>,
    pub legend_halign: LegendHAlign,
    pub legend_valign: LegendVAlign,
    pub warnings: Vec<Diagnostic>,
    /// Whether `hide unlinked` was active for this document.
    pub hide_unlinked: bool,
    /// IDs of participants that were removed by the `hide unlinked` filter.
    pub hidden_participants: Vec<String>,
    pub sprites: SpriteRegistry,
    pub list_sprites: bool,
    /// Optional mainframe title (`mainframe <text>` keyword — feature 1.43).
    pub mainframe: Option<String>,
}

impl Default for SequenceDocument {
    fn default() -> Self {
        Self {
            participants: Vec::new(),
            participant_groups: Vec::new(),
            events: Vec::new(),
            teoz: false,
            title: None,
            header: None,
            header_align: MetadataHAlign::default(),
            footer: None,
            footer_align: MetadataHAlign::default(),
            caption: None,
            legend: None,
            skinparams: Vec::new(),
            style: SequenceStyle::default(),
            footbox_visible: true,
            scale: None,
            legend_halign: LegendHAlign::default(),
            legend_valign: LegendVAlign::default(),
            warnings: Vec::new(),
            hide_unlinked: false,
            hidden_participants: Vec::new(),
            sprites: SpriteRegistry::new(),
            list_sprites: false,
            mainframe: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SequencePage {
    pub participants: Vec<Participant>,
    pub participant_groups: Vec<SequenceParticipantGroup>,
    pub events: Vec<SequenceEvent>,
    pub teoz: bool,
    pub title: Option<String>,
    pub header: Option<String>,
    pub header_align: MetadataHAlign,
    pub footer: Option<String>,
    pub footer_align: MetadataHAlign,
    pub caption: Option<String>,
    pub legend: Option<String>,
    pub skinparams: Vec<(String, String)>,
    pub style: SequenceStyle,
    pub footbox_visible: bool,
    pub scale: Option<ScaleSpec>,
    pub legend_halign: LegendHAlign,
    pub legend_valign: LegendVAlign,
    pub warnings: Vec<Diagnostic>,
    /// Whether `hide unlinked` was active for this page.
    pub hide_unlinked: bool,
    /// IDs of participants that were removed by the `hide unlinked` filter.
    pub hidden_participants: Vec<String>,
    pub sprites: SpriteRegistry,
    pub list_sprites: bool,
    /// Optional mainframe title (`mainframe <text>` keyword — feature 1.43).
    pub mainframe: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Participant {
    pub id: String,
    pub display: String,
    pub role: ParticipantRole,
    pub explicit: bool,
}

#[derive(Debug, Clone)]
pub struct SequenceParticipantGroup {
    pub label: Option<String>,
    pub color: Option<String>,
    pub participant_ids: Vec<String>,
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
        style: SequenceMessageStyle,
        from_virtual: Option<VirtualEndpoint>,
        to_virtual: Option<VirtualEndpoint>,
    },
    Note {
        kind: NoteKind,
        position: String,
        target: Option<String>,
        text: String,
        /// When `true`, align this note at the same y level as the preceding note.
        aligned: bool,
    },
    GroupStart {
        kind: String,
        label: Option<String>,
    },
    GroupEnd,
    Delay(Option<String>),
    Divider(Option<String>),
    Separator(Option<String>),
    Spacer(Option<i32>),
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SequenceMessageStyle {
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
    /// Short arrow (`?->` / `->?`) — stub from the diagram edge (feature 1.30).
    Short,
}
