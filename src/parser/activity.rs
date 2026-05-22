fn parse_activity_step(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if let Some(label) = parse_activity_swimlane(trimmed) {
        return Some(activity_step_statement(
            ActivityStepKind::PartitionStart,
            Some(label),
        ));
    }
    if let Some(label) = parse_activity_arrow_directive(trimmed) {
        return Some(activity_step_statement(
            ActivityStepKind::Arrow,
            Some(label),
        ));
    }
    if let Some((label, color)) = parse_activity_colored_connector(trimmed) {
        return Some(activity_step_statement(
            ActivityStepKind::Connector,
            Some(activity_style_label(label, color.as_deref())),
        ));
    }
    if let Some(label) = parse_activity_connector(trimmed) {
        return Some(activity_step_statement(
            ActivityStepKind::Connector,
            Some(label),
        ));
    }
    if let Some((label, color)) = parse_activity_colored_action(trimmed) {
        return Some(activity_step_statement(
            ActivityStepKind::Action,
            Some(activity_style_label(label, color.as_deref())),
        ));
    }
    // `:action;` or `:action` form (including SDL terminator variants: |, <, >, /, \, ], })
    if let Some(rest) = trimmed.strip_prefix(':') {
        let (body, sdl_shape) = parse_activity_action_terminator(rest);
        let label = if let Some(shape) = sdl_shape {
            if body.is_empty() {
                None
            } else {
                Some(format!("\x1fsdl:{shape}\x1f{body}"))
            }
        } else if body.is_empty() {
            None
        } else {
            Some(body.to_string())
        };
        return Some(activity_step_statement(ActivityStepKind::Action, label));
    }
    if trimmed == "start" {
        return Some(activity_step_statement(ActivityStepKind::Start, None));
    }
    if trimmed == "stop" {
        return Some(activity_step_statement(ActivityStepKind::Stop, None));
    }
    if trimmed == "end" {
        return Some(activity_step_statement(ActivityStepKind::End, None));
    }
    if trimmed == "else" || trimmed.starts_with("else ") || trimmed.starts_with("else(") {
        let label = if trimmed == "else" {
            None
        } else {
            extract_paren_label(trimmed.trim_start_matches("else").trim())
        };
        return Some(activity_step_statement(ActivityStepKind::Else, label));
    }
    if trimmed == "endif" {
        return Some(activity_step_statement(ActivityStepKind::EndIf, None));
    }
    if trimmed == "fork" {
        return Some(activity_step_statement(ActivityStepKind::Fork, None));
    }
    if trimmed == "fork again" {
        return Some(activity_step_statement(ActivityStepKind::ForkAgain, None));
    }
    if trimmed == "end fork" || trimmed == "endfork" {
        return Some(activity_step_statement(ActivityStepKind::EndFork, None));
    }
    if trimmed == "split" {
        return Some(activity_step_statement(
            ActivityStepKind::Fork,
            Some("split".to_string()),
        ));
    }
    if trimmed == "split again" {
        return Some(activity_step_statement(
            ActivityStepKind::ForkAgain,
            Some("split again".to_string()),
        ));
    }
    if trimmed == "end split" || trimmed == "endsplit" || trimmed == "end merge" {
        return Some(activity_step_statement(
            ActivityStepKind::EndFork,
            Some("end split".to_string()),
        ));
    }
    if trimmed == "endwhile" || trimmed == "end while" {
        return Some(activity_step_statement(ActivityStepKind::EndWhile, None));
    }
    if trimmed == "repeat" {
        return Some(activity_step_statement(ActivityStepKind::RepeatStart, None));
    }
    if let Some(rest) = trimmed.strip_prefix("if ") {
        return Some(activity_step_statement(
            ActivityStepKind::IfStart,
            Some(parse_activity_if_label(rest.trim())),
        ));
    }
    if let Some(rest) = trimmed.strip_prefix("switch ") {
        return Some(activity_step_statement(
            ActivityStepKind::IfStart,
            Some(format!(
                "switch {}",
                extract_paren_label(rest.trim()).unwrap_or_else(|| rest.trim().to_string())
            )),
        ));
    }
    if let Some(rest) = trimmed.strip_prefix("case ") {
        return Some(activity_step_statement(
            ActivityStepKind::Else,
            extract_paren_label(rest.trim()).or_else(|| Some(rest.trim().to_string())),
        ));
    }
    if trimmed == "endswitch" || trimmed == "end switch" {
        return Some(activity_step_statement(
            ActivityStepKind::EndIf,
            Some("endswitch".to_string()),
        ));
    }
    if let Some(rest) = trimmed.strip_prefix("elseif ") {
        return Some(activity_step_statement(
            ActivityStepKind::Else,
            Some(format!("elseif {}", parse_activity_if_label(rest.trim()))),
        ));
    }
    if let Some(label) = parse_activity_note_step(trimmed) {
        return Some(activity_step_statement(ActivityStepKind::Note, Some(label)));
    }
    if let Some(rest) = trimmed.strip_prefix("while ") {
        return Some(activity_step_statement(
            ActivityStepKind::WhileStart,
            Some(parse_activity_condition_with_branches(rest.trim())),
        ));
    }
    if let Some(rest) = trimmed.strip_prefix("repeat while") {
        let r = rest.trim();
        return Some(activity_step_statement(
            ActivityStepKind::RepeatWhile,
            (!r.is_empty()).then(|| parse_activity_condition_with_branches(r)),
        ));
    }
    if trimmed == "end repeat" || trimmed == "endrepeat" {
        return Some(activity_step_statement(
            ActivityStepKind::EndWhile,
            Some("end repeat".to_string()),
        ));
    }
    if let Some((label, color)) = parse_activity_partition_like(trimmed) {
        return Some(activity_step_statement(
            ActivityStepKind::PartitionStart,
            Some(activity_style_label(label, color.as_deref())),
        ));
    }
    if trimmed == "}" || trimmed == "end group" {
        // Treat lone `}` inside activity as partition close.
        return Some(activity_step_statement(ActivityStepKind::PartitionEnd, None));
    }
    if let Some(rest) = trimmed.strip_prefix("label ") {
        return Some(activity_step_statement(
            ActivityStepKind::Action,
            Some(format!("label {}", rest.trim())),
        ));
    }
    if let Some(rest) = trimmed.strip_prefix("goto ") {
        return Some(activity_step_statement(
            ActivityStepKind::Action,
            Some(format!("goto {}", rest.trim())),
        ));
    }
    if let Some(rest) = trimmed.strip_prefix("backward") {
        let label = rest
            .trim()
            .trim_start_matches(':')
            .trim_end_matches(';')
            .trim();
        return Some(activity_step_statement(
            ActivityStepKind::Action,
            Some(if label.is_empty() {
                "backward".to_string()
            } else {
                format!("backward {label}")
            }),
        ));
    }
    if trimmed == "kill" {
        return Some(activity_step_statement(
            ActivityStepKind::Kill,
            Some(trimmed.to_string()),
        ));
    }
    if trimmed == "detach" {
        return Some(activity_step_statement(
            ActivityStepKind::Detach,
            Some(trimmed.to_string()),
        ));
    }
    if trimmed == "break" || trimmed == "continue" {
        return Some(activity_step_statement(
            ActivityStepKind::Action,
            Some(trimmed.to_string()),
        ));
    }
    None
}

