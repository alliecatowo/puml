use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::diagnostic::Diagnostic;

use super::{
    ParseOptions, PreprocCallable, PreprocCallableKind, PreprocParam, PreprocState,
    MAX_PREPROC_CALL_DEPTH, MAX_PREPROC_WHILE_ITERATIONS,
};

// Forward-declare functions that live in sibling modules but are called from here.
// We go through super:: to avoid import cycles.
use super::control::preprocess_text;
use super::macros::expand_preprocessor_text;

/// Dispatch a known preprocessor builtin. Returns `Ok(Some(result))` if the
/// name maps to a builtin, `Ok(None)` if the name is not recognised so the
/// caller can fall through to its unknown-function diagnostic.
///
/// Time/IO-sensitive builtins (`%date`, `%getenv`) deliberately return an
/// empty string. PlantUML's defaults inject the current wall-clock or process
/// environment, which would defeat determinism: identical source must yield
/// identical bytes for `cargo test`/`puml --check` to be useful.
pub(super) fn dispatch_builtin(
    name: &str,
    args_raw: &str,
    state: &PreprocState,
    call_depth: usize,
) -> Result<Option<String>, Diagnostic> {
    // Expand each argument as a preprocessor expression so callers may chain
    // builtins (e.g. `%upper(%substr("hello", 1, 3))`).
    let args = split_args(args_raw)?;
    let mut expanded_args = Vec::with_capacity(args.len());
    for a in &args {
        let trimmed = a.trim();
        let stripped = strip_quotes(trimmed);
        let val = if stripped.as_ptr() == trimmed.as_ptr() && stripped.len() == trimmed.len() {
            // Unquoted: still allow recursive expansion (e.g. `$var`).
            expand_preprocessor_text(trimmed, state, call_depth + 1)?
        } else {
            // Quoted: expand the inside (variable substitution still applies)
            // and preserve as a literal string.
            expand_preprocessor_text(&stripped, state, call_depth + 1)?
        };
        expanded_args.push(val);
    }
    let arg = |idx: usize| expanded_args.get(idx).cloned().unwrap_or_default();
    let argc = expanded_args.len();

    let result: Option<String> = match name {
        "strlen" | "length" | "len" => Some(arg(0).chars().count().to_string()),
        "count" => Some(preprocessor_size(&arg(0)).to_string()),
        "size" => Some(preprocessor_size(&arg(0)).to_string()),
        "eval" | "eval_int" => Some(
            super::includes::eval_int_expr(&arg(0))
                .map(|n| n.to_string())
                .unwrap_or_else(|| arg(0)),
        ),
        "eval_bool" | "eval_boolean" => {
            Some(super::includes::evaluate_scalar_expr(&arg(0))?.to_string())
        }
        "if" | "ternary" | "iif" => Some(if super::includes::evaluate_scalar_expr(&arg(0))? {
            arg(1)
        } else {
            arg(2)
        }),
        "splitstr" => {
            // %splitstr(s, sep) → returns the comma-joined fields after
            // splitting `s` on `sep`. PlantUML returns a deterministic
            // representation usable as the right-hand side of !foreach.
            let s = arg(0);
            let sep = arg(1);
            if sep.is_empty() {
                Some(s)
            } else {
                Some(s.split(sep.as_str()).collect::<Vec<&str>>().join(","))
            }
        }
        "splitstr_regex" | "split_regex" => {
            Some(split_preprocessor_regex(&arg(0), &arg(1)).join(","))
        }
        "split" => {
            let s = arg(0);
            let sep = arg(1);
            if sep.is_empty() {
                Some(s)
            } else {
                Some(
                    s.split(sep.as_str())
                        .map(|v| format!("\"{}\"", v.replace('"', "\\\"")))
                        .collect::<Vec<_>>()
                        .join(","),
                )
            }
        }
        "join" => {
            let list = preprocessor_list_items(&arg(0));
            Some(list.join(&arg(1)))
        }
        "list" | "array" | "newlist" => Some(preprocessor_list_literal(&expanded_args)),
        "range" => Some(preprocessor_range(
            &arg(0),
            &arg(1),
            expanded_args.get(2).map(String::as_str),
        )),
        "list_size" | "array_size" | "map_size" | "dict_size" | "json_size" => {
            Some(preprocessor_size(&arg(0)).to_string())
        }
        "list_is_empty" | "array_is_empty" | "empty" => {
            Some((preprocessor_size(&arg(0)) == 0).to_string())
        }
        "list_clear" | "array_clear" => Some("[]".to_string()),
        "is_empty" => {
            Some((arg(0).trim().is_empty() || preprocessor_size(&arg(0)) == 0).to_string())
        }
        "is_number" | "is_int" | "is_integer" => Some(
            super::includes::eval_int_expr(arg(0).trim())
                .is_some()
                .to_string(),
        ),
        "list_contains"
        | "array_contains"
        | "contains_list"
        | "list_contains_value"
        | "array_contains_value" => Some(
            preprocessor_list_items(&arg(0))
                .contains(&arg(1))
                .to_string(),
        ),
        "list_indexof" | "array_indexof" | "indexof" => Some(
            preprocessor_list_items(&arg(0))
                .iter()
                .position(|item| item == &arg(1))
                .map(|idx| idx.to_string())
                .unwrap_or_else(|| "-1".to_string()),
        ),
        "list_sort" | "array_sort" => {
            let mut items = preprocessor_list_items(&arg(0));
            items.sort();
            Some(preprocessor_list_literal(&items))
        }
        "list_reverse" | "array_reverse" => {
            let mut items = preprocessor_list_items(&arg(0));
            items.reverse();
            Some(preprocessor_list_literal(&items))
        }
        "list_get" | "array_get" | "list_at" | "array_at" => {
            let fallback = if argc >= 3 { arg(2) } else { String::new() };
            Some(
                preprocessor_list_items(&arg(0))
                    .get(parse_int_lenient(&arg(1)).max(0) as usize)
                    .cloned()
                    .unwrap_or(fallback),
            )
        }
        "list_slice" | "array_slice" | "list_sublist" | "array_sublist" | "sublist" => Some(
            preprocessor_list_slice(&arg(0), &arg(1), expanded_args.get(2).map(String::as_str)),
        ),
        "first" | "list_first" | "array_first" => Some(
            preprocessor_list_items(&arg(0))
                .first()
                .cloned()
                .unwrap_or_default(),
        ),
        "last" | "list_last" | "array_last" => Some(
            preprocessor_list_items(&arg(0))
                .last()
                .cloned()
                .unwrap_or_default(),
        ),
        "list_add" | "array_add" | "list_append" | "array_append" | "list_push" | "array_push" => {
            let mut items = preprocessor_list_items(&arg(0));
            items.push(arg(1));
            Some(preprocessor_list_literal(&items))
        }
        "list_set" | "array_set" => {
            let mut items = preprocessor_list_items(&arg(0));
            let idx = parse_int_lenient(&arg(1)).max(0) as usize;
            if idx < items.len() {
                items[idx] = arg(2);
            } else {
                while items.len() < idx {
                    items.push(String::new());
                }
                items.push(arg(2));
            }
            Some(preprocessor_list_literal(&items))
        }
        "list_insert" | "array_insert" => {
            let mut items = preprocessor_list_items(&arg(0));
            let idx = parse_int_lenient(&arg(1)).max(0) as usize;
            let idx = idx.min(items.len());
            items.insert(idx, arg(2));
            Some(preprocessor_list_literal(&items))
        }
        "list_remove" | "array_remove" => {
            let key = arg(1);
            let mut items = preprocessor_list_items(&arg(0));
            if let Ok(idx) = key.trim().parse::<usize>() {
                if idx < items.len() {
                    items.remove(idx);
                }
            } else {
                items.retain(|item| item != &key);
            }
            Some(preprocessor_list_literal(&items))
        }
        "list_pop" | "array_pop" => {
            let mut items = preprocessor_list_items(&arg(0));
            let _ = items.pop();
            Some(preprocessor_list_literal(&items))
        }
        "list_shift" | "array_shift" => {
            let mut items = preprocessor_list_items(&arg(0));
            if !items.is_empty() {
                items.remove(0);
            }
            Some(preprocessor_list_literal(&items))
        }
        "map" | "dict" | "newmap" => Some(preprocessor_map_literal(&expanded_args)),
        "map_clear" | "dict_clear" => Some("{}".to_string()),
        "map_is_empty" | "dict_is_empty" => Some((preprocessor_size(&arg(0)) == 0).to_string()),
        "map_merge" | "dict_merge" | "json_merge" => {
            Some(preprocessor_json_merge(&arg(0), &arg(1)))
        }
        "map_entries" | "dict_entries" | "entries" => Some(preprocessor_map_entries(&arg(0))),
        "map_contains_key" | "dict_contains_key" | "contains_key" | "has_key" | "map_has_key"
        | "dict_has_key" | "json_contains_key" | "json_has_key" | "map_includes_key"
        | "dict_includes_key" => Some(json_contains_key(&arg(0), &arg(1)).to_string()),
        "map_contains_value"
        | "dict_contains_value"
        | "contains_value"
        | "has_value"
        | "json_contains_value"
        | "map_includes_value"
        | "dict_includes_value" => Some(json_contains_value(&arg(0), &arg(1)).to_string()),
        "get" | "map_get" | "dict_get" | "json_get" => {
            let fallback = if argc >= 3 { arg(2) } else { String::new() };
            Some(preprocessor_get_opt(&arg(0), &arg(1)).unwrap_or(fallback))
        }
        "set" | "put" | "json_set" | "map_put" | "map_set" | "dict_put" | "dict_set" => {
            Some(preprocessor_set(&arg(0), &arg(1), &arg(2)))
        }
        "remove" | "map_remove" | "map_delete" | "dict_remove" | "dict_delete" | "json_remove"
        | "json_delete" => Some(preprocessor_remove(&arg(0), &arg(1))),
        "keys" | "map_keys" | "dict_keys" => Some(preprocessor_json_keys(&arg(0)).join(",")),
        "values" | "map_values" | "dict_values" => {
            Some(preprocessor_json_values(&arg(0)).join(","))
        }
        "json_type" | "get_json_type" => Some(preprocessor_json_type(&arg(0))),
        "json_is_valid" | "is_json" | "is_object" | "is_map" => Some(
            serde_json::from_str::<serde_json::Value>(arg(0).trim())
                .is_ok()
                .to_string(),
        ),
        "is_list" | "is_array" => Some(
            serde_json::from_str::<serde_json::Value>(arg(0).trim())
                .ok()
                .and_then(|value| value.as_array().map(|_| true))
                .unwrap_or(false)
                .to_string(),
        ),
        "str2json" => Some(preprocessor_str2json(&arg(0))),
        "json_add" => Some(preprocessor_set(&arg(0), &arg(1), &arg(2))),
        "strpos" => {
            let s = arg(0);
            let sub = arg(1);
            Some(match s.find(sub.as_str()) {
                Some(byte_idx) => {
                    // Return char index (PlantUML semantics).
                    let char_idx = s[..byte_idx].chars().count();
                    char_idx.to_string()
                }
                None => "-1".to_string(),
            })
        }
        "substr" => {
            let s = arg(0);
            let start = parse_int_lenient(&arg(1)).max(0) as usize;
            let chars: Vec<char> = s.chars().collect();
            let start = start.min(chars.len());
            let end = if argc >= 3 {
                let len = parse_int_lenient(&arg(2));
                if len < 0 {
                    chars.len()
                } else {
                    (start + len as usize).min(chars.len())
                }
            } else {
                chars.len()
            };
            Some(chars[start..end].iter().collect())
        }
        "intval" => Some(parse_int_lenient(&arg(0)).to_string()),
        "str" | "string" | "stringify" | "json_stringify" => Some(arg(0)),
        "quote" => Some(format!("\"{}\"", arg(0).replace('"', "\\\""))),
        "unquote" => Some(strip_quotes(&arg(0))),
        "trim" => Some(arg(0).trim().to_string()),
        "ltrim" => Some(arg(0).trim_start().to_string()),
        "rtrim" => Some(arg(0).trim_end().to_string()),
        "replace" => Some(arg(0).replace(&arg(1), &arg(2))),
        "equals" | "eq" | "strcmp" => Some((arg(0) == arg(1)).to_string()),
        "equals_ignore_case" | "eq_ignore_case" | "strcmp_ignore_case" => {
            Some(arg(0).eq_ignore_ascii_case(&arg(1)).to_string())
        }
        "startswith" | "starts_with" => Some(arg(0).starts_with(&arg(1)).to_string()),
        "startswith_ignore_case" | "starts_with_ignore_case" => Some(
            arg(0)
                .to_ascii_lowercase()
                .starts_with(&arg(1).to_ascii_lowercase())
                .to_string(),
        ),
        "endswith" | "ends_with" => Some(arg(0).ends_with(&arg(1)).to_string()),
        "endswith_ignore_case" | "ends_with_ignore_case" => Some(
            arg(0)
                .to_ascii_lowercase()
                .ends_with(&arg(1).to_ascii_lowercase())
                .to_string(),
        ),
        "contains" => Some(arg(0).contains(&arg(1)).to_string()),
        "contains_ignore_case" => Some(
            arg(0)
                .to_ascii_lowercase()
                .contains(&arg(1).to_ascii_lowercase())
                .to_string(),
        ),
        "boolval" => Some(boolval(&arg(0)).to_string()),
        "true" => Some("true".to_string()),
        "false" => Some("false".to_string()),
        "not" => Some((!boolval(&arg(0))).to_string()),
        "lower" => Some(arg(0).to_lowercase()),
        "upper" => Some(arg(0).to_uppercase()),
        "chr" => {
            let n = parse_int_lenient(&arg(0));
            if n < 0 {
                Some(String::new())
            } else if let Some(c) = u32::try_from(n).ok().and_then(char::from_u32) {
                Some(c.to_string())
            } else {
                Some(String::new())
            }
        }
        "dec2hex" => {
            let n = parse_int_lenient(&arg(0));
            if n < 0 {
                Some(String::new())
            } else {
                Some(format!("{:x}", n))
            }
        }
        "hex2dec" => {
            let s = arg(0);
            let cleaned = s.trim().trim_start_matches("0x").trim_start_matches("0X");
            Some(
                i64::from_str_radix(cleaned, 16)
                    .map(|n| n.to_string())
                    .unwrap_or_else(|_| "0".to_string()),
            )
        }
        "ord" => Some(
            arg(0)
                .chars()
                .next()
                .map(|c| (c as u32).to_string())
                .unwrap_or_else(|| "0".to_string()),
        ),
        // Time-/env-sensitive builtins: empty for determinism.
        "date" | "time" | "now" | "timestamp" | "getenv" | "env" | "getenv_default" => {
            Some(String::new())
        }
        // Random-sensitive builtins: fixed deterministic value.
        "random" | "rand" | "random_int" | "random_number" => Some("0".to_string()),
        "uuid" | "random_uuid" => Some("00000000-0000-0000-0000-000000000000".to_string()),
        // Local/remote IO helpers are disabled rather than implicitly reading
        // host state during preprocessing.
        "load_file"
        | "load_text"
        | "load_string"
        | "load_data"
        | "load_bytes"
        | "load_json"
        | "load_yaml"
        | "load_csv"
        | "load_sprite"
        | "load_sprites"
        | "read_file"
        | "read_text"
        | "file_exists"
        | "exists"
        | "include_file_exists" => {
            return Err(Diagnostic::error_code(
                "E_PREPROC_UNSAFE_BUILTIN",
                format!(
                    "preprocessor builtin `%{}(...)` is disabled for deterministic offline execution",
                    name
                ),
            ));
        }
        "dirpath" => {
            let p = arg(0);
            Some(
                std::path::Path::new(&p)
                    .parent()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            )
        }
        "filename" => {
            let p = arg(0);
            Some(
                std::path::Path::new(&p)
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            )
        }
        "filenameroot" => {
            let p = arg(0);
            Some(
                std::path::Path::new(&p)
                    .file_stem()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            )
        }
        // %feature — always "false" for unknown features; deterministic and safe
        "feature" => Some("false".to_string()),
        // %get_variable_value — fully resolved string value of a variable
        "get_variable_value" => {
            let key = arg(0);
            Some(state.vars.get(&key).cloned().unwrap_or_default())
        }
        // %variable_exists — true if the variable is defined in state
        "variable_exists" => {
            let key = arg(0);
            Some(state.vars.contains_key(&key).to_string())
        }
        // %function_exists — true if a function callable is registered
        "function_exists" => {
            let key = arg(0);
            Some(
                state
                    .callables
                    .get(&key)
                    .map(|c| c.kind == PreprocCallableKind::Function)
                    .unwrap_or(false)
                    .to_string(),
            )
        }
        "procedure_exists" => {
            let key = arg(0);
            Some(
                state
                    .callables
                    .get(&key)
                    .map(|c| c.kind == PreprocCallableKind::Procedure)
                    .unwrap_or(false)
                    .to_string(),
            )
        }
        "get_all_stdlib" => Some(
            crate::stdlib::local_stdlib_inventory(None)
                .map(|entries| crate::stdlib::stdlib_paths_json(&entries))
                .unwrap_or_else(|_| "[]".to_string()),
        ),
        // %newline — literal newline character (PlantUML parity)
        "newline" => Some("\n".to_string()),
        // %retrieve_procedure_return — last procedure return value (stateless in our
        // deterministic model; procedures cannot return values so always empty)
        "retrieve_procedure_return" => Some(String::new()),
        "set_variable_value" => {
            // Read-only in our model; document by returning empty.
            Some(String::new())
        }
        "get_json_attribute" => {
            let json = arg(0);
            let key = arg(1);
            Some(get_json_attribute(&json, &key))
        }
        "json_key_exists" => {
            let json = arg(0);
            let key = arg(1);
            Some(json_contains_key(&json, &key).to_string())
        }
        "json_keys" => Some(preprocessor_json_keys(&arg(0)).join(",")),
        "json_values" => Some(preprocessor_json_values(&arg(0)).join(",")),
        "false_then_true" => {
            let key = arg(0);
            let mut counts = state.false_then_true_counts.borrow_mut();
            let entry = counts.entry(key).or_insert(0);
            let result = if *entry == 0 { "false" } else { "true" };
            *entry = entry.saturating_add(1);
            Some(result.to_string())
        }
        "true_then_false" => {
            let key = arg(0);
            let mut counts = state.true_then_false_counts.borrow_mut();
            let entry = counts.entry(key).or_insert(0);
            let result = if *entry == 0 { "true" } else { "false" };
            *entry = entry.saturating_add(1);
            Some(result.to_string())
        }
        "invoke_procedure" | "call_user_func" => {
            if expanded_args.is_empty() {
                return Err(Diagnostic::error_code(
                    "E_PREPROC_DYNAMIC_UNSUPPORTED",
                    format!(
                        "dynamic preprocessor invocation `%{}(...)` requires a callable name argument",
                        name
                    ),
                ));
            }
            let callable_name = strip_quotes(&expanded_args[0]);
            if callable_name.is_empty() {
                return Err(Diagnostic::error_code(
                    "E_PREPROC_DYNAMIC_UNSUPPORTED",
                    format!(
                        "dynamic preprocessor invocation `%{}(...)` requires a non-empty callable name",
                        name
                    ),
                ));
            }
            let callable = state.callables.get(&callable_name).ok_or_else(|| {
                Diagnostic::error_code(
                    "E_PREPROC_CALL_UNKNOWN",
                    format!("unknown callable `{callable_name}`"),
                )
            })?;
            if callable.kind != PreprocCallableKind::Function {
                return Err(Diagnostic::error_code(
                    "E_PREPROC_DYNAMIC_UNSUPPORTED",
                    format!(
                        "dynamic preprocessor invocation `%{}(...)` only supports functions in expression context",
                        name
                    ),
                ));
            }
            let tail = split_args(args_raw)?
                .into_iter()
                .skip(1)
                .collect::<Vec<_>>()
                .join(", ");
            Some(execute_function_call(
                &callable_name,
                &tail,
                state,
                call_depth + 1,
            )?)
        }
        "abs" => Some(parse_int_lenient(&arg(0)).abs().to_string()),
        "min" => Some(
            expanded_args
                .iter()
                .map(|value| parse_int_lenient(value))
                .min()
                .unwrap_or(0)
                .to_string(),
        ),
        "max" => Some(
            expanded_args
                .iter()
                .map(|value| parse_int_lenient(value))
                .max()
                .unwrap_or(0)
                .to_string(),
        ),
        "is_dark" => Some(is_dark_color(&arg(0)).to_string()),
        "reverse_color" => Some(
            parse_hex_rgb(&arg(0))
                .map(|(r, g, b)| format!("#{:02x}{:02x}{:02x}", 255 - r, 255 - g, 255 - b))
                .unwrap_or_default(),
        ),
        "lighten" => Some(adjust_color(&arg(0), parse_int_lenient(&arg(1)), true)),
        "darken" => Some(adjust_color(&arg(0), parse_int_lenient(&arg(1)), false)),
        _ => None,
    };
    Ok(result)
}

