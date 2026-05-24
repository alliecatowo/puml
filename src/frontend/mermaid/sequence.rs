use crate::{source::Span, Diagnostic};

use super::common::strip_mermaid_comment;

pub(super) fn adapt_mermaid_sequence(source: &str) -> Result<String, Diagnostic> {
    let mut out = Vec::new();
    let mut saw_non_empty = false;
    let mut saw_sequence_header = false;
    let mut block_stack: Vec<&'static str> = Vec::new();
    let mut offset = 0usize;

    for raw_line in source.lines() {
        let line = strip_mermaid_comment(raw_line).trim();
        let span = Span::new(offset, offset + raw_line.len());
        offset += raw_line.len() + 1;

        if line.is_empty() || line.starts_with("%%") {
            continue;
        }

        if !saw_non_empty {
            saw_non_empty = true;
            if line.eq_ignore_ascii_case("sequenceDiagram") {
                saw_sequence_header = true;
                continue;
            }
            return Err(Diagnostic::error_code(
                "E_MERMAID_FAMILY_UNSUPPORTED",
                "mermaid frontend currently supports sequence diagrams only (expected `sequenceDiagram`)",
            )
            .with_span(span));
        }

        if let Some(converted) = adapt_mermaid_declaration(line) {
            out.push(converted);
            continue;
        }

        if let Some(converted) = adapt_mermaid_message(line) {
            out.push(converted);
            continue;
        }

        if let Some(converted) = adapt_mermaid_note(line) {
            out.push(converted);
            continue;
        }

        if let Some(converted) = adapt_mermaid_lifecycle(line) {
            out.push(converted);
            continue;
        }

        if let Some(block) = adapt_mermaid_block(line) {
            match block {
                MermaidSequenceBlock::Start {
                    mermaid_kind,
                    output,
                } => {
                    block_stack.push(mermaid_kind);
                    out.push(output);
                }
                MermaidSequenceBlock::Else(output) => out.push(output),
                MermaidSequenceBlock::End => {
                    let output = if matches!(block_stack.pop(), Some("box")) {
                        "end box".to_string()
                    } else {
                        "end".to_string()
                    };
                    out.push(output);
                }
            }
            continue;
        }

        if let Some(converted) = adapt_mermaid_create_destroy(line) {
            out.push(converted);
            continue;
        }

        if let Some(converted) = adapt_mermaid_link(line) {
            out.push(converted);
            continue;
        }

        if line.eq_ignore_ascii_case("autonumber") {
            out.push("autonumber".to_string());
            continue;
        }

        if let Some(title) = line.strip_prefix("title ") {
            if !title.trim().is_empty() {
                out.push(format!("title {}", title.trim()));
                continue;
            }
        }

        if let Some(code) = classify_unsupported_mermaid_construct(line) {
            return Err(Diagnostic::error_code(
                code,
                format!("unsupported mermaid sequence construct: `{line}`"),
            )
            .with_span(span));
        }

        return Err(Diagnostic::error_code(
            "E_MERMAID_CONSTRUCT_UNSUPPORTED",
            format!("unsupported mermaid sequence construct: `{line}`"),
        )
        .with_span(span));
    }

    if !saw_sequence_header {
        return Err(Diagnostic::error_code(
            "E_MERMAID_EMPTY",
            "mermaid sequence input is empty or missing `sequenceDiagram` header",
        ));
    }

    Ok(out.join("\n"))
}

fn adapt_mermaid_declaration(line: &str) -> Option<String> {
    let mut words = line.split_ascii_whitespace();
    let head = words.next()?;
    if !matches!(head, "participant" | "actor") {
        return None;
    }
    let tail = words.collect::<Vec<_>>().join(" ");
    if tail.is_empty() {
        return None;
    }
    Some(format!("{head} {tail}"))
}

fn adapt_mermaid_message(line: &str) -> Option<String> {
    let (core, label) = line.split_once(':')?;
    let (from, arrow, to) = split_mermaid_message_core(core.trim())?;
    let mapped_arrow = match arrow {
        "->>" => "->>",
        "-->>" => "-->>",
        "->" => "->",
        "-->" => "-->",
        "-x" => "->x",
        "--x" => "-->x",
        "-)" => "->>",
        "--)" => "-->>",
        _ => return None,
    };

    Some(format!(
        "{} {} {}: {}",
        from.trim(),
        mapped_arrow,
        to.trim(),
        label.trim()
    ))
}

fn adapt_mermaid_note(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    if !lower.starts_with("note ") {
        return None;
    }
    let (head, body) = line.split_once(':')?;
    let prefix = &head["note ".len()..];
    let body = body.trim();
    if body.is_empty() {
        return None;
    }

    let lower_prefix = prefix.to_ascii_lowercase();
    if lower_prefix.starts_with("over ") {
        let target = prefix["over ".len()..].trim();
        if target.is_empty() {
            return None;
        }
        return Some(format!("note over {target}: {body}"));
    }
    if lower_prefix.starts_with("left of ") {
        let target = prefix["left of ".len()..].trim();
        if target.is_empty() {
            return None;
        }
        return Some(format!("note left of {target}: {body}"));
    }
    if lower_prefix.starts_with("right of ") {
        let target = prefix["right of ".len()..].trim();
        if target.is_empty() {
            return None;
        }
        return Some(format!("note right of {target}: {body}"));
    }
    None
}

fn adapt_mermaid_lifecycle(line: &str) -> Option<String> {
    let mut parts = line.split_ascii_whitespace();
    let head = parts.next()?;
    if !matches!(head, "activate" | "deactivate" | "destroy") {
        return None;
    }
    let target = parts.collect::<Vec<_>>().join(" ");
    if target.is_empty() {
        return None;
    }
    Some(format!("{head} {target}"))
}

