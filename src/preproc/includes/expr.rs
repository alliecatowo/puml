use crate::diagnostic::Diagnostic;
use crate::preproc::macros::expand_preprocessor_text;
use crate::preproc::PreprocState;

pub(in crate::preproc) fn evaluate_preprocess_expr(
    expr: &str,
    state: &PreprocState,
) -> Result<bool, Diagnostic> {
    let raw = expr.trim();
    if raw.is_empty() {
        return Err(Diagnostic::error_code(
            "E_PREPROC_EXPR_REQUIRED",
            "preprocessor condition requires an expression",
        ));
    }
    // Compound boolean: split top-level || then && and recurse on each half
    if let Some((lhs, rhs)) = split_top_level(raw, "||") {
        return Ok(evaluate_preprocess_expr(&lhs, state)? || evaluate_preprocess_expr(&rhs, state)?);
    }
    if let Some((lhs, rhs)) = split_top_level_word(raw, "or") {
        return Ok(evaluate_preprocess_expr(&lhs, state)? || evaluate_preprocess_expr(&rhs, state)?);
    }
    if let Some((lhs, rhs)) = split_top_level(raw, "&&") {
        return Ok(evaluate_preprocess_expr(&lhs, state)? && evaluate_preprocess_expr(&rhs, state)?);
    }
    if let Some((lhs, rhs)) = split_top_level_word(raw, "and") {
        return Ok(evaluate_preprocess_expr(&lhs, state)? && evaluate_preprocess_expr(&rhs, state)?);
    }
    if let Some((lhs, rhs)) = split_top_level_word(raw, "xor") {
        return Ok(evaluate_preprocess_expr(&lhs, state)? ^ evaluate_preprocess_expr(&rhs, state)?);
    }

    if let Some((negated, name)) = parse_defined_call(raw) {
        let defined = state.defines.contains_key(name) || state.vars.contains_key(name);
        return Ok(if negated { !defined } else { defined });
    }

    let substituted = expand_preprocessor_text(raw, state, 0)?;
    evaluate_scalar_expr(substituted.trim())
}

fn parse_defined_call(expr: &str) -> Option<(bool, &str)> {
    let trimmed = expr.trim();
    let (negated, rest) = if let Some(rem) = trimmed.strip_prefix('!') {
        (true, rem.trim_start())
    } else {
        (false, trimmed)
    };
    let lower = rest.to_ascii_lowercase();
    if !lower.starts_with("defined") {
        return None;
    }
    let rest = &rest["defined".len()..];
    let name = rest
        .trim_start()
        .strip_prefix('(')?
        .strip_suffix(')')?
        .trim();
    if name.is_empty() {
        return None;
    }
    Some((negated, name))
}

