/// Specialized diagram renderers for @startregex, @startebnf, @startchart,
/// @startmath/@startlatex, @startsdl, and @startditaa diagram families.
///
/// These bypass the main AST parser pipeline and implement their own
/// mini-parsers and SVG renderers.
use crate::diagnostic::Diagnostic;
use crate::theme::resolve_sequence_theme_preset;
use std::collections::BTreeMap;

// ─── Public dispatch ──────────────────────────────────────────────────────────

/// Try to render `source` as one of the specialized diagram families.
/// Returns `Some(svg)` if the source is recognized, `None` otherwise.
pub fn try_render_specialized(source: &str) -> Option<Result<String, Diagnostic>> {
    let trimmed = source.trim();
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("@startregex") {
        Some(render_regex(trimmed))
    } else if lower.starts_with("@startebnf") {
        Some(render_ebnf(trimmed))
    } else if lower.starts_with("@startchart") {
        Some(render_chart(trimmed))
    } else if lower.starts_with("@startmath") {
        Some(render_math(trimmed))
    } else if lower.starts_with("@startlatex") {
        Some(render_latex(trimmed))
    } else if lower.starts_with("@startsdl") {
        Some(render_sdl(trimmed))
    } else if lower.starts_with("@startditaa") {
        Some(render_ditaa(trimmed))
    } else {
        None
    }
}

// ─── Shared SVG utilities ─────────────────────────────────────────────────────

fn escape_xml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn svg_header(width: i32, height: i32) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    )
}

fn svg_white_bg() -> &'static str {
    "<rect width=\"100%\" height=\"100%\" fill=\"white\"/>"
}

/// Strip the @start.../@end... wrapper and return body lines and optional title.
fn strip_block<'a>(source: &'a str, start_tag: &str, end_tag: &str) -> (&'a str, Option<String>) {
    let mut lines = source.lines();
    let first_line = lines.next().unwrap_or("").trim();
    // consume @startXXX line, possibly with a title
    let tag_lower = first_line.to_ascii_lowercase();
    let rest_after_tag = if tag_lower.starts_with(start_tag) {
        first_line[start_tag.len()..].trim()
    } else {
        first_line
    };
    let title: Option<String> = if rest_after_tag.starts_with('"') {
        Some(rest_after_tag.trim_matches('"').to_string())
    } else if !rest_after_tag.is_empty() {
        Some(rest_after_tag.to_string())
    } else {
        None
    };

    // The body is everything between the @start and @end lines
    let body_start = first_line.len() + 1; // skip first line + newline
    let body_end = if let Some(pos) = source.to_ascii_lowercase().rfind(end_tag) {
        // go back to find start of that line
        let before = &source[..pos];
        before.rfind('\n').map(|i| i + 1).unwrap_or(0)
    } else {
        source.len()
    };

    let body = source
        .get(body_start.min(source.len())..body_end.min(source.len()))
        .unwrap_or("");
    (body, title)
}

// ─── Railroad diagram shared primitives ──────────────────────────────────────

/// A railroad diagram element.
#[derive(Debug, Clone)]
enum RailNode {
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
struct RailStyle {
    literal_fill: String,
    literal_stroke: String,
    literal_text: String,
    nonterminal_fill: String,
    nonterminal_stroke: String,
    nonterminal_text: String,
    charclass_fill: String,
    charclass_stroke: String,
    charclass_text: String,
    anchor_fill: String,
    anchor_stroke: String,
    anchor_text: String,
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

struct RailLayout {
    svg: String,
    width: i32,
    height: i32,
    /// Y position of the track center-line within this element's bounding box
    mid_y: i32,
}

fn layout_rail(node: &RailNode) -> RailLayout {
    layout_rail_with_style(node, &RailStyle::default())
}

fn layout_rail_with_style(node: &RailNode, style: &RailStyle) -> RailLayout {
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
                "<circle cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>
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
                "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>
<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{}\">{}</text>",
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
            let mut out = String::new();
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
            let mut out = String::new();
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
            let mut out = String::new();
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
            out.push_str(&format!(
                "<metadata data-regex-repeat=\"{}\"/>",
                escape_xml(&label)
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
            let mut out = String::new();
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
            if items.iter().all(|item| matches!(item, RailNode::Literal(_))) {
                let mut literal = String::new();
                for item in items {
                    if let RailNode::Literal(text) = item {
                        literal.push_str(text);
                    }
                }
                format!("'{literal}'")
            } else {
                items.iter().map(rail_node_label).collect::<Vec<_>>().join("")
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
        RailNode::Empty => String::new(),
    }
}

impl RailLayout {
    fn w_or_min(&self) -> i32 {
        self.width.max(40)
    }
}

/// Render a complete railroad diagram as SVG.
fn render_railroad(title: &str, root: &RailNode) -> String {
    let inner = layout_rail(root);
    let margin = 20;
    let title_h = if title.is_empty() { 0 } else { 28 };
    let width = inner.width + margin * 2 + 40; // +40 for entry/exit track lines
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

// ─── Family 1: @startregex ────────────────────────────────────────────────────

fn render_regex(source: &str) -> Result<String, Diagnostic> {
    let (body, _title) = strip_block(source, "@startregex", "@endregex");
    let mut locale = RegexLocale::English;
    let mut patterns = Vec::new();
    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('\'') {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(rest) = lower
            .strip_prefix("locale ")
            .or_else(|| lower.strip_prefix("language "))
            .or_else(|| lower.strip_prefix("lang "))
        {
            locale = RegexLocale::from_name(rest.trim());
            continue;
        }
        patterns.push(line.to_string());
    }
    let pattern = patterns.join("\n");
    if pattern.is_empty() {
        return Err(Diagnostic::error(
            "[E_REGEX_EMPTY] @startregex body is empty",
        ));
    }
    let node = if patterns.len() == 1 {
        parse_regex_to_rail(&patterns[0], locale)
    } else {
        RailNode::Alternation(
            patterns
                .iter()
                .map(|pattern| parse_regex_to_rail(pattern, locale))
                .collect(),
        )
    };
    let title = format!("/{}/", pattern.replace('\n', " | "));
    Ok(render_railroad(&title, &node))
}

#[derive(Debug, Clone, Copy)]
enum RegexLocale {
    English,
    French,
    Spanish,
}

impl RegexLocale {
    fn from_name(name: &str) -> Self {
        match name {
            "fr" | "fra" | "fre" | "french" | "francais" | "français" => Self::French,
            "es" | "spa" | "spanish" | "espanol" | "español" => Self::Spanish,
            _ => Self::English,
        }
    }

    fn label(self, key: &str) -> &'static str {
        match (self, key) {
            (Self::French, "digit") => "chiffre",
            (Self::French, "word") => "mot",
            (Self::French, "space") => "espace",
            (Self::French, "any") => "tout",
            (Self::French, "start") => "debut",
            (Self::French, "end") => "fin",
            (Self::Spanish, "digit") => "digito",
            (Self::Spanish, "word") => "palabra",
            (Self::Spanish, "space") => "espacio",
            (Self::Spanish, "any") => "cualquiera",
            (Self::Spanish, "start") => "inicio",
            (Self::Spanish, "end") => "fin",
            (_, "digit") => "digit",
            (_, "word") => "word",
            (_, "space") => "whitespace",
            (_, "any") => "any char",
            (_, "start") => "start",
            (_, "end") => "end",
            _ => "regex",
        }
    }
}

/// Parse a regex pattern string into a RailNode AST.
/// Supports: literals, `.`, `|`, `(...)`, `[...]`, `*`, `+`, `?`, `^`, `$`.
fn parse_regex_to_rail(pattern: &str, locale: RegexLocale) -> RailNode {
    let chars: Vec<char> = pattern.chars().collect();
    let (node, _) = parse_regex_alternation(&chars, 0, locale);
    node
}

fn parse_regex_alternation(chars: &[char], start: usize, locale: RegexLocale) -> (RailNode, usize) {
    let mut branches = Vec::new();
    let (first, mut pos) = parse_regex_sequence(chars, start, locale);
    branches.push(first);
    while pos < chars.len() && chars[pos] == '|' {
        pos += 1;
        let (branch, new_pos) = parse_regex_sequence(chars, pos, locale);
        branches.push(branch);
        pos = new_pos;
    }
    if branches.len() == 1 {
        (branches.remove(0), pos)
    } else {
        (RailNode::Alternation(branches), pos)
    }
}

fn parse_regex_sequence(chars: &[char], start: usize, locale: RegexLocale) -> (RailNode, usize) {
    let mut items = Vec::new();
    let mut pos = start;
    while pos < chars.len() {
        match chars[pos] {
            ')' | '|' => break,
            '^' | '$' => {
                let sym = if chars[pos] == '^' {
                    locale.label("start").to_string()
                } else {
                    locale.label("end").to_string()
                };
                pos += 1;
                items.push(RailNode::Anchor(sym));
            }
            '(' => {
                pos += 1; // consume '('
                let (inner, new_pos) = parse_regex_alternation(chars, pos, locale);
                pos = new_pos;
                if pos < chars.len() && chars[pos] == ')' {
                    pos += 1;
                }
                // Check quantifier
                let (node, new_pos2) = apply_quantifier(inner, chars, pos);
                pos = new_pos2;
                items.push(node);
            }
            '[' => {
                pos += 1;
                let mut cls = String::new();
                while pos < chars.len() && chars[pos] != ']' {
                    cls.push(chars[pos]);
                    pos += 1;
                }
                if pos < chars.len() {
                    pos += 1; // consume ']'
                }
                let node = RailNode::CharClass(cls);
                let (node, new_pos) = apply_quantifier(node, chars, pos);
                pos = new_pos;
                items.push(node);
            }
            '\\' => {
                pos += 1;
                let escaped = if pos < chars.len() {
                    let c = chars[pos];
                    pos += 1;
                    regex_escape_label(c, locale)
                } else {
                    "\\".to_string()
                };
                let node = RailNode::CharClass(escaped);
                let (node, new_pos) = apply_quantifier(node, chars, pos);
                pos = new_pos;
                items.push(node);
            }
            '.' => {
                pos += 1;
                let node = RailNode::CharClass(locale.label("any").to_string());
                let (node, new_pos) = apply_quantifier(node, chars, pos);
                pos = new_pos;
                items.push(node);
            }
            c => {
                let lit = c.to_string();
                pos += 1;
                let node = RailNode::Literal(lit);
                let (node, new_pos) = apply_quantifier(node, chars, pos);
                pos = new_pos;
                items.push(node);
            }
        }
    }
    if items.is_empty() {
        (RailNode::Empty, pos)
    } else if items.len() == 1 {
        (items.remove(0), pos)
    } else {
        (RailNode::Sequence(items), pos)
    }
}

fn regex_escape_label(ch: char, locale: RegexLocale) -> String {
    match ch {
        'd' => format!("\\d {}", locale.label("digit")),
        'D' => format!("\\D not {}", locale.label("digit")),
        'w' => format!("\\w {}", locale.label("word")),
        'W' => format!("\\W not {}", locale.label("word")),
        's' => format!("\\s {}", locale.label("space")),
        'S' => format!("\\S not {}", locale.label("space")),
        't' => "\\t tab".to_string(),
        'n' => "\\n newline".to_string(),
        other => format!("\\{other}"),
    }
}

fn apply_quantifier(node: RailNode, chars: &[char], pos: usize) -> (RailNode, usize) {
    if pos >= chars.len() {
        return (node, pos);
    }
    match chars[pos] {
        '*' => (RailNode::Repeat(Box::new(node)), pos + 1),
        '+' => (RailNode::OneOrMore(Box::new(node)), pos + 1),
        '?' => (RailNode::Optional(Box::new(node)), pos + 1),
        '{' => {
            let mut p = pos + 1;
            while p < chars.len() && chars[p] != '}' {
                p += 1;
            }
            if p < chars.len() && chars[p] == '}' {
                let spec: String = chars[pos..=p].iter().collect();
                return (RailNode::CountedRepeat(Box::new(node), spec), p + 1);
            }
            (node, pos)
        }
        _ => (node, pos),
    }
}

// ─── Family 2: @startebnf ─────────────────────────────────────────────────────

fn render_ebnf(source: &str) -> Result<String, Diagnostic> {
    let (body, doc_title) = strip_block(source, "@startebnf", "@endebnf");
    let (body, style, notes) = parse_ebnf_render_directives(body);

    // Parse rules: "name = body ;"
    let rules = parse_ebnf_rules(&body);
    if rules.is_empty() {
        return Err(Diagnostic::error(
            "[E_EBNF_EMPTY] @startebnf body contains no rules",
        ));
    }

    // Render each rule as a separate railroad diagram stacked vertically
    let mut rule_svgs: Vec<(String, RailNode)> = Vec::new();
    for (name, body_str) in &rules {
        let node = parse_ebnf_body(body_str);
        rule_svgs.push((name.clone(), node));
    }

    // Compute per-rule widths and heights
    let margin = 20;
    let label_h = 20;
    let gap_between = 24;

    let layouts: Vec<(String, RailLayout)> = rule_svgs
        .iter()
        .map(|(name, node)| (name.clone(), layout_rail_with_style(node, &style)))
        .collect();

    let max_inner_w = layouts.iter().map(|(_, l)| l.width).max().unwrap_or(200);
    let note_w = if notes.is_empty() { 0 } else { 220 };
    let svg_w = max_inner_w + margin * 2 + 40 + note_w;
    let total_h: i32 = layouts
        .iter()
        .map(|(_, l)| l.height + label_h + gap_between)
        .sum::<i32>()
        + margin * 2;

    let mut out = String::new();
    out.push_str(&svg_header(svg_w, total_h));
    out.push_str(svg_white_bg());
    out.push_str("<g data-ebnf-style=\"customizable\">");

    if let Some(title) = &doc_title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            svg_w / 2, margin + 16,
            escape_xml(title)
        ));
    }

    let mut y = margin + if doc_title.is_some() { 32 } else { 0 };

    for (name, layout) in &layouts {
        // Rule label
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"#555\">{} :=</text>",
            margin, y + 14, escape_xml(name)
        ));
        if let Some(note) = notes.get(name) {
            let nx = svg_w - note_w + 14;
            out.push_str(&format!(
                "<g data-ebnf-note-for=\"{}\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"34\" rx=\"6\" ry=\"6\" fill=\"#fff7ed\" stroke=\"#f97316\" stroke-width=\"1\"/><text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#7c2d12\">{}</text></g>",
                escape_xml(name),
                nx,
                y - 4,
                note_w - 28,
                nx + 8,
                y + 18,
                escape_xml(note)
            ));
        }
        y += label_h;

