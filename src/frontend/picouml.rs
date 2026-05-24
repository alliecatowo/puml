use super::{FrontendResult, SourceMap};
use crate::Diagnostic;

pub(crate) fn adapt(source: &str) -> Result<FrontendResult, Diagnostic> {
    // First pass: strip block comments `[/* ... */]`.
    let original = source;
    let source = picouml_strip_block_comments(source);

    let mut out = String::new();
    let mut saw_picouml_markers = false;
    let mut saw_uml_markers = false;
    let mut in_group_block = false;

    for raw_line in source.lines() {
        let trimmed = raw_line.trim();

        if matches_prefixed_uml_marker(trimmed, "@startpicouml") {
            saw_picouml_markers = true;
            let converted = replace_prefixed_marker(raw_line, "@startpicouml", "@startuml");
            out.push_str(&converted);
            out.push('\n');
            continue;
        }
        if matches_prefixed_uml_marker(trimmed, "@endpicouml") {
            saw_picouml_markers = true;
            let converted = replace_prefixed_marker(raw_line, "@endpicouml", "@enduml");
            out.push_str(&converted);
            out.push('\n');
            continue;
        }
        if matches_prefixed_uml_marker(trimmed, "@startuml")
            || matches_prefixed_uml_marker(trimmed, "@enduml")
        {
            saw_uml_markers = true;
        }

        // Translate PicoUML-specific constructs.
        if let Some(converted) = adapt_picouml_line(trimmed, &mut in_group_block) {
            out.push_str(&converted);
            out.push('\n');
            continue;
        }

        out.push_str(raw_line);
        out.push('\n');
    }

    if saw_picouml_markers && saw_uml_markers {
        return Err(Diagnostic::error_code(
            "E_PICOUML_MARKER_MIXED",
            "picouml frontend does not allow mixing `@startpicouml/@endpicouml` with `@startuml/@enduml` markers",
        ));
    }

    let source_map = SourceMap::line_map(original, &out);
    Ok(FrontendResult::new(out, source_map))
}