pub(in crate::preproc) fn evaluate_scalar_expr(expr: &str) -> Result<bool, Diagnostic> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Ok(false);
    }

    if let Some(inner) = strip_outer_balanced_parens(trimmed) {
        return evaluate_scalar_expr(inner);
    }

    // Compound boolean: try top-level || then && (split outside quotes/parens)
    if let Some((lhs, rhs)) = split_top_level(trimmed, "||") {
        return Ok(evaluate_scalar_expr(&lhs)? || evaluate_scalar_expr(&rhs)?);
    }
    if let Some((lhs, rhs)) = split_top_level_word(trimmed, "or") {
        return Ok(evaluate_scalar_expr(&lhs)? || evaluate_scalar_expr(&rhs)?);
    }
    if let Some((lhs, rhs)) = split_top_level(trimmed, "&&") {
        return Ok(evaluate_scalar_expr(&lhs)? && evaluate_scalar_expr(&rhs)?);
    }
    if let Some((lhs, rhs)) = split_top_level_word(trimmed, "and") {
        return Ok(evaluate_scalar_expr(&lhs)? && evaluate_scalar_expr(&rhs)?);
    }
    if let Some((lhs, rhs)) = split_top_level_word(trimmed, "xor") {
        return Ok(evaluate_scalar_expr(&lhs)? ^ evaluate_scalar_expr(&rhs)?);
    }

    let lower_trimmed = trimmed.to_ascii_lowercase();
    if lower_trimmed.starts_with("not ") {
        return evaluate_scalar_expr(trimmed[3..].trim_start()).map(|v| !v);
    }
    if let Some((lhs, rhs)) = split_top_level(trimmed, "==") {
        return Ok(normalize_expr_value(&lhs) == normalize_expr_value(&rhs));
    }
    if let Some((lhs, rhs)) = split_top_level(trimmed, "!=") {
        return Ok(normalize_expr_value(&lhs) != normalize_expr_value(&rhs));
    }
    if let Some((lhs, rhs)) = split_top_level(trimmed, "<>") {
        return Ok(normalize_expr_value(&lhs) != normalize_expr_value(&rhs));
    }
    // Numeric comparisons: check two-char operators before one-char to avoid splitting <=/>= wrong.
    if let Some((lhs, rhs)) = split_top_level(trimmed, "<=") {
        let a = normalize_expr_value(&lhs)
            .parse::<i64>()
            .unwrap_or(i64::MIN);
        let b = normalize_expr_value(&rhs)
            .parse::<i64>()
            .unwrap_or(i64::MAX);
        return Ok(a <= b);
    }
    if let Some((lhs, rhs)) = split_top_level(trimmed, ">=") {
        let a = normalize_expr_value(&lhs)
            .parse::<i64>()
            .unwrap_or(i64::MAX);
        let b = normalize_expr_value(&rhs)
            .parse::<i64>()
            .unwrap_or(i64::MIN);
        return Ok(a >= b);
    }
    if let Some((lhs, rhs)) = split_top_level(trimmed, "<") {
        let a = normalize_expr_value(&lhs)
            .parse::<i64>()
            .unwrap_or(i64::MIN);
        let b = normalize_expr_value(&rhs)
            .parse::<i64>()
            .unwrap_or(i64::MAX);
        return Ok(a < b);
    }
    if let Some((lhs, rhs)) = split_top_level(trimmed, ">") {
        let a = normalize_expr_value(&lhs)
            .parse::<i64>()
            .unwrap_or(i64::MAX);
        let b = normalize_expr_value(&rhs)
            .parse::<i64>()
            .unwrap_or(i64::MIN);
        return Ok(a > b);
    }
    if let Some(inner) = trimmed.strip_prefix('!') {
        return evaluate_scalar_expr(inner).map(|v| !v);
    }
    if trimmed.contains('(') || trimmed.contains(')') {
        return Err(Diagnostic::error_code(
            "E_PREPROC_EXPR_UNSUPPORTED",
            "only simple conditions are supported in this preprocessor slice",
        ));
    }

    let normalized = normalize_expr_value(trimmed);
    if normalized.is_empty() {
        return Ok(false);
    }
    match normalized.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => return Ok(true),
        "false" | "no" | "off" => return Ok(false),
        _ => {}
    }
    if let Ok(n) = normalized.parse::<i64>() {
        return Ok(n != 0);
    }
    Ok(false)
}

fn strip_outer_balanced_parens(expr: &str) -> Option<&str> {
    if !expr.starts_with('(') || !expr.ends_with(')') {
        return None;
    }
    let bytes = expr.as_bytes();
    let mut depth: i32 = 0;
    let mut in_str = false;
    for (idx, b) in bytes.iter().enumerate() {
        if in_str {
            if *b == b'"' {
                in_str = false;
            }
            continue;
        }
        match *b {
            b'"' => in_str = true,
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
                if depth == 0 && idx != bytes.len() - 1 {
                    return None;
                }
            }
            _ => {}
        }
    }
    if depth == 0 {
        Some(expr[1..expr.len() - 1].trim())
    } else {
        None
    }
}

/// Split `expr` on the first top-level occurrence of `sep`, respecting
/// parentheses depth and double-quoted strings. Returns None if `sep`
/// is absent at depth zero (which keeps short-circuit chains correct).
fn split_top_level(expr: &str, sep: &str) -> Option<(String, String)> {
    let bytes = expr.as_bytes();
    let sep_bytes = sep.as_bytes();
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut i = 0;
    while i + sep_bytes.len() <= bytes.len() {
        let b = bytes[i];
        if in_str {
            if b == b'"' {
                in_str = false;
            }
        } else {
            match b {
                b'"' => in_str = true,
                b'(' => depth += 1,
                b')' => depth = depth.saturating_sub(1),
                _ => {}
            }
            if depth == 0 && bytes[i..].starts_with(sep_bytes) {
                let lhs = expr[..i].trim().to_string();
                let rhs = expr[i + sep_bytes.len()..].trim().to_string();
                if !lhs.is_empty() && !rhs.is_empty() {
                    return Some((lhs, rhs));
                }
            }
        }
        i += 1;
    }
    None
}

