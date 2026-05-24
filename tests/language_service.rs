use puml::diagnostic::Severity;
use puml::language_service::{
    completion_items, definition, diagnostics, diagnostics_with_options, document_symbols, hover,
    references, rename, resolve_completion_item, CompletionItemKind, DocumentSnapshot,
    DocumentSymbolKind,
};
use puml::source::Span;
use puml::{CompatMode, DeterminismMode, FrontendSelection, ParsePipelineOptions};

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

#[test]
fn navigation_is_available_without_lsp_transport() {
    let source = "@startuml\nparticipant Alice\nAlice -> Bob: hi\n@enduml\n";
    let snapshot = DocumentSnapshot::new(source.to_string(), 1);

    let declaration = definition(&snapshot, (2, 1)).expect("definition hit");
    assert_eq!(
        &source[declaration.span.start..declaration.span.end],
        "participant Alice"
    );

    let refs = references(source, (2, 1));
    assert_eq!(refs.len(), 2);

    let edits = rename(source, (2, 1), "User");
    assert_eq!(edits.len(), 2);
    assert!(edits.iter().all(|edit| edit.new_text == "User"));
}

#[test]
fn diagnostics_reports_parse_errors_without_lsp_transport() {
    let source = "@startuml\nA ->\n@enduml\n";

    let report = diagnostics(source);

    assert_eq!(report.diagnostics.len(), 1);
    let diagnostic = &report.diagnostics[0];
    assert_eq!(diagnostic.code.as_deref(), Some("E_ARROW_INVALID"));
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.span, Some(Span::new(10, 14)));
    assert_eq!(diagnostic.range.unwrap().start.line, 2);
    assert_eq!(diagnostic.range.unwrap().start.column, 1);
}

#[test]
fn diagnostics_range_uses_shared_unicode_line_column_mapping() {
    let source = "@startuml\nµ ->\n@enduml\n";

    let report = diagnostics(source);

    assert_eq!(report.diagnostics.len(), 1);
    let range = report.diagnostics[0].range.expect("range");
    assert_eq!(range.start.line, 2);
    assert_eq!(range.start.column, 1);
    assert_eq!(range.end.line, 2);
    assert_eq!(range.end.column, 5);
}

#[test]
fn diagnostics_reports_normalization_warnings_without_lsp_transport() {
    let source = "@startuml\nskinparam TotallyUnknownColor red\nA -> B\n@enduml\n";

    let report = diagnostics(source);

    assert_eq!(report.diagnostics.len(), 1);
    let diagnostic = &report.diagnostics[0];
    assert_eq!(diagnostic.code.as_deref(), Some("W_SKINPARAM_UNSUPPORTED"));
    assert_eq!(diagnostic.severity, Severity::Warning);
    assert_eq!(diagnostic.span, Some(Span::new(10, 43)));
    assert_eq!(diagnostic.range.unwrap().start.line, 2);
    assert_eq!(diagnostic.range.unwrap().start.column, 1);
}

#[test]
fn diagnostics_reports_typed_state_unsupported_code_without_lsp_transport() {
    let source = "@startuml\nstate Idle\nunknown state syntax\n@enduml\n";

    let report = diagnostics(source);

    assert_eq!(report.diagnostics.len(), 1);
    let diagnostic = &report.diagnostics[0];
    assert_eq!(
        diagnostic.code.as_deref(),
        Some("E_STATE_UNSUPPORTED_SYNTAX")
    );
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(diagnostic.span, Some(Span::new(21, 41)));
    assert_eq!(diagnostic.range.unwrap().start.line, 3);
    assert_eq!(diagnostic.range.unwrap().start.column, 1);
}

#[test]
fn diagnostics_reports_frontend_adapter_warnings_without_lsp_transport() {
    let source = "flowchart LR\nclassDef hot fill:#fef3c7,stroke:#92400e\nA[API]:::hot --> B\n";
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Mermaid,
        compat: CompatMode::Strict,
        determinism: DeterminismMode::Strict,
        include_root: None,
        ..ParsePipelineOptions::default()
    };

    let report = diagnostics_with_options(source, &options);

    assert_eq!(report.diagnostics.len(), 1);
    let diagnostic = &report.diagnostics[0];
    assert_eq!(diagnostic.code.as_deref(), Some("W_MERMAID_STYLE_PARTIAL"));
    assert_eq!(diagnostic.severity, Severity::Warning);
    assert_eq!(diagnostic.span, Some(Span::new(13, 53)));
    assert_eq!(diagnostic.range.unwrap().start.line, 2);
    assert_eq!(diagnostic.range.unwrap().start.column, 1);
}
