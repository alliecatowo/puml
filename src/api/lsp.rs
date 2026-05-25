use crate::language_service;

/// Returns the LSP server capabilities object that `puml-lsp` advertises
/// during the `initialize` handshake. Exposing this here lets both the
/// `puml-lsp` binary and the `puml --dump-capabilities` CLI flag share a
/// single source of truth.
pub fn lsp_capabilities() -> serde_json::Value {
    serde_json::json!({
        "textDocumentSync":{"openClose":true,"change":2,"save":{"includeText":true}},
        "completionProvider":{"resolveProvider":true},
        "hoverProvider":true,
        "definitionProvider":true,
        "referencesProvider":true,
        "renameProvider":{"prepareProvider":true},
        "documentSymbolProvider":true,
        "workspaceSymbolProvider":true,
        "semanticTokensProvider":{"legend":{"tokenTypes":language_service::semantic_token_legend(),"tokenModifiers":[]},"full":true},
        "documentFormattingProvider":true,
        "documentRangeFormattingProvider":true,
        "foldingRangeProvider":true,
        "selectionRangeProvider":true,
        "documentLinkProvider":{},
        "colorProvider":true,
        "codeActionProvider":true,
        "executeCommandProvider":{"commands":["puml.applyFormat","puml.renderSvg","puml.renderScene","puml.export","puml.explainDiagnostic","puml.languageService"]},
        "workspace":{"workspaceFolders":{"supported":true,"changeNotifications":true}}
    })
}
