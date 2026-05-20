use crate::ast::{DiagramKind, Document, StatementKind};
use crate::diagnostic::Diagnostic;
use crate::model::{
    ChenAttr, ChenAttrKind, ChenDocument, ChenEntity, ChenRelParticipant, ChenRelationship,
};

/// Parse the raw-body lines of a `@startchen` ... `@endchen` block into a
/// [`ChenDocument`].
///
/// Grammar (line-oriented, case-insensitive keywords):
///
/// ```text
/// entity NAME [weak] {
///   [key]           ATTR_NAME
///   [multivalued]   ATTR_NAME
///   [derived]       ATTR_NAME
///   ATTR_NAME
/// }
///
/// relationship NAME [identifying] {
///   ENTITY_NAME -> ENTITY_NAME [CARD:CARD]
///   [key] ATTR_NAME
/// }
/// ```
///
/// Cardinality tokens: `1`, `N`, `M`, `0..1`, `0..N` (case-insensitive).
pub fn normalize_chen(document: Document) -> Result<ChenDocument, Diagnostic> {
    debug_assert_eq!(document.kind, DiagramKind::Chen);

    let mut raw_lines: Vec<String> = Vec::new();
    let mut title: Option<String> = None;

    for stmt in &document.statements {
        match &stmt.kind {
            StatementKind::RawBody(line) => {
                let trimmed = line.trim();
                // Handle `title ...` inside the block.
                if let Some(rest) = trimmed.strip_prefix("title ") {
                    title = Some(rest.trim().to_string());
                } else {
                    raw_lines.push(trimmed.to_string());
                }
            }
            StatementKind::Title(t) => title = Some(t.clone()),
            _ => {}
        }
    }

    let mut entities: Vec<ChenEntity> = Vec::new();
    let mut relationships: Vec<ChenRelationship> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    let mut i = 0usize;
    while i < raw_lines.len() {
        let line = raw_lines[i].trim();
        if line.is_empty() || line.starts_with('\'') || line.starts_with("//") {
            i += 1;
            continue;
        }

        if let Some(entity) = try_parse_entity_start(line) {
            let (name, is_weak, has_block) = entity;
            let mut attrs = Vec::new();
            if has_block {
                i += 1;
                while i < raw_lines.len() {
                    let inner = raw_lines[i].trim();
                    if inner == "}" {
                        break;
                    }
                    if !inner.is_empty() {
                        if let Some(attr) = parse_attribute_line(inner) {
                            attrs.push(attr);
                        } else {
                            warnings.push(Diagnostic::warning(format!(
                                "[W_CHEN_ATTR] unrecognised attribute line: `{inner}`"
                            )));
                        }
                    }
                    i += 1;
                }
            }
            let grid_col = entities.len() % 3;
            let grid_row = entities.len() / 3;
            entities.push(ChenEntity {
                name,
                is_weak,
                attrs,
                grid_col,
                grid_row,
            });
            i += 1;
            continue;
        }

        if let Some(rel) = try_parse_relationship_start(line) {
            let (name, is_identifying, has_block) = rel;
            let mut participants: Vec<ChenRelParticipant> = Vec::new();
            let mut rel_attrs: Vec<ChenAttr> = Vec::new();
            if has_block {
                i += 1;
                while i < raw_lines.len() {
                    let inner = raw_lines[i].trim();
                    if inner == "}" {
                        break;
                    }
                    if !inner.is_empty() {
                        if let Some(parts) = try_parse_participant_line(inner) {
                            participants.extend(parts);
                        } else if let Some(attr) = parse_attribute_line(inner) {
                            rel_attrs.push(attr);
                        }
                    }
                    i += 1;
                }
            }
            relationships.push(ChenRelationship {
                name,
                is_identifying,
                participants,
                attrs: rel_attrs,
            });
            i += 1;
            continue;
        }

        // Unrecognised line.
        warnings.push(Diagnostic::warning(format!(
            "[W_CHEN_SYNTAX] unrecognised chen syntax: `{line}`"
        )));
        i += 1;
    }

    // Re-assign grid positions now that all entities are known.
    for (idx, entity) in entities.iter_mut().enumerate() {
        entity.grid_col = idx % 3;
        entity.grid_row = idx / 3;
    }

    Ok(ChenDocument {
        entities,
        relationships,
        title,
        warnings,
    })
}