fn split_mermaid_message_core(core: &str) -> Option<(&str, &str, &str)> {
    for arrow in ["-->>", "--)", "--x", "->>", "-->", "-)", "-x", "->"] {
        if let Some(idx) = core.find(arrow) {
            let lhs = core[..idx].trim();
            let rhs = core[idx + arrow.len()..].trim();
            if lhs.is_empty() || rhs.is_empty() {
                return None;
            }
            return Some((lhs, arrow, rhs));
        }
    }
    None
}

fn classify_unsupported_mermaid_construct(_line: &str) -> Option<&'static str> {
    // All previously-unsupported block/create/destroy/link constructs now
    // have explicit adapter routes (see `adapt_mermaid_block`,
    // `adapt_mermaid_create_destroy`, `adapt_mermaid_link`). Leaving this
    // hook in place keeps the diagnostic shape stable in case we need to
    // re-introduce targeted classifications later.
    None
}

enum MermaidSequenceBlock {
    Start {
        mermaid_kind: &'static str,
        output: String,
    },
    Else(String),
    End,
}

fn adapt_mermaid_block(line: &str) -> Option<MermaidSequenceBlock> {
    let first = line.split_ascii_whitespace().next()?.to_ascii_lowercase();
    match first.as_str() {
        "alt" => {
            let label = line["alt".len()..].trim();
            Some(MermaidSequenceBlock::Start {
                mermaid_kind: "alt",
                output: if label.is_empty() {
                    "alt".to_string()
                } else {
                    format!("alt {label}")
                },
            })
        }
        "else" => {
            let label = line["else".len()..].trim();
            Some(MermaidSequenceBlock::Else(if label.is_empty() {
                "else".to_string()
            } else {
                format!("else {label}")
            }))
        }
        "opt" => {
            let label = line["opt".len()..].trim();
            Some(MermaidSequenceBlock::Start {
                mermaid_kind: "opt",
                output: if label.is_empty() {
                    "opt".to_string()
                } else {
                    format!("opt {label}")
                },
            })
        }
        "loop" => {
            let label = line["loop".len()..].trim();
            Some(MermaidSequenceBlock::Start {
                mermaid_kind: "loop",
                output: if label.is_empty() {
                    "loop".to_string()
                } else {
                    format!("loop {label}")
                },
            })
        }
        "par" => {
            let label = line["par".len()..].trim();
            Some(MermaidSequenceBlock::Start {
                mermaid_kind: "par",
                output: if label.is_empty() {
                    "par".to_string()
                } else {
                    format!("par {label}")
                },
            })
        }
        "and" => {
            // Mermaid's `and` inside a par maps to PlantUML's `else` branch.
            let label = line["and".len()..].trim();
            Some(MermaidSequenceBlock::Else(if label.is_empty() {
                "else".to_string()
            } else {
                format!("else {label}")
            }))
        }
        "critical" => {
            let label = line["critical".len()..].trim();
            Some(MermaidSequenceBlock::Start {
                mermaid_kind: "critical",
                output: if label.is_empty() {
                    "critical".to_string()
                } else {
                    format!("critical {label}")
                },
            })
        }
        "option" => {
            // Mermaid `option` inside `critical` maps to PlantUML's `else`.
            let label = line["option".len()..].trim();
            Some(MermaidSequenceBlock::Else(if label.is_empty() {
                "else".to_string()
            } else {
                format!("else {label}")
            }))
        }
        "break" => {
            let label = line["break".len()..].trim();
            Some(MermaidSequenceBlock::Start {
                mermaid_kind: "break",
                output: if label.is_empty() {
                    "break".to_string()
                } else {
                    format!("break {label}")
                },
            })
        }
        "rect" => {
            // `rect rgb(...)` becomes a `group` block (color is dropped).
            let label = line["rect".len()..].trim();
            Some(MermaidSequenceBlock::Start {
                mermaid_kind: "rect",
                output: if label.is_empty() {
                    "group".to_string()
                } else {
                    format!("group {label}")
                },
            })
        }
        "box" => {
            let label = line["box".len()..].trim();
            Some(MermaidSequenceBlock::Start {
                mermaid_kind: "box",
                output: if label.is_empty() {
                    "box".to_string()
                } else {
                    format!("box {label}")
                },
            })
        }
        "end" => Some(MermaidSequenceBlock::End),
        _ => None,
    }
}

fn adapt_mermaid_create_destroy(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("create ") {
        // Mermaid form: `create participant X` or `create X`.
        let trimmed = line[7..].trim();
        let payload = if let Some(p) = trimmed.strip_prefix("participant ") {
            p.trim()
        } else if let Some(p) = trimmed.strip_prefix("actor ") {
            p.trim()
        } else {
            trimmed
        };
        if payload.is_empty() || rest.trim().is_empty() {
            return None;
        }
        return Some(format!("create {payload}"));
    }
    if let Some(rest) = lower.strip_prefix("destroy ") {
        let target = line[8..].trim();
        if target.is_empty() || rest.trim().is_empty() {
            return None;
        }
        return Some(format!("destroy {target}"));
    }
    None
}

fn adapt_mermaid_link(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    if !(lower.starts_with("link ") || lower.starts_with("links ")) {
        return None;
    }
    // We don't render real links yet, but we accept the syntax by collapsing
    // it to a benign comment-style placeholder that the downstream parser
    // will skip without complaint.
    Some(format!("' [link] {}", line.trim()))
}
