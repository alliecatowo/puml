use super::FrontendResult;
use crate::{source::Span, Diagnostic};

mod class;
mod common;
mod er;
mod flowchart;
mod sequence;
mod state;

use class::adapt_mermaid_classdiagram;
use common::{result_from_adapted, strip_mermaid_comment};
use er::adapt_mermaid_erdiagram;
use flowchart::adapt_mermaid_flowchart;
use sequence::adapt_mermaid_sequence;
use state::adapt_mermaid_statediagram;

pub(crate) fn adapt(source: &str) -> Result<FrontendResult, Diagnostic> {
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
        return adapt_mermaid_sequence(source).map(|adapted| result_from_adapted(source, adapted));
    }
    if lower.starts_with("flowchart ") || lower.starts_with("graph ") {
        return adapt_mermaid_flowchart(source);
    }
    if lower == "classdiagram" {
        return adapt_mermaid_classdiagram(source)
            .map(|adapted| result_from_adapted(source, adapted));
    }
    if lower == "statediagram" || lower == "statediagram-v2" {
        return adapt_mermaid_statediagram(source);
    }
    if lower == "erdiagram" {
        return adapt_mermaid_erdiagram(source).map(|adapted| result_from_adapted(source, adapted));
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