fn split_top_level_word(expr: &str, sep: &str) -> Option<(String, String)> {
    let sep_lower = sep.to_ascii_lowercase();
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut token_start: Option<usize> = None;
    for (idx, ch) in expr.char_indices() {
        if in_str {
            if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => {
                if let Some(start) = token_start.take() {
                    if is_top_level_word_match(expr, start, idx, &sep_lower, depth) {
                        return split_word_at(expr, start, idx);
                    }
                }
                in_str = true;
            }
            '(' => {
                if let Some(start) = token_start.take() {
                    if is_top_level_word_match(expr, start, idx, &sep_lower, depth) {
                        return split_word_at(expr, start, idx);
                    }
                }
                depth += 1;
            }
            ')' => {
                if let Some(start) = token_start.take() {
                    if is_top_level_word_match(expr, start, idx, &sep_lower, depth) {
                        return split_word_at(expr, start, idx);
                    }
                }
                depth = depth.saturating_sub(1);
            }
            c if c.is_whitespace() => {
                if let Some(start) = token_start.take() {
                    if is_top_level_word_match(expr, start, idx, &sep_lower, depth) {
                        return split_word_at(expr, start, idx);
                    }
                }
            }
            _ => {
                if token_start.is_none() {
                    token_start = Some(idx);
                }
            }
        }
    }
    if let Some(start) = token_start {
        if is_top_level_word_match(expr, start, expr.len(), &sep_lower, depth) {
            return split_word_at(expr, start, expr.len());
        }
    }
    None
}

fn is_top_level_word_match(
    expr: &str,
    start: usize,
    end: usize,
    sep_lower: &str,
    depth: i32,
) -> bool {
    depth == 0 && expr[start..end].eq_ignore_ascii_case(sep_lower)
}

fn split_word_at(expr: &str, start: usize, end: usize) -> Option<(String, String)> {
    let lhs = expr[..start].trim().to_string();
    let rhs = expr[end..].trim().to_string();
    if lhs.is_empty() || rhs.is_empty() {
        None
    } else {
        Some((lhs, rhs))
    }
}

fn normalize_expr_value(value: &str) -> String {
    let normalized = value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string();
    if let Some(n) = eval_int_expr(&normalized) {
        n.to_string()
    } else {
        normalized
    }
}

/// Evaluate a simple two-operand integer arithmetic expression such as "3 + 1"
/// or "5 - 2".  Only +, -, *, / operators on integer literals are handled.
/// Returns `None` if the expression is not in this form (non-integer values or
/// more complex expressions), so the caller can fall back to the raw string.
pub(in crate::preproc) fn eval_simple_arithmetic(expr: &str) -> Option<i64> {
    eval_int_expr(expr)
}

pub(in crate::preproc) fn eval_int_expr(expr: &str) -> Option<i64> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(inner) = strip_outer_balanced_parens(trimmed) {
        return eval_int_expr(inner);
    }
    if let Some((lhs, op, rhs)) = split_top_level_arithmetic(trimmed, &['+', '-']) {
        let a = eval_int_expr(lhs)?;
        let b = eval_int_expr(rhs)?;
        return Some(if op == '+' { a + b } else { a - b });
    }
    if let Some((lhs, op, rhs)) = split_top_level_arithmetic(trimmed, &['*', '/', '%']) {
        let a = eval_int_expr(lhs)?;
        let b = eval_int_expr(rhs)?;
        return match op {
            '*' => Some(a * b),
            '/' if b != 0 => Some(a / b),
            '%' if b != 0 => Some(a % b),
            _ => None,
        };
    }
    trimmed.parse::<i64>().ok()
}

fn split_top_level_arithmetic<'a>(expr: &'a str, ops: &[char]) -> Option<(&'a str, char, &'a str)> {
    let mut depth = 0i32;
    let mut in_str = false;
    let mut last = None;
    for (idx, ch) in expr.char_indices() {
        if in_str {
            if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 && ops.contains(&ch) => {
                if ch == '-' {
                    let prev = expr[..idx].chars().rev().find(|c| !c.is_whitespace());
                    if prev.is_none() || matches!(prev, Some('(' | '+' | '-' | '*' | '/' | '%')) {
                        continue;
                    }
                }
                last = Some((idx, ch));
            }
            _ => {}
        }
    }
    let (idx, op) = last?;
    let lhs = expr[..idx].trim();
    let rhs = expr[idx + op.len_utf8()..].trim();
    if lhs.is_empty() || rhs.is_empty() {
        None
    } else {
        Some((lhs, op, rhs))
    }
}
