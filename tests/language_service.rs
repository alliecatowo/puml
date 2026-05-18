use puml::language_service::{
    completion_items, document_symbols, hover, resolve_completion_item, CompletionItemKind,
    DocumentSymbolKind,
};
use puml::source::Span;

#[test]
fn document_symbols_are_available_without_lsp_transport() {
    let source = "@startuml\nparticipant Alice\nBob -> Alice: hello\n@enduml\n";
    let document = puml::parse(source).expect("source parses");

    let symbols = document_symbols(&document);

    assert_eq!(symbols.len(), 2);
    assert_eq!(symbols[0].name, "Alice");
    assert_eq!(symbols[0].kind, DocumentSymbolKind::Participant);
    assert_eq!(symbols[0].span, Span::new(10, 27));
    assert_eq!(symbols[1].name, "Bob -> Alice");
    assert_eq!(symbols[1].kind, DocumentSymbolKind::Message);
    assert_eq!(symbols[1].span, Span::new(28, 47));
}

#[test]
fn document_symbols_skip_non_symbol_statements() {
    let source = "@startuml\ntitle Checkout\nAlice -> Bob: pay\n@enduml\n";
    let document = puml::parse(source).expect("source parses");

    let symbols = document_symbols(&document);

    assert_eq!(symbols.len(), 1);
    assert_eq!(symbols[0].name, "Alice -> Bob");
    assert_eq!(symbols[0].kind, DocumentSymbolKind::Message);
}

#[test]
fn completion_items_are_available_without_lsp_transport() {
    let completions = completion_items();
    let labels: Vec<&str> = completions.items.iter().map(|item| item.label).collect();

    assert!(!completions.is_incomplete);
    assert!(labels.contains(&"@startuml"));
    assert!(labels.contains(&"participant"));
    assert!(labels.contains(&"class"));
    assert!(labels.contains(&"state"));
    assert!(labels.contains(&"start"));
    assert!(labels.contains(&"autonumber stop"));
    assert!(labels.contains(&"|||"));
    assert!(labels.contains(&"-->>"));
}

#[test]
fn completion_resolve_returns_detail_and_documentation_without_lsp_transport() {
    let item = resolve_completion_item("participant").expect("completion item");

    assert_eq!(item.kind, CompletionItemKind::Keyword);
    assert_eq!(item.detail, "Participant");
    assert_eq!(item.documentation, "Declare a participant.");
}

#[test]
fn hover_returns_documentation_for_arrow_symbols_without_lsp_transport() {
    let source = "@startuml\nA --> B: hi\n@enduml\n";
    let markdown = hover(source, (1, 3)).expect("hover").markdown;

    assert!(markdown.contains("`-->`"));
    assert!(markdown.contains("Dashed message arrow"));
}

#[test]
fn hover_returns_plain_word_for_unknown_identifiers_without_lsp_transport() {
    let source = "@startuml\nAlice -> Bob: hi\n@enduml\n";
    let markdown = hover(source, (1, 1)).expect("hover").markdown;

    assert_eq!(markdown, "`Alice`");
}