        let mid_y = y + layout.mid_y + 10;

        // Entry circle + line
        out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"#333\"/>",
            margin, mid_y
        ));
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
            margin,
            mid_y,
            margin + 20,
            mid_y
        ));

        // Diagram content
        out.push_str(&format!(
            "<g transform=\"translate({},{})\">{}</g>",
            margin + 20,
            y + 10,
            layout.svg
        ));

        // Exit track + double circle
        let exit_x = margin + 20 + layout.width;
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#666\" stroke-width=\"1.5\"/>",
            exit_x,
            mid_y,
            exit_x + 20,
            mid_y
        ));
        out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"none\" stroke=\"#333\" stroke-width=\"2\"/>",
            exit_x + 20, mid_y
        ));
        out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"3\" fill=\"#333\"/>",
            exit_x + 20,
            mid_y
        ));

        y += layout.height + gap_between + 10;
    }

    out.push_str("</g>");
    out.push_str("</svg>");
    Ok(out)
}

fn parse_ebnf_render_directives(body: &str) -> (String, RailStyle, BTreeMap<String, String>) {
    let mut style = RailStyle::default();
    let mut notes = BTreeMap::new();
    let mut rule_lines = Vec::new();

    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() {
            rule_lines.push(raw.to_string());
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if let Some((name, note)) = parse_ebnf_note(line) {
            notes.insert(name, note);
            continue;
        }
        if lower.starts_with("style ") || lower.starts_with("skinparam ") {
            apply_ebnf_style_directive(line, &mut style);
            continue;
        }
        rule_lines.push(raw.to_string());
    }

    (rule_lines.join("\n"), style, notes)
}

fn parse_ebnf_note(line: &str) -> Option<(String, String)> {
    let lower = line.to_ascii_lowercase();
    let rest = lower.strip_prefix("note ")?;
    let source_rest = &line[line.len() - rest.len()..];
    let (name, note) = source_rest.split_once(':')?;
    let name = name.trim().trim_matches('"').to_string();
    let note = note.trim().trim_matches('"').to_string();
    if name.is_empty() || note.is_empty() {
        None
    } else {
        Some((name, note))
    }
}

fn apply_ebnf_style_directive(line: &str, style: &mut RailStyle) {
    let lower = line.to_ascii_lowercase();
    let words: Vec<&str> = line.split_whitespace().collect();
    let color = words
        .iter()
        .rev()
        .find(|word| word.starts_with('#'))
        .copied();
    let Some(color) = color else {
        return;
    };
    if lower.contains("terminal") && !lower.contains("nonterminal") {
        style.literal_fill = color.to_string();
    } else if lower.contains("nonterminal") || lower.contains("non-terminal") {
        style.nonterminal_fill = color.to_string();
    } else if lower.contains("charclass") || lower.contains("characterclass") {
        style.charclass_fill = color.to_string();
    } else if lower.contains("anchor") {
        style.anchor_fill = color.to_string();
    }
}

/// Parse EBNF rules: "name = body ;" lines, possibly multi-line.
fn parse_ebnf_rules(body: &str) -> Vec<(String, String)> {
    let mut rules = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_body = String::new();

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Check if line contains '=' (new rule start)
        if let Some(eq_pos) = line.find('=').or_else(|| line.find("::=")) {
            // Save previous rule if any
            if let Some(name) = current_name.take() {
                let trimmed = current_body.trim().trim_end_matches(';').trim().to_string();
                rules.push((name, trimmed));
                current_body.clear();
            }
            let name = line[..eq_pos]
                .trim()
                .trim_end_matches(':')
                .trim()
                .to_string();
            let rest_start = if line[eq_pos..].starts_with("::=") {
                eq_pos + 3
            } else {
                eq_pos + 1
            };
            let rest = line[rest_start..].trim();
            current_name = Some(name);
            // Check if rule ends on same line
            if rest.ends_with(';') {
                let body_str = rest.trim_end_matches(';').trim().to_string();
                rules.push((current_name.take().unwrap(), body_str));
                current_body.clear();
            } else {
                current_body = rest.to_string();
            }
        } else if current_name.is_some() {
            if !current_body.is_empty() {
                current_body.push(' ');
            }
            if line.ends_with(';') {
                current_body.push_str(line.trim_end_matches(';').trim());
                let name = current_name.take().unwrap();
                let trimmed = current_body.trim().to_string();
                rules.push((name, trimmed));
                current_body.clear();
            } else {
                current_body.push_str(line);
            }
        }
    }
    // Flush last rule
    if let Some(name) = current_name {
        let trimmed = current_body.trim().trim_end_matches(';').trim().to_string();
        if !trimmed.is_empty() {
            rules.push((name, trimmed));
        }
    }
    rules
}

/// Parse an EBNF body expression into a RailNode.
/// Grammar: body = alt ; alt = seq { '|' seq } ; seq = item { ',' item } ;
/// item = group | repeat | optional | literal | name ;
fn parse_ebnf_body(body: &str) -> RailNode {
    let tokens = tokenize_ebnf(body);
    if tokens.is_empty() {
        return RailNode::Empty;
    }
    let (node, _) = ebnf_parse_alternation(&tokens, 0);
    node
}

#[derive(Debug, Clone, PartialEq)]
enum EbnfToken {
    Comma,
    Pipe,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    LParen,
    RParen,
    Semicolon,
    Literal(String),
    Ident(String),
}

fn tokenize_ebnf(s: &str) -> Vec<EbnfToken> {
    let mut tokens = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
            }
            ',' => {
                chars.next();
                tokens.push(EbnfToken::Comma);
            }
            '|' => {
                chars.next();
                tokens.push(EbnfToken::Pipe);
            }
            '{' => {
                chars.next();
                tokens.push(EbnfToken::LBrace);
            }
            '}' => {
                chars.next();
                tokens.push(EbnfToken::RBrace);
            }
            '[' => {
                chars.next();
                tokens.push(EbnfToken::LBracket);
            }
            ']' => {
                chars.next();
                tokens.push(EbnfToken::RBracket);
            }
            '(' => {
                chars.next();
                tokens.push(EbnfToken::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(EbnfToken::RParen);
            }
            ';' => {
                chars.next();
                tokens.push(EbnfToken::Semicolon);
            }
            '"' | '\'' => {
                let quote = c;
                chars.next();
                let mut lit = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch == quote {
                        chars.next();
                        break;
                    }
                    lit.push(ch);
                    chars.next();
                }
                tokens.push(EbnfToken::Literal(lit));
            }
            _ if c.is_alphanumeric() || c == '_' => {
                let mut ident = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                        ident.push(ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(EbnfToken::Ident(ident));
            }
            _ => {
                chars.next();
            }
        }
    }
    tokens
}

fn ebnf_parse_alternation(tokens: &[EbnfToken], pos: usize) -> (RailNode, usize) {
    let mut branches = Vec::new();
    let (first, mut p) = ebnf_parse_sequence(tokens, pos);
    branches.push(first);
    while p < tokens.len() && tokens[p] == EbnfToken::Pipe {
        p += 1;
        let (branch, new_p) = ebnf_parse_sequence(tokens, p);
        branches.push(branch);
        p = new_p;
    }
    if branches.len() == 1 {
        (branches.remove(0), p)
    } else {
        (RailNode::Alternation(branches), p)
    }
}

fn ebnf_parse_sequence(tokens: &[EbnfToken], pos: usize) -> (RailNode, usize) {
    let mut items = Vec::new();
    let (first, mut p) = ebnf_parse_item(tokens, pos);
    items.push(first);
    while p < tokens.len() && tokens[p] == EbnfToken::Comma {
        p += 1;
        let (item, new_p) = ebnf_parse_item(tokens, p);
        items.push(item);
        p = new_p;
    }
    if items.len() == 1 {
        (items.remove(0), p)
    } else {
        (RailNode::Sequence(items), p)
    }
}

fn ebnf_parse_item(tokens: &[EbnfToken], pos: usize) -> (RailNode, usize) {
    if pos >= tokens.len() {
        return (RailNode::Empty, pos);
    }
    match &tokens[pos] {
        EbnfToken::LBrace => {
            let (inner, p) = ebnf_parse_alternation(tokens, pos + 1);
            let p = if p < tokens.len() && tokens[p] == EbnfToken::RBrace {
                p + 1
            } else {
                p
            };
            (RailNode::Repeat(Box::new(inner)), p)
        }
        EbnfToken::LBracket => {
            let (inner, p) = ebnf_parse_alternation(tokens, pos + 1);
            let p = if p < tokens.len() && tokens[p] == EbnfToken::RBracket {
                p + 1
            } else {
                p
            };
            (RailNode::Optional(Box::new(inner)), p)
        }
        EbnfToken::LParen => {
            let (inner, p) = ebnf_parse_alternation(tokens, pos + 1);
            let p = if p < tokens.len() && tokens[p] == EbnfToken::RParen {
                p + 1
            } else {
                p
            };
            (inner, p)
        }
        EbnfToken::Literal(s) => (RailNode::Literal(s.clone()), pos + 1),
        EbnfToken::Ident(s) => (RailNode::NonTerminal(s.clone()), pos + 1),
        _ => (RailNode::Empty, pos + 1),
    }
}

// ─── Family 3: @startchart ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChartType {
    Bar,
    Line,
    Area,
    Scatter,
    Pie,
    Column, // same as bar but explicit
}

#[derive(Debug, Clone)]
struct ChartData {
    label: String,
    value: f64,
}

#[derive(Debug, Clone)]
struct ChartAnnotation {
    target: String,
    text: String,
}

#[derive(Debug, Clone)]
struct ChartRenderOptions {
    background: String,
    axis_color: Option<String>,
    palette: Vec<String>,
    annotations: Vec<ChartAnnotation>,
    caption: Option<String>,
}

impl Default for ChartRenderOptions {
    fn default() -> Self {
        Self {
            background: "white".to_string(),
            axis_color: None,
            palette: Vec::new(),
            annotations: Vec::new(),
            caption: None,
        }
    }
}

fn render_chart(source: &str) -> Result<String, Diagnostic> {
    // Parse @startchart <type> header
    let first_line = source.lines().next().unwrap_or("").trim();
    let chart_type_str = first_line
        .to_ascii_lowercase()
        .strip_prefix("@startchart")
        .unwrap_or("")
        .trim()
        .to_string();
    let chart_type = match chart_type_str.split_whitespace().next().unwrap_or("") {
        "line" => ChartType::Line,
        "area" => ChartType::Area,
        "scatter" => ChartType::Scatter,
        "pie" => ChartType::Pie,
        "column" => ChartType::Column,
        _ => ChartType::Bar, // "bar" is default
    };

    let (body, _) = strip_block(source, "@startchart", "@endchart");
    let mut title: Option<String> = None;
    let mut data: Vec<ChartData> = Vec::new();
    let mut options = ChartRenderOptions::default();

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("title ") {
            title = Some(line[6..].trim().to_string());
            continue;
        }
        if lower.starts_with("caption ") {
            options.caption = Some(line[8..].trim().to_string());
            continue;
        }
        if let Some(theme_name) = line.strip_prefix("!theme ") {
            let preset = resolve_sequence_theme_preset(theme_name).map_err(Diagnostic::error)?;
            options.background = preset
                .style
                .background_color
                .unwrap_or_else(|| "white".to_string());
            options.axis_color = Some(preset.style.arrow_color);
            options.palette = vec![
                preset.style.participant_border_color,
                preset.style.lifeline_border_color,
                preset.style.group_border_color,
            ];
            continue;
        }
        if parse_chart_annotation(line, &mut options.annotations) {
            continue;
        }
        if parse_chart_style(line, &mut options) {
            continue;
        }
        // Parse `"label" : value`, `label : value`, or `"label" value` (legacy)
        let sep_pos = line.rfind(':').or_else(|| {
            // Fall back to splitting on the last whitespace if no colon
            line.rfind(char::is_whitespace)
        });
        if let Some(pos) = sep_pos {
            let label_part = line[..pos]
                .trim_end_matches(':')
                .trim()
                .trim_matches('"')
                .to_string();
            let val_part = line[pos + 1..].trim();
            if let Ok(val) = val_part.parse::<f64>() {
                if !label_part.is_empty() {
                    data.push(ChartData {
                        label: label_part,
                        value: val,
                    });
                }
            }
        }
    }

    if data.is_empty() {
        return Err(Diagnostic::error(
            "[E_CHART_EMPTY] @startchart contains no data rows",
        ));
    }

    let svg = match chart_type {
        ChartType::Bar | ChartType::Column => render_bar_chart(&data, &title, false),
        ChartType::Line => render_line_chart(&data, &title),
        ChartType::Area => render_area_chart(&data, &title),
        ChartType::Scatter => render_scatter_chart(&data, &title),
        ChartType::Pie => render_pie_chart(&data, &title),
    }?;
    Ok(apply_chart_render_options(svg, &options))
}

const CHART_COLORS: &[&str] = &[
    "#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f", "#edc948", "#b07aa1", "#ff9da7",
    "#9c755f", "#bab0ac",
];

fn bar_color(idx: usize) -> &'static str {
    CHART_COLORS[idx % CHART_COLORS.len()]
}

fn parse_chart_annotation(line: &str, annotations: &mut Vec<ChartAnnotation>) -> bool {
    let lower = line.to_ascii_lowercase();
    if let Some(rest) = lower
        .strip_prefix("annotation ")
        .or_else(|| lower.strip_prefix("annotate "))
    {
        let source_rest = &line[line.len() - rest.len()..];
        if let Some((target, text)) = source_rest.split_once(':') {
            annotations.push(ChartAnnotation {
                target: target.trim().trim_matches('"').to_string(),
                text: text.trim().trim_matches('"').to_string(),
            });
            return true;
        }
    }
    if let Some(rest) = lower.strip_prefix("note at ") {
        let source_rest = &line[line.len() - rest.len()..];
        if let Some((target, text)) = source_rest.split_once(':') {
            annotations.push(ChartAnnotation {
                target: target.trim().trim_matches('"').to_string(),
                text: text.trim().trim_matches('"').to_string(),
            });
            return true;
        }
    }
    if let Some(rest) = lower.strip_prefix("note ") {
        let source_rest = &line[line.len() - rest.len()..];
        if let Some((text, target)) = source_rest.split_once(" at ") {
            annotations.push(ChartAnnotation {
                target: target.trim().trim_matches('"').to_string(),
                text: text.trim().trim_matches('"').to_string(),
            });
            return true;
        }
    }
    false
}

