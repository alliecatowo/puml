/// Specialized diagram renderers for @startregex, @startebnf, @startchart,
/// @startmath, @startsdl, and @startditaa diagram families.
///
/// These bypass the main AST parser pipeline and implement their own
/// mini-parsers and SVG renderers.
use crate::diagnostic::Diagnostic;

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
    match node {
        RailNode::Literal(text) => {
            let w = rail_box_width(text);
            let h = RAIL_BOX_H;
            let mid = h / 2;
            let svg = format!(
                "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" rx=\"14\" ry=\"14\" fill=\"#fff8e1\" stroke=\"#f9a825\" stroke-width=\"1.5\"/>
<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"#333\">{}</text>",
                w, h,
                w / 2, mid,
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
                "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"#e8f5e9\" stroke=\"#388e3c\" stroke-width=\"1.5\"/>
<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"#1b5e20\">{}</text>",
                w, h,
                w / 2, mid,
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
                "<circle cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"#e3f2fd\" stroke=\"#1976d2\" stroke-width=\"1.5\"/>
<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"#1565c0\">{}</text>",
                w / 2, mid,
                w / 2, mid,
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
                "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"#fce4ec\" stroke=\"#c62828\" stroke-width=\"1.5\"/>
<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"#b71c1c\">{}</text>",
                w, h,
                w / 2, mid,
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
            let children: Vec<RailLayout> = items.iter().map(layout_rail).collect();
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
            let children: Vec<RailLayout> = branches.iter().map(layout_rail).collect();
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
            let child = layout_rail(inner);
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
            let child = layout_rail(inner);
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
        RailNode::Optional(inner) => {
            let child = layout_rail(inner);
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
    let pattern = body.trim();
    if pattern.is_empty() {
        return Err(Diagnostic::error(
            "[E_REGEX_EMPTY] @startregex body is empty",
        ));
    }
    let node = parse_regex_to_rail(pattern);
    let title = format!("/{}/", pattern);
    Ok(render_railroad(&title, &node))
}

/// Parse a regex pattern string into a RailNode AST.
/// Supports: literals, `.`, `|`, `(...)`, `[...]`, `*`, `+`, `?`, `^`, `$`.
fn parse_regex_to_rail(pattern: &str) -> RailNode {
    let chars: Vec<char> = pattern.chars().collect();
    let (node, _) = parse_regex_alternation(&chars, 0);
    node
}

fn parse_regex_alternation(chars: &[char], start: usize) -> (RailNode, usize) {
    let mut branches = Vec::new();
    let (first, mut pos) = parse_regex_sequence(chars, start);
    branches.push(first);
    while pos < chars.len() && chars[pos] == '|' {
        pos += 1;
        let (branch, new_pos) = parse_regex_sequence(chars, pos);
        branches.push(branch);
        pos = new_pos;
    }
    if branches.len() == 1 {
        (branches.remove(0), pos)
    } else {
        (RailNode::Alternation(branches), pos)
    }
}

fn parse_regex_sequence(chars: &[char], start: usize) -> (RailNode, usize) {
    let mut items = Vec::new();
    let mut pos = start;
    while pos < chars.len() {
        match chars[pos] {
            ')' | '|' => break,
            '^' | '$' => {
                let sym = chars[pos].to_string();
                pos += 1;
                items.push(RailNode::Anchor(sym));
            }
            '(' => {
                pos += 1; // consume '('
                let (inner, new_pos) = parse_regex_alternation(chars, pos);
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
                    format!("\\{}", c)
                } else {
                    "\\".to_string()
                };
                let node = RailNode::Literal(escaped);
                let (node, new_pos) = apply_quantifier(node, chars, pos);
                pos = new_pos;
                items.push(node);
            }
            '.' => {
                pos += 1;
                let node = RailNode::CharClass(".".to_string());
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

fn apply_quantifier(node: RailNode, chars: &[char], pos: usize) -> (RailNode, usize) {
    if pos >= chars.len() {
        return (node, pos);
    }
    match chars[pos] {
        '*' => (RailNode::Repeat(Box::new(node)), pos + 1),
        '+' => (RailNode::OneOrMore(Box::new(node)), pos + 1),
        '?' => (RailNode::Optional(Box::new(node)), pos + 1),
        '{' => {
            // consume {n,m} quantifier as "one or more" for simplicity
            let mut p = pos + 1;
            while p < chars.len() && chars[p] != '}' {
                p += 1;
            }
            if p < chars.len() {
                p += 1;
            }
            (RailNode::Repeat(Box::new(node)), p)
        }
        _ => (node, pos),
    }
}

// ─── Family 2: @startebnf ─────────────────────────────────────────────────────

fn render_ebnf(source: &str) -> Result<String, Diagnostic> {
    let (body, doc_title) = strip_block(source, "@startebnf", "@endebnf");

    // Parse rules: "name = body ;"
    let rules = parse_ebnf_rules(body);
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
        .map(|(name, node)| (name.clone(), layout_rail(node)))
        .collect();

    let max_inner_w = layouts.iter().map(|(_, l)| l.width).max().unwrap_or(200);
    let svg_w = max_inner_w + margin * 2 + 40;
    let total_h: i32 = layouts
        .iter()
        .map(|(_, l)| l.height + label_h + gap_between)
        .sum::<i32>()
        + margin * 2;

    let mut out = String::new();
    out.push_str(&svg_header(svg_w, total_h));
    out.push_str(svg_white_bg());

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

    out.push_str("</svg>");
    Ok(out)
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
        if let Some(eq_pos) = line.find('=') {
            // Save previous rule if any
            if let Some(name) = current_name.take() {
                let trimmed = current_body.trim().trim_end_matches(';').trim().to_string();
                rules.push((name, trimmed));
                current_body.clear();
            }
            let name = line[..eq_pos].trim().to_string();
            let rest = line[eq_pos + 1..].trim();
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
    Pie,
    Column, // same as bar but explicit
}

#[derive(Debug, Clone)]
struct ChartData {
    label: String,
    value: f64,
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
        "pie" => ChartType::Pie,
        "column" => ChartType::Column,
        _ => ChartType::Bar, // "bar" is default
    };

    let (body, _) = strip_block(source, "@startchart", "@endchart");
    let mut title: Option<String> = None;
    let mut data: Vec<ChartData> = Vec::new();

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
        // Parse "label" : value or label : value
        if let Some(colon_pos) = line.rfind(':') {
            let label_part = line[..colon_pos].trim().trim_matches('"').to_string();
            let val_part = line[colon_pos + 1..].trim();
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

    match chart_type {
        ChartType::Bar | ChartType::Column => render_bar_chart(&data, &title, false),
        ChartType::Line => render_line_chart(&data, &title),
        ChartType::Pie => render_pie_chart(&data, &title),
    }
}

const CHART_COLORS: &[&str] = &[
    "#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f", "#edc948", "#b07aa1", "#ff9da7",
    "#9c755f", "#bab0ac",
];

fn bar_color(idx: usize) -> &'static str {
    CHART_COLORS[idx % CHART_COLORS.len()]
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

// ─── Family 4: @startmath ────────────────────────────────────────────────────

fn render_math(source: &str) -> Result<String, Diagnostic> {
    let (body, title) = strip_block(source, "@startmath", "@endmath");
    let expr = body.trim();
    if expr.is_empty() {
        return Err(Diagnostic::error("[E_MATH_EMPTY] @startmath body is empty"));
    }

    let tokens = tokenize_latex(expr);
    let rendered = render_math_tokens(&tokens);

    let title_h = if title.is_some() { 28 } else { 0 };
    let w = (rendered.width + 80).max(200);
    let h = rendered.height + 60 + title_h;

    let mut out = String::new();
    out.push_str(&svg_header(w, h));
    out.push_str(svg_white_bg());

    if let Some(t) = &title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"22\" font-family=\"monospace\" font-size=\"14\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#333\">{}</text>",
            w / 2, escape_xml(t)
        ));
    }

    // Render expression background
    let ex = (w - rendered.width) / 2;
    let ey = title_h + 20;
    out.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" fill=\"#f9f9f9\" stroke=\"#ddd\" stroke-width=\"1\"/>",
        ex - 10, ey - 10, rendered.width + 20, rendered.height + 20
    ));
    out.push_str(&format!(
        "<g transform=\"translate({},{})\">{}</g>",
        ex, ey, rendered.svg
    ));

    out.push_str("</svg>");
    Ok(out)
}

#[derive(Debug, Clone)]
enum MathToken {
    Char(char),
    Sub,                   // _{...}
    Sup,                   // ^{...}
    Frac,                  // \frac{a}{b}
    Sqrt,                  // \sqrt{x}
    Greek(String),         // \alpha etc
    Op(String),            // \sum, \int, \prod
    Group(Vec<MathToken>), // {...}
    Literal(String),       // \text{...}
}

fn tokenize_latex(s: &str) -> Vec<MathToken> {
    let chars: Vec<char> = s.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\\' => {
                i += 1;
                let mut name = String::new();
                while i < chars.len() && chars[i].is_alphabetic() {
                    name.push(chars[i]);
                    i += 1;
                }
                match name.as_str() {
                    "frac" => tokens.push(MathToken::Frac),
                    "sqrt" => tokens.push(MathToken::Sqrt),
                    "sum" => tokens.push(MathToken::Op("∑".to_string())),
                    "int" => tokens.push(MathToken::Op("∫".to_string())),
                    "prod" => tokens.push(MathToken::Op("∏".to_string())),
                    "alpha" => tokens.push(MathToken::Greek("α".to_string())),
                    "beta" => tokens.push(MathToken::Greek("β".to_string())),
                    "gamma" => tokens.push(MathToken::Greek("γ".to_string())),
                    "delta" => tokens.push(MathToken::Greek("δ".to_string())),
                    "epsilon" => tokens.push(MathToken::Greek("ε".to_string())),
                    "theta" => tokens.push(MathToken::Greek("θ".to_string())),
                    "lambda" => tokens.push(MathToken::Greek("λ".to_string())),
                    "mu" => tokens.push(MathToken::Greek("μ".to_string())),
                    "pi" => tokens.push(MathToken::Greek("π".to_string())),
                    "sigma" => tokens.push(MathToken::Greek("σ".to_string())),
                    "tau" => tokens.push(MathToken::Greek("τ".to_string())),
                    "phi" => tokens.push(MathToken::Greek("φ".to_string())),
                    "omega" => tokens.push(MathToken::Greek("ω".to_string())),
                    "infty" | "infinity" => tokens.push(MathToken::Greek("∞".to_string())),
                    "cdot" | "cdots" => tokens.push(MathToken::Char('·')),
                    "ldots" => tokens.push(MathToken::Literal("...".to_string())),
                    "text" => tokens.push(MathToken::Char(' ')), // simplified
                    "left" | "right" => {}                       // ignore bracket modifiers
                    _ => tokens.push(MathToken::Literal(format!("\\{}", name))),
                }
            }
            '_' => {
                tokens.push(MathToken::Sub);
                i += 1;
            }
            '^' => {
                tokens.push(MathToken::Sup);
                i += 1;
            }
            '{' => {
                i += 1;
                let mut depth = 1;
                let mut inner = String::new();
                while i < chars.len() {
                    if chars[i] == '{' {
                        depth += 1;
                    } else if chars[i] == '}' {
                        depth -= 1;
                        if depth == 0 {
                            i += 1;
                            break;
                        }
                    }
                    inner.push(chars[i]);
                    i += 1;
                }
                let inner_tokens = tokenize_latex(&inner);
                tokens.push(MathToken::Group(inner_tokens));
            }
            '}' => {
                i += 1;
            }
            ' ' | '\t' => {
                i += 1;
            }
            c => {
                tokens.push(MathToken::Char(c));
                i += 1;
            }
        }
    }
    tokens
}

