use crate::formatter;
use crate::source::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    pub span: Span,
    pub new_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatDocumentResult {
    pub edits: Vec<TextEdit>,
    pub formatted: String,
    pub changed: bool,
}

pub fn format_document(source: &str) -> FormatDocumentResult {
    let formatted = formatter::format_source(source);
    let edits = if formatted.changed {
        vec![TextEdit {
            span: Span {
                start: 0,
                end: source.len(),
            },
            new_text: formatted.formatted.clone(),
        }]
    } else {
        Vec::new()
    };
    FormatDocumentResult {
        edits,
        formatted: formatted.formatted,
        changed: formatted.changed,
    }
}
