use super::EXIT_IO;
use puml::source::Span;
use puml::{extract_markdown_diagrams, Diagnostic, DiagramInput, FrontendSelection};
use std::fs;
use std::io::{self, Read};
use std::path::Path;

pub(super) struct InputDiagram {
    pub(super) source: String,
    pub(super) source_span: Option<Span>,
    pub(super) frontend_hint: Option<FrontendSelection>,
    pub(super) output_name_hint: Option<String>,
}

pub(super) fn read_input(
    path: Option<&Path>,
) -> Result<(String, String, Option<&Path>), (u8, String)> {
    match path {
        Some(p) if p != Path::new("-") => {
            let raw = fs::read_to_string(p)
                .map_err(|e| (EXIT_IO, format!("failed to read '{}': {e}", p.display())))?;
            Ok((p.display().to_string(), raw, Some(p)))
        }
        _ => {
            let mut raw = String::new();
            io::stdin()
                .read_to_string(&mut raw)
                .map_err(|e| (EXIT_IO, format!("failed to read stdin: {e}")))?;
            Ok(("stdin".to_string(), raw, None))
        }
    }
}

pub(super) fn should_extract_markdown(from_markdown_flag: bool, input_path: Option<&Path>) -> bool {
    if from_markdown_flag {
        return true;
    }

    input_path
        .and_then(|path| path.extension())
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "md" | "markdown" | "mdown"
            )
        })
        .unwrap_or(false)
}

pub(super) fn frontend_hint_for_path(path: Option<&Path>) -> Option<FrontendSelection> {
    path.and_then(|path| path.extension())
        .and_then(|ext| ext.to_str())
        .and_then(|ext| match ext.to_ascii_lowercase().as_str() {
            "picouml" => Some(FrontendSelection::Picouml),
            _ => None,
        })
}

pub(super) fn split_diagrams(
    raw: &str,
    from_markdown: bool,
    markdown_name_prefix: Option<&str>,
    file_frontend_hint: Option<FrontendSelection>,
) -> Result<Vec<InputDiagram>, Diagnostic> {
    if from_markdown {
        let diagrams = extract_markdown_diagrams(raw)
            .into_iter()
            .enumerate()
            .map(
                |(
                    idx,
                    DiagramInput {
                        source,
                        span_in_input,
                        fence_frontend,
                    },
                )| InputDiagram {
                    source,
                    source_span: Some(span_in_input),
                    frontend_hint: Some(fence_frontend),
                    output_name_hint: Some(match markdown_name_prefix {
                        Some(prefix) => format!("{prefix}_snippet_{}", idx + 1),
                        None => format!("snippet-{}", idx + 1),
                    }),
                },
            )
            .collect::<Vec<_>>();
        return Ok(diagrams);
    }

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let mut blocks = Vec::new();

    let has_startuml_marker = raw.lines().any(|line| {
        let marker = strip_inline_plantuml_comment(line).trim();
        matches_uml_marker(marker, "@startuml")
    });
    if has_startuml_marker {
        let mut current = Vec::new();
        let mut in_block = false;
        let mut block_start_line = 0usize;
        for (line_idx, line) in raw.lines().enumerate() {
            let marker = strip_inline_plantuml_comment(line).trim();
            if matches_uml_marker(marker, "@startuml") {
                if in_block {
                    return Err(Diagnostic::error(format!(
                        "unmatched @startuml/@enduml boundary: found @startuml at line {} before closing previous block started at line {}",
                        line_idx + 1,
                        block_start_line
                    )));
                }
                in_block = true;
                block_start_line = line_idx + 1;
                current.clear();
            }
            if matches_uml_marker(marker, "@enduml") && !in_block {
                return Err(Diagnostic::error(format!(
                    "unmatched @startuml/@enduml boundary: found @enduml at line {} without a preceding @startuml",
                    line_idx + 1
                )));
            }
            if in_block {
                current.push(line);
            }
            if in_block && matches_uml_marker(marker, "@enduml") {
                blocks.push(InputDiagram {
                    source: current.join("\n").trim().to_string(),
                    source_span: None,
                    frontend_hint: file_frontend_hint,
                    output_name_hint: None,
                });
                current.clear();
                in_block = false;
            }
        }
        if in_block {
            return Err(Diagnostic::error(format!(
                "unmatched @startuml/@enduml boundary: @startuml at line {} is missing a closing @enduml",
                block_start_line
            )));
        }
        if !blocks.is_empty() {
            return Ok(blocks);
        }
    }

    Ok(vec![InputDiagram {
        source: trimmed.to_string(),
        source_span: None,
        frontend_hint: file_frontend_hint,
        output_name_hint: None,
    }])
}

fn strip_inline_plantuml_comment(line: &str) -> &str {
    let mut in_quotes = false;
    for (idx, ch) in line.char_indices() {
        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }
        if ch == '\'' && !in_quotes {
            return &line[..idx];
        }
    }
    line
}

fn matches_uml_marker(line: &str, marker: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    if !lower.starts_with(marker) {
        return false;
    }
    let rest = &line[marker.len()..];
    rest.is_empty() || rest.starts_with(char::is_whitespace)
}
