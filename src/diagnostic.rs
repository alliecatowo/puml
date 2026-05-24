use crate::model::NormalizedDocument;
use crate::source::Span;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub span: Option<Span>,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DiagnosticJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    pub code: Option<String>,
    pub severity: &'static str,
    pub message: String,
    pub span: Option<DiagnosticSpanJson>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub snippet: Option<String>,
    pub caret: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct DiagnosticSpanJson {
    pub start: usize,
    pub end: usize,
}

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
            severity: Severity::Error,
        }
    }

    pub fn error_code(code: &str, message: impl AsRef<str>) -> Self {
        Self::error(format!("[{code}] {}", message.as_ref()))
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
            severity: Severity::Warning,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn line_col(&self, source: &str) -> Option<(usize, usize)> {
        let span = self.span?;
        Some(offset_to_line_col(source, span.start))
    }

    pub fn render_with_source(&self, source: &str) -> String {
        if let Some(span) = self.span {
            let (line, col) = offset_to_line_col(source, span.start);
            let caret = render_caret_line(source, span);
            format!(
                "{} at line {}, column {}\n{}",
                self.message, line, col, caret
            )
        } else {
            self.message.clone()
        }
    }

    pub fn to_json_with_source(&self, source: &str) -> DiagnosticJson {
        let code = diagnostic_code(&self.message);
        let (line, column) = self
            .span
            .map(|span| offset_to_line_col(source, span.start))
            .map(|(l, c)| (Some(l), Some(c)))
            .unwrap_or((None, None));

        let (snippet, caret) = if let Some(span) = self.span {
            let (line_start, line_end) = containing_line_bounds(source, span.start);
            let line_src = source[line_start..line_end].to_string();
            let marker = render_caret_line(source, span)
                .lines()
                .nth(1)
                .unwrap_or_default()
                .to_string();
            (Some(line_src), Some(marker))
        } else {
            (None, None)
        };

        DiagnosticJson {
            file: None,
            code,
            severity: match self.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            },
            message: self.message.clone(),
            span: self.span.map(|s| DiagnosticSpanJson {
                start: s.start,
                end: s.end,
            }),
            line,
            column,
            snippet,
            caret,
        }
    }
}

pub fn diagnostic_code(message: &str) -> Option<String> {
    if let Some(rest) = message.strip_prefix('[') {
        if let Some((code, _message_tail)) = rest.split_once("] ") {
            if !code.is_empty() {
                return Some(code.to_string());
            }
        }
    }
    None
}

pub fn diagnostic_message_and_code(message: &str) -> (&str, Option<&str>) {
    let Some(rest) = message.strip_prefix('[') else {
        return (message, None);
    };
    let Some((code, message_tail)) = rest.split_once("] ") else {
        return (message, None);
    };
    if code.is_empty() {
        return (message, None);
    }
    (message_tail, Some(code))
}

pub fn normalized_warnings(model: &NormalizedDocument) -> &[Diagnostic] {
    match model {
        NormalizedDocument::Sequence(sequence) => &sequence.warnings,
        NormalizedDocument::Family(family) => &family.warnings,
        NormalizedDocument::FamilyPages(pages) => pages
            .iter()
            .find_map(|page| (!page.warnings.is_empty()).then_some(page.warnings.as_slice()))
            .unwrap_or(&[]),
        NormalizedDocument::Timeline(timeline) => &timeline.warnings,
        NormalizedDocument::State(state) => &state.warnings,
        NormalizedDocument::Json(doc) => &doc.warnings,
        NormalizedDocument::Yaml(doc) => &doc.warnings,
        NormalizedDocument::Nwdiag(doc) => &doc.warnings,
        NormalizedDocument::Archimate(doc) => &doc.warnings,
        NormalizedDocument::Regex(doc) => &doc.warnings,
        NormalizedDocument::Ebnf(doc) => &doc.warnings,
        NormalizedDocument::Math(doc) => &doc.warnings,
        NormalizedDocument::Sdl(doc) => &doc.warnings,
        NormalizedDocument::Ditaa(doc) => &doc.warnings,
        NormalizedDocument::Chart(doc) => &doc.warnings,
    }
}

