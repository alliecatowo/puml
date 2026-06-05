use super::inline::parse_inline;
use super::{CreoleLine, CreoleSpan};

pub(super) fn normalize_line_breaks(text: &str) -> String {
    // Replace <br>, <br/>, <br /> (case-insensitive) with \n.
    // Replace the two-character sequence backslash + 'n' with \n.
    let mut s = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Check for <br> variants.
        if bytes[i] == b'<' {
            // Try to match <br>, <br/>, <br />
            let rest = &text[i..];
            let rest_lower: String = rest
                .chars()
                .take(7)
                .collect::<String>()
                .to_ascii_lowercase();
            if rest_lower.starts_with("<br>") {
                s.push('\n');
                i += 4;
                continue;
            }
            if rest_lower.starts_with("<br/>") {
                s.push('\n');
                i += 5;
                continue;
            }
            if rest_lower.starts_with("<br />") {
                s.push('\n');
                i += 6;
                continue;
            }
        }
        // Check for \n (two-char backslash + n).
        if bytes[i] == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'n' {
            s.push('\n');
            i += 2;
            continue;
        }
        s.push(text[i..].chars().next().unwrap());
        // Advance by the char's byte length.
        let ch = text[i..].chars().next().unwrap();
        i += ch.len_utf8();
    }
    s
}

pub(super) fn parse_block_line(raw_line: &str) -> CreoleLine {
    let trimmed = raw_line.trim();
    if trimmed.is_empty() {
        return parse_inline(raw_line);
    }

    if let Some((level, text)) = parse_heading_line(trimmed) {
        let mut line = parse_inline(text);
        // Font sizes approximate the h1=1.5x, h2=1.3x, h3=1.15x multipliers
        // against a base of 16 px: 24, 21, 18, 16.
        let size = match level {
            1 => 24,
            2 => 21,
            3 => 18,
            _ => 16,
        };
        for span in &mut line {
            span.bold = true;
            span.size = Some(size);
        }
        return line;
    }

    // Plain horizontal rules (----, ====, ____) become SVG <line> sentinels.
    // Titled section rules (.. Title ..) keep their text-based rendering.
    if is_plain_horizontal_rule(trimmed) {
        return vec![CreoleSpan {
            is_hr: true,
            ..Default::default()
        }];
    }
    if let Some(titled_text) = parse_titled_rule_line(trimmed) {
        return vec![CreoleSpan {
            text: titled_text,
            mono: true,
            color: Some("#64748b".to_string()),
            ..Default::default()
        }];
    }

    if let Some((prefix, rest)) = parse_list_line(raw_line) {
        let mut line = parse_inline(rest);
        line.insert(
            0,
            CreoleSpan {
                text: prefix,
                ..Default::default()
            },
        );
        return line;
    }

    if let Some((prefix, rest)) = parse_tree_line(raw_line) {
        let mut line = parse_inline(rest);
        line.insert(
            0,
            CreoleSpan {
                text: prefix,
                mono: true,
                ..Default::default()
            },
        );
        return line;
    }

    if let Some(line) = parse_table_line(trimmed) {
        return line;
    }

    if let Some(line) = parse_definition_list_line(trimmed) {
        return line;
    }

    parse_inline(raw_line)
}

fn parse_heading_line(line: &str) -> Option<(usize, &str)> {
    let level = line.chars().take_while(|&ch| ch == '=').count();
    if !(1..=4).contains(&level) {
        return None;
    }

    let rest = line.get(level..)?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }

    let mut text = rest.trim();
    let suffix = "=".repeat(level);
    if text.ends_with(&suffix) {
        text = text[..text.len() - suffix.len()].trim_end();
    }
    if text.is_empty() {
        return None;
    }
    Some((level, text))
}

/// Returns `true` for lines that are solely repetitions of `-`, `=`, or `_`
/// (at least 4 characters).  These become SVG `<line>` elements.
fn is_plain_horizontal_rule(line: &str) -> bool {
    line.len() >= 4
        && matches!(line.as_bytes().first(), Some(b'-' | b'=' | b'_'))
        && line.bytes().all(|b| b == line.as_bytes()[0])
}

/// Returns `Some(text)` for titled section-rule syntaxes that keep a
/// text-based representation:
///   `.. Title ..`  — dot-dot fencing
///   `==+ Title ==+` — five or more equals signs on each side (PlantUML
///                     Chapter 22 titled divider, e.g. `=== Section ===`)
fn parse_titled_rule_line(line: &str) -> Option<String> {
    // `.. Title ..` variant.
    if line.len() >= 4 && line.starts_with("..") && line.ends_with("..") {
        let title = line[2..line.len() - 2].trim();
        if title.is_empty() {
            return Some("------------------------".to_string());
        }
        return Some(format!("---------- {title} ----------"));
    }

    // `====+ Title ====+` variant: 5+ equals signs as fences (but NOT a plain
    // HR — those are all-equals lines with no text).  At least 3 `=` on each
    // side and a non-empty title in between.
    let leading = line.chars().take_while(|&ch| ch == '=').count();
    if leading >= 3 {
        let rest = &line[leading..];
        let trailing = rest.chars().rev().take_while(|&ch| ch == '=').count();
        if trailing >= 3 {
            let title = rest[..rest.len() - trailing].trim();
            if !title.is_empty() {
                return Some(format!("---------- {title} ----------"));
            }
        }
    }

    None
}

