use super::collections::preprocessor_list_items;
use super::json::{json_value_at_path, json_value_to_preproc_string, split_json_path};
use super::value::parse_int_lenient;
use crate::preproc::MAX_PREPROC_WHILE_ITERATIONS;

pub(super) fn preprocessor_size(raw: &str) -> usize {
    let trimmed = raw.trim();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(items) = value.as_array() {
            return items.len();
        }
        if let Some(obj) = value.as_object() {
            return obj.len();
        }
    }
    if trimmed.starts_with('[') && trimmed.ends_with(']') {
        return preprocessor_list_items(trimmed).len();
    }
    trimmed.chars().count()
}

pub(super) fn preprocessor_range(start: &str, end: &str, step: Option<&str>) -> String {
    let start = parse_int_lenient(start);
    let end = parse_int_lenient(end);
    let mut step = step
        .map(parse_int_lenient)
        .unwrap_or_else(|| if start <= end { 1 } else { -1 });
    if step == 0 {
        step = if start <= end { 1 } else { -1 };
    }
    let mut values = Vec::new();
    let mut current = start;
    let mut guard = 0usize;
    while guard <= MAX_PREPROC_WHILE_ITERATIONS
        && ((step > 0 && current <= end) || (step < 0 && current >= end))
    {
        values.push(current.to_string());
        current += step;
        guard += 1;
    }
    preprocessor_list_literal(&values)
}

pub(super) fn preprocessor_list_literal(items: &[String]) -> String {
    let values = items
        .iter()
        .map(|item| json_value_from_preproc(item))
        .collect::<Vec<_>>();
    serde_json::Value::Array(values).to_string()
}

pub(super) fn preprocessor_map_literal(args: &[String]) -> String {
    let mut obj = serde_json::Map::new();
    for chunk in args.chunks(2) {
        if let [key, value] = chunk {
            obj.insert(key.clone(), json_value_from_preproc(value));
        }
    }
    serde_json::Value::Object(obj).to_string()
}

pub(super) fn preprocessor_map_entries(json: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json.trim()) else {
        return "[]".to_string();
    };
    let Some(obj) = value.as_object() else {
        return "[]".to_string();
    };
    let rows = obj
        .iter()
        .map(|(key, value)| {
            serde_json::Value::Array(vec![
                serde_json::Value::String(key.clone()),
                serde_json::Value::String(json_value_to_preproc_string(value)),
            ])
        })
        .collect::<Vec<_>>();
    serde_json::Value::Array(rows).to_string()
}

pub(super) fn preprocessor_str2json(raw: &str) -> String {
    serde_json::from_str::<serde_json::Value>(raw.trim())
        .unwrap_or_else(|_| serde_json::Value::String(raw.to_string()))
        .to_string()
}

pub(super) fn preprocessor_get_opt(container: &str, key: &str) -> Option<String> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(container.trim()) {
        if let Ok(idx) = key.trim().parse::<usize>() {
            return value
                .as_array()
                .and_then(|items| items.get(idx))
                .map(json_value_to_preproc_string);
        }
        return json_value_at_path(&value, key).map(json_value_to_preproc_string);
    }
    preprocessor_list_items(container)
        .get(key.trim().parse::<usize>().unwrap_or(usize::MAX))
        .cloned()
}

pub(super) fn preprocessor_set(container: &str, key: &str, replacement: &str) -> String {
    if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(container.trim()) {
        if set_json_value_at_path(
            &mut value,
            &split_json_path(key),
            json_value_from_preproc(replacement),
        ) {
            return value.to_string();
        }
        if let Some(obj) = value.as_object_mut() {
            obj.insert(
                key.to_string(),
                serde_json::Value::String(replacement.to_string()),
            );
            return serde_json::Value::Object(obj.clone()).to_string();
        }
        if let Some(arr) = value.as_array_mut() {
            if let Ok(idx) = key.trim().parse::<usize>() {
                if let Some(slot) = arr.get_mut(idx) {
                    *slot = serde_json::Value::String(replacement.to_string());
                }
            }
            return serde_json::Value::Array(arr.clone()).to_string();
        }
    }
    container.to_string()
}

pub(super) fn preprocessor_remove(container: &str, key: &str) -> String {
    if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(container.trim()) {
        let _ = remove_json_value_at_path(&mut value, &split_json_path(key));
        return value.to_string();
    }
    let mut items = preprocessor_list_items(container);
    if let Ok(idx) = key.trim().parse::<usize>() {
        if idx < items.len() {
            items.remove(idx);
        }
    } else {
        items.retain(|item| item != key);
    }
    preprocessor_list_literal(&items)
}

