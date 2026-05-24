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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_document_returns_transport_neutral_full_document_edit() {
        let source = "@startuml\n  alt ok  \nAlice -> Bob\nend\n@enduml\n";

        let result = format_document(source);

        assert!(result.changed);
        assert_eq!(result.edits.len(), 1);
        assert_eq!(
            result.edits[0].span,
            Span {
                start: 0,
                end: source.len()
            }
        );
        assert_eq!(
            result.formatted,
            "@startuml\nalt ok\n  Alice -> Bob\nend\n@enduml\n"
        );
        assert_eq!(result.edits[0].new_text, result.formatted);
    }

    #[test]
    fn format_document_has_no_edit_when_already_formatted() {
        let source = "@startuml\nAlice -> Bob\n@enduml\n";
        let result = format_document(source);
        assert!(!result.changed);
        assert!(result.edits.is_empty());
        assert_eq!(result.formatted, source);
    }
}
