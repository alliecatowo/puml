/// A railroad diagram element.
#[derive(Debug, Clone)]
pub(crate) enum RailNode {
    /// A literal terminal in a rounded rect.
    Literal(String),
    /// Sequence of nodes.
    Sequence(Vec<RailNode>),
    /// Alternation / choice: parallel branches.
    Alternation(Vec<RailNode>),
    /// Zero-or-more repetition (loop back).
    Repeat(Box<RailNode>),
    /// Optional (zero-or-one).
    Optional(Box<RailNode>),
    /// One-or-more (+ quantifier): item then loop.
    OneOrMore(Box<RailNode>),
    /// Counted repeat ({n}, {n,m}, {n,}, {,m}) with exact source label.
    CountedRepeat(Box<RailNode>, String),
    /// Non-terminal reference.
    NonTerminal(String),
    /// Anchor (^, $).
    Anchor(String),
    /// Character class.
    CharClass(String),
    /// EBNF special sequence: ? ... ?
    Special(String),
    /// Empty / epsilon.
    Empty,
}

/// Layout a RailNode and return an SVG group string plus (width, height, midY).
/// `mid_y` is the vertical center of the track baseline.
pub(super) const RAIL_PAD_X: i32 = 8;
pub(super) const RAIL_FONT_W: i32 = 8; // approximate char width in monospace 12
pub(super) const RAIL_BOX_H: i32 = 28;
pub(super) const RAIL_GAP: i32 = 20; // horizontal gap between elements in a sequence
pub(super) const RAIL_ALT_GAP: i32 = 24; // vertical gap between alternation branches

#[derive(Debug, Clone)]
pub(crate) struct RailStyle {
    pub(crate) literal_fill: String,
    pub(crate) literal_stroke: String,
    pub(crate) literal_text: String,
    pub(crate) nonterminal_fill: String,
    pub(crate) nonterminal_stroke: String,
    pub(crate) nonterminal_text: String,
    pub(crate) charclass_fill: String,
    pub(crate) charclass_stroke: String,
    pub(crate) charclass_text: String,
    pub(crate) anchor_fill: String,
    pub(crate) anchor_stroke: String,
    pub(crate) anchor_text: String,
}

impl Default for RailStyle {
    fn default() -> Self {
        Self {
            literal_fill: "#fff8e1".to_string(),
            literal_stroke: "#f9a825".to_string(),
            literal_text: "#333".to_string(),
            nonterminal_fill: "#e8f5e9".to_string(),
            nonterminal_stroke: "#388e3c".to_string(),
            nonterminal_text: "#1b5e20".to_string(),
            charclass_fill: "#fce4ec".to_string(),
            charclass_stroke: "#c62828".to_string(),
            charclass_text: "#b71c1c".to_string(),
            anchor_fill: "#e3f2fd".to_string(),
            anchor_stroke: "#1976d2".to_string(),
            anchor_text: "#1565c0".to_string(),
        }
    }
}

pub(crate) struct RailLayout {
    pub(crate) svg: String,
    pub(crate) width: i32,
    pub(crate) height: i32,
    /// Y position of the track center-line within this element's bounding box
    pub(crate) mid_y: i32,
}
