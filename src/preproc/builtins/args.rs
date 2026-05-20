use crate::diagnostic::Diagnostic;

use crate::preproc::PreprocParam;

/// Strip a single layer of matching double quotes from a value.
pub(in crate::preproc) fn strip_quotes(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

pub(in crate::preproc) fn parse_int_lenient(s: &str) -> i64 {
    let t = s.trim();
    if t.is_empty() {
        return 0;
    }
    if let Ok(n) = t.parse::<i64>() {
        return n;
    }
    // PlantUML's `%intval` is lenient: extract the longest leading numeric
    // prefix (optionally signed) and fall back to 0 when nothing parses.
    let bytes = t.as_bytes();
    let mut end = 0usize;
    if !bytes.is_empty() && (bytes[0] == b'-' || bytes[0] == b'+') {
        end += 1;
    }
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == 0 || (end == 1 && (bytes[0] == b'-' || bytes[0] == b'+')) {
        return 0;
    }
    t[..end].parse::<i64>().unwrap_or(0)
}

pub(in crate::preproc) fn extract_parenthesized_args(
    chars: &[char],
    open_idx: usize,
) -> Result<(String, usize), Diagnostic> {
    let mut depth = 0usize;
    let mut i = open_idx;
    let mut in_quotes = false;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '"' {
            in_quotes = !in_quotes;
        } else if !in_quotes {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    let args: String = chars[open_idx + 1..i].iter().collect();
                    return Ok((args, i + 1));
                }
            }
        }
        i += 1;
    }
    Err(Diagnostic::error_code(
        "E_PREPROC_CALL_SYNTAX",
        "malformed preprocessor call: missing closing `)`",
    ))
}

pub(in crate::preproc) fn parse_params(raw: &str) -> Result<Vec<PreprocParam>, Diagnostic> {
    let mut params = Vec::new();
    let normalized = raw.replace("##", ",");
    for piece in split_args(&normalized)? {
        let trimmed = piece.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (name_part, default) = if let Some((n, d)) = trimmed.split_once('=') {
            (n.trim(), Some(d.trim().to_string()))
        } else {
            (trimmed, None)
        };
        let name = name_part.trim_start_matches('$').trim().to_string();
        if name.is_empty() {
            return Err(Diagnostic::error_code(
                "E_PREPROC_SIGNATURE",
                "parameter name cannot be empty",
            ));
        }
        params.push(PreprocParam { name, default });
    }
    Ok(params)
}

pub(in crate::preproc) fn split_args(raw: &str) -> Result<Vec<String>, Diagnostic> {
    let mut out = Vec::new();
    let mut curr = String::new();
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut in_quotes = false;
    for ch in raw.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            curr.push(ch);
            continue;
        }
        if !in_quotes {
            match ch {
                '(' => paren_depth += 1,
                ')' => {
                    if paren_depth == 0 {
                        return Err(Diagnostic::error_code(
                            "E_PREPROC_CALL_SYNTAX",
                            "unbalanced `)` in argument list",
                        ));
                    }
                    paren_depth -= 1;
                }
                '{' => brace_depth += 1,
                '}' => brace_depth = brace_depth.saturating_sub(1),
                '[' => bracket_depth += 1,
                ']' => bracket_depth = bracket_depth.saturating_sub(1),
                ',' if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                    out.push(curr.trim().to_string());
                    curr.clear();
                    continue;
                }
                _ => {}
            }
        }
        curr.push(ch);
    }
    if in_quotes || paren_depth != 0 || brace_depth != 0 || bracket_depth != 0 {
        return Err(Diagnostic::error_code(
            "E_PREPROC_CALL_SYNTAX",
            "malformed argument list",
        ));
    }
    if !curr.trim().is_empty() {
        out.push(curr.trim().to_string());
    }
    Ok(out)
}