/// Strip PicoUML block comments of the form `[/* ... */]` (possibly multiline).
fn picouml_strip_block_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        // Look for `[/*`
        if i + 2 < bytes.len() && bytes[i] == b'[' && bytes[i + 1] == b'/' && bytes[i + 2] == b'*' {
            // Find closing `*/]`
            let mut j = i + 3;
            while j + 2 < bytes.len() {
                if bytes[j] == b'*' && bytes[j + 1] == b'/' && bytes[j + 2] == b']' {
                    j += 3;
                    break;
                }
                j += 1;
            }
            i = j;
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Adapt a single PicoUML content line to its PlantUML equivalent.
/// Returns `Some(converted)` if the line needed adaptation, `None` to pass through unchanged.
fn adapt_picouml_line(line: &str, in_group_block: &mut bool) -> Option<String> {
    // `=>` sync-call arrow: `A => B : msg`  →  `A -> B : msg <<sync>>`
    // `~>` async arrow:     `A ~> B : msg`  →  `A -> B : msg <<async>>`
    for (pico_arrow, plantuml_arrow, stereotype) in
        [("=>", "->", "<<sync>>"), ("~>", "->", "<<async>>")]
    {
        if let Some(converted) = adapt_picouml_arrow(line, pico_arrow, plantuml_arrow, stereotype) {
            return Some(converted);
        }
    }
    // Reverse aliases keep PicoUML's compact call notation symmetric:
    // `A <= B : msg` means `B -> A : msg <<sync>>`.
    // `A <~ B : msg` means `B -> A : msg <<async>>`.
    for (pico_arrow, plantuml_arrow, stereotype) in
        [("<=", "->", "<<sync>>"), ("<~", "->", "<<async>>")]
    {
        if let Some(converted) =
            adapt_picouml_reverse_arrow(line, pico_arrow, plantuml_arrow, stereotype)
        {
            return Some(converted);
        }
    }

    // `note left A : text`  →  `note left of A : text`
    // `note right A : text`  →  `note right of A : text`
    if let Some(converted) = adapt_picouml_note(line) {
        return Some(converted);
    }

    // `group X / Y`  →  `group X\nY` (the label part after `/` is extra context)
    // `end` inside such a block is already valid PlantUML; we close our tracking.
    if let Some(converted) = adapt_picouml_group(line, in_group_block) {
        return Some(converted);
    }

    None
}

/// Convert PicoUML custom arrow syntax to PlantUML with stereotype suffix.
fn adapt_picouml_arrow(
    line: &str,
    pico_arrow: &str,
    plantuml_arrow: &str,
    stereotype: &str,
) -> Option<String> {
    // We require ` : ` to distinguish an arrow with label.  The arrow may appear with or without label.
    let arrow_idx = line.find(pico_arrow)?;
    let before = &line[..arrow_idx];
    let after = &line[arrow_idx + pico_arrow.len()..];

    // Make sure this isn't already handled by the base `->` path.
    // The PicoUML arrows are `=>` and `~>` — never appear as vanilla PlantUML.
    // Validate rough arrow-line shape: `A => B` or `A => B : msg`
    let from = before.trim();
    if from.is_empty() {
        return None;
    }

    let (to, label) = if let Some((to_part, msg)) = after.split_once(':') {
        (to_part.trim(), Some(msg.trim()))
    } else {
        (after.trim(), None)
    };

    if to.is_empty() {
        return None;
    }

    Some(if let Some(lbl) = label {
        format!("{from} {plantuml_arrow} {to} : {lbl} {stereotype}")
    } else {
        format!("{from} {plantuml_arrow} {to} : {stereotype}")
    })
}

/// Convert reverse PicoUML custom arrow syntax to PlantUML with stereotype suffix.
fn adapt_picouml_reverse_arrow(
    line: &str,
    pico_arrow: &str,
    plantuml_arrow: &str,
    stereotype: &str,
) -> Option<String> {
    let arrow_idx = line.find(pico_arrow)?;
    let before = &line[..arrow_idx];
    let after = &line[arrow_idx + pico_arrow.len()..];
    let to = before.trim();
    if to.is_empty() {
        return None;
    }

    let (from, label) = if let Some((from_part, msg)) = after.split_once(':') {
        (from_part.trim(), Some(msg.trim()))
    } else {
        (after.trim(), None)
    };
    if from.is_empty() {
        return None;
    }

    Some(if let Some(lbl) = label {
        format!("{from} {plantuml_arrow} {to} : {lbl} {stereotype}")
    } else {
        format!("{from} {plantuml_arrow} {to} : {stereotype}")
    })
}

/// Convert `note left A : text` / `note right A : text` to `note left of A : text`.
fn adapt_picouml_note(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    if lower.starts_with("note over ") {
        let rest = &line["note over ".len()..];
        let (target, text) = rest.split_once(':')?;
        let target = target.trim();
        let text = text.trim();
        if target.is_empty() || text.is_empty() {
            return None;
        }
        return Some(format!("note over {target}: {text}"));
    }
    if lower.starts_with("note ") {
        let rest = &line["note ".len()..];
        let (target, text) = rest.split_once(':')?;
        let target = target.trim();
        let text = text.trim();
        if target.is_empty() || text.is_empty() || !target.contains(',') {
            return None;
        }
        return Some(format!("note over {target}: {text}"));
    }
    let suffix = if lower.starts_with("note left ") {
        Some(("left", &line["note left ".len()..]))
    } else if lower.starts_with("note right ") {
        Some(("right", &line["note right ".len()..]))
    } else {
        None
    }?;

    let (side, rest) = suffix;
    // If it's already `note left of` or `note right of`, don't double-convert.
    let rest_lower = rest.to_ascii_lowercase();
    if rest_lower.starts_with("of ") || rest_lower.starts_with("of\t") {
        return None;
    }

    let (target, text) = rest.split_once(':')?;
    let target = target.trim();
    let text = text.trim();
    if target.is_empty() || text.is_empty() {
        return None;
    }

    Some(format!("note {side} of {target} : {text}"))
}

/// Convert `group X / Y` to `group X` (with `Y` appended as a newline in the label).
fn adapt_picouml_group(line: &str, in_group_block: &mut bool) -> Option<String> {
    let lower = line.to_ascii_lowercase();

    if lower == "end" && *in_group_block {
        *in_group_block = false;
        return Some("end".to_string());
    }

    if !lower.starts_with("group ") {
        return None;
    }

    let rest = &line["group ".len()..].trim();
    if rest.is_empty() {
        return None;
    }

    *in_group_block = true;

    // Split on ` / ` to get label parts.
    if let Some((main_label, extra)) = rest.split_once(" / ") {
        let main = main_label.trim();
        let extra = extra.trim();
        if extra.is_empty() {
            Some(format!("group {main}"))
        } else {
            // Encode the extra label part as a newline in the group label.
            Some(format!("group {main}\\n{extra}"))
        }
    } else {
        // No slash, pass through as-is (the `group X` form is already valid PlantUML).
        None
    }
}

fn replace_prefixed_marker(line: &str, marker: &str, replacement: &str) -> String {
    let lower = line.to_ascii_lowercase();
    let marker_len = marker.len();
    if !lower.trim_start().starts_with(marker) {
        return line.to_string();
    }
    let leading_ws = line.len() - line.trim_start().len();
    let rest_start = leading_ws + marker_len;
    let mut out = String::new();
    out.push_str(&line[..leading_ws]);
    out.push_str(replacement);
    out.push_str(line.get(rest_start..).unwrap_or_default());
    out
}

fn matches_prefixed_uml_marker(line: &str, marker: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let marker_len = marker.len();
    if !lower.starts_with(marker) {
        return false;
    }
    let rest = &line[marker_len..];
    rest.is_empty() || rest.starts_with(char::is_whitespace)
}
