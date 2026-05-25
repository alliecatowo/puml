mod completion;
mod completion_extra;
#[cfg(test)]
mod completion_tests;
mod diagnostics;
mod document;
mod formatting;
mod hover;
mod navigation;
mod surface;
mod symbols;
mod syntax;
mod text;
mod tokens;

pub use completion::{
    completion_items, resolve_completion_item, CompletionItem, CompletionItemKind, CompletionList,
};
pub use diagnostics::{
    diagnostics, diagnostics_with_options, explain_diagnostic, DiagnosticExplanation,
    DiagnosticsReport, LanguageDiagnostic, SourcePosition, SourceRange,
};
pub use document::{DocumentSnapshot, SnapshotOptions};
pub use formatting::{format_document, FormatDocumentResult, TextEdit};
pub use hover::{hover, Hover};
pub use navigation::{definition, prepare_rename, references, rename, NavigationHit};
pub use surface::language_service_surface_json;
pub use symbols::{document_symbols, DocumentSymbol, DocumentSymbolKind};
pub use syntax::{syntax_token_specs, SyntaxTokenKind, SyntaxTokenSpec};
pub use text::{lc_to_offset, offset_to_lc, word_range_at_pos};
pub use tokens::{
    semantic_token_legend, semantic_tokens, SemanticToken, SemanticTokenKind, SEMANTIC_TOKEN_TYPES,
};