fn parse_chart_style(line: &str, options: &mut ChartRenderOptions) -> bool {
    let lower = line.to_ascii_lowercase();
    let parts: Vec<&str> = line.split_whitespace().collect();
    if lower.starts_with("skinparam ") && lower.contains("backgroundcolor") {
        if let Some(color) = parts.last().filter(|part| part.starts_with('#')) {
            options.background = (*color).to_string();
        }
        return true;
    }
    if lower.starts_with("skinparam ") && lower.contains("axiscolor") {
        if let Some(color) = parts.last().filter(|part| part.starts_with('#')) {
            options.axis_color = Some((*color).to_string());
        }
        return true;
    }
    if lower.starts_with("skinparam ") && lower.contains("chart") {
        if let Some(color) = parts.last().filter(|part| part.starts_with('#')) {
            options.palette.push((*color).to_string());
        }
        return true;
    }
    if lower.starts_with("palette ") {
        options.palette = parts
            .iter()
            .skip(1)
            .filter(|part| part.starts_with('#'))
            .map(|part| (*part).to_string())
            .collect();
        return true;
    }
    false
}

fn apply_chart_render_options(mut svg: String, options: &ChartRenderOptions) -> String {
    if options.background != "white" {
        svg = svg.replacen(
            "fill=\"white\"/>",
            &format!("fill=\"{}\"/>", escape_xml(&options.background)),
            1,
        );
    }
    if let Some(axis_color) = &options.axis_color {
        svg = svg.replace(
            "stroke=\"#888\"",
            &format!("stroke=\"{}\"", escape_xml(axis_color)),
        );
    }
    let mut additions = String::new();
    if !options.palette.is_empty() {
        additions.push_str(&format!(
            "<metadata data-chart-palette=\"{}\"/>",
            escape_xml(&options.palette.join(" "))
        ));
    }
    let mut y = 34;
    for ann in &options.annotations {
        additions.push_str(&format!(
            "<g data-chart-annotation=\"{}\"><rect x=\"560\" y=\"{}\" width=\"190\" height=\"24\" rx=\"5\" ry=\"5\" fill=\"#fff7ed\" stroke=\"#f97316\" stroke-width=\"1\"/><text x=\"570\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#7c2d12\">{}: {}</text></g>",
            escape_xml(&ann.target),
            y,
            y + 16,
            escape_xml(&ann.target),
            escape_xml(&ann.text)
        ));
        y += 30;
    }
    if let Some(caption) = &options.caption {
        additions.push_str(&format!(
            "<text data-chart-caption=\"true\" x=\"50%\" y=\"96%\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
            escape_xml(caption)
        ));
    }
    if additions.is_empty() {
        svg
    } else {
        svg.replace("</svg>", &format!("{additions}</svg>"))
    }
}

fn render_bar_chart(
    data: &[ChartData],
    title: &Option<String>,
    _vertical: bool,
) -> Result<String, Diagnostic> {
    let margin_left = 50i32;
    let margin_right = 20i32;
    let margin_top = if title.is_some() { 48 } else { 20 };
    let margin_bottom = 40i32;
    let chart_w = (data.len() as i32) * 60 + margin_left + margin_right;
    let chart_h = 300i32;
    let plot_w = chart_w - margin_left - margin_right;
    let plot_h = chart_h - margin_top - margin_bottom;

    let max_val = data.iter().map(|d| d.value).fold(0.0f64, f64::max);
    if max_val == 0.0 {
        return Err(Diagnostic::error(
            "[E_CHART_ZERO] chart has all-zero values",
        ));
    }

    let bar_w = (plot_w / data.len() as i32 - 8).max(8);
    let x_step = plot_w / data.len() as i32;

    let mut out = String::new();
    out.push_str(&svg_header(chart_w, chart_h));
    out.push_str(svg_white_bg());

    // Title
    if let Some(t) = title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"24\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            chart_w / 2, escape_xml(t)
        ));
    }

    // Axes
    let ax = margin_left;
    let ay_top = margin_top;
    let ay_bot = chart_h - margin_bottom;
    let ax_right = chart_w - margin_right;
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\" stroke-width=\"1.5\"/>",
        ax, ay_top, ax, ay_bot
    ));
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\" stroke-width=\"1.5\"/>",
        ax, ay_bot, ax_right, ay_bot
    ));

    // Y-axis labels
    for i in 0..=4 {
        let val = max_val * (i as f64) / 4.0;
        let y_pos = ay_bot - (plot_h * i / 4);
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#ddd\" stroke-width=\"1\"/>",
            ax, y_pos, ax_right, y_pos
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#666\" text-anchor=\"end\">{:.0}</text>",
            ax - 4, y_pos + 4, val
        ));
    }

    // Bars
    for (idx, d) in data.iter().enumerate() {
        let bx = ax + idx as i32 * x_step + (x_step - bar_w) / 2;
        let bar_h = ((d.value / max_val) * plot_h as f64) as i32;
        let by = ay_bot - bar_h;
        let color = bar_color(idx);
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" rx=\"2\"/>",
            bx, by, bar_w, bar_h, color
        ));
        // Value label on top
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" text-anchor=\"middle\" fill=\"{}\">{:.0}</text>",
            bx + bar_w / 2, by - 3, color, d.value
        ));
        // X-axis label
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" text-anchor=\"middle\" fill=\"#444\">{}</text>",
            bx + bar_w / 2, ay_bot + 14, escape_xml(&d.label)
        ));
    }

    out.push_str("</svg>");
    Ok(out)
}

fn render_line_chart(data: &[ChartData], title: &Option<String>) -> Result<String, Diagnostic> {
    let margin_left = 50i32;
    let margin_right = 20i32;
    let margin_top = if title.is_some() { 48 } else { 20 };
    let margin_bottom = 40i32;
    let chart_w = (data.len() as i32) * 60 + margin_left + margin_right;
    let chart_h = 300i32;
    let plot_w = chart_w - margin_left - margin_right;
    let plot_h = chart_h - margin_top - margin_bottom;

    let max_val = data.iter().map(|d| d.value).fold(0.0f64, f64::max);
    if max_val == 0.0 {
        return Err(Diagnostic::error(
            "[E_CHART_ZERO] chart has all-zero values",
        ));
    }

    let x_step = plot_w / (data.len() as i32 - 1).max(1);
    let ax = margin_left;
    let ay_top = margin_top;
    let ay_bot = chart_h - margin_bottom;
    let ax_right = chart_w - margin_right;

    let mut out = String::new();
    out.push_str(&svg_header(chart_w, chart_h));
    out.push_str(svg_white_bg());

    if let Some(t) = title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"24\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            chart_w / 2, escape_xml(t)
        ));
    }

    // Axes
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\" stroke-width=\"1.5\"/>",
        ax, ay_top, ax, ay_bot
    ));
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\" stroke-width=\"1.5\"/>",
        ax, ay_bot, ax_right, ay_bot
    ));

    // Y-grid
    for i in 0..=4 {
        let val = max_val * (i as f64) / 4.0;
        let y_pos = ay_bot - (plot_h * i / 4);
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#eee\" stroke-width=\"1\"/>",
            ax, y_pos, ax_right, y_pos
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"#666\" text-anchor=\"end\">{:.0}</text>",
            ax - 4, y_pos + 4, val
        ));
    }

    // Compute point coords
    let points: Vec<(i32, i32)> = data
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let px = ax + i as i32 * x_step;
            let py = ay_bot - ((d.value / max_val) * plot_h as f64) as i32;
            (px, py)
        })
        .collect();

    // Polyline
    if points.len() >= 2 {
        let pts_str: String = points
            .iter()
            .map(|(x, y)| format!("{},{}", x, y))
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(
            "<polyline points=\"{}\" fill=\"none\" stroke=\"#4e79a7\" stroke-width=\"2\"/>",
            pts_str
        ));
    }

    // Points and labels
    for (i, (px, py)) in points.iter().enumerate() {
        out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"4\" fill=\"#4e79a7\" stroke=\"white\" stroke-width=\"1.5\"/>",
            px, py
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" text-anchor=\"middle\" fill=\"#444\">{}</text>",
            px, ay_bot + 14, escape_xml(&data[i].label)
        ));
    }

    out.push_str("</svg>");
    Ok(out)
}

fn render_area_chart(data: &[ChartData], title: &Option<String>) -> Result<String, Diagnostic> {
    let margin_left = 50i32;
    let margin_right = 20i32;
    let margin_top = if title.is_some() { 48 } else { 20 };
    let margin_bottom = 40i32;
    let chart_w = (data.len() as i32) * 60 + margin_left + margin_right;
    let chart_h = 300i32;
    let plot_w = chart_w - margin_left - margin_right;
    let plot_h = chart_h - margin_top - margin_bottom;

    let max_val = data.iter().map(|d| d.value).fold(0.0f64, f64::max);
    if max_val == 0.0 {
        return Err(Diagnostic::error(
            "[E_CHART_ZERO] chart has all-zero values",
        ));
    }

    let x_step = plot_w / (data.len() as i32 - 1).max(1);
    let ax = margin_left;
    let ay_top = margin_top;
    let ay_bot = chart_h - margin_bottom;
    let ax_right = chart_w - margin_right;

    let mut out = String::new();
    out.push_str(&svg_header(chart_w, chart_h));
    out.push_str(svg_white_bg());
    if let Some(t) = title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"24\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            chart_w / 2, escape_xml(t)
        ));
    }

    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\" stroke-width=\"1.5\"/>",
        ax, ay_top, ax, ay_bot
    ));
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\" stroke-width=\"1.5\"/>",
        ax, ay_bot, ax_right, ay_bot
    ));

    let points: Vec<(i32, i32)> = data
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let px = ax + i as i32 * x_step;
            let py = ay_bot - ((d.value / max_val) * plot_h as f64) as i32;
            (px, py)
        })
        .collect();

    if points.len() >= 2 {
        let mut area_points = format!("{},{} ", points[0].0, ay_bot);
        area_points.push_str(
            &points
                .iter()
                .map(|(x, y)| format!("{},{}", x, y))
                .collect::<Vec<_>>()
                .join(" "),
        );
        area_points.push_str(&format!(" {},{}", points[points.len() - 1].0, ay_bot));
        out.push_str(&format!(
            "<polygon points=\"{}\" fill=\"#4e79a733\" stroke=\"none\"/>",
            area_points
        ));
        out.push_str(&format!(
            "<polyline points=\"{}\" fill=\"none\" stroke=\"#4e79a7\" stroke-width=\"2\"/>",
            points
                .iter()
                .map(|(x, y)| format!("{},{}", x, y))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }

    for (i, (px, py)) in points.iter().enumerate() {
        out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"3.5\" fill=\"#4e79a7\" stroke=\"white\" stroke-width=\"1.2\"/>",
            px, py
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" text-anchor=\"middle\" fill=\"#444\">{}</text>",
            px, ay_bot + 14, escape_xml(&data[i].label)
        ));
    }

    out.push_str("</svg>");
    Ok(out)
}

fn render_scatter_chart(data: &[ChartData], title: &Option<String>) -> Result<String, Diagnostic> {
    let margin_left = 50i32;
    let margin_right = 20i32;
    let margin_top = if title.is_some() { 48 } else { 20 };
    let margin_bottom = 40i32;
    let chart_w = (data.len() as i32) * 60 + margin_left + margin_right;
    let chart_h = 300i32;
    let plot_w = chart_w - margin_left - margin_right;
    let plot_h = chart_h - margin_top - margin_bottom;

    let max_val = data.iter().map(|d| d.value).fold(0.0f64, f64::max);
    if max_val == 0.0 {
        return Err(Diagnostic::error(
            "[E_CHART_ZERO] chart has all-zero values",
        ));
    }

    let x_step = plot_w / (data.len() as i32 - 1).max(1);
    let ax = margin_left;
    let ay_top = margin_top;
    let ay_bot = chart_h - margin_bottom;
    let ax_right = chart_w - margin_right;

    let mut out = String::new();
    out.push_str(&svg_header(chart_w, chart_h));
    out.push_str(svg_white_bg());
    if let Some(t) = title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"24\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            chart_w / 2, escape_xml(t)
        ));
    }
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\" stroke-width=\"1.5\"/>",
        ax, ay_top, ax, ay_bot
    ));
    out.push_str(&format!(
        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#888\" stroke-width=\"1.5\"/>",
        ax, ay_bot, ax_right, ay_bot
    ));
    for (i, d) in data.iter().enumerate() {
        let px = ax + i as i32 * x_step;
        let py = ay_bot - ((d.value / max_val) * plot_h as f64) as i32;
        let color = bar_color(i);
        out.push_str(&format!(
            "<circle cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{}\" fill-opacity=\"0.85\" stroke=\"white\" stroke-width=\"1\"/>",
            px, py, color
        ));
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" text-anchor=\"middle\" fill=\"#444\">{}</text>",
            px, ay_bot + 14, escape_xml(&d.label)
        ));
    }
    out.push_str("</svg>");
    Ok(out)
}

