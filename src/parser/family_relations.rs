use super::*;

/// Parse tail-form inline relation style from the RHS of a relation.
///
/// PlantUML supports the form `A --> B #line:red;line.bold;text:blue : label`
/// where the `#...` token immediately follows the target node name (before the
/// colon label, which is already stripped by `split_family_relation_label`).
///
/// This function looks for a trailing `#...` inline-style token on `rhs` and,
/// if found, returns `(clean_rhs, Some(merged_style))`. The extracted style is
/// merged into the existing bracket-style (from `[#color]` within the arrow).
///
/// Spec: PlantUML Language Reference 3.36.
pub(crate) fn parse_rhs_inline_relation_style(
    rhs: &str,
    existing: &mut ParsedFamilyRelationStyle,
) -> String {
    let trimmed = rhs.trim();
    // Look for the last `#` that is preceded by whitespace (or is at the start),
    // meaning it starts a tail style token rather than being part of the node name.
    let mut hash_pos: Option<usize> = None;
    let mut prev_was_space = true;
    for (idx, ch) in trimmed.char_indices() {
        if ch == '#' && prev_was_space {
            hash_pos = Some(idx);
        }
        prev_was_space = ch.is_ascii_whitespace();
    }
    let Some(hpos) = hash_pos else {
        return trimmed.to_string();
    };
    let candidate = &trimmed[hpos..];
    // The token must consist of `#` followed by valid inline-style chars only.
    let token_len = candidate
        .char_indices()
        .take_while(|(_, ch)| {
            ch.is_ascii_alphanumeric() || matches!(ch, '#' | '_' | '-' | ':' | ';' | '.')
        })
        .map(|(i, ch)| i + ch.len_utf8())
        .last()
        .unwrap_or(0);
    if token_len == 0 {
        return trimmed.to_string();
    }
    let token = &candidate[..token_len];
    // After the token there must be nothing (no trailing node-name chars).
    let after = candidate[token_len..].trim();
    if !after.is_empty() {
        return trimmed.to_string();
    }
    // Now parse the token into style components and merge into `existing`.
    let mut found_any = false;
    for (idx, raw_part) in token.trim_start_matches('#').split(';').enumerate() {
        let part = raw_part.trim();
        if part.is_empty() {
            continue;
        }
        let lower = part.to_ascii_lowercase();
        let lower_stripped = lower
            .strip_prefix("line.")
            .or_else(|| lower.strip_prefix("line:"))
            .unwrap_or(lower.as_str());
        if matches!(lower_stripped, "dashed" | "dotted" | "dash" | "dot") {
            existing.dashed = true;
            found_any = true;
        } else if matches!(lower_stripped, "bold" | "thick") {
            existing.thickness = Some(existing.thickness.unwrap_or(3).max(3));
            found_any = true;
        } else if lower_stripped == "thin" {
            existing.thickness = Some(1);
            found_any = true;
        } else if lower_stripped == "hidden" {
            existing.hidden = true;
            found_any = true;
        } else if lower_stripped == "plain" {
            existing.dashed = false;
            existing.thickness = None;
            found_any = true;
        } else if let Some(color_str) = lower_stripped
            .strip_prefix("back:")
            .or_else(|| lower_stripped.strip_prefix("color:"))
        {
            // background/fill color on a relation: treat as line color
            if let Some(color) = crate::theme::color::parse_relation_color_token(color_str) {
                existing.line_color = Some(color);
                found_any = true;
            }
        } else if lower_stripped.starts_with("text:") {
            // text: color — accepted as no-op for now (relation labels don't yet have per-label color)
            found_any = true;
        } else {
            // `lower_stripped` might be a color name (e.g. `line:green` → stripped to `green`).
            // Also handle first part as a bare color: `#red` or `#FF0000`.
            let hex_prefixed_stripped = format!("#{lower_stripped}");
            let color_opt = crate::theme::color::parse_relation_color_token(lower_stripped)
                .or_else(|| crate::theme::color::parse_relation_color_token(&hex_prefixed_stripped))
                .or_else(|| {
                    if idx == 0 {
                        let hex_prefixed = format!("#{part}");
                        crate::theme::color::parse_relation_color_token(part).or_else(|| {
                            crate::theme::color::parse_relation_color_token(&hex_prefixed)
                        })
                    } else {
                        None
                    }
                });
            if let Some(color) = color_opt {
                existing.line_color = Some(color);
                found_any = true;
            }
        }
    }
    if found_any {
        trimmed[..hpos].trim_end().to_string()
    } else {
        trimmed.to_string()
    }
}

