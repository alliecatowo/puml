use super::*;

pub(super) fn parse_family_relation(
    line: &str,
    family: Option<DiagramKind>,
) -> Option<StatementKind> {
    match family {
        Some(DiagramKind::Class)
        | Some(DiagramKind::Object)
        | Some(DiagramKind::UseCase)
        | Some(DiagramKind::Salt)
        | Some(DiagramKind::Component)
        | Some(DiagramKind::Deployment) => {}
        _ => return None,
    }

    let (core, raw_label) = split_family_relation_label(line);
    let (lhs, arrow, relation_style, rhs) = split_family_arrow_styled(core)?;
    if !arrow.contains('-') && !arrow.contains('.') {
        return None;
    }
    let (rhs, trailing_stereotype) = split_relation_trailing_stereotype(rhs);
    let (label, label_stereotype) = split_relation_label_stereotype(raw_label);
    // Strip surrounding double-quotes from labels produced by preprocessor macro
    // expansion (e.g. C4 Rel() emits `from --> to : "Label"` with quotes intact).
    let label = label.map(|l| {
        let t = l.trim().to_string();
        if t.starts_with('"') && t.ends_with('"') && t.len() >= 2 {
            t[1..t.len() - 1].to_string()
        } else {
            t
        }
    });
    let (lhs_core, left_cardinality, left_role) = parse_relation_side_annotations(lhs, true);
    let (rhs_core, right_cardinality, right_role) = parse_relation_side_annotations(rhs, false);
    let (lhs_core, left_lollipop) = strip_lollipop_endpoint(&lhs_core);
    let (rhs_core, right_lollipop) = strip_lollipop_endpoint(&rhs_core);
    if normalize_virtual_endpoint(&lhs_core).is_some()
        || normalize_virtual_endpoint(&rhs_core).is_some()
        || looks_like_virtual_endpoint_syntax(&lhs_core)
        || looks_like_virtual_endpoint_syntax(&rhs_core)
    {
        return None;
    }
    let from = clean_bracketed_ident(&lhs_core);
    let to = clean_bracketed_ident(&rhs_core);
    if from.is_empty() || to.is_empty() {
        return None;
    }
    Some(StatementKind::FamilyRelation(FamilyRelation {
        from,
        to,
        arrow,
        label,
        stereotype: label_stereotype.or(trailing_stereotype),
        left_cardinality,
        right_cardinality,
        left_role,
        right_role,
        line_color: relation_style.line_color,
        dashed: relation_style.dashed,
        hidden: relation_style.hidden,
        thickness: relation_style.thickness,
        direction: relation_style.direction,
        left_lollipop,
        right_lollipop,
    }))
}

fn strip_lollipop_endpoint(side: &str) -> (String, bool) {
    let trimmed = side.trim();
    if let Some(rest) = trimmed.strip_prefix("()") {
        return (rest.trim_start().to_string(), true);
    }
    if let Some(rest) = trimmed.strip_suffix("()") {
        return (rest.trim_end().to_string(), true);
    }
    (trimmed.to_string(), false)
}

fn split_relation_label_stereotype(label: Option<String>) -> (Option<String>, Option<String>) {
    let Some(label) = label else {
        return (None, None);
    };
    let trimmed = label.trim();
    if let Some((stereotype, rest)) = parse_leading_stereotype(trimmed) {
        let label = rest.trim();
        return (
            (!label.is_empty()).then(|| label.to_string()),
            Some(stereotype),
        );
    }
    (Some(label), None)
}

fn split_relation_trailing_stereotype(side: &str) -> (&str, Option<String>) {
    let trimmed = side.trim();
    let Some(open) = trimmed.rfind("<<") else {
        return (side, None);
    };
    let before = trimmed[..open].trim_end();
    let tail = trimmed[open..].trim();
    if before.is_empty() {
        return (side, None);
    }
    if let Some((stereotype, rest)) = parse_leading_stereotype(tail) {
        if rest.trim().is_empty() {
            return (before, Some(stereotype));
        }
    }
    (side, None)
}

