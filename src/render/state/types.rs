// Layout constants for the state diagram renderer.
// Shared between state.rs and its scene/label sub-modules.

pub(super) const STATE_NODE_W: i32 = 140;
pub(super) const STATE_NODE_H: i32 = 40;
pub(super) const STATE_NODE_GAP_X: i32 = 60;
pub(super) const STATE_NODE_GAP_Y: i32 = 60;
pub(super) const STATE_MARGIN: i32 = 30;
// X-offset of the right-side gutter column used for sink states.
pub(super) const STATE_SINK_GUTTER_GAP: i32 = 80;
pub(super) const COMPOSITE_PAD_X: i32 = 16;
pub(super) const COMPOSITE_PAD_Y: i32 = 36; // extra space for composite header label
pub(super) const COMPOSITE_PAD_BOT: i32 = 12;
pub(super) const REGION_DIVIDER_GAP: i32 = 24; // gap between concurrent regions / divider clearance
pub(super) const STATE_LABEL_LINE_H: i32 = 14;
pub(super) const STATE_LABEL_CHAR_W: i32 = 7;
pub(super) const STATE_LABEL_NODE_CLEARANCE: i32 = 12;
pub(super) const STATE_LABEL_LABEL_CLEARANCE: i32 = 8;
pub(super) const STATE_LABEL_WRAP_COLS: usize = 24;
pub(super) const STATE_NOTE_FILL: &str = "#fff8c4";
pub(super) const STATE_NOTE_BORDER: &str = "#111111";
pub(super) const STATE_NOTE_PAD_X: i32 = 10;
pub(super) const STATE_NOTE_PAD_Y: i32 = 10;

/// A placed node entry in the flat coord map.
/// Stores the node's top-left (x, y) and its full rendered size (w, h).
#[derive(Clone, Copy)]
pub(super) struct PlacedNode {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
}

#[derive(Clone)]
pub(super) struct StateLabelLayout {
    pub(super) cx: i32,
    pub(super) top: i32,
    pub(super) lines: Vec<String>,
    pub(super) bounds: LabelBounds,
}

#[derive(Clone, Copy)]
pub(super) struct LabelBounds {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
}
