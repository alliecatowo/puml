mod classdiagram;
mod erdiagram;
mod flowchart;
mod sequence;
mod statediagram;

use crate::{source::Span, Diagnostic};

/// Top-level Mermaid -> PlantUML adapter. Inspects the leading directive and
/// routes to the appropriate family-specific sub-adapter.
pub(crate) fn adapt_to_plantuml(source: &str) -> Result<String, Diagnostic> {
    // Scan for the first non-empty, non-comment line to detect the family.
    let mut first_directive: Option<(&str, Span)> = None;
    let mut offset = 0usize;
    for raw_line in source.lines() {
        let span = Span::new(offset, offset + raw_line.len());
        offset += raw_line.len() + 1;
        let line = strip_mermaid_comment(raw_line).trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        first_directive = Some((line, span));
        break;
    }

    let (directive, directive_span) = match first_directive {
        Some(d) => d,
        None => {
            return Err(Diagnostic::error_code(
                "E_MERMAID_EMPTY",
                "mermaid input is empty or contains only comments",
            ));
        }
    };

    let lower = directive.to_ascii_lowercase();
    // Route by leading directive keyword.
    if lower == "sequencediagram" {
        return sequence::adapt(source);
    }
    if lower.starts_with("flowchart ") || lower.starts_with("graph ") {
        return flowchart::adapt(source);
    }
    if lower == "classdiagram" {
        return classdiagram::adapt(source);
    }
    if lower == "statediagram" || lower == "statediagram-v2" {
        return statediagram::adapt(source);
    }
    if lower == "erdiagram" {
        return erdiagram::adapt(source);
    }

    Err(Diagnostic::error_code(
        "E_MERMAID_FAMILY_UNSUPPORTED",
        format!(
            "mermaid frontend does not support this diagram type: `{directive}`; \
             supported families are sequenceDiagram, flowchart, classDiagram, stateDiagram, erDiagram"
        ),
    )
    .with_span(directive_span))
}

fn strip_mermaid_comment(line: &str) -> &str {
    line.split_once("%%").map_or(line, |(prefix, _)| prefix)
}
