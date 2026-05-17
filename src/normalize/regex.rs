use super::*;

pub(super) fn normalize_regex(document: Document) -> Result<RegexDocument, Diagnostic> {
    let (title, body) = collect_raw_body(&document);
    let mut patterns = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    for line in body {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let tokens = parse_regex_tokens(trimmed, &mut warnings);
        patterns.push(RegexPattern {
            source: trimmed.to_string(),
            tokens,
        });
    }
    Ok(RegexDocument {
        title,
        patterns,
        warnings,
    })
}

fn parse_regex_tokens(input: &str, warnings: &mut Vec<Diagnostic>) -> Vec<RegexToken> {
    let chars: Vec<char> = input.chars().collect();
    let mut idx = 0usize;
    let tokens = parse_regex_alt(&chars, &mut idx, false, warnings);
    if idx < chars.len() {
        warnings.push(Diagnostic::warning(format!(
            "[W_REGEX_UNCONSUMED] trailing input not consumed at offset {idx}"
        )));
    }
    tokens
}

fn parse_regex_alt(
    chars: &[char],
    idx: &mut usize,
    in_group: bool,
    warnings: &mut Vec<Diagnostic>,
) -> Vec<RegexToken> {
    let mut branches: Vec<Vec<RegexToken>> = Vec::new();
    let mut current = parse_regex_seq(chars, idx, in_group, warnings);
    while *idx < chars.len() && chars[*idx] == '|' {
        *idx += 1;
        branches.push(std::mem::take(&mut current));
        current = parse_regex_seq(chars, idx, in_group, warnings);
    }
    if branches.is_empty() {
        current
    } else {
        branches.push(current);
        vec![RegexToken::Alt(branches)]
    }
}

fn parse_regex_seq(
    chars: &[char],
    idx: &mut usize,
    in_group: bool,
    warnings: &mut Vec<Diagnostic>,
) -> Vec<RegexToken> {
    let mut out: Vec<RegexToken> = Vec::new();
    let mut literal = String::new();
    while *idx < chars.len() {
        let ch = chars[*idx];
        if ch == ')' && in_group {
            break;
        }
        if ch == '|' {
            break;
        }
        if ch == '(' {
            flush_literal(&mut literal, &mut out);
            *idx += 1;
            let inner = parse_regex_alt(chars, idx, true, warnings);
            if *idx < chars.len() && chars[*idx] == ')' {
                *idx += 1;
            } else {
                warnings.push(Diagnostic::warning(
                    "[W_REGEX_UNBALANCED] missing closing `)`",
                ));
            }
            push_with_repeat(RegexToken::Group(inner), chars, idx, &mut out);
            continue;
        }
        if ch == '[' {
            flush_literal(&mut literal, &mut out);
            *idx += 1;
            let mut class = String::new();
            while *idx < chars.len() && chars[*idx] != ']' {
                class.push(chars[*idx]);
                *idx += 1;
            }
            if *idx < chars.len() {
                *idx += 1;
            } else {
                warnings.push(Diagnostic::warning(
                    "[W_REGEX_UNBALANCED] missing closing `]`",
                ));
            }
            push_with_repeat(RegexToken::CharClass(class), chars, idx, &mut out);
            continue;
        }
        if ch == '\\' {
            flush_literal(&mut literal, &mut out);
            *idx += 1;
            if *idx < chars.len() {
                let esc = chars[*idx];
                *idx += 1;
                push_with_repeat(RegexToken::Escape(esc), chars, idx, &mut out);
            } else {
                warnings.push(Diagnostic::warning(
                    "[W_REGEX_TRAILING_ESCAPE] trailing backslash",
                ));
            }
            continue;
        }
        if ch == '.' {
            flush_literal(&mut literal, &mut out);
            *idx += 1;
            push_with_repeat(RegexToken::AnyChar, chars, idx, &mut out);
            continue;
        }
        if ch == '^' || ch == '$' {
            flush_literal(&mut literal, &mut out);
            *idx += 1;
            out.push(RegexToken::Anchor(ch.to_string()));
            continue;
        }
        if ch == '{' {
            flush_literal(&mut literal, &mut out);
            let mut spec = String::new();
            while *idx < chars.len() && chars[*idx] != '}' {
                spec.push(chars[*idx]);
                *idx += 1;
            }
            if *idx < chars.len() {
                *idx += 1;
            }
            warnings.push(Diagnostic::warning(format!(
                "[W_REGEX_QUANT_UNSUPPORTED] quantifier `{{{}}}` not fully supported",
                spec.trim_start_matches('{')
            )));
            out.push(RegexToken::Unsupported(format!("{{{spec}}}")));
            continue;
        }
        if matches!(ch, '*' | '+' | '?') {
            // Stray quantifier with no prior atom; treat as literal.
            flush_literal(&mut literal, &mut out);
            *idx += 1;
            warnings.push(Diagnostic::warning(format!(
                "[W_REGEX_STRAY_QUANT] stray quantifier `{ch}`"
            )));
            out.push(RegexToken::Unsupported(ch.to_string()));
            continue;
        }
        literal.push(ch);
        *idx += 1;
        // Peek for following quantifier on the last character of literal.
        if *idx < chars.len() && matches!(chars[*idx], '*' | '+' | '?') {
            // Split off the last char as its own atom so the quantifier applies to it.
            let last = literal.pop();
            flush_literal(&mut literal, &mut out);
            if let Some(c) = last {
                push_with_repeat(RegexToken::Literal(c.to_string()), chars, idx, &mut out);
            }
        }
    }
    flush_literal(&mut literal, &mut out);
    out
}

