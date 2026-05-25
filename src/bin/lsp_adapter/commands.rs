use super::document_features::formatting_edits;
use super::render::{
    export_result, lsp_frontend_hint, output_format_from_hint, render_result, render_scene_result,
};
use super::Doc;
use puml::language_service::{explain_diagnostic, language_service_surface_json};
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn execute_command(msg: &Value, docs: &HashMap<String, Doc>) -> Value {
    let cmd = msg
        .pointer("/params/command")
        .and_then(Value::as_str)
        .unwrap_or("");
    let args = command_args(msg);
    let uri = command_uri(args).unwrap_or("");
    let options = command_options(args);
    let frontend = options.and_then(lsp_frontend_hint);
    match cmd {
        "puml.renderSvg" => docs
            .get(uri)
            .map(|d| render_result(&d.text, frontend))
            .unwrap_or_else(|| {
                json!({"svg":"","width":0,"height":0,"diagnostics":[{"message":"document not open"}]})
            }),
        "puml.renderScene" => docs
            .get(uri)
            .map(|d| render_scene_result(&d.text, frontend))
            .unwrap_or_else(|| missing_document_result("puml.renderScene")),
        "puml.export" => {
            let format = options
                .and_then(|value| value.get("format").or_else(|| value.get("target")))
                .and_then(Value::as_str)
                .and_then(output_format_from_hint)
                .unwrap_or(puml::output::OutputFormat::Svg);
            docs.get(uri)
                .map(|d| export_result(&d.text, frontend, format))
                .unwrap_or_else(|| missing_document_result("puml.export"))
        }
        "puml.explainDiagnostic" => explain_diagnostic_result(args),
        "puml.languageService" => language_service_surface_json(),
        "puml.applyFormat" => docs
            .get(uri)
            .map(|d| formatting_edits(&d.text))
            .unwrap_or(Value::Array(vec![])),
        _ => json!({"error":format!("unknown command: {cmd}")}),
    }
}

pub fn direct_command_result(command: &str, msg: &Value, docs: &HashMap<String, Doc>) -> Value {
    let mut params = json!({
        "params": {
            "command": command,
            "arguments": [msg.pointer("/params").cloned().unwrap_or(Value::Null)]
        }
    });
    if let Some(uri) = msg
        .pointer("/params/textDocument/uri")
        .and_then(Value::as_str)
        .or_else(|| msg.pointer("/params/uri").and_then(Value::as_str))
    {
        params["params"]["arguments"] =
            json!([uri, msg.pointer("/params").cloned().unwrap_or(Value::Null)]);
    }
    execute_command(&params, docs)
}

fn command_args(msg: &Value) -> &[Value] {
    msg.pointer("/params/arguments")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn command_uri(args: &[Value]) -> Option<&str> {
    args.first().and_then(|arg| {
        arg.as_str()
            .or_else(|| arg.pointer("/textDocument/uri").and_then(Value::as_str))
            .or_else(|| arg.get("uri").and_then(Value::as_str))
    })
}

fn command_options(args: &[Value]) -> Option<&Value> {
    args.iter()
        .find(|arg| {
            arg.get("format")
                .or_else(|| arg.get("target"))
                .or_else(|| arg.get("frontend"))
                .or_else(|| arg.get("dialect"))
                .or_else(|| arg.get("language"))
                .is_some()
        })
        .or_else(|| args.iter().find(|arg| arg.is_object()))
}

fn missing_document_result(schema: &str) -> Value {
    json!({
        "schema": schema,
        "schemaVersion": 1,
        "diagnostics": [{
            "code": "E_DOCUMENT_NOT_OPEN",
            "severity": "error",
            "message": "document not open"
        }]
    })
}

fn explain_diagnostic_result(args: &[Value]) -> Value {
    let diagnostic = args
        .iter()
        .find(|arg| arg.get("code").is_some() || arg.get("message").is_some())
        .or_else(|| args.first())
        .unwrap_or(&Value::Null);
    let code = diagnostic.get("code").and_then(|value| {
        value
            .as_str()
            .or_else(|| value.get("value").and_then(Value::as_str))
    });
    let message = diagnostic.get("message").and_then(Value::as_str);
    let explanation = explain_diagnostic(code, message);
    json!({
        "schema": "puml.explainDiagnostic",
        "schemaVersion": 1,
        "diagnostic": {
            "code": explanation.code,
            "message": message,
            "range": diagnostic.get("range").cloned().unwrap_or(Value::Null),
            "category": diagnostic
                .pointer("/data/category")
                .or_else(|| diagnostic.get("category"))
                .cloned()
                .unwrap_or(Value::Null)
        },
        "explanation": {
            "summary": explanation.summary,
            "action": explanation.action
        },
        "diagnostics": []
    })
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