fn render_pie_chart(data: &[ChartData], title: &Option<String>) -> Result<String, Diagnostic> {
    let total: f64 = data.iter().map(|d| d.value).sum();
    if total == 0.0 {
        return Err(Diagnostic::error(
            "[E_CHART_ZERO] pie chart has all-zero values",
        ));
    }

    let legend_w = 180i32;
    let diagram_size = 300i32;
    let chart_w = diagram_size + legend_w + 20;
    let chart_h = diagram_size + 40;
    let cx = diagram_size / 2;
    let cy = diagram_size / 2 + 30;
    let r = (diagram_size / 2 - 20).min(110);

    let mut out = String::new();
    out.push_str(&svg_header(chart_w, chart_h));
    out.push_str(svg_white_bg());

    if let Some(t) = title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"20\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            diagram_size / 2, escape_xml(t)
        ));
    }

    let mut start_angle: f64 = -90.0_f64.to_radians(); // start from top

    for (idx, d) in data.iter().enumerate() {
        let sweep = (d.value / total) * 2.0 * std::f64::consts::PI;
        let end_angle = start_angle + sweep;
        let large_arc = if sweep > std::f64::consts::PI { 1 } else { 0 };

        let x1 = cx as f64 + r as f64 * start_angle.cos();
        let y1 = cy as f64 + r as f64 * start_angle.sin();
        let x2 = cx as f64 + r as f64 * end_angle.cos();
        let y2 = cy as f64 + r as f64 * end_angle.sin();

        let color = CHART_COLORS[idx % CHART_COLORS.len()];
        out.push_str(&format!(
            "<path d=\"M {} {} L {} {} A {} {} 0 {} 1 {} {} Z\" fill=\"{}\" stroke=\"white\" stroke-width=\"1.5\"/>",
            cx, cy,
            x1 as i32, y1 as i32,
            r, r, large_arc,
            x2 as i32, y2 as i32,
            color
        ));

        // Mid-angle for label positioning
        let mid_angle = start_angle + sweep / 2.0;
        let label_r = r as f64 * 0.65;
        let lx = cx as f64 + label_r * mid_angle.cos();
        let ly = cy as f64 + label_r * mid_angle.sin();
        let pct = (d.value / total * 100.0) as i32;
        if pct >= 5 {
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"white\" font-weight=\"600\">{}%</text>",
                lx as i32, ly as i32, pct
            ));
        }

        start_angle = end_angle;
    }

    // Legend
    let legend_x = diagram_size + 10;
    let mut legend_y = 40;
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" font-weight=\"600\" fill=\"#333\">Legend</text>",
        legend_x, legend_y
    ));
    legend_y += 18;
    for (idx, d) in data.iter().enumerate() {
        let color = CHART_COLORS[idx % CHART_COLORS.len()];
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"12\" height=\"12\" fill=\"{}\"/>",
            legend_x,
            legend_y - 10,
            color
        ));
        let label = format!("{} ({:.0})", d.label, d.value);
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#333\">{}</text>",
            legend_x + 18, legend_y, escape_xml(&label)
        ));
        legend_y += 18;
    }

    out.push_str("</svg>");
    Ok(out)
}

// ─── Family 5a: @startsdl ────────────────────────────────────────────────────

fn render_sdl(source: &str) -> Result<String, Diagnostic> {
    let (body, title) = strip_block(source, "@startsdl", "@endsdl");

    let mut states: Vec<String> = Vec::new();
    let mut state_kinds: std::collections::BTreeMap<String, SdlStateKindLocal> =
        std::collections::BTreeMap::new();
    let mut transitions: Vec<(String, String, Option<String>)> = Vec::new();

    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("state ") {
            let rest = line[6..].trim();
            // Check if it's a transition: "state A -> B : label"
            if let Some(arrow_pos) = rest.find("->") {
                let from = rest[..arrow_pos].trim().to_string();
                let rest2 = &rest[arrow_pos + 2..];
                let (to, label) = if let Some(colon_pos) = rest2.find(':') {
                    let to = rest2[..colon_pos].trim().to_string();
                    let label = rest2[colon_pos + 1..].trim().to_string();
                    (to, if label.is_empty() { None } else { Some(label) })
                } else {
                    (rest2.trim().to_string(), None)
                };
                // Add states implicitly
                if !states.contains(&from) {
                    states.push(from.clone());
                }
                if !states.contains(&to) {
                    states.push(to.clone());
                }
                transitions.push((from, to, label));
            } else {
                // Pure state declaration
                let (name, kind) = parse_sdl_state_decl(rest);
                if !name.is_empty() && !states.contains(&name) {
                    states.push(name.clone());
                }
                if !name.is_empty() {
                    state_kinds.insert(name, kind);
                }
            }
        }
    }

    if states.is_empty() {
        return Err(Diagnostic::error(
            "[E_SDL_EMPTY] @startsdl contains no states",
        ));
    }

    // Layout: arrange states in a grid (2 columns)
    let cols = 2i32;
    let state_w = 160i32;
    let state_h = 40i32;
    let gap_x = 80i32;
    let gap_y = 60i32;
    let margin = 30i32;
    let title_h = if title.is_some() { 32 } else { 0 };

    let n = states.len() as i32;
    let rows = (n + cols - 1) / cols;
    let total_w = margin * 2 + cols * state_w + (cols - 1) * gap_x;
    let total_h = margin * 2 + title_h + rows * state_h + (rows - 1) * gap_y + 20;

    // Assign coords
    let mut coords: std::collections::BTreeMap<String, (i32, i32)> =
        std::collections::BTreeMap::new();
    for (idx, name) in states.iter().enumerate() {
        let col = idx as i32 % cols;
        let row = idx as i32 / cols;
        let x = margin + col * (state_w + gap_x);
        let y = margin + title_h + row * (state_h + gap_y);
        coords.insert(name.clone(), (x, y));
    }

    let mut out = String::new();
    out.push_str(&svg_header(total_w, total_h));
    out.push_str(svg_white_bg());
    out.push_str("<defs><marker id=\"sdlarrow\" markerWidth=\"8\" markerHeight=\"8\" refX=\"6\" refY=\"3\" orient=\"auto\"><path d=\"M0,0 L0,6 L8,3 z\" fill=\"#444\"/></marker></defs>");

    if let Some(t) = &title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            total_w / 2, margin + 18, escape_xml(t)
        ));
    }

    // Draw transitions first (under nodes)
    for (from, to, label) in &transitions {
        if let (Some(&(fx, fy)), Some(&(tx, ty))) = (coords.get(from), coords.get(to)) {
            let (x1, y1, x2, y2) = sdl_transition_endpoints(fx, fy, tx, ty, state_w, state_h);
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#444\" stroke-width=\"1.5\" marker-end=\"url(#sdlarrow)\"/>",
                x1, y1, x2, y2
            ));
            if let Some(lbl) = label {
                let mx = (x1 + x2) / 2;
                let my = (y1 + y2) / 2 - 6;
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#555\" text-anchor=\"middle\">{}</text>",
                    mx, my, escape_xml(lbl)
                ));
            }
        }
    }

    // Draw SDL state nodes
    for name in &states {
        if let Some(&(x, y)) = coords.get(name) {
            let kind = *state_kinds.get(name).unwrap_or(&SdlStateKindLocal::Normal);
            render_sdl_state_node(&mut out, name, kind, x, y, state_w, state_h);
        }
    }

    out.push_str("</svg>");
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SdlStateKindLocal {
    Normal,
    Start,
    End,
    Input,
    Output,
    Decision,
}

fn parse_sdl_state_decl(raw: &str) -> (String, SdlStateKindLocal) {
    let mut text = raw.trim().to_string();
    let mut kind = SdlStateKindLocal::Normal;
    if let (Some(start), Some(end)) = (text.find("<<"), text.find(">>")) {
        if start < end {
            let tag = text[start + 2..end].trim().to_ascii_lowercase();
            kind = match tag.as_str() {
                "start" | "*" => SdlStateKindLocal::Start,
                "end" | "stop" => SdlStateKindLocal::End,
                "input" => SdlStateKindLocal::Input,
                "output" => SdlStateKindLocal::Output,
                "decision" => SdlStateKindLocal::Decision,
                _ => SdlStateKindLocal::Normal,
            };
            text = format!("{}{}", text[..start].trim(), text[end + 2..].trim());
            text = text.trim().to_string();
        }
    }
    (text, kind)
}

fn sdl_transition_endpoints(
    fx: i32,
    fy: i32,
    tx: i32,
    ty: i32,
    sw: i32,
    sh: i32,
) -> (i32, i32, i32, i32) {
    let fcx = fx + sw / 2;
    let fcy = fy + sh / 2;
    let tcx = tx + sw / 2;
    let tcy = ty + sh / 2;
    let dx = tcx - fcx;
    let dy = tcy - fcy;
    if dx.abs() >= dy.abs() {
        if dx >= 0 {
            (fcx + sw / 2, fcy, tcx - sw / 2, tcy)
        } else {
            (fcx - sw / 2, fcy, tcx + sw / 2, tcy)
        }
    } else if dy >= 0 {
        (fcx, fcy + sh / 2, tcx, tcy - sh / 2)
    } else {
        (fcx, fcy - sh / 2, tcx, tcy + sh / 2)
    }
}

/// Render an SDL state node: rounded-corner rectangle with slight color.
fn render_sdl_state_node(
    out: &mut String,
    name: &str,
    kind: SdlStateKindLocal,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    match kind {
        SdlStateKindLocal::Start => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"14\" fill=\"#1a237e\"/>",
                x + w / 2,
                y + h / 2
            ));
        }
        SdlStateKindLocal::End => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"14\" fill=\"none\" stroke=\"#1a237e\" stroke-width=\"2\"/>",
                x + w / 2,
                y + h / 2
            ));
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"9\" fill=\"#1a237e\"/>",
                x + w / 2,
                y + h / 2
            ));
        }
        SdlStateKindLocal::Decision => {
            let cx = x + w / 2;
            let cy = y + h / 2;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"#ede7f6\" stroke=\"#5e35b1\" stroke-width=\"2\"/>",
                cx, y, x + w, cy, cx, y + h, x, cy
            ));
        }
        SdlStateKindLocal::Input => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"#e3f2fd\" stroke=\"#1e88e5\" stroke-width=\"2\"/>",
                x, y, w, h
            ));
        }
        SdlStateKindLocal::Output => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"#e8f5e9\" stroke=\"#43a047\" stroke-width=\"2\"/>",
                x, y, w, h
            ));
        }
        SdlStateKindLocal::Normal => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"#e8eaf6\" stroke=\"#3949ab\" stroke-width=\"2\"/>",
                x, y, w, h
            ));
        }
    }
    if !matches!(kind, SdlStateKindLocal::Start) {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"#1a237e\">{}</text>",
            x + w / 2, y + h / 2, escape_xml(name)
        ));
    }
}

// ─── Family 1: @startmath ─────────────────────────────────────────────────────
//
// Real LaTeX expression tree with layout engine.

/// A node in the math expression AST.
#[derive(Debug, Clone)]
enum Expr {
    Literal(String),
    Text(String),
    Sub(Box<Expr>, Box<Expr>),
    Sup(Box<Expr>, Box<Expr>),
    SubSup(Box<Expr>, Box<Expr>, Box<Expr>),
    Frac(Box<Expr>, Box<Expr>),
    Sqrt(Box<Expr>),
    Accent {
        kind: String,
        inner: Box<Expr>,
    },
    Greek(char),
    Matrix {
        env: String,
        rows: Vec<Vec<Expr>>,
    },
    BigOp {
        op: char,
        sub: Box<Expr>,
        sup: Box<Expr>,
    },
    Group(Vec<Expr>),
}

/// Tokenizer output for LaTeX
#[derive(Debug, Clone)]
enum LatexToken {
    Char(char),
    Command(String),
    Sub,
    Sup,
    LBrace,
    RBrace,
    Space,
}

fn tokenize_latex_raw(s: &str) -> Vec<LatexToken> {
    let chars: Vec<char> = s.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                i += 1;
                if i >= chars.len() {
                    break;
                }
                if chars[i].is_alphabetic() {
                    let mut name = String::new();
                    while i < chars.len() && chars[i].is_alphabetic() {
                        name.push(chars[i]);
                        i += 1;
                    }
                    tokens.push(LatexToken::Command(name));
                } else {
                    // single-char escape like \\ \, etc.
                    tokens.push(LatexToken::Char(chars[i]));
                    i += 1;
                }
            }
            '_' => {
                tokens.push(LatexToken::Sub);
                i += 1;
            }
            '^' => {
                tokens.push(LatexToken::Sup);
                i += 1;
            }
            '{' => {
                tokens.push(LatexToken::LBrace);
                i += 1;
            }
            '}' => {
                tokens.push(LatexToken::RBrace);
                i += 1;
            }
            ' ' | '\t' | '\n' | '\r' => {
                tokens.push(LatexToken::Space);
                i += 1;
            }
            c => {
                tokens.push(LatexToken::Char(c));
                i += 1;
            }
        }
    }
    tokens
}

