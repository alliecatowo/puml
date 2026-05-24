fn parse_activity_swimlane(line: &str) -> Option<String> {
    if !line.starts_with('|') || !line.ends_with('|') {
        return None;
    }
    let mut color: Option<&str> = None;
    let parts: Vec<&str> = line
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .filter(|part| {
            if part.is_empty() {
                return false;
            }
            if part.starts_with('#') {
                color = Some(part);
                return false;
            }
            true
        })
        .collect();
    parts.last().map(|part| activity_style_label(*part, color))
}

fn parse_activity_colored_action(line: &str) -> Option<(String, Option<String>)> {
    let rest = line.strip_prefix('#')?;
    let (_color, body) = rest.split_once(':')?;
    if body.trim_start().starts_with('(') {
        return None;
    }
    let (body_text, sdl_shape) = parse_activity_action_terminator(body.trim());
    let label = if let Some(shape) = sdl_shape {
        if body_text.is_empty() {
            return None;
        }
        format!("\x1fsdl:{shape}\x1f{body_text}")
    } else {
        body_text.to_string()
    };
    (!label.is_empty()).then(|| {
        (
            label,
            Some(normalize_activity_color_token(_color)),
        )
    })
}

fn parse_activity_colored_connector(line: &str) -> Option<(String, Option<String>)> {
    let rest = line.strip_prefix('#')?;
    let (color, body) = rest.split_once(':')?;
    let label = parse_activity_connector(body.trim())?;
    Some((label, Some(normalize_activity_color_token(color))))
}

fn parse_activity_connector(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches(';').trim();
    let rest = trimmed.strip_prefix('(')?;
    let close = rest.find(')')?;
    let id = rest[..close].trim();
    if id.is_empty() || id.len() > 8 {
        return None;
    }
    let suffix = rest[close + 1..].trim();
    Some(if suffix.is_empty() {
        format!("({id})")
    } else {
        format!("({id}) {suffix}")
    })
}

fn parse_activity_partition_like(line: &str) -> Option<(String, Option<String>)> {
    let (keyword, rest) = ["partition ", "group ", "package ", "rectangle ", "card "]
        .iter()
        .find_map(|prefix| line.strip_prefix(prefix).map(|rest| (*prefix, rest)))?;
    let raw = rest.trim().trim_end_matches('{').trim();
    let mut color: Option<String> = None;
    let clean: Vec<&str> = raw
        .split_whitespace()
        .filter(|tok| {
            if tok.starts_with('#') {
                color = Some(normalize_activity_color_token(tok));
                return false;
            }
            true
        })
        .collect();
    let label = if clean.is_empty() {
        raw.to_string()
    } else {
        clean.join(" ")
    };
    let label = strip_wrapping_quotes(&label).to_string();
    if label.is_empty() {
        Some((keyword.trim().to_string(), color))
    } else {
        Some((label, color))
    }
}

fn parse_activity_arrow_directive(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_end_matches(';').trim();
    if !trimmed.starts_with('-') {
        return None;
    }
    let arrow_end = trimmed.find("->")? + 2;
    let arrow_token = &trimmed[..arrow_end];
    if !arrow_token.ends_with("->") {
        return None;
    }
    let tail = trimmed[arrow_end..].trim();
    let mut color: Option<&str> = None;
    let mut dashed = false;
    let mut hidden = false;
    let mut bold = false;
    if let Some(style) = arrow_token
        .strip_prefix("-[")
        .and_then(|value| value.strip_suffix("]->"))
    {
        for part in style.split(',').map(str::trim).filter(|part| !part.is_empty()) {
            match part {
                "dashed" | "dotted" => dashed = true,
                "hidden" => hidden = true,
                "bold" => bold = true,
                _ if part.starts_with('#') => color = Some(part),
                _ => {}
            }
        }
    } else if arrow_token != "->" && arrow_token != "-->" {
        return None;
    }
    let mut parts = vec!["\x1factivity:arrow".to_string()];
    if let Some(color) = color {
        parts.push(format!("color:{}", normalize_activity_color_token(color)));
    }
    if dashed {
        parts.push("dashed:1".to_string());
    }
    if hidden {
        parts.push("hidden:1".to_string());
    }
    if bold {
        parts.push("bold:1".to_string());
    }
    if !tail.is_empty() {
        parts.push(format!("label:{tail}"));
    }
    Some(parts.join("\x1f"))
}

/// Parse an activity action body (after the leading `:`), extracting the SDL
/// terminator character if present.
///
/// Returns `(body_text, Some("sdl_shape"))` or `(body_text, None)` for plain `;`.
///
/// PlantUML SDL terminators (final character before optional whitespace):
///   `;`  → plain rounded rectangle (default, no marker)
///   `|`  → horizontal bar / procedure
///   `<`  → left-pointing chevron (receive)
///   `>`  → right-pointing chevron (send)
///   `/`  → parallelogram slanting right (input)
///   `\`  → parallelogram slanting left (output)
///   `]`  → right bracket / condition
///   `}`  → closing brace / return
fn parse_activity_action_terminator(rest: &str) -> (&str, Option<&'static str>) {
    let raw = rest.trim_end();
    let (stripped, terminator) = match raw.as_bytes().last() {
        Some(b';') => (&raw[..raw.len() - 1], None),
        Some(b'|') => (&raw[..raw.len() - 1], Some("bar")),
        Some(b'<') => (&raw[..raw.len() - 1], Some("receive")),
        Some(b'>') => (&raw[..raw.len() - 1], Some("send")),
        Some(b'/') => (&raw[..raw.len() - 1], Some("input")),
        Some(b'\\') => (&raw[..raw.len() - 1], Some("output")),
        Some(b']') => (&raw[..raw.len() - 1], Some("bracket")),
        Some(b'}') => (&raw[..raw.len() - 1], Some("brace")),
        _ => (raw, None),
    };
    (stripped.trim(), terminator)
}

fn activity_style_label(label: impl Into<String>, fill_color: Option<&str>) -> String {
    let label = label.into();
    match fill_color {
        Some(color) if !color.trim().is_empty() => {
            format!(
                "\x1fstyle:fill:{}\x1f{}",
                normalize_activity_color_token(color),
                label
            )
        }
        _ => label,
    }
}

fn normalize_activity_color_token(token: &str) -> String {
    let raw = token.trim().trim_start_matches('#');
    let is_hex = matches!(raw.len(), 3 | 4 | 6 | 8) && raw.chars().all(|c| c.is_ascii_hexdigit());
    if is_hex {
        format!("#{raw}")
    } else {
        raw.to_string()
    }
}

fn strip_wrapping_quotes(input: &str) -> &str {
    let trimmed = input.trim();
    trimmed
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(trimmed)
}

fn activity_step_statement(kind: ActivityStepKind, label: Option<String>) -> StatementKind {
    StatementKind::ActivityStep(ActivityStep { kind, label })
}