/// Pre-process a relation line by extracting and removing any tail-form inline
/// style token (`#color`, `#line:red;line.bold`) that appears after the RHS
/// node name.
///
/// Returns `(cleaned_line, extracted_style)`. The cleaned line has the style
/// token removed so that `split_family_relation_label` does not accidentally
/// split on the `:` inside `#line:color`.
///
/// The style token must:
/// - Start with `#` preceded by whitespace
/// - Consist entirely of `[A-Za-z0-9#:;._-]` chars (no spaces)
/// - Be followed by optional whitespace, then end-of-line or ` : <label>`
///   (i.e., it cannot be in the middle of the line)
pub(crate) fn pre_strip_inline_relation_style(
    line: &str,
) -> (String, Option<ParsedFamilyRelationStyle>) {
    let trimmed = line.trim();
    // Scan backwards from end (or from the ` : label` boundary) to find a `#` token.
    // We look for the pattern: `... <rhs_ident> <WS> #<inline_style_chars> [<WS> : <label>]`
    // Strategy: find the last `#` preceded by whitespace. Check the token after it.
    // Then verify the token is immediately followed by end-of-string or ` :` (label boundary).

    // First, find the optional label split point ` : ` or ` :`
    // We must be careful: the real label colon is ` :` where the space precedes it.
    // Our inline style contains `:` without a space before it (e.g., `#line:red`).
    // Look for the LAST ` : ` or trailing ` :` that is NOT inside the style token.
    // Since we don't know where the style token ends yet, we need a different approach.

    // Approach: scan for `#` tokens preceded by whitespace. For each candidate,
    // verify it contains no spaces (valid style token chars only).
    let mut hash_candidates: Vec<usize> = Vec::new();
    let mut prev_was_space = false;
    let mut in_quote = false;
    for (idx, ch) in trimmed.char_indices() {
        if ch == '"' {
            in_quote = !in_quote;
        }
        if !in_quote && ch == '#' && (idx == 0 || prev_was_space) {
            hash_candidates.push(idx);
        }
        prev_was_space = !in_quote && ch.is_ascii_whitespace();
    }

    // Process candidates from last to first (rightmost wins)
    for hpos in hash_candidates.into_iter().rev() {
        let candidate = &trimmed[hpos..];
        let token_len = candidate
            .char_indices()
            .take_while(|(_, ch)| {
                ch.is_ascii_alphanumeric() || matches!(ch, '#' | '_' | '-' | ':' | ';' | '.')
            })
            .map(|(i, ch)| i + ch.len_utf8())
            .last()
            .unwrap_or(0);
        if token_len == 0 {
            continue;
        }
        let token = &candidate[..token_len];
        let after_token = candidate[token_len..].trim_start();
        // After the token there must be nothing, or `: <label>` (colon preceded by optional space)
        let is_at_end = after_token.is_empty();
        let is_before_label = after_token.starts_with(':') && {
            let rest = after_token[1..].trim();
            !rest.is_empty() && !suffix_has_family_relation_arrow(rest)
        };
        if !is_at_end && !is_before_label {
            continue;
        }
        // Parse the style token
        let mut style = ParsedFamilyRelationStyle::default();
        let mut found_any = false;
        for (idx2, raw_part) in token.trim_start_matches('#').split(';').enumerate() {
            let part = raw_part.trim();
            if part.is_empty() {
                continue;
            }
            let lower = part.to_ascii_lowercase();
            let lower_stripped = lower
                .strip_prefix("line.")
                .or_else(|| lower.strip_prefix("line:"))
                .unwrap_or(lower.as_str());
            if matches!(lower_stripped, "dashed" | "dotted" | "dash" | "dot") {
                style.dashed = true;
                found_any = true;
            } else if matches!(lower_stripped, "bold" | "thick") {
                style.thickness = Some(style.thickness.unwrap_or(3).max(3));
                found_any = true;
            } else if lower_stripped == "thin" {
                style.thickness = Some(1);
                found_any = true;
            } else if lower_stripped == "hidden" {
                style.hidden = true;
                found_any = true;
            } else if lower_stripped == "plain" {
                style.dashed = false;
                style.thickness = None;
                found_any = true;
            } else if let Some(color_str) = lower_stripped
                .strip_prefix("back:")
                .or_else(|| lower_stripped.strip_prefix("color:"))
            {
                if let Some(color) = crate::theme::color::parse_relation_color_token(color_str) {
                    style.line_color = Some(color);
                    found_any = true;
                }
            } else if lower_stripped.starts_with("text:") {
                // text: color — accepted as no-op
                found_any = true;
            } else {
                // `lower_stripped` might itself be a color name (e.g. `line:green` → `green`)
                // Try it as a color before falling back to the raw `part`.
                let hex_prefixed_stripped = format!("#{lower_stripped}");
                let color_opt = crate::theme::color::parse_relation_color_token(lower_stripped)
                    .or_else(|| {
                        crate::theme::color::parse_relation_color_token(&hex_prefixed_stripped)
                    })
                    .or_else(|| {
                        if idx2 == 0 {
                            let hex_prefixed = format!("#{part}");
                            crate::theme::color::parse_relation_color_token(part).or_else(|| {
                                crate::theme::color::parse_relation_color_token(&hex_prefixed)
                            })
                        } else {
                            None
                        }
                    });
                if let Some(color) = color_opt {
                    style.line_color = Some(color);
                    found_any = true;
                }
            }
        }
        if !found_any {
            continue;
        }
        // Build the cleaned line: everything before the token, then the after-token suffix
        let before = trimmed[..hpos].trim_end();
        let cleaned = if after_token.is_empty() {
            before.to_string()
        } else {
            // after_token starts with `: label`
            format!("{before} {after_token}")
        };
        return (cleaned, Some(style));
    }

    (trimmed.to_string(), None)
}

