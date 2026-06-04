use crate::frontend::{FrontendBuilder, FrontendResult};
use crate::{source::Span, Diagnostic};

use super::common::strip_mermaid_comment;

// ---------------------------------------------------------------------------
// flowchart / graph adapter → PlantUML component-style
// ---------------------------------------------------------------------------

/// Translate a Mermaid `flowchart TD` / `graph TD` block into a PlantUML
/// component-style diagram that the existing component renderer accepts.
///
/// Node shapes handled (Phase A):
///   `A`            → `component A`        (bare id, square fallback)
///   `A[Label]`     → `component "Label" as A`
///   `A(Label)`     → `component "Label" as A`        (rounded)
///   `A((Label))`   → `component "Label" as A`        (circle)
///   `A{Label}`     → `component "Label" as A`        (diamond / decision)
///   `A>Label]`     → `component "Label" as A`        (asymmetric)
///   `A[[Label]]`   → `component "Label" as A`        (subroutine)
///   `A[(Label)]`   → `component "Label" as A`        (cylinder / DB)
///   `A:::cls`      → `<<cls>>` stereotype on the component
///
/// Arrow forms handled (Phase A):
///   `A --> B`           → `A --> B`              (open arrow)
///   `A --- B`           → `A -- B`               (line, no arrow)
///   `A -.- B`           → `A .. B`               (dotted line)
///   `A -.-> B`          → `A ..> B`              (dotted arrow)
///   `A ==> B`           → `A --> B`              (thick arrow; thickness lost)
///   `A === B`           → `A -- B`               (thick line; thickness lost)
///   `A --x B`           → `A --x B` → relation w/ x-marker (cross terminator)
///   `A --o B`           → `A --o B` → relation w/ circle terminator
///   `A -->|label| B`    → `A --> B : label`
///   `A -- label --> B`  → `A --> B : label`
///
/// Subgraphs:
///   `subgraph name`              → `package "name" {`
///   `subgraph id1 [Title]`       → `package "Title" {`
///   `end`                        → `}`
pub(super) fn adapt_mermaid_flowchart(source: &str) -> Result<FrontendResult, Diagnostic> {
    use std::collections::{BTreeMap, BTreeSet};

    // We use a two-phase approach so that bare-id component declarations are
    // emitted BEFORE the edge lines that reference them. This ensures the parser
    // sees the `component X` declarations first and classifies the output as a
    // component diagram rather than a sequence diagram (which would emit
    // W_SEQUENCE_UNSUPPORTED_SYNTAX warnings for the component keyword).
    //
    // Phase 1: scan the source, collecting class defs, diagnostics, declared IDs,
    //          referenced IDs, and the body lines (edges + node decls) into a buffer.
    // Phase 2: emit `component <id>` for any referenced-but-undeclared IDs, then
    //          flush the body buffer.

    let mut out = FrontendBuilder::new();
    let mut first = true;
    let mut directive_span = Span::new(0, 0);
    let mut class_defs: BTreeMap<String, String> = BTreeMap::new();
    let mut declared_ids: BTreeSet<String> = BTreeSet::new();
    // referenced_ids: (id, span) tuples for backfill step.
    let mut referenced_ids: Vec<(String, Span)> = Vec::new();
    // body_lines: deferred output lines (edges, node decls, subgraph) collected
    // during the main pass so we can prepend component declarations before them.
    let mut body_lines: Vec<(String, Span)> = Vec::new();
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
            out.push_line("@startuml", span);
            // Skip the `flowchart TD` / `graph TD` directive line.
            continue;
        }

        if let Some((class_name, fill, ignored_attrs)) = parse_flowchart_class_def(line) {
            if !ignored_attrs.is_empty() {
                out.push_diagnostic(
                    Diagnostic::warning(format!(
                        "[W_MERMAID_STYLE_PARTIAL] mermaid flowchart class `{class_name}` only preserves fill color; ignored attributes: {ignored_attrs}"
                    ))
                    .with_span(span),
                );
            }
            class_defs.insert(class_name, fill);
            continue;
        }

        if let Some((converted, warning, ids)) = adapt_flowchart_style(line, &class_defs, span) {
            if let Some(warning) = warning {
                out.push_diagnostic(warning);
            }
            for id in ids {
                declared_ids.insert(id);
            }
            for body_line in converted.lines() {
                body_lines.push((body_line.to_string(), span));
            }
            continue;
        }

        if is_unsupported_flowchart_directive(line) {
            return Err(Diagnostic::error_code(
                "E_MERMAID_FEATURE_LOSS",
                format!("unsupported mermaid flowchart construct would be dropped: `{line}`"),
            )
            .with_span(span));
        }

        // Try to parse as an arrow statement first.
        if let Some((converted, ids)) = adapt_flowchart_edge(line) {
            for id in &ids {
                referenced_ids.push((id.clone(), span));
            }
            // Mark explicitly-declared ids (ones with a label or class
            // suffix) as declared so we don't double-emit a bare
            // `component X` later.
            for converted_line in converted.lines() {
                if let Some(id) = component_decl_id(converted_line) {
                    declared_ids.insert(id.to_string());
                }
            }
            for body_line in converted.lines() {
                body_lines.push((body_line.to_string(), span));
            }
            continue;
        }

        // Node declaration: `ID[Label]`, `ID{Label}`, `ID(Label)`, bare `ID`.
        if let Some((converted, id)) = adapt_flowchart_node(line) {
            declared_ids.insert(id);
            body_lines.push((converted, span));
            continue;
        }

        // Subgraph / end – map to `package`/`end`.
        if let Some(rest) = line.strip_prefix("subgraph ") {
            let (sub_id, sub_label) = parse_subgraph_header(rest.trim());
            body_lines.push((format!("package \"{sub_label}\" {{"), span));
            // Track the synthetic subgraph id so a later `end` line
            // doesn't trigger a spurious bare-id backfill.
            declared_ids.insert(sub_id);
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower == "end" || lower == "end subgraph" {
            body_lines.push(("}".to_string(), span));
            continue;
        }

        return Err(Diagnostic::error_code(
            "E_MERMAID_FEATURE_LOSS",
            format!("unsupported mermaid flowchart construct would be dropped: `{line}`"),
        )
        .with_span(span));
    }

    // Phase 2: emit component declarations for referenced-but-undeclared IDs
    // BEFORE the edge lines. This ensures the parser classifies the output as a
    // component diagram, not a sequence diagram.
    for (id, original_span) in &referenced_ids {
        if declared_ids.insert(id.clone()) {
            out.push_line(format!("component {id}"), *original_span);
        }
    }

    // Now flush the deferred body lines (edges, node decls, subgraphs).
    for (line, span) in body_lines {
        out.push_line(line, span);
    }

    out.push_line("@enduml", directive_span);
    Ok(out.finish())
}

