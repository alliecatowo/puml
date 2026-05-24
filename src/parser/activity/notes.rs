fn parse_activity_note_step(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let (_, side, floating) = parse_activity_note_prefix(line, &lower)?;
    let prefix = ACTIVITY_NOTE_PREFIXES
        .iter()
        .find(|(prefix, _, _)| lower.starts_with(*prefix))?
        .0;
    let rest = line[prefix.len()..]
        .trim()
        .trim_start_matches(':')
        .trim_end_matches(';')
        .trim();
    let text = if rest.is_empty() {
        "note".to_string()
    } else {
        rest.to_string()
    };
    Some(activity_note_label(side, floating, &text))
}

fn parse_activity_multiline_note_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Option<(StatementKind, usize)> {
    let lower = line.to_ascii_lowercase();
    let (prefix, side, floating) = parse_activity_note_prefix(line, &lower)?;
    let rest = line[prefix.len()..].trim();
    let mut body = Vec::new();
    if let Some((_, inline)) = rest.split_once(':') {
        let inline = inline.trim().trim_end_matches(';').trim();
        if !inline.is_empty() {
            body.push(inline.to_string());
        }
        let next_content = lines
            .iter()
            .skip(start + 1)
            .map(|(raw, _)| raw.trim())
            .find(|trimmed| !trimmed.is_empty());
        if !next_content.is_some_and(|trimmed| {
            trimmed.eq_ignore_ascii_case("end note") || trimmed.eq_ignore_ascii_case("endnote")
        }) {
            return None;
        }
    }

    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("end note") || trimmed.eq_ignore_ascii_case("endnote") {
            let text = if body.is_empty() {
                "note".to_string()
            } else {
                body.join("\n")
            };
            return Some((
                activity_step_statement(
                    ActivityStepKind::Note,
                    Some(activity_note_label(side, floating, &text)),
                ),
                idx,
            ));
        }
        body.push(trimmed.to_string());
    }

    None
}

const ACTIVITY_NOTE_PREFIXES: &[(&str, &str, bool)] = &[
    ("floating note left", "left", true),
    ("floating note right", "right", true),
    ("floating note", "right", true),
    ("note left", "left", false),
    ("note right", "right", false),
    ("note top", "top", false),
    ("note bottom", "bottom", false),
];

fn parse_activity_note_prefix(
    line: &str,
    lower: &str,
) -> Option<(&'static str, &'static str, bool)> {
    let (prefix, side, floating) = ACTIVITY_NOTE_PREFIXES
        .iter()
        .find(|(prefix, _, _)| lower.starts_with(*prefix))?;
    let next = line.get(prefix.len()..)?.chars().next();
    if !matches!(next, None | Some(':') | Some(';')) && !next.is_some_and(char::is_whitespace) {
        return None;
    }
    Some((*prefix, *side, *floating))
}

fn activity_note_label(side: &str, floating: bool, text: &str) -> String {
    format!(
        "\x1factivity:note:side={side}:floating={}\x1f{text}",
        u8::from(floating)
    )
}
