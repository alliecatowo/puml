use crate::Diagnostic;

use super::strip_mermaid_comment;

/// Translate Mermaid `stateDiagram`/`stateDiagram-v2` to PlantUML.
///
/// Supported forms:
///   `[*] --> Still`      -> `[*] --> Still`
///   `Still --> Moving`   -> `Still --> Moving`
///   `state "label" as X` -> `state "label" as X`
///   `state X {`          -> `state X {`
///   `}`                  -> `}`
///   `note right of X ...` -> emitted as comment (notes not yet supported in state renderer)
pub(super) fn adapt(source: &str) -> Result<String, Diagnostic> {
    let mut out = vec!["@startuml".to_string()];
    let mut first = true;

    for raw_line in source.lines() {
        let line = strip_mermaid_comment(raw_line).trim();
        if line.is_empty() || line.starts_with("%%") {
            continue;
        }
        if first {
            first = false;
            // Skip the `stateDiagram` / `stateDiagram-v2` directive.
            continue;
        }

        // Transition lines: `X --> Y` or `X --> Y : label` - pass through.
        if line.contains("-->") {
            out.push(line.to_string());
            continue;
        }

        // `state "label" as X` - pass through (PlantUML syntax).
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("state ") {
            out.push(line.to_string());
            continue;
        }

        // `[*]` bare pseudo-state declaration - pass through.
        if line == "[*]" {
            out.push(line.to_string());
            continue;
        }

        // `}` closing block.
        if line == "}" {
            out.push("}".to_string());
            continue;
        }

        // `note`, `--` dividers, etc. - emit as benign comment.
        out.push(format!("' [stateDiagram] {line}"));
    }

    out.push("@enduml".to_string());
    Ok(out.join("\n"))
}