fn is_unsupported_flowchart_directive(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    matches!(
        lower.split_ascii_whitespace().next(),
        Some("click" | "linkstyle" | "accdescr" | "acctitle")
    )
}

/// Extract the id from a generated `component <id>` or `component "label" as <id>`
/// declaration line. Returns `None` for any other line shape.
fn component_decl_id(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("component ")?;
    if let Some(idx) = rest.find(" as ") {
        // `"label" as ID <<stereo>>` — id starts after " as " and runs
        // until whitespace or `<<`.
        let after_as = &rest[idx + 4..];
        let id_end = after_as
            .find(|c: char| c.is_whitespace() || c == '<')
            .unwrap_or(after_as.len());
        Some(after_as[..id_end].trim())
    } else {
        // Bare `component ID` (possibly with `<<stereo>>` trailing).
        let id_end = rest
            .find(|c: char| c.is_whitespace() || c == '<')
            .unwrap_or(rest.len());
        Some(rest[..id_end].trim())
    }
}

/// Parse a subgraph header. Mermaid permits two forms:
///   `subgraph id1`               — id only, used as label
///   `subgraph id1 [Display Name]` — id + explicit title in `[...]`
///
/// Returns `(id, label_to_render)`.
fn parse_subgraph_header(rest: &str) -> (String, String) {
    if let Some(bracket_start) = rest.find('[') {
        let id = rest[..bracket_start].trim().to_string();
        let after = &rest[bracket_start + 1..];
        if let Some(close) = after.rfind(']') {
            let label = after[..close].trim().trim_matches('"').to_string();
            if !label.is_empty() {
                return (if id.is_empty() { label.clone() } else { id }, label);
            }
        }
    }
    let cleaned = rest.trim().trim_matches('"').to_string();
    (cleaned.clone(), cleaned)
}

