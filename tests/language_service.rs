use puml::language_service::{document_symbols, DocumentSymbolKind};
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
