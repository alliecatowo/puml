use std::collections::BTreeMap;

use crate::diagnostic::Diagnostic;

use super::builtins::{
    dispatch_builtin, execute_function_call, extract_parenthesized_args, split_args,
};
use super::{
    PreprocCallableKind, PreprocMacro, PreprocState, PreprocVariableScope, PreprocessDirective,
    MAX_PREPROC_CALL_DEPTH, MAX_PREPROC_MACRO_EXPANSION_BYTES,
};

pub(super) fn parse_macro_define(body: &str) -> Result<Option<(String, PreprocMacro)>, Diagnostic> {
    let trimmed = body.trim();
    let Some(open) = trimmed.find('(') else {
        return Ok(None);
    };
    let name = trimmed[..open].trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Ok(None);
    }
    let chars = trimmed.chars().collect::<Vec<_>>();
    let (params_raw, close_next) = extract_parenthesized_args(&chars, open)?;
    let rest = trimmed[close_next..].trim();
    let params = super::builtins::parse_params(&params_raw)?;
    Ok(Some((
        name.to_string(),
        PreprocMacro {
            params,
            body: rest.to_string(),
        },
    )))
}

pub(super) fn substitute_defines(
    line: &str,
    defines: &BTreeMap<String, String>,
    macros: &BTreeMap<String, PreprocMacro>,
) -> Result<String, Diagnostic> {
    let mut out = String::with_capacity(line.len());
    let chars = line.chars().collect::<Vec<_>>();
    let mut i = 0usize;
    let mut in_quotes = false;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '"' {
            in_quotes = !in_quotes;
            out.push(ch);
            i += 1;
            continue;
        }
        if !in_quotes && (ch.is_ascii_alphabetic() || ch == '_') {
            let start = i;
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let token = chars[start..j].iter().collect::<String>();
            let mut k = j;
            while k < chars.len() && chars[k].is_whitespace() {
                k += 1;
            }
            if k < chars.len() && chars[k] == '(' {
                if let Some(mac) = macros.get(&token) {
                    let (args_raw, next_idx) = extract_parenthesized_args(&chars, k)?;
                    let args = split_args(&args_raw)?;
                    out.push_str(&expand_macro_body(mac, &args));
                    i = next_idx;
                    continue;
                }
            }
            if let Some(value) = defines.get(token.as_str()) {
                out.push_str(value);
            } else {
                out.push_str(&token);
            }
            i = j;
            continue;
        }
        out.push(ch);
        i += 1;
    }
    Ok(out)
}

pub(super) fn expand_macro_body(mac: &PreprocMacro, args: &[String]) -> String {
    let mut positional = Vec::new();
    let mut keyword = BTreeMap::new();
    for arg in args {
        if let Some((key, value)) = arg.split_once('=') {
            keyword.insert(
                key.trim().trim_start_matches('$').to_string(),
                value.trim().to_string(),
            );
        } else {
            positional.push(arg.clone());
        }
    }
    let mut bindings = BTreeMap::new();
    let mut pos_idx = 0usize;
    for param in &mac.params {
        let value = if let Some(value) = keyword.remove(&param.name) {
            value
        } else if let Some(value) = positional.get(pos_idx) {
            pos_idx += 1;
            value.clone()
        } else {
            param.default.clone().unwrap_or_default()
        };
        bindings.insert(param.name.clone(), value);
    }
    substitute_macro_params(&mac.body, &bindings)
}

pub(super) fn substitute_macro_params(body: &str, bindings: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(body.len());
    let chars = body.chars().collect::<Vec<_>>();
    let mut i = 0usize;
    let mut in_quotes = false;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '"' {
            in_quotes = !in_quotes;
            out.push(ch);
            i += 1;
            continue;
        }
        if !in_quotes && ch == '$' && i + 1 < chars.len() {
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let name = chars[i + 1..j].iter().collect::<String>();
            if let Some(value) = bindings.get(&name) {
                out.push_str(value);
            } else {
                out.push('$');
                out.push_str(&name);
            }
            i = j;
            continue;
        }
        if !in_quotes && (ch.is_ascii_alphabetic() || ch == '_') {
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let name = chars[i..j].iter().collect::<String>();
            if let Some(value) = bindings.get(&name) {
                out.push_str(value);
            } else {
                out.push_str(&name);
            }
            i = j;
            continue;
        }
        out.push(ch);
        i += 1;
    }
    out
}

