use crate::frontend::{FrontendBuilder, FrontendResult};
use crate::{source::Span, Diagnostic};

use super::common::strip_mermaid_comment;

// ---------------------------------------------------------------------------
// flowchart / graph adapter → PlantUML component-style
// ---------------------------------------------------------------------------

/// Translate a Mermaid `flowchart TD` / `graph TD` block into a PlantUML
/// component-style diagram that the existing component renderer accepts.
///
/// Node shapes handled:
///   `A[Label]`   → `component "Label" as A`
///   `A{Label}`   → `component "Label" as A`   (decision – best-effort)
///   `A(Label)`   → `component "Label" as A`
///   `A`          → bare id kept as `component A`
///
/// Arrow forms:
///   `A --> B`           → `A --> B`
///   `A -->|cond| B`     → `A --> B : cond`
///   `A -- text --> B`   → `A --> B : text`
pub(super) fn adapt_mermaid_flowchart(source: &str) -> Result<FrontendResult, Diagnostic> {
    use std::collections::BTreeMap;

    let mut out = FrontendBuilder::new();
    let mut first = true;
    let mut directive_span = Span::new(0, 0);
    let mut class_defs: BTreeMap<String, String> = BTreeMap::new();
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

        if let Some((converted, warning)) = adapt_flowchart_style(line, &class_defs, span) {
            if let Some(warning) = warning {
                out.push_diagnostic(warning);
            }
            out.push_lines(converted, span);
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
        if let Some(converted) = adapt_flowchart_edge(line) {
            out.push_lines(converted, span);
            continue;
        }

        // Node declaration: `ID[Label]`, `ID{Label}`, `ID(Label)`, bare `ID`.
        if let Some(converted) = adapt_flowchart_node(line) {
            out.push_line(converted, span);
            continue;
        }

        // Subgraph / end – map to `package`/`end`.
        if let Some(rest) = line.strip_prefix("subgraph ") {
            let label = rest.trim().trim_matches('"');
            out.push_line(format!("package \"{label}\" {{"), span);
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if lower == "end" || lower == "end subgraph" {
            out.push_line("}", span);
            continue;
        }

        return Err(Diagnostic::error_code(
            "E_MERMAID_FEATURE_LOSS",
            format!("unsupported mermaid flowchart construct would be dropped: `{line}`"),
        )
        .with_span(span));
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

/// Extract a node's canonical id and optional label from Mermaid node syntax.
/// Returns `(id, label)`.
fn parse_flowchart_node_id_label(token: &str) -> (String, Option<String>) {
    // Match `ID[Label]`, `ID{Label}`, `ID(Label)`, `ID((Label))`.
    for (open, close) in [("[", "]"), ("{", "}"), ("((", "))"), ("(", ")")] {
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
    // Bare id.
    (token.trim().to_string(), None)
}

fn adapt_flowchart_node(line: &str) -> Option<String> {
    // Must not contain `-->` or `---` to be a pure node declaration.
    if line.contains("-->") || line.contains("---") || line.contains("-.->") {
        return None;
    }
    let (id, label, class_name) = parse_flowchart_node_token(line);
    if id.is_empty() {
        return None;
    }
    Some(format_flowchart_node_declaration(
        &id,
        label.as_deref(),
        class_name.as_deref(),
    ))
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
) -> Option<(String, Option<Diagnostic>)> {
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
        return Some((format!("component \"{id}\" as {id} {fill}"), warning));
    }
    if let Some(rest) = line.trim().strip_prefix("class ") {
        let mut parts = rest.split_ascii_whitespace();
        let ids = parts.next()?;
        let class_name = parts.next()?;
        let fill = class_defs.get(class_name)?;
        let lines = ids
            .split(',')
            .filter(|id| !id.trim().is_empty())
            .map(|id| {
                let id = id.trim();
                format!("component \"{id}\" as {id} {fill}")
            })
            .collect::<Vec<_>>();
        if lines.is_empty() {
            return None;
        }
        return Some((lines.join("\n"), None));
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

/// Parse a Mermaid flowchart edge: `A --> B`, `A -->|label| B`,
/// `A -- label --> B`, `A -.-> B`, etc.
fn adapt_flowchart_edge(line: &str) -> Option<String> {
    // Detect edge by presence of `-->`, `-.->`, `-->`
    let arrow_forms = ["-.->", "-->", "---"];
    let mut best: Option<(usize, &str)> = None;
    for arrow in &arrow_forms {
        if let Some(pos) = line.find(arrow) {
            if best.is_none_or(|(p, _)| pos < p) {
                best = Some((pos, arrow));
            }
        }
    }
    let (arrow_pos, arrow_str) = best?;

    let lhs_raw = line[..arrow_pos].trim();
    let rhs_raw = line[arrow_pos + arrow_str.len()..].trim();

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
    let plantuml_arrow = if arrow_str == "-.->)" { "..>" } else { "-->" };
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
    Some(out.join("\n"))
}
