use crate::diagnostic::Diagnostic;

use super::{
    boolval, deterministic_preproc_now_seconds, dispatch_color_builtin, execute_function_call,
    format_preprocessor_date, format_preprocessor_time, get_json_attribute, json_contains_key,
    json_contains_value, parse_int_lenient, preprocessor_get_opt, preprocessor_json_keys,
    preprocessor_json_merge, preprocessor_json_type, preprocessor_json_values,
    preprocessor_list_items, preprocessor_list_literal, preprocessor_list_slice,
    preprocessor_map_entries, preprocessor_map_literal, preprocessor_range, preprocessor_remove,
    preprocessor_set, preprocessor_size, preprocessor_str2json, split_args,
    split_preprocessor_regex, strip_quotes,
};
use crate::preproc::includes;
use crate::preproc::macros::expand_preprocessor_text;
use crate::preproc::{PreprocCallableKind, PreprocState};

/// Dispatch a known preprocessor builtin. Returns `Ok(Some(result))` if the
/// name maps to a builtin, `Ok(None)` if the name is not recognised so the
/// caller can fall through to its unknown-function diagnostic.
///
/// Time-sensitive builtins (`%date`, `%now`) use a deterministic clock instead
/// of the host wall-clock. By default the clock is Unix epoch second 0; callers
/// may inject `PUML_NOW=<epoch-seconds>` with `-D`/`ParseOptions::inject_vars`
/// to make date-driven diagrams reproducible while still useful. Host
/// environment access (`%getenv`) remains empty for byte-identical output.
pub(in crate::preproc) fn dispatch_builtin(
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
            includes::eval_int_expr(&arg(0))
                .map(|n| n.to_string())
                .unwrap_or_else(|| arg(0)),
        ),
        "eval_bool" | "eval_boolean" => Some(includes::evaluate_scalar_expr(&arg(0))?.to_string()),
        "if" | "ternary" | "iif" => Some(if includes::evaluate_scalar_expr(&arg(0))? {
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
        "is_number" | "is_int" | "is_integer" => {
            Some(includes::eval_int_expr(arg(0).trim()).is_some().to_string())
        }
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
        "now" | "timestamp" => Some(deterministic_preproc_now_seconds(state).to_string()),
        "date" => Some(format_preprocessor_date(
            expanded_args.first().map(String::as_str),
            expanded_args.get(1).map(String::as_str),
            state,
        )),
        "time" => Some(format_preprocessor_time(
            expanded_args.first().map(String::as_str),
            state,
        )),
        // Environment-sensitive builtins: empty for determinism.
        "getenv" | "env" | "getenv_default" => Some(String::new()),
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
        // %mod(a, b) — integer modulo.  PlantUML semantics: remainder after
        // Euclidean division (result sign matches the divisor, same as Java
        // `Math.floorMod`).  When `b` is zero we return 0 rather than panic.
        "mod" => {
            let a = parse_int_lenient(&arg(0));
            let b = parse_int_lenient(&arg(1));
            Some(if b == 0 { 0 } else { a.rem_euclid(b) }.to_string())
        }
        // Color builtins: is_dark, is_light, reverse_color, reverse_hsluv_color,
        // lighten, darken, hsl_color — delegated to color.rs dispatcher.
        name if dispatch_color_builtin(name, &arg(0), &arg(1), &arg(2), &arg(3), argc)
            .is_some() =>
        {
            dispatch_color_builtin(name, &arg(0), &arg(1), &arg(2), &arg(3), argc)
        }
        // %version — deterministic version string matching the PUML crate version.
        "version" => Some(env!("CARGO_PKG_VERSION").to_string()),
        _ => None,
    };
    Ok(result)
}
