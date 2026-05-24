use puml::language_service::lc_to_offset;
use serde_json::Value;

pub fn open(v: &Value) -> Option<(String, i64, String)> {
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

pub fn change(v: &Value, current: Option<&str>) -> Option<(String, i64, String)> {
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

pub fn apply_incremental_change(text: &mut String, range: &Value, replacement: &str) -> Option<()> {
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

pub fn get_config_section(cfg: &Value, section: &str) -> Value {
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
    use puml::language_service::{lc_to_offset, references, word_range_at_pos};
    use serde_json::{json, Value};

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
    fn config_and_change_helpers_cover_empty_invalid_and_boundary_paths() {
        let config = json!({
            "puml": {
                "server": {
                    "trace": "messages"
                }
            }
        });
        assert_eq!(
            get_config_section(&config, "puml.server.trace"),
            Value::String("messages".to_string())
        );
        assert_eq!(get_config_section(&config, "puml.missing"), Value::Null);
        assert_eq!(get_config_section(&config, "puml..trace"), Value::Null);

        let empty_change = json!({
            "params": {
                "textDocument": {"uri": "file:///d.puml", "version": 2},
                "contentChanges": []
            }
        });
        assert!(change(&empty_change, Some("@startuml\n@enduml\n")).is_none());

        let full_replace = json!({
            "params": {
                "textDocument": {"uri": "file:///d.puml", "version": 3},
                "contentChanges": [{"text": "@startuml\nBob -> Alice\n@enduml\n"}]
            }
        });
        let (_, version, updated) = change(&full_replace, None).expect("full replace");
        assert_eq!(version, 3);
        assert!(updated.contains("Bob -> Alice"));

        let mut original = "abc".to_string();
        let bad_range = json!({
            "start": {"line": 0, "character": 3},
            "end": {"line": 0, "character": 1}
        });
        assert!(apply_incremental_change(&mut original, &bad_range, "x").is_none());

        assert_eq!(references("Alice Alice_Bob Bob", (0, 1)).len(), 1);
        assert!(word_range_at_pos("Alice -> Bob", (0, 5)).is_none());
        assert_eq!(lc_to_offset("a\nβ", 1, 1), "a\nβ".len());
    }
}
