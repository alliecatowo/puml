use crate::preproc::MAX_PREPROC_WHILE_ITERATIONS;

use super::args::{parse_int_lenient, strip_quotes};

/// JSON key lookup supporting simple dot-path and array-index access so
/// `%get_json_attribute` can serve patterns like:
///   `%get_json_attribute($cfg, "name")`           — top-level string key
///   `%get_json_attribute($cfg, "users[0].name")`  — nested path
///
/// Returns the value as a string (quotes stripped for string values; numeric /
/// boolean / null left verbatim). Returns an empty string when the input is
/// not valid JSON, the path is missing, or the value is a nested
/// object/array (callers may then pass sub-JSON to a further call).
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

fn json_value_at_path<'a>(
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

fn json_value_to_preproc_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => value.to_string(),
    }
}

/// Split a JSON path like `users[0].name` into segments `["users", "[0]", "name"]`.
fn split_json_path(path: &str) -> Vec<String> {
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

pub(in crate::preproc) fn json_contains_key(json: &str, key: &str) -> bool {
    if let Ok(root) = serde_json::from_str::<serde_json::Value>(json.trim()) {
        return json_value_at_path(&root, key).is_some();
    }

    // Reuse the top-level key scan rather than the full path traversal so that
    // an empty-value key still reports as present (PlantUML semantics).
    !get_json_top_level_key(json, key).is_empty()
}

pub(in crate::preproc) fn json_contains_value(json: &str, needle: &str) -> bool {
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

pub(in crate::preproc) fn preprocessor_list_slice(
    raw: &str,
    start: &str,
    len: Option<&str>,
) -> String {
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

#[derive(Clone)]
enum SimpleRegexAtom {
    Any,
    Literal(char),
    Whitespace,
    Digit,
    Word,
    Class(Vec<(char, char)>, bool),
}

#[derive(Clone)]
struct SimpleRegexPart {
    atom: SimpleRegexAtom,
    min: usize,
    max: Option<usize>,
}

pub(in crate::preproc) fn split_preprocessor_regex(s: &str, pattern: &str) -> Vec<String> {
    if pattern.is_empty() {
        return vec![s.to_string()];
    }
    let Some(parts) = parse_simple_regex(pattern) else {
        return s.split(pattern).map(str::to_string).collect();
    };
    let chars = s.chars().collect::<Vec<_>>();
    let mut fields = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        if let Some(len) = match_simple_regex_at(&chars, i, &parts, 0) {
            if len > 0 {
                fields.push(chars[start..i].iter().collect());
                i += len;
                start = i;
                continue;
            }
        }
        i += 1;
    }
    fields.push(chars[start..].iter().collect());
    fields
}

fn parse_simple_regex(pattern: &str) -> Option<Vec<SimpleRegexPart>> {
    let chars = pattern.chars().collect::<Vec<_>>();
    let mut parts = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let atom = match chars[i] {
            '\\' => {
                i += 1;
                if i >= chars.len() {
                    return None;
                }
                match chars[i] {
                    's' => SimpleRegexAtom::Whitespace,
                    'd' => SimpleRegexAtom::Digit,
                    'w' => SimpleRegexAtom::Word,
                    other => SimpleRegexAtom::Literal(other),
                }
            }
            '[' => {
                let (atom, next) = parse_simple_regex_class(&chars, i + 1)?;
                i = next;
                atom
            }
            '.' => SimpleRegexAtom::Any,
            '|' | '(' | ')' | '{' | '}' => return None,
            other => SimpleRegexAtom::Literal(other),
        };
        i += 1;
        let (min, max) = if i < chars.len() {
            match chars[i] {
                '+' => {
                    i += 1;
                    (1, None)
                }
                '*' => {
                    i += 1;
                    (0, None)
                }
                '?' => {
                    i += 1;
                    (0, Some(1))
                }
                _ => (1, Some(1)),
            }
        } else {
            (1, Some(1))
        };
        parts.push(SimpleRegexPart { atom, min, max });
    }
    Some(parts)
}

fn parse_simple_regex_class(chars: &[char], mut i: usize) -> Option<(SimpleRegexAtom, usize)> {
    let mut negated = false;
    if i < chars.len() && chars[i] == '^' {
        negated = true;
        i += 1;
    }
    let mut ranges = Vec::new();
    while i < chars.len() && chars[i] != ']' {
        let start = if chars[i] == '\\' {
            i += 1;
            if i >= chars.len() {
                return None;
            }
            chars[i]
        } else {
            chars[i]
        };
        if i + 2 < chars.len() && chars[i + 1] == '-' && chars[i + 2] != ']' {
            let end = chars[i + 2];
            ranges.push((start, end));
            i += 3;
        } else {
            ranges.push((start, start));
            i += 1;
        }
    }
    if i >= chars.len() || chars[i] != ']' {
        return None;
    }
    Some((SimpleRegexAtom::Class(ranges, negated), i))
}

fn match_simple_regex_at(
    chars: &[char],
    pos: usize,
    parts: &[SimpleRegexPart],
    part_idx: usize,
) -> Option<usize> {
    if part_idx >= parts.len() {
        return Some(0);
    }
    let part = &parts[part_idx];
    let mut max_count = 0usize;
    while pos + max_count < chars.len()
        && part.max.map(|max| max_count < max).unwrap_or(true)
        && simple_regex_atom_matches(&part.atom, chars[pos + max_count])
    {
        max_count += 1;
    }
    if max_count < part.min {
        return None;
    }
    for count in (part.min..=max_count).rev() {
        if let Some(rest) = match_simple_regex_at(chars, pos + count, parts, part_idx + 1) {
            return Some(count + rest);
        }
    }
    None
}

fn simple_regex_atom_matches(atom: &SimpleRegexAtom, ch: char) -> bool {
    match atom {
        SimpleRegexAtom::Any => true,
        SimpleRegexAtom::Literal(lit) => *lit == ch,
        SimpleRegexAtom::Whitespace => ch.is_whitespace(),
        SimpleRegexAtom::Digit => ch.is_ascii_digit(),
        SimpleRegexAtom::Word => ch.is_ascii_alphanumeric() || ch == '_',
        SimpleRegexAtom::Class(ranges, negated) => {
            let matched = ranges.iter().any(|(start, end)| *start <= ch && ch <= *end);
            matched ^ *negated
        }
    }
}

pub(in crate::preproc) fn preprocessor_size(raw: &str) -> usize {
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

pub(in crate::preproc) fn preprocessor_range(start: &str, end: &str, step: Option<&str>) -> String {
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

pub(in crate::preproc) fn preprocessor_list_literal(items: &[String]) -> String {
    let values = items
        .iter()
        .map(|item| json_value_from_preproc(item))
        .collect::<Vec<_>>();
    serde_json::Value::Array(values).to_string()
}

pub(in crate::preproc) fn preprocessor_map_literal(args: &[String]) -> String {
    let mut obj = serde_json::Map::new();
    for chunk in args.chunks(2) {
        if let [key, value] = chunk {
            obj.insert(key.clone(), json_value_from_preproc(value));
        }
    }
    serde_json::Value::Object(obj).to_string()
}

pub(in crate::preproc) fn preprocessor_map_entries(json: &str) -> String {
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

pub(in crate::preproc) fn preprocessor_str2json(raw: &str) -> String {
    serde_json::from_str::<serde_json::Value>(raw.trim())
        .unwrap_or_else(|_| serde_json::Value::String(raw.to_string()))
        .to_string()
}

pub(in crate::preproc) fn preprocessor_get_opt(container: &str, key: &str) -> Option<String> {
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

pub(in crate::preproc) fn preprocessor_set(
    container: &str,
    key: &str,
    replacement: &str,
) -> String {
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

pub(in crate::preproc) fn preprocessor_remove(container: &str, key: &str) -> String {
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

pub(in crate::preproc) fn preprocessor_json_merge(lhs: &str, rhs: &str) -> String {
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

pub(in crate::preproc) fn preprocessor_json_type(raw: &str) -> String {
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

pub(in crate::preproc) fn json_value_from_preproc(raw: &str) -> serde_json::Value {
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

pub(in crate::preproc) fn preprocessor_json_keys(json: &str) -> Vec<String> {
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

pub(in crate::preproc) fn preprocessor_json_values(json: &str) -> Vec<String> {
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

/// Read a JSON-ish scalar/object/array value starting at `*idx`, advancing
/// `*idx` past the value. Returns `Some(str)` for unwrapped string scalars.
fn read_json_value(bytes: &[u8], idx: &mut usize) -> Option<String> {
    if *idx >= bytes.len() {
        return None;
    }
    match bytes[*idx] {
        b'"' => {
            *idx += 1;
            let start = *idx;
            while *idx < bytes.len() && bytes[*idx] != b'"' {
                if bytes[*idx] == b'\\' && *idx + 1 < bytes.len() {
                    *idx += 2;
                    continue;
                }
                *idx += 1;
            }
            let end = *idx;
            if *idx < bytes.len() {
                *idx += 1; // closing "
            }
            std::str::from_utf8(&bytes[start..end])
                .ok()
                .map(str::to_string)
        }
        b'{' | b'[' => {
            let open = bytes[*idx];
            let close = if open == b'{' { b'}' } else { b']' };
            let mut depth = 1usize;
            *idx += 1;
            while *idx < bytes.len() && depth > 0 {
                let c = bytes[*idx];
                if c == b'"' {
                    *idx += 1;
                    while *idx < bytes.len() && bytes[*idx] != b'"' {
                        if bytes[*idx] == b'\\' && *idx + 1 < bytes.len() {
                            *idx += 2;
                            continue;
                        }
                        *idx += 1;
                    }
                    if *idx < bytes.len() {
                        *idx += 1;
                    }
                    continue;
                }
                if c == open {
                    depth += 1;
                } else if c == close {
                    depth -= 1;
                }
                *idx += 1;
            }
            None
        }
        _ => {
            while *idx < bytes.len() {
                let c = bytes[*idx];
                if c == b',' || c == b'}' || c == b']' || c.is_ascii_whitespace() {
                    break;
                }
                *idx += 1;
            }
            None
        }
    }
}