pub(super) fn substitute_vars(line: &str, vars: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(line.len());
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;
    let mut in_quotes = false;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '"' {
            in_quotes = !in_quotes;
            out.push(ch);
            i += 1;
            continue;
        }
        if !in_quotes
            && ch == '$'
            && i + 1 < chars.len()
            && (chars[i + 1].is_ascii_alphanumeric() || chars[i + 1] == '_')
        {
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            let name: String = chars[i + 1..j].iter().collect();
            if let Some(value) = vars.get(&name) {
                let (json_path, next_idx) = collect_json_path_suffix(&chars, j);
                if !json_path.is_empty()
                    && serde_json::from_str::<serde_json::Value>(value.trim()).is_ok()
                {
                    out.push_str(&super::builtins::get_json_attribute(value, &json_path));
                    i = next_idx;
                    continue;
                }
                out.push_str(value);
            } else {
                out.push('$');
                out.push_str(&name);
            }
            i = j;
            continue;
        }
        out.push(ch);
        i += 1;
    }
    out
}

pub(super) fn collect_json_path_suffix(chars: &[char], start: usize) -> (String, usize) {
    let mut path = String::new();
    let mut i = start;
    while i < chars.len() {
        match chars[i] {
            '.' => {
                let mut j = i + 1;
                if j >= chars.len()
                    || !(chars[j].is_ascii_alphabetic() || chars[j] == '_' || chars[j] == '-')
                {
                    break;
                }
                if !path.is_empty() {
                    path.push('.');
                }
                while j < chars.len()
                    && (chars[j].is_ascii_alphanumeric() || chars[j] == '_' || chars[j] == '-')
                {
                    path.push(chars[j]);
                    j += 1;
                }
                i = j;
            }
            '[' => {
                let mut j = i + 1;
                let mut in_quotes = false;
                let mut quote = '\0';
                while j < chars.len() {
                    let ch = chars[j];
                    if in_quotes {
                        if ch == quote {
                            in_quotes = false;
                        }
                    } else if ch == '"' || ch == '\'' {
                        in_quotes = true;
                        quote = ch;
                    } else if ch == ']' {
                        break;
                    }
                    j += 1;
                }
                if j >= chars.len() || chars[j] != ']' {
                    break;
                }
                for ch in &chars[i..=j] {
                    path.push(*ch);
                }
                i = j + 1;
            }
            _ => break,
        }
    }
    (path, i)
}

pub(super) fn substitute_tokens_and_vars(
    line: &str,
    state: &PreprocState,
) -> Result<String, Diagnostic> {
    let mut current = line.to_string();
    for _ in 0..MAX_PREPROC_CALL_DEPTH {
        let next = substitute_defines(&current, &state.defines, &state.macros)?;
        if next == current {
            return Ok(substitute_vars(&next, &state.vars));
        }
        if next.len() > MAX_PREPROC_MACRO_EXPANSION_BYTES {
            return Err(Diagnostic::error_code(
                "E_PREPROC_MACRO_DEPTH",
                format!(
                    "preprocessor macro expansion exceeded maximum of {MAX_PREPROC_MACRO_EXPANSION_BYTES} bytes"
                ),
            ));
        }
        current = next;
    }
    Err(Diagnostic::error_code(
        "E_PREPROC_MACRO_DEPTH",
        format!("preprocessor macro expansion exceeded maximum of {MAX_PREPROC_CALL_DEPTH}"),
    ))
}

pub(super) fn parse_variable_assignment(
    name: &str,
    arg: &str,
    raw: &str,
) -> Option<PreprocessDirective> {
    parse_variable_assignment_with_scope(name, arg, raw, PreprocVariableScope::Default)
}

pub(super) fn parse_variable_assignment_with_scope(
    name: &str,
    arg: &str,
    raw: &str,
    scope: PreprocVariableScope,
) -> Option<PreprocessDirective> {
    let var = name.strip_prefix('$')?.trim().to_string();
    if var.is_empty() {
        return Some(PreprocessDirective::JsonPreproc(raw.to_string()));
    }
    if let Some(value) = arg.strip_prefix("?=") {
        return Some(PreprocessDirective::VariableAssign {
            name: var,
            value: value.trim().to_string(),
            conditional: true,
            scope,
        });
    }
    if let Some(value) = arg.strip_prefix('=') {
        return Some(PreprocessDirective::VariableAssign {
            name: var,
            value: value.trim().to_string(),
            conditional: false,
            scope,
        });
    }
    Some(PreprocessDirective::JsonPreproc(raw.to_string()))
}