/// Map a LaTeX command name to an Expr node.
fn command_to_expr(name: &str) -> Expr {
    match name {
        // Greek lowercase
        "alpha" => Expr::Greek('α'),
        "beta" => Expr::Greek('β'),
        "gamma" => Expr::Greek('γ'),
        "delta" => Expr::Greek('δ'),
        "epsilon" => Expr::Greek('ε'),
        "varepsilon" => Expr::Greek('ε'),
        "zeta" => Expr::Greek('ζ'),
        "eta" => Expr::Greek('η'),
        "theta" => Expr::Greek('θ'),
        "vartheta" => Expr::Greek('ϑ'),
        "iota" => Expr::Greek('ι'),
        "kappa" => Expr::Greek('κ'),
        "lambda" => Expr::Greek('λ'),
        "mu" => Expr::Greek('μ'),
        "nu" => Expr::Greek('ν'),
        "xi" => Expr::Greek('ξ'),
        "pi" => Expr::Greek('π'),
        "varpi" => Expr::Greek('ϖ'),
        "rho" => Expr::Greek('ρ'),
        "sigma" => Expr::Greek('σ'),
        "tau" => Expr::Greek('τ'),
        "upsilon" => Expr::Greek('υ'),
        "phi" => Expr::Greek('φ'),
        "varphi" => Expr::Greek('φ'),
        "chi" => Expr::Greek('χ'),
        "psi" => Expr::Greek('ψ'),
        "omega" => Expr::Greek('ω'),
        // Greek uppercase
        "Alpha" => Expr::Greek('Α'),
        "Beta" => Expr::Greek('Β'),
        "Gamma" => Expr::Greek('Γ'),
        "Delta" => Expr::Greek('Δ'),
        "Epsilon" => Expr::Greek('Ε'),
        "Zeta" => Expr::Greek('Ζ'),
        "Eta" => Expr::Greek('Η'),
        "Theta" => Expr::Greek('Θ'),
        "Iota" => Expr::Greek('Ι'),
        "Kappa" => Expr::Greek('Κ'),
        "Lambda" => Expr::Greek('Λ'),
        "Mu" => Expr::Greek('Μ'),
        "Nu" => Expr::Greek('Ν'),
        "Xi" => Expr::Greek('Ξ'),
        "Pi" => Expr::Greek('Π'),
        "Rho" => Expr::Greek('Ρ'),
        "Sigma" => Expr::Greek('Σ'),
        "Tau" => Expr::Greek('Τ'),
        "Upsilon" => Expr::Greek('Υ'),
        "Phi" => Expr::Greek('Φ'),
        "Chi" => Expr::Greek('Χ'),
        "Psi" => Expr::Greek('Ψ'),
        "Omega" => Expr::Greek('Ω'),
        // Infinity
        "infty" | "infinity" => Expr::Greek('∞'),
        // Operators
        "pm" => Expr::Literal("±".to_string()),
        "mp" => Expr::Literal("∓".to_string()),
        "times" => Expr::Literal("×".to_string()),
        "div" => Expr::Literal("÷".to_string()),
        "leq" | "le" => Expr::Literal("≤".to_string()),
        "geq" | "ge" => Expr::Literal("≥".to_string()),
        "neq" | "ne" => Expr::Literal("≠".to_string()),
        "approx" => Expr::Literal("≈".to_string()),
        "rightarrow" | "to" => Expr::Literal("→".to_string()),
        "leftarrow" | "gets" => Expr::Literal("←".to_string()),
        "Rightarrow" => Expr::Literal("⇒".to_string()),
        "Leftarrow" => Expr::Literal("⇐".to_string()),
        "leftrightarrow" => Expr::Literal("↔".to_string()),
        "cdot" => Expr::Literal("·".to_string()),
        "cdots" => Expr::Literal("···".to_string()),
        "ldots" => Expr::Literal("…".to_string()),
        "partial" => Expr::Greek('∂'),
        "nabla" => Expr::Literal("∇".to_string()),
        "in" => Expr::Literal("∈".to_string()),
        "notin" => Expr::Literal("∉".to_string()),
        "subset" => Expr::Literal("⊂".to_string()),
        "supset" => Expr::Literal("⊃".to_string()),
        "cup" => Expr::Literal("∪".to_string()),
        "cap" => Expr::Literal("∩".to_string()),
        "forall" => Expr::Literal("∀".to_string()),
        "exists" => Expr::Literal("∃".to_string()),
        "emptyset" | "varnothing" => Expr::Literal("∅".to_string()),
        "land" | "wedge" => Expr::Literal("∧".to_string()),
        "lor" | "vee" => Expr::Literal("∨".to_string()),
        "neg" | "lnot" => Expr::Literal("¬".to_string()),
        "therefore" => Expr::Literal("∴".to_string()),
        "because" => Expr::Literal("∵".to_string()),
        "equiv" => Expr::Literal("≡".to_string()),
        "propto" => Expr::Literal("∝".to_string()),
        "sim" => Expr::Literal("∼".to_string()),
        "simeq" => Expr::Literal("≃".to_string()),
        "cong" => Expr::Literal("≅".to_string()),
        "ll" => Expr::Literal("≪".to_string()),
        "gg" => Expr::Literal("≫".to_string()),
        "subseteq" => Expr::Literal("⊆".to_string()),
        "supseteq" => Expr::Literal("⊇".to_string()),
        "oplus" => Expr::Literal("⊕".to_string()),
        "otimes" => Expr::Literal("⊗".to_string()),
        "perp" => Expr::Literal("⊥".to_string()),
        "parallel" => Expr::Literal("∥".to_string()),
        "angle" => Expr::Literal("∠".to_string()),
        "degree" => Expr::Literal("°".to_string()),
        "lfloor" => Expr::Literal("⌊".to_string()),
        "rfloor" => Expr::Literal("⌋".to_string()),
        "lceil" => Expr::Literal("⌈".to_string()),
        "rceil" => Expr::Literal("⌉".to_string()),
        "sin" | "cos" | "tan" | "cot" | "sec" | "csc" | "log" | "ln" | "lim" | "min"
        | "max" | "det" | "dim" | "ker" | "Pr" => Expr::Literal(name.to_string()),
        "," | ";" | ":" | "quad" | "qquad" => Expr::Literal(" ".to_string()),
        // Ignore decorators
        "left" | "right" | "big" | "bigg" | "Big" | "Bigg" => Expr::Literal(String::new()),
        "text" | "mathrm" | "mathit" | "mathbf" | "mathbb" | "mathcal" | "operatorname" => {
            // Will be handled specially in parser – return empty, content comes from next group
            Expr::Literal(String::new())
        }
        _ => Expr::Literal(format!("\\{}", name)),
    }
}

/// Returns true for big operator commands.
fn is_big_op(name: &str) -> Option<char> {
    match name {
        "sum" => Some('∑'),
        "int" => Some('∫'),
        "oint" => Some('∮'),
        "prod" => Some('∏'),
        "coprod" => Some('∐'),
        "bigoplus" => Some('⊕'),
        "bigotimes" => Some('⊗'),
        "bigcup" => Some('⋃'),
        "bigcap" => Some('⋂'),
        _ => None,
    }
}

/// Returns true for commands that need to consume following groups (\frac, \sqrt, etc.)
fn is_frac(name: &str) -> bool {
    matches!(name, "frac" | "dfrac" | "tfrac" | "cfrac")
}

fn is_sqrt(name: &str) -> bool {
    matches!(name, "sqrt" | "cbrt")
}

fn read_braced_literal(tokens: &[LatexToken], idx: &mut usize) -> Option<String> {
    skip_spaces(tokens, idx);
    if !matches!(tokens.get(*idx), Some(LatexToken::LBrace)) {
        return None;
    }
    *idx += 1;
    let mut out = String::new();
    while *idx < tokens.len() {
        match &tokens[*idx] {
            LatexToken::RBrace => {
                *idx += 1;
                return Some(out);
            }
            LatexToken::Char(c) => {
                out.push(*c);
                *idx += 1;
            }
            LatexToken::Command(name) => {
                out.push_str(name);
                *idx += 1;
            }
            LatexToken::Space => {
                out.push(' ');
                *idx += 1;
            }
            LatexToken::Sub => {
                out.push('_');
                *idx += 1;
            }
            LatexToken::Sup => {
                out.push('^');
                *idx += 1;
            }
            LatexToken::LBrace => {
                out.push('{');
                *idx += 1;
            }
        }
    }
    None
}

fn peek_end_env(tokens: &[LatexToken], idx: usize, env: &str) -> Option<usize> {
    if !matches!(tokens.get(idx), Some(LatexToken::Command(name)) if name == "end") {
        return None;
    }
    let mut cursor = idx + 1;
    let name = read_braced_literal(tokens, &mut cursor)?;
    if name.trim() == env {
        Some(cursor)
    } else {
        None
    }
}

fn parse_cell_expr(tokens: &[LatexToken]) -> Expr {
    let mut idx = 0;
    Expr::Group(parse_expr_seq(tokens, &mut idx))
}

fn parse_matrix_env(tokens: &[LatexToken], idx: &mut usize, env: &str) -> Expr {
    let mut rows: Vec<Vec<Expr>> = Vec::new();
    let mut row: Vec<Expr> = Vec::new();
    let mut cell: Vec<LatexToken> = Vec::new();
    let mut depth = 0usize;

    while *idx < tokens.len() {
        if depth == 0 {
            if let Some(end_idx) = peek_end_env(tokens, *idx, env) {
                row.push(parse_cell_expr(&cell));
                if !row.is_empty() {
                    rows.push(row);
                }
                *idx = end_idx;
                return Expr::Matrix {
                    env: env.to_string(),
                    rows,
                };
            }
            match &tokens[*idx] {
                LatexToken::Char('&') => {
                    row.push(parse_cell_expr(&cell));
                    cell.clear();
                    *idx += 1;
                    continue;
                }
                LatexToken::Char('\\') => {
                    row.push(parse_cell_expr(&cell));
                    cell.clear();
                    rows.push(row);
                    row = Vec::new();
                    *idx += 1;
                    continue;
                }
                _ => {}
            }
        }

        match &tokens[*idx] {
            LatexToken::LBrace => depth += 1,
            LatexToken::RBrace => depth = depth.saturating_sub(1),
            _ => {}
        }
        cell.push(tokens[*idx].clone());
        *idx += 1;
    }

    row.push(parse_cell_expr(&cell));
    rows.push(row);
    Expr::Matrix {
        env: env.to_string(),
        rows,
    }
}

fn is_matrix_env(name: &str) -> bool {
    matches!(
        name,
        "matrix" | "pmatrix" | "bmatrix" | "Bmatrix" | "vmatrix" | "Vmatrix" | "smallmatrix"
            | "array" | "aligned" | "align"
    )
}

/// Parse a sequence of LaTeX tokens into a Vec of Expr nodes.
fn parse_expr_seq(tokens: &[LatexToken], idx: &mut usize) -> Vec<Expr> {
    let mut exprs = Vec::new();
    while *idx < tokens.len() {
        match &tokens[*idx] {
            LatexToken::RBrace => {
                // End of a group context – stop parsing
                break;
            }
            LatexToken::Space => {
                *idx += 1;
            }
            LatexToken::LBrace => {
                *idx += 1; // consume '{'
                let inner = parse_expr_seq(tokens, idx);
                if *idx < tokens.len() {
                    *idx += 1; // consume '}'
                }
                exprs.push(Expr::Group(inner));
            }
            LatexToken::Sub => {
                *idx += 1;
                let base = exprs.pop().unwrap_or(Expr::Literal(String::new()));
                let sub = parse_single_expr(tokens, idx);
                skip_spaces(tokens, idx);
                if *idx < tokens.len() && matches!(tokens[*idx], LatexToken::Sup) {
                    *idx += 1;
                    let sup = parse_single_expr(tokens, idx);
                    exprs.push(Expr::SubSup(Box::new(base), Box::new(sub), Box::new(sup)));
                } else {
                    exprs.push(Expr::Sub(Box::new(base), Box::new(sub)));
                }
            }
            LatexToken::Sup => {
                *idx += 1;
                let base = exprs.pop().unwrap_or(Expr::Literal(String::new()));
                let sup = parse_single_expr(tokens, idx);
                skip_spaces(tokens, idx);
                if *idx < tokens.len() && matches!(tokens[*idx], LatexToken::Sub) {
                    *idx += 1;
                    let sub = parse_single_expr(tokens, idx);
                    exprs.push(Expr::SubSup(Box::new(base), Box::new(sub), Box::new(sup)));
                } else {
                    exprs.push(Expr::Sup(Box::new(base), Box::new(sup)));
                }
            }
            LatexToken::Char(c) => {
                exprs.push(Expr::Literal(c.to_string()));
                *idx += 1;
            }
            LatexToken::Command(name) => {
                let name = name.clone();
                *idx += 1;
                if name == "begin" {
                    if let Some(env) = read_braced_literal(tokens, idx) {
                        if is_matrix_env(env.trim()) {
                            exprs.push(parse_matrix_env(tokens, idx, env.trim()));
                        } else {
                            exprs.push(Expr::Literal(format!("\\begin{{{}}}", env)));
                        }
                    }
                } else if let Some(op_char) = is_big_op(&name) {
                    // Parse optional sub and sup
                    let mut sub = Expr::Literal(String::new());
                    let mut sup = Expr::Literal(String::new());
                    // Peek for _ or ^
                    loop {
                        skip_spaces(tokens, idx);
                        if *idx >= tokens.len() {
                            break;
                        }
                        match &tokens[*idx] {
                            LatexToken::Sub => {
                                *idx += 1;
                                sub = parse_single_expr(tokens, idx);
                            }
                            LatexToken::Sup => {
                                *idx += 1;
                                sup = parse_single_expr(tokens, idx);
                            }
                            _ => break,
                        }
                    }
                    exprs.push(Expr::BigOp {
                        op: op_char,
                        sub: Box::new(sub),
                        sup: Box::new(sup),
                    });
                } else if is_frac(&name) {
                    skip_spaces(tokens, idx);
                    let num = parse_single_expr(tokens, idx);
                    skip_spaces(tokens, idx);
                    let den = parse_single_expr(tokens, idx);
                    exprs.push(Expr::Frac(Box::new(num), Box::new(den)));
                } else if is_sqrt(&name) {
                    // Optionally skip [n] for nth root
                    skip_spaces(tokens, idx);
                    if *idx < tokens.len() {
                        if let LatexToken::Char('[') = &tokens[*idx] {
                            // consume until ']'
                            *idx += 1;
                            while *idx < tokens.len() {
                                if let LatexToken::Char(']') = &tokens[*idx] {
                                    *idx += 1;
                                    break;
                                }
                                *idx += 1;
                            }
                        }
                    }
                    skip_spaces(tokens, idx);
                    let inner = parse_single_expr(tokens, idx);
                    exprs.push(Expr::Sqrt(Box::new(inner)));
                } else if matches!(
                    name.as_str(),
                    "hat" | "bar" | "overline" | "underline" | "vec" | "dot" | "ddot"
                ) {
                    skip_spaces(tokens, idx);
                    let inner = parse_single_expr(tokens, idx);
                    exprs.push(Expr::Accent {
                        kind: name,
                        inner: Box::new(inner),
                    });
                } else if matches!(
                    name.as_str(),
                    "text" | "mathrm" | "mathit" | "mathbf" | "mathbb" | "mathcal" | "operatorname"
                ) {
                    // Consume following group and render it as literal text
                    skip_spaces(tokens, idx);
                    if let Some(text) = read_braced_literal(tokens, idx) {
                        exprs.push(Expr::Text(text));
                    } else {
                        let inner = parse_single_expr(tokens, idx);
                        exprs.push(inner);
                    }
                } else if matches!(
                    name.as_str(),
                    "left" | "right" | "big" | "bigg" | "Big" | "Bigg"
                ) {
                    // consume and render the following delimiter; \left. / \right. are invisible
                    if *idx < tokens.len() {
                        if let LatexToken::Char(c) = &tokens[*idx] {
                            if *c != '.' {
                                exprs.push(Expr::Literal(c.to_string()));
                            }
                            *idx += 1;
                        }
                    }
                } else {
                    exprs.push(command_to_expr(&name));
                }
            }
        }
    }
    exprs
}

fn skip_spaces(tokens: &[LatexToken], idx: &mut usize) {
    while *idx < tokens.len() {
        if let LatexToken::Space = &tokens[*idx] {
            *idx += 1;
        } else {
            break;
        }
    }
}

