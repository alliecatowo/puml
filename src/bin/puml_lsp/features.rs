use super::document::{lsp_parse, Doc};
use super::position::{is_ident, offset_to_lc, pos, range, word_range_at_pos, RefHit};
use super::rendering::{lsp_frontend_hint, render_result};
use puml::ast::StatementKind;
use puml::language_service::{
    completion_items, document_symbols, format_document, hover as language_hover,
    resolve_completion_item, semantic_tokens as shared_semantic_tokens, CompletionItemKind,
    DocumentSymbolKind, SemanticTokenKind,
};
use puml::normalize_family;
use serde_json::{json, Value};
use std::collections::HashMap;

pub(crate) fn completions() -> Vec<Value> {
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

pub(crate) fn resolve_completion(item: &Value) -> Value {
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

pub(crate) fn hover(d: &Doc, posn: (u64, u64)) -> Option<Value> {
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

pub(crate) fn definition(d: &Doc, uri: &str, posn: (u64, u64)) -> Option<Value> {
    let (s, e) = word_range_at_pos(&d.text, posn)?;
    let sym = &d.text[s..e];
    let decl = find_participant_decl(d.parsed.as_ref()?, sym)?;
    Some(json!([{"uri":uri,"range":range(&d.text,decl.0,decl.1)}]))
}

pub(crate) fn references(d: &Doc, uri: &str, posn: (u64, u64)) -> Value {
    let mut out = Vec::new();
    if let Some((s, e)) = word_range_at_pos(&d.text, posn) {
        let sym = &d.text[s..e];
        for hit in find_word_refs(&d.text, sym) {
            out.push(json!({"uri":uri,"range":range(&d.text,hit.start,hit.end)}));
        }
    }
    Value::Array(out)
}

pub(crate) fn rename(d: &Doc, uri: &str, posn: (u64, u64), new_name: &str) -> Value {
    let mut edits = Vec::new();
    if let Some((s, e)) = word_range_at_pos(&d.text, posn) {
        let sym = &d.text[s..e];
        for hit in find_word_refs(&d.text, sym) {
            edits.push(json!({"range":range(&d.text,hit.start,hit.end),"newText":new_name}));
        }
    }
    json!({"changes":{uri:edits}})
}

pub(crate) fn document_symbols_lsp(d: &Doc) -> Value {
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

pub(crate) fn workspace_symbols(docs: &HashMap<String, Doc>, q: &str) -> Value {
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

pub(crate) fn formatting_edits(text: &str) -> Value {
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

pub(crate) fn folding_ranges(d: &Doc) -> Value {
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

pub(crate) fn selection_ranges(d: &Doc, msg: &Value) -> Value {
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

pub(crate) fn document_links(d: &Doc) -> Value {
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

pub(crate) fn semantic_tokens(d: &Doc) -> Value {
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

pub(crate) fn document_colors(d: &Doc) -> Value {
    let mut out = Vec::new();
    for hit in find_hex_colors(&d.text) {
        if let Some((r, g, b, a)) = decode_hex_color(&d.text[hit.start..hit.end]) {
            out.push(json!({"range":range(&d.text,hit.start,hit.end),"color":{"red":r,"green":g,"blue":b,"alpha":a}}));
        }
    }
    Value::Array(out)
}

pub(crate) fn color_presentation(msg: &Value) -> Value {
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

pub(crate) fn code_actions(uri: &str, d: &Doc, msg: &Value) -> Value {
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

pub(crate) fn execute_command(msg: &Value, docs: &HashMap<String, Doc>) -> Value {
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

fn find_participant_decl(doc: &puml::Document, sym: &str) -> Option<(usize, usize)> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
}