struct MathRender {
    svg: String,
    width: i32,
    height: i32,
    baseline: i32, // y position of the text baseline
}

fn render_math_tokens(tokens: &[MathToken]) -> MathRender {
    // Simple left-to-right layout
    let mut x = 0i32;
    let base_font = 20i32;
    let baseline = 24i32;
    let mut svg = String::new();
    let mut max_h = baseline + 8;

    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            MathToken::Op(sym) => {
                // Big operator
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"28\" fill=\"#111\">{}</text>",
                    x, baseline + 6, escape_xml(sym)
                ));
                x += 22;
                max_h = max_h.max(baseline + 8);
                i += 1;
                // Check for sub/sup
                if i < tokens.len() {
                    if let MathToken::Sub = &tokens[i] {
                        i += 1;
                        let (sub_svg, _sub_w) =
                            render_sub_sup(tokens, i, true, x - 22, baseline + 14);
                        svg.push_str(&sub_svg);
                        max_h = max_h.max(baseline + 26);
                        // consume group
                        if i < tokens.len() {
                            i += 1;
                        }
                    }
                }
                if i < tokens.len() {
                    if let MathToken::Sup = &tokens[i] {
                        i += 1;
                        let (sup_svg, _sup_w) =
                            render_sub_sup(tokens, i, false, x - 22, baseline - 14);
                        svg.push_str(&sup_svg);
                        if i < tokens.len() {
                            i += 1;
                        }
                    }
                }
            }
            MathToken::Frac => {
                i += 1;
                // Next two groups are numerator and denominator
                let (num_tokens, den_tokens) = {
                    let num = if i < tokens.len() {
                        if let MathToken::Group(g) = &tokens[i] {
                            i += 1;
                            g.clone()
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    };
                    let den = if i < tokens.len() {
                        if let MathToken::Group(g) = &tokens[i] {
                            i += 1;
                            g.clone()
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    };
                    (num, den)
                };
                let num_r = render_math_tokens(&num_tokens);
                let den_r = render_math_tokens(&den_tokens);
                let frac_w = num_r.width.max(den_r.width) + 8;
                let num_x = x + (frac_w - num_r.width) / 2;
                let den_x = x + (frac_w - den_r.width) / 2;
                let num_y = baseline - num_r.baseline - 4;
                let den_y = baseline + 6;
                svg.push_str(&format!("<g transform=\"translate({},{})\">", num_x, num_y));
                svg.push_str(&num_r.svg);
                svg.push_str("</g>");
                svg.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#333\" stroke-width=\"1.5\"/>",
                    x, baseline + 1, x + frac_w, baseline + 1
                ));
                svg.push_str(&format!("<g transform=\"translate({},{})\">", den_x, den_y));
                svg.push_str(&den_r.svg);
                svg.push_str("</g>");
                x += frac_w + 4;
                max_h = max_h.max(den_y + den_r.height + 4);
            }
            MathToken::Sqrt => {
                i += 1;
                let inner_tokens = if i < tokens.len() {
                    if let MathToken::Group(g) = &tokens[i] {
                        i += 1;
                        g.clone()
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                };
                let inner = render_math_tokens(&inner_tokens);
                let iw = inner.width + 8;
                let ih = inner.height + 4;
                let top_y = baseline - inner.baseline - 2;
                // sqrt symbol: radical path
                svg.push_str(&format!(
                    "<path d=\"M {} {} L {} {} L {} {} L {} {}\" fill=\"none\" stroke=\"#333\" stroke-width=\"1.5\"/>",
                    x, baseline,
                    x + 6, baseline + 4,
                    x + 12, top_y,
                    x + 12 + iw, top_y
                ));
                svg.push_str(&format!(
                    "<g transform=\"translate({},{})\">{}</g>",
                    x + 16,
                    top_y + 4,
                    inner.svg
                ));
                x += 16 + iw + 4;
                max_h = max_h.max(top_y + ih);
            }
            MathToken::Sub => {
                i += 1;
                let (sub_svg, sub_w) = render_sub_sup(tokens, i, true, x, baseline + 8);
                svg.push_str(&sub_svg);
                x += sub_w;
                max_h = max_h.max(baseline + 20);
                if i < tokens.len() {
                    i += 1;
                }
            }
            MathToken::Sup => {
                i += 1;
                let (sup_svg, sup_w) = render_sub_sup(tokens, i, false, x, baseline - 12);
                svg.push_str(&sup_svg);
                x += sup_w;
                if i < tokens.len() {
                    i += 1;
                }
            }
            MathToken::Char(c) => {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\">{}</text>",
                    x, baseline, base_font, escape_xml(&c.to_string())
                ));
                x += if c.is_alphabetic() { 14 } else { 10 };
                i += 1;
            }
            MathToken::Greek(sym) => {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\">{}</text>",
                    x, baseline, base_font, escape_xml(sym)
                ));
                x += 16;
                i += 1;
            }
            MathToken::Literal(s) => {
                svg.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\">{}</text>",
                    x, baseline, base_font - 4, escape_xml(s)
                ));
                x += s.len() as i32 * 8;
                i += 1;
            }
            MathToken::Group(group_tokens) => {
                let inner = render_math_tokens(group_tokens);
                svg.push_str(&format!(
                    "<g transform=\"translate({},0)\">{}</g>",
                    x, inner.svg
                ));
                x += inner.width;
                max_h = max_h.max(inner.height);
                i += 1;
            }
        }
    }

    MathRender {
        svg,
        width: x,
        height: max_h,
        baseline,
    }
}