/// Extract a node's canonical id and optional label from Mermaid node syntax.
/// Returns `(id, label)`. The bracket order is significant: longer / more
/// specific delimiters must be tried before the shorter ones so
/// `A((label))` is not misread as `A(label)` with a stray `)`.
fn parse_flowchart_node_id_label(token: &str) -> (String, Option<String>) {
    // Order matters: try `[[`, `[(`, `((`, `[`, `{`, `(`, then asymmetric `>...]`.
    for (open, close) in [
        ("[[", "]]"),
        ("[(", ")]"),
        ("((", "))"),
        ("[", "]"),
        ("{", "}"),
        ("(", ")"),
    ] {
        if let Some(bracket_start) = token.find(open) {
            let id = token[..bracket_start].trim().to_string();
            if !id.is_empty() {
                let rest = &token[bracket_start + open.len()..];
                if let Some(end) = rest.rfind(close) {
                    let label = rest[..end].trim().to_string();
                    return (id, if label.is_empty() { None } else { Some(label) });
                }
            }
        }
    }
    // Asymmetric `A>Label]` shape – no matching opener for `>`, so handle
    // it explicitly. The `>` must appear after an id and before a label
    // that terminates in `]`.
    if let Some(gt_pos) = token.find('>') {
        let id = token[..gt_pos].trim().to_string();
        let rest = &token[gt_pos + 1..];
        if !id.is_empty() {
            if let Some(end) = rest.rfind(']') {
                let label = rest[..end].trim().to_string();
                return (id, if label.is_empty() { None } else { Some(label) });
            }
        }
    }
    // Bare id.
    (token.trim().to_string(), None)
}

fn adapt_flowchart_node(line: &str) -> Option<(String, String)> {
    // Must not contain an edge connector to be a pure node declaration.
    if line_contains_edge_connector(line) {
        return None;
    }
    let (id, label, class_name) = parse_flowchart_node_token(line);
    if id.is_empty() {
        return None;
    }
    let decl = format_flowchart_node_declaration(&id, label.as_deref(), class_name.as_deref());
    Some((decl, id))
}

fn parse_flowchart_node_token(token: &str) -> (String, Option<String>, Option<String>) {
    let (node_part, class_name) = token.split_once(":::").unwrap_or((token, ""));
    let (id, label) = parse_flowchart_node_id_label(node_part);
    let class_name = (!class_name.trim().is_empty()).then(|| class_name.trim().to_string());
    (id, label, class_name)
}

fn format_flowchart_node_declaration(
    id: &str,
    label: Option<&str>,
    class_name: Option<&str>,
) -> String {
    let class_suffix = class_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!(" <<{value}>>"))
        .unwrap_or_default();
    if let Some(lbl) = label {
        format!("component \"{lbl}\" as {id}{class_suffix}")
    } else {
        format!("component {id}{class_suffix}")
    }
}

fn parse_flowchart_class_def(line: &str) -> Option<(String, String, String)> {
    let rest = line.trim().strip_prefix("classDef ")?;
    let (name, attrs) = rest.split_once(char::is_whitespace)?;
    let fill = parse_mermaid_style_fill(attrs)?;
    Some((
        name.trim().to_string(),
        fill,
        ignored_mermaid_style_attrs(attrs),
    ))
}

fn adapt_flowchart_style(
    line: &str,
    class_defs: &std::collections::BTreeMap<String, String>,
    span: Span,
) -> Option<(String, Option<Diagnostic>, Vec<String>)> {
    if let Some(rest) = line.trim().strip_prefix("style ") {
        let (id, attrs) = rest.split_once(char::is_whitespace)?;
        let fill = parse_mermaid_style_fill(attrs)?;
        let id = id.trim();
        let warning = ignored_mermaid_style_attrs(attrs);
        let warning = (!warning.is_empty()).then(|| {
            Diagnostic::warning(format!(
                "[W_MERMAID_STYLE_PARTIAL] mermaid flowchart style for `{id}` only preserves fill color; ignored attributes: {warning}"
            ))
            .with_span(span)
        });
        return Some((
            format!("component \"{id}\" as {id} {fill}"),
            warning,
            vec![id.to_string()],
        ));
    }
    if let Some(rest) = line.trim().strip_prefix("class ") {
        let mut parts = rest.split_ascii_whitespace();
        let ids = parts.next()?;
        let class_name = parts.next()?;
        let fill = class_defs.get(class_name)?;
        let mut id_list = Vec::new();
        let lines = ids
            .split(',')
            .filter(|id| !id.trim().is_empty())
            .map(|id| {
                let id = id.trim();
                id_list.push(id.to_string());
                format!("component \"{id}\" as {id} {fill}")
            })
            .collect::<Vec<_>>();
        if lines.is_empty() {
            return None;
        }
        return Some((lines.join("\n"), None, id_list));
    }
    None
}

