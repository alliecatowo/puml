use crate::{source::Span, Diagnostic};

pub(crate) mod mermaid;
pub(crate) mod picouml;

#[derive(Debug, Clone)]
pub(crate) struct FrontendResult {
    pub source: String,
    pub source_map: SourceMap,
    pub diagnostics: Vec<Diagnostic>,
}

impl FrontendResult {
    pub fn new(source: String, source_map: SourceMap) -> Self {
        Self {
            source,
            source_map,
            diagnostics: Vec::new(),
        }
    }

    pub fn with_diagnostics(mut self, diagnostics: Vec<Diagnostic>) -> Self {
        self.diagnostics = diagnostics;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SourceMap {
    mappings: Vec<MappedSpan>,
}

impl SourceMap {
    pub fn new(mappings: Vec<MappedSpan>) -> Self {
        Self { mappings }
    }

    pub fn line_map(original: &str, generated: &str) -> Self {
        let original_spans = line_spans(original);
        let generated_spans = line_spans(generated);
        let fallback = original_spans
            .last()
            .copied()
            .unwrap_or_else(|| Span::new(0, 0));
        let mappings = generated_spans
            .into_iter()
            .enumerate()
            .map(|(idx, generated)| MappedSpan {
                generated,
                original: original_spans.get(idx).copied().unwrap_or(fallback),
            })
            .collect();
        Self { mappings }
    }

    pub fn map_span(&self, span: Span) -> Span {
        if span.is_empty() {
            return self
                .mappings
                .iter()
                .find(|mapping| {
                    mapping.generated.start <= span.start && span.start <= mapping.generated.end
                })
                .map(|mapping| mapping.original)
                .unwrap_or(span);
        }

        self.mappings
            .iter()
            .find(|mapping| spans_overlap(mapping.generated, span))
            .map(|mapping| mapping.original)
            .unwrap_or(span)
    }

    pub fn map_diagnostic(&self, mut diagnostic: Diagnostic) -> Diagnostic {
        if let Some(span) = diagnostic.span {
            diagnostic.span = Some(self.map_span(span));
        }
        diagnostic
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MappedSpan {
    pub generated: Span,
    pub original: Span,
}

pub(crate) struct FrontendBuilder {
    source: String,
    mappings: Vec<MappedSpan>,
    diagnostics: Vec<Diagnostic>,
}

impl FrontendBuilder {
    pub fn new() -> Self {
        Self {
            source: String::new(),
            mappings: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn push_line(&mut self, line: impl AsRef<str>, original: Span) {
        let line = line.as_ref();
        let generated = Span::new(self.source.len(), self.source.len() + line.len());
        self.source.push_str(line);
        self.source.push('\n');
        self.mappings.push(MappedSpan {
            generated,
            original,
        });
    }

    pub fn push_lines(&mut self, lines: impl AsRef<str>, original: Span) {
        for line in lines.as_ref().lines() {
            self.push_line(line, original);
        }
    }

    pub fn push_diagnostic(&mut self, diagnostic: Diagnostic) {
        self.diagnostics.push(diagnostic);
    }

    pub fn finish(self) -> FrontendResult {
        FrontendResult::new(
            self.source.trim_end_matches('\n').to_string(),
            SourceMap::new(self.mappings),
        )
        .with_diagnostics(self.diagnostics)
    }
}

fn line_spans(source: &str) -> Vec<Span> {
    let mut spans = Vec::new();
    let mut offset = 0usize;
    for raw_line in source.lines() {
        spans.push(Span::new(offset, offset + raw_line.len()));
        offset += raw_line.len() + 1;
    }
    spans
}

fn spans_overlap(left: Span, right: Span) -> bool {
    left.start < right.end && right.start < left.end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Diagnostic;

    #[test]
    fn source_map_rewrites_generated_diagnostic_to_original_span() {
        let original = "flowchart TD\nclick A \"https://example.test\"\n";
        let generated = "@startuml\n' generated placeholder\n@enduml";
        let source_map = SourceMap::new(vec![
            MappedSpan {
                generated: Span::new(0, 9),
                original: Span::new(0, 12),
            },
            MappedSpan {
                generated: Span::new(10, 33),
                original: Span::new(13, 43),
            },
            MappedSpan {
                generated: Span::new(34, 41),
                original: Span::new(0, 12),
            },
        ]);

        let diagnostic = Diagnostic::error("generated error").with_span(Span::new(12, 22));
        let mapped = source_map.map_diagnostic(diagnostic);

        assert_eq!(mapped.span, Some(Span::new(13, 43)));
        assert_eq!(mapped.line_col(original), Some((2, 1)));
        assert_eq!(SourceMap::line_map(original, generated).mappings.len(), 3);
    }

    #[test]
    fn source_map_fallback_to_last_original_span_for_extra_generated_lines() {
        let original = "flowchart TD\nA --> B\n";
        let generated = "@startuml\nA --> B\n'; extra synthetic line\n@enduml\n";

        let mappings = SourceMap::line_map(original, generated).mappings;
        assert_eq!(mappings.len(), 4);
        assert_eq!(mappings[0].original, Span::new(0, 12));
        assert_eq!(mappings[1].original, Span::new(13, 20));
        assert_eq!(mappings[2].original, Span::new(13, 20));
        assert_eq!(mappings[3].original, Span::new(13, 20));
    }
}