fn looks_like_old_activity_flow(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("(*)")
        || trimmed.starts_with("-->")
        || (trimmed.contains("-->") && trimmed.contains("(*)"))
}

fn parse_activity_old_style_flow(line: &str) -> Option<Vec<StatementKind>> {
    let trimmed = line.trim();
    let arrow_idx = trimmed.find("-->")?;
    let lhs = trimmed[..arrow_idx].trim();
    let rhs = trimmed[arrow_idx + 3..].trim();
    let mut steps = Vec::new();

    if lhs == "(*)" {
        steps.push(activity_step_statement(ActivityStepKind::Start, None));
    }

    if let Some(target) = parse_old_activity_arrow_target(rhs) {
        if target == "(*)" {
            steps.push(activity_step_statement(ActivityStepKind::Stop, None));
        } else if target.eq_ignore_ascii_case("detach") {
            steps.push(activity_step_statement(
                ActivityStepKind::Detach,
                Some("detach".to_string()),
            ));
        } else {
            steps.push(activity_step_statement(
                ActivityStepKind::Action,
                Some(target),
            ));
        }
    }

    (!steps.is_empty()).then_some(steps)
}

fn parse_old_activity_arrow_target(rhs: &str) -> Option<String> {
    let mut rest = rhs.trim();
    if let Some(after_label) = rest.strip_prefix('[') {
        let close = after_label.find(']')?;
        rest = after_label[close + 1..].trim();
    }
    if let Some(after_dir) = rest.strip_prefix("right of ") {
        rest = after_dir.trim();
    } else if let Some(after_dir) = rest.strip_prefix("left of ") {
        rest = after_dir.trim();
    } else if let Some(after_dir) = rest.strip_prefix("up of ") {
        rest = after_dir.trim();
    } else if let Some(after_dir) = rest.strip_prefix("down of ") {
        rest = after_dir.trim();
    }

    if rest == "(*)" {
        return Some(rest.to_string());
    }

    if let Some(label) = parse_quoted_activity_label(rest) {
        return Some(label);
    }

    let label = rest.trim_end_matches(';').trim();
    (!label.is_empty()).then(|| label.to_string())
}

