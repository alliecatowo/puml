use super::*;

/// Parse a swimlane header line of the form `|...|`.
///
/// Supported variants (PlantUML reference §6.13 / §6.14):
/// - `|Lane|`                     — plain swimlane
/// - `|#color|Lane|`              — colored swimlane
/// - `|#color|Lane|`              — color + name combo
/// - `|= Lane|`                   — **bold** header (display only, `=` stripped from name)
/// - `|<<role>>Lane|`             — stereotype; `<<role>>` stripped from identifier,
///   stored as `lane_stereotype=role` metadata in the label
///
/// The returned label encodes both the clean lane identifier (used for node
/// routing) and any display modifiers, using `\x1f`-delimited fields.
pub(crate) fn parse_activity_swimlane(line: &str) -> Option<String> {
    if !line.starts_with('|') || !line.ends_with('|') {
        return None;
    }
    let mut color: Option<&str> = None;
    let mut bold = false;
    let mut stereotype: Option<String> = None;

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

    let raw_name = parts.last().copied()?;
    // Strip leading `=` bold modifier
    let raw_name = if let Some(rest) = raw_name.strip_prefix('=') {
        bold = true;
        rest.trim()
    } else {
        raw_name
    };
    // Strip leading `<<stereotype>>` prefix
    let clean_name = if raw_name.starts_with("<<") {
        if let Some(close) = raw_name.find(">>") {
            let stereo = &raw_name[2..close];
            if !stereo.is_empty() {
                stereotype = Some(stereo.to_string());
            }
            raw_name[close + 2..].trim()
        } else {
            raw_name
        }
    } else {
        raw_name
    };

    // Build the style-annotated label.  The clean_name is the lane identifier;
    // bold and stereotype are display-only hints encoded as \x1f markers.
    let mut style_parts: Vec<String> = Vec::new();
    if bold {
        style_parts.push("\x1fswim:bold\x1f".to_string());
    }
    if let Some(ref stereo) = stereotype {
        style_parts.push(format!("\x1fswim:stereotype={stereo}\x1f"));
    }
    let annotated_name = if style_parts.is_empty() {
        clean_name.to_string()
    } else {
        format!("{}{}", style_parts.concat(), clean_name)
    };
    Some(activity_style_label(annotated_name, color))
}

pub(crate) fn parse_activity_colored_action(line: &str) -> Option<(String, Option<String>)> {
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
    (!label.is_empty()).then(|| (label, Some(normalize_activity_color_token(_color))))
}

pub(crate) fn parse_activity_colored_connector(line: &str) -> Option<(String, Option<String>)> {
    let rest = line.strip_prefix('#')?;
    let (color, body) = rest.split_once(':')?;
    let label = parse_activity_connector(body.trim())?;
    Some((label, Some(normalize_activity_color_token(color))))
}

pub(crate) fn parse_activity_connector(line: &str) -> Option<String> {
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

pub(crate) fn parse_activity_partition_like(line: &str) -> Option<(String, Option<String>)> {
    let (keyword, rest) = ["partition ", "group ", "package ", "rectangle ", "card "]
        .iter()
        .find_map(|prefix| line.strip_prefix(prefix).map(|rest| (*prefix, rest)))?;
    let keyword_name = keyword.trim();
    let is_block_scope = line.trim_end().ends_with('{') || keyword_name == "group";
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
        Some((keyword_name.to_string(), color))
    } else if is_block_scope {
        Some((activity_partition_block_label(label), color))
    } else {
        Some((label, color))
    }
}

