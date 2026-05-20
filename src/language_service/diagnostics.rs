use crate::diagnostic::{Diagnostic, Severity};
use crate::source::Span;
use crate::{
    normalize_family, parse_with_pipeline_options, NormalizedDocument, ParsePipelineOptions,
};

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
    let diagnostics = match parse_with_pipeline_options(source, options).and_then(normalize_family)
    {
        Ok(model) => normalized_warnings(&model)
            .iter()
            .map(|diagnostic| language_diagnostic(source, diagnostic))
            .collect(),
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

fn normalized_warnings(model: &NormalizedDocument) -> &[Diagnostic] {
    match model {
        NormalizedDocument::Sequence(sequence) => &sequence.warnings,
        NormalizedDocument::Family(family) => &family.warnings,
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
        NormalizedDocument::Chen(doc) => &doc.warnings,
    }
}

fn diagnostic_code(message: &str) -> Option<String> {
    let rest = message.strip_prefix('[')?;
    let (code, _tail) = rest.split_once("] ")?;
    if code.is_empty() {
        None
    } else {
        Some(code.to_string())
    }
}

fn source_position(source: &str, offset: usize) -> SourcePosition {
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
    SourcePosition {
        line,
        column: source[line_start..off].chars().count() + 1,
    }
}
