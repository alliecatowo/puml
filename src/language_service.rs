mod completion;
mod diagnostics;
mod formatting;
mod hover;
mod semantic;
mod symbols;
mod util;

pub use completion::{
    completion_items, resolve_completion_item, CompletionItem, CompletionItemKind, CompletionList,
};
pub use diagnostics::{
    diagnostics, diagnostics_with_options, DiagnosticsReport, LanguageDiagnostic, SourcePosition,
    SourceRange,
};
pub use formatting::{format_document, FormatDocumentResult, TextEdit};
pub use hover::{hover, Hover};
pub use semantic::{semantic_tokens, SemanticToken, SemanticTokenKind};
pub use symbols::{document_symbols, DocumentSymbol, DocumentSymbolKind};

#[cfg(test)]
mod tests;
