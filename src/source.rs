#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub const fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

#[derive(Debug, Clone)]
pub struct Source {
    text: String,
}

impl Source {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }

    pub fn slice(&self, span: Span) -> &str {
        &self.text[span.start.min(self.text.len())..span.end.min(self.text.len())]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SourceMap {
    mappings: Vec<MappedSpan>,
}

impl SourceMap {
    pub fn new(mappings: Vec<MappedSpan>) -> Self {
        Self { mappings }
    }

    pub fn line_map(original: &str, generated: &str) -> Self {
        let original_spans = line_spans(original);
        let generated_spans = line_spans(generated);
        let mappings = generated_spans
            .into_iter()
            .enumerate()
            .map(|(idx, generated)| MappedSpan {
                generated,
                original: original_spans
                    .get(idx)
                    .copied()
                    .unwrap_or_else(|| Span::new(0, 0)),
                source: None,
            })
            .collect();
        Self { mappings }
    }

    pub fn map_span(&self, span: Span) -> Span {
        self.find_mapping(span)
            .map(|mapping| mapping.original)
            .unwrap_or(span)
    }

    pub fn len(&self) -> usize {
        self.mappings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }

    pub fn map_diagnostic(&self, mut diagnostic: crate::Diagnostic) -> crate::Diagnostic {
        if let Some(span) = diagnostic.span {
            if let Some(mapping) = self.find_mapping(span) {
                diagnostic.span = Some(mapping.original);
                if let Some(source) = &mapping.source {
                    diagnostic = diagnostic.with_source(source.clone());
                } else {
                    diagnostic.source = None;
                }
            }
        }
        diagnostic
    }

    fn find_mapping(&self, span: Span) -> Option<&MappedSpan> {
        if span.is_empty() {
            return self.mappings.iter().find(|mapping| {
                mapping.generated.start <= span.start && span.start <= mapping.generated.end
            });
        }

        self.mappings
            .iter()
            .find(|mapping| spans_overlap(mapping.generated, span))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MappedSpan {
    pub generated: Span,
    pub original: Span,
    pub source: Option<DiagnosticSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticSource {
    pub file: Option<String>,
    pub source_name: Option<String>,
    pub span: Span,
    pub line: usize,
    pub column: usize,
    pub snippet: String,
    pub caret: String,
    pub include_stack: Vec<String>,
}

impl DiagnosticSource {
    pub fn from_source(
        file: Option<String>,
        source_name: Option<String>,
        source: &str,
        span: Span,
        include_stack: Vec<String>,
    ) -> Self {
        let (line, column) = offset_to_line_col(source, span.start);
        let (line_start, line_end) = containing_line_bounds(source, span.start);
        let snippet = source[line_start..line_end].to_string();
        let caret = render_caret_marker(source, span);
        Self {
            file,
            source_name,
            span,
            line,
            column,
            snippet,
            caret,
            include_stack,
        }
    }
}

pub fn line_spans(source: &str) -> Vec<Span> {
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
    let col = source[line_start..off].chars().count() + 1;
    (line, col)
}

fn render_caret_marker(source: &str, span: Span) -> String {
    let (line_start, line_end) = containing_line_bounds(source, span.start);
    let line = &source[line_start..line_end];
    let caret_start = span.start.saturating_sub(line_start).min(line.len());
    let span_len = span
        .end
        .saturating_sub(span.start)
        .max(1)
        .min(line.len().saturating_sub(caret_start).max(1));
    format!(
        "{}{}",
        " ".repeat(line[..caret_start].chars().count()),
        "^".repeat(span_len.max(1))
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_len_and_empty_state_handle_reversed_bounds() {
        assert_eq!(Span::new(3, 8).len(), 5);
        assert_eq!(Span::new(8, 3).len(), 0);
        assert!(!Span::new(3, 8).is_empty());
        assert!(Span::new(8, 3).is_empty());
        assert!(Span::new(4, 4).is_empty());
    }

    #[test]
    fn source_slice_clamps_to_text_bounds() {
        let source = Source::new("diagram");

        assert_eq!(source.as_str(), "diagram");
        assert_eq!(source.slice(Span::new(2, 5)), "agr");
        assert_eq!(source.slice(Span::new(4, 99)), "ram");
        assert_eq!(source.slice(Span::new(99, 120)), "");
    }

    #[test]
    fn source_map_can_carry_include_origin() {
        let original = "!include child.puml\n";
        let origin = DiagnosticSource::from_source(
            Some("child.puml".to_string()),
            Some("child.puml".to_string()),
            original,
            Span::new(0, 19),
            vec!["root.puml".to_string(), "child.puml".to_string()],
        );
        let map = SourceMap::new(vec![MappedSpan {
            generated: Span::new(0, 12),
            original: Span::new(0, 19),
            source: Some(origin),
        }]);
        let diagnostic = crate::Diagnostic::error("bad").with_span(Span::new(3, 4));
        let mapped = map.map_diagnostic(diagnostic);

        assert_eq!(mapped.span, Some(Span::new(0, 19)));
        assert_eq!(mapped.source.as_ref().unwrap().line, 1);
        assert_eq!(
            mapped.source.as_ref().unwrap().file.as_deref(),
            Some("child.puml")
        );
    }
}