/// Extract swimlane display metadata embedded in a lane label by
/// [`parse_activity_swimlane`].
///
/// Returns `(clean_name, bold, stereotype)` where `clean_name` has all `\x1f`
/// swim markers stripped, `bold` is true when `|= Name|` was parsed, and
/// `stereotype` carries the `<<role>>` text if present.
#[allow(dead_code)]
pub(crate) fn extract_swimlane_display_meta(label: &str) -> (&str, bool, Option<&str>) {
    let mut rest = label;
    let mut bold = false;
    let mut stereotype: Option<&str> = None;
    loop {
        if let Some(after) = rest.strip_prefix("\x1fswim:bold\x1f") {
            bold = true;
            rest = after;
        } else if let Some(after) = rest.strip_prefix("\x1fswim:stereotype=") {
            if let Some(end) = after.find('\x1f') {
                stereotype = Some(&after[..end]);
                rest = &after[end + 1..];
            } else {
                break;
            }
        } else {
            break;
        }
    }
    (rest, bold, stereotype)
}

pub(crate) fn parse_activity_arrow_directive(line: &str) -> Option<String> {
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
        for part in style
            .split([',', ';'])
            .map(str::trim)
            .filter(|part| !part.is_empty())
        {
            match part {
                "dashed" | "dotted" | "line.dashed" | "line.dotted" => dashed = true,
                "hidden" => hidden = true,
                "bold" | "line.bold" | "line.thick" => bold = true,
                _ if part.starts_with('#') => color = Some(part),
                _ if part.starts_with("thickness=") || part.starts_with("line.thickness=") => {
                    bold = true;
                }
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
    let tail = tail.trim_start_matches(':').trim();
    if !tail.is_empty() {
        parts.push(format!("label:{tail}"));
    }
    Some(parts.join("\x1f"))
}

/// Parse an activity action body (after the leading `:`) for suffix fill-color.
///
/// PlantUML allows `:action text; #color` where the `#color` token follows the
/// terminator character.  This function returns the body-before-color and the
/// color token as an owned `String`, allowing the caller to apply fill styling.
///
/// Handles both the plain-semicolon form (`;  #color`) and SDL terminators
/// (`|  #color`, `>  #color`, etc.).
///
/// Returns `(body_with_terminator, Some(color))` if a suffix color is
/// present, or `(rest_unchanged, None)` when no color suffix is found.
pub(crate) fn strip_activity_action_suffix_color(rest: &str) -> (&str, Option<String>) {
    // Locate the last `#` that could be the start of a color token.
    let Some(hash_pos) = rest.rfind('#') else {
        return (rest, None);
    };
    let candidate = &rest[hash_pos..];
    // A color token must start with `#` followed by at least one alphanumeric character.
    let after_hash = &candidate[1..];
    if after_hash.is_empty()
        || !after_hash
            .chars()
            .next()
            .map_or(false, |c| c.is_ascii_alphanumeric())
    {
        return (rest, None);
    }
    // The `#` must be preceded by whitespace (separator between terminator and color).
    // This guards against `#` embedded mid-label (e.g. `:#item;` remains untouched).
    if hash_pos > 0 && !rest[..hash_pos].ends_with(|c: char| c.is_ascii_whitespace()) {
        return (rest, None);
    }
    let color = normalize_activity_color_token(candidate);
    (&rest[..hash_pos], Some(color))
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
pub(crate) fn parse_activity_action_terminator(rest: &str) -> (&str, Option<&'static str>) {
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

pub(crate) fn activity_style_label(label: impl Into<String>, fill_color: Option<&str>) -> String {
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

pub(crate) fn activity_partition_block_label(label: String) -> String {
    format!("\x1factivity:partition:block\x1f{label}")
}

pub(crate) fn normalize_activity_color_token(token: &str) -> String {
    let raw = token.trim().trim_start_matches('#');
    let is_hex = matches!(raw.len(), 3 | 4 | 6 | 8) && raw.chars().all(|c| c.is_ascii_hexdigit());
    if is_hex {
        format!("#{raw}")
    } else {
        raw.to_string()
    }
}

pub(crate) fn strip_wrapping_quotes(input: &str) -> &str {
    let trimmed = input.trim();
    trimmed
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or(trimmed)
}

pub(crate) fn activity_step_statement(
    kind: ActivityStepKind,
    label: Option<String>,
) -> StatementKind {
    StatementKind::ActivityStep(ActivityStep { kind, label })
}
