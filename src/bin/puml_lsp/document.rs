use super::position::lc_to_offset;
use puml::{parse_with_pipeline_options, Document, FrontendSelection, ParsePipelineOptions};
use serde_json::Value;

#[derive(Clone, Debug)]
pub(crate) struct Doc {
    pub(crate) text: String,
    pub(crate) parsed: Option<Document>,
    pub(crate) version: i64,
}

pub(crate) fn lsp_parse(src: &str) -> Result<Document, puml::Diagnostic> {
    lsp_parse_with_frontend(src, None)
}

pub(crate) fn lsp_parse_with_frontend(
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

pub(crate) fn open(v: &Value) -> Option<(String, i64, String)> {
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

pub(crate) fn change(v: &Value, current: Option<&str>) -> Option<(String, i64, String)> {
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

pub(crate) fn apply_incremental_change(
    text: &mut String,
    range: &Value,
    replacement: &str,
) -> Option<()> {
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

pub(crate) fn get_config_section(cfg: &Value, section: &str) -> Value {
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
}