pub fn render_caret_line(source: &str, span: Span) -> String {
    let (line_start, line_end) = containing_line_bounds(source, span.start);
    let line = &source[line_start..line_end];
    let caret_start = span.start.saturating_sub(line_start).min(line.len());
    let span_len = span
        .end
        .saturating_sub(span.start)
        .max(1)
        .min(line.len().saturating_sub(caret_start).max(1));
    let marker = format!(
        "{}{}",
        " ".repeat(byte_col_to_visual_col(&line[..caret_start])),
        "^".repeat(span_len.max(1))
    );
    format!("{line}\n{marker}")
}

fn containing_line_bounds(source: &str, offset: usize) -> (usize, usize) {
    let off = offset.min(source.len());
    let start = source[..off].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = source[off..]
        .find('\n')
        .map(|i| off + i)
        .unwrap_or(source.len());
    (start, end)
}

pub fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let off = offset.min(source.len());
    let mut line = 1usize;
    let mut line_start = 0usize;
    for (idx, ch) in source.char_indices() {
        if idx >= off {
            break;
        }
        if ch == '\n' {
            line += 1;
            line_start = idx + 1;
        }
    }
    let col = byte_col_to_visual_col(&source[line_start..off]) + 1;
    (line, col)
}

fn byte_col_to_visual_col(s: &str) -> usize {
    s.chars().count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_and_render_with_source_handle_spans_and_plain_messages() {
        let source = "alpha\nβeta line\nomega";
        let diagnostic = Diagnostic::error("bad token").with_span(Span { start: 6, end: 10 });

        assert_eq!(diagnostic.line_col(source), Some((2, 1)));
        assert_eq!(
            diagnostic.render_with_source(source),
            "bad token at line 2, column 1\nβeta line\n^^^^"
        );

        let plain = Diagnostic::warning("heads up");
        assert_eq!(plain.line_col(source), None);
        assert_eq!(plain.render_with_source(source), "heads up");
    }

    #[test]
    fn to_json_with_source_includes_code_location_and_snippet() {
        let source = "alpha\nβeta line\nomega";
        let diagnostic =
            Diagnostic::error_code("E_UTF", "bad token").with_span(Span { start: 6, end: 8 });

        let json = diagnostic.to_json_with_source(source);
        assert_eq!(json.code.as_deref(), Some("E_UTF"));
        assert_eq!(json.severity, "error");
        assert_eq!(json.line, Some(2));
        assert_eq!(json.column, Some(1));
        assert_eq!(json.snippet.as_deref(), Some("βeta line"));
        assert_eq!(json.caret.as_deref(), Some("^^"));
    }

    #[test]
    fn to_json_with_source_without_span_keeps_optional_fields_empty() {
        let diagnostic = Diagnostic::warning("[] not a coded warning");
        let json = diagnostic.to_json_with_source("source");

        assert_eq!(json.code, None);
        assert_eq!(json.severity, "warning");
        assert_eq!(json.line, None);
        assert_eq!(json.column, None);
        assert_eq!(json.snippet, None);
        assert_eq!(json.caret, None);
    }

    #[test]
    fn render_caret_line_clamps_to_line_bounds_and_marks_zero_length_spans() {
        let source = "abc\ndef";

        assert_eq!(
            render_caret_line(source, Span { start: 4, end: 400 }),
            "def\n^^^"
        );
        assert_eq!(
            render_caret_line(source, Span { start: 4, end: 4 }),
            "def\n^"
        );
    }

    #[test]
    fn coded_messages_require_non_empty_bracket_prefix() {
        let coded = Diagnostic::error_code("E_PARSE", "unexpected token");
        let empty = Diagnostic::warning("[] missing code");
        let plain = Diagnostic::warning("[oops]missing separator");

        assert_eq!(
            coded.to_json_with_source("x").code.as_deref(),
            Some("E_PARSE")
        );
        assert_eq!(empty.to_json_with_source("x").code, None);
        assert_eq!(plain.to_json_with_source("x").code, None);
    }
}
