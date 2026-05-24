use crate::frontend::{FrontendBuilder, FrontendResult};
use crate::{source::Span, Diagnostic};

use super::common::strip_mermaid_comment;

// ---------------------------------------------------------------------------
// stateDiagram adapter → PlantUML state diagram
// ---------------------------------------------------------------------------

/// Translate Mermaid `stateDiagram`/`stateDiagram-v2` to PlantUML.
///
/// Supported forms:
///   `[*] --> Still`      → `[*] --> Still`
///   `Still --> Moving`   → `Still --> Moving`
///   `state "label" as X` → `state "label" as X`
///   `state X {`          → `state X {`
///   `}`                  → `}`
///   `note right of X ...` → emitted as comment (notes not yet supported in state renderer)
pub(super) fn adapt_mermaid_statediagram(source: &str) -> Result<FrontendResult, Diagnostic> {
    let mut out = FrontendBuilder::new();
    let mut first = true;
    let mut directive_span = Span::new(0, 0);
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
            // Skip the `stateDiagram` / `stateDiagram-v2` directive.
            continue;
        }

        // Transition lines: `X --> Y` or `X --> Y : label` – pass through.
        if line.contains("-->") {
            out.push_line(line, span);
            continue;
        }

        // `state "label" as X` – pass through (PlantUML syntax).
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("state ") {
            out.push_line(line, span);
            continue;
        }

        // `[*]` bare pseudo-state declaration – pass through.
        if line == "[*]" {
            out.push_line(line, span);
            continue;
        }

        // `}` closing block.
        if line == "}" {
            out.push_line("}", span);
            continue;
        }

        // Preserve historical render behavior by deferring unsupported state
        // constructs as comments, but report the feature loss explicitly.
        out.push_diagnostic(
            Diagnostic::warning(format!(
                "[W_MERMAID_STATE_DEFERRED] unsupported mermaid state construct was deferred: `{line}`"
            ))
            .with_span(span),
        );
        out.push_line(format!("' [stateDiagram] {line}"), span);
    }

    out.push_line("@enduml", directive_span);
    Ok(out.finish())
}
