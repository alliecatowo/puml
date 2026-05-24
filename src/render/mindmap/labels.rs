use super::super::creole_text;
use super::{MINDMAP_CHAR_PX, MINDMAP_NODE_PAD_X};
use crate::creole::tokenize_creole;

fn mindmap_max_chars(maximum_width: Option<i32>) -> Option<usize> {
    let px = maximum_width.filter(|w| *w > 0)?;
    let inner = px.saturating_sub(MINDMAP_NODE_PAD_X);
    Some((inner / MINDMAP_CHAR_PX).max(1) as usize)
}

/// Word-wrap `text` at `max_chars` per line (monospace heuristic, 7px/char).
fn wrap_mindmap_label(text: &str, max_chars: usize) -> String {
    text.split('\n')
        .flat_map(|line| wrap_mindmap_line(line, max_chars))
        .collect::<Vec<_>>()
        .join("\n")
}

fn wrap_mindmap_line(line: &str, max_chars: usize) -> Vec<String> {
    if line.is_empty() {
        return vec![String::new()];
    }
    let words = line.split_whitespace().collect::<Vec<_>>();
    if words.is_empty() {
        return vec![String::new()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in words {
        let word_len = word.chars().count();
        if current.is_empty() {
            if word_len <= max_chars {
                current.push_str(word);
            } else {
                lines.extend(chunk_mindmap_word(word, max_chars));
            }
            continue;
        }
        let next_len = current.chars().count() + 1 + word_len;
        if next_len <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(current);
            if word_len <= max_chars {
                current = word.to_string();
            } else {
                let mut chunks = chunk_mindmap_word(word, max_chars);
                let tail = chunks.pop().unwrap_or_default();
                lines.extend(chunks);
                current = tail;
            }
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn chunk_mindmap_word(text: &str, max_chars: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        current.push(ch);
        if current.chars().count() >= max_chars {
            out.push(current);
            current = String::new();
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        vec![String::new()]
    } else {
        out
    }
}

pub(super) fn prepare_mindmap_label(raw: &str, maximum_width: Option<i32>) -> String {
    match mindmap_max_chars(maximum_width) {
        Some(max_chars) => wrap_mindmap_label(raw, max_chars),
        None => raw.to_string(),
    }
}

fn mindmap_label_attrs(font_size: i32, font_family: &str, font_weight: &str) -> String {
    format!(
        "text-anchor=\"middle\" dominant-baseline=\"middle\" font-family=\"{font_family}\" font-size=\"{font_size}\" font-weight=\"{font_weight}\""
    )
}

/// Emit a centered multi-line `<text>` element with Creole markup support.
pub(super) fn render_mindmap_node_label(
    x: i32,
    y_center: i32,
    text: &str,
    font_size: i32,
    font_family: &str,
    font_weight: &str,
    font_color: &str,
) -> String {
    let attrs = mindmap_label_attrs(font_size, font_family, font_weight);
    let lines: Vec<&str> = text.split('\n').collect();
    if lines.len() <= 1 {
        return creole_text(x, y_center, &attrs, text, font_color);
    }

    let creole_lines = tokenize_creole(text);
    let line_h = (font_size as f32 * 1.25) as i32;
    let n = creole_lines.len() as i32;
    let total_h = line_h * (n - 1);
    let start_y = y_center - total_h / 2;
    let mut out = format!("<text x=\"{x}\" y=\"{start_y}\" {attrs}>");
    for (i, line) in creole_lines.iter().enumerate() {
        let y = start_y + (i as i32) * line_h;
        let inner = render_creole_line_to_tspans_inline(line, font_color);
        out.push_str(&format!(
            "<tspan x=\"{x}\" y=\"{y}\">{inner}</tspan>",
            x = x,
            y = y
        ));
    }
    out.push_str("</text>");
    out
}

fn render_creole_line_to_tspans_inline(
    line: &crate::creole::CreoleLine,
    default_color: &str,
) -> String {
    use crate::creole::render_creole_line_to_tspans;
    render_creole_line_to_tspans(line, 0, default_color)
}

/// Width of a multi-line label = the longest line, in monospace char units.
pub(super) fn multiline_char_width(text: &str) -> i32 {
    text.split('\n')
        .map(|s| s.chars().count() as i32)
        .max()
        .unwrap_or(0)
}

/// Number of lines in a (possibly multi-line) label.
pub(super) fn multiline_line_count(text: &str) -> i32 {
    text.split('\n').count() as i32
}
