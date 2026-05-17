// ─── Family 2: @startebnf ─────────────────────────────────────────────────────

use super::railroad::{layout_rail_with_style, RailLayout, RailNode, RailStyle};
use super::shared::{escape_xml, strip_block, svg_header, svg_white_bg};
use crate::diagnostic::Diagnostic;
use std::collections::BTreeMap;

pub(super) fn render_ebnf(source: &str) -> Result<String, Diagnostic> {
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

    let mut in_style_block = false;
    for raw in body.lines() {
        let line = raw.trim();
        if line.is_empty() {
            rule_lines.push(raw.to_string());
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("<style") {
            in_style_block = true;
            continue;
        }
        if in_style_block {
            apply_ebnf_style_directive(line, &mut style);
            if lower.contains("</style>") {
                in_style_block = false;
            }
            continue;
        }
        if lower.starts_with("title ") || lower.starts_with("legend ") || lower == "end legend" {
            continue;
        }
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
    Star,
    Question,
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
            '*' => {
                chars.next();
                tokens.push(EbnfToken::Star);
            }
            '?' => {
                chars.next();
                let mut special = String::new();
                while let Some(&ch) = chars.peek() {
                    chars.next();
                    if ch == '?' {
                        break;
                    }
                    special.push(ch);
                }
                tokens.push(EbnfToken::Question);
                tokens.push(EbnfToken::Literal(format!("?{}?", special.trim())));
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
    if let Some((node, next)) = parse_ebnf_prefix_repeat(tokens, pos) {
        return (node, next);
    }
    let (node, p) = match &tokens[pos] {
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
        EbnfToken::Question => {
            if let Some(EbnfToken::Literal(s)) = tokens.get(pos + 1) {
                let text = s.trim_matches('?').trim().to_string();
                (RailNode::Special(text), pos + 2)
            } else {
                (RailNode::Empty, pos + 1)
            }
        }
        EbnfToken::Literal(s) => (RailNode::Literal(s.clone()), pos + 1),
        EbnfToken::Ident(s) => (RailNode::NonTerminal(s.clone()), pos + 1),
        _ => (RailNode::Empty, pos + 1),
    };
    if let Some((spec, next)) = parse_ebnf_counted_repeat(tokens, p) {
        (RailNode::CountedRepeat(Box::new(node), spec), next)
    } else {
        (node, p)
    }
}

fn parse_ebnf_prefix_repeat(tokens: &[EbnfToken], pos: usize) -> Option<(RailNode, usize)> {
    let Some(EbnfToken::Ident(count)) = tokens.get(pos) else {
        return None;
    };
    if !count.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    if tokens.get(pos + 1) != Some(&EbnfToken::Star) {
        return None;
    }
    let (inner, next) = ebnf_parse_item(tokens, pos + 2);
    Some((
        RailNode::CountedRepeat(Box::new(inner), format!("{{{count}}}")),
        next,
    ))
}

fn parse_ebnf_counted_repeat(tokens: &[EbnfToken], pos: usize) -> Option<(String, usize)> {
    if pos >= tokens.len() || tokens[pos] != EbnfToken::LBrace {
        return None;
    }
    let mut p = pos + 1;
    let mut saw_digit = false;
    let mut saw_comma = false;
    let mut spec = String::new();
    while p < tokens.len() {
        match &tokens[p] {
            EbnfToken::Ident(s) if s.chars().all(|ch| ch.is_ascii_digit()) => {
                saw_digit = true;
                spec.push_str(s);
                p += 1;
            }
            EbnfToken::Comma if !saw_comma => {
                saw_comma = true;
                spec.push(',');
                p += 1;
            }
            EbnfToken::RBrace => {
                if saw_digit {
                    return Some((format!("{{{spec}}}"), p + 1));
                }
                return None;
            }
            _ => return None,
        }
    }
    None
}
