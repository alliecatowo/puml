use super::protocol::{pos, range};
use super::render::lsp_parse;
use super::Doc;
use puml::ast::StatementKind;
use puml::language_service::{format_document, offset_to_lc, word_range_at_pos};
use puml::normalize_family;
use serde_json::{json, Value};

#[derive(Clone, Debug)]
struct RefHit {
    start: usize,
    end: usize,
}

pub fn formatting_edits(text: &str) -> Value {
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

pub fn folding_ranges(d: &Doc) -> Value {
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

pub fn selection_ranges(d: &Doc, msg: &Value) -> Value {
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

pub fn document_links(d: &Doc) -> Value {
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

pub fn document_colors(d: &Doc) -> Value {
    let mut out = Vec::new();
    for hit in find_hex_colors(&d.text) {
        if let Some((r, g, b, a)) = decode_hex_color(&d.text[hit.start..hit.end]) {
            out.push(json!({"range":range(&d.text,hit.start,hit.end),"color":{"red":r,"green":g,"blue":b,"alpha":a}}));
        }
    }
    Value::Array(out)
}

pub fn color_presentation(msg: &Value) -> Value {
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

pub fn code_actions(uri: &str, d: &Doc, msg: &Value) -> Value {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lsp_adapter::render::lsp_parse;
    use serde_json::json;

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
    fn folding_selection_colors_and_presentations_cover_lsp_shape_helpers() {
        let src = "@startuml\nAlice -> Bob: hi\nalt ok\nBob -> Alice: yes\nend\n@enduml\n";
        let doc = Doc {
            text: src.to_string(),
            version: 1,
            parsed: lsp_parse(src).ok(),
        };

        let folds = folding_ranges(&doc);
        assert!(folds.as_array().expect("fold ranges").len() <= 1);

        let selections = selection_ranges(
            &doc,
            &json!({
                "params": {
                    "positions": [
                        {"line": 1, "character": 6},
                        {"line": 0, "character": 0}
                    ]
                }
            }),
        );
        assert_eq!(selections.as_array().expect("selection ranges").len(), 2);

        let color_doc = Doc {
            text:
                "@startuml\nskinparam backgroundColor #abc\nAlice -> Bob #11223344: hi\n@enduml\n"
                    .to_string(),
            version: 1,
            parsed: None,
        };
        let colors = document_colors(&color_doc);
        let arr = colors.as_array().expect("document colors");
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["color"]["alpha"], 1.0);
        assert!(arr[1]["color"]["alpha"].as_f64().expect("alpha") < 1.0);

        let opaque = color_presentation(&json!({
            "params": {"color": {"red": 1.0, "green": 0.5, "blue": 0.0, "alpha": 1.0}}
        }));
        assert_eq!(opaque[0]["label"], "#FF7F00");

        let translucent = color_presentation(&json!({
            "params": {"color": {"red": 0.0, "green": 0.0, "blue": 1.0, "alpha": 0.5}}
        }));
        assert_eq!(translucent[0]["label"], "#0000FF7F");
    }

    #[test]
    fn code_actions_cover_success_and_error_paths() {
        let valid_src = "@startuml\nAlice -> Bob: hi\n@enduml\n";
        let invalid_src = "@startuml\nAlice ->\n@enduml\n";
        let valid_doc = Doc {
            text: valid_src.to_string(),
            version: 1,
            parsed: lsp_parse(valid_src).ok(),
        };
        let invalid_doc = Doc {
            text: invalid_src.to_string(),
            version: 1,
            parsed: lsp_parse(invalid_src).ok(),
        };

        let valid_actions = code_actions(
            "file:///valid.puml",
            &valid_doc,
            &json!({"params": {"context": {"diagnostics": []}}}),
        );
        let valid_titles = valid_actions
            .as_array()
            .expect("actions")
            .iter()
            .map(|item| item["title"].as_str().expect("title"))
            .collect::<Vec<_>>();
        assert!(valid_titles.contains(&"Format document"));
        assert!(valid_titles.contains(&"Render SVG preview"));

        let invalid_actions = code_actions(
            "file:///invalid.puml",
            &invalid_doc,
            &json!({"params": {"context": {"diagnostics": [{"severity": 1}]}}}),
        );
        let invalid_titles = invalid_actions
            .as_array()
            .expect("actions")
            .iter()
            .map(|item| item["title"].as_str().expect("title"))
            .collect::<Vec<_>>();
        assert!(invalid_titles.contains(&"Fix formatting and retry"));
        assert!(!invalid_titles.contains(&"Render SVG preview"));
    }
}
