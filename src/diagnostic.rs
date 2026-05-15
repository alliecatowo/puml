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
}
