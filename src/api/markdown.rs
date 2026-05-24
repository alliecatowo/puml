use super::types::{DiagramInput, FrontendSelection};
use crate::source::Span;

pub fn extract_markdown_diagrams(source: &str) -> Vec<DiagramInput> {
    let mut out = Vec::new();
    let mut in_fence = false;
    let mut fence_marker = '`';
    let mut fence_len = 0usize;
    let mut fence_frontend = FrontendSelection::Auto;
    let mut content_start = 0usize;
    let mut cursor = 0usize;

    for line in source.split_inclusive('\n') {
        let line_start = cursor;
        cursor += line.len();

        let (marker, marker_count, rest) = parse_fence_line(line);

        if !in_fence {
            if marker_count >= 3 {
                if let Some(frontend) = parse_diagram_fence_frontend(rest) {
                    in_fence = true;
                    fence_marker = marker;
                    fence_len = marker_count;
                    fence_frontend = frontend;
                    content_start = cursor;
                }
            }
            continue;
        }

        if marker == fence_marker && marker_count >= fence_len && rest.is_empty() {
            let span = Span::new(content_start, line_start);
            out.push(DiagramInput {
                source: source[span.start.min(source.len())..span.end.min(source.len())]
                    .to_string(),
                span_in_input: span,
                fence_frontend,
            });
            in_fence = false;
            continue;
        }
    }

    if in_fence {
        let span = Span::new(content_start, source.len());
        out.push(DiagramInput {
            source: source[span.start.min(source.len())..span.end.min(source.len())].to_string(),
            span_in_input: span,
            fence_frontend,
        });
    }

    out
}

fn parse_fence_line(line: &str) -> (char, usize, &str) {
    let without_newline = line.trim_end_matches(['\n', '\r']);
    let leading_spaces = without_newline
        .as_bytes()
        .iter()
        .take_while(|&&b| b == b' ')
        .count();
    if leading_spaces > 3 {
        return ('\0', 0, without_newline);
    }

    let trimmed_line = &without_newline[leading_spaces..];
    let mut chars = trimmed_line.chars();
    let marker = match chars.next() {
        Some('`') => '`',
        Some('~') => '~',
        _ => return ('\0', 0, trimmed_line),
    };
    let marker_count = 1 + chars.take_while(|ch| *ch == marker).count();
    let rest = trimmed_line[marker_count..].trim();
    (marker, marker_count, rest)
}

fn parse_diagram_fence_frontend(info: &str) -> Option<FrontendSelection> {
    let lang = info.split_ascii_whitespace().next().unwrap_or_default();
    if lang.eq_ignore_ascii_case("mermaid") {
        return Some(FrontendSelection::Mermaid);
    }
    if lang.eq_ignore_ascii_case("picouml") {
        return Some(FrontendSelection::Picouml);
    }

    if is_plantuml_family_fence_lang(lang) {
        return Some(FrontendSelection::Auto);
    }

    None
}

fn is_plantuml_family_fence_lang(lang: &str) -> bool {
    lang.eq_ignore_ascii_case("puml")
        || lang.eq_ignore_ascii_case("pumlx")
        || lang.eq_ignore_ascii_case("plantuml")
        || lang.eq_ignore_ascii_case("uml")
        || lang.eq_ignore_ascii_case("puml-sequence")
        || lang.eq_ignore_ascii_case("uml-sequence")
}