pub(crate) fn parse_family_relation(
    line: &str,
    family: Option<DiagramKind>,
) -> Option<Vec<StatementKind>> {
    // When family is still unknown, only C4 legacy relation macros are accepted.
    // This supports valid C4 inputs where relations appear before declarations.
    if family.is_none() {
        return parse_c4_legacy_family_relation(line, None);
    }

    match family {
        Some(DiagramKind::Class)
        | Some(DiagramKind::Object)
        | Some(DiagramKind::UseCase)
        | Some(DiagramKind::Salt)
        | Some(DiagramKind::MindMap)
        | Some(DiagramKind::Wbs)
        | Some(DiagramKind::Component)
        | Some(DiagramKind::Deployment) => {}
        _ => return None,
    }

    if let Some(kinds) = parse_c4_legacy_family_relation(line, family) {
        return Some(kinds);
    }

    // Pre-strip tail-form inline relation style before label splitting.
    // PlantUML 3.36: `A --> B #line:red;line.bold : label`
    // Without this pre-pass, the `:` inside `#line:color` confuses the label splitter.
    let (preprocessed_line, pre_style) = pre_strip_inline_relation_style(line);
    let line = preprocessed_line.as_str();

    let (core, raw_label) = split_family_relation_label(line);
    let (lhs, arrow, mut relation_style, rhs) = split_family_arrow_styled(core)?;
    if !arrow.contains('-') && !arrow.contains('.') {
        return None;
    }
    // Merge pre-stripped style into relation_style
    if let Some(pre) = pre_style {
        if pre.line_color.is_some() {
            relation_style.line_color = pre.line_color;
        }
        if pre.dashed {
            relation_style.dashed = pre.dashed;
        }
        if pre.hidden {
            relation_style.hidden = pre.hidden;
        }
        if pre.thickness.is_some() {
            relation_style.thickness = pre.thickness;
        }
    }
    // Strip any remaining tail-form inline relation style from the RHS.
    let rhs = parse_rhs_inline_relation_style(rhs, &mut relation_style);
    let rhs = rhs.as_str();
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
    // Component/Deployment diagrams use `[Name]` bracket syntax for nodes, so
    // `looks_like_virtual_endpoint_syntax` (which rejects any `[`/`]`) must be
    // skipped for those families.
    let is_bracket_family = matches!(
        family,
        Some(DiagramKind::Component) | Some(DiagramKind::Deployment)
    );
    if normalize_virtual_endpoint(&lhs_core).is_some()
        || normalize_virtual_endpoint(&rhs_core).is_some()
        || (!is_bracket_family && looks_like_virtual_endpoint_syntax(&lhs_core))
        || (!is_bracket_family && looks_like_virtual_endpoint_syntax(&rhs_core))
    {
        return None;
    }
    let from = clean_bracketed_ident(&lhs_core);
    let to = clean_bracketed_ident(&rhs_core);
    if from.is_empty() || to.is_empty() {
        return None;
    }
    Some(vec![StatementKind::FamilyRelation(FamilyRelation {
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
    })])
}

