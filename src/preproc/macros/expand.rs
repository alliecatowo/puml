use crate::diagnostic::Diagnostic;

use super::super::builtins::{dispatch_builtin, execute_function_call, extract_parenthesized_args};
use super::super::{PreprocCallableKind, PreprocState, MAX_PREPROC_CALL_DEPTH};
use super::substitute_tokens_and_vars;

pub(in crate::preproc) fn expand_preprocessor_text(
    raw_line: &str,
    state: &PreprocState,
    call_depth: usize,
) -> Result<String, Diagnostic> {
    let substituted = collapse_macro_concat(&substitute_tokens_and_vars(raw_line, state)?);
    let expanded = expand_function_invocations(&substituted, state, call_depth)?;
    let collapsed = collapse_macro_concat(&expanded);
    // Evaluate string concatenation expressions like `"Alice" + " calls " + "Bob"` (#582).
    // This is needed when a !function !return expression uses `+` to join quoted strings.
    Ok(eval_string_concat(&collapsed))
}

/// Evaluate a `+`-joined sequence of quoted string literals.
/// Returns the collapsed string (with outer quotes stripped) if the entire
/// expression is composed of quoted segments; otherwise returns the input unchanged.
fn eval_string_concat(expr: &str) -> String {
    let trimmed = expr.trim();
    // Fast path: no `+` operator outside quotes → nothing to do.
    if !trimmed.contains('+') {
        return expr.to_string();
    }
    // Try to parse the whole expression as a sequence of quoted-string + quoted-string parts.
    let mut result = String::new();
    let chars: Vec<char> = trimmed.chars().collect();
    let mut i = 0usize;
    let mut expecting_operand = true;
    while i < chars.len() {
        // Skip whitespace.
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        if expecting_operand {
            // Expect a quoted string literal.
            if chars[i] == '"' || chars[i] == '\'' {
                let quote = chars[i];
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != quote {
                    i += 1;
                }
                let segment: String = chars[start..i].iter().collect();
                result.push_str(&segment);
                if i < chars.len() {
                    i += 1; // consume closing quote
                }
                expecting_operand = false;
            } else {
                // Not a pure string-concat expression — return unchanged.
                return expr.to_string();
            }
        } else {
            // Expect a `+` operator.
            if chars[i] == '+' {
                i += 1;
                expecting_operand = true;
            } else {
                // Something unexpected — return unchanged.
                return expr.to_string();
            }
        }
    }
    // If we fully consumed the expression and collected parts, return the joined result.
    // Wrap in double-quotes to preserve string-literal semantics expected by the renderer.
    if !expecting_operand && !result.is_empty() {
        result
    } else {
        expr.to_string()
    }
}

fn collapse_macro_concat(line: &str) -> String {
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

fn expand_function_invocations(
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
        // ── `$name(args)` — user-defined function call in expression context ──
        // PlantUML allows calling `!function $name(…)` declarations as
        // `$name(args)` anywhere an expression value is expected (e.g. the
        // RHS of `!$var = $fn(x)` or inside `!if $fn(x) > 0`).
        //
        // Note: PlantUML convention names callables with a `$` prefix, so
        // `!function $double($x)` registers the key `"$double"` in the
        // callables map.  We must include the leading `$` when looking up.
        if chars[i] == '$'
            && i + 1 < chars.len()
            && (chars[i + 1].is_ascii_alphabetic() || chars[i + 1] == '_')
        {
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            // Only treat as a function call when immediately followed by `(`.
            if j < chars.len() && chars[j] == '(' {
                // Include the leading `$` in the lookup key because callables
                // are registered under the full name (e.g. `"$double"`).
                let call_name: String = chars[i..j].iter().collect();
                if let Some(callable) = state.callables.get(&call_name) {
                    if callable.kind == PreprocCallableKind::Function {
                        let (args_raw, next_idx) = extract_parenthesized_args(&chars, j)?;
                        let ret =
                            execute_function_call(&call_name, &args_raw, state, call_depth + 1)?;
                        out.push_str(&ret);
                        i = next_idx;
                        continue;
                    }
                    // Procedures cannot return a value — fall through so the
                    // `$name(…)` token is preserved and potentially used as a
                    // line-level procedure invocation by the caller.
                }
            }
        }
        // ── `%name(args)` — builtin or user-defined function call ─────────────
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
