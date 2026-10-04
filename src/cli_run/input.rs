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

fn strip_bom(raw: String) -> String {
    match raw.strip_prefix('\u{feff}') {
        Some(rest) => rest.to_string(),
        None => raw,
    }
}

pub(super) fn read_input(
    path: Option<&Path>,
) -> Result<(String, String, Option<&Path>), (u8, String)> {
    match path {
        Some(p) if p != Path::new("-") => {
            let raw = fs::read_to_string(p)
                .map_err(|e| (EXIT_IO, format!("failed to read '{}': {e}", p.display())))?;
            Ok((p.display().to_string(), strip_bom(raw), Some(p)))
        }
        _ => {
            let mut raw = String::new();
            io::stdin()
                .read_to_string(&mut raw)
                .map_err(|e| (EXIT_IO, format!("failed to read stdin: {e}")))?;
            Ok(("stdin".to_string(), strip_bom(raw), None))
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
            // Mermaid uses `.mmd` (canonical, Mermaid CLI's default) and the
            // less-common `.mermaid` extension on disk; map both to the
            // Mermaid frontend adapter so `puml foo.mmd` "just works".
            "mmd" | "mermaid" => Some(FrontendSelection::Mermaid),
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

    let has_start_marker = raw
        .lines()
        .any(|line| start_marker_kind(strip_inline_plantuml_comment(line).trim()).is_some());
    if has_start_marker {
        let mut current = Vec::new();
        // (kind, 1-based start line) of the block being collected.
        let mut open: Option<(&'static str, usize)> = None;
        for (line_idx, line) in raw.lines().enumerate() {
            let marker = strip_inline_plantuml_comment(line).trim();
            if let Some(kind) = start_marker_kind(marker) {
                if let Some((open_kind, open_line)) = open {
                    return Err(Diagnostic::error(format!(
                        "unmatched @start{open_kind}/@end{open_kind} boundary: found @start{kind} at line {} before closing previous block started at line {open_line}",
                        line_idx + 1,
                    )));
                }
                open = Some((kind, line_idx + 1));
                current.clear();
            } else if open.is_none() {
                if let Some(kind) = end_marker_kind(marker) {
                    return Err(Diagnostic::error(format!(
                        "unmatched @start{kind}/@end{kind} boundary: found @end{kind} at line {} without a preceding @start{kind}",
                        line_idx + 1
                    )));
                }
            }
            if open.is_some() {
                current.push(line);
            }
            if open.is_some() {
                // Any end marker closes the block; a start/end kind mismatch is
                // diagnosed downstream (e.g. E_PICOUML_MARKER_MIXED).
                if end_marker_kind(marker).is_some() {
                    blocks.push(InputDiagram {
                        source: current.join("\n").trim().to_string(),
                        source_span: None,
                        frontend_hint: file_frontend_hint,
                        output_name_hint: None,
                    });
                    current.clear();
                    open = None;
                }
            }
        }
        if let Some((kind, line)) = open {
            return Err(Diagnostic::error(format!(
                "unmatched @start{kind}/@end{kind} boundary: @start{kind} at line {line} is missing a closing @end{kind}"
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

/// Diagram kinds that can appear as `@start<kind>` ... `@end<kind>` blocks.
const BLOCK_KINDS: &[&str] = &[
    "uml",
    "mindmap",
    "wbs",
    "gantt",
    "json",
    "yaml",
    "salt",
    "ditaa",
    "regex",
    "ebnf",
    "math",
    "latex",
    "wire",
    "sdl",
    "nwdiag",
    "files",
    "chronology",
    "chen",
    "chart",
    "board",
    "archimate",
    "picouml",
];

fn start_marker_kind(marker: &str) -> Option<&'static str> {
    BLOCK_KINDS
        .iter()
        .copied()
        .find(|kind| matches_uml_marker(marker, &format!("@start{kind}")))
}

fn end_marker_kind(marker: &str) -> Option<&'static str> {
    BLOCK_KINDS
        .iter()
        .copied()
        .find(|kind| matches_uml_marker(marker, &format!("@end{kind}")))
}
