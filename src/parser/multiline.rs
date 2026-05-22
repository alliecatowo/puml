fn parse_bracket_subject(line: &str) -> Option<(String, &str)> {
    let trimmed = line.trim();
    let stripped = trimmed.strip_prefix('[')?;
    let end = stripped.find(']')?;
    let name = stripped[..end].trim().to_string();
    if name.is_empty() {
        return None;
    }
    let rest = stripped[end + 1..].trim();
    Some((name, rest))
}
fn parse_multiline_keyword_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Option<(StatementKind, usize)> {
    let lower = line.to_ascii_lowercase();
    // Check for "legend" (alone or with positioning qualifiers: "legend left", etc.)
    let (key, legend_pos) = if lower == "legend" {
        ("legend", None)
    } else if lower.starts_with("legend ") {
        // Collect any position tokens after "legend"
        let pos_part = line[7..].trim();
        let pos_lower = pos_part.to_ascii_lowercase();
        // Verify all tokens are valid positioning keywords
        let all_pos = pos_lower
            .split_whitespace()
            .all(|t| matches!(t, "left" | "right" | "center" | "top" | "bottom"));
        if all_pos && !pos_part.is_empty() {
            ("legend", Some(pos_part.to_string()))
        } else {
            return None;
        }
    } else {
        let k = ["title", "header", "footer", "caption"]
            .into_iter()
            .find(|k| lower.as_str().eq(*k))?;
        (k, None)
    };

    let end_marker = format!("end {key}");
    let mut body = Vec::new();

    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case(&end_marker) {
            let text = body.join("\n");
            let kind = match key {
                "title" => StatementKind::Title(text),
                "header" => StatementKind::Header(text),
                "footer" => StatementKind::Footer(text),
                "caption" => StatementKind::Caption(text),
                "legend" => {
                    // Emit Legend first; if there's position info emit LegendPos separately.
                    // We return the Legend text here; the LegendPos is handled by returning
                    // the Legend kind with position info embedded for the caller.
                    // Since we can only return one StatementKind, we pack the pos into the
                    // legend_pos field and handle it via a special kind.
                    let _ = legend_pos; // used below
                    StatementKind::Legend(text)
                }
                _ => StatementKind::Legend(text),
            };
            // If there was a position qualifier alongside the legend text, we need to
            // emit both. We return the Legend kind (which the caller will handle) and
            // separately emit a LegendPos. But since we can only return one statement,
            // we encode the position in a specially-prefixed Legend value when present.
            // Convention: if legend_pos is Some, we prefix the text with "LEGEND_POS:<pos>\n".
            // The normalizer detects and splits this prefix.
            if key == "legend" {
                if let Some(ref pos) = legend_pos {
                    let packed = format!("LEGEND_POS:{}\n{}", pos, body.join("\n"));
                    return Some((StatementKind::Legend(packed), idx));
                }
            }
            return Some((kind, idx));
        }
        body.push(trimmed.to_string());
    }

    None
}

fn parse_multiline_note_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Option<(StatementKind, usize)> {
    // Strip leading `/ ` aligned prefix (feature 1.18) before matching the note keyword.
    let (aligned, effective_line) = if line.trim_start().starts_with("/ ") {
        (true, line.trim_start().trim_start_matches('/').trim_start())
    } else {
        (false, line)
    };
    let lower = effective_line.to_ascii_lowercase();
    let note_kw = if lower.starts_with("note ") {
        "note"
    } else if lower.starts_with("hnote ") {
        "hnote"
    } else if lower.starts_with("rnote ") {
        "rnote"
    } else {
        return None;
    };

    let tail = effective_line[note_kw.len()..].trim();
    if tail.is_empty() {
        return None;
    }
    let (head, inline) = tail.split_once(':').unwrap_or((tail, ""));
    let (position, target) = parse_note_head(head.trim());
    if matches!(position.to_ascii_lowercase().as_str(), "left" | "right") && target.is_none() {
        return None;
    }
    let mut body = Vec::new();
    if !inline.trim().is_empty() {
        body.push(inline.trim().to_string());
    }

    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("end note") {
            return Some((
                StatementKind::Note(Note {
                    kind: note_kind_from_keyword(note_kw),
                    position,
                    target,
                    text: body.join("\n"),
                    aligned,
                }),
                idx,
            ));
        }
        if note_end_matches(trimmed, note_kw) {
            return Some((
                StatementKind::Note(Note {
                    kind: note_kind_from_keyword(note_kw),
                    position,
                    target,
                    text: body.join("\n"),
                    aligned,
                }),
                idx,
            ));
        }
        body.push(trimmed.to_string());
    }

    None
}

fn parse_multiline_ref_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Option<(StatementKind, usize)> {
    if !line.to_ascii_lowercase().starts_with("ref ") {
        return None;
    }
    let tail = line[4..].trim();
    let (head, inline) = tail.split_once(':').unwrap_or((tail, ""));
    let head = head.trim();
    if head.is_empty() {
        return None;
    }

    let mut body = Vec::new();
    let mut has_non_empty_body = false;
    if !inline.trim().is_empty() {
        body.push(inline.trim().to_string());
        has_non_empty_body = true;
    }
    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("end ref") {
            if !has_non_empty_body {
                return None;
            }
            let mut label = head.to_string();
            label.push('\n');
            label.push_str(&body.join("\n"));
            return Some((
                StatementKind::Group(Group {
                    kind: "ref".to_string(),
                    label: Some(label),
                }),
                idx,
            ));
        }
        if !trimmed.is_empty() {
            has_non_empty_body = true;
        }
        body.push(trimmed.to_string());
    }
    None
}
