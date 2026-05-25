use std::path::PathBuf;

use crate::diagnostic::Diagnostic;
use crate::source::{DiagnosticSource, MappedSpan, Span};

pub(super) fn push_mapped_line(
    out: &mut String,
    mappings: &mut Vec<MappedSpan>,
    line: &str,
    source: &str,
    raw_span: Span,
    include_stack: &[PathBuf],
) {
    let generated = Span::new(out.len(), out.len() + line.len());
    out.push_str(line);
    out.push('\n');
    mappings.push(MappedSpan {
        generated,
        original: raw_span,
        source: (!include_stack.is_empty())
            .then(|| preproc_source(source, raw_span, include_stack)),
    });
}

pub(super) fn annotate_preproc_diagnostic(
    diagnostic: Diagnostic,
    source: &str,
    raw_span: Span,
    include_stack: &[PathBuf],
) -> Diagnostic {
    if diagnostic.source.is_some() {
        return diagnostic;
    }
    let diagnostic = if diagnostic.span.is_some() {
        diagnostic
    } else {
        diagnostic.with_span(raw_span)
    };
    if include_stack.is_empty() {
        diagnostic
    } else {
        diagnostic.with_source(preproc_source(source, raw_span, include_stack))
    }
}

pub(super) fn preproc_source(
    source: &str,
    span: Span,
    include_stack: &[PathBuf],
) -> DiagnosticSource {
    let file = include_stack.last().map(|path| path.display().to_string());
    let source_name = file.clone().or_else(|| Some("<input>".to_string()));
    let include_stack = include_stack
        .iter()
        .map(|path| path.display().to_string())
        .collect();
    DiagnosticSource::from_source(file, source_name, source, span, include_stack)
}