pub(crate) fn parse_association_class_relation(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    let after_open = trimmed.strip_prefix('(')?;
    let close = after_open.find(')')?;
    let pair = &after_open[..close];
    let (left_raw, right_raw) = pair.split_once(',')?;
    let left = clean_ident(left_raw);
    let right = clean_ident(right_raw);
    if left.is_empty() || right.is_empty() {
        return None;
    }
    let rest = after_open[close + 1..].trim();
    let arrow_len = family_arrow_token_len(rest)?;
    let arrow = normalize_family_arrow_token(&rest[..arrow_len]);
    let association_raw = rest[arrow_len..].trim();
    let association = clean_bracketed_ident(association_raw);
    if association.is_empty() {
        return None;
    }
    Some(StatementKind::AssociationClass {
        left,
        right,
        association,
        arrow,
    })
}

pub(crate) fn association_class_family_relations(
    left: String,
    right: String,
    association: String,
    arrow: String,
) -> Vec<FamilyRelation> {
    vec![
        FamilyRelation {
            from: left.clone(),
            to: right.clone(),
            arrow,
            label: None,
            stereotype: None,
            left_cardinality: None,
            right_cardinality: None,
            left_role: None,
            right_role: None,
            line_color: None,
            dashed: false,
            hidden: false,
            thickness: None,
            direction: None,
            left_lollipop: false,
            right_lollipop: false,
        },
        FamilyRelation {
            from: association.clone(),
            to: left,
            arrow: "..".to_string(),
            label: None,
            stereotype: None,
            left_cardinality: None,
            right_cardinality: None,
            left_role: None,
            right_role: None,
            line_color: None,
            dashed: false,
            hidden: false,
            thickness: None,
            direction: None,
            left_lollipop: false,
            right_lollipop: false,
        },
        FamilyRelation {
            from: association,
            to: right,
            arrow: "..".to_string(),
            label: None,
            stereotype: None,
            left_cardinality: None,
            right_cardinality: None,
            left_role: None,
            right_role: None,
            line_color: None,
            dashed: false,
            hidden: false,
            thickness: None,
            direction: None,
            left_lollipop: false,
            right_lollipop: false,
        },
    ]
}

