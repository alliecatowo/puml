use super::*;
pub(crate) fn parse_sprite_statement(
    lines: &[(&str, Span)],
    start_idx: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    let Some(rest) = line.strip_prefix("sprite ") else {
        return Ok(None);
    };
    let mut parts = rest.split_whitespace();
    let Some(raw_name) = parts.next() else {
        return Err(
            Diagnostic::error("[E_SPRITE_INVALID] sprite name is missing")
                .with_span(lines[start_idx].1),
        );
    };
    let after_name = rest[raw_name.len()..].trim();
    if after_name.is_empty() {
        return Err(
            Diagnostic::error("[E_SPRITE_INVALID] sprite body is missing")
                .with_span(lines[start_idx].1),
        );
    }

    if after_name.starts_with("jar:") {
        let sprite = crate::sprites::builtin_sprite(raw_name, after_name);
        return Ok(Some((StatementKind::SpriteDef(sprite), start_idx)));
    }

    if after_name.to_ascii_lowercase().starts_with("<svg") {
        let mut svg_lines = vec![after_name.to_string()];
        let mut end_idx = start_idx;
        if !after_name.to_ascii_lowercase().contains("</svg>") {
            let mut found = false;
            for (idx, (raw, _span)) in lines.iter().enumerate().skip(start_idx + 1) {
                svg_lines.push((*raw).to_string());
                end_idx = idx;
                if raw.to_ascii_lowercase().contains("</svg>") {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(Diagnostic::error(
                    "[E_SPRITE_INVALID] inline SVG sprite is missing </svg>",
                )
                .with_span(lines[start_idx].1));
            }
        }
        let sprite = crate::sprites::parse_svg_sprite(raw_name, &svg_lines.join("\n"))
            .map_err(|d| d.with_span(lines[start_idx].1))?;
        return Ok(Some((StatementKind::SpriteDef(sprite), end_idx)));
    }

    let (spec, after_spec) = if after_name.starts_with('[') {
        let Some(close) = after_name.find(']') else {
            return Err(
                Diagnostic::error("[E_SPRITE_INVALID] sprite size spec is not closed")
                    .with_span(lines[start_idx].1),
            );
        };
        (&after_name[..=close], after_name[close + 1..].trim())
    } else {
        ("", after_name)
    };
    let parsed_spec = if spec.is_empty() {
        None
    } else {
        Some(
            crate::sprites::parse_sprite_header_spec(spec).ok_or_else(|| {
                Diagnostic::error(format!(
                    "[E_SPRITE_INVALID] invalid sprite size/depth spec `{spec}`"
                ))
                .with_span(lines[start_idx].1)
            })?,
        )
    };

    if let Some(first_payload) = after_spec.strip_prefix('{') {
        let mut rows: Vec<String> = Vec::new();
        let mut end_idx = start_idx;
        let inline_after_open = first_payload.trim();
        if let Some(before_close) = inline_after_open.strip_suffix('}') {
            let compact = before_close.trim();
            if !compact.is_empty() {
                rows.extend(compact.split_whitespace().map(str::to_string));
            }
        } else {
            if !inline_after_open.is_empty() {
                rows.extend(inline_after_open.split_whitespace().map(str::to_string));
            }
            let mut found = false;
            for (idx, (raw, span)) in lines.iter().enumerate().skip(start_idx + 1) {
                let trimmed = strip_inline_plantuml_comment(raw).trim();
                if trimmed == "}" {
                    end_idx = idx;
                    found = true;
                    break;
                }
                if let Some(before_close) = trimmed.strip_suffix('}') {
                    let compact = before_close.trim();
                    if !compact.is_empty() {
                        rows.extend(compact.split_whitespace().map(str::to_string));
                    }
                    end_idx = idx;
                    found = true;
                    break;
                }
                if trimmed.is_empty() {
                    end_idx = idx;
                    continue;
                }
                if trimmed.chars().any(char::is_whitespace) {
                    return Err(Diagnostic::error(
                        "[E_SPRITE_INVALID] sprite rows cannot contain whitespace",
                    )
                    .with_span(*span));
                }
                rows.push(trimmed.to_string());
                end_idx = idx;
            }
            if !found {
                return Err(Diagnostic::error(
                    "[E_SPRITE_INVALID] sprite block is missing closing `}`",
                )
                .with_span(lines[start_idx].1));
            }
        }
        let (width, height, levels, _compressed) = parsed_spec.unwrap_or((0, 0, 16, false));
        let sprite = crate::sprites::parse_hex_grid_sprite(
            raw_name,
            (width > 0).then_some(width),
            (height > 0).then_some(height),
            levels,
            &rows,
        )
        .map_err(|d| d.with_span(lines[start_idx].1))?;
        return Ok(Some((StatementKind::SpriteDef(sprite), end_idx)));
    }

    if let Some((width, height, levels, compressed)) = parsed_spec {
        if after_spec.is_empty() {
            return Err(
                Diagnostic::error("[E_SPRITE_INVALID] encoded sprite payload is missing")
                    .with_span(lines[start_idx].1),
            );
        }
        let sprite = crate::sprites::parse_packed_sprite(
            raw_name, width, height, levels, compressed, after_spec,
        )
        .map_err(|d| d.with_span(lines[start_idx].1))?;
        return Ok(Some((StatementKind::SpriteDef(sprite), start_idx)));
    }

    Err(Diagnostic::error(format!(
        "[E_SPRITE_INVALID] unsupported sprite syntax `{line}`"
    ))
    .with_span(lines[start_idx].1))
}