/// Parse exactly one expression node (either a group or a single token).
fn parse_single_expr(tokens: &[LatexToken], idx: &mut usize) -> Expr {
    skip_spaces(tokens, idx);
    if *idx >= tokens.len() {
        return Expr::Literal(String::new());
    }
    match &tokens[*idx].clone() {
        LatexToken::LBrace => {
            *idx += 1;
            let inner = parse_expr_seq(tokens, idx);
            if *idx < tokens.len() {
                *idx += 1; // consume '}'
            }
            Expr::Group(inner)
        }
        LatexToken::Char(c) => {
            let c = *c;
            *idx += 1;
            Expr::Literal(c.to_string())
        }
        LatexToken::Command(name) => {
            let name = name.clone();
            *idx += 1;
            if name == "begin" {
                if let Some(env) = read_braced_literal(tokens, idx) {
                    if is_matrix_env(env.trim()) {
                        parse_matrix_env(tokens, idx, env.trim())
                    } else {
                        Expr::Literal(format!("\\begin{{{}}}", env))
                    }
                } else {
                    Expr::Literal("\\begin".to_string())
                }
            } else if is_frac(&name) {
                skip_spaces(tokens, idx);
                let num = parse_single_expr(tokens, idx);
                skip_spaces(tokens, idx);
                let den = parse_single_expr(tokens, idx);
                Expr::Frac(Box::new(num), Box::new(den))
            } else if is_sqrt(&name) {
                skip_spaces(tokens, idx);
                let inner = parse_single_expr(tokens, idx);
                Expr::Sqrt(Box::new(inner))
            } else if matches!(
                name.as_str(),
                "hat" | "bar" | "overline" | "underline" | "vec" | "dot" | "ddot"
            ) {
                skip_spaces(tokens, idx);
                let inner = parse_single_expr(tokens, idx);
                Expr::Accent {
                    kind: name,
                    inner: Box::new(inner),
                }
            } else if matches!(
                name.as_str(),
                "text" | "mathrm" | "mathit" | "mathbf" | "mathbb" | "mathcal" | "operatorname"
            ) {
                skip_spaces(tokens, idx);
                if let Some(text) = read_braced_literal(tokens, idx) {
                    Expr::Text(text)
                } else {
                    parse_single_expr(tokens, idx)
                }
            } else {
                command_to_expr(&name)
            }
        }
        LatexToken::Sub | LatexToken::Sup => Expr::Literal(String::new()),
        LatexToken::RBrace | LatexToken::Space => Expr::Literal(String::new()),
    }
}

fn parse_math_expr(s: &str) -> Vec<Expr> {
    let tokens = tokenize_latex_raw(s);
    let mut idx = 0;
    parse_expr_seq(&tokens, &mut idx)
}

// ─── Layout engine ────────────────────────────────────────────────────────────

/// Layout result for an expression node.
struct Layout {
    svg: String,
    width: f64,
    height: f64,
    ascent: f64, // distance from top to baseline
}

const BASE_FONT: f64 = 20.0;
const CHAR_W_RATIO: f64 = 0.55; // approximate char width as fraction of font-size
const SUB_SCALE: f64 = 0.65;
const SUP_SCALE: f64 = 0.65;
const FRAC_PAD: f64 = 4.0;
const SQRT_LEAN: f64 = 8.0; // width of the radical foot

fn char_width(font_size: f64) -> f64 {
    font_size * CHAR_W_RATIO
}

fn layout_expr(expr: &Expr, font_size: f64) -> Layout {
    match expr {
        Expr::Literal(s) => {
            if s.is_empty() {
                return Layout {
                    svg: String::new(),
                    width: 0.0,
                    height: font_size,
                    ascent: font_size * 0.8,
                };
            }
            // Estimate width: each char is ~char_w
            let char_w = char_width(font_size);
            let width = s.chars().count() as f64 * char_w;
            let height = font_size * 1.2;
            let ascent = font_size * 0.8;
            let svg = format!(
                "<text x=\"0\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\">{}</text>",
                ascent,
                font_size,
                escape_xml(s)
            );
            Layout {
                svg,
                width,
                height,
                ascent,
            }
        }
        Expr::Text(s) => {
            let char_w = char_width(font_size);
            let width = s.chars().count() as f64 * char_w;
            let height = font_size * 1.2;
            let ascent = font_size * 0.8;
            let svg = format!(
                "<text x=\"0\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\">{}</text>",
                ascent,
                font_size,
                escape_xml(s)
            );
            Layout {
                svg,
                width,
                height,
                ascent,
            }
        }
        Expr::Greek(c) => {
            let char_w = char_width(font_size);
            let width = char_w * 1.2; // Greek chars slightly wider
            let height = font_size * 1.2;
            let ascent = font_size * 0.8;
            let svg = format!(
                "<text x=\"0\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\">{}</text>",
                ascent,
                font_size,
                escape_xml(&c.to_string())
            );
            Layout {
                svg,
                width,
                height,
                ascent,
            }
        }
        Expr::Matrix { env, rows } => {
            let cell_pad_x = font_size * 0.45;
            let row_gap = font_size * 0.25;
            let layouts: Vec<Vec<Layout>> = rows
                .iter()
                .map(|row| row.iter().map(|cell| layout_expr(cell, font_size * 0.9)).collect())
                .collect();
            let col_count = layouts.iter().map(|row| row.len()).max().unwrap_or(0);
            let mut col_widths = vec![font_size * 0.6; col_count];
            let mut row_heights = vec![font_size; layouts.len()];
            let mut row_ascents = vec![font_size * 0.75; layouts.len()];
            for (r, row) in layouts.iter().enumerate() {
                for (c, cell) in row.iter().enumerate() {
                    col_widths[c] = col_widths[c].max(cell.width);
                    row_heights[r] = row_heights[r].max(cell.height);
                    row_ascents[r] = row_ascents[r].max(cell.ascent);
                }
            }
            let body_w = col_widths.iter().sum::<f64>() + cell_pad_x * 2.0 * col_count as f64;
            let body_h = row_heights.iter().sum::<f64>()
                + row_gap * layouts.len().saturating_sub(1) as f64;
            let fence_w = if env == "matrix" || env == "smallmatrix" || env == "aligned" || env == "align" {
                0.0
            } else {
                font_size * 0.45
            };
            let total_w = body_w + fence_w * 2.0;
            let total_h = body_h.max(font_size);
            let ascent = total_h * 0.58;
            let mut svg = format!("<g data-math-env=\"{}\">", escape_xml(env));

            let mut y = 0.0;
            for (r, row) in layouts.iter().enumerate() {
                let mut x = fence_w;
                for (c, cell) in row.iter().enumerate() {
                    let cell_x = x + cell_pad_x + (col_widths[c] - cell.width) / 2.0;
                    let cell_y = y + row_ascents[r] - cell.ascent;
                    svg.push_str(&format!(
                        "<g transform=\"translate({},{})\">{}</g>",
                        cell_x, cell_y, cell.svg
                    ));
                    x += col_widths[c] + cell_pad_x * 2.0;
                }
                y += row_heights[r] + row_gap;
            }

            match env.as_str() {
                "pmatrix" => {
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">(</text><text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">)</text>",
                        fence_w / 2.0, ascent, total_h * 1.15, total_w - fence_w / 2.0, ascent, total_h * 1.15
                    ));
                }
                "bmatrix" => {
                    svg.push_str(&format!(
                        "<path d=\"M {},0 L 0,0 L 0,{} L {},{}\" fill=\"none\" stroke=\"#333\" stroke-width=\"1.4\"/><path d=\"M {},0 L {},0 L {},{} L {},{}\" fill=\"none\" stroke=\"#333\" stroke-width=\"1.4\"/>",
                        fence_w, total_h, fence_w, total_h, total_w - fence_w, total_w, total_w, total_h, total_w - fence_w, total_h
                    ));
                }
                "Bmatrix" => {
                    svg.push_str(&format!(
                        "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">{{</text><text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">}}</text>",
                        fence_w / 2.0, ascent, total_h * 1.15, total_w - fence_w / 2.0, ascent, total_h * 1.15
                    ));
                }
                "vmatrix" | "Vmatrix" => {
                    let sw = if env == "Vmatrix" { 2.2 } else { 1.4 };
                    svg.push_str(&format!(
                        "<line x1=\"{}\" y1=\"0\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"{}\"/><line x1=\"{}\" y1=\"0\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"{}\"/>",
                        fence_w / 2.0, fence_w / 2.0, total_h, sw, total_w - fence_w / 2.0, total_w - fence_w / 2.0, total_h, sw
                    ));
                }
                _ => {}
            }
            svg.push_str("</g>");
            Layout {
                svg,
                width: total_w,
                height: total_h,
                ascent,
            }
        }
        Expr::Group(exprs) => layout_group(exprs, font_size),
        Expr::Sub(base, sub) => {
            let base_l = layout_expr(base, font_size);
            let sub_l = layout_expr(sub, font_size * SUB_SCALE);
            // sub goes below-right of base, shifted down by 0.3em
            let sub_shift = font_size * 0.3;
            let sub_x = base_l.width;
            let sub_y = base_l.ascent + sub_shift;
            let total_w = base_l.width + sub_l.width;
            let total_h = (sub_y + sub_l.height).max(base_l.height);
            let ascent = base_l.ascent;
            let svg = format!(
                "{}<g transform=\"translate({},{})\">{}</g>",
                base_l.svg,
                sub_x,
                sub_y - sub_l.ascent,
                sub_l.svg
            );
            Layout {
                svg,
                width: total_w,
                height: total_h,
                ascent,
            }
        }
        Expr::Sup(base, sup) => {
            let base_l = layout_expr(base, font_size);
            let sup_l = layout_expr(sup, font_size * SUP_SCALE);
            // sup goes above-right of base, shifted up by 0.5em
            let sup_shift = font_size * 0.5;
            let sup_x = base_l.width;
            let sup_y = base_l.ascent - sup_shift - sup_l.ascent;
            let _actual_sup_y = sup_y.min(0.0);
            let dy = if sup_y < 0.0 { -sup_y } else { 0.0 };
            let total_w = base_l.width + sup_l.width;
            let total_h = (base_l.height + dy).max(sup_l.height + dy);
            let ascent = base_l.ascent + dy;
            let svg =
                format!(
                "<g transform=\"translate(0,{})\">{}<g transform=\"translate({},{})\">{}</g></g>",
                dy, base_l.svg, sup_x, sup_y + dy, sup_l.svg
            );
            Layout {
                svg,
                width: total_w,
                height: total_h,
                ascent,
            }
        }
        Expr::SubSup(base, sub, sup) => {
            let base_l = layout_expr(base, font_size);
            let sub_l = layout_expr(sub, font_size * SUB_SCALE);
            let sup_l = layout_expr(sup, font_size * SUP_SCALE);
            let script_w = sub_l.width.max(sup_l.width);
            let sup_shift = font_size * 0.5;
            let sub_shift = font_size * 0.3;
            let sup_y = base_l.ascent - sup_shift - sup_l.ascent;
            let dy = if sup_y < 0.0 { -sup_y } else { 0.0 };
            let sub_y = base_l.ascent + sub_shift + dy;
            let total_w = base_l.width + script_w;
            let total_h = (sub_y + sub_l.height - sub_l.ascent).max(base_l.height + dy);
            let ascent = base_l.ascent + dy;
            let svg = format!(
                "<g transform=\"translate(0,{})\">{}<g transform=\"translate({},{})\">{}</g><g transform=\"translate({},{})\">{}</g></g>",
                dy,
                base_l.svg,
                base_l.width,
                sup_y + dy,
                sup_l.svg,
                base_l.width,
                sub_y - sub_l.ascent,
                sub_l.svg
            );
            Layout {
                svg,
                width: total_w,
                height: total_h,
                ascent,
            }
        }
        Expr::Frac(num, den) => {
            let num_l = layout_expr(num, font_size * 0.85);
            let den_l = layout_expr(den, font_size * 0.85);
            let inner_w = num_l.width.max(den_l.width) + FRAC_PAD * 2.0;
            // Line at the middle
            let line_y = num_l.height + FRAC_PAD;
            let total_h = num_l.height + FRAC_PAD + 2.0 + FRAC_PAD + den_l.height;
            let ascent = line_y + 1.0; // baseline at the fraction line
            let num_x = (inner_w - num_l.width) / 2.0;
            let den_x = (inner_w - den_l.width) / 2.0;
            let den_y = line_y + 2.0 + FRAC_PAD;
            let svg = format!(
                "<g transform=\"translate({},0)\">{}</g>\
                 <line x1=\"0\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"1.2\"/>\
                 <g transform=\"translate({},{})\">{}</g>",
                num_x,
                num_l.svg,
                line_y + 1.0,
                inner_w,
                line_y + 1.0,
                den_x,
                den_y,
                den_l.svg
            );
            Layout {
                svg,
                width: inner_w,
                height: total_h,
                ascent,
            }
        }
        Expr::Sqrt(inner) => {
            let inner_l = layout_expr(inner, font_size);
            let pad = 4.0;
            let inner_x = SQRT_LEAN + pad;
            let inner_y = pad;
            let total_w = inner_x + inner_l.width + pad;
            let total_h = inner_l.height + pad * 2.0;
            let ascent = inner_l.ascent + pad;
            // Radical path: short foot then up to top then horizontal overline
            let foot_x = 0.0;
            let foot_y = total_h * 0.75;
            let corner_x = SQRT_LEAN * 0.5;
            let corner_y = total_h;
            let top_left_x = SQRT_LEAN;
            let top_left_y = inner_y;
            let overline_end_x = total_w - 1.0;
            let svg = format!(
                "<path d=\"M {},{} L {},{} L {},{} L {},{}\" fill=\"none\" stroke=\"#333\" stroke-width=\"1.5\"/>\
                 <g transform=\"translate({},{})\">{}</g>",
                foot_x, foot_y,
                corner_x, corner_y,
                top_left_x, top_left_y,
                overline_end_x, top_left_y,
                inner_x, inner_y,
                inner_l.svg
            );
            Layout {
                svg,
                width: total_w,
                height: total_h,
                ascent,
            }
        }
        Expr::Accent { kind, inner } => {
            let inner_l = layout_expr(inner, font_size);
            let top_pad = if kind == "underline" { 2.0 } else { font_size * 0.35 };
            let bottom_pad = if kind == "underline" {
                font_size * 0.25
            } else {
                0.0
            };
            let width = inner_l.width.max(font_size * 0.7);
            let height = inner_l.height + top_pad + bottom_pad;
            let ascent = inner_l.ascent + top_pad;
            let inner_x = (width - inner_l.width) / 2.0;
            let mut svg = String::new();
            match kind.as_str() {
                "hat" => svg.push_str(&format!(
                    "<path d=\"M {},{} L {},{} L {},{}\" fill=\"none\" stroke=\"#333\" stroke-width=\"1.2\"/>",
                    width * 0.2,
                    top_pad,
                    width * 0.5,
                    1.0,
                    width * 0.8,
                    top_pad
                )),
                "vec" => svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"1.2\" marker-end=\"url(#math-arrow)\"/>",
                    width * 0.15,
                    top_pad * 0.55,
                    width * 0.85,
                    top_pad * 0.55
                )),
                "dot" | "ddot" => {
                    svg.push_str(&format!(
                        "<circle cx=\"{}\" cy=\"{}\" r=\"1.5\" fill=\"#333\"/>",
                        width * 0.45,
                        top_pad * 0.45
                    ));
                    if kind == "ddot" {
                        svg.push_str(&format!(
                            "<circle cx=\"{}\" cy=\"{}\" r=\"1.5\" fill=\"#333\"/>",
                            width * 0.6,
                            top_pad * 0.45
                        ));
                    }
                }
                "underline" => svg.push_str(&format!(
                    "<line x1=\"0\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"1.2\"/>",
                    inner_l.height + top_pad + 2.0,
                    width,
                    inner_l.height + top_pad + 2.0
                )),
                _ => svg.push_str(&format!(
                    "<line x1=\"0\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"1.2\"/>",
                    top_pad * 0.45,
                    width,
                    top_pad * 0.45
                )),
            }
            svg.push_str(&format!(
                "<g transform=\"translate({},{})\">{}</g>",
                inner_x, top_pad, inner_l.svg
            ));
            Layout {
                svg,
                width,
                height,
                ascent,
            }
        }
        Expr::BigOp { op, sub, sup } => {
            let op_font = font_size * 1.6;
            let op_char = op.to_string();
            let op_char_w = char_width(op_font) * 1.4;
            let op_h = op_font * 1.2;
            let op_ascent = op_font * 0.8;

            let sub_l = layout_expr(sub, font_size * SUB_SCALE);
            let sup_l = layout_expr(sup, font_size * SUP_SCALE);

            let inner_w = op_char_w.max(sub_l.width).max(sup_l.width);

            // sup above operator, sub below
            let sup_y = 0.0;
            let op_y = sup_l.height + 2.0;
            let sub_y = op_y + op_h + 2.0;
            let total_h = sub_y + sub_l.height;
            let ascent = op_y + op_ascent;

            let op_x = (inner_w - op_char_w) / 2.0;
            let sup_x = (inner_w - sup_l.width) / 2.0;
            let sub_x = (inner_w - sub_l.width) / 2.0;

            let svg = format!(
                "<g transform=\"translate({},{})\">{}</g>\
                 <text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\" text-anchor=\"middle\">{}</text>\
                 <g transform=\"translate({},{})\">{}</g>",
                sup_x, sup_y, sup_l.svg,
                op_x + op_char_w / 2.0, op_y + op_ascent, op_font, escape_xml(&op_char),
                sub_x, sub_y, sub_l.svg
            );
            Layout {
                svg,
                width: inner_w,
                height: total_h,
                ascent,
            }
        }
    }
}