fn flush_literal(literal: &mut String, out: &mut Vec<RegexToken>) {
    if !literal.is_empty() {
        out.push(RegexToken::Literal(std::mem::take(literal)));
    }
}

fn push_with_repeat(token: RegexToken, chars: &[char], idx: &mut usize, out: &mut Vec<RegexToken>) {
    if let Some(kind) = parse_regex_repeat_kind(chars, idx) {
        out.push(RegexToken::Repeat {
            inner: Box::new(token),
            kind,
        });
        return;
    }
    out.push(token);
}

fn parse_regex_repeat_kind(chars: &[char], idx: &mut usize) -> Option<RepeatKind> {
    if *idx >= chars.len() {
        return None;
    }
    match chars[*idx] {
        '*' => {
            *idx += 1;
            Some(RepeatKind::ZeroOrMore)
        }
        '+' => {
            *idx += 1;
            Some(RepeatKind::OneOrMore)
        }
        '?' => {
            *idx += 1;
            Some(RepeatKind::ZeroOrOne)
        }
        '{' => parse_regex_braced_repeat(chars, idx),
        _ => None,
    }
}

fn parse_regex_braced_repeat(chars: &[char], idx: &mut usize) -> Option<RepeatKind> {
    let start = *idx;
    let mut cursor = start + 1;
    let mut spec = String::new();
    while cursor < chars.len() && chars[cursor] != '}' {
        spec.push(chars[cursor]);
        cursor += 1;
    }
    if cursor >= chars.len() {
        return None;
    }

    let spec = spec.trim();
    if spec.is_empty() {
        return None;
    }

    let kind = if let Some((min_raw, max_raw)) = spec.split_once(',') {
        let min_raw = min_raw.trim();
        let max_raw = max_raw.trim();
        let min = if min_raw.is_empty() {
            None
        } else {
            Some(min_raw.parse::<u32>().ok()?)
        };
        let max = if max_raw.is_empty() {
            None
        } else {
            Some(max_raw.parse::<u32>().ok()?)
        };
        if let (Some(min), Some(max)) = (min, max) {
            if min > max {
                return None;
            }
        }
        RepeatKind::Range { min, max }
    } else {
        RepeatKind::Exact(spec.parse::<u32>().ok()?)
    };

    *idx = cursor + 1;
    Some(kind)
}
