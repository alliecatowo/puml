use super::collections::preprocessor_list_items;
use super::scanner::read_json_value;
use super::value::strip_quotes;

pub(in crate::preproc) fn get_json_attribute(json: &str, key: &str) -> String {
    if let Ok(root) = serde_json::from_str::<serde_json::Value>(json.trim()) {
        if let Some(value) = json_value_at_path(&root, key) {
            return json_value_to_preproc_string(value);
        }
        return String::new();
    }

    // Split the key path into segments: "a.b[2].c" → ["a", "b", "[2]", "c"]
    let segments = split_json_path(key);
    let mut current = json.trim().to_string();
    for segment in &segments {
        if let Some(idx_str) = segment.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            // Array index access
            let idx: usize = idx_str.trim().parse().unwrap_or(usize::MAX);
            current = json_array_index(&current, idx);
        } else {
            // Object key access
            current = get_json_top_level_key(&current, segment);
        }
        if current.is_empty() {
            return String::new();
        }
    }
    current
}

pub(super) fn json_value_at_path<'a>(
    root: &'a serde_json::Value,
    path: &str,
) -> Option<&'a serde_json::Value> {
    let mut current = root;
    for segment in split_json_path(path) {
        if let Some(inner) = segment.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            if let Ok(idx) = inner.trim().parse::<usize>() {
                current = current.as_array()?.get(idx)?;
            } else {
                let key = strip_quotes(inner.trim());
                current = current.as_object()?.get(key.as_str())?;
            }
        } else {
            current = current.as_object()?.get(segment.as_str())?;
        }
    }
    Some(current)
}

pub(super) fn json_value_to_preproc_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => value.to_string(),
    }
}

/// Split a JSON path like `users[0].name` into segments `["users", "[0]", "name"]`.
pub(super) fn split_json_path(path: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = path.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '.' => {
                if !current.is_empty() {
                    segments.push(current.clone());
                    current.clear();
                }
            }
            '[' => {
                if !current.is_empty() {
                    segments.push(current.clone());
                    current.clear();
                }
                current.push('[');
                i += 1;
                while i < chars.len() && chars[i] != ']' {
                    current.push(chars[i]);
                    i += 1;
                }
                current.push(']');
                segments.push(current.clone());
                current.clear();
            }
            c => current.push(c),
        }
        i += 1;
    }
    if !current.is_empty() {
        segments.push(current);
    }
    segments
}

/// Look up a single top-level object key in a JSON string.
fn get_json_top_level_key(json: &str, key: &str) -> String {
    let bytes = json.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'{' {
        return String::new();
    }
    i += 1;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] == b'}' {
            break;
        }
        if bytes[i] != b'"' {
            return String::new();
        }
        i += 1;
        let key_start = i;
        while i < bytes.len() && bytes[i] != b'"' {
            if bytes[i] == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            i += 1;
        }
        if i >= bytes.len() {
            return String::new();
        }
        let candidate = &json[key_start..i];
        i += 1; // closing "
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b':' {
            return String::new();
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let value_start = i;
        let value = read_json_value(bytes, &mut i);
        if candidate == key {
            return value.unwrap_or_else(|| json[value_start..i].to_string());
        }
    }
    String::new()
}

/// Return the Nth element of a JSON array as a string (for further traversal).
fn json_array_index(json: &str, idx: usize) -> String {
    let bytes = json.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= bytes.len() || bytes[i] != b'[' {
        return String::new();
    }
    i += 1;
    let mut count = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b',') {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] == b']' {
            break;
        }
        let value_start = i;
        let value = read_json_value(bytes, &mut i);
        if count == idx {
            return value.unwrap_or_else(|| json[value_start..i].to_string());
        }
        count += 1;
    }
    String::new()
}

pub(super) fn json_contains_key(json: &str, key: &str) -> bool {
    if let Ok(root) = serde_json::from_str::<serde_json::Value>(json.trim()) {
        return json_value_at_path(&root, key).is_some();
    }

    // Reuse the top-level key scan rather than the full path traversal so that
    // an empty-value key still reports as present (PlantUML semantics).
    !get_json_top_level_key(json, key).is_empty()
}

pub(super) fn json_contains_value(json: &str, needle: &str) -> bool {
    let Ok(root) = serde_json::from_str::<serde_json::Value>(json.trim()) else {
        return preprocessor_list_items(json)
            .iter()
            .any(|item| item == needle);
    };
    json_value_contains_preproc_string(&root, needle)
}

fn json_value_contains_preproc_string(value: &serde_json::Value, needle: &str) -> bool {
    if json_value_to_preproc_string(value) == needle {
        return true;
    }
    match value {
        serde_json::Value::Array(items) => items
            .iter()
            .any(|item| json_value_contains_preproc_string(item, needle)),
        serde_json::Value::Object(obj) => obj
            .values()
            .any(|item| json_value_contains_preproc_string(item, needle)),
        _ => false,
    }
}
