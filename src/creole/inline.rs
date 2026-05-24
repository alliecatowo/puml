use super::{CreoleLine, CreoleSpan};

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

pub(super) fn parse_inline(text: &str) -> CreoleLine {
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
