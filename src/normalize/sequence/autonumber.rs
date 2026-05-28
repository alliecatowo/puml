use super::state::SequenceNormalizeState;
use super::*;

pub(super) fn canonicalize_autonumber_raw(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut out = String::with_capacity(trimmed.len());
    let mut in_quotes = false;
    let mut prev_space = false;
    for ch in trimmed.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            prev_space = false;
            out.push(ch);
            continue;
        }
        if ch.is_whitespace() && !in_quotes {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
            continue;
        }
        prev_space = false;
        out.push(ch);
    }
    Some(out.trim().to_string())
}

impl SequenceNormalizeState {
    pub(super) fn handle_autonumber(
        &mut self,
        span: crate::source::Span,
        value: Option<String>,
    ) -> Result<(), Diagnostic> {
        groups::mark_group_content(&mut self.group_stack);
        if let Some(raw) = value.as_deref() {
            validate_autonumber_raw(raw).map_err(|reason| {
                Diagnostic::error(format!("[E_AUTONUMBER_FORMAT_UNSUPPORTED] {reason}"))
                    .with_span(span)
            })?;
        }
        self.events.push(SequenceEvent {
            span,
            kind: SequenceEventKind::Autonumber(
                value.as_deref().and_then(canonicalize_autonumber_raw),
            ),
        });
        Ok(())
    }
}

pub(super) fn validate_autonumber_raw(raw: &str) -> Result<(), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("stop")
        || trimmed.eq_ignore_ascii_case("off")
        || trimmed.eq_ignore_ascii_case("resume")
    {
        return Ok(());
    }

    let (format, body) = if trimmed.contains('"') {
        let Some((format, before)) = trailing_quoted_format(trimmed) else {
            return Err("malformed quoted autonumber format; quote-delimited format must be the final token".to_string());
        };
        (Some(format), before.trim_end())
    } else {
        (None, trimmed)
    };

    let mut tokens: Vec<&str> = body.split_whitespace().collect();
    let mut resume = false;
    if tokens.len() == 2
        && tokens[0].eq_ignore_ascii_case("inc")
        && is_autonumber_increment_level(tokens[1])
    {
        return Ok(());
    }
    if matches!(tokens.first(), Some(token) if token.eq_ignore_ascii_case("resume")) {
        resume = true;
        tokens.remove(0);
    }

    let mut idx = 0usize;
    if resume {
        if idx < tokens.len() && tokens[idx].parse::<u64>().is_ok() {
            idx += 1;
        }
    } else if idx < tokens.len() {
        if is_autonumber_counter_token(tokens[idx]) {
            idx += 1;
            if idx < tokens.len() && tokens[idx].parse::<u64>().is_ok() {
                idx += 1;
            } else if idx < tokens.len() && looks_like_autonumber_counter_token(tokens[idx]) {
                return Err(
                    "unsupported autonumber syntax; increment must be an unsigned integer"
                        .to_string(),
                );
            }
        } else if looks_like_autonumber_counter_token(tokens[idx]) {
            return Err(
                "malformed dotted autonumber start; expected dot-separated unsigned integers"
                    .to_string(),
            );
        }
    }

    let unquoted_format = if idx < tokens.len() {
        let fmt = tokens[idx];
        idx += 1;
        Some(fmt)
    } else {
        None
    };

    if idx < tokens.len() {
        return Err(
            "unsupported autonumber syntax; expected `autonumber [start] [increment] [format]` or `autonumber resume [increment] [format]`".to_string(),
        );
    }

    if let Some(fmt) = format.or(unquoted_format.map(str::to_string)) {
        validate_autonumber_format(&fmt)?;
    }

    Ok(())
}

fn is_autonumber_counter_token(token: &str) -> bool {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed
        .split(['.', ';', ',', ':'])
        .all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))
}

fn looks_like_autonumber_counter_token(token: &str) -> bool {
    let trimmed = token.trim();
    trimmed
        .bytes()
        .any(|b| matches!(b, b'.' | b';' | b',' | b':'))
        && trimmed
            .bytes()
            .all(|b| b.is_ascii_digit() || matches!(b, b'.' | b';' | b',' | b':'))
        && trimmed.bytes().any(|b| b.is_ascii_digit())
}

fn is_autonumber_increment_level(token: &str) -> bool {
    token.len() == 1 && token.bytes().all(|b| b.is_ascii_alphabetic())
}

fn trailing_quoted_format(raw: &str) -> Option<(String, &str)> {
    let trimmed = raw.trim_end();
    let end = trimmed.strip_suffix('"')?;
    let start = end.rfind('"')?;
    let format = end[start + 1..].to_string();
    let prefix = &end[..start];
    Some((format, prefix))
}

fn validate_autonumber_format(format: &str) -> Result<(), String> {
    let fmt = format.trim();
    if fmt.is_empty() {
        return Err("autonumber format must not be empty".to_string());
    }
    // HTML-tagged formats like `<b>[000]</b>` are valid in PlantUML and we
    // support them: the numeric placeholder (`0+` or `#+`) is found inside the
    // template and replaced while HTML tags pass through unchanged.
    if fmt.contains('"') {
        return Err("autonumber format must not contain an embedded quote".to_string());
    }
    Ok(())
}
