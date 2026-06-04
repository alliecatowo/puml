use crate::{
    source::{MappedSpan, SourceMap, Span},
    Diagnostic,
};

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
            source: None,
        });
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
                source: None,
            },
            MappedSpan {
                generated: Span::new(10, 33),
                original: Span::new(13, 43),
                source: None,
            },
            MappedSpan {
                generated: Span::new(34, 41),
                original: Span::new(0, 12),
                source: None,
            },
        ]);

        let diagnostic = Diagnostic::error("generated error").with_span(Span::new(12, 22));
        let mapped = source_map.map_diagnostic(diagnostic);

        assert_eq!(mapped.span, Some(Span::new(13, 43)));
        assert_eq!(mapped.line_col(original), Some((2, 1)));
        assert_eq!(SourceMap::line_map(original, generated).len(), 3);
    }

    #[test]
    fn source_map_fallback_to_last_original_span_for_extra_generated_lines() {
        let original = "flowchart TD\nA --> B\n";
        let generated = "@startuml\nA --> B\n'; extra synthetic line\n@enduml\n";
        let source_map = SourceMap::line_map(original, generated);

        let mut line_offsets = Vec::new();
        let mut offset = 0usize;
        for line in generated.lines() {
            let start = offset;
            let end = offset + line.len();
            line_offsets.push((start, end));
            offset += line.len() + 1;
        }

        assert_eq!(source_map.len(), 4);
        assert_eq!(line_offsets.len(), 4);
        let first = source_map.map_span(Span::new(line_offsets[0].0 + 1, line_offsets[0].0 + 2));
        let second = source_map.map_span(Span::new(line_offsets[1].0 + 1, line_offsets[1].0 + 2));
        let third = source_map.map_span(Span::new(line_offsets[2].0 + 1, line_offsets[2].0 + 2));
        let fourth = source_map.map_span(Span::new(line_offsets[3].0 + 1, line_offsets[3].0 + 2));
        assert_ne!(first, second);
        assert_eq!(second, third);
        assert_eq!(second, fourth);
    }
}