fn render_sub_sup(
    tokens: &[MathToken],
    i: usize,
    is_sub: bool,
    base_x: i32,
    y: i32,
) -> (String, i32) {
    if i >= tokens.len() {
        return (String::new(), 0);
    }
    let font_size = 13i32;
    let (content, width) = match &tokens[i] {
        MathToken::Group(g) => {
            let inner = render_math_tokens(g);
            (
                format!(
                    "<g transform=\"translate({},{})\"><g transform=\"scale(0.65)\">{}</g></g>",
                    base_x, y, inner.svg
                ),
                (inner.width as f64 * 0.65) as i32 + 2,
            )
        }
        MathToken::Char(c) => {
            let svg = format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\">{}</text>",
                base_x, y + font_size, font_size, escape_xml(&c.to_string())
            );
            (svg, 10)
        }
        MathToken::Greek(s) => {
            let svg = format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"serif\" font-size=\"{}\" fill=\"#111\">{}</text>",
                base_x, y + font_size, font_size, escape_xml(s)
            );
            (svg, 12)
        }
        _ => (String::new(), 0),
    };
    let _ = is_sub;
    (content, width)
}

// ─── Family 5a: @startsdl ────────────────────────────────────────────────────

fn render_sdl(source: &str) -> Result<String, Diagnostic> {
    let (body, title) = strip_block(source, "@startsdl", "@endsdl");

    let mut states: Vec<String> = Vec::new();
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
                let name = rest.to_string();
                if !name.is_empty() && !states.contains(&name) {
                    states.push(name);
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
            render_sdl_state_node(&mut out, name, x, y, state_w, state_h);
        }
    }

    out.push_str("</svg>");
    Ok(out)
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
fn render_sdl_state_node(out: &mut String, name: &str, x: i32, y: i32, w: i32, h: i32) {
    // SDL uses rounded rectangles for normal states
    out.push_str(&format!(
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"#e8eaf6\" stroke=\"#3949ab\" stroke-width=\"2\"/>",
        x, y, w, h
    ));
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"#1a237e\">{}</text>",
        x + w / 2, y + h / 2, escape_xml(name)
    ));
}

