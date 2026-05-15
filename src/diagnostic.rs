use crate::source::Span;

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

impl Diagnostic {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
            severity: Severity::Error,
        }
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

fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
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
