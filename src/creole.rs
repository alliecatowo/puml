/// Creole inline text formatting for PlantUML labels.
///
/// Supports: **bold**, //italic//, ""mono"", __underline__, --strikethrough--,
/// ~~wave underline~~, [[url label]] hyperlinks, <color:X>text</color>,
/// <size:N>text</size>, legacy HTML-style tags, \n line breaks, basic block
/// Creole line forms, and <&icon> placeholders.

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreoleSpan {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub mono: bool,
    pub underline: bool,
    pub strike: bool,
    pub wave: bool,
    pub color: Option<String>,
    pub background: Option<String>,
    pub size: Option<u32>,
    pub font: Option<String>,
    pub baseline_shift: Option<String>,
    pub decoration_color: Option<String>,
    pub link: Option<String>,
    pub link_tooltip: Option<String>,
}

/// A line is a list of spans. Multiple lines come from \n splits.
pub type CreoleLine = Vec<CreoleSpan>;

/// Decode PlantUML's input-side Unicode escape forms without treating decoded
/// characters as new Creole markup.
///
/// Supported forms:
///   - decimal numeric character references: `&#8734;`
///   - hexadecimal numeric character references: `&#x221E;`
///   - PlantUML codepoint tags: `<U+221E>`
///   - a deterministic small emoji subset / fallback: `<:calendar:>`,
///     `<:1f600:>`, unknown names as `:name:`
pub fn decode_unicode_escapes(text: &str) -> String {
    if !text.contains("&#")
        && !text.contains("<U+")
        && !text.contains("<u+")
        && !text.contains("<:")
        && !text.contains("<#")
    {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];

        if let Some((decoded, consumed)) = decode_numeric_character_reference(rest) {
            out.push(decoded);
            i += consumed;
            continue;
        }

        if let Some((decoded, consumed)) = decode_codepoint_tag(rest) {
            out.push(decoded);
            i += consumed;
            continue;
        }

        if let Some((decoded, consumed)) = decode_emoji_tag(rest) {
            out.push_str(&decoded);
            i += consumed;
            continue;
        }

        if let Some((decoded, consumed)) = decode_colored_emoji_tag(rest) {
            out.push_str(&decoded);
            i += consumed;
            continue;
        }

        let ch = rest.chars().next().expect("non-empty rest");
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Tokenize `text` into lines of spans.
///
/// Line breaks come from:
///   - literal `\n` characters in the string
///   - the two-character sequence `\\n` (backslash + n) in the source
///   - `<br>` / `<br/>` tags
pub fn tokenize_creole(text: &str) -> Vec<CreoleLine> {
    // First normalize line-break representations into real '\n'.
    let normalized = normalize_line_breaks(text);

    let mut all_lines: Vec<CreoleLine> = Vec::new();
    for raw_line in normalized.split('\n') {
        all_lines.push(parse_block_line(raw_line));
    }
    all_lines
}

/// Render a single `CreoleLine` to SVG `<tspan>` elements.
///
/// `base_x` is the x coordinate of the text element.
/// `default_color` is used when no span-level color override is present.
/// Returns a string of concatenated `<tspan>` elements (no wrapper `<text>`).
pub fn render_creole_line_to_tspans(
    line: &CreoleLine,
    _base_x: i32,
    default_color: &str,
) -> String {
    if line.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    for span in line {
        if let Some(url) = &span.link {
            out.push_str(&format!(
                "<a xlink:href=\"{}\" xmlns:xlink=\"http://www.w3.org/1999/xlink\">",
                escape_attr(url)
            ));
            if let Some(tooltip) = &span.link_tooltip {
                out.push_str(&format!("<title>{}</title>", escape_xml(tooltip)));
            }
        }

        let mut style_parts: Vec<String> = Vec::new();
        if span.bold {
            style_parts.push("font-weight=\"bold\"".to_string());
        }
        if span.italic {
            style_parts.push("font-style=\"italic\"".to_string());
        }
        if span.mono {
            style_parts.push("font-family=\"monospace\"".to_string());
        }
        if let Some(font) = &span.font {
            style_parts.push(format!("font-family=\"{}\"", escape_attr(font)));
        }

        let mut text_decorations: Vec<&str> = Vec::new();
        if span.underline || span.link.is_some() {
            text_decorations.push("underline");
        }
        if span.strike {
            text_decorations.push("line-through");
        }
        if span.wave {
            text_decorations.push("underline");
            style_parts.push("text-decoration-style=\"wavy\"".to_string());
        }
        if !text_decorations.is_empty() {
            style_parts.push(format!(
                "text-decoration=\"{}\"",
                text_decorations.join(" ")
            ));
        }
        if let Some(decoration_color) = &span.decoration_color {
            style_parts.push(format!(
                "text-decoration-color=\"{}\"",
                escape_attr(decoration_color)
            ));
        }

        let color = if span.link.is_some() {
            "blue".to_string()
        } else if let Some(c) = &span.color {
            c.clone()
        } else {
            default_color.to_string()
        };
        if color != default_color || span.link.is_some() {
            style_parts.push(format!("fill=\"{}\"", escape_attr(&color)));
        }

        if let Some(size) = span.size {
            style_parts.push(format!("font-size=\"{}\"", size));
        }
        if let Some(background) = &span.background {
            style_parts.push(format!(
                "data-creole-back=\"{}\" style=\"background-color:{}\"",
                escape_attr(background),
                escape_attr(background)
            ));
        }
        if let Some(baseline_shift) = &span.baseline_shift {
            style_parts.push(format!(
                "baseline-shift=\"{}\"",
                escape_attr(baseline_shift)
            ));
            if span.size.is_none() {
                style_parts.push("font-size=\"80%\"".to_string());
            }
        }

        let attrs = if style_parts.is_empty() {
            String::new()
        } else {
            format!(" {}", style_parts.join(" "))
        };

        out.push_str(&format!(
            "<tspan{}>{}</tspan>",
            attrs,
            escape_xml(&span.text)
        ));

        if span.link.is_some() {
            out.push_str("</a>");
        }
    }
    out
}

/// Render multi-line creole text into a sequence of `<tspan>` elements with
/// `dy="1.2em"` for subsequent lines. The first line sits at the caller's `y`
/// position; later lines advance by `dy`.
///
/// Returns a flat string of `<tspan>` elements. Pass this inside a `<text>` tag.
pub fn render_creole_to_svg_tspans(
    lines: &[CreoleLine],
    base_x: i32,
    default_color: &str,
) -> String {
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        let dy_attr = if i == 0 {
            String::new()
        } else {
            " dy=\"1.2em\"".to_string()
        };
        let x_attr = format!(" x=\"{}\"", base_x);
        let inner = render_creole_line_to_tspans(line, base_x, default_color);
        out.push_str(&format!("<tspan{}{}>", x_attr, dy_attr));
        out.push_str(&inner);
        out.push_str("</tspan>");
    }
    out
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn normalize_line_breaks(text: &str) -> String {
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

/// State carried through the inline parser.
#[derive(Debug, Default, Clone)]
struct InlineState {
    bold: bool,
    italic: bool,
    mono: bool,
    underline: bool,
    strike: bool,
    wave: bool,
    color: Option<String>,
    background: Option<String>,
    size: Option<u32>,
    font: Option<String>,
    baseline_shift: Option<String>,
    decoration_color: Option<String>,
    plain: bool,
}

fn span_from_state(text: String, state: &InlineState) -> CreoleSpan {
    if state.plain {
        return CreoleSpan {
            text,
            ..Default::default()
        };
    }

    CreoleSpan {
        text,
        bold: state.bold,
        italic: state.italic,
        mono: state.mono,
        underline: state.underline,
        strike: state.strike,
        wave: state.wave,
        color: state.color.clone(),
        background: state.background.clone(),
        size: state.size,
        font: state.font.clone(),
        baseline_shift: state.baseline_shift.clone(),
        decoration_color: state.decoration_color.clone(),
        link: None,
        link_tooltip: None,
    }
}

fn parse_block_line(raw_line: &str) -> CreoleLine {
    let trimmed = raw_line.trim();
    if trimmed.is_empty() {
        return parse_inline(raw_line);
    }

    if let Some((level, text)) = parse_heading_line(trimmed) {
        let mut line = parse_inline(text);
        let size = match level {
            1 => 24,
            2 => 20,
            3 => 16,
            _ => 14,
        };
        for span in &mut line {
            span.bold = true;
            span.size = Some(size);
        }
        return line;
    }

    if let Some(text) = parse_horizontal_rule_line(trimmed) {
        return vec![CreoleSpan {
            text,
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

    parse_inline(raw_line)
}

fn parse_inline(text: &str) -> CreoleLine {
    let mut spans: Vec<CreoleSpan> = Vec::new();
    let mut state = InlineState::default();
    let mut buf = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    macro_rules! flush {
        () => {
            if !buf.is_empty() {
                spans.push(span_from_state(buf.clone(), &state));
                buf.clear();
            }
        };
    }

    while i < len {
        let rest: String = chars[i..].iter().collect();

        if state.plain {
            if rest.to_ascii_lowercase().starts_with("</plain>") {
                flush!();
                state.plain = false;
                i += 8;
            } else {
                buf.push(chars[i]);
                i += 1;
            }
            continue;
        }

        // --- ~ escape: keep the next Creole metacharacter literal. ---
        if chars[i] == '~' && i + 1 < len && chars[i + 1] != '~' {
            if i + 2 < len && is_creole_pair(chars[i + 1], chars[i + 2]) {
                buf.push(chars[i + 1]);
                buf.push(chars[i + 2]);
                i += 3;
            } else {
                buf.push(chars[i + 1]);
                i += 2;
            }
            continue;
        }

        // --- **bold** ---
        if chars[i] == '*' && i + 1 < len && chars[i + 1] == '*' {
            if !state.bold && !pair_exists_after(&chars, i + 2, '*', '*') && buf.contains("**") {
                buf.push('*');
                buf.push('*');
                i += 2;
                continue;
            }
            flush!();
            state.bold = !state.bold;
            i += 2;
            continue;
        }

        // --- //italic// ---
        if chars[i] == '/' && i + 1 < len && chars[i + 1] == '/' {
            if !state.italic && !pair_exists_after(&chars, i + 2, '/', '/') && buf.contains("//") {
                buf.push('/');
                buf.push('/');
                i += 2;
                continue;
            }
            flush!();
            state.italic = !state.italic;
            i += 2;
            continue;
        }

        // --- ""mono"" ---
        if chars[i] == '"' && i + 1 < len && chars[i + 1] == '"' {
            if !state.mono && !pair_exists_after(&chars, i + 2, '"', '"') && buf.contains("\"\"") {
                buf.push('"');
                buf.push('"');
                i += 2;
                continue;
            }
            flush!();
            state.mono = !state.mono;
            i += 2;
            continue;
        }

        // --- __underline__ ---
        if chars[i] == '_' && i + 1 < len && chars[i + 1] == '_' {
            if !state.underline && !pair_exists_after(&chars, i + 2, '_', '_') && buf.contains("__")
            {
                buf.push('_');
                buf.push('_');
                i += 2;
                continue;
            }
            flush!();
            state.underline = !state.underline;
            i += 2;
            continue;
        }

        // --- --strike-- ---
        if chars[i] == '-' && i + 1 < len && chars[i + 1] == '-' {
            if !state.strike && !pair_exists_after(&chars, i + 2, '-', '-') && buf.contains("--") {
                buf.push('-');
                buf.push('-');
                i += 2;
                continue;
            }
            flush!();
            state.strike = !state.strike;
            i += 2;
            continue;
        }

        // --- ~~wave underline~~ ---
        if chars[i] == '~' && i + 1 < len && chars[i + 1] == '~' {
            if !state.wave && !pair_exists_after(&chars, i + 2, '~', '~') && buf.contains("~~") {
                buf.push('~');
                buf.push('~');
                i += 2;
                continue;
            }
            flush!();
            state.wave = !state.wave;
            i += 2;
            continue;
        }

        // --- [[url label]] or [[url]] ---
        if chars[i] == '[' && i + 1 < len && chars[i + 1] == '[' {
            flush!();
            // Find closing ]]
            let start = i + 2;
            let mut j = start;
            while j + 1 < len && !(chars[j] == ']' && chars[j + 1] == ']') {
                j += 1;
            }
            if j + 1 < len {
                let inner: String = chars[start..j].iter().collect();
                let (url, tooltip, label) = parse_link_inner(&inner);
                let mut span = span_from_state(label, &state);
                span.underline = true;
                span.color = Some("blue".to_string());
                span.link = Some(url);
                span.link_tooltip = tooltip;
                spans.push(span);
                i = j + 2;
            } else {
                // Malformed — treat as literal
                buf.push('[');
                buf.push('[');
                i += 2;
            }
            continue;
        }

        // --- HTML / Creole tags starting with '<' ---
        if chars[i] == '<' {
            // <&icon>  — require non-empty icon name
            if let Some(inner) = strip_tag_prefix(&rest, "<&", ">").filter(|s| !s.is_empty()) {
                flush!();
                spans.push(span_from_state(format!("[{}]", inner.trim()), &state));
                i += 2 + inner.len() + 1;
                continue;
            }

            // <code>...</code> is inline verbatim monospaced text.
            if rest.to_ascii_lowercase().starts_with("<code>") {
                if let Some(close) = find_case_insensitive(&rest, "</code>") {
                    flush!();
                    let inner = &rest[6..close];
                    let mut code_state = state.clone();
                    code_state.mono = true;
                    code_state.bold = false;
                    code_state.italic = false;
                    code_state.underline = false;
                    code_state.strike = false;
                    code_state.wave = false;
                    spans.push(span_from_state(inner.to_string(), &code_state));
                    i += close + 7;
                    continue;
                }
            }

            // <color:X>
            if let Some(after) = parse_open_tag_with_value(&rest, "color") {
                flush!();
                let color_val = after.0.to_string();
                state.color = Some(color_val);
                i += after.1;
                continue;
            }

            // </color>
            if rest.to_ascii_lowercase().starts_with("</color>") {
                flush!();
                state.color = None;
                i += 8;
                continue;
            }

            // <size:N>
            if let Some(after) = parse_open_tag_with_value(&rest, "size") {
                flush!();
                if let Ok(n) = after.0.parse::<u32>() {
                    state.size = Some(n);
                }
                i += after.1;
                continue;
            }

            // </size>
            if rest.to_ascii_lowercase().starts_with("</size>") {
                flush!();
                state.size = None;
                i += 7;
                continue;
            }

            // <font:Name>
            if let Some(after) = parse_open_tag_with_value(&rest, "font") {
                flush!();
                state.font = Some(after.0.to_string());
                i += after.1;
                continue;
            }

            // </font>
            if rest.to_ascii_lowercase().starts_with("</font>") {
                flush!();
                state.font = None;
                i += 7;
                continue;
            }

            // <back:X>
            if let Some(after) = parse_open_tag_with_value(&rest, "back") {
                flush!();
                state.background = Some(after.0.to_string());
                i += after.1;
                continue;
            }

            // </back>
            if rest.to_ascii_lowercase().starts_with("</back>") {
                flush!();
                state.background = None;
                i += 7;
                continue;
            }

            // <b>
            if rest.to_ascii_lowercase().starts_with("<b>") {
                flush!();
                state.bold = true;
                i += 3;
                continue;
            }

            // </b>
            if rest.to_ascii_lowercase().starts_with("</b>") {
                flush!();
                state.bold = false;
                i += 4;
                continue;
            }

            // <i>
            if rest.to_ascii_lowercase().starts_with("<i>") {
                flush!();
                state.italic = true;
                i += 3;
                continue;
            }

            // </i>
            if rest.to_ascii_lowercase().starts_with("</i>") {
                flush!();
                state.italic = false;
                i += 4;
                continue;
            }

            // <u> / <u:color>
            if rest.to_ascii_lowercase().starts_with("<u>") {
                flush!();
                state.underline = true;
                i += 3;
                continue;
            }
            if let Some(after) = parse_open_tag_with_value(&rest, "u") {
                flush!();
                state.underline = true;
                state.decoration_color = Some(after.0.to_string());
                i += after.1;
                continue;
            }

            // </u>
            if rest.to_ascii_lowercase().starts_with("</u>") {
                flush!();
                state.underline = false;
                if !state.strike && !state.wave {
                    state.decoration_color = None;
                }
                i += 4;
                continue;
            }

            // <s> / <s:color>
            if rest.to_ascii_lowercase().starts_with("<s>") {
                flush!();
                state.strike = true;
                i += 3;
                continue;
            }
            if let Some(after) = parse_open_tag_with_value(&rest, "s") {
                flush!();
                state.strike = true;
                state.decoration_color = Some(after.0.to_string());
                i += after.1;
                continue;
            }
            if rest.to_ascii_lowercase().starts_with("</s>") {
                flush!();
                state.strike = false;
                if !state.underline && !state.wave {
                    state.decoration_color = None;
                }
                i += 4;
                continue;
            }

            // <w> / <w:color>
            if rest.to_ascii_lowercase().starts_with("<w>") {
                flush!();
                state.wave = true;
                i += 3;
                continue;
            }
            if let Some(after) = parse_open_tag_with_value(&rest, "w") {
                flush!();
                state.wave = true;
                state.decoration_color = Some(after.0.to_string());
                i += after.1;
                continue;
            }
            if rest.to_ascii_lowercase().starts_with("</w>") {
                flush!();
                state.wave = false;
                if !state.underline && !state.strike {
                    state.decoration_color = None;
                }
                i += 4;
                continue;
            }

            if rest.to_ascii_lowercase().starts_with("<plain>") {
                flush!();
                state.plain = true;
                i += 7;
                continue;
            }

            if rest.to_ascii_lowercase().starts_with("<sub>") {
                flush!();
                state.baseline_shift = Some("sub".to_string());
                i += 5;
                continue;
            }
            if rest.to_ascii_lowercase().starts_with("</sub>") {
                flush!();
                state.baseline_shift = None;
                i += 6;
                continue;
            }
            if rest.to_ascii_lowercase().starts_with("<sup>") {
                flush!();
                state.baseline_shift = Some("super".to_string());
                i += 5;
                continue;
            }
            if rest.to_ascii_lowercase().starts_with("</sup>") {
                flush!();
                state.baseline_shift = None;
                i += 6;
                continue;
            }

            // Not a recognized tag — treat '<' as literal text; escape_xml handles it.
            buf.push('<');
            i += 1;
            continue;
        }

        buf.push(chars[i]);
        i += 1;
    }

    flush!();
    if spans.is_empty() && buf.is_empty() {
        spans.push(CreoleSpan {
            text: String::new(),
            ..Default::default()
        });
    }
    spans
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

fn parse_horizontal_rule_line(line: &str) -> Option<String> {
    if line.len() >= 4
        && matches!(line.as_bytes().first(), Some(b'-' | b'=' | b'_'))
        && line.bytes().all(|b| b == line.as_bytes()[0])
    {
        return Some("------------------------".to_string());
    }

    if line.len() >= 4 && line.starts_with("..") && line.ends_with("..") {
        let title = line[2..line.len() - 2].trim();
        if title.is_empty() {
            return Some("------------------------".to_string());
        }
        return Some(format!("---------- {title} ----------"));
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

    let prefix = format!(
        "{}{}{}",
        " ".repeat(leading_spaces + depth.saturating_sub(1) * 2),
        if marker == '*' { "- " } else { "1. " },
        ""
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

fn parse_link_inner(inner: &str) -> (String, Option<String>, String) {
    if let Some(open) = inner.find('{') {
        if let Some(relative_close) = inner[open + 1..].find('}') {
            let close = open + 1 + relative_close;
            let url = inner[..open].trim().to_string();
            if !url.is_empty() {
                let tooltip = inner[open + 1..close].to_string();
                let label = inner[close + 1..].trim();
                let label = if label.is_empty() {
                    url.clone()
                } else {
                    label.to_string()
                };
                return (url, Some(tooltip), label);
            }
        }
    }

    let (target, label) = if let Some(sp) = inner.find(char::is_whitespace) {
        (&inner[..sp], inner[sp..].trim_start().to_string())
    } else {
        (inner, inner.to_string())
    };

    (target.to_string(), None, label)
}

fn find_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())
}

fn is_creole_pair(a: char, b: char) -> bool {
    matches!(
        (a, b),
        ('*', '*') | ('/', '/') | ('"', '"') | ('_', '_') | ('-', '-') | ('~', '~') | ('[', '[')
    )
}

fn pair_exists_after(chars: &[char], start: usize, a: char, b: char) -> bool {
    chars
        .get(start..)
        .is_some_and(|tail| tail.windows(2).any(|pair| pair[0] == a && pair[1] == b))
}

/// Try to match `<tagname:value>` at the start of `s` (case-insensitive).
/// Returns `Some((value, consumed_bytes))` on success.
fn parse_open_tag_with_value<'a>(s: &'a str, tagname: &str) -> Option<(&'a str, usize)> {
    let lower = s.to_ascii_lowercase();
    let prefix = format!("<{}:", tagname);
    if !lower.starts_with(&prefix) {
        return None;
    }
    let value_start = prefix.len();
    let close = s[value_start..].find('>')?;
    let value = &s[value_start..value_start + close];
    let consumed = value_start + close + 1; // includes '>'
    Some((value, consumed))
}

fn decode_numeric_character_reference(s: &str) -> Option<(char, usize)> {
    if !s.starts_with("&#") {
        return None;
    }

    let after_prefix = &s[2..];
    let (radix, digits_start) = if after_prefix.starts_with('x') || after_prefix.starts_with('X') {
        (16, 3)
    } else {
        (10, 2)
    };
    let close = s[digits_start..].find(';')? + digits_start;
    let digits = &s[digits_start..close];
    if digits.is_empty() || !digits.chars().all(|ch| ch.is_digit(radix) && ch.is_ascii()) {
        return None;
    }

    let value = u32::from_str_radix(digits, radix).ok()?;
    let decoded = char::from_u32(value)?;
    Some((decoded, close + 1))
}

fn decode_codepoint_tag(s: &str) -> Option<(char, usize)> {
    if !s
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("<u+"))
    {
        return None;
    }

    let close = s[3..].find('>')? + 3;
    let digits = &s[3..close];
    if !valid_codepoint_hex(digits) {
        return None;
    }

    let value = u32::from_str_radix(digits, 16).ok()?;
    let decoded = char::from_u32(value)?;
    Some((decoded, close + 1))
}

fn decode_emoji_tag(s: &str) -> Option<(String, usize)> {
    if !s.starts_with("<:") {
        return None;
    }

    let token_end = s[2..].find(":>")? + 2;
    let token = &s[2..token_end];
    if token.is_empty() {
        return None;
    }

    let decoded = decode_emoji_token(token)?;
    Some((decoded, token_end + 2))
}

fn decode_colored_emoji_tag(s: &str) -> Option<(String, usize)> {
    if !s.starts_with("<#") {
        return None;
    }

    let close = s.find(":>")?;
    let inner = &s[2..close];
    let token_start = inner.rfind(':')? + 1;
    let token = &inner[token_start..];
    if token.is_empty() {
        return None;
    }

    let decoded = decode_emoji_token(token)?;
    Some((decoded, close + 2))
}

fn decode_emoji_token(token: &str) -> Option<String> {
    if valid_codepoint_hex(token) {
        let value = u32::from_str_radix(token, 16).ok()?;
        return char::from_u32(value).map(|ch| ch.to_string());
    }

    let normalized = token.trim().to_ascii_lowercase().replace(['-', ' '], "_");
    let mapped = match normalized.as_str() {
        "calendar" => "📅",
        "check" | "white_check_mark" => "✅",
        "grin" | "grinning" | "smile" | "smiley" => "😀",
        "heart" | "red_heart" => "❤",
        "innocent" => "😇",
        "star" => "⭐",
        "sunglasses" => "😎",
        "sun" | "sunny" => "☀",
        "warning" => "⚠",
        _ if normalized
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '+' || ch == '-')
            && !normalized.is_empty() =>
        {
            return Some(format!(":{normalized}:"));
        }
        _ => return None,
    };
    Some(mapped.to_string())
}

fn creole_hash_color(color: &str) -> String {
    if color.chars().all(|ch| ch.is_ascii_hexdigit()) && matches!(color.len(), 3 | 6 | 8) {
        format!("#{color}")
    } else {
        color.to_string()
    }
}

fn valid_codepoint_hex(s: &str) -> bool {
    (1..=6).contains(&s.len()) && s.chars().all(|ch| ch.is_ascii_hexdigit())
}

/// Match a literal prefix + suffix pattern (e.g. `<&` ... `>`).
fn strip_tag_prefix<'a>(s: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    let lower_prefix = s
        .chars()
        .take(prefix.len())
        .collect::<String>()
        .to_ascii_lowercase();
    if lower_prefix != prefix.to_ascii_lowercase() {
        return None;
    }
    let inner_start = prefix.len();
    let close = s[inner_start..].find(suffix)?;
    Some(&s[inner_start..inner_start + close])
}

fn escape_xml(s: &str) -> String {
    let decoded = decode_unicode_escapes(s);
    let mut out = String::with_capacity(decoded.len());
    for ch in decoded.chars() {
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

fn escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("&quot;"),
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn single_line(text: &str) -> CreoleLine {
        let lines = tokenize_creole(text);
        assert_eq!(lines.len(), 1);
        lines.into_iter().next().unwrap()
    }

    #[test]
    fn plain_text_is_single_span() {
        let line = single_line("hello world");
        assert_eq!(line.len(), 1);
        assert_eq!(line[0].text, "hello world");
        assert!(!line[0].bold);
    }

    #[test]
    fn bold_toggles_state() {
        let line = single_line("**bold** plain");
        assert_eq!(line[0].text, "bold");
        assert!(line[0].bold);
        assert_eq!(line[1].text, " plain");
        assert!(!line[1].bold);
    }

    #[test]
    fn italic_toggles_state() {
        let line = single_line("//italic// text");
        assert!(line[0].italic);
        assert!(!line[1].italic);
    }

    #[test]
    fn mono_toggles_state() {
        let line = single_line("\"\"code\"\" text");
        assert!(line[0].mono);
        assert!(!line[1].mono);
    }

    #[test]
    fn underline_toggles_state() {
        let line = single_line("__ul__ text");
        assert!(line[0].underline);
        assert!(!line[1].underline);
    }

    #[test]
    fn strike_toggles_state() {
        let line = single_line("--strike-- text");
        assert!(line[0].strike);
        assert!(!line[1].strike);
    }

    #[test]
    fn link_with_label() {
        let line = single_line("[[https://example.com click me]]");
        assert_eq!(line[0].link.as_deref(), Some("https://example.com"));
        assert_eq!(line[0].text, "click me");
        assert!(line[0].underline);
    }

    #[test]
    fn link_without_label_uses_url() {
        let line = single_line("[[https://example.com]]");
        assert_eq!(line[0].link.as_deref(), Some("https://example.com"));
        assert_eq!(line[0].text, "https://example.com");
    }

    #[test]
    fn color_tag() {
        let line = single_line("<color:red>text</color>");
        assert_eq!(line[0].color.as_deref(), Some("red"));
        assert_eq!(line[0].text, "text");
    }

    #[test]
    fn hex_color_tag() {
        let line = single_line("<color:#FF0000>red</color>");
        assert_eq!(line[0].color.as_deref(), Some("#FF0000"));
    }

    #[test]
    fn size_tag() {
        let line = single_line("<size:18>big</size>");
        assert_eq!(line[0].size, Some(18));
        assert_eq!(line[0].text, "big");
    }

    #[test]
    fn html_bold_tag() {
        let line = single_line("<b>bold</b> plain");
        assert!(line[0].bold);
        assert!(!line[1].bold);
    }

    #[test]
    fn html_italic_tag() {
        let line = single_line("<i>italic</i> plain");
        assert!(line[0].italic);
        assert!(!line[1].italic);
    }

    #[test]
    fn html_underline_tag() {
        let line = single_line("<u>ul</u> plain");
        assert!(line[0].underline);
        assert!(!line[1].underline);
    }

    #[test]
    fn newline_splits_into_multiple_lines() {
        let lines = tokenize_creole("line1\nline2");
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0][0].text, "line1");
        assert_eq!(lines[1][0].text, "line2");
    }

    #[test]
    fn backslash_n_splits_into_multiple_lines() {
        // \n in the source string (the two characters \ and n)
        let lines = tokenize_creole(r"line1\nline2");
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn icon_placeholder() {
        let line = single_line("<&home>");
        assert_eq!(line[0].text, "[home]");
    }

    #[test]
    fn mixed_bold_italic_nesting() {
        let line = single_line("**bold //bi//** only bold");
        // "bold //bi//" is bold; "bi" is bold+italic; " only bold" is plain
        // But since we parse sequentially, order matters.
        let bold_span = line.iter().find(|s| s.text == "bold ");
        assert!(bold_span.is_some_and(|s| s.bold));
    }

    #[test]
    fn render_bold_span() {
        let lines = tokenize_creole("**hi**");
        let out = render_creole_line_to_tspans(&lines[0], 0, "black");
        assert!(out.contains("font-weight=\"bold\""));
        assert!(out.contains(">hi<"));
    }

    #[test]
    fn render_link_span() {
        let lines = tokenize_creole("[[https://x.com go]]");
        let out = render_creole_line_to_tspans(&lines[0], 0, "black");
        assert!(out.contains("xlink:href=\"https://x.com\""));
        assert!(out.contains("fill=\"blue\""));
        assert!(out.contains(">go<"));
    }

    #[test]
    fn render_multi_line_tspans() {
        let lines = tokenize_creole("line1\nline2");
        let out = render_creole_to_svg_tspans(&lines, 10, "black");
        assert!(out.contains("x=\"10\""));
        assert!(out.contains("dy=\"1.2em\""));
    }

    #[test]
    fn decodes_decimal_and_hex_numeric_character_references() {
        assert_eq!(
            decode_unicode_escapes("decimal &#8734; hex &#x221E; upper &#X1F600;"),
            "decimal ∞ hex ∞ upper 😀"
        );
    }

    #[test]
    fn decodes_u_plus_codepoint_tags() {
        assert_eq!(
            decode_unicode_escapes("This is <U+221E> and <u+1F527>"),
            "This is ∞ and 🔧"
        );
    }

    #[test]
    fn decodes_small_emoji_map_and_deterministic_fallback() {
        assert_eq!(
            decode_unicode_escapes("<:calendar:> <:1f600:> <:not_in_small_map:> <#green:sunny:>"),
            "📅 😀 :not_in_small_map: ☀"
        );
    }

    #[test]
    fn leaves_invalid_unicode_escapes_literal() {
        let text = "bad &#xZZ; missing &#9731 no-code <U+110000> no-end <U+221E emoji <::>";
        assert_eq!(decode_unicode_escapes(text), text);
    }

    #[test]
    fn rendered_creole_decodes_escapes_and_removes_escape_text() {
        let lines = tokenize_creole("snow &#9731; infinity <U+221E> <:calendar:>");
        let out = render_creole_line_to_tspans(&lines[0], 0, "black");

        assert!(out.contains("snow ☃ infinity ∞ 📅"));
        assert!(!out.contains("&#9731;"));
        assert!(!out.contains("&lt;U+221E&gt;"));
        assert!(!out.contains("&lt;:calendar:&gt;"));
    }

    #[test]
    fn tilde_escapes_creole_markers() {
        let line = single_line("~**literal** and ~[[x]]");
        assert_eq!(line.len(), 1);
        assert_eq!(line[0].text, "**literal** and [[x]]");
        assert!(!line[0].bold);
        assert!(line[0].link.is_none());
    }

    #[test]
    fn wave_underline_creole_and_html_tag_render() {
        let creole = single_line("~~wave~~");
        assert!(creole[0].wave);

        let html = single_line("<w:red>wave</w>");
        assert!(html[0].wave);
        assert_eq!(html[0].decoration_color.as_deref(), Some("red"));

        let out = render_creole_line_to_tspans(&html, 0, "black");
        assert!(out.contains("text-decoration-style=\"wavy\""));
        assert!(out.contains("text-decoration-color=\"red\""));
    }

    #[test]
    fn link_tooltip_renders_svg_title() {
        let line = single_line("[[https://example.com{Open docs} docs]]");
        assert_eq!(line[0].link.as_deref(), Some("https://example.com"));
        assert_eq!(line[0].link_tooltip.as_deref(), Some("Open docs"));
        assert_eq!(line[0].text, "docs");

        let out = render_creole_line_to_tspans(&line, 0, "black");
        assert!(out.contains("<title>Open docs</title>"));
    }

    #[test]
    fn headings_become_bold_sized_lines() {
        let lines = tokenize_creole("= Main title =\n=== Minor");
        assert_eq!(lines[0][0].text, "Main title");
        assert!(lines[0][0].bold);
        assert_eq!(lines[0][0].size, Some(24));
        assert_eq!(lines[1][0].size, Some(16));
    }

    #[test]
    fn list_lines_add_indented_prefixes_without_triggering_bold() {
        let lines = tokenize_creole("* Bullet\n** Nested\n# Numbered\n## Nested number");
        assert_eq!(lines[0][0].text, "- ");
        assert_eq!(lines[0][1].text, "Bullet");
        assert_eq!(lines[1][0].text, "  - ");
        assert_eq!(lines[2][0].text, "1. ");
        assert_eq!(lines[3][0].text, "  1. ");
        assert!(!lines[1][1].bold);
    }

    #[test]
    fn horizontal_rule_lines_render_as_rule_text() {
        let lines = tokenize_creole("----\n.. Section ..");
        assert_eq!(lines[0][0].text, "------------------------");
        assert!(lines[0][0].mono);
        assert_eq!(lines[1][0].text, "---------- Section ----------");
    }

    #[test]
    fn code_tag_is_verbatim_monospace() {
        let line = single_line("<code>**not bold** & raw</code>");
        assert_eq!(line[0].text, "**not bold** & raw");
        assert!(line[0].mono);
        assert!(!line[0].bold);
    }

    #[test]
    fn table_lines_mark_headers_and_cell_backgrounds() {
        let line = single_line("|= Name |<#FF8080> Value |");
        assert_eq!(line[0].text, "Name");
        assert!(line[0].bold);
        assert!(line[0].mono);
        assert_eq!(line[2].text, "Value");
        assert_eq!(line[2].background.as_deref(), Some("#FF8080"));
    }

    #[test]
    fn row_background_applies_to_table_cells() {
        let line = single_line("<#yellow>| a | b |");
        assert_eq!(line[0].background.as_deref(), Some("yellow"));
        assert_eq!(line[2].background.as_deref(), Some("yellow"));
    }

    #[test]
    fn tree_lines_use_text_tree_prefix() {
        let line = single_line("  |_ child");
        assert_eq!(line[0].text, "  `- ");
        assert!(line[0].mono);
        assert_eq!(line[1].text, "child");
    }

    #[test]
    fn remaining_html_tags_set_span_state() {
        let strike = single_line("<s:green>gone</s>");
        assert!(strike[0].strike);
        assert_eq!(strike[0].decoration_color.as_deref(), Some("green"));

        let plain = single_line("<b><plain>**literal**</plain></b>");
        assert_eq!(plain[0].text, "**literal**");
        assert!(!plain[0].bold);

        let back = single_line("<back:#ffeeaa>highlight</back>");
        assert_eq!(back[0].background.as_deref(), Some("#ffeeaa"));

        let font = single_line("<font:serif>face</font>");
        assert_eq!(font[0].font.as_deref(), Some("serif"));

        let sub = single_line("H<sub>2</sub>O");
        assert_eq!(sub[1].baseline_shift.as_deref(), Some("sub"));

        let sup = single_line("x<sup>2</sup>");
        assert_eq!(sup[1].baseline_shift.as_deref(), Some("super"));
    }

    #[test]
    fn render_remaining_html_tag_attributes() {
        let lines =
            tokenize_creole("<font:serif><back:yellow><sub>x</sub></back></font> <s>gone</s>");
        let out = render_creole_line_to_tspans(&lines[0], 0, "black");
        assert!(out.contains("font-family=\"serif\""));
        assert!(out.contains("data-creole-back=\"yellow\""));
        assert!(out.contains("baseline-shift=\"sub\""));
        assert!(out.contains("line-through"));
    }
}
