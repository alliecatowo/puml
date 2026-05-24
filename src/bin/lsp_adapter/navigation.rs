use super::protocol::range;
use super::Doc;
use puml::ast::StatementKind;
use puml::language_service::{
    definition as language_definition, document_symbols, references as language_references,
    rename as language_rename, DocumentSymbolKind,
};
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn definition_lsp(d: &Doc, uri: &str, posn: (u64, u64)) -> Option<Value> {
    language_definition(d, posn)
        .map(|hit| json!([{"uri":uri,"range":range(&d.text, hit.span.start, hit.span.end)}]))
}

pub fn references_lsp(d: &Doc, uri: &str, posn: (u64, u64)) -> Value {
    Value::Array(
        language_references(&d.text, posn)
            .into_iter()
            .map(|hit| json!({"uri":uri,"range":range(&d.text,hit.span.start,hit.span.end)}))
            .collect(),
    )
}

pub fn rename_lsp(d: &Doc, uri: &str, posn: (u64, u64), new_name: &str) -> Value {
    let edits = language_rename(&d.text, posn, new_name)
        .into_iter()
        .map(
            |edit| json!({"range":range(&d.text,edit.span.start,edit.span.end),"newText":edit.new_text}),
        )
        .collect::<Vec<_>>();
    json!({"changes":{uri:edits}})
}

pub fn document_symbols_lsp(d: &Doc) -> Value {
    let symbols = d
        .parsed
        .as_ref()
        .map(document_symbols)
        .unwrap_or_default()
        .into_iter()
        .map(|symbol| {
            json!({
                "name": symbol.name,
                "kind": lsp_symbol_kind(symbol.kind),
                "range": range(&d.text, symbol.span.start, symbol.span.end),
                "selectionRange": range(&d.text, symbol.selection_span.start, symbol.selection_span.end)
            })
        })
        .collect();
    Value::Array(symbols)
}

pub fn workspace_symbols(docs: &HashMap<String, Doc>, q: &str) -> Value {
    let mut out = Vec::new();
    for (uri, d) in docs {
        if let Some(doc) = &d.parsed {
            for st in &doc.statements {
                if let StatementKind::Participant(p) = &st.kind {
                    if p.name.to_ascii_lowercase().contains(q) {
                        out.push(json!({"name":p.name,"kind":5,"location":{"uri":uri,"range":range(&d.text,st.span.start,st.span.end)}}));
                    }
                }
            }
        }
    }
    Value::Array(out)
}

fn lsp_symbol_kind(kind: DocumentSymbolKind) -> i32 {
    match kind {
        DocumentSymbolKind::Participant => 5,
        DocumentSymbolKind::Message => 12,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp_adapter::render::lsp_parse;

    #[test]
    fn definition_returns_location_shape() {
        let src = "@startuml\nparticipant Alice\nAlice -> Bob: hi\n@enduml\n";
        let doc = Doc {
            text: src.to_string(),
            version: 1,
            parsed: lsp_parse(src).ok(),
        };
        let out = definition_lsp(&doc, "file:///test.puml", (2, 1)).expect("definition");
        let first = out
            .as_array()
            .and_then(|arr| arr.first())
            .cloned()
            .expect("array item");
        assert!(first.get("uri").is_some());
        assert!(first.get("range").is_some());
        assert!(first.get("selectionRange").is_none());
    }

    #[test]
    fn workspace_symbols_cover_query_helpers() {
        let src =
            "@startuml\nparticipant Alice\nparticipant ApiGateway\nAlice -> ApiGateway\n@enduml\n";
        let mut docs = HashMap::new();
        docs.insert(
            "file:///flow.puml".to_string(),
            Doc {
                text: src.to_string(),
                version: 1,
                parsed: lsp_parse(src).ok(),
            },
        );

        let out = workspace_symbols(&docs, "api");
        let arr = out.as_array().expect("symbols");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "ApiGateway");
        assert_eq!(arr[0]["location"]["uri"], "file:///flow.puml");
    }
}