fn parse_list_line(line: &str) -> Option<(String, &str)> {
    let trimmed_start = line.trim_start();
    let leading_spaces = line.len().saturating_sub(trimmed_start.len());
    let marker = trimmed_start.chars().next()?;
    if marker != '*' && marker != '#' {
        return None;
    }

    let depth = trimmed_start.chars().take_while(|&ch| ch == marker).count();
    if depth == 0 {
        return None;
    }

    let rest = trimmed_start.get(depth..)?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }

    // Use a Unicode bullet glyph for unordered lists (#1554) and a numeric
    // prefix for ordered lists.  Nested items are indented by 2 spaces per level.
    let bullet = if marker == '*' {
        match depth {
            1 => "\u{2022} ", // • BULLET
            2 => "\u{25E6} ", // ◦ WHITE BULLET
            _ => "\u{2023} ", // ‣ TRIANGULAR BULLET
        }
    } else {
        "1. "
    };
    let prefix = format!(
        "{}{}",
        " ".repeat(leading_spaces + depth.saturating_sub(1) * 2),
        bullet,
    );
    Some((prefix, rest.trim_start()))
}

fn parse_tree_line(line: &str) -> Option<(String, &str)> {
    let trimmed_start = line.trim_start();
    let leading_spaces = line.len().saturating_sub(trimmed_start.len());
    let rest = trimmed_start.strip_prefix("|_")?;
    if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
        return None;
    }

    Some((
        format!("{}{} ", " ".repeat(leading_spaces), "`-"),
        rest.trim_start(),
    ))
}

fn parse_table_line(line: &str) -> Option<CreoleLine> {
    let (row_background, body) = parse_row_background(line);
    if !body.starts_with('|') {
        return None;
    }

    let cells = body
        .split('|')
        .skip(1)
        .filter(|cell| !cell.is_empty())
        .collect::<Vec<_>>();
    if cells.is_empty() {
        return None;
    }

    let mut line = Vec::new();
    for (idx, raw_cell) in cells.iter().enumerate() {
        if idx > 0 {
            line.push(CreoleSpan {
                text: " | ".to_string(),
                mono: true,
                ..Default::default()
            });
        }

        let (cell_background, cell) = parse_cell_background(raw_cell.trim());
        let (header, text) = if let Some(rest) = cell.strip_prefix('=') {
            (true, rest.trim())
        } else {
            (false, cell.trim())
        };
        let mut spans = parse_inline(text);
        for span in &mut spans {
            span.bold |= header;
            span.mono = true;
            span.background = cell_background
                .clone()
                .or_else(|| row_background.clone())
                .or_else(|| span.background.clone());
        }
        line.extend(spans);
    }

    Some(line)
}

fn parse_row_background(line: &str) -> (Option<String>, &str) {
    if let Some(rest) = line.strip_prefix("<#") {
        if let Some(close) = rest.find('>') {
            let color = &rest[..close];
            let body = &rest[close + 1..];
            if body.starts_with('|') && !color.is_empty() {
                return (Some(creole_hash_color(color)), body);
            }
        }
    }
    (None, line)
}

fn parse_cell_background(cell: &str) -> (Option<String>, &str) {
    if let Some(rest) = cell.strip_prefix("<#") {
        if let Some(close) = rest.find('>') {
            let color = &rest[..close];
            let body = rest[close + 1..].trim_start();
            if !color.is_empty() {
                return (Some(creole_hash_color(color)), body);
            }
        }
    }
    (None, cell)
}

fn creole_hash_color(color: &str) -> String {
    if color.chars().all(|ch| ch.is_ascii_hexdigit()) && matches!(color.len(), 3 | 6 | 8) {
        format!("#{color}")
    } else {
        color.to_string()
    }
}

/// Parse a Creole definition-list line of the form `; Term : Definition` or
/// just `; Term` (term without an inline definition).
///
/// The term is rendered bold; if a definition is present it follows a " : "
/// separator in normal weight. Both term and definition text go through the
/// inline parser so they can contain their own inline markup.
///
/// Returns `None` for lines that do not start with `;` followed by whitespace.
fn parse_definition_list_line(line: &str) -> Option<CreoleLine> {
    let rest = line.strip_prefix(';')?;
    if rest.is_empty() || !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let content = rest.trim_start();
    if content.is_empty() {
        return None;
    }

    let mut result = Vec::new();

    // Split on the first ` : ` separator (space-colon-space) to separate
    // term from definition.  A bare `;` without ` : ` renders just the term.
    if let Some(sep) = content.find(" : ") {
        let term_str = &content[..sep];
        let def_str = &content[sep + 3..];

        // Term spans — apply bold to every span.
        let mut term_spans = parse_inline(term_str);
        for s in &mut term_spans {
            s.bold = true;
        }
        result.extend(term_spans);

        // Separator.
        result.push(CreoleSpan {
            text: " : ".to_string(),
            ..Default::default()
        });

        // Definition spans — plain inline text.
        let def_spans = parse_inline(def_str);
        result.extend(def_spans);
    } else {
        // Term only — render bold.
        let mut term_spans = parse_inline(content);
        for s in &mut term_spans {
            s.bold = true;
        }
        result.extend(term_spans);
    }

    Some(result)
}