pub(super) fn parse_scoped_variable_assignment(
    arg: &str,
    raw: &str,
    scope: PreprocVariableScope,
) -> Option<PreprocessDirective> {
    let trimmed = arg.trim_start();
    if !trimmed.starts_with('$') {
        return Some(PreprocessDirective::JsonPreproc(raw.to_string()));
    }
    let chars = trimmed.chars().collect::<Vec<_>>();
    let mut end = 1usize;
    while end < chars.len() && (chars[end].is_ascii_alphanumeric() || chars[end] == '_') {
        end += 1;
    }
    let name = chars[..end].iter().collect::<String>();
    let rest = chars[end..].iter().collect::<String>();
    parse_variable_assignment_with_scope(&name, rest.trim_start(), raw, scope)
}

pub(super) fn parse_named_call(rest: &str) -> Option<(String, String)> {
    let rest = rest.trim();
    let open = rest.find('(')?;
    let close = rest.rfind(')')?;
    if close <= open || close != rest.len() - 1 {
        return None;
    }
    let name = rest[..open].trim();
    let mut chars = name.chars();
    let first = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_' || first == '$')
        || !chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }
    let args = rest[open + 1..close].trim().to_string();
    Some((name.to_string(), args))
}

pub(super) fn expand_preprocessor_text(
    raw_line: &str,
    state: &PreprocState,
    call_depth: usize,
) -> Result<String, Diagnostic> {
    let substituted = collapse_macro_concat(&substitute_tokens_and_vars(raw_line, state)?);
    let expanded = expand_function_invocations(&substituted, state, call_depth)?;
    Ok(collapse_macro_concat(&expanded))
}

pub(super) fn collapse_macro_concat(line: &str) -> String {
    if !line.contains("##") {
        return line.to_string();
    }
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0usize;
    let mut in_double_quote = false;
    let mut in_single_quote = false;
    while i < chars.len() {
        if chars[i] == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
            out.push(chars[i]);
            i += 1;
            continue;
        }
        if chars[i] == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
            out.push(chars[i]);
            i += 1;
            continue;
        }
        if !in_double_quote
            && !in_single_quote
            && chars[i] == '#'
            && i + 1 < chars.len()
            && chars[i + 1] == '#'
        {
            while out.ends_with(char::is_whitespace) {
                out.pop();
            }
            i += 2;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

pub(super) fn expand_function_invocations(
    line: &str,
    state: &PreprocState,
    call_depth: usize,
) -> Result<String, Diagnostic> {
    if call_depth > MAX_PREPROC_CALL_DEPTH {
        return Err(Diagnostic::error_code(
            "E_PREPROC_CALL_DEPTH",
            format!("preprocessor call depth exceeded maximum of {MAX_PREPROC_CALL_DEPTH}"),
        ));
    }

    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len());
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '%'
            && i + 1 < chars.len()
            && (chars[i + 1].is_ascii_alphabetic() || chars[i + 1] == '_')
        {
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            if j < chars.len() && chars[j] == '(' {
                let call_name: String = chars[i + 1..j].iter().collect();
                let (args_raw, next_idx) = extract_parenthesized_args(&chars, j)?;
                // 1) User-defined callable wins over a builtin of the same
                //    name (parity with PlantUML which lets users shadow).
                if let Some(callable) = state.callables.get(&call_name) {
                    if callable.kind != PreprocCallableKind::Function {
                        return Err(Diagnostic::error_code(
                            "E_PREPROC_CALL_KIND",
                            format!(
                                "`{}` is a procedure and cannot be called as `%...` function",
                                call_name
                            ),
                        ));
                    }
                    let ret = execute_function_call(&call_name, &args_raw, state, call_depth + 1)?;
                    out.push_str(&ret);
                    i = next_idx;
                    continue;
                }
                // 2) Builtin dispatch.
                if let Some(ret) = dispatch_builtin(&call_name, &args_raw, state, call_depth)? {
                    out.push_str(&ret);
                    i = next_idx;
                    continue;
                }
                // 3) Otherwise, unknown — deterministic diagnostic.
                return Err(Diagnostic::error_code(
                    "E_PREPROC_BUILTIN_UNSUPPORTED",
                    format!(
                        "preprocessor builtin or unknown function `%{}(...)` is not supported in this deterministic subset",
                        call_name
                    ),
                ));
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    Ok(out)
}