fn parse_leading_stereotype(s: &str) -> Option<(String, &str)> {
    let rest = s.trim_start().strip_prefix("<<")?;
    let close = rest.find(">>")?;
    let value = rest[..close].trim();
    if value.is_empty() {
        return None;
    }
    Some((value.to_string(), &rest[close + 2..]))
}

pub(super) fn parse_family_member_row(
    line: &str,
    family: Option<DiagramKind>,
) -> Option<StatementKind> {
    let family = match family {
        Some(DiagramKind::Class | DiagramKind::Object | DiagramKind::UseCase) => family?,
        _ => return None,
    };
    if split_family_arrow(line).is_some() {
        return None;
    }
    let (owner, member) = line.split_once(':')?;
    if owner.contains("--") || owner.contains("..") || owner.contains("->") || owner.contains("<-")
    {
        return None;
    }
    let owner = clean_bracketed_ident(owner);
    let member = member.trim();
    if owner.is_empty() || member.is_empty() {
        return None;
    }
    let members = vec![parse_class_member(member)];
    Some(match family {
        DiagramKind::Object => StatementKind::ObjectDecl(ObjectDecl {
            name: owner,
            alias: None,
            members,
        }),
        DiagramKind::UseCase => StatementKind::UseCaseDecl(UseCaseDecl {
            name: owner,
            alias: None,
            members,
        }),
        _ => StatementKind::ClassDecl(ClassDecl {
            name: owner,
            alias: None,
            members,
        }),
    })
}

