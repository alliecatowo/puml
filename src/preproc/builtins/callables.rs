use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::diagnostic::Diagnostic;

use crate::preproc::control::preprocess_text;
use crate::preproc::macros::expand_preprocessor_text;
use crate::preproc::{
    ParseOptions, PreprocCallable, PreprocCallableKind, PreprocState, MAX_PREPROC_CALL_DEPTH,
};

use super::args::{parse_params, split_args, strip_quotes};
pub(in crate::preproc) fn parse_callable_definition(
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

pub(in crate::preproc) fn bind_callable_args(
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

pub(in crate::preproc) fn execute_function_call(
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
pub(in crate::preproc) fn execute_procedure_call(
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
pub(in crate::preproc) fn invoke_dynamic_procedure(
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
