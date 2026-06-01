use crate::ast::NoteKind;
use crate::model::{
    LegendHAlign, LegendVAlign, MetadataHAlign, ParticipantRole, ScaleSpec, SequenceMessageStyle,
    VirtualEndpoint,
};
use crate::theme::SequenceStyle;

#[derive(Debug, Clone)]
pub struct Scene {
    pub width: i32,
    pub height: i32,
    pub header: Option<Label>,
    pub title: Option<Label>,
    pub caption: Option<Label>,
    pub footer: Option<Label>,
    pub participants: Vec<ParticipantBox>,
    pub footboxes: Vec<ParticipantBox>,
    pub lifelines: Vec<Lifeline>,
    pub messages: Vec<MessageLine>,
    pub activations: Vec<ActivationBox>,
    pub lifecycle_markers: Vec<LifecycleMarker>,
    pub notes: Vec<NoteBox>,
    pub groups: Vec<GroupBox>,
    pub structures: Vec<StructureLine>,
    pub style: SequenceStyle,
    pub scale: Option<ScaleSpec>,
    pub legend_text: Option<String>,
    pub legend_halign: LegendHAlign,
    pub legend_valign: LegendVAlign,
    /// Optional mainframe title — when `Some`, a UML mainframe border is drawn
    /// around the whole diagram with the given title text in the top-left notch.
    pub mainframe: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Label {
    pub x: i32,
    pub y: i32,
    pub lines: Vec<String>,
    pub align: MetadataHAlign,
}

#[derive(Debug, Clone)]
pub struct ParticipantBox {
    pub id: String,
    pub display_lines: Vec<String>,
    pub role: ParticipantRole,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextOverflowPolicy {
    WrapAndGrow,
    EllipsisSingleLine,
}

#[derive(Debug, Clone)]
pub struct Lifeline {
    pub participant_id: String,
    pub x: i32,
    pub y1: i32,
    pub y2: i32,
}

#[derive(Debug, Clone)]
pub struct MessageLine {
    pub from_id: String,
    pub to_id: String,
    pub x1: i32,
    pub y: i32,
    pub route_y: i32,
    pub x2: i32,
    pub arrow: String,
    pub label: Option<String>,
    pub label_lines: Vec<String>,
    pub style: SequenceMessageStyle,
    pub from_virtual: Option<VirtualEndpoint>,
    pub to_virtual: Option<VirtualEndpoint>,
}

#[derive(Debug, Clone)]
pub struct ActivationBox {
    pub participant_id: String,
    pub x: i32,
    pub y1: i32,
    pub y2: i32,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct LifecycleMarker {
    pub participant_id: String,
    pub x: i32,
    pub y: i32,
    pub kind: LifecycleMarkerKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleMarkerKind {
    Create,
    Destroy,
}

#[derive(Debug, Clone)]
pub struct NoteBox {
    pub target_id: Option<String>,
    pub kind: NoteKind,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct GroupBox {
    pub kind: String,
    pub label: Option<String>,
    pub color: Option<String>,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub separators: Vec<GroupSeparator>,
}

#[derive(Debug, Clone)]
pub struct GroupSeparator {
    pub y: i32,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StructureLine {
    pub kind: StructureKind,
    pub y: i32,
    pub x1: i32,
    pub x2: i32,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructureKind {
    Delay,
    Divider,
    Separator,
    Spacer,
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutOptions {
    pub margin: i32,
    pub participant_spacing: i32,
    pub participant_width: i32,
    pub participant_height: i32,
    pub title_height: i32,
    pub message_row_height: i32,
    pub note_width: i32,
    pub note_padding: i32,
    pub footer_height: i32,
    pub text_overflow_policy: TextOverflowPolicy,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            // Sequence density retune (#1371): tuned to match PlantUML's default
            // participant column width (~100px center-to-center), participant box
            // width (~80px), and message row height (~28px).  The prior values
            // (160/120/40) produced 2.80–3.31× area ratios vs PlantUML; these
            // values target ≤ 2.0× on the four audited fixtures.
            margin: 16,
            participant_spacing: 100,
            participant_width: 80,
            participant_height: 32,
            title_height: 28,
            message_row_height: 28,
            note_width: 140,
            note_padding: 6,
            footer_height: 24,
            text_overflow_policy: TextOverflowPolicy::WrapAndGrow,
        }
    }
}
