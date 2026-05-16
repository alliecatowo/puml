/// Creole inline text formatting for PlantUML labels.
///
/// Supports: **bold**, //italic//, ""mono"", __underline__, --strikethrough--,
/// [[url label]] hyperlinks, <color:X>text</color>, <size:N>text</size>,
/// <b>, <i>, <u> HTML tags, \n line breaks, and <&icon> placeholders.

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CreoleSpan {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub mono: bool,
    pub underline: bool,
    pub strike: bool,
    pub color: Option<String>,
    pub size: Option<u32>,
    pub link: Option<String>,
}

/// A line is a list of spans. Multiple lines come from \n splits.
pub type CreoleLine = Vec<CreoleSpan>;

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
        all_lines.push(parse_inline(raw_line));
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

        let mut text_decorations: Vec<&str> = Vec::new();
        if span.underline || span.link.is_some() {
            text_decorations.push("underline");
        }
        if span.strike {
            text_decorations.push("line-through");
        }
        if !text_decorations.is_empty() {
            style_parts.push(format!("text-decoration=\"{}\"", text_decorations.join(" ")));
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
            let rest_lower: String = rest.chars().take(7).collect::<String>().to_ascii_lowercase();
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
    color: Option<String>,
    size: Option<u32>,
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
                spans.push(CreoleSpan {
                    text: buf.clone(),
                    bold: state.bold,
                    italic: state.italic,
                    mono: state.mono,
                    underline: state.underline,
                    strike: state.strike,
                    color: state.color.clone(),
                    size: state.size,
                    link: None,
                });
                buf.clear();
            }
        };
    }

    while i < len {
        // --- **bold** ---
        if chars[i] == '*' && i + 1 < len && chars[i + 1] == '*' {
            flush!();
            state.bold = !state.bold;
            i += 2;
            continue;
        }

        // --- //italic// ---
        if chars[i] == '/' && i + 1 < len && chars[i + 1] == '/' {
            flush!();
            state.italic = !state.italic;
            i += 2;
            continue;
        }

        // --- ""mono"" ---
        if chars[i] == '"' && i + 1 < len && chars[i + 1] == '"' {
            flush!();
            state.mono = !state.mono;
            i += 2;
            continue;
        }

        // --- __underline__ ---
        if chars[i] == '_' && i + 1 < len && chars[i + 1] == '_' {
            flush!();
            state.underline = !state.underline;
            i += 2;
            continue;
        }

        // --- --strike-- ---
        if chars[i] == '-' && i + 1 < len && chars[i + 1] == '-' {
            flush!();
            state.strike = !state.strike;
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
                let (url, label) = if let Some(sp) = inner.find(' ') {
                    (inner[..sp].to_string(), inner[sp + 1..].to_string())
                } else {
                    (inner.clone(), inner)
                };
                spans.push(CreoleSpan {
                    text: label,
                    bold: state.bold,
                    italic: state.italic,
                    mono: state.mono,
                    underline: true,
                    strike: state.strike,
                    color: Some("blue".to_string()),
                    size: state.size,
                    link: Some(url),
                });
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
            let rest: String = chars[i..].iter().collect();

            // <&icon>  — require non-empty icon name
            if let Some(inner) = strip_tag_prefix(&rest, "<&", ">").filter(|s| !s.is_empty()) {
                flush!();
                spans.push(CreoleSpan {
                    text: format!("[{}]", inner.trim()),
                    bold: state.bold,
                    italic: state.italic,
                    mono: state.mono,
                    underline: state.underline,
                    strike: state.strike,
                    color: state.color.clone(),
                    size: state.size,
                    link: None,
                });
                i += 2 + inner.len() + 1;
                continue;
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

            // <u>
            if rest.to_ascii_lowercase().starts_with("<u>") {
                flush!();
                state.underline = true;
                i += 3;
                continue;
            }

            // </u>
            if rest.to_ascii_lowercase().starts_with("</u>") {
                flush!();
                state.underline = false;
                i += 4;
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
        assert!(bold_span.map_or(false, |s| s.bold));
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
}
