mod lsp_adapter;

use lsp_adapter::commands::{direct_command_result, execute_command};
use lsp_adapter::completion::{completions, hover, resolve_completion};
use lsp_adapter::diagnostics::pub_diag;
use lsp_adapter::document::{change, get_config_section, open};
use lsp_adapter::document_features::{
    code_actions, color_presentation, document_colors, document_links, folding_ranges,
    formatting_edits, selection_ranges,
};
use lsp_adapter::navigation::{
    definition_lsp, document_symbols_lsp, references_lsp, rename_lsp, workspace_symbols,
};
use lsp_adapter::protocol::{caps, err, notif, range, read_msg, read_pos, resp};
use lsp_adapter::render::{lsp_frontend_hint, render_result};
use lsp_adapter::semantic::semantic_tokens;
use lsp_adapter::Doc;
use puml::language_service::prepare_rename;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io;

fn main() {
    let mut r = io::BufReader::new(io::stdin().lock());
    let mut w = io::BufWriter::new(io::stdout().lock());
    let mut docs: HashMap<String, Doc> = HashMap::new();
    let mut workspace_config: Value = json!({});
    while let Ok(Some(msg)) = read_msg(&mut r) {
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        match method {
            "initialize" => {
                let _ = resp(
                    &mut w,
                    msg.get("id").cloned().unwrap_or(Value::Null),
                    json!({"capabilities":caps()}),
                );
            }
            "initialized" | "$/cancelRequest" | "$/setTrace" => {}
            "workspace/didChangeConfiguration" => {
                workspace_config = msg
                    .pointer("/params/settings")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
            }
            "workspace/didChangeWorkspaceFolders" | "workspace/didChangeWatchedFiles" => {}
            "textDocument/didSave" => {
                if let Some(uri) = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                {
                    if let Some(doc) = docs.get(uri) {
                        let _ = pub_diag(&mut w, uri, doc.version, &doc.text);
                    }
                }
            }
            "window/workDoneProgress/create" => {
                let _ = resp(
                    &mut w,
                    msg.get("id").cloned().unwrap_or(Value::Null),
                    Value::Null,
                );
            }
            "workspace/configuration" => {
                let items = msg
                    .pointer("/params/items")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    let section = item
                        .get("section")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .trim();
                    if section.is_empty() {
                        out.push(workspace_config.clone());
                    } else {
                        out.push(get_config_section(&workspace_config, section));
                    }
                }
                let _ = resp(
                    &mut w,
                    msg.get("id").cloned().unwrap_or(Value::Null),
                    Value::Array(out),
                );
            }
            "shutdown" => {
                let _ = resp(
                    &mut w,
                    msg.get("id").cloned().unwrap_or(Value::Null),
                    Value::Null,
                );
            }
            "exit" => break,
            "textDocument/didOpen" => {
                if let Some((u, v, t)) = open(&msg) {
                    docs.insert(u.clone(), Doc::new(t.clone(), v));
                    let _ = pub_diag(&mut w, &u, v, &t);
                }
            }
            "textDocument/didChange" => {
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let current = docs.get(uri).map(|d| d.text.as_str());
                if let Some((u, v, t)) = change(&msg, current) {
                    if let Some(prev) = docs.get(&u) {
                        if v < prev.version {
                            continue;
                        }
                    }
                    docs.insert(u.clone(), Doc::new(t.clone(), v));
                    let _ = pub_diag(&mut w, &u, v, &t);
                }
            }
            "textDocument/didClose" => {
                if let Some(u) = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                {
                    docs.remove(u);
                    let _ = notif(
                        &mut w,
                        "textDocument/publishDiagnostics",
                        json!({"uri":u,"diagnostics":[]}),
                    );
                }
            }
            "textDocument/completion" => {
                let _ = resp(
                    &mut w,
                    msg.get("id").cloned().unwrap_or(Value::Null),
                    json!({"isIncomplete":false,"items":completions()}),
                );
            }
            "completionItem/resolve" => {
                let _ = resp(
                    &mut w,
                    msg.get("id").cloned().unwrap_or(Value::Null),
                    resolve_completion(msg.pointer("/params").unwrap_or(&Value::Null)),
                );
            }
            "textDocument/hover" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let pos = read_pos(&msg).unwrap_or((0, 0));
                let out = docs.get(uri).and_then(|d| hover(d, pos));
                let _ = resp(&mut w, id, out.unwrap_or(Value::Null));
            }
            "textDocument/definition" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let pos = read_pos(&msg).unwrap_or((0, 0));
                let out = docs
                    .get(uri)
                    .and_then(|d| definition_lsp(d, uri, pos))
                    .unwrap_or(Value::Array(vec![]));
                let _ = resp(&mut w, id, out);
            }
            "textDocument/references" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let pos = read_pos(&msg).unwrap_or((0, 0));
                let out = docs
                    .get(uri)
                    .map(|d| references_lsp(d, uri, pos))
                    .unwrap_or(Value::Array(vec![]));
                let _ = resp(&mut w, id, out);
            }
            "textDocument/prepareRename" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let pos = read_pos(&msg).unwrap_or((0, 0));
                let out = docs
                    .get(uri)
                    .and_then(|d| {
                        prepare_rename(&d.text, pos)
                            .map(|span| range(&d.text, span.start, span.end))
                    })
                    .unwrap_or(Value::Null);
                let _ = resp(&mut w, id, out);
            }
            "textDocument/rename" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let pos = read_pos(&msg).unwrap_or((0, 0));
                let new_name = msg
                    .pointer("/params/newName")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let out = docs
                    .get(uri)
                    .map(|d| rename_lsp(d, uri, pos, new_name))
                    .unwrap_or(json!({"changes":{}}));
                let _ = resp(&mut w, id, out);
            }
            "textDocument/documentSymbol" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let out = docs
                    .get(uri)
                    .map(document_symbols_lsp)
                    .unwrap_or(Value::Array(vec![]));
                let _ = resp(&mut w, id, out);
            }
            "workspace/symbol" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let query = msg
                    .pointer("/params/query")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_ascii_lowercase();
                let out = workspace_symbols(&docs, &query);
                let _ = resp(&mut w, id, out);
            }
            "textDocument/formatting"
            | "textDocument/rangeFormatting"
            | "textDocument/onTypeFormatting" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .or_else(|| msg.pointer("/params/textDocument"))
                    .and_then(|v| v.get("uri"))
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let out = docs
                    .get(uri)
                    .map(|d| formatting_edits(&d.text))
                    .unwrap_or(Value::Array(vec![]));
                let _ = resp(&mut w, id, out);
            }
            "textDocument/foldingRange" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let out = docs
                    .get(uri)
                    .map(folding_ranges)
                    .unwrap_or(Value::Array(vec![]));
                let _ = resp(&mut w, id, out);
            }
            "textDocument/selectionRange" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let out = docs
                    .get(uri)
                    .map(|d| selection_ranges(d, &msg))
                    .unwrap_or(Value::Array(vec![]));
                let _ = resp(&mut w, id, out);
            }
            "textDocument/documentLink" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let out = docs
                    .get(uri)
                    .map(document_links)
                    .unwrap_or(Value::Array(vec![]));
                let _ = resp(&mut w, id, out);
            }
            "textDocument/documentColor" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let out = docs
                    .get(uri)
                    .map(document_colors)
                    .unwrap_or(Value::Array(vec![]));
                let _ = resp(&mut w, id, out);
            }
            "textDocument/colorPresentation" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let out = color_presentation(&msg);
                let _ = resp(&mut w, id, out);
            }
            "textDocument/semanticTokens/full" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let out = docs
                    .get(uri)
                    .map(semantic_tokens)
                    .unwrap_or(json!({"data":[]}));
                let _ = resp(&mut w, id, out);
            }
            "textDocument/codeAction" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let out = docs
                    .get(uri)
                    .map(|d| code_actions(uri, d, &msg))
                    .unwrap_or(Value::Array(vec![]));
                let _ = resp(&mut w, id, out);
            }
            "workspace/executeCommand" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let out = execute_command(&msg, &docs);
                let _ = resp(&mut w, id, out);
            }
            "puml/renderSvg" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let frontend = msg.pointer("/params").and_then(lsp_frontend_hint);
                let result = docs
                    .get(uri)
                    .map(|d| render_result(&d.text, frontend))
                    .unwrap_or_else(|| json!({"svg":"","width":0,"height":0,"diagnostics":[]}));
                let _ = resp(&mut w, id, result);
            }
            "puml/renderScene" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let result = direct_command_result("puml.renderScene", &msg, &docs);
                let _ = resp(&mut w, id, result);
            }
            "puml/export" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let result = direct_command_result("puml.export", &msg, &docs);
                let _ = resp(&mut w, id, result);
            }
            "puml/explainDiagnostic" => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let result = direct_command_result("puml.explainDiagnostic", &msg, &docs);
                let _ = resp(&mut w, id, result);
            }
            _ => {
                if msg.get("id").is_some() {
                    let _ = err(
                        &mut w,
                        msg.get("id").cloned().unwrap_or(Value::Null),
                        -32601,
                        "method not found",
                    );
                }
            }
        }
    }
}
