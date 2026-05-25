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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticExplanation {
    pub code: Option<String>,
    pub summary: String,
    pub action: String,
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

pub fn explain_diagnostic(code: Option<&str>, message: Option<&str>) -> DiagnosticExplanation {
    let code = code.filter(|value| !value.trim().is_empty());
    let (summary, action) = match code.unwrap_or("") {
        "E_ARROW_INVALID" | "E_ENDPOINT_COMBINATION" | "E_SHORTCUT_INVALID"
        | "E_SHORTCUT_VIRTUAL" => (
            "The message arrow or endpoint syntax is not valid.",
            "Check the arrow token and both endpoints, then use a supported PlantUML-style sequence arrow.",
        ),
        "E_PARTICIPANT_DUPLICATE" => (
            "The same participant was declared more than once.",
            "Remove the duplicate declaration or give one participant a distinct alias.",
        ),
        "E_FAMILY_UNKNOWN" | "E_UNKNOWN" => (
            "The diagram family or statement could not be recognized.",
            "Verify the @start... header and the statement near the diagnostic span.",
        ),
        "E_INCLUDE_URL_DISABLED" | "E_IMPORT_URL_DISABLED" | "E_URL_DISABLED" => (
            "The source references a URL include, but URL includes are disabled for this entry point.",
            "Use a local include or enable URL includes in a caller that explicitly permits network access.",
        ),
        value if value.starts_with("E_INCLUDE_") || value.starts_with("E_IMPORT_") => (
            "The preprocessor could not resolve an include or import.",
            "Check the include path, stdlib pack name, and the configured include root.",
        ),
        value if value.starts_with("E_PREPROC_") => (
            "The preprocessor rejected a directive or macro expression.",
            "Check directive ordering, required arguments, and matching conditional blocks.",
        ),
        value
            if value.contains("UNCLOSED")
                || value.contains("UNMATCHED")
                || value.contains("MISMATCH") =>
        {
            (
                "A block delimiter or paired statement is missing or out of order.",
                "Add the matching start/end statement or move the unmatched statement into the correct block.",
            )
        }
        value if value.contains("UNSUPPORTED") => (
            "The syntax was recognized, but this renderer does not support that feature yet.",
            "Simplify the diagram or use a currently supported equivalent.",
        ),
        "" => (
            "No diagnostic code was provided.",
            "Use the message and span to inspect the source near the reported range.",
        ),
        _ => (
            "The language service reported a puml diagnostic.",
            "Inspect the message and source range, then adjust the diagram source accordingly.",
        ),
    };

    DiagnosticExplanation {
        code: code.map(str::to_string),
        summary: summary.to_string(),
        action: message
            .filter(|value| !value.trim().is_empty())
            .map(|value| format!("{action} Diagnostic message: {value}"))
            .unwrap_or_else(|| action.to_string()),
    }
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
