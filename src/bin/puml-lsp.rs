use puml::ast::StatementKind;
use puml::diagnostic::Severity;
use puml::language_service::{
    completion_items, diagnostics_with_options, document_symbols, format_document,
    hover as language_hover, resolve_completion_item, semantic_tokens as shared_semantic_tokens,
    CompletionItemKind, DocumentSymbolKind, SemanticTokenKind,
};
use puml::{
    normalize_family, parse_with_pipeline_options, render_svg_pages_from_model, Document,
    FrontendSelection, ParsePipelineOptions,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

#[derive(Clone, Debug)]
struct Doc {
    text: String,
    parsed: Option<Document>,
    version: i64,
}

#[derive(Clone, Debug)]
struct RefHit {
    start: usize,
    end: usize,
}

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
                    let parsed = lsp_parse(&t).ok();
                    docs.insert(
                        u.clone(),
                        Doc {
                            text: t.clone(),
                            parsed,
                            version: v,
                        },
                    );
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
                    let parsed = lsp_parse(&t).ok();
                    docs.insert(
                        u.clone(),
                        Doc {
                            text: t.clone(),
                            parsed,
                            version: v,
                        },
                    );
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
                    .and_then(|d| definition(d, uri, pos))
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
                    .map(|d| references(d, uri, pos))
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
                        word_range_at_pos(&d.text, pos).map(|(s, e)| range(&d.text, s, e))
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
                    .map(|d| rename(d, uri, pos, new_name))
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

fn caps() -> Value {
    puml::lsp_capabilities()
}
fn completions() -> Vec<Value> {
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

fn resolve_completion(item: &Value) -> Value {
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

fn hover(d: &Doc, posn: (u64, u64)) -> Option<Value> {
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

fn definition(d: &Doc, uri: &str, posn: (u64, u64)) -> Option<Value> {
    let (s, e) = word_range_at_pos(&d.text, posn)?;
    let sym = &d.text[s..e];
    let decl = find_participant_decl(d.parsed.as_ref()?, sym)?;
    Some(json!([{"uri":uri,"range":range(&d.text,decl.0,decl.1)}]))
}
fn references(d: &Doc, uri: &str, posn: (u64, u64)) -> Value {
    let mut out = Vec::new();
    if let Some((s, e)) = word_range_at_pos(&d.text, posn) {
        let sym = &d.text[s..e];
        for hit in find_word_refs(&d.text, sym) {
            out.push(json!({"uri":uri,"range":range(&d.text,hit.start,hit.end)}));
        }
    }
    Value::Array(out)
}
fn rename(d: &Doc, uri: &str, posn: (u64, u64), new_name: &str) -> Value {
    let mut edits = Vec::new();
    if let Some((s, e)) = word_range_at_pos(&d.text, posn) {
        let sym = &d.text[s..e];
        for hit in find_word_refs(&d.text, sym) {
            edits.push(json!({"range":range(&d.text,hit.start,hit.end),"newText":new_name}));
        }
    }
    json!({"changes":{uri:edits}})
}

fn document_symbols_lsp(d: &Doc) -> Value {
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
fn workspace_symbols(docs: &HashMap<String, Doc>, q: &str) -> Value {
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

fn formatting_edits(text: &str) -> Value {
    Value::Array(
        format_document(text)
            .edits
            .into_iter()
            .map(|edit| {
                json!({
                    "range": range(text, edit.span.start, edit.span.end),
                    "newText": edit.new_text
                })
            })
            .collect(),
    )
}
fn folding_ranges(d: &Doc) -> Value {
    let mut out = Vec::new();
    if let Some(doc) = &d.parsed {
        for st in &doc.statements {
            if matches!(st.kind, StatementKind::Group(_) | StatementKind::Note(_)) {
                let a = offset_to_lc(&d.text, st.span.start);
                let b = offset_to_lc(&d.text, st.span.end);
                if b.0 > a.0 {
                    out.push(json!({"startLine":a.0,"endLine":b.0}));
                }
            }
        }
    }
    Value::Array(out)
}
fn selection_ranges(d: &Doc, msg: &Value) -> Value {
    let arr = msg
        .pointer("/params/positions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    for p in arr {
        let l = p.get("line").and_then(Value::as_u64).unwrap_or(0);
        let c = p.get("character").and_then(Value::as_u64).unwrap_or(0);
        if let Some((s, e)) = word_range_at_pos(&d.text, (l, c)) {
            out.push(json!({"range":range(&d.text,s,e),"parent":{"range":{"start":{"line":0,"character":0},"end":pos(&d.text,d.text.len())}}}));
        } else {
            out.push(
                json!({"range":{"start":{"line":0,"character":0},"end":pos(&d.text,d.text.len())}}),
            );
        }
    }
    Value::Array(out)
}
fn document_links(d: &Doc) -> Value {
    let mut out = Vec::new();
    let mut line_offset = 0usize;
    for line in d.text.split_inclusive('\n') {
        let st = line.strip_suffix('\n').unwrap_or(line);
        if let Some(ix) = st.find("!include") {
            let include_end = ix + 8;
            let tail = &st[include_end..];
            let ws = tail.chars().take_while(|ch| ch.is_whitespace()).count();
            let path_start = include_end + ws;
            let path = st[path_start..].trim_end();
            if !path.is_empty() {
                let off = line_offset + path_start;
                out.push(json!({"range":range(&d.text,off,off+path.len()),"target":path}));
            }
        }
        line_offset += line.len();
    }
    Value::Array(out)
}
fn semantic_tokens(d: &Doc) -> Value {
    let mut data = Vec::<u32>::new();
    let mut prev_line = 0u32;
    let mut prev_char = 0u32;
    for token in shared_semantic_tokens(&d.text) {
        let (l, c) = offset_to_lc(&d.text, token.span.start);
        let dl = l as u32 - prev_line;
        let dc = if dl == 0 {
            c as u32 - prev_char
        } else {
            c as u32
        };
        data.extend([
            dl,
            dc,
            (token.span.end - token.span.start) as u32,
            lsp_semantic_token_type(token.kind),
            0,
        ]);
        prev_line = l as u32;
        prev_char = c as u32;
    }
    json!({"data":data})
}

fn lsp_semantic_token_type(kind: SemanticTokenKind) -> u32 {
    match kind {
        SemanticTokenKind::Keyword => 0,
        SemanticTokenKind::Operator => 1,
    }
}

fn document_colors(d: &Doc) -> Value {
    let mut out = Vec::new();
    for hit in find_hex_colors(&d.text) {
        if let Some((r, g, b, a)) = decode_hex_color(&d.text[hit.start..hit.end]) {
            out.push(json!({"range":range(&d.text,hit.start,hit.end),"color":{"red":r,"green":g,"blue":b,"alpha":a}}));
        }
    }
    Value::Array(out)
}

fn color_presentation(msg: &Value) -> Value {
    let r = msg
        .pointer("/params/color/red")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let g = msg
        .pointer("/params/color/green")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let b = msg
        .pointer("/params/color/blue")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let a = msg
        .pointer("/params/color/alpha")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    let label = if a < 1.0 {
        format!(
            "#{:02X}{:02X}{:02X}{:02X}",
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8,
            (a * 255.0) as u8
        )
    } else {
        format!(
            "#{:02X}{:02X}{:02X}",
            (r * 255.0) as u8,
            (g * 255.0) as u8,
            (b * 255.0) as u8
        )
    };
    json!([{"label":label}])
}

fn code_actions(uri: &str, d: &Doc, msg: &Value) -> Value {
    let mut out = vec![json!({
        "title":"Format document",
        "kind":"source.format",
        "command":{"title":"Format document","command":"puml.applyFormat","arguments":[uri]}
    })];
    if lsp_parse(&d.text).and_then(normalize_family).is_ok() {
        out.push(json!({
            "title":"Render SVG preview",
            "kind":"refactor.rewrite",
            "command":{"title":"Render SVG preview","command":"puml.renderSvg","arguments":[uri]}
        }));
    }
    let has_errors = msg
        .pointer("/params/context/diagnostics")
        .and_then(Value::as_array)
        .map(|diagnostics| {
            diagnostics
                .iter()
                .any(|d| d.get("severity").and_then(Value::as_i64).unwrap_or(0) == 1)
        })
        .unwrap_or(false);
    if has_errors {
        out.push(json!({
            "title":"Fix formatting and retry",
            "kind":"quickfix",
            "command":{"title":"Fix formatting and retry","command":"puml.applyFormat","arguments":[uri]}
        }));
    }
    Value::Array(out)
}

fn execute_command(msg: &Value, docs: &HashMap<String, Doc>) -> Value {
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
            .unwrap_or_else(|| json!({"svg":"","width":0,"height":0,"diagnostics":[{"message":"document not open"}]})),
        "puml.applyFormat" => docs
            .get(uri)
            .map(|d| formatting_edits(&d.text))
            .unwrap_or(Value::Array(vec![])),
        _ => json!({"error":format!("unknown command: {cmd}")}),
    }
}

fn find_hex_colors(src: &str) -> Vec<RefHit> {
    let mut out = Vec::new();
    let b = src.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] as char == '#' {
            let mut j = i + 1;
            while j < b.len() && (b[j] as char).is_ascii_hexdigit() {
                j += 1;
            }
            let len = j - (i + 1);
            if matches!(len, 3 | 4 | 6 | 8) {
                out.push(RefHit { start: i, end: j });
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

fn decode_hex_color(v: &str) -> Option<(f64, f64, f64, f64)> {
    let s = v.strip_prefix('#')?;
    let (r, g, b, a) = match s.len() {
        3 => {
            let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
            (r, g, b, 255)
        }
        4 => {
            let r = u8::from_str_radix(&s[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&s[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&s[2..3].repeat(2), 16).ok()?;
            let a = u8::from_str_radix(&s[3..4].repeat(2), 16).ok()?;
            (r, g, b, a)
        }
        6 => (
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
            255,
        ),
        8 => (
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
            u8::from_str_radix(&s[6..8], 16).ok()?,
        ),
        _ => return None,
    };
    Some((
        r as f64 / 255.0,
        g as f64 / 255.0,
        b as f64 / 255.0,
        a as f64 / 255.0,
    ))
}

fn find_participant_decl(doc: &Document, sym: &str) -> Option<(usize, usize)> {
    for st in &doc.statements {
        if let StatementKind::Participant(p) = &st.kind {
            if p.name == sym || p.alias.as_deref() == Some(sym) {
                return Some((st.span.start, st.span.end));
            }
        }
    }
    None
}
fn find_word_refs(src: &str, sym: &str) -> Vec<RefHit> {
    let mut v = Vec::new();
    if sym.is_empty() {
        return v;
    }
    let b = src.as_bytes();
    let sb = sym.as_bytes();
    let mut i = 0;
    while i + sb.len() <= b.len() {
        if &b[i..i + sb.len()] == sb {
            let left = i == 0 || !is_ident(b[i - 1] as char);
            let right = i + sb.len() == b.len() || !is_ident(b[i + sb.len()] as char);
            if left && right {
                v.push(RefHit {
                    start: i,
                    end: i + sb.len(),
                });
            }
            i += sb.len();
        } else {
            i += 1;
        }
    }
    v
}

fn is_ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}
fn word_range_at_pos(src: &str, posn: (u64, u64)) -> Option<(usize, usize)> {
    let off = lc_to_offset(src, posn.0 as usize, posn.1 as usize);
    if off >= src.len() {
        return None;
    }
    let b = src.as_bytes();
    if !is_ident(b[off] as char) {
        return None;
    }
    let mut s = off;
    while s > 0 && is_ident(b[s - 1] as char) {
        s -= 1;
    }
    let mut e = off;
    while e < b.len() && is_ident(b[e] as char) {
        e += 1;
    }
    Some((s, e))
}
fn lc_to_offset(src: &str, line: usize, ch: usize) -> usize {
    let mut l = 0usize;
    let mut c = 0usize;
    for (i, k) in src.char_indices() {
        if l == line && c == ch {
            return i;
        }
        if k == '\n' {
            l += 1;
            c = 0;
        } else {
            c += 1;
        }
    }
    src.len()
}
fn offset_to_lc(src: &str, off: usize) -> (usize, usize) {
    let mut l = 0usize;
    let mut c = 0usize;
    for (i, k) in src.char_indices() {
        if i >= off.min(src.len()) {
            break;
        }
        if k == '\n' {
            l += 1;
            c = 0;
        } else {
            c += 1;
        }
    }
    (l, c)
}
fn read_pos(msg: &Value) -> Option<(u64, u64)> {
    Some((
        msg.pointer("/params/position/line")?.as_u64()?,
        msg.pointer("/params/position/character")?.as_u64()?,
    ))
}
fn range(src: &str, s: usize, e: usize) -> Value {
    json!({"start":pos(src,s),"end":pos(src,e.max(s+1))})
}

fn render_result(src: &str, frontend: Option<FrontendSelection>) -> Value {
    match lsp_parse_with_frontend(src, frontend).and_then(normalize_family) {
        Ok(model) => {
            let pages = render_svg_pages_from_model(&model);
            json!({
                "svg": pages.first().cloned().unwrap_or_default(),
                "svgs": pages,
                "width": 0,
                "height": 0,
                "diagnostics": []
            })
        }
        Err(d) => json!({
            "svg": "",
            "svgs": [],
            "width": 0,
            "height": 0,
            "diagnostics": [d.to_json_with_source(src)]
        }),
    }
}

fn lsp_parse(src: &str) -> Result<Document, puml::Diagnostic> {
    lsp_parse_with_frontend(src, None)
}

fn lsp_parse_with_frontend(
    src: &str,
    frontend: Option<FrontendSelection>,
) -> Result<Document, puml::Diagnostic> {
    parse_with_pipeline_options(
        src,
        &ParsePipelineOptions {
            frontend: frontend.unwrap_or(FrontendSelection::Auto),
            ..ParsePipelineOptions::default()
        },
    )
}

fn lsp_frontend_hint(params: &Value) -> Option<FrontendSelection> {
    let raw = params
        .get("frontend")
        .or_else(|| params.get("dialect"))
        .or_else(|| params.get("language"))
        .and_then(Value::as_str)?;
    frontend_selection_from_hint(raw)
}

fn frontend_selection_from_hint(raw: &str) -> Option<FrontendSelection> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "auto" | "puml" | "pumlx" => Some(FrontendSelection::Auto),
        "plantuml" | "uml" | "puml-sequence" | "uml-sequence" => Some(FrontendSelection::Plantuml),
        "mermaid" | "mmd" => Some(FrontendSelection::Mermaid),
        "picouml" | "pico" => Some(FrontendSelection::Picouml),
        _ => None,
    }
}

fn open(v: &Value) -> Option<(String, i64, String)> {
    Some((
        v.pointer("/params/textDocument/uri")?.as_str()?.to_string(),
        v.pointer("/params/textDocument/version")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        v.pointer("/params/textDocument/text")?
            .as_str()?
            .to_string(),
    ))
}
fn change(v: &Value, current: Option<&str>) -> Option<(String, i64, String)> {
    let arr = v.pointer("/params/contentChanges")?.as_array()?;
    let mut text = current.unwrap_or_default().to_string();
    let mut saw_full_replace = false;
    for change in arr {
        let delta = change.get("text")?.as_str()?;
        if let Some(range) = change.get("range") {
            apply_incremental_change(&mut text, range, delta)?;
        } else {
            text = delta.to_string();
            saw_full_replace = true;
        }
    }
    if arr.is_empty() {
        return None;
    }
    Some((
        v.pointer("/params/textDocument/uri")?.as_str()?.to_string(),
        v.pointer("/params/textDocument/version")
            .and_then(Value::as_i64)
            .unwrap_or(0),
        if saw_full_replace || current.is_some() {
            text
        } else {
            arr.last()?.get("text")?.as_str()?.to_string()
        },
    ))
}

fn apply_incremental_change(text: &mut String, range: &Value, replacement: &str) -> Option<()> {
    let sl = range.pointer("/start/line")?.as_u64()? as usize;
    let sc = range.pointer("/start/character")?.as_u64()? as usize;
    let el = range.pointer("/end/line")?.as_u64()? as usize;
    let ec = range.pointer("/end/character")?.as_u64()? as usize;
    let start = lc_to_offset(text, sl, sc);
    let end = lc_to_offset(text, el, ec);
    if start > end || end > text.len() {
        return None;
    }
    text.replace_range(start..end, replacement);
    Some(())
}

fn get_config_section(cfg: &Value, section: &str) -> Value {
    let mut cur = cfg;
    for part in section.split('.') {
        if part.is_empty() {
            return Value::Null;
        }
        match cur.get(part) {
            Some(next) => cur = next,
            None => return Value::Null,
        }
    }
    cur.clone()
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn incremental_change_replaces_target_span() {
        let mut src = "@startuml\nA -> B : hi\n@enduml\n".to_string();
        let range = json!({
            "start":{"line":1,"character":2},
            "end":{"line":1,"character":4}
        });
        apply_incremental_change(&mut src, &range, "-->").expect("valid change");
        assert!(src.contains("A --> B"));
    }

    #[test]
    fn change_applies_multiple_deltas_in_order() {
        let msg = json!({
            "params":{
                "textDocument":{"uri":"file:///d.puml","version":2},
                "contentChanges":[
                    {"range":{"start":{"line":1,"character":0},"end":{"line":1,"character":1}},"text":"X"},
                    {"range":{"start":{"line":1,"character":5},"end":{"line":1,"character":6}},"text":"Y"}
                ]
            }
        });
        let (_, _, updated) = change(&msg, Some("@startuml\nA -> B\n@enduml\n")).expect("change");
        assert!(updated.contains("X -> Y"));
    }

    #[test]
    fn definition_returns_location_shape() {
        let src = "@startuml\nparticipant Alice\nAlice -> Bob: hi\n@enduml\n";
        let doc = Doc {
            text: src.to_string(),
            version: 1,
            parsed: lsp_parse(src).ok(),
        };
        let out = definition(&doc, "file:///test.puml", (2, 1)).expect("definition");
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
    fn semantic_tokens_do_not_overlap_arrow_tokens() {
        let src = "@startuml\nAlice --> Bob\n@enduml\n";
        let doc = Doc {
            text: src.to_string(),
            version: 1,
            parsed: lsp_parse(src).ok(),
        };
        let out = semantic_tokens(&doc);
        let data = out
            .get("data")
            .and_then(Value::as_array)
            .expect("semantic token data");
        // one operator token for "-->", encoded in groups of 5 u32 values.
        let token_count = data.len() / 5;
        assert_eq!(token_count, 1);
        assert_eq!(data[2].as_u64(), Some(3));
        assert_eq!(data[3].as_u64(), Some(1));
    }

    #[test]
    fn document_links_handles_duplicate_lines_and_whitespace() {
        let src = "!include   ./a.puml\n!include   ./a.puml\n";
        let doc = Doc {
            text: src.to_string(),
            version: 1,
            parsed: None,
        };
        let out = document_links(&doc);
        let arr = out.as_array().expect("array");
        assert_eq!(arr.len(), 2);
        let first_start = arr[0]
            .get("range")
            .and_then(|r| r.get("start"))
            .and_then(|s| s.get("character"))
            .and_then(Value::as_u64)
            .expect("char");
        let second_line = arr[1]
            .get("range")
            .and_then(|r| r.get("start"))
            .and_then(|s| s.get("line"))
            .and_then(Value::as_u64)
            .expect("line");
        assert_eq!(first_start, 11);
        assert_eq!(second_line, 1);
    }

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

    #[test]
    fn render_result_uses_family_renderer_for_class_state_and_activity() {
        let cases = [
            ("@startuml\nclass User\n@enduml\n", "User"),
            (
                "@startuml\nstate Waiting\n[*] --> Waiting\n@enduml\n",
                "Waiting",
            ),
            ("@startuml\nstart\n:Work;\nstop\n@enduml\n", "Work"),
        ];

        for (src, needle) in cases {
            let out = render_result(src, None);
            assert_eq!(out["diagnostics"].as_array().expect("diagnostics").len(), 0);
            let svg = out["svg"].as_str().expect("svg");
            assert!(svg.contains("<svg"));
            assert!(
                svg.contains(needle),
                "family render output should contain {needle}: {svg}"
            );
        }
    }

    #[test]
    fn render_result_honors_frontend_hint_for_mermaid_and_picouml() {
        let mermaid = render_result(
            "classDiagram\nclass User\nUser : +id\n",
            Some(FrontendSelection::Mermaid),
        );
        assert_eq!(
            mermaid["diagnostics"]
                .as_array()
                .expect("mermaid diagnostics")
                .len(),
            0
        );
        assert!(mermaid["svg"]
            .as_str()
            .expect("mermaid svg")
            .contains("User"));

        let picouml = render_result(
            "@startpicouml\nAlice => Bob : request\n@endpicouml\n",
            Some(FrontendSelection::Picouml),
        );
        assert_eq!(
            picouml["diagnostics"]
                .as_array()
                .expect("picouml diagnostics")
                .len(),
            0
        );
        assert!(picouml["svg"]
            .as_str()
            .expect("picouml svg")
            .contains("request"));
    }

    #[test]
    fn publish_diagnostics_includes_diagnostic_code_when_present() {
        let mut out = Vec::new();
        let src = "@startuml\nA ->\n@enduml\n";
        pub_diag(&mut out, "file:///a.puml", 3, src).expect("publish diagnostics");

        let raw = String::from_utf8(out).expect("utf8");
        let payload = raw
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .expect("lsp frame");
        let msg: Value = serde_json::from_str(payload).expect("json frame");
        let first = msg["params"]["diagnostics"][0].clone();
        assert_eq!(first["source"], "puml");
        assert_eq!(first["severity"], 1);
        assert_eq!(first["code"], "E_ARROW_INVALID");
    }

    #[test]
    fn publish_diagnostics_does_not_fetch_url_includes() {
        let mut out = Vec::new();
        let src = "@startuml\n!include https://example.com/remote.puml\n@enduml\n";
        pub_diag(&mut out, "file:///a.puml", 3, src).expect("publish diagnostics");

        let raw = String::from_utf8(out).expect("utf8");
        let payload = raw
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .expect("lsp frame");
        let msg: Value = serde_json::from_str(payload).expect("json frame");
        let first = msg["params"]["diagnostics"][0].clone();
        assert_eq!(first["source"], "puml");
        assert_eq!(first["severity"], 1);
        assert_eq!(first["code"], "E_INCLUDE_URL_DISABLED");
    }
}
fn pub_diag(w: &mut impl Write, uri: &str, ver: i64, src: &str) -> io::Result<()> {
    let report = diagnostics_with_options(src, &ParsePipelineOptions::default());
    let ds = report.diagnostics.iter().map(diag).collect::<Vec<_>>();
    notif(
        w,
        "textDocument/publishDiagnostics",
        json!({"uri":uri,"version":ver,"diagnostics":ds}),
    )
}
fn diag(diagnostic: &puml::language_service::LanguageDiagnostic) -> Value {
    let range = diagnostic
        .range
        .map(|range| {
            json!({
                "start": {
                    "line": range.start.line.saturating_sub(1),
                    "character": range.start.column.saturating_sub(1)
                },
                "end": {
                    "line": range.end.line.saturating_sub(1),
                    "character": range.end.column.saturating_sub(1)
                }
            })
        })
        .unwrap_or_else(
            || json!({"start":{"line":0,"character":0},"end":{"line":0,"character":1}}),
        );
    json!({
        "range": range,
        "severity":lsp_severity(diagnostic.severity),
        "source":"puml",
        "code":diagnostic.code.clone(),
        "message":diagnostic.message.clone()
    })
}

fn lsp_severity(severity: Severity) -> i32 {
    match severity {
        Severity::Error => 1,
        Severity::Warning => 2,
    }
}

fn pos(src: &str, off: usize) -> Value {
    let (l, c) = offset_to_lc(src, off);
    json!({"line":l,"character":c})
}
fn read_msg(r: &mut impl BufRead) -> io::Result<Option<Value>> {
    let mut len = None;
    loop {
        let mut line = String::new();
        if r.read_line(&mut line)? == 0 {
            return Ok(None);
        };
        if line == "\r\n" {
            break;
        }
        if let Some(v) = line.strip_prefix("Content-Length:") {
            len = v.trim().parse::<usize>().ok();
        }
    }
    let n = match len {
        Some(v) => v,
        None => return Ok(None),
    };
    let mut b = vec![0; n];
    std::io::Read::read_exact(r, &mut b)?;
    Ok(serde_json::from_slice(&b).ok())
}
fn resp(w: &mut impl Write, id: Value, result: Value) -> io::Result<()> {
    send(w, &json!({"jsonrpc":"2.0","id":id,"result":result}))
}
fn err(w: &mut impl Write, id: Value, code: i32, m: &str) -> io::Result<()> {
    send(
        w,
        &json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":m}}),
    )
}
fn notif(w: &mut impl Write, m: &str, p: Value) -> io::Result<()> {
    send(w, &json!({"jsonrpc":"2.0","method":m,"params":p}))
}
fn send(w: &mut impl Write, v: &Value) -> io::Result<()> {
    let b = serde_json::to_vec(v)?;
    write!(w, "Content-Length: {}\r\n\r\n", b.len())?;
    w.write_all(&b)?;
    w.flush()
}
