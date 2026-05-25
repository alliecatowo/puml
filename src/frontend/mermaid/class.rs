use crate::frontend::{FrontendBuilder, FrontendResult};
use crate::{source::Span, Diagnostic};

use super::common::strip_mermaid_comment;

// ---------------------------------------------------------------------------
// classDiagram adapter → PlantUML class diagram
// ---------------------------------------------------------------------------

/// Translate Mermaid `classDiagram` to PlantUML `@startuml` / `@enduml` with
/// `class Name { members }` blocks and `A <|-- B` relations.
///
/// Mermaid forms supported:
///   `Animal <|-- Dog`               → kept as-is (PlantUML-compatible)
///   `Animal : +String name`         → collected into class Animal { } block
///   `Animal : +eat()`               → collected into class Animal { } block
///   `class Dog { +bark() }`         → emit class Dog { +bark() }
///   `class Dog { \n+bark()\n}`      → multi-line form (each member on its own line)
pub(super) fn adapt_mermaid_classdiagram(source: &str) -> Result<FrontendResult, Diagnostic> {
    use std::collections::BTreeMap;

    // We do two passes: first gather all class members from `ClassName : member`
    // lines, then emit relations, inline classes, and finally gathered classes.
    let mut class_members: BTreeMap<String, Vec<GeneratedLine>> = BTreeMap::new();
    let mut relations: Vec<GeneratedLine> = Vec::new();
    let mut inline_classes: Vec<GeneratedBlock> = Vec::new();
    let mut out = FrontendBuilder::new();

    let mut first = true;
    let mut directive_span = Span::new(0, 0);
    let mut in_class_block: Option<(String, Span, Vec<GeneratedLine>)> = None;
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
            // Skip `classDiagram` directive.
            continue;
        }

        // If we're inside a `class Foo {` block, accumulate members until `}`.
        if let Some((ref class_name, block_span, ref mut members)) = in_class_block {
            if line == "}" {
                let class_name = class_name.clone();
                let members = members.clone();
                inline_classes.push(format_class_block(&class_name, &members, block_span));
                in_class_block = None;
            } else {
                members.push(GeneratedLine::new(line, span));
            }
            continue;
        }

        // `ClassName : member` form – accumulate into class_members.
        if let Some(converted) = adapt_classdiagram_member_line(line) {
            let (cname, member) = converted;
            class_members
                .entry(cname)
                .or_default()
                .push(GeneratedLine::new(member, span));
            continue;
        }

        // Relation line: `A <|-- B`, `A --> B`, `A -- B`, etc.
        if let Some(rel) = adapt_classdiagram_relation(line) {
            relations.push(GeneratedLine::new(rel, span));
            continue;
        }

        // `class Foo {` — start of inline block.
        if let Some(rest) = line.strip_prefix("class ") {
            let rest = rest.trim();
            if let Some(block) = adapt_inline_class_block(rest, span) {
                inline_classes.push(block);
                continue;
            }
            if rest.ends_with('{') {
                let class_name = rest.trim_end_matches('{').trim().to_string();
                in_class_block = Some((class_name, span, Vec::new()));
                continue;
            }
            // Bare `class Foo` declaration.
            let class_name = rest.trim().to_string();
            if !class_name.is_empty() {
                inline_classes.push(GeneratedBlock::single(format!("class {class_name}"), span));
            }
            continue;
        }

        out.push_diagnostic(
            Diagnostic::warning(format!(
                "[W_MERMAID_CLASS_DEFERRED] unsupported mermaid class construct was deferred: `{line}`"
            ))
            .with_span(span),
        );
        inline_classes.push(GeneratedBlock::single(
            format!("' [classDiagram] {line}"),
            span,
        ));
    }

    // If a block was never closed, flush it anyway.
    if let Some((class_name, block_span, members)) = in_class_block {
        out.push_diagnostic(
            Diagnostic::warning(format!(
                "[W_MERMAID_CLASS_MALFORMED] mermaid class block `{class_name}` was not closed; lowered the members collected so far"
            ))
            .with_span(block_span),
        );
        inline_classes.push(format_class_block(&class_name, &members, block_span));
    }

    // Build output.
    out.push_line("@startuml", directive_span);

    // Collect all class names that appear in relations so we can ensure they
    // have at least a bare `class X` declaration before the first relation.
    // This guarantees the parser detects `DiagramKind::Class` before it sees
    // the first relation line, which requires the kind to already be Class.
    let mut declared: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

    // Emit gathered class blocks from `ClassName : member` lines first.
    for (class_name, members) in &class_members {
        declared.insert(class_name.clone());
        let block = format_class_block(
            class_name,
            members,
            members.first().map_or(directive_span, |member| member.span),
        );
        out.push_generated_block(&block);
    }

    // Emit inline class declarations / blocks.
    for item in &inline_classes {
        // Track names from `class Foo` or `class Foo { ... }` items.
        if let Some(rest) = item.first_line().strip_prefix("class ") {
            let name = rest.split_whitespace().next().unwrap_or("").to_string();
            if !name.is_empty() {
                declared.insert(name);
            }
        }
        out.push_generated_block(item);
    }

    // For any class referenced only in relations, emit a bare declaration first
    // so the family is established before we emit relation lines.
    for rel in &relations {
        // Extract lhs and rhs names separated by arrow tokens.
        for arrow in &[
            "<|--", "--|>", "*--", "--*", "o--", "--o", "-->", "<--", "--",
        ] {
            if let Some(idx) = rel.text.find(arrow) {
                let lhs = rel.text[..idx].trim().to_string();
                let rhs = rel.text[idx + arrow.len()..]
                    .split(':')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !lhs.is_empty() && !declared.contains(&lhs) {
                    out.push_line(format!("class {lhs}"), rel.span);
                    declared.insert(lhs);
                }
                if !rhs.is_empty() && !declared.contains(&rhs) {
                    out.push_line(format!("class {rhs}"), rel.span);
                    declared.insert(rhs);
                }
                break;
            }
        }
    }

    // Emit relations after all class declarations.
    for rel in &relations {
        out.push_line(&rel.text, rel.span);
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
struct GeneratedBlock {
    lines: Vec<GeneratedLine>,
}

impl GeneratedBlock {
    fn single(text: impl Into<String>, span: Span) -> Self {
        Self {
            lines: vec![GeneratedLine::new(text, span)],
        }
    }

    fn first_line(&self) -> &str {
        self.lines.first().map_or("", |line| line.text.as_str())
    }
}

trait FrontendBuilderClassExt {
    fn push_generated_block(&mut self, block: &GeneratedBlock);
}

impl FrontendBuilderClassExt for FrontendBuilder {
    fn push_generated_block(&mut self, block: &GeneratedBlock) {
        for line in &block.lines {
            self.push_line(&line.text, line.span);
        }
    }
}

fn format_class_block(
    name: &str,
    members: &[GeneratedLine],
    declaration_span: Span,
) -> GeneratedBlock {
    if members.is_empty() {
        GeneratedBlock::single(format!("class {name}"), declaration_span)
    } else {
        let mut lines = Vec::with_capacity(members.len() + 2);
        lines.push(GeneratedLine::new(
            format!("class {name} {{"),
            declaration_span,
        ));
        lines.extend(members.iter().cloned());
        lines.push(GeneratedLine::new("}", declaration_span));
        GeneratedBlock { lines }
    }
}

fn adapt_inline_class_block(rest: &str, span: Span) -> Option<GeneratedBlock> {
    let (class_name, body_with_close) = rest.split_once('{')?;
    let class_name = class_name.trim();
    let body = body_with_close.strip_suffix('}')?.trim();
    if class_name.is_empty() {
        return None;
    }
    let members = body
        .split(';')
        .map(str::trim)
        .filter(|member| !member.is_empty())
        .map(|member| GeneratedLine::new(member, span))
        .collect::<Vec<_>>();
    Some(format_class_block(class_name, &members, span))
}

/// Parse a `ClassName : member` line.  Returns `(class_name, member)`.
fn adapt_classdiagram_member_line(line: &str) -> Option<(String, String)> {
    // Must not look like a relation (no `<`, `>`, `--`).
    if line.contains("--") || line.contains('<') || line.contains('>') {
        return None;
    }
    let (class_name, member) = line.split_once(':')?;
    let class_name = class_name.trim();
    let member = member.trim();
    if class_name.is_empty() || member.is_empty() {
        return None;
    }
    // Class name must not contain spaces (would indicate it's something else).
    if class_name.contains(' ') {
        return None;
    }
    Some((class_name.to_string(), member.to_string()))
}

/// Try to parse a Mermaid class relation line.
/// Mermaid relations that are already PlantUML-compatible are passed through.
fn adapt_classdiagram_relation(line: &str) -> Option<String> {
    // Must contain `--` to be a relation.
    if !line.contains("--") {
        return None;
    }
    // Mermaid relation forms:
    //   `A <|-- B`   inheritance  (PlantUML: `A <|-- B`)
    //   `A *-- B`    composition
    //   `A o-- B`    aggregation
    //   `A --> B`    association
    //   `A -- B`     link
    //   `A ..> B`    dependency
    //   `A ..|> B`   realization
    // Many of these are already valid PlantUML; we pass them through.
    // Strip optional label suffix `: label`.
    let (core, label) = if let Some((c, l)) = line.split_once(':') {
        // Make sure lhs contains `--` so we don't misparse member lines.
        if c.contains("--") {
            (c.trim(), Some(l.trim()))
        } else {
            (line, None)
        }
    } else {
        (line, None)
    };

    // Verify there's at least one `--` in core.
    if !core.contains("--") && !core.contains("..") {
        return None;
    }

    if let Some(lbl) = label {
        Some(format!("{core} : {lbl}"))
    } else {
        Some(core.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classdiagram_uses_original_span_for_deferred_lines() {
        let source = "classDiagram\nclass Dog\nclick Dog callback\n";
        let result = adapt_mermaid_classdiagram(source).expect("class adapter");

        assert_eq!(result.diagnostics.len(), 1);
        let diagnostic = &result.diagnostics[0];
        assert!(diagnostic.message.contains("W_MERMAID_CLASS_DEFERRED"));
        assert_eq!(diagnostic.line_col(source), Some((3, 1)));
        assert_eq!(
            diagnostic.span.map(|span| &source[span.start..span.end]),
            Some("click Dog callback")
        );
    }
}
