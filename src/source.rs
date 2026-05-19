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
}
