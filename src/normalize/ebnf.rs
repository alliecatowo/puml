use super::*;

pub(super) fn normalize_ebnf(document: Document) -> Result<EbnfDocument, Diagnostic> {
    let (title, body) = collect_raw_body(&document);
    let mut rules = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let joined = body.join("\n");
    // Split rules on `;` terminator.
    for chunk in joined.split(';') {
        let chunk = chunk.trim();
        if chunk.is_empty() {
            continue;
        }
        let Some((name, body)) = chunk.split_once('=') else {
            warnings.push(Diagnostic::warning(format!(
                "[W_EBNF_RULE_MALFORMED] missing `=` in rule `{chunk}`"
            )));
            continue;
        };
        let name = name.trim().to_string();
        let body = body.trim().to_string();
        let tokens = parse_ebnf_tokens(&body, &mut warnings);
        rules.push(EbnfRule { name, body, tokens });
    }
    Ok(EbnfDocument {
        title,
        rules,
        warnings,
    })
}

fn parse_ebnf_tokens(input: &str, warnings: &mut Vec<Diagnostic>) -> Vec<EbnfToken> {
    let chars: Vec<char> = input.chars().collect();
    let mut idx = 0usize;
    let tokens = parse_ebnf_alt(&chars, &mut idx, None, warnings);
    if idx < chars.len() {
        warnings.push(Diagnostic::warning(format!(
            "[W_EBNF_UNCONSUMED] trailing input at offset {idx}"
        )));
    }
    tokens
}

fn parse_ebnf_alt(
    chars: &[char],
    idx: &mut usize,
    terminator: Option<char>,
    warnings: &mut Vec<Diagnostic>,
) -> Vec<EbnfToken> {
    let mut branches: Vec<Vec<EbnfToken>> = Vec::new();
    let mut current = parse_ebnf_seq(chars, idx, terminator, warnings);
    while *idx < chars.len() && chars[*idx] == '|' {
        *idx += 1;
        branches.push(std::mem::take(&mut current));
        current = parse_ebnf_seq(chars, idx, terminator, warnings);
    }
    if branches.is_empty() {
        current
    } else {
        branches.push(current);
        vec![EbnfToken::Alt(branches)]
    }
}

fn parse_ebnf_seq(
    chars: &[char],
    idx: &mut usize,
    terminator: Option<char>,
    warnings: &mut Vec<Diagnostic>,
) -> Vec<EbnfToken> {
    let mut out: Vec<EbnfToken> = Vec::new();
    while *idx < chars.len() {
        let ch = chars[*idx];
        if Some(ch) == terminator {
            break;
        }
        if ch == '|' {
            break;
        }
        if ch.is_whitespace() {
            *idx += 1;
            continue;
        }
        if ch == '"' || ch == '\'' {
            let quote = ch;
            *idx += 1;
            let mut s = String::new();
            while *idx < chars.len() && chars[*idx] != quote {
                s.push(chars[*idx]);
                *idx += 1;
            }
            if *idx < chars.len() {
                *idx += 1;
            }
            let token = EbnfToken::Terminal(s);
            push_ebnf_with_repeat(token, chars, idx, &mut out);
            continue;
        }
        if ch == '(' {
            *idx += 1;
            let inner = parse_ebnf_alt(chars, idx, Some(')'), warnings);
            if *idx < chars.len() && chars[*idx] == ')' {
                *idx += 1;
            } else {
                warnings.push(Diagnostic::warning(
                    "[W_EBNF_UNBALANCED] missing closing `)`",
                ));
            }
            push_ebnf_with_repeat(EbnfToken::Group(inner), chars, idx, &mut out);
            continue;
        }
        if ch == '[' {
            *idx += 1;
            let inner = parse_ebnf_alt(chars, idx, Some(']'), warnings);
            if *idx < chars.len() && chars[*idx] == ']' {
                *idx += 1;
            } else {
                warnings.push(Diagnostic::warning(
                    "[W_EBNF_UNBALANCED] missing closing `]`",
                ));
            }
            push_ebnf_with_repeat(EbnfToken::Optional(inner), chars, idx, &mut out);
            continue;
        }
        if ch == '{' {
            *idx += 1;
            let inner = parse_ebnf_alt(chars, idx, Some('}'), warnings);
            if *idx < chars.len() && chars[*idx] == '}' {
                *idx += 1;
            } else {
                warnings.push(Diagnostic::warning(
                    "[W_EBNF_UNBALANCED] missing closing `}`",
                ));
            }
            push_ebnf_with_repeat(EbnfToken::Repetition(inner), chars, idx, &mut out);
            continue;
        }
        if ch.is_alphanumeric() || ch == '_' {
            let mut name = String::new();
            while *idx < chars.len()
                && (chars[*idx].is_alphanumeric() || chars[*idx] == '_' || chars[*idx] == '-')
            {
                name.push(chars[*idx]);
                *idx += 1;
            }
            push_ebnf_with_repeat(EbnfToken::NonTerminal(name), chars, idx, &mut out);
            continue;
        }
        // Unknown character; skip with warning.
        warnings.push(Diagnostic::warning(format!(
            "[W_EBNF_UNSUPPORTED_CHAR] unsupported character `{ch}`"
        )));
        out.push(EbnfToken::Unsupported(ch.to_string()));
        *idx += 1;
    }
    out
}

fn push_ebnf_with_repeat(
    token: EbnfToken,
    chars: &[char],
    idx: &mut usize,
    out: &mut Vec<EbnfToken>,
) {
    if let Some(kind) = parse_ebnf_repeat_kind(chars, idx) {
        out.push(EbnfToken::Repeat {
            inner: Box::new(token),
            kind,
        });
        return;
    }
    out.push(token);
}

fn parse_ebnf_repeat_kind(chars: &[char], idx: &mut usize) -> Option<RepeatKind> {
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
        '{' => parse_ebnf_braced_repeat(chars, idx),
        _ => None,
    }
}

fn parse_ebnf_braced_repeat(chars: &[char], idx: &mut usize) -> Option<RepeatKind> {
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
