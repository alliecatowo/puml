use crate::frontend::{FrontendBuilder, FrontendResult};
use crate::{source::Span, Diagnostic};

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
pub(super) fn adapt_mermaid_erdiagram(source: &str) -> Result<FrontendResult, Diagnostic> {
    use std::collections::BTreeMap;

    let mut entities: BTreeMap<String, EntityBlock> = BTreeMap::new();
    let mut relations: Vec<GeneratedLine> = Vec::new();
    let mut deferred: Vec<GeneratedLine> = Vec::new();
    let mut out = FrontendBuilder::new();
    let mut first = true;
    let mut directive_span = Span::new(0, 0);
    let mut in_entity_block: Option<String> = None;
    let mut offset = 0usize;

    for raw_line in source.lines() {
        let span = Span::new(offset, offset + raw_line.len());
        offset += raw_line.len() + 1;
        let line = strip_mermaid_comment(raw_line).trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        if first {
            first = false;
            directive_span = span;
            // Skip `erDiagram` directive.
            continue;
        }

        // If we're inside an entity attribute block.
        if let Some(entity) = &in_entity_block {
            if line == "}" {
                in_entity_block = None;
            } else {
                entities
                    .entry(entity.clone())
                    .or_insert_with(|| EntityBlock::new(span))
                    .push(GeneratedLine::new(line, span));
            }
            continue;
        }

        // `ENTITY {` – start of attribute block.
        if line.ends_with('{') {
            let entity_name = line.trim_end_matches('{').trim().to_string();
            if !entity_name.is_empty() {
                entities
                    .entry(entity_name.clone())
                    .or_insert_with(|| EntityBlock::new(span));
                in_entity_block = Some(entity_name);
            }
            continue;
        }

        // Relation line: `CUSTOMER ||--o{ ORDER : places`
        // Split on `:` to get core and label.
        if let Some((core, label)) = line.split_once(':') {
            if let Some(rel) = adapt_er_relation(core.trim(), label.trim(), span, &mut entities) {
                relations.push(GeneratedLine::new(rel, span));
                continue;
            }
        }

        // Bare entity name (no relation, no block).
        if !line.contains(' ') {
            entities
                .entry(line.to_string())
                .or_insert_with(|| EntityBlock::new(span));
            continue;
        }

        out.push_diagnostic(
            Diagnostic::warning(format!(
                "[W_MERMAID_ER_DEFERRED] unsupported mermaid ER construct was deferred: `{line}`"
            ))
            .with_span(span),
        );
        deferred.push(GeneratedLine::new(format!("' [erDiagram] {line}"), span));
    }

    out.push_line("@startuml", directive_span);

    // Emit entity class declarations.
    for (entity, block) in &entities {
        if block.attributes.is_empty() {
            out.push_line(format!("class {entity}"), block.span);
        } else {
            let declaration_span = block
                .attributes
                .first()
                .map_or(block.span, |line| line.span);
            out.push_line(format!("class {entity} {{"), declaration_span);
            for attribute in &block.attributes {
                out.push_line(&attribute.text, attribute.span);
            }
            out.push_line("}", declaration_span);
        }
    }

    // Emit relations.
    for rel in &relations {
        out.push_line(&rel.text, rel.span);
    }

    for line in &deferred {
        out.push_line(&line.text, line.span);
    }

    out.push_line("@enduml", directive_span);
    Ok(out.finish())
}

#[derive(Debug, Clone)]
struct GeneratedLine {
    text: String,
    span: Span,
}

impl GeneratedLine {
    fn new(text: impl Into<String>, span: Span) -> Self {
        Self {
            text: text.into(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
struct EntityBlock {
    span: Span,
    attributes: Vec<GeneratedLine>,
}

impl EntityBlock {
    fn new(span: Span) -> Self {
        Self {
            span,
            attributes: Vec::new(),
        }
    }

    fn push(&mut self, attribute: GeneratedLine) {
        self.attributes.push(attribute);
    }
}

/// Parse a Mermaid ER relation core like `CUSTOMER ||--o{ ORDER`.
/// Registers entity names as a side-effect.
fn adapt_er_relation(
    core: &str,
    label: &str,
    span: Span,
    entities: &mut std::collections::BTreeMap<String, EntityBlock>,
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

    entities
        .entry(lhs_entity.clone())
        .or_insert_with(|| EntityBlock::new(span));
    entities
        .entry(rhs_entity.clone())
        .or_insert_with(|| EntityBlock::new(span));

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
