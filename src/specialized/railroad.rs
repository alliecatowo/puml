// ─── Railroad diagram shared primitives ──────────────────────────────────────

use super::shared::{escape_xml, svg_header, svg_white_bg};

/// A railroad diagram element.
#[derive(Debug, Clone)]
pub(super) enum RailNode {
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
const RAIL_PAD_X: i32 = 8;
const RAIL_FONT_W: i32 = 8; // approximate char width in monospace 12
const RAIL_BOX_H: i32 = 28;
const RAIL_GAP: i32 = 20; // horizontal gap between elements in a sequence
const RAIL_ALT_GAP: i32 = 24; // vertical gap between alternation branches

#[derive(Debug, Clone)]
pub(super) struct RailStyle {
    pub(super) literal_fill: String,
    pub(super) literal_stroke: String,
    pub(super) literal_text: String,
    pub(super) nonterminal_fill: String,
    pub(super) nonterminal_stroke: String,
    pub(super) nonterminal_text: String,
    pub(super) charclass_fill: String,
    pub(super) charclass_stroke: String,
    pub(super) charclass_text: String,
    pub(super) anchor_fill: String,
    pub(super) anchor_stroke: String,
    pub(super) anchor_text: String,
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

fn rail_literal_width(text: &str) -> i32 {
    (text.len() as i32) * RAIL_FONT_W + RAIL_PAD_X * 2 + 8
}

fn rail_box_width(text: &str) -> i32 {
    rail_literal_width(text).max(40)
}

pub(super) struct RailLayout {
    pub(super) svg: String,
    pub(super) width: i32,
    pub(super) height: i32,
    /// Y position of the track center-line within this element's bounding box
    pub(super) mid_y: i32,
}

fn layout_rail(node: &RailNode) -> RailLayout {
    layout_rail_with_style(node, &RailStyle::default())
}

pub(super) fn layout_rail_with_style(node: &RailNode, style: &RailStyle) -> RailLayout {
    match node {
        RailNode::Literal(text) => {
            let w = rail_box_width(text);
            let h = RAIL_BOX_H;
            let mid = h / 2;
            let svg = format!(
                "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" rx=\"14\" ry=\"14\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>
<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{}\">{}</text>",
                w, h,
                style.literal_fill,
                style.literal_stroke,
                w / 2, mid,
                style.literal_text,
                escape_xml(text)
            );
            RailLayout {
                svg,
                width: w,
                height: h,
                mid_y: mid,
            }
        }
        RailNode::NonTerminal(name) => {
            let w = rail_box_width(name);
            let h = RAIL_BOX_H;
            let mid = h / 2;
            let svg = format!(
                "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>
<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{}\">{}</text>",
                w, h,
                style.nonterminal_fill,
                style.nonterminal_stroke,
                w / 2, mid,
                style.nonterminal_text,
                escape_xml(name)
            );
            RailLayout {
                svg,
                width: w,
                height: h,
                mid_y: mid,
            }
        }
        RailNode::Anchor(sym) => {
            let w = 30;
            let h = RAIL_BOX_H;
            let mid = h / 2;
            let svg = format!(
                "<metadata class=\"regex-token regex-anchor\"/><circle cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>
<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{}\">{}</text>",
                w / 2, mid,
                style.anchor_fill,
                style.anchor_stroke,
                w / 2, mid,
                style.anchor_text,
                escape_xml(sym)
            );
            RailLayout {
                svg,
                width: w,
                height: h,
                mid_y: mid,
            }
        }
        RailNode::CharClass(cls) => {
            let text = format!("[{}]", cls);
            let w = rail_box_width(&text);
            let h = RAIL_BOX_H;
            let mid = h / 2;
            let svg = format!(
                "<metadata class=\"regex-token regex-charclass\" data-regex-class=\"{}\"/><rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>
<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{}\">{}</text>",
                escape_xml(cls),
                w, h,
                style.charclass_fill,
                style.charclass_stroke,
                w / 2, mid,
                style.charclass_text,
                escape_xml(&text)
            );
            RailLayout {
                svg,
                width: w,
                height: h,
                mid_y: mid,
            }
        }
        RailNode::Special(text) => {
            let label = format!("? {} ?", text);
            let w = rail_box_width(&label);
            let h = RAIL_BOX_H;
            let mid = h / 2;
            let svg = format!(
                "<metadata class=\"ebnf-token ebnf-special\" data-ebnf-special=\"{}\"/><rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"#ede9fe\" stroke=\"#7c3aed\" stroke-width=\"1.5\" stroke-dasharray=\"4 2\"/>
<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"#4c1d95\">{}</text>",
                escape_xml(text),
                w, h,
                w / 2, mid,
                escape_xml(&label)
            );
            RailLayout {
                svg,
                width: w,
                height: h,
                mid_y: mid,
            }
        }
        RailNode::Empty => RailLayout {
            svg: String::new(),
            width: 0,
            height: RAIL_BOX_H,
            mid_y: RAIL_BOX_H / 2,
        },
        RailNode::Sequence(items) => {
            let children: Vec<RailLayout> = items
                .iter()
                .map(|item| layout_rail_with_style(item, style))
                .collect();
            if children.is_empty() {
                return RailLayout {
                    svg: String::new(),
                    width: 0,
                    height: RAIL_BOX_H,
                    mid_y: RAIL_BOX_H / 2,
                };
            }
            // Align all mid_y to the max
            let mid_y = children.iter().map(|c| c.mid_y).max().unwrap_or(0);
            let height = children
                .iter()
                .map(|c| c.height - c.mid_y + mid_y)
                .max()
                .unwrap_or(0)
                .max(RAIL_BOX_H);
            let mut x = 0i32;
            let mut out = String::new();
            for (i, child) in children.iter().enumerate() {
                let offset_y = mid_y - child.mid_y;
                out.push_str(&format!(
                    "<g transform=\"translate({},{})\">{}</g>",
                    x, offset_y, child.svg
                ));
                // Draw connector line to next
                if i + 1 < children.len() {
                    let cx1 = x + child.width;
                    let cx2 = x + child.width + RAIL_GAP;
                    out.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
                        cx1, mid_y, cx2, mid_y
                    ));
                    x += child.width + RAIL_GAP;
                } else {
                    x += child.width;
                }
            }
            let total_w = x;
            RailLayout {
                svg: out,
                width: total_w,
                height,
                mid_y,
            }
        }
        RailNode::Alternation(branches) => {
            if branches.is_empty() {
                return RailLayout {
                    svg: String::new(),
                    width: 0,
                    height: RAIL_BOX_H,
                    mid_y: RAIL_BOX_H / 2,
                };
            }
            let children: Vec<RailLayout> = branches
                .iter()
                .map(|branch| layout_rail_with_style(branch, style))
                .collect();
            let max_w = children.iter().map(|c| c.w_or_min()).max().unwrap_or(60);
            let total_h: i32 = children
                .iter()
                .map(|c| c.height + RAIL_ALT_GAP)
                .sum::<i32>()
                - RAIL_ALT_GAP;
            let mid_y = RAIL_BOX_H / 2; // first branch is the nominal track
                                        // Width includes entry/exit lines
            let inner_w = max_w + 40;
            let total_w = inner_w;
            let mut out = String::from(
                "<metadata class=\"regex-token regex-alt\"/><metadata class=\"ebnf-token ebnf-alt\"/>",
            );
            let mut y = 0i32;
            for (i, child) in children.iter().enumerate() {
                let branch_mid = y + child.mid_y;
                let cx = (inner_w - child.w_or_min()) / 2;
                // Horizontal line from left join point to branch start
                out.push_str(&format!(
                    "<line x1=\"20\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
                    branch_mid, cx, branch_mid
                ));
                // Render child
                out.push_str(&format!(
                    "<g transform=\"translate({},{})\">{}</g>",
                    cx, y, child.svg
                ));
                // Horizontal line from branch end to right join point
                let right_x = cx + child.w_or_min();
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
                    right_x, branch_mid, inner_w - 20, branch_mid
                ));
                if i + 1 < children.len() {
                    y += child.height + RAIL_ALT_GAP;
                }
            }
            // Vertical join lines on left and right
            let first_mid = children[0].mid_y;
            let last_mid = y + children.last().unwrap().mid_y;
            out.push_str(&format!(
                "<line x1=\"20\" y1=\"{}\" x2=\"20\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
                first_mid, last_mid
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
                inner_w - 20, first_mid, inner_w - 20, last_mid
            ));
            RailLayout {
                svg: out,
                width: total_w,
                height: total_h,
                mid_y,
            }
        }
        RailNode::Repeat(inner) => {
            let child = layout_rail_with_style(inner, style);
            let w = child.width + 40;
            let h = child.height + 24;
            let mid_y = child.mid_y;
            let mut out = String::from(
                "<metadata class=\"regex-token regex-repeat\"/><metadata class=\"ebnf-token ebnf-repetition\"/>",
            );
            // Position child
            out.push_str(&format!(
                "<g transform=\"translate(20,0)\">{}</g>",
                child.svg
            ));
            // Forward track lines
            out.push_str(&format!(
                "<line x1=\"0\" y1=\"{}\" x2=\"20\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
                mid_y, mid_y
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
                20 + child.width, mid_y, w, mid_y
            ));
            // Loop-back arrow path below
            let loop_y = h - 8;
            out.push_str(&format!(
                "<path d=\"M {} {} Q {} {} {} {}\" stroke=\"#999\" stroke-width=\"1\" fill=\"none\" stroke-dasharray=\"4 2\"/>",
                20 + child.width, mid_y,
                20 + child.width, loop_y,
                20, loop_y
            ));
            out.push_str(&format!(
                "<path d=\"M {} {} Q {} {} {} {}\" stroke=\"#999\" stroke-width=\"1\" fill=\"none\" stroke-dasharray=\"4 2\"/>",
                20, loop_y,
                20, mid_y,
                20, mid_y
            ));
            // Arrow head for loop direction
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"#999\"/>",
                20,
                mid_y,
                26,
                mid_y - 4,
                26,
                mid_y + 4
            ));
            // "0+/n+" label below loop line
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#888\" text-anchor=\"middle\">*</text>",
                w / 2, loop_y - 2
            ));
            RailLayout {
                svg: out,
                width: w,
                height: h,
                mid_y,
            }
        }
        RailNode::OneOrMore(inner) => {
            let child = layout_rail_with_style(inner, style);
            let w = child.width + 40;
            let h = child.height + 24;
            let mid_y = child.mid_y;
            let mut out = String::from(
                "<metadata class=\"regex-token regex-repeat\"/><metadata class=\"ebnf-token ebnf-repetition\"/>",
            );
            out.push_str(&format!(
                "<g transform=\"translate(20,0)\">{}</g>",
                child.svg
            ));
            out.push_str(&format!(
                "<line x1=\"0\" y1=\"{}\" x2=\"20\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
                mid_y, mid_y
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
                20 + child.width, mid_y, w, mid_y
            ));
            let loop_y = h - 8;
            out.push_str(&format!(
                "<path d=\"M {} {} Q {} {} {} {}\" stroke=\"#999\" stroke-width=\"1\" fill=\"none\" stroke-dasharray=\"4 2\"/>",
                20 + child.width, mid_y,
                20 + child.width, loop_y,
                20, loop_y
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#888\" text-anchor=\"middle\">+</text>",
                w / 2, loop_y - 2
            ));
            RailLayout {
                svg: out,
                width: w,
                height: h,
                mid_y,
            }
        }
        RailNode::CountedRepeat(inner, spec) => {
            let child = layout_rail_with_style(inner, style);
            let w = child.width + 40;
            let h = child.height + 28;
            let mid_y = child.mid_y;
            let inner_label = match inner.as_ref() {
                RailNode::Alternation(_) => format!("({})", rail_node_label(inner)),
                _ => rail_node_label(inner),
            };
            let label = format!("{}{}", inner_label, spec);
            let mut out = String::new();
            out.push_str(
                "<metadata class=\"regex-token regex-repeat\"/><metadata class=\"ebnf-token ebnf-repetition\"/>",
            );
            out.push_str(&format!(
                "<metadata data-regex-repeat=\"{}\"/>",
                escape_xml(&label)
            ));
            out.push_str(&format!(
                "<metadata data-rail-repeat-label=\"{}\"/>",
                escape_xml(&repeat_label(spec))
            ));
            out.push_str(&format!(
                "<g transform=\"translate(20,0)\">{}</g>",
                child.svg
            ));
            out.push_str(&format!(
                "<line x1=\"0\" y1=\"{}\" x2=\"20\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
                mid_y, mid_y
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
                20 + child.width, mid_y, w, mid_y
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#888\" text-anchor=\"middle\">{}</text>",
                w / 2,
                h - 6,
                escape_xml(spec)
            ));
            RailLayout {
                svg: out,
                width: w,
                height: h,
                mid_y,
            }
        }
        RailNode::Optional(inner) => {
            let child = layout_rail_with_style(inner, style);
            let w = child.width + 40;
            let h = child.height + 24;
            let mid_y = child.mid_y;
            let skip_y = h - 12;
            let mut out = String::from(
                "<metadata class=\"regex-token regex-repeat\"/><metadata class=\"ebnf-token ebnf-optional\"/>",
            );
            out.push_str(&format!(
                "<g transform=\"translate(20,0)\">{}</g>",
                child.svg
            ));
            out.push_str(&format!(
                "<line x1=\"0\" y1=\"{}\" x2=\"20\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
                mid_y, mid_y
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
                20 + child.width, mid_y, w, mid_y
            ));
            // Skip path at bottom
            out.push_str(&format!(
                "<path d=\"M 0 {} Q 0 {} {} {}\" stroke=\"#aaa\" stroke-width=\"1\" fill=\"none\" stroke-dasharray=\"3 3\"/>",
                mid_y, skip_y, w / 2, skip_y
            ));
            out.push_str(&format!(
                "<path d=\"M {} {} Q {} {} {} {}\" stroke=\"#aaa\" stroke-width=\"1\" fill=\"none\" stroke-dasharray=\"3 3\"/>",
                w / 2, skip_y, w, skip_y, w, mid_y
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#aaa\" text-anchor=\"middle\">?</text>",
                w / 2, skip_y - 2
            ));
            RailLayout {
                svg: out,
                width: w,
                height: h,
                mid_y,
            }
        }
    }
}