/// Strip a single layer of matching double quotes from a value.
pub(super) fn strip_quotes(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

pub(super) fn parse_int_lenient(s: &str) -> i64 {
    let t = s.trim();
    if t.is_empty() {
        return 0;
    }
    if let Ok(n) = t.parse::<i64>() {
        return n;
    }
    // PlantUML's `%intval` is lenient: extract the longest leading numeric
    // prefix (optionally signed) and fall back to 0 when nothing parses.
    let bytes = t.as_bytes();
    let mut end = 0usize;
    if !bytes.is_empty() && (bytes[0] == b'-' || bytes[0] == b'+') {
        end += 1;
    }
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end == 0 || (end == 1 && (bytes[0] == b'-' || bytes[0] == b'+')) {
        return 0;
    }
    t[..end].parse::<i64>().unwrap_or(0)
}

fn parse_hex_rgb(raw: &str) -> Option<(u8, u8, u8)> {
    let mut s = raw.trim();
    if let Some(rest) = s.strip_prefix('#') {
        s = rest;
    }
    if s.len() == 3 {
        let mut expanded = String::with_capacity(6);
        for ch in s.chars() {
            expanded.push(ch);
            expanded.push(ch);
        }
        return parse_hex_rgb(&expanded);
    }
    if s.len() != 6 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some((r, g, b))
}

fn is_dark_color(raw: &str) -> bool {
    let Some((r, g, b)) = parse_hex_rgb(raw) else {
        return false;
    };
    let luminance = (u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114) / 1000;
    luminance < 128
}

fn adjust_color(raw: &str, pct: i64, lighten: bool) -> String {
    let Some((r, g, b)) = parse_hex_rgb(raw) else {
        return String::new();
    };
    let pct = pct.clamp(0, 100) as i32;
    let adjust = |v: u8| -> u8 {
        let v = i32::from(v);
        let next = if lighten {
            v + ((255 - v) * pct / 100)
        } else {
            v - (v * pct / 100)
        };
        next.clamp(0, 255) as u8
    };
    format!("#{:02x}{:02x}{:02x}", adjust(r), adjust(g), adjust(b))
}

/// PlantUML-ish truthiness for `%boolval`/`%not`.
fn boolval(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    !matches!(lower.as_str(), "0" | "false" | "no" | "off")
}

/// JSON key lookup supporting simple dot-path and array-index access so
/// `%get_json_attribute` can serve patterns like:
///   `%get_json_attribute($cfg, "name")`           — top-level string key
///   `%get_json_attribute($cfg, "users[0].name")`  — nested path
///
/// Returns the value as a string (quotes stripped for string values; numeric /
/// boolean / null left verbatim). Returns an empty string when the input is
/// not valid JSON, the path is missing, or the value is a nested
/// object/array (callers may then pass sub-JSON to a further call).
pub(super) fn get_json_attribute(json: &str, key: &str) -> String {
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

fn json_contains_key(json: &str, key: &str) -> bool {
    if let Ok(root) = serde_json::from_str::<serde_json::Value>(json.trim()) {
        return json_value_at_path(&root, key).is_some();
    }

    // Reuse the top-level key scan rather than the full path traversal so that
    // an empty-value key still reports as present (PlantUML semantics).
    !get_json_top_level_key(json, key).is_empty()
}

fn json_contains_value(json: &str, needle: &str) -> bool {
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

pub(super) fn preprocessor_list_items(raw: &str) -> Vec<String> {
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

fn preprocessor_list_slice(raw: &str, start: &str, len: Option<&str>) -> String {
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

pub(super) fn preprocessor_foreach_bindings(
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

fn split_preprocessor_regex(s: &str, pattern: &str) -> Vec<String> {
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

fn preprocessor_range(start: &str, end: &str, step: Option<&str>) -> String {
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

fn preprocessor_map_literal(args: &[String]) -> String {
    let mut obj = serde_json::Map::new();
    for chunk in args.chunks(2) {
        if let [key, value] = chunk {
            obj.insert(key.clone(), json_value_from_preproc(value));
        }
    }
    serde_json::Value::Object(obj).to_string()
}

fn preprocessor_map_entries(json: &str) -> String {
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

fn preprocessor_str2json(raw: &str) -> String {
    serde_json::from_str::<serde_json::Value>(raw.trim())
        .unwrap_or_else(|_| serde_json::Value::String(raw.to_string()))
        .to_string()
}

fn preprocessor_get_opt(container: &str, key: &str) -> Option<String> {
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

fn preprocessor_set(container: &str, key: &str, replacement: &str) -> String {
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

fn preprocessor_remove(container: &str, key: &str) -> String {
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

fn preprocessor_json_merge(lhs: &str, rhs: &str) -> String {
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

fn preprocessor_json_type(raw: &str) -> String {
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

fn preprocessor_json_keys(json: &str) -> Vec<String> {
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

fn preprocessor_json_values(json: &str) -> Vec<String> {
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

pub(super) fn extract_parenthesized_args(
    chars: &[char],
    open_idx: usize,
) -> Result<(String, usize), Diagnostic> {
    let mut depth = 0usize;
    let mut i = open_idx;
    let mut in_quotes = false;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '"' {
            in_quotes = !in_quotes;
        } else if !in_quotes {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    let args: String = chars[open_idx + 1..i].iter().collect();
                    return Ok((args, i + 1));
                }
            }
        }
        i += 1;
    }
    Err(Diagnostic::error_code(
        "E_PREPROC_CALL_SYNTAX",
        "malformed preprocessor call: missing closing `)`",
    ))
}

pub(super) fn parse_callable_definition(
    header: &str,
    body: &[&str],
    kind: PreprocCallableKind,
) -> Result<(String, PreprocCallable), Diagnostic> {
    let sig = header
        .trim_start_matches('!')
        .split_once(char::is_whitespace)
        .map(|(_, r)| r.trim())
        .unwrap_or_default();
    let open = sig.find('(').ok_or_else(|| {
        Diagnostic::error_code(
            "E_PREPROC_SIGNATURE",
            "callable signature requires `(…)` parameter list",
        )
    })?;
    let close = sig.rfind(')').ok_or_else(|| {
        Diagnostic::error_code(
            "E_PREPROC_SIGNATURE",
            "callable signature requires closing `)`",
        )
    })?;
    if close < open {
        return Err(Diagnostic::error_code(
            "E_PREPROC_SIGNATURE",
            "invalid callable signature",
        ));
    }
    let name = sig[..open].trim().to_string();
    if name.is_empty() {
        return Err(Diagnostic::error_code(
            "E_PREPROC_SIGNATURE",
            "callable name is required",
        ));
    }
    let params_raw = &sig[open + 1..close];
    let params = parse_params(params_raw)?;
    let callable = PreprocCallable {
        kind,
        params,
        body: body.iter().map(|s| (*s).to_string()).collect(),
    };
    Ok((name, callable))
}

pub(super) fn parse_params(raw: &str) -> Result<Vec<PreprocParam>, Diagnostic> {
    let mut params = Vec::new();
    let normalized = raw.replace("##", ",");
    for piece in split_args(&normalized)? {
        let trimmed = piece.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (name_part, default) = if let Some((n, d)) = trimmed.split_once('=') {
            (n.trim(), Some(d.trim().to_string()))
        } else {
            (trimmed, None)
        };
        let name = name_part.trim_start_matches('$').trim().to_string();
        if name.is_empty() {
            return Err(Diagnostic::error_code(
                "E_PREPROC_SIGNATURE",
                "parameter name cannot be empty",
            ));
        }
        params.push(PreprocParam { name, default });
    }
    Ok(params)
}

pub(super) fn split_args(raw: &str) -> Result<Vec<String>, Diagnostic> {
    let mut out = Vec::new();
    let mut curr = String::new();
    let mut paren_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut in_quotes = false;
    for ch in raw.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            curr.push(ch);
            continue;
        }
        if !in_quotes {
            match ch {
                '(' => paren_depth += 1,
                ')' => {
                    if paren_depth == 0 {
                        return Err(Diagnostic::error_code(
                            "E_PREPROC_CALL_SYNTAX",
                            "unbalanced `)` in argument list",
                        ));
                    }
                    paren_depth -= 1;
                }
                '{' => brace_depth += 1,
                '}' => brace_depth = brace_depth.saturating_sub(1),
                '[' => bracket_depth += 1,
                ']' => bracket_depth = bracket_depth.saturating_sub(1),
                ',' if paren_depth == 0 && brace_depth == 0 && bracket_depth == 0 => {
                    out.push(curr.trim().to_string());
                    curr.clear();
                    continue;
                }
                _ => {}
            }
        }
        curr.push(ch);
    }
    if in_quotes || paren_depth != 0 || brace_depth != 0 || bracket_depth != 0 {
        return Err(Diagnostic::error_code(
            "E_PREPROC_CALL_SYNTAX",
            "malformed argument list",
        ));
    }
    if !curr.trim().is_empty() {
        out.push(curr.trim().to_string());
    }
    Ok(out)
}

pub(super) fn bind_callable_args(
    callable: &PreprocCallable,
    args_raw: &str,
    state: &PreprocState,
    call_depth: usize,
) -> Result<BTreeMap<String, String>, Diagnostic> {
    let args_normalized = args_raw.replace("##", ",");
    let mut bound = BTreeMap::new();
    let mut positional = Vec::new();
    let mut keyword = BTreeMap::new();
    for arg in split_args(&args_normalized)? {
        if let Some((k, v)) = arg.split_once('=') {
            keyword.insert(
                k.trim().trim_start_matches('$').to_string(),
                expand_preprocessor_text(v.trim(), state, call_depth)?,
            );
        } else if !arg.trim().is_empty() {
            positional.push(expand_preprocessor_text(arg.trim(), state, call_depth)?);
        }
    }

    let mut pos_idx = 0usize;
    for param in &callable.params {
        if let Some(v) = keyword.remove(&param.name) {
            bound.insert(param.name.clone(), v);
            continue;
        }
        if pos_idx < positional.len() {
            bound.insert(param.name.clone(), positional[pos_idx].clone());
            pos_idx += 1;
            continue;
        }
        if let Some(default) = &param.default {
            bound.insert(
                param.name.clone(),
                expand_preprocessor_text(default, state, call_depth)?,
            );
            continue;
        }
        return Err(Diagnostic::error_code(
            "E_PREPROC_ARG_REQUIRED",
            format!("missing required argument `{}`", param.name),
        ));
    }
    if pos_idx < positional.len() || !keyword.is_empty() {
        return Err(Diagnostic::error_code(
            "E_PREPROC_ARG_MISMATCH",
            "argument list does not match callable signature",
        ));
    }
    Ok(bound)
}

pub(super) fn execute_function_call(
    name: &str,
    args_raw: &str,
    state: &PreprocState,
    call_depth: usize,
) -> Result<String, Diagnostic> {
    let callable = state.callables.get(name).ok_or_else(|| {
        Diagnostic::error_code(
            "E_PREPROC_CALL_UNKNOWN",
            format!("unknown callable `{name}`"),
        )
    })?;
    if callable.kind != PreprocCallableKind::Function {
        return Err(Diagnostic::error_code(
            "E_PREPROC_CALL_KIND",
            format!("`{name}` is not a function"),
        ));
    }
    let bindings = bind_callable_args(callable, args_raw, state, call_depth)?;
    let mut local_state = state.clone();
    local_state.global_assigns.borrow_mut().clear();
    for (k, v) in &bindings {
        local_state.vars.insert(k.clone(), v.clone());
    }
    let mut local_out = String::new();
    for raw in &callable.body {
        let line = raw.trim();
        if !line.to_ascii_lowercase().starts_with("!return") {
            preprocess_text(
                raw,
                &ParseOptions::default(),
                &mut local_state,
                &mut Vec::new(),
                &mut BTreeSet::new(),
                0,
                call_depth + 1,
                &mut local_out,
            )?;
            continue;
        }
        let trimmed_return = raw.trim_start();
        let expr = trimmed_return
            .trim_start_matches("!return")
            .trim_start()
            .to_string();
        return expand_preprocessor_text(&expr, &local_state, call_depth + 1);
    }
    Err(Diagnostic::error_code(
        "E_PREPROC_RETURN_REQUIRED",
        format!("function `{name}` must contain `!return`"),
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_procedure_call(
    name: &str,
    args_raw: &str,
    state: &mut PreprocState,
    options: &ParseOptions,
    include_stack: &mut Vec<PathBuf>,
    include_once_seen: &mut BTreeSet<PathBuf>,
    depth: usize,
    call_depth: usize,
    out: &mut String,
) -> Result<(), Diagnostic> {
    if call_depth > MAX_PREPROC_CALL_DEPTH {
        return Err(Diagnostic::error_code(
            "E_PREPROC_CALL_DEPTH",
            format!("preprocessor call depth exceeded maximum of {MAX_PREPROC_CALL_DEPTH}"),
        ));
    }
    let callable = state.callables.get(name).cloned().ok_or_else(|| {
        Diagnostic::error_code(
            "E_PREPROC_CALL_UNKNOWN",
            format!("unknown callable `{name}`"),
        )
    })?;
    if callable.kind != PreprocCallableKind::Procedure {
        return Err(Diagnostic::error_code(
            "E_PREPROC_CALL_KIND",
            format!("`{name}` is not a procedure"),
        ));
    }
    let bindings = bind_callable_args(&callable, args_raw, state, call_depth)?;
    if callable
        .body
        .iter()
        .any(|raw| raw.trim().to_ascii_lowercase().starts_with("!return"))
    {
        return Err(Diagnostic::error_code(
            "E_PREPROC_RETURN_UNEXPECTED",
            format!("procedure `{name}` cannot contain `!return`"),
        ));
    }
    let mut local_state = state.clone();
    for (k, v) in &bindings {
        local_state.vars.insert(k.clone(), v.clone());
    }
    let local = callable.body.join("\n");
    if !local.trim().is_empty() {
        preprocess_text(
            &local,
            options,
            &mut local_state,
            include_stack,
            include_once_seen,
            depth,
            call_depth + 1,
            out,
        )?;
        if local_state.loop_signal.is_some() {
            state.loop_signal = local_state.loop_signal.take();
        }
        let globals = local_state.global_assigns.borrow().clone();
        for name in globals {
            if let Some(value) = local_state.vars.get(&name) {
                state.vars.insert(name.clone(), value.clone());
            } else {
                state.vars.remove(&name);
            }
            state.global_assigns.borrow_mut().insert(name);
        }
        Ok(())
    } else {
        Ok(())
    }
}

/// Execute a dynamic `%invoke_procedure("name"[, args...])` line-level
/// invocation. The procedure name must resolve at expand time to a previously
/// declared `!procedure` (we explicitly do not support free-form code paths).
#[allow(clippy::too_many_arguments)]
pub(super) fn invoke_dynamic_procedure(
    raw: &str,
    state: &mut PreprocState,
    options: &ParseOptions,
    include_stack: &mut Vec<PathBuf>,
    include_once_seen: &mut BTreeSet<PathBuf>,
    depth: usize,
    call_depth: usize,
    out: &mut String,
) -> Result<(), Diagnostic> {
    let trimmed = raw.trim();
    let lower = trimmed.to_ascii_lowercase();
    let prefix = if lower.starts_with("%invoke_procedure(") {
        "%invoke_procedure("
    } else if lower.starts_with("%call_user_func(") {
        "%call_user_func("
    } else {
        return Err(Diagnostic::error_code(
            "E_PREPROC_DYNAMIC_UNSUPPORTED",
            format!("dynamic preprocessor invocation `{raw}` is malformed"),
        ));
    };
    let body = &trimmed[prefix.len()..];
    let body = body.strip_suffix(')').ok_or_else(|| {
        Diagnostic::error_code(
            "E_PREPROC_CALL_SYNTAX",
            format!("malformed dynamic procedure invocation `{raw}`"),
        )
    })?;
    let parts = split_args(body)?;
    let mut iter = parts.into_iter();
    let name_raw = iter.next().ok_or_else(|| {
        Diagnostic::error_code(
            "E_PREPROC_DYNAMIC_UNSUPPORTED",
            "%invoke_procedure requires a procedure name argument",
        )
    })?;
    let name_resolved = expand_preprocessor_text(&name_raw, state, call_depth)?;
    let name = strip_quotes(&name_resolved);
    if name.is_empty() {
        return Err(Diagnostic::error_code(
            "E_PREPROC_DYNAMIC_UNSUPPORTED",
            "%invoke_procedure requires a non-empty procedure name",
        ));
    }
    let remaining: Vec<String> = iter.collect();
    let args_raw = remaining.join(", ");
    execute_procedure_call(
        &name,
        &args_raw,
        state,
        options,
        include_stack,
        include_once_seen,
        depth,
        call_depth + 1,
        out,
    )
}
