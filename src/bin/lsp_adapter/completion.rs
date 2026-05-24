use super::Doc;
use puml::language_service::{
    completion_items, hover as language_hover, resolve_completion_item, CompletionItemKind,
};
use serde_json::{json, Value};

pub fn completions() -> Vec<Value> {
    let list = completion_items();
    list.items
        .iter()
        .map(|item| {
            json!({
                "label": item.label,
                "kind": lsp_completion_item_kind(item.kind)
            })
        })
        .collect()
}

pub fn resolve_completion(item: &Value) -> Value {
    let mut resolved = item.clone();
    let Some(label) = item.get("label").and_then(Value::as_str) else {
        return resolved;
    };
    let Some(spec) = resolve_completion_item(label) else {
        return resolved;
    };
    if let Some(obj) = resolved.as_object_mut() {
        obj.insert("detail".to_string(), Value::String(spec.detail.to_string()));
        obj.insert(
            "documentation".to_string(),
            json!({"kind":"markdown","value":spec.documentation}),
        );
    }
    resolved
}

pub fn hover(d: &Doc, posn: (u64, u64)) -> Option<Value> {
    language_hover(&d.text, posn).map(|hover| {
        json!({
            "contents": {
                "kind": "markdown",
                "value": hover.markdown
            }
        })
    })
}

fn lsp_completion_item_kind(kind: CompletionItemKind) -> u32 {
    match kind {
        CompletionItemKind::Keyword => 14,
        CompletionItemKind::Snippet => 15,
        CompletionItemKind::Operator => 24,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp_adapter::render::lsp_parse;
    use serde_json::json;

    #[test]
    fn completion_baseline_includes_top_level_and_arrow_items() {
        let items = completions();
        let labels: Vec<String> = items
            .iter()
            .filter_map(|item| item.get("label").and_then(Value::as_str))
            .map(ToOwned::to_owned)
            .collect();
        assert!(labels.contains(&"@startuml".to_string()));
        assert!(labels.contains(&"participant".to_string()));
        assert!(labels.contains(&"class".to_string()));
        assert!(labels.contains(&"state".to_string()));
        assert!(labels.contains(&"start".to_string()));
        assert!(labels.contains(&"autonumber stop".to_string()));
        assert!(labels.contains(&"|||".to_string()));
        assert!(labels.contains(&"-->>".to_string()));
    }

    #[test]
    fn completion_resolve_adds_detail_and_documentation() {
        let resolved = resolve_completion(&json!({"label":"participant","kind":14}));
        assert_eq!(resolved["detail"], "Participant");
        assert_eq!(
            resolved["documentation"]["kind"],
            Value::String("markdown".to_string())
        );
        assert!(resolved["documentation"]["value"]
            .as_str()
            .expect("markdown value")
            .contains("Declare a participant"));
    }

    #[test]
    fn hover_returns_arrow_documentation_for_symbol_positions() {
        let src = "@startuml\nA --> B: hi\n@enduml\n";
        let doc = Doc {
            text: src.to_string(),
            version: 1,
            parsed: lsp_parse(src).ok(),
        };
        let out = hover(&doc, (1, 3)).expect("hover");
        assert!(out["contents"]["value"]
            .as_str()
            .expect("hover markdown")
            .contains("Dashed message arrow"));
    }
}
