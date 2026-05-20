use std::collections::{BTreeMap, BTreeSet};

use crate::Diagnostic;

use super::strip_mermaid_comment;

/// Translate Mermaid `classDiagram` to PlantUML `@startuml` / `@enduml` with
/// `class Name { members }` blocks and `A <|-- B` relations.
///
/// Mermaid forms supported:
///   `Animal <|-- Dog`               -> kept as-is (PlantUML-compatible)
///   `Animal : +String name`         -> collected into class Animal { } block
///   `Animal : +eat()`               -> collected into class Animal { } block
///   `class Dog { +bark() }`         -> emit class Dog { +bark() }
///   `class Dog { \n+bark()\n}`      -> multi-line form (each member on its own line)
pub(super) fn adapt(source: &str) -> Result<String, Diagnostic> {
    // We do two passes: first gather all class members from `ClassName : member`
    // lines, then emit relations, inline classes, and finally gathered classes.
    let mut class_members: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut relations: Vec<String> = Vec::new();
    let mut inline_classes: Vec<String> = Vec::new();

    let mut first = true;
    let mut in_class_block: Option<(String, Vec<String>)> = None;
    let lines_iter = source.lines();

    for raw_line in lines_iter {
        let line = strip_mermaid_comment(raw_line).trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        if first {
            first = false;
            // Skip `classDiagram` directive.
            continue;
        }

        // If we're inside a `class Foo {` block, accumulate members until `}`.
        if let Some((ref class_name, ref mut members)) = in_class_block {
            if line == "}" {
                let class_name = class_name.clone();
                let members = members.clone();
                inline_classes.push(format_class_block(&class_name, &members));
                in_class_block = None;
            } else {
                members.push(line.to_string());
            }
            continue;
        }

        // `ClassName : member` form - accumulate into class_members.
        if let Some(converted) = adapt_classdiagram_member_line(line) {
            let (cname, member) = converted;
            class_members.entry(cname).or_default().push(member);
            continue;
        }

        // Relation line: `A <|-- B`, `A --> B`, `A -- B`, etc.
        if let Some(rel) = adapt_classdiagram_relation(line) {
            relations.push(rel);
            continue;
        }

        // `class Foo {` - start of inline block.
        if let Some(rest) = line.strip_prefix("class ") {
            let rest = rest.trim();
            if rest.ends_with('{') {
                let class_name = rest.trim_end_matches('{').trim().to_string();
                in_class_block = Some((class_name, Vec::new()));
                continue;
            }
            // Bare `class Foo` declaration.
            let class_name = rest.trim().to_string();
            if !class_name.is_empty() {
                inline_classes.push(format!("class {class_name}"));
            }
            continue;
        }

        // Ignore `%%comment` and other unrecognised lines gracefully.
        inline_classes.push(format!("' [classDiagram] {line}"));
    }

    // If a block was never closed, flush it anyway.
    if let Some((class_name, members)) = in_class_block {
        inline_classes.push(format_class_block(&class_name, &members));
    }

    // Build output.
    let mut out = vec!["@startuml".to_string()];

    // Collect all class names that appear in relations so we can ensure they
    // have at least a bare `class X` declaration before the first relation.
    // This guarantees the parser detects `DiagramKind::Class` before it sees
    // the first relation line, which requires the kind to already be Class.
    let mut declared: BTreeSet<String> = BTreeSet::new();

    // Emit gathered class blocks from `ClassName : member` lines first.
    for (class_name, members) in &class_members {
        declared.insert(class_name.clone());
        out.push(format_class_block(class_name, members));
    }

    // Emit inline class declarations / blocks.
    for item in &inline_classes {
        // Track names from `class Foo` or `class Foo { ... }` items.
        if let Some(rest) = item.strip_prefix("class ") {
            let name = rest.split_whitespace().next().unwrap_or("").to_string();
            if !name.is_empty() {
                declared.insert(name);
            }
        }
        out.push(item.clone());
    }

    // For any class referenced only in relations, emit a bare declaration first
    // so the family is established before we emit relation lines.
    for rel in &relations {
        // Extract lhs and rhs names separated by arrow tokens.
        for arrow in &[
            "<|--", "--|>", "*--", "--*", "o--", "--o", "-->", "<--", "--",
        ] {
            if let Some(idx) = rel.find(arrow) {
                let lhs = rel[..idx].trim().to_string();
                let rhs = rel[idx + arrow.len()..]
                    .split(':')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !lhs.is_empty() && !declared.contains(&lhs) {
                    out.push(format!("class {lhs}"));
                    declared.insert(lhs);
                }
                if !rhs.is_empty() && !declared.contains(&rhs) {
                    out.push(format!("class {rhs}"));
                    declared.insert(rhs);
                }
                break;
            }
        }
    }

    // Emit relations after all class declarations.
    for rel in &relations {
        out.push(rel.clone());
    }

    out.push("@enduml".to_string());
    Ok(out.join("\n"))
}

fn format_class_block(name: &str, members: &[String]) -> String {
    if members.is_empty() {
        format!("class {name}")
    } else {
        let body = members.join("\n");
        format!("class {name} {{\n{body}\n}}")
    }
}

/// Parse a `ClassName : member` line. Returns `(class_name, member)`.
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