fn parse_quoted_activity_label(input: &str) -> Option<String> {
    let input = input.trim();
    let rest = input.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

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

fn parse_activity_note_step(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let prefixes = [
        "floating note left",
        "floating note right",
        "floating note",
        "note left",
        "note right",
        "note top",
        "note bottom",
    ];
    let prefix = prefixes.iter().find(|prefix| lower.starts_with(**prefix))?;
    let rest = line[prefix.len()..]
        .trim()
        .trim_start_matches(':')
        .trim_end_matches(';')
        .trim();
    Some(if rest.is_empty() {
        "note".to_string()
    } else {
        rest.to_string()
    })
}

fn parse_activity_if_label(input: &str) -> String {
    let lower = input.to_ascii_lowercase();
    if let Some(idx) = lower.find(" then ") {
        let condition_raw = input[..idx].trim();
        let then_raw = input[idx + " then ".len()..].trim();
        let condition = parse_activity_condition_with_branches(condition_raw);
        if let Some(branch) = extract_paren_label(then_raw) {
            if !branch.is_empty() {
                return format!("{condition} / {branch}");
            }
        }
        return condition;
    }
    let body = input.trim_end_matches("then").trim();
    extract_paren_label(body).unwrap_or_else(|| body.to_string())
}

fn parse_activity_condition_with_branches(input: &str) -> String {
    let trimmed = input.trim();
    let condition = extract_first_paren_label(trimmed).unwrap_or_else(|| {
        trimmed
            .split_once(" is ")
            .map(|(lhs, _)| lhs.trim())
            .unwrap_or(trimmed)
            .trim_end_matches("then")
            .trim()
            .to_string()
    });
    let mut parts = vec![condition];
    for marker in [" is ", " then ", " not "] {
        if let Some((_, tail)) = trimmed.split_once(marker) {
            if let Some(value) = extract_first_paren_label(tail) {
                if !value.is_empty() {
                    parts.push(value);
                }
            }
        }
    }
    parts.join(" / ")
}

fn extract_first_paren_label(input: &str) -> Option<String> {
    let s = input.trim();
    let open = s.find('(')?;
    let close = s[open + 1..].find(')')? + open + 1;
    if close <= open {
        return None;
    }
    Some(s[open + 1..close].trim().to_string())
}

fn extract_paren_label(input: &str) -> Option<String> {
    let s = input.trim();
    let open = s.find('(')?;
    let close = s.rfind(')')?;
    if close <= open {
        return None;
    }
    Some(s[open + 1..close].trim().to_string())
}