fn parse_relation_side_annotations(
    side: &str,
    is_left: bool,
) -> (String, Option<String>, Option<String>) {
    let trimmed = side.trim();
    if trimmed.is_empty() {
        return (String::new(), None, None);
    }

    let mut rem = trimmed.to_string();
    let mut cardinality: Option<String> = None;
    let mut role: Option<String> = None;

    if is_left {
        loop {
            let t = rem.trim_end();
            if t.ends_with(']') {
                if let Some(start_bracket) = t.rfind('[') {
                    let value = t[start_bracket + 1..t.len() - 1].trim();
                    let endpoint = t[..start_bracket].trim_end();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if role.is_none() {
                            role = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            if let Some(stripped) = t.strip_suffix('"') {
                if let Some(start_quote) = stripped.rfind('"') {
                    let value = stripped[start_quote + 1..].trim();
                    let endpoint = t[..start_quote].trim_end();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if cardinality.is_none() {
                            cardinality = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            if let Some(colon) = t.rfind(" :") {
                let value = t[colon + 2..].trim();
                let endpoint = t[..colon].trim_end();
                if !value.is_empty() && !endpoint.is_empty() {
                    if role.is_none() {
                        role = Some(value.to_string());
                    }
                    rem = endpoint.to_string();
                    continue;
                }
            }
            break;
        }
    } else {
        loop {
            let t = rem.trim_start();
            if let Some(rest) = t.strip_prefix('"') {
                if let Some(end_quote_rel) = rest.find('"') {
                    let value = rest[..end_quote_rel].trim();
                    let endpoint = rest[end_quote_rel + 1..].trim_start();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if cardinality.is_none() {
                            cardinality = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            if let Some(rest) = t.strip_prefix('[') {
                if let Some(end_bracket_rel) = rest.find(']') {
                    let value = rest[..end_bracket_rel].trim();
                    let endpoint = rest[end_bracket_rel + 1..].trim_start();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if role.is_none() {
                            role = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            if let Some(rest) = t.strip_prefix(':') {
                let value_len = rest
                    .char_indices()
                    .take_while(|(_, ch)| !ch.is_whitespace())
                    .map(|(idx, ch)| idx + ch.len_utf8())
                    .last()
                    .unwrap_or(0);
                if value_len > 0 {
                    let value = rest[..value_len].trim();
                    let endpoint = rest[value_len..].trim_start();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if role.is_none() {
                            role = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            break;
        }
    }

    (rem.trim().to_string(), cardinality, role)
}

#[derive(Debug, Clone, Default)]
pub(super) struct ParsedFamilyRelationStyle {
    pub(super) line_color: Option<String>,
    pub(super) dashed: bool,
    pub(super) hidden: bool,
    pub(super) thickness: Option<u8>,
    pub(super) direction: Option<String>,
}

pub(super) fn split_family_arrow(core: &str) -> Option<(&str, String, &str)> {
    split_family_arrow_styled(core).map(|(lhs, arrow, _, rhs)| (lhs, arrow, rhs))
}

pub(super) fn split_family_arrow_styled(
    core: &str,
) -> Option<(&str, String, ParsedFamilyRelationStyle, &str)> {
    let mut in_quote = false;
    for (idx, ch) in core.char_indices() {
        if ch == '"' {
            in_quote = !in_quote;
            continue;
        }
        if in_quote {
            continue;
        }
        if !matches!(ch, '-' | '.' | '<' | '*' | 'o' | '+' | '|') {
            continue;
        }
        let rest = &core[idx..];
        let Some(len) = family_arrow_token_len(rest) else {
            continue;
        };
        if len == 1 {
            continue;
        }
        let lhs = core[..idx].trim();
        let rhs = core[idx + len..].trim();
        if lhs.is_empty() || rhs.is_empty() {
            continue;
        }
        let raw_arrow = &rest[..len];
        let arrow = normalize_family_arrow_token(raw_arrow);
        if arrow.is_empty() {
            continue;
        }
        let mut relation_style = parse_family_relation_style(raw_arrow);
        relation_style.direction = parse_family_relation_direction(raw_arrow);
        return Some((lhs, arrow, relation_style, rhs));
    }
    None
}

fn parse_family_relation_style(raw_arrow: &str) -> ParsedFamilyRelationStyle {
    let mut style = ParsedFamilyRelationStyle::default();
    let mut rest = raw_arrow;
    while let Some(open) = rest.find('[') {
        let after_open = &rest[open + 1..];
        let Some(close) = after_open.find(']') else {
            break;
        };
        let content = &after_open[..close];
        for part in content.split(|ch: char| ch == ',' || ch == ';' || ch.is_whitespace()) {
            let token = part.trim();
            if token.is_empty() {
                continue;
            }
            let lower_raw = token.to_ascii_lowercase();
            let lower = lower_raw
                .strip_prefix("line.")
                .or_else(|| lower_raw.strip_prefix("line:"))
                .unwrap_or(lower_raw.as_str());
            match lower {
                "dashed" | "dotted" | "dash" | "dot" => style.dashed = true,
                "hidden" => style.hidden = true,
                "bold" | "thick" => style.thickness = Some(style.thickness.unwrap_or(3).max(3)),
                "thin" => style.thickness = Some(1),
                _ => {
                    if let Some(value) = lower
                        .strip_prefix("thickness=")
                        .or_else(|| lower.strip_prefix("thickness:"))
                        .or_else(|| lower.strip_prefix("weight="))
                        .or_else(|| lower.strip_prefix("weight:"))
                    {
                        if let Ok(n) = value.trim().parse::<u8>() {
                            style.thickness = Some(n.clamp(1, 8));
                        }
                    } else if let Some(color) = parse_relation_color_token(token) {
                        style.line_color = Some(color);
                    }
                }
            }
        }
        rest = &after_open[close + 1..];
    }
    style
}

fn parse_family_relation_direction(raw_arrow: &str) -> Option<String> {
    let mut cleaned = String::new();
    let mut in_bracket = false;
    for ch in raw_arrow.chars() {
        match ch {
            '[' => in_bracket = true,
            ']' => in_bracket = false,
            _ if !in_bracket => cleaned.push(ch),
            _ => {}
        }
    }
    let lower = cleaned.to_ascii_lowercase();
    for (needle, direction) in [
        ("left", "left"),
        ("right", "right"),
        ("up", "up"),
        ("down", "down"),
        ("l", "left"),
        ("r", "right"),
        ("u", "up"),
        ("d", "down"),
    ] {
        if lower.contains(needle) {
            return Some(direction.to_string());
        }
    }
    None
}

pub(super) fn parse_relation_color_token(token: &str) -> Option<String> {
    let trimmed = token.trim();
    if trimmed.len() == 7
        && trimmed.starts_with('#')
        && trimmed[1..].chars().all(|ch| ch.is_ascii_hexdigit())
    {
        return Some(trimmed.to_ascii_lowercase());
    }
    crate::theme::css3_color_to_hex(trimmed.trim_start_matches('#')).map(str::to_string)
}

fn family_arrow_token_len(s: &str) -> Option<usize> {
    if let Some(len) = directional_family_arrow_token_len(s) {
        return Some(len);
    }

    let len = s
        .char_indices()
        .take_while(|(_, ch)| matches!(ch, '-' | '.' | '<' | '>' | '|' | '*' | 'o' | '+'))
        .map(|(idx, ch)| idx + ch.len_utf8())
        .last()?;
    let token = &s[..len];
    if is_family_arrow_token(token) {
        Some(len)
    } else {
        None
    }
}

fn directional_family_arrow_token_len(s: &str) -> Option<usize> {
    let dirs = ["left", "right", "up", "down", "l", "r", "u", "d"];
    for prefix_len in 1..=2 {
        let prefix = s.get(..prefix_len)?;
        if !prefix.chars().all(|ch| matches!(ch, '-' | '.')) {
            continue;
        }
        let after_prefix = &s[prefix_len..];
        if let Some(after_directive) = after_prefix.strip_prefix('[') {
            if let Some(close) = after_directive.find(']') {
                let after_with_optional_dir = &after_directive[close + 1..];
                let after = dirs
                    .iter()
                    .find_map(|dir| after_with_optional_dir.strip_prefix(dir))
                    .unwrap_or(after_with_optional_dir);
                let dir_len = after_with_optional_dir.len().saturating_sub(after.len());
                let suffix_len = after
                    .char_indices()
                    .take_while(|(_, ch)| matches!(ch, '-' | '.' | '<' | '>' | '|'))
                    .map(|(idx, ch)| idx + ch.len_utf8())
                    .last()
                    .unwrap_or(0);
                if suffix_len > 0 {
                    return Some(prefix_len + close + 2 + dir_len + suffix_len);
                }
            }
        }
        for dir in dirs {
            if let Some(after_dir) = after_prefix.strip_prefix(dir) {
                let suffix_len = after_dir
                    .char_indices()
                    .take_while(|(_, ch)| matches!(ch, '-' | '.' | '<' | '>' | '|'))
                    .map(|(idx, ch)| idx + ch.len_utf8())
                    .last()
                    .unwrap_or(0);
                if suffix_len > 0 {
                    return Some(prefix_len + dir.len() + suffix_len);
                }
            }
        }
    }
    None
}

fn is_family_arrow_token(token: &str) -> bool {
    token.contains('-') || token.contains('<') || token.contains('>') || token.contains("..")
}

fn normalize_family_arrow_token(token: &str) -> String {
    let mut out = String::new();
    let mut chars = token.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '[' {
            // Strip bracketed direction/color annotations like [left], [#red]
            for next in chars.by_ref() {
                if next == ']' {
                    break;
                }
            }
            continue;
        }
        // 'o' is a valid arrow-head marker char (aggregation / hollow diamond).
        // All other alphabetic runs are direction/color keywords — strip them.
        if ch.is_ascii_alphabetic() && ch != 'o' {
            continue;
        }
        out.push(ch);
    }
    out
}

pub(super) fn clean_bracketed_ident(s: &str) -> String {
    let trimmed = s.trim();
    // Preserve special state markers like [*] verbatim.
    if trimmed == "[*]" || trimmed == "[H]" || trimmed == "[H*]" {
        return trimmed.to_string();
    }
    // Allow `[Name]` shorthand: strip the surrounding brackets if balanced and no interior bracket.
    if let Some(inner) = trimmed.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        if !inner.contains('[') && !inner.contains(']') && !inner.is_empty() {
            return clean_ident(inner);
        }
    }
    // Strip `()` interface-style prefix `() Name`.
    if let Some(rest) = trimmed.strip_prefix("()") {
        return clean_ident(rest.trim());
    }
    clean_ident(trimmed)
}
