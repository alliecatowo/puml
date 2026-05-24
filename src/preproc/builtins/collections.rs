use super::constructors::preprocessor_list_literal;
use super::json::json_value_to_preproc_string;
use super::value::{parse_int_lenient, strip_quotes};

pub(in crate::preproc) fn preprocessor_list_items(raw: &str) -> Vec<String> {
    let trimmed = raw.trim();
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(items) = value.as_array() {
            return items.iter().map(json_value_to_preproc_string).collect();
        }
    }
    trimmed
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|s| strip_quotes(s.trim()))
        .filter(|s| !s.is_empty())
        .collect()
}

pub(super) fn preprocessor_list_slice(raw: &str, start: &str, len: Option<&str>) -> String {
    let items = preprocessor_list_items(raw);
    if items.is_empty() {
        return "[]".to_string();
    }
    let start = parse_int_lenient(start).max(0) as usize;
    let start = start.min(items.len());
    let end = match len {
        Some(value) => {
            let len = parse_int_lenient(value);
            if len < 0 {
                items.len()
            } else {
                start.saturating_add(len as usize).min(items.len())
            }
        }
        None => items.len(),
    };
    preprocessor_list_literal(&items[start..end])
}

pub(in crate::preproc) fn preprocessor_foreach_bindings(
    var_names: &[String],
    rhs: &str,
) -> Vec<Vec<(String, String)>> {
    if var_names.len() == 1 {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(rhs.trim()) {
            if let Some(obj) = value.as_object() {
                return obj
                    .keys()
                    .map(|key| vec![(var_names[0].clone(), key.clone())])
                    .collect();
            }
        }
    }
    if var_names.len() == 2 {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(rhs.trim()) {
            if let Some(obj) = value.as_object() {
                return obj
                    .iter()
                    .map(|(key, value)| {
                        vec![
                            (var_names[0].clone(), key.clone()),
                            (var_names[1].clone(), json_value_to_preproc_string(value)),
                        ]
                    })
                    .collect();
            }
            if let Some(items) = value.as_array() {
                return items
                    .iter()
                    .enumerate()
                    .map(|(idx, value)| {
                        vec![
                            (var_names[0].clone(), idx.to_string()),
                            (var_names[1].clone(), json_value_to_preproc_string(value)),
                        ]
                    })
                    .collect();
            }
        }
    }

    preprocessor_list_items(rhs)
        .into_iter()
        .enumerate()
        .map(|(idx, item)| {
            if var_names.len() == 1 {
                return vec![(var_names[0].clone(), item)];
            }
            let mut values = preprocessor_list_items(&item);
            if values.len() <= 1 {
                values = vec![idx.to_string(), item];
            }
            var_names
                .iter()
                .enumerate()
                .map(|(var_idx, name)| {
                    (
                        name.clone(),
                        values.get(var_idx).cloned().unwrap_or_default(),
                    )
                })
                .collect()
        })
        .collect()
}