pub(super) fn preprocessor_json_merge(lhs: &str, rhs: &str) -> String {
    let Ok(mut left) = serde_json::from_str::<serde_json::Value>(lhs.trim()) else {
        return rhs.to_string();
    };
    let Ok(right) = serde_json::from_str::<serde_json::Value>(rhs.trim()) else {
        return left.to_string();
    };
    merge_json_values(&mut left, right);
    left.to_string()
}

fn merge_json_values(left: &mut serde_json::Value, right: serde_json::Value) {
    match (left, right) {
        (serde_json::Value::Object(dst), serde_json::Value::Object(src)) => {
            for (key, value) in src {
                match dst.get_mut(&key) {
                    Some(existing) => merge_json_values(existing, value),
                    None => {
                        dst.insert(key, value);
                    }
                }
            }
        }
        (serde_json::Value::Array(dst), serde_json::Value::Array(src)) => {
            dst.extend(src);
        }
        (dst, src) => {
            *dst = src;
        }
    }
}

pub(super) fn preprocessor_json_type(raw: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(raw.trim()) {
        Ok(serde_json::Value::Object(_)) => "object".to_string(),
        Ok(serde_json::Value::Array(_)) => "array".to_string(),
        Ok(serde_json::Value::String(_)) => "string".to_string(),
        Ok(serde_json::Value::Number(_)) => "number".to_string(),
        Ok(serde_json::Value::Bool(_)) => "boolean".to_string(),
        Ok(serde_json::Value::Null) => "null".to_string(),
        Err(_) => "string".to_string(),
    }
}

pub(super) fn json_value_from_preproc(raw: &str) -> serde_json::Value {
    serde_json::from_str::<serde_json::Value>(raw.trim())
        .unwrap_or_else(|_| serde_json::Value::String(raw.to_string()))
}

fn set_json_value_at_path(
    value: &mut serde_json::Value,
    segments: &[String],
    replacement: serde_json::Value,
) -> bool {
    let Some((head, tail)) = segments.split_first() else {
        *value = replacement;
        return true;
    };
    if let Some(inner) = head.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        let Ok(idx) = inner.trim().parse::<usize>() else {
            return false;
        };
        if !value.is_array() {
            *value = serde_json::Value::Array(Vec::new());
        }
        let Some(items) = value.as_array_mut() else {
            return false;
        };
        while items.len() <= idx {
            items.push(serde_json::Value::Null);
        }
        if tail.is_empty() {
            items[idx] = replacement;
            true
        } else {
            set_json_value_at_path(&mut items[idx], tail, replacement)
        }
    } else {
        if !value.is_object() {
            *value = serde_json::Value::Object(serde_json::Map::new());
        }
        let Some(obj) = value.as_object_mut() else {
            return false;
        };
        if tail.is_empty() {
            obj.insert(head.clone(), replacement);
            true
        } else {
            let next = obj.entry(head.clone()).or_insert_with(|| {
                if tail.first().map(|s| s.starts_with('[')).unwrap_or(false) {
                    serde_json::Value::Array(Vec::new())
                } else {
                    serde_json::Value::Object(serde_json::Map::new())
                }
            });
            set_json_value_at_path(next, tail, replacement)
        }
    }
}

fn remove_json_value_at_path(value: &mut serde_json::Value, segments: &[String]) -> bool {
    let Some((head, tail)) = segments.split_first() else {
        return false;
    };
    if let Some(inner) = head.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        let Ok(idx) = inner.trim().parse::<usize>() else {
            return false;
        };
        let Some(items) = value.as_array_mut() else {
            return false;
        };
        if tail.is_empty() {
            if idx < items.len() {
                items.remove(idx);
                return true;
            }
            return false;
        }
        items
            .get_mut(idx)
            .map(|next| remove_json_value_at_path(next, tail))
            .unwrap_or(false)
    } else {
        let Some(obj) = value.as_object_mut() else {
            return false;
        };
        if tail.is_empty() {
            return obj.remove(head).is_some();
        }
        obj.get_mut(head)
            .map(|next| remove_json_value_at_path(next, tail))
            .unwrap_or(false)
    }
}

pub(super) fn preprocessor_json_keys(json: &str) -> Vec<String> {
    serde_json::from_str::<serde_json::Value>(json.trim())
        .ok()
        .and_then(|value| {
            value.as_object().map(|obj| {
                obj.keys()
                    .map(|key| format!("\"{}\"", key.replace('"', "\\\"")))
                    .collect::<Vec<_>>()
            })
        })
        .unwrap_or_default()
}

pub(super) fn preprocessor_json_values(json: &str) -> Vec<String> {
    serde_json::from_str::<serde_json::Value>(json.trim())
        .ok()
        .and_then(|value| {
            value.as_object().map(|obj| {
                obj.values()
                    .map(json_value_to_preproc_string)
                    .collect::<Vec<_>>()
            })
        })
        .unwrap_or_default()
}