fn layout_group(exprs: &[Expr], font_size: f64) -> Layout {
    if exprs.is_empty() {
        return Layout {
            svg: String::new(),
            width: 0.0,
            height: font_size * 1.2,
            ascent: font_size * 0.8,
        };
    }
    let layouts: Vec<Layout> = exprs.iter().map(|e| layout_expr(e, font_size)).collect();
    // Align all nodes by baseline
    let max_ascent = layouts.iter().map(|l| l.ascent).fold(0.0f64, f64::max);
    let max_below = layouts
        .iter()
        .map(|l| l.height - l.ascent)
        .fold(0.0f64, f64::max);
    let total_h = max_ascent + max_below;
    let mut x = 0.0f64;
    let mut svg = String::new();
    for l in &layouts {
        if l.width == 0.0 && l.svg.is_empty() {
            continue;
        }
        let dy = max_ascent - l.ascent;
        svg.push_str(&format!(
            "<g transform=\"translate({},{})\">{}</g>",
            x, dy, l.svg
        ));
        x += l.width;
    }
    // Add small gap between items
    let total_w = x;
    Layout {
        svg,
        width: total_w,
        height: total_h,
        ascent: max_ascent,
    }
}

fn render_math(source: &str) -> Result<String, Diagnostic> {
    let (body, title) = strip_block(source, "@startmath", "@endmath");
    let expr_text = body.trim();
    if expr_text.is_empty() {
        return Err(Diagnostic::error("[E_MATH_EMPTY] @startmath body is empty"));
    }

    // Parse and layout
    let exprs = parse_math_expr(expr_text);
    let layout = layout_group(&exprs, BASE_FONT);

    let title_h = if title.is_some() { 28i32 } else { 0 };
    let margin = 30i32;
    let w = (layout.width as i32 + margin * 2).max(200);
    let h = layout.height as i32 + margin * 2 + title_h;

    let mut out = String::new();
    out.push_str(&svg_header(w, h));
    out.push_str(svg_white_bg());
    out.push_str(
        "<defs><marker id=\"math-arrow\" markerWidth=\"7\" markerHeight=\"5\" refX=\"6\" refY=\"2.5\" orient=\"auto\"><path d=\"M0,0 L0,5 L7,2.5 z\" fill=\"#333\"/></marker></defs>",
    );

    if let Some(t) = &title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"22\" font-family=\"serif\" font-size=\"14\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#333\">{}</text>",
            w / 2,
            escape_xml(t)
        ));
    }

    // Expression background box
    let ex = (w as f64 - layout.width) / 2.0;
    let ey = title_h + margin;
    out.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"#f9f9f9\" stroke=\"#ddd\" stroke-width=\"1\"/>",
        (ex - 10.0) as i32, ey - 10, (layout.width + 20.0) as i32, (layout.height + 20.0) as i32
    ));

    out.push_str(&format!(
        "<g transform=\"translate({},{})\">{}</g>",
        ex as i32, ey, layout.svg
    ));

    out.push_str("</svg>");
    Ok(out)
}

fn render_latex(source: &str) -> Result<String, Diagnostic> {
    let (body, title) = strip_block(source, "@startlatex", "@endlatex");
    let mut normalized = String::from("@startmath");
    if let Some(t) = title {
        normalized.push(' ');
        normalized.push('"');
        normalized.push_str(&t.replace('"', "\\\""));
        normalized.push('"');
    }
    normalized.push('\n');
    normalized.push_str(body);
    normalized.push('\n');
    normalized.push_str("@endmath\n");
    render_math(&normalized)
}

// ─── Family 2: @startditaa ─────────────────────────────────────────────────────
//
// Real ASCII art rasterizer: 5-pass approach.

/// Color hints inside ditaa boxes
fn hint_to_fill(hint: &str) -> Option<&'static str> {
    match hint {
        "cBLU" | "cBlu" => Some("#aad4f5"),
        "cRED" | "cRed" => Some("#f5aaaa"),
        "cGRE" | "cGre" => Some("#aaf5aa"),
        "cYEL" | "cYel" => Some("#f5f5aa"),
        "cBLK" | "cBlk" => Some("#222222"),
        "cWHI" | "cWhi" => Some("#ffffff"),
        "cPNK" | "cPnk" => Some("#f5aad4"),
        "cORA" | "cOra" => Some("#f5d4aa"),
        "cGRA" | "cGra" => Some("#cccccc"),
        _ => None,
    }
}

/// Shape types detected in the grid.
#[derive(Debug, Clone)]
enum ShapeKind {
    Rect,
    RoundedRect,
    Document,
    Cylinder,
    Diamond,
}

/// A detected shape.
#[derive(Debug, Clone)]
struct Shape {
    kind: ShapeKind,
    r1: usize,
    c1: usize,
    r2: usize,
    c2: usize,
    fill: String,
    dashed: bool,
    text_lines: Vec<(usize, String)>, // (row_idx, text)
}

/// A connector arrow or line.
#[derive(Debug, Clone)]
struct Connector {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    has_head_end: bool,
    has_head_start: bool,
    dashed: bool,
}

