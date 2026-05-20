use super::*;
use crate::source::Span;

#[test]
fn semantic_tokens_prefer_longest_operator_match() {
    let source = "@startuml\nAlice --> Bob\n@enduml\n";

    let tokens = semantic_tokens(source);

    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].kind, SemanticTokenKind::Operator);
    assert_eq!(&source[tokens[0].span.start..tokens[0].span.end], "-->");
}

#[test]
fn semantic_tokens_keep_stable_order_and_keyword_boundaries() {
    let source = "@startuml\nparticipant Alice\nAlice -> Bob\nparticipantAlias -> Bob\n@enduml\n";

    let tokens = semantic_tokens(source);
    let rendered = tokens
        .iter()
        .map(|token| (&source[token.span.start..token.span.end], token.kind))
        .collect::<Vec<_>>();

    assert_eq!(
        rendered,
        vec![
            ("participant", SemanticTokenKind::Keyword),
            ("->", SemanticTokenKind::Operator),
            ("->", SemanticTokenKind::Operator),
        ]
    );
}

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
fn hover_returns_completion_docs_for_symbol_and_word() {
    let source = "@startuml\nAlice --> Bob\nparticipant User\n@enduml\n";

    let symbol_hover = hover(source, (1, 7)).expect("symbol hover should resolve");
    assert!(symbol_hover.markdown.contains("`-->`"));
    assert!(symbol_hover.markdown.contains("Dashed message arrow."));

    let keyword_hover = hover(source, (2, 2)).expect("keyword hover should resolve");
    assert!(keyword_hover.markdown.contains("`participant`"));
    assert!(keyword_hover.markdown.contains("Declare a participant."));
}

#[test]
fn hover_falls_back_to_word_literal_for_unknown_identifier() {
    let source = "@startuml\nfoobar\n@enduml\n";
    let h = hover(source, (1, 1)).expect("hover should produce fallback");
    assert_eq!(h.markdown, "`foobar`");
}

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

#[test]
fn format_document_has_no_edit_when_already_formatted() {
    let source = "@startuml\nAlice -> Bob\n@enduml\n";
    let result = format_document(source);
    assert!(!result.changed);
    assert!(result.edits.is_empty());
    assert_eq!(result.formatted, source);
}
