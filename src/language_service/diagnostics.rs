use crate::diagnostic::{
    diagnostic_code, normalized_warnings, offset_to_line_col, Diagnostic, Severity,
};
use crate::source::Span;
use crate::{normalize_family, parse_with_pipeline_result_options, ParsePipelineOptions};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticsReport {
    pub diagnostics: Vec<LanguageDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageDiagnostic {
    pub code: Option<String>,
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
    pub range: Option<SourceRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceRange {
    pub start: SourcePosition,
    pub end: SourcePosition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourcePosition {
    /// One-based line number in the source document.
    pub line: usize,
    /// One-based Unicode scalar column in the source line.
    pub column: usize,
}

pub fn diagnostics(source: &str) -> DiagnosticsReport {
    diagnostics_with_options(source, &ParsePipelineOptions::default())
}

pub fn diagnostics_with_options(source: &str, options: &ParsePipelineOptions) -> DiagnosticsReport {
    let diagnostics = match parse_with_pipeline_result_options(source, options) {
        Ok(result) => {
            let frontend_diagnostics = result.diagnostics;
            match normalize_family(result.document) {
                Ok(model) => frontend_diagnostics
                    .iter()
                    .chain(normalized_warnings(&model).iter())
                    .map(|diagnostic| language_diagnostic(source, diagnostic))
                    .collect(),
                Err(diagnostic) => frontend_diagnostics
                    .iter()
                    .chain(std::iter::once(&diagnostic))
                    .map(|diagnostic| language_diagnostic(source, diagnostic))
                    .collect(),
            }
        }
        Err(diagnostic) => vec![language_diagnostic(source, &diagnostic)],
    };

    DiagnosticsReport { diagnostics }
}

fn language_diagnostic(source: &str, diagnostic: &Diagnostic) -> LanguageDiagnostic {
    LanguageDiagnostic {
        code: diagnostic_code(&diagnostic.message),
        severity: diagnostic.severity,
        message: diagnostic.message.clone(),
        span: diagnostic.span,
        range: diagnostic.span.map(|span| SourceRange {
            start: source_position(source, span.start),
            end: source_position(source, span.end.max(span.start + 1)),
        }),
    }
}

fn source_position(source: &str, offset: usize) -> SourcePosition {
    let (line, column) = offset_to_line_col(source, offset);
    SourcePosition { line, column }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostics_extracts_code_and_range() {
        let source = "@startuml\nfoo bar\n@enduml\n";
        let report = diagnostics(source);
        assert!(!report.diagnostics.is_empty());
        assert_eq!(
            report.diagnostics[0].code.as_deref(),
            Some("E_FAMILY_UNKNOWN")
        );
        assert!(report.diagnostics[0].range.is_none());
    }
}