pub(crate) fn strip_lollipop_endpoint(side: &str) -> (String, bool) {
    let trimmed = side.trim();
    if let Some(rest) = trimmed.strip_prefix("()") {
        return (rest.trim_start().to_string(), true);
    }
    if let Some(rest) = trimmed.strip_suffix("()") {
        return (rest.trim_end().to_string(), true);
    }
    (trimmed.to_string(), false)
}

pub(crate) fn split_relation_label_stereotype(
    label: Option<String>,
) -> (Option<String>, Option<String>) {
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

pub(crate) fn split_relation_trailing_stereotype(side: &str) -> (&str, Option<String>) {
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

pub(crate) fn parse_leading_stereotype(s: &str) -> Option<(String, &str)> {
    let rest = s.trim_start().strip_prefix("<<")?;
    let close = rest.find(">>")?;
    let value = rest[..close].trim();
    if value.is_empty() {
        return None;
    }
    Some((value.to_string(), &rest[close + 2..]))
}

pub(crate) fn parse_family_member_row(
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

pub(crate) fn parse_family_visibility_control(
    line: &str,
    family: Option<DiagramKind>,
) -> Option<StatementKind> {
    let lower = line.to_ascii_lowercase();
    // `hide @unlinked` and `remove @unlinked` are component/deployment-specific
    // but may appear before the diagram family is detected (family == None).
    // Handle them before the family gate so they are not misinterpreted.
    if lower == "hide @unlinked" || lower == "remove @unlinked" {
        let is_component_family = matches!(
            family,
            None | Some(DiagramKind::Component | DiagramKind::Deployment)
        );
        if is_component_family {
            let keyword = if lower.starts_with("hide") {
                "hide @unlinked"
            } else {
                "remove @unlinked"
            };
            return Some(StatementKind::HideOption(keyword.to_string()));
        }
    }
    if lower == "hide empty description" && matches!(family, None | Some(DiagramKind::State)) {
        return Some(StatementKind::HideOption("empty description".to_string()));
    }
    if family.is_none() {
        for keyword in ["hide", "remove", "restore"] {
            let Some(rest) = lower.strip_prefix(&format!("{keyword} ")) else {
                continue;
            };
            let rest = rest.trim();
            if rest == "*" || rest.starts_with('$') {
                return Some(StatementKind::HideOption(format!("{keyword} node {rest}")));
            }
        }
    }
    if !matches!(
        family,
        Some(
            DiagramKind::Class
                | DiagramKind::Object
                | DiagramKind::UseCase
                | DiagramKind::Component
                | DiagramKind::Deployment
        )
    ) {
        return None;
    }
    if lower.starts_with("hide ") {
        let rest = line.strip_prefix("hide ").unwrap_or("").trim();
        if rest.eq_ignore_ascii_case("@unlinked") {
            return Some(StatementKind::HideOption("hide @unlinked".to_string()));
        }
        if !rest.is_empty() {
            return Some(StatementKind::HideOption(format!("hide node {rest}")));
        }
    }
    if lower.starts_with("remove ") {
        let rest = line.strip_prefix("remove ").unwrap_or("").trim();
        if rest.eq_ignore_ascii_case("@unlinked") {
            // `remove @unlinked` is synonymous with `hide @unlinked` — both drop
            // all nodes that have no relation edges.
            return Some(StatementKind::HideOption("hide @unlinked".to_string()));
        }
        if !rest.is_empty() {
            return Some(StatementKind::HideOption(format!("remove node {rest}")));
        }
    }
    if lower.starts_with("restore ") {
        let rest = line.strip_prefix("restore ").unwrap_or("").trim();
        if !rest.is_empty() {
            return Some(StatementKind::HideOption(format!("restore node {rest}")));
        }
    }
    if lower.starts_with("show ") {
        let rest = line.strip_prefix("show ").unwrap_or("").trim();
        if !rest.is_empty() {
            return Some(StatementKind::HideOption(format!("show {rest}")));
        }
    }
    None
}

pub(crate) fn parse_relation_side_annotations(
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