fn rail_node_label(node: &RailNode) -> String {
    match node {
        RailNode::Literal(text) => format!("'{text}'"),
        RailNode::Sequence(items) => {
            if items
                .iter()
                .all(|item| matches!(item, RailNode::Literal(_)))
            {
                let mut literal = String::new();
                for item in items {
                    if let RailNode::Literal(text) = item {
                        literal.push_str(text);
                    }
                }
                format!("'{literal}'")
            } else {
                items
                    .iter()
                    .map(rail_node_label)
                    .collect::<Vec<_>>()
                    .join("")
            }
        }
        RailNode::Alternation(branches) => format!(
            "alt({})",
            branches
                .iter()
                .map(rail_node_label)
                .collect::<Vec<_>>()
                .join("|")
        ),
        RailNode::Repeat(inner) => format!("{}*", rail_node_label(inner)),
        RailNode::Optional(inner) => format!("{}?", rail_node_label(inner)),
        RailNode::OneOrMore(inner) => format!("{}+", rail_node_label(inner)),
        RailNode::CountedRepeat(inner, spec) => format!("{}{}", rail_node_label(inner), spec),
        RailNode::NonTerminal(name) => name.clone(),
        RailNode::Anchor(sym) => sym.clone(),
        RailNode::CharClass(text) => text.clone(),
        RailNode::Special(text) => format!("? {text} ?"),
        RailNode::Empty => String::new(),
    }
}