// ─── Family 5b: @startditaa ──────────────────────────────────────────────────

fn render_ditaa(source: &str) -> Result<String, Diagnostic> {
    let (body, title) = strip_block(source, "@startditaa", "@endditaa");

    if body.trim().is_empty() {
        return Err(Diagnostic::error(
            "[E_DITAA_EMPTY] @startditaa body is empty",
        ));
    }

    // Parse ASCII grid
    let lines: Vec<Vec<char>> = body.lines().map(|l| l.chars().collect()).collect();

    if lines.is_empty() {
        return Err(Diagnostic::error(
            "[E_DITAA_EMPTY] @startditaa has no grid content",
        ));
    }

    let cell_w = 10i32;
    let cell_h = 16i32;
    let grid_rows = lines.len() as i32;
    let grid_cols = lines.iter().map(|r| r.len()).max().unwrap_or(0) as i32;
    let title_h = if title.is_some() { 28 } else { 0 };
    let margin = 16i32;
    let svg_w = grid_cols * cell_w + margin * 2;
    let svg_h = grid_rows * cell_h + margin * 2 + title_h;

    // Find rectangles: look for '+' corners
    let get = |r: usize, c: usize| -> char {
        lines
            .get(r)
            .and_then(|row| row.get(c))
            .copied()
            .unwrap_or(' ')
    };

    let mut rects: Vec<(usize, usize, usize, usize)> = Vec::new(); // (r1,c1,r2,c2)
    for r1 in 0..lines.len() {
        for c1 in 0..lines[r1].len() {
            if get(r1, c1) == '+' {
                // Try to find a rectangle with top-left at (r1,c1)
                // Find right '+' on same row
                let mut c2 = c1 + 1;
                while c2 < lines[r1].len() {
                    if get(r1, c2) == '+' {
                        // Check top edge is all '-'
                        let top_ok = (c1 + 1..c2).all(|c| matches!(get(r1, c), '-' | '='));
                        if top_ok {
                            // Find bottom '+' on same column c1
                            let mut r2 = r1 + 1;
                            while r2 < lines.len() {
                                if get(r2, c1) == '+' && get(r2, c2) == '+' {
                                    // Check bottom edge
                                    let bot_ok =
                                        (c1 + 1..c2).all(|c| matches!(get(r2, c), '-' | '='));
                                    // Check left and right edges
                                    let left_ok =
                                        (r1 + 1..r2).all(|r| matches!(get(r, c1), '|' | ':'));
                                    let right_ok =
                                        (r1 + 1..r2).all(|r| matches!(get(r, c2), '|' | ':'));
                                    if bot_ok && left_ok && right_ok {
                                        rects.push((r1, c1, r2, c2));
                                        break;
                                    }
                                }
                                r2 += 1;
                            }
                        }
                        break;
                    } else if !matches!(get(r1, c2), '-' | '=') {
                        break;
                    }
                    c2 += 1;
                }
            }
        }
    }

    let mut out = String::new();
    out.push_str(&svg_header(svg_w, svg_h));
    out.push_str(svg_white_bg());

    if let Some(t) = &title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"14\" font-weight=\"600\" text-anchor=\"middle\" fill=\"#222\">{}</text>",
            svg_w / 2, margin + 16, escape_xml(t)
        ));
    }

    // Draw rectangles
    for &(r1, c1, r2, c2) in &rects {
        let rx = margin + c1 as i32 * cell_w;
        let ry = margin + title_h + r1 as i32 * cell_h;
        let rw = (c2 - c1) as i32 * cell_w;
        let rh = (r2 - r1) as i32 * cell_h;
        out.push_str(&format!(
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#f0f4ff\" stroke=\"#3344aa\" stroke-width=\"1.5\"/>",
            rx, ry, rw, rh
        ));

        // Render text content inside rectangle
        for (row_idx, row) in lines.iter().enumerate().take(r2).skip(r1 + 1) {
            // collect inner chars between c1+1 and c2
            let inner: String = ((c1 + 1)..c2)
                .filter_map(|c| row.get(c))
                .filter(|&&ch| ch != '|' && ch != ':')
                .collect();
            let inner = inner.trim().to_string();
            if !inner.is_empty() {
                let tx = rx + rw / 2;
                let ty = margin + title_h + row_idx as i32 * cell_h + cell_h - 3;
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" text-anchor=\"middle\" fill=\"#111\">{}</text>",
                    tx, ty, escape_xml(&inner)
                ));
            }
        }
    }

    // Draw arrows: find '->' or '<-', '|v', '|^' patterns and render as SVG arrows
    let adef = "<defs><marker id=\"da\" markerWidth=\"6\" markerHeight=\"6\" refX=\"5\" refY=\"3\" orient=\"auto\"><path d=\"M0,0 L0,6 L6,3 z\" fill=\"#444\"/></marker></defs>";
    out.push_str(adef);

    // Horizontal arrows: sequences of '-' ending in '>' or '<'
    for (row_idx, row) in lines.iter().enumerate() {
        let mut c = 0;
        while c < row.len() {
            if row[c] == '-' {
                // start of horizontal line
                let c_start = c;
                while c < row.len() && row[c] == '-' {
                    c += 1;
                }
                if c < row.len() && row[c] == '>' {
                    // right-pointing arrow from c_start to c
                    let x1 = margin + c_start as i32 * cell_w;
                    let x2 = margin + c as i32 * cell_w;
                    let y = margin + title_h + row_idx as i32 * cell_h + cell_h / 2;
                    // check if this is inside a rect border (skip)
                    let is_rect_edge = rects.iter().any(|&(r1, c1, r2, c2)| {
                        (row_idx == r1 || row_idx == r2) && c_start >= c1 && c <= c2
                    });
                    if !is_rect_edge {
                        out.push_str(&format!(
                            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#444\" stroke-width=\"1.5\" marker-end=\"url(#da)\"/>",
                            x1, y, x2, y
                        ));
                    }
                    c += 1;
                } else if c_start > 0 && row.get(c_start - 1) == Some(&'<') {
                    // left-pointing arrow
                    let x1 = margin + (c_start - 1) as i32 * cell_w;
                    let x2 = margin + c as i32 * cell_w;
                    let y = margin + title_h + row_idx as i32 * cell_h + cell_h / 2;
                    out.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#444\" stroke-width=\"1.5\"/>",
                        x1, y, x2, y
                    ));
                    c += 1;
                }
            } else {
                c += 1;
            }
        }
    }

    // Also render '+---+' connector lines as simple lines when not part of rect
    // (handled above — rectangles are already drawn)

    // Render remaining text characters that aren't part of box borders
    for (row_idx, row) in lines.iter().enumerate() {
        for (c, &ch) in row.iter().enumerate() {
            let in_rect = rects
                .iter()
                .any(|&(r1, c1, r2, c2)| row_idx >= r1 && row_idx <= r2 && c >= c1 && c <= c2);
            if in_rect {
                continue; // handled in rect rendering
            }
            // Skip connector/frame chars
            if matches!(
                ch,
                '+' | '-' | '|' | '=' | ':' | '>' | '<' | 'v' | '^' | ' '
            ) {
                continue;
            }
            let tx = margin + c as i32 * cell_w;
            let ty = margin + title_h + row_idx as i32 * cell_h + cell_h - 3;
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#222\">{}</text>",
                tx, ty, escape_xml(&ch.to_string())
            ));
        }
    }

    out.push_str("</svg>");
    Ok(out)
}
