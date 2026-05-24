use crate::Diagnostic;

use super::common::strip_mermaid_comment;

// ---------------------------------------------------------------------------
// erDiagram adapter → PlantUML class-style diagram
// ---------------------------------------------------------------------------

/// Translate Mermaid `erDiagram` to a PlantUML class-style diagram.
///
/// Mermaid ER relation line:
///   `CUSTOMER ||--o{ ORDER : places`
///
/// We translate each entity name to a `class` declaration and each relation to
/// a PlantUML association arrow, carrying the cardinality string as an arrow
/// label for readability.
///
/// Cardinality glyph map (lossy but human-readable):
///   `||--o{`  →  `"1" --> "0..*"`
///   `||--|{`  →  `"1" --> "1..*"`
///   `}o--o{`  →  `"0..*" --> "0..*"`
///   etc.
///
/// Exact visual fidelity is not the goal; the output must parse cleanly.
pub(super) fn adapt_mermaid_erdiagram(source: &str) -> Result<String, Diagnostic> {
    use std::collections::BTreeSet;

    let mut entities: BTreeSet<String> = BTreeSet::new();
    let mut relations: Vec<String> = Vec::new();
    let mut first = true;
    let mut in_entity_block: Option<String> = None;

    for raw_line in source.lines() {
        let line = strip_mermaid_comment(raw_line).trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        if first {
            first = false;
            // Skip `erDiagram` directive.
            continue;
        }

        // If we're inside an entity attribute block.
        if in_entity_block.is_some() {
            if line == "}" {
                in_entity_block = None;
            }
            // Attribute lines are ignored for now – entity is already registered.
            continue;
        }

        // `ENTITY {` – start of attribute block.
        if line.ends_with('{') {
            let entity_name = line.trim_end_matches('{').trim().to_string();
            if !entity_name.is_empty() {
                entities.insert(entity_name.clone());
                in_entity_block = Some(entity_name);
            }
            continue;
        }

        // Relation line: `CUSTOMER ||--o{ ORDER : places`
        // Split on `:` to get core and label.
        if let Some((core, label)) = line.split_once(':') {
            if let Some(rel) = adapt_er_relation(core.trim(), label.trim(), &mut entities) {
                relations.push(rel);
                continue;
            }
        }

        // Bare entity name (no relation, no block).
        if !line.contains(' ') {
            entities.insert(line.to_string());
            continue;
        }

        // Unknown line – emit as comment.
        relations.push(format!("' [erDiagram] {line}"));
    }

    let mut out = vec!["@startuml".to_string()];

    // Emit entity class declarations.
    for entity in &entities {
        out.push(format!("class {entity}"));
    }

    // Emit relations.
    for rel in &relations {
        out.push(rel.clone());
    }

    out.push("@enduml".to_string());
    Ok(out.join("\n"))
}

/// Parse a Mermaid ER relation core like `CUSTOMER ||--o{ ORDER`.
/// Registers entity names as a side-effect.
fn adapt_er_relation(
    core: &str,
    label: &str,
    entities: &mut std::collections::BTreeSet<String>,
) -> Option<String> {
    // Mermaid ER cardinality tokens on the left and right of the `--`.
    // The double dash `--` separates the two sides.
    let dash_idx = core.find("--")?;
    // Split into: lhs_with_card `ENTITY ||` and rhs_with_card `o{ ENTITY`.
    let lhs_part = &core[..dash_idx]; // e.g. `CUSTOMER ||`
    let rhs_part = &core[dash_idx + 2..]; // e.g. `o{ ORDER`

    // lhs_part ends with the cardinality token; entity name is before the token.
    let (lhs_entity, lhs_card) = split_er_entity_and_card(lhs_part, true)?;
    let (rhs_entity, rhs_card) = split_er_entity_and_card(rhs_part, false)?;

    entities.insert(lhs_entity.clone());
    entities.insert(rhs_entity.clone());

    let card_str = format!("{lhs_card}--{rhs_card}");
    let rel_label = if label.is_empty() {
        card_str
    } else {
        format!("{card_str} {label}")
    };

    Some(format!("{lhs_entity} --> {rhs_entity} : {rel_label}"))
}

/// Split an ER half-line into `(entity_name, cardinality_string)`.
/// `is_lhs` controls whether the cardinality token is at the end (lhs) or start (rhs).
fn split_er_entity_and_card(part: &str, is_lhs: bool) -> Option<(String, String)> {
    let part = part.trim();
    // Cardinality tokens: `||`, `|{`, `|o`, `}{`, `}|`, `}o`, `o|`, `o{`, `o}`
    // These are 2-character tokens.
    let card_tokens = [
        "||", "|{", "|o", "}{", "}|", "}o", "o|", "o{", "o}", "{|", "{o",
    ];

    if is_lhs {
        // Entity is at the beginning; cardinality token at the end.
        for token in &card_tokens {
            if let Some(stripped) = part.strip_suffix(token) {
                let entity = stripped.trim();
                if !entity.is_empty() {
                    return Some((entity.to_string(), token.to_string()));
                }
            }
        }
        // No token found – treat the whole thing as the entity with empty card.
        if !part.is_empty() {
            return Some((part.to_string(), String::new()));
        }
    } else {
        // Entity is at the end; cardinality token at the start.
        for token in &card_tokens {
            if let Some(stripped) = part.strip_prefix(token) {
                let entity = stripped.trim();
                if !entity.is_empty() {
                    return Some((entity.to_string(), token.to_string()));
                }
            }
        }
        // No token found – treat the whole thing as the entity.
        if !part.is_empty() {
            return Some((part.to_string(), String::new()));
        }
    }
    None
}