fn repeat_label(spec: &str) -> String {
    let body = spec.trim_matches(|ch| ch == '{' || ch == '}');
    match body.split_once(',') {
        Some(("", max)) => format!("up to {}", max.trim()),
        Some((min, "")) => format!("at least {}", min.trim()),
        Some((min, max)) => format!("{} to {}", min.trim(), max.trim()),
        None if !body.is_empty() => format!("exactly {body}"),
        _ => "counted repeat".to_string(),
    }
}

impl RailLayout {
    fn w_or_min(&self) -> i32 {
        self.width.max(40)
    }
}

/// Render a complete railroad diagram as SVG.
pub(super) fn render_railroad(title: &str, root: &RailNode) -> String {
    let inner = layout_rail(root);
    let margin = 20;
    let title_h = if title.is_empty() { 0 } else { 28 };
    // Canvas width must accommodate the title text (approx 9px per char) as well as the
    // railroad diagram, so long source-pattern titles are never clipped (#514).
    let title_w = if title.is_empty() {
        0
    } else {
        (title.len() as i32) * 9 + margin * 2
    };
    let width = (inner.width + margin * 2 + 40).max(title_w); // +40 for entry/exit track lines
    let height = inner.height + margin * 2 + title_h + 20;

    let mut out = String::new();
    out.push_str(&svg_header(width, height));
    out.push_str(svg_white_bg());

    if !title.is_empty() {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"15\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            width / 2, margin + 18, escape_xml(title)
        ));
    }

    let base_y = margin + title_h;
    let mid_y = base_y + inner.mid_y + 10;

    // Entry track
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
        margin,
        mid_y,
        margin + 20,
        mid_y
    ));
    // Start circle
    out.push_str(&format!(
        "<circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"#333\"/>",
        margin, mid_y
    ));

    // Diagram content
    out.push_str(&format!(
        "<g transform=\"translate({},{})\">{}</g>",
        margin + 20,
        base_y + 10,
        inner.svg
    ));

    // Exit track
    let exit_x = margin + 20 + inner.width;
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
        exit_x,
        mid_y,
        exit_x + 20,
        mid_y
    ));
    // End double circle
    out.push_str(&format!(
        "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"none\" stroke=\"#333\" stroke-width=\"2\"/>",
        exit_x + 20,
        mid_y
    ));
    out.push_str(&format!(
        "<circle cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"#333\"/>",
        exit_x + 20,
        mid_y
    ));

    out.push_str("</svg>");
    out
}
