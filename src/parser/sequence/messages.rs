use super::*;

pub(super) fn parse_message(line: &str) -> Option<StatementKind> {
    let (line, parallel) = split_parallel_message_prefix(line);
    let (core, label) = split_message_label(line);
    let (lhs_raw, arrow, rhs_raw) = split_arrow(core)?;
    let mut style = parse_arrow_style(arrow);
    style.parallel = parallel;
    let parsed_arrow = parse_arrow(arrow)?;
    let (from_id_raw, from_modifier) = split_lifecycle_modifier(lhs_raw);
    let (to_id_raw, to_modifier) = split_lifecycle_modifier(rhs_raw);

    let from = if let Some(v) = normalize_virtual_endpoint(from_id_raw) {
        v
    } else {
        if looks_like_virtual_endpoint_syntax(from_id_raw) {
            return None;
        }
        clean_ident(from_id_raw)
    };
    let to = if let Some(v) = normalize_virtual_endpoint(to_id_raw) {
        v
    } else {
        if looks_like_virtual_endpoint_syntax(to_id_raw) {
            return None;
        }
        clean_ident(to_id_raw)
    };

    if from.is_empty() || to.is_empty() {
        return None;
    }

    let mut arrow_encoded = parsed_arrow.to_string();
    if let Some(modifier) = from_modifier {
        arrow_encoded.push_str("@L");
        arrow_encoded.push_str(modifier);
    }
    if let Some(modifier) = to_modifier {
        arrow_encoded.push_str("@R");
        arrow_encoded.push_str(modifier);
    }

    let from_virtual = ast_virtual_endpoint_from_id(&from, true);
    let to_virtual = ast_virtual_endpoint_from_id(&to, false);
    Some(StatementKind::Message(Message {
        from,
        to,
        arrow: arrow_encoded,
        label,
        style,
        from_virtual,
        to_virtual,
    }))
}

fn split_parallel_message_prefix(line: &str) -> (&str, bool) {
    let trimmed = line.trim_start();
    if let Some(rest) = trimmed.strip_prefix('&') {
        let rest = rest.trim_start();
        if !rest.is_empty() {
            return (rest, true);
        }
    }
    (line, false)
}

fn parse_arrow_style(arrow: &str) -> MessageStyle {
    let mut style = MessageStyle::default();
    if strip_sequence_arrow_brackets(arrow).contains('.') {
        style.dotted = true;
    }
    let mut chars = arrow.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '[' {
            continue;
        }
        let mut body = String::new();
        for inner in chars.by_ref() {
            if inner == ']' {
                break;
            }
            body.push(inner);
        }
        for token in body
            .split([',', ';'])
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            let lower = token.to_ascii_lowercase();
            match lower.as_str() {
                "hidden" | "line.hidden" => style.hidden = true,
                "dashed" | "line.dashed" => style.dashed = true,
                "dotted" | "line.dotted" => style.dotted = true,
                "bold" | "thick" | "line.bold" | "line.thick" => style.thickness = Some(3),
                "thin" | "line.thin" => style.thickness = Some(1),
                _ if token.starts_with('#')
                    && matches!(token.len(), 4 | 5 | 7 | 9)
                    && token[1..].bytes().all(|b| b.is_ascii_hexdigit()) =>
                {
                    style.color = Some(format!("#{}", token[1..].to_ascii_lowercase()));
                }
                _ if token.starts_with('#')
                    && token[1..].bytes().all(|b| b.is_ascii_alphabetic()) =>
                {
                    style.color = Some(token[1..].to_ascii_lowercase());
                }
                _ if token.bytes().all(|b| b.is_ascii_alphabetic()) => {
                    style.color = Some(lower);
                }
                _ => {
                    if let Some(value) = lower
                        .strip_prefix("thickness=")
                        .or_else(|| lower.strip_prefix("thickness:"))
                        .or_else(|| lower.strip_prefix("thickness "))
                        .or_else(|| lower.strip_prefix("line.thickness="))
                        .or_else(|| lower.strip_prefix("line.thickness:"))
                        .or_else(|| lower.strip_prefix("line.thickness "))
                    {
                        if let Ok(n) = value.trim().parse::<u8>() {
                            style.thickness = Some(n.clamp(1, 8));
                        }
                    }
                }
            }
        }
    }
    style
}

fn ast_virtual_endpoint_from_id(id: &str, is_from: bool) -> Option<VirtualEndpoint> {
    let (side, kind) = match id {
        "[" => (VirtualEndpointSide::Left, VirtualEndpointKind::Plain),
        "]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Plain),
        "[o" => (VirtualEndpointSide::Left, VirtualEndpointKind::Circle),
        "o]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Circle),
        "[x" => (VirtualEndpointSide::Left, VirtualEndpointKind::Cross),
        "x]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Cross),
        "[*]" => (
            if is_from {
                VirtualEndpointSide::Left
            } else {
                VirtualEndpointSide::Right
            },
            VirtualEndpointKind::Filled,
        ),
        _ => return None,
    };
    Some(VirtualEndpoint { side, kind })
}
