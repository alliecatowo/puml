/// Parse an `endwhile` or `end while` line, optionally with an exit label.
///
/// Matches:
/// - `endwhile`           → exit label: None
/// - `endwhile (no)`      → exit label: Some("no")
/// - `endwhile(no)`       → exit label: Some("no")
/// - `end while`          → exit label: None
/// - `end while (done)`   → exit label: Some("done")
///
/// Returns `None` if the line is not an endwhile token.
pub(crate) fn parse_activity_endwhile(input: &str) -> Option<Option<String>> {
    let lower = input.to_ascii_lowercase();
    let rest = if lower.starts_with("endwhile") {
        &input["endwhile".len()..]
    } else if lower.starts_with("end while") {
        &input["end while".len()..]
    } else {
        return None;
    };
    let rest = rest.trim();
    let label = if rest.is_empty() {
        None
    } else {
        extract_paren_label(rest).filter(|s| !s.is_empty())
    };
    Some(label)
}

pub(crate) fn parse_activity_if_label(input: &str) -> String {
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

pub(crate) fn parse_activity_condition_with_branches(input: &str) -> String {
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

pub(crate) fn extract_first_paren_label(input: &str) -> Option<String> {
    let s = input.trim();
    let open = s.find('(')?;
    let close = s[open + 1..].find(')')? + open + 1;
    if close <= open {
        return None;
    }
    Some(s[open + 1..close].trim().to_string())
}

pub(crate) fn extract_paren_label(input: &str) -> Option<String> {
    let s = input.trim();
    let open = s.find('(')?;
    let close = s.rfind(')')?;
    if close <= open {
        return None;
    }
    Some(s[open + 1..close].trim().to_string())
}