fn render_ditaa(source: &str) -> Result<String, Diagnostic> {
    let (body, title) = strip_block(source, "@startditaa", "@endditaa");
    let options = parse_ditaa_options(source.lines().next().unwrap_or(""));

    if body.trim().is_empty() {
        return Err(Diagnostic::error(
            "[E_DITAA_EMPTY] @startditaa body is empty",
        ));
    }

    // Build padded grid
    let lines: Vec<Vec<char>> = body.lines().map(|l| l.chars().collect()).collect();
    if lines.is_empty() {
        return Err(Diagnostic::error(
            "[E_DITAA_EMPTY] @startditaa has no grid content",
        ));
    }

    let cell_w = 10i32 * options.scale;
    let cell_h = 16i32 * options.scale;
    let grid_rows = lines.len();
    let grid_cols = lines.iter().map(|r| r.len()).max().unwrap_or(0);
    let title_h = if title.is_some() { 28i32 } else { 0 };
    let margin = 16i32;
    let svg_w = grid_cols as i32 * cell_w + margin * 2;
    let svg_h = grid_rows as i32 * cell_h + margin * 2 + title_h;

    let get = |r: usize, c: usize| -> char {
        lines
            .get(r)
            .and_then(|row| row.get(c))
            .copied()
            .unwrap_or(' ')
    };

    // ── Pass 1: detect shapes ──────────────────────────────────────────────────

    let mut shapes: Vec<Shape> = Vec::new();
    // Track which cells are claimed by a shape
    let mut claimed = vec![vec![false; grid_cols + 1]; grid_rows + 1];

    for r1 in 0..grid_rows {
        for c1 in 0..grid_cols {
            let tl = get(r1, c1);
            if tl != '+' && tl != '(' {
                continue;
            }
            // Check if this corner has already been claimed as part of a larger shape
            if claimed[r1][c1] {
                continue;
            }
            let rounded_start = tl == '(';

            // Find right corner on same row
            let row_len = lines[r1].len();
            let mut c2_candidates: Vec<usize> = Vec::new();
            let mut cc = c1 + 1;
            while cc < row_len {
                let ch = get(r1, cc);
                if ch == '+' || ch == ')' {
                    // Verify top edge is continuous
                    let top_ok = (c1 + 1..cc).all(|c| matches!(get(r1, c), '-' | '=' | ' '));
                    if top_ok {
                        c2_candidates.push(cc);
                    }
                    break; // only try nearest right corner
                } else if !matches!(ch, '-' | '=' | ' ') {
                    break;
                }
                cc += 1;
            }

            for c2 in c2_candidates {
                let tr = get(r1, c2);
                let rounded_end = tr == ')';

                // Find bottom corners
                let mut r2_candidates: Vec<usize> = Vec::new();
                let mut rr = r1 + 1;
                while rr < grid_rows {
                    let bl = get(rr, c1);
                    let br = get(rr, c2);
                    if (bl == '+' || bl == '(') && (br == '+' || br == ')') {
                        // Verify all edges
                        let bot_ok = (c1 + 1..c2).all(|c| matches!(get(rr, c), '-' | '=' | ' '));
                        let left_ok =
                            (r1 + 1..rr).all(|r| matches!(get(r, c1), '|' | ':' | '+' | ' '));
                        let right_ok =
                            (r1 + 1..rr).all(|r| matches!(get(r, c2), '|' | ':' | '+' | ' '));
                        if bot_ok && left_ok && right_ok {
                            r2_candidates.push(rr);
                        }
                        break;
                    } else if !matches!(bl, '|' | ':' | ' ' | '+') {
                        break;
                    }
                    rr += 1;
                }

                for r2 in r2_candidates {
                    // Determine fill by scanning for color hints inside box
                    let mut fill = "#f0f4ff".to_string();
                    let mut dashed = false;
                    let mut text_lines: Vec<(usize, String)> = Vec::new();

                    for row_idx in (r1 + 1)..r2 {
                        let mut inner = String::new();
                        for ci in (c1 + 1)..c2 {
                            let ch = get(row_idx, ci);
                            if !matches!(ch, '|' | ':') {
                                inner.push(ch);
                            }
                        }
                        let trimmed_inner = inner.trim().to_string();

                        // Color hint detection
                        for word in trimmed_inner.split_whitespace() {
                            if let Some(f) = hint_to_fill(word) {
                                fill = f.to_string();
                            }
                        }

                        // Check for dashed edges
                        if (c1 + 1..c2).any(|c| get(r1, c) == '=')
                            || (r1 + 1..r2).any(|r| get(r, c1) == ':')
                        {
                            dashed = true;
                        }

                        // Remove color hints from display text
                        let display: String = trimmed_inner
                            .split_whitespace()
                            .filter(|w| hint_to_fill(w).is_none())
                            .collect::<Vec<_>>()
                            .join(" ");

                        if !display.is_empty() {
                            text_lines.push((row_idx, display));
                        }
                    }

                    // Determine shape kind
                    let kind = if rounded_start || rounded_end {
                        ShapeKind::RoundedRect
                    } else {
                        // Check for cylinder: top row has '(' at c1+1 and ')' at c2-1
                        let maybe_cyl = c2 > c1 + 2
                            && (r1 + 1..r2).all(|r| get(r, c1) == '|' && get(r, c2) == '|')
                            && get(r1, c1 + 1) == '('
                            && get(r1, c2 - 1) == ')';
                        // Check for diamond: /...\ top and \.../ bottom
                        let maybe_diamond = c2 > c1 + 2
                            && get(r1, c1 + 1) == '/'
                            && get(r1, c2 - 1) == '\\'
                            && get(r2, c1 + 1) == '\\'
                            && get(r2, c2 - 1) == '/';
                        // Check for document: bottom row has '~' wave
                        let maybe_doc = (c1 + 1..c2).any(|c| get(r2, c) == '~');

                        if maybe_diamond {
                            ShapeKind::Diamond
                        } else if maybe_cyl {
                            ShapeKind::Cylinder
                        } else if maybe_doc {
                            ShapeKind::Document
                        } else {
                            ShapeKind::Rect
                        }
                    };

                    // Mark cells as claimed
                    for row in claimed.iter_mut().take(r2 + 1).skip(r1) {
                        for c in c1..=c2 {
                            if c < row.len() {
                                row[c] = true;
                            }
                        }
                    }

                    shapes.push(Shape {
                        kind,
                        r1,
                        c1,
                        r2,
                        c2,
                        fill,
                        dashed,
                        text_lines,
                    });
                }
            }
        }
    }

    // ── Pass 2: connector detection ────────────────────────────────────────────

    let mut connectors: Vec<Connector> = Vec::new();

    // Horizontal connectors (sequences of '-' or '=' not part of shape border)
    for (row_idx, row) in lines.iter().enumerate() {
        let mut c = 0usize;
        while c < row.len() {
            let ch = row[c];
            if ch == '<' && c + 1 < row.len() && row[c + 1] == '-' {
                // Left-pointing arrow start
                let c_start = c;
                c += 1;
                let dashed = row[c] == '=';
                while c < row.len() && matches!(row[c], '-' | '=' | '+') {
                    c += 1;
                }
                let c_end = c;
                // Check not on shape border
                let is_border = shapes.iter().any(|s| {
                    (row_idx == s.r1 || row_idx == s.r2) && c_start >= s.c1 && c_end <= s.c2
                });
                if !is_border {
                    let x1 = margin + c_start as i32 * cell_w;
                    let x2 = margin + c_end as i32 * cell_w;
                    let y = margin + title_h + row_idx as i32 * cell_h + cell_h / 2;
                    connectors.push(Connector {
                        x1,
                        y1: y,
                        x2,
                        y2: y,
                        has_head_end: false,
                        has_head_start: true,
                        dashed,
                    });
                }
            } else if matches!(ch, '-' | '=') {
                let c_start = c;
                let dashed = ch == '=';
                while c < row.len() && matches!(row[c], '-' | '=' | '+') {
                    c += 1;
                }
                let has_head = c < row.len() && row[c] == '>';
                if has_head {
                    c += 1;
                }
                let c_end = c;

                let is_border = shapes.iter().any(|s| {
                    (row_idx == s.r1 || row_idx == s.r2) && c_start >= s.c1 && c_end <= s.c2
                });
                if !is_border && c_end > c_start {
                    let x1 = margin + c_start as i32 * cell_w;
                    let x2 = margin + (c_end - if has_head { 1 } else { 0 }) as i32 * cell_w;
                    let y = margin + title_h + row_idx as i32 * cell_h + cell_h / 2;
                    connectors.push(Connector {
                        x1,
                        y1: y,
                        x2,
                        y2: y,
                        has_head_end: has_head,
                        has_head_start: false,
                        dashed,
                    });
                }
            } else {
                c += 1;
            }
        }
    }

    // Vertical connectors (sequences of '|' or ':')
    for col_idx in 0..grid_cols {
        let mut r = 0usize;
        while r < grid_rows {
            let ch = get(r, col_idx);
            if ch == '^' && r + 1 < grid_rows && matches!(get(r + 1, col_idx), '|' | ':') {
                // Upward arrow
                let r_start = r;
                r += 1;
                let dashed = get(r, col_idx) == ':';
                while r < grid_rows && matches!(get(r, col_idx), '|' | ':' | '+') {
                    r += 1;
                }
                let r_end = r;
                let is_border = shapes.iter().any(|s| {
                    (col_idx == s.c1 || col_idx == s.c2) && r_start >= s.r1 && r_end <= s.r2
                });
                if !is_border {
                    let x = margin + col_idx as i32 * cell_w + cell_w / 2;
                    let y1 = margin + title_h + r_end as i32 * cell_h;
                    let y2 = margin + title_h + r_start as i32 * cell_h;
                    connectors.push(Connector {
                        x1: x,
                        y1,
                        x2: x,
                        y2,
                        has_head_end: true,
                        has_head_start: false,
                        dashed,
                    });
                }
            } else if matches!(ch, '|' | ':') {
                let r_start = r;
                let dashed = ch == ':';
                while r < grid_rows && matches!(get(r, col_idx), '|' | ':' | '+') {
                    r += 1;
                }
                // Check if next char is 'v'
                let has_head = r < grid_rows && get(r, col_idx) == 'v';
                if has_head {
                    r += 1;
                }
                let r_end = r;
                let is_border = shapes.iter().any(|s| {
                    (col_idx == s.c1 || col_idx == s.c2) && r_start >= s.r1 && r_end <= s.r2
                });
                if !is_border && r_end > r_start {
                    let x = margin + col_idx as i32 * cell_w + cell_w / 2;
                    let y1 = margin + title_h + r_start as i32 * cell_h;
                    let y2 = margin + title_h + r_end as i32 * cell_h;
                    connectors.push(Connector {
                        x1: x,
                        y1,
                        x2: x,
                        y2,
                        has_head_end: has_head,
                        has_head_start: false,
                        dashed,
                    });
                }
            } else {
                r += 1;
            }
        }
    }

    // Diagonal connectors, a common ditaa idiom for loose ASCII wiring.
    for (row_idx, row) in lines.iter().enumerate() {
        for (c, &ch) in row.iter().enumerate() {
            if !matches!(ch, '/' | '\\') {
                continue;
            }
            let in_shape = shapes
                .iter()
                .any(|s| row_idx >= s.r1 && row_idx <= s.r2 && c >= s.c1 && c <= s.c2);
            if in_shape {
                continue;
            }
            let x = margin + c as i32 * cell_w + cell_w / 2;
            let y = margin + title_h + row_idx as i32 * cell_h + cell_h / 2;
            let (x1, y1, x2, y2) = if ch == '/' {
                (
                    x - cell_w / 2,
                    y + cell_h / 2,
                    x + cell_w / 2,
                    y - cell_h / 2,
                )
            } else {
                (
                    x - cell_w / 2,
                    y - cell_h / 2,
                    x + cell_w / 2,
                    y + cell_h / 2,
                )
            };
            let has_head_start = match ch {
                '/' => {
                    let r = (row_idx + 1).min(grid_rows.saturating_sub(1));
                    (c.saturating_sub(3)..=c + 1).any(|cc| get(r, cc) == '<')
                        || (c.saturating_sub(3)..=c).any(|cc| get(row_idx, cc) == '<')
                        || (0..grid_cols).any(|cc| get(r, cc) == '<')
                }
                '\\' => {
                    let r = row_idx.saturating_sub(1);
                    (c.saturating_sub(3)..=c + 1).any(|cc| get(r, cc) == '<')
                        || (c.saturating_sub(3)..=c).any(|cc| get(row_idx, cc) == '<')
                        || (0..grid_cols).any(|cc| get(r, cc) == '<')
                }
                _ => false,
            };
            let has_head_end = match ch {
                '/' => {
                    (c + 1 < grid_cols && row_idx > 0 && get(row_idx - 1, c + 1) == '>')
                        || (c + 1 < grid_cols && row_idx > 0 && get(row_idx - 1, c + 1) == '^')
                }
                '\\' => {
                    (c + 1 < grid_cols && row_idx + 1 < grid_rows && get(row_idx + 1, c + 1) == '>')
                        || (c + 1 < grid_cols
                            && row_idx + 1 < grid_rows
                            && get(row_idx + 1, c + 1) == 'v')
                }
                _ => false,
            };
            connectors.push(Connector {
                x1,
                y1,
                x2,
                y2,
                has_head_end,
                has_head_start,
                dashed: false,
            });
        }
    }

    // ── Pass 3: SVG emission ──────────────────────────────────────────────────

    let mut out = String::new();
    out.push_str(&svg_header(svg_w, svg_h));
    if !options.transparent {
        if let Some(background) = &options.background {
            out.push_str(&format!(
                "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
                escape_xml(background)
            ));
        } else {
            out.push_str(svg_white_bg());
        }
    }

    // Arrow markers
    out.push_str(
        "<defs>\
         <marker id=\"da\" markerWidth=\"8\" markerHeight=\"6\" refX=\"6\" refY=\"3\" orient=\"auto\">\
         <path d=\"M0,0 L0,6 L8,3 z\" fill=\"#444\"/></marker>\
         <marker id=\"dah\" markerWidth=\"8\" markerHeight=\"6\" refX=\"2\" refY=\"3\" orient=\"auto\">\
         <path d=\"M8,0 L8,6 L0,3 z\" fill=\"#444\"/></marker>\
         </defs>",
    );
    if options.shadow {
        out.push_str("<defs><filter id=\"ditaa-shadow\" x=\"-20%\" y=\"-20%\" width=\"140%\" height=\"140%\"><feDropShadow dx=\"2\" dy=\"2\" stdDeviation=\"1.5\" flood-color=\"#00000033\"/></filter></defs>");
    }

    if let Some(t) = &title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"14\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            svg_w / 2, margin + 16, escape_xml(t)
        ));
    }

    // Draw shapes
    for shape in &shapes {
        let rx = margin + shape.c1 as i32 * cell_w;
        let ry = margin + title_h + shape.r1 as i32 * cell_h;
        let rw = (shape.c2 - shape.c1) as i32 * cell_w;
        let rh = (shape.r2 - shape.r1) as i32 * cell_h;
        let stroke = if shape.dashed {
            "stroke-dasharray=\"6,3\""
        } else {
            ""
        };
        let filter = if options.shadow {
            "filter=\"url(#ditaa-shadow)\""
        } else {
            ""
        };

        match shape.kind {
            ShapeKind::Rect => {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {} {}/>",
                    rx, ry, rw, rh, shape.fill, stroke, filter
                ));
            }
            ShapeKind::RoundedRect => {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"12\" ry=\"12\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {} {}/>",
                    rx, ry, rw, rh, shape.fill, stroke, filter
                ));
            }
            ShapeKind::Document => {
                // Draw as rect with curved bottom
                let cx = rx + rw / 2;
                let bot_y = ry + rh;
                out.push_str(&format!(
                    "<path d=\"M {},{} L {},{} L {},{} Q {},{} {},{} Q {},{} {},{}  Z\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {}/>",
                    rx, ry,
                    rx + rw, ry,
                    rx + rw, bot_y - 8,
                    cx + rw / 4, bot_y + 6, cx, bot_y - 4,
                    cx - rw / 4, bot_y - 14, rx, bot_y - 8,
                    shape.fill, stroke
                ));
            }
            ShapeKind::Cylinder => {
                let cx = rx + rw / 2;
                let ell_ry = 6i32;
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {}/>\
                     <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\"/>\
                     <ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"none\" stroke=\"#3344aa\" stroke-width=\"1\"/>",
                    rx, ry + ell_ry, rw, rh - ell_ry, shape.fill, stroke,
                    cx, ry + ell_ry, rw / 2, ell_ry, shape.fill,
                    cx, ry + rh, rw / 2, ell_ry
                ));
            }
            ShapeKind::Diamond => {
                let cx = rx + rw / 2;
                let cy = ry + rh / 2;
                out.push_str(&format!(
                    "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"#3344aa\" stroke-width=\"1.5\" {}/>",
                    cx, ry,
                    rx + rw, cy,
                    cx, ry + rh,
                    rx, cy,
                    shape.fill, stroke
                ));
            }
        }

        // Render text inside shape
        for (row_idx, text) in &shape.text_lines {
            let tx = rx + rw / 2;
            let ty = margin + title_h + *row_idx as i32 * cell_h + cell_h - 3;
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" text-anchor=\"middle\" fill=\"#111\">{}</text>",
                tx, ty, escape_xml(text)
            ));
        }
    }

    // Draw connectors
    for conn in &connectors {
        let dash = if conn.dashed {
            " stroke-dasharray=\"6,3\""
        } else {
            ""
        };
        let mut marker_end = "";
        let mut marker_start = "";
        if conn.has_head_end {
            marker_end = " marker-end=\"url(#da)\"";
        }
        if conn.has_head_start {
            marker_start = " marker-start=\"url(#dah)\"";
        }
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#444\" stroke-width=\"1.5\"{}{}{}/>",
            conn.x1, conn.y1, conn.x2, conn.y2, dash, marker_end, marker_start
        ));
    }

    // ── Pass 4: render unclaimed text ─────────────────────────────────────────

    for (row_idx, row) in lines.iter().enumerate() {
        for (c, &ch) in row.iter().enumerate() {
            // Skip if inside a shape region
            let in_shape = shapes
                .iter()
                .any(|s| row_idx >= s.r1 && row_idx <= s.r2 && c >= s.c1 && c <= s.c2);
            if in_shape {
                continue;
            }
            // Skip structural chars and arrows
            if matches!(
                ch,
                '+' | '-'
                    | '|'
                    | '='
                    | ':'
                    | '>'
                    | '<'
                    | 'v'
                    | '^'
                    | ' '
                    | '~'
                    | '('
                    | ')'
                    | '/'
                    | '\\'
            ) {
                continue;
            }
            let tx = margin + c as i32 * cell_w;
            let ty = margin + title_h + row_idx as i32 * cell_h + cell_h - 3;
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#222\">{}</text>",
                tx, ty, escape_xml(&ch.to_string())
            ));
        }
    }

    out.push_str("</svg>");
    Ok(out)
}

#[derive(Debug, Clone)]
struct DitaaOptions {
    scale: i32,
    transparent: bool,
    shadow: bool,
    background: Option<String>,
}

fn parse_ditaa_options(first_line: &str) -> DitaaOptions {
    let mut options = DitaaOptions {
        scale: 1,
        transparent: false,
        shadow: false,
        background: None,
    };
    let lower = first_line.to_ascii_lowercase();
    if let Some(pos) = lower.find("scale=") {
        let n: String = lower[pos + 6..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(v) = n.parse::<i32>() {
            options.scale = v.clamp(1, 4);
        }
    }
    if lower.contains("transparent=true") || lower.contains("transparent=yes") {
        options.transparent = true;
    }
    if lower.contains("shadow=true") || lower.contains("shadow=yes") {
        options.shadow = true;
    }
    if let Some(pos) = lower.find("background=") {
        let value: String = first_line[pos + "background=".len()..]
            .chars()
            .take_while(|c| !c.is_whitespace())
            .collect();
        if !value.is_empty() {
            options.background = Some(value);
        }
    }
    options
}