fn parse_mermaid_style_fill(attrs: &str) -> Option<String> {
    attrs
        .split(',')
        .find_map(|part| part.trim().strip_prefix("fill:"))
        .map(str::trim)
        .filter(|value| {
            value.starts_with('#') || crate::theme::color::css3_color_to_hex(value).is_some()
        })
        .map(|value| crate::theme::color::resolve_css3_color_or_original(value).unwrap_or_default())
}

fn ignored_mermaid_style_attrs(attrs: &str) -> String {
    attrs
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .filter(|part| !part.starts_with("fill:"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Return true if the line plausibly contains a flowchart edge connector,
/// in any of the recognized Mermaid forms. Used to distinguish a
/// "pure node declaration" line from an "arrow expression" line.
fn line_contains_edge_connector(line: &str) -> bool {
    // Note: `-->` and friends, `==>`/`===`, `-.->`/`-.-`, plus the
    // terminator variants `--x`/`--o`/`==x`/`==o`/`-.-x`/`-.-o`.
    // Order doesn't matter here — any hit means "not a pure node".
    const CONNECTORS: &[&str] = &[
        "-.->", "-.-x", "-.-o", "-.-", "==>", "==x", "==o", "===", "-->", "--x", "--o", "---",
    ];
    CONNECTORS.iter().any(|c| line.contains(c))
}

/// Edge "kind" extracted from a Mermaid arrow connector. Used to drive
/// the PlantUML translation (arrow body + terminator selection).
#[derive(Clone, Copy, PartialEq, Eq)]
enum EdgeKind {
    /// Solid line with default open arrowhead: `-->`
    SolidArrow,
    /// Solid line, no arrowhead: `---`
    SolidLine,
    /// Dotted line with open arrowhead: `-.->`
    DottedArrow,
    /// Dotted line, no arrowhead: `-.-`
    DottedLine,
    /// Thick (bold) line with open arrowhead: `==>`. Mapped to `-->` for
    /// now since the component renderer doesn't have a thick variant.
    ThickArrow,
    /// Thick (bold) line, no arrowhead: `===`.
    ThickLine,
    /// Solid line ending in `x` (cross terminator): `--x`. Maps to `--x` PUML.
    SolidCross,
    /// Solid line ending in `o` (circle terminator): `--o`. Maps to `--o` PUML.
    SolidCircle,
    /// Thick line ending in `x`: `==x`.
    ThickCross,
    /// Thick line ending in `o`: `==o`.
    ThickCircle,
    /// Dotted line ending in `x`: `-.-x`.
    DottedCross,
    /// Dotted line ending in `o`: `-.-o`.
    DottedCircle,
}

impl EdgeKind {
    fn to_plantuml_arrow(self) -> &'static str {
        match self {
            EdgeKind::SolidArrow | EdgeKind::ThickArrow => "-->",
            EdgeKind::SolidLine | EdgeKind::ThickLine => "--",
            EdgeKind::DottedArrow => "..>",
            EdgeKind::DottedLine => "..",
            EdgeKind::SolidCross | EdgeKind::ThickCross => "--x",
            EdgeKind::SolidCircle | EdgeKind::ThickCircle => "--o",
            // The PlantUML component family doesn't have dotted+terminator
            // arrows, so we degrade dotted-terminator forms to solid.
            EdgeKind::DottedCross => "--x",
            EdgeKind::DottedCircle => "--o",
        }
    }
}

/// Match the longest connector at `pos` in `line`. Order is critical:
/// connectors that share a prefix (e.g. `-.->`, `-.-`) must be matched
/// longest-first or the parser will split a `-.->` as `-.-` + leftover `>`.
fn match_edge_connector_at(line: &str, pos: usize) -> Option<(EdgeKind, usize)> {
    let tail = &line[pos..];
    const CONNECTORS: &[(&str, EdgeKind)] = &[
        ("-.->", EdgeKind::DottedArrow),
        ("-.-x", EdgeKind::DottedCross),
        ("-.-o", EdgeKind::DottedCircle),
        ("-.-", EdgeKind::DottedLine),
        ("==>", EdgeKind::ThickArrow),
        ("==x", EdgeKind::ThickCross),
        ("==o", EdgeKind::ThickCircle),
        ("===", EdgeKind::ThickLine),
        ("-->", EdgeKind::SolidArrow),
        ("--x", EdgeKind::SolidCross),
        ("--o", EdgeKind::SolidCircle),
        ("---", EdgeKind::SolidLine),
    ];
    for (connector, kind) in CONNECTORS {
        if tail.starts_with(connector) {
            return Some((*kind, connector.len()));
        }
    }
    None
}

/// Find the first edge connector in `line`, returning its (kind, position, length).
fn find_first_edge_connector(line: &str) -> Option<(EdgeKind, usize, usize)> {
    let bytes = line.as_bytes();
    for pos in 0..bytes.len() {
        // Only consider start positions where the next char could begin a
        // connector ('-' or '=') to skip work.
        match bytes[pos] {
            b'-' | b'=' => {
                if let Some((kind, len)) = match_edge_connector_at(line, pos) {
                    return Some((kind, pos, len));
                }
            }
            _ => continue,
        }
    }
    None
}

/// Parse a Mermaid flowchart edge: `A --> B`, `A -->|label| B`,
/// `A -- label --> B`, `A -.-> B`, `A ==> B`, `A --x B`, `A --o B`, etc.
///
/// Returns `(generated_plantuml_lines, [referenced_ids])`. The id list
/// drives the bare-id backfill step in the outer adapter.
fn adapt_flowchart_edge(line: &str) -> Option<(String, Vec<String>)> {
    let (kind, arrow_pos, arrow_len) = find_first_edge_connector(line)?;
    let lhs_raw = line[..arrow_pos].trim();
    let rhs_raw = line[arrow_pos + arrow_len..].trim();

    // Handle `A -->|label| B` form: label is embedded in arrow suffix `|label|`.
    let (rhs_token, edge_label) = if let Some(stripped) = rhs_raw.strip_prefix('|') {
        if let Some(end_pipe) = stripped.find('|') {
            let label = stripped[..end_pipe].trim().to_string();
            let rhs_after = stripped[end_pipe + 1..].trim();
            (rhs_after, Some(label))
        } else {
            (rhs_raw, None)
        }
    } else {
        (rhs_raw, None)
    };

    // Handle `A -- label --> B` form: label is embedded in lhs `A -- label`.
    let (lhs_token, lhs_label) = if lhs_raw.contains(" -- ") {
        let idx = lhs_raw.rfind(" -- ")?;
        let id = lhs_raw[..idx].trim();
        let label = lhs_raw[idx + 4..].trim();
        (id, Some(label.to_string()))
    } else {
        (lhs_raw, None)
    };

    let (from_id, from_label, from_class) = parse_flowchart_node_token(lhs_token);
    let (to_id, to_label, to_class) = parse_flowchart_node_token(rhs_token);

    if from_id.is_empty() || to_id.is_empty() {
        return None;
    }

    let label = edge_label.or(lhs_label);
    let plantuml_arrow = kind.to_plantuml_arrow();
    let mut out = Vec::new();
    if from_label.is_some() || from_class.is_some() {
        out.push(format_flowchart_node_declaration(
            &from_id,
            from_label.as_deref(),
            from_class.as_deref(),
        ));
    }
    if to_label.is_some() || to_class.is_some() {
        out.push(format_flowchart_node_declaration(
            &to_id,
            to_label.as_deref(),
            to_class.as_deref(),
        ));
    }
    if let Some(lbl) = label {
        out.push(format!("{from_id} {plantuml_arrow} {to_id} : {lbl}"));
    } else {
        out.push(format!("{from_id} {plantuml_arrow} {to_id}"));
    }
    Some((out.join("\n"), vec![from_id, to_id]))
}