/// Tries to parse `entity NAME [weak] {` or `entity NAME [weak]` (no block).
/// Returns `(name, is_weak, has_block)` on success.
fn try_parse_entity_start(line: &str) -> Option<(String, bool, bool)> {
    let lower = line.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("entity ")
        .or_else(|| lower.strip_prefix("entity\t"))?;
    // Extract name (up to first whitespace or `{`).
    let rest = rest.trim_start();
    let name_end = rest
        .char_indices()
        .find(|(_, c)| c.is_whitespace() || *c == '{')
        .map(|(i, _)| i)
        .unwrap_or(rest.len());
    let name_lower = &rest[..name_end];
    // Reconstruct original-case name from the input line.
    let prefix_len = line.to_ascii_lowercase().find(name_lower)?;
    let name = line[prefix_len..prefix_len + name_end].to_string();
    if name.is_empty() {
        return None;
    }
    let after_name = rest[name_end..].trim();
    let is_weak = after_name.to_ascii_lowercase().starts_with("weak");
    let has_block = line.trim_end().ends_with('{');
    Some((name, is_weak, has_block))
}

/// Tries to parse `relationship NAME [identifying] {` or inline.
fn try_parse_relationship_start(line: &str) -> Option<(String, bool, bool)> {
    let lower = line.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("relationship ")
        .or_else(|| lower.strip_prefix("relation "))?;
    let rest = rest.trim_start();
    let name_end = rest
        .char_indices()
        .find(|(_, c)| c.is_whitespace() || *c == '{')
        .map(|(i, _)| i)
        .unwrap_or(rest.len());
    let name_lower = &rest[..name_end];
    let prefix_len = line.to_ascii_lowercase().find(name_lower)?;
    let name = line[prefix_len..prefix_len + name_end].to_string();
    if name.is_empty() {
        return None;
    }
    let after_name = rest[name_end..].trim();
    let is_identifying = after_name.to_ascii_lowercase().contains("identifying");
    let has_block = line.trim_end().ends_with('{');
    Some((name, is_identifying, has_block))
}

/// Parse an attribute line such as `key StudentID`, `multivalued Hobbies`,
/// `derived Age`, or plain `Name`.
fn parse_attribute_line(line: &str) -> Option<ChenAttr> {
    let lower = line.to_ascii_lowercase();
    let (kind, name_raw) = if lower.starts_with("key ") {
        (ChenAttrKind::Key, &line[4..])
    } else if lower.starts_with("[key] ") {
        (ChenAttrKind::Key, &line[6..])
    } else if lower.starts_with("multivalued ") {
        (ChenAttrKind::Multivalued, &line[12..])
    } else if lower.starts_with("derived ") {
        (ChenAttrKind::Derived, &line[8..])
    } else {
        (ChenAttrKind::Regular, line)
    };
    let name = name_raw.trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some(ChenAttr { name, kind })
}

/// Parse a participant line: `EntityA -> EntityB [1:N]`
/// Returns a Vec of participants (usually 2).
fn try_parse_participant_line(line: &str) -> Option<Vec<ChenRelParticipant>> {
    // Must contain `->`.
    let arrow_pos = line.find("->")?;
    let lhs_raw = line[..arrow_pos].trim();
    let rhs_raw = line[arrow_pos + 2..].trim();

    // Parse cardinality from bracketed `[1:N]` or colon-separated `: 1 : N`.
    let (rhs_name, lhs_card, rhs_card) = parse_cardinality_suffix(lhs_raw, rhs_raw);

    if lhs_raw.is_empty() || rhs_name.is_empty() {
        return None;
    }

    Some(vec![
        ChenRelParticipant {
            entity: lhs_raw.to_string(),
            cardinality: lhs_card,
        },
        ChenRelParticipant {
            entity: rhs_name,
            cardinality: rhs_card,
        },
    ])
}

/// Extract `(rhs_entity_name, lhs_cardinality, rhs_cardinality)` from a rhs
/// string that may contain `[1:N]` or `: 1 : N` style cardinality annotation.
fn parse_cardinality_suffix(_lhs: &str, rhs: &str) -> (String, String, String) {
    // Try bracket style: `CourseName [1:N]`
    if let Some(bracket_start) = rhs.rfind('[') {
        if let Some(bracket_end) = rhs.rfind(']') {
            if bracket_end > bracket_start {
                let inner = &rhs[bracket_start + 1..bracket_end];
                let name = rhs[..bracket_start].trim().to_string();
                if let Some((l, r)) = inner.split_once(':') {
                    return (name, l.trim().to_string(), r.trim().to_string());
                }
                return (name, String::from("1"), inner.trim().to_string());
            }
        }
    }
    // Try colon style: `CourseName : 1 : N`
    if let Some(first_colon) = rhs.find(':') {
        let name = rhs[..first_colon].trim().to_string();
        let rest = rhs[first_colon + 1..].trim();
        if let Some(second_colon) = rest.find(':') {
            let l = rest[..second_colon].trim().to_string();
            let r = rest[second_colon + 1..].trim().to_string();
            return (name, l, r);
        }
        // Only one cardinality — use it for rhs.
        return (name, String::from("1"), rest.to_string());
    }
    // No cardinality annotation — use defaults.
    (rhs.to_string(), String::from("1"), String::from("N"))
}
