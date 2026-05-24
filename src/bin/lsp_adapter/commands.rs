use super::document_features::formatting_edits;
use super::render::{lsp_frontend_hint, render_result};
use super::Doc;
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn execute_command(msg: &Value, docs: &HashMap<String, Doc>) -> Value {
    let cmd = msg
        .pointer("/params/command")
        .and_then(Value::as_str)
        .unwrap_or("");
    let uri = msg
        .pointer("/params/arguments/0")
        .and_then(Value::as_str)
        .unwrap_or("");
    let frontend = msg
        .pointer("/params/arguments/1")
        .and_then(lsp_frontend_hint);
    match cmd {
        "puml.renderSvg" => docs
            .get(uri)
            .map(|d| render_result(&d.text, frontend))
            .unwrap_or_else(|| {
                json!({"svg":"","width":0,"height":0,"diagnostics":[{"message":"document not open"}]})
            }),
        "puml.applyFormat" => docs
            .get(uri)
            .map(|d| formatting_edits(&d.text))
            .unwrap_or(Value::Array(vec![])),
        _ => json!({"error":format!("unknown command: {cmd}")}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp_adapter::render::lsp_parse;
    use serde_json::json;

    #[test]
    fn execute_command_covers_success_error_and_missing_doc_paths() {
        let valid_src = "@startuml\nAlice -> Bob: hi\n@enduml\n";
        let valid_doc = Doc {
            text: valid_src.to_string(),
            version: 1,
            parsed: lsp_parse(valid_src).ok(),
        };

        let mut docs = HashMap::new();
        docs.insert("file:///valid.puml".to_string(), valid_doc);

        let rendered = execute_command(
            &json!({"params": {"command": "puml.renderSvg", "arguments": ["file:///valid.puml"]}}),
            &docs,
        );
        assert!(rendered["svg"].as_str().expect("svg").contains("<svg"));

        let missing = execute_command(
            &json!({"params": {"command": "puml.renderSvg", "arguments": ["file:///missing.puml"]}}),
            &docs,
        );
        assert_eq!(missing["diagnostics"][0]["message"], "document not open");

        let unknown = execute_command(
            &json!({"params": {"command": "puml.nope", "arguments": ["file:///valid.puml"]}}),
            &docs,
        );
        assert_eq!(unknown["error"], "unknown command: puml.nope");
    }
}
