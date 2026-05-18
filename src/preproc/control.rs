use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::diagnostic::Diagnostic;

use super::builtins::{
    execute_procedure_call, invoke_dynamic_procedure, parse_callable_definition,
    preprocessor_foreach_bindings,
};
use super::includes::{
    consume_preprocessor_block, eval_simple_arithmetic, evaluate_assert_expression,
    evaluate_preprocess_expr, extract_url, fetch_url_include, find_matching_endfor,
    find_matching_endwhile, parse_preprocess_directive, process_import_directive,
    process_include_directive, process_include_many_directive,
};
use super::macros::{expand_preprocessor_text, parse_macro_define, parse_named_call};
use super::{
    ConditionalFrame, ParseOptions, PreprocCallableKind, PreprocLoopSignal, PreprocState,
    PreprocVariableScope, PreprocessDirective, MAX_INCLUDE_DEPTH, MAX_PREPROC_WHILE_ITERATIONS,
};

pub(crate) fn preprocess(source: &str, options: &ParseOptions) -> Result<String, Diagnostic> {
    let mut state = PreprocState::default();
    let mut include_stack = Vec::new();
    let mut include_once_seen = BTreeSet::new();
    let mut expanded = String::new();

    preprocess_text(
        source,
        options,
        &mut state,
        &mut include_stack,
        &mut include_once_seen,
        0,
        0,
        &mut expanded,
    )?;

    Ok(expanded)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn preprocess_text(
    source: &str,
    options: &ParseOptions,
    state: &mut PreprocState,
    include_stack: &mut Vec<PathBuf>,
    include_once_seen: &mut BTreeSet<PathBuf>,
    depth: usize,
    call_depth: usize,
    out: &mut String,
) -> Result<(), Diagnostic> {
    if depth > MAX_INCLUDE_DEPTH {
        return Err(Diagnostic::error(format!(
            "include depth exceeded maximum of {MAX_INCLUDE_DEPTH}"
        )));
    }

    let lines = source.lines().collect::<Vec<_>>();
    let mut i = 0usize;
    let mut conditionals = Vec::<ConditionalFrame>::new();

    while i < lines.len() {
        let raw_line = lines[i];
        let line = raw_line.trim();
        let active = is_active(&conditionals);

        if let Some(directive) = parse_preprocess_directive(line) {
            match directive {
                PreprocessDirective::If(expr) => {
                    let cond = if active {
                        evaluate_preprocess_expr(&expr, state)?
                    } else {
                        false
                    };
                    conditionals.push(ConditionalFrame {
                        parent_active: active,
                        branch_taken: cond && active,
                        current_active: cond && active,
                        seen_else: false,
                    });
                }
                PreprocessDirective::IfDef { name, negated } => {
                    let cond = if active {
                        state.defines.contains_key(&name) ^ negated
                    } else {
                        false
                    };
                    conditionals.push(ConditionalFrame {
                        parent_active: active,
                        branch_taken: cond && active,
                        current_active: cond && active,
                        seen_else: false,
                    });
                }
                PreprocessDirective::ElseIf(expr) => {
                    let Some(frame) = conditionals.last_mut() else {
                        return Err(Diagnostic::error_code(
                            "E_PREPROC_COND_UNEXPECTED",
                            "`!elseif` without open conditional block",
                        ));
                    };
                    if frame.seen_else {
                        return Err(Diagnostic::error_code(
                            "E_PREPROC_COND_ORDER",
                            "`!elseif` cannot appear after `!else`",
                        ));
                    }
                    if !frame.parent_active || frame.branch_taken {
                        frame.current_active = false;
                    } else {
                        let cond = evaluate_preprocess_expr(&expr, state)?;
                        frame.current_active = cond;
                        frame.branch_taken |= cond;
                    }
                }
                PreprocessDirective::Else => {
                    let Some(frame) = conditionals.last_mut() else {
                        return Err(Diagnostic::error_code(
                            "E_PREPROC_COND_UNEXPECTED",
                            "`!else` without open conditional block",
                        ));
                    };
                    if frame.seen_else {
                        return Err(Diagnostic::error_code(
                            "E_PREPROC_COND_ORDER",
                            "conditional block already has an `!else` branch",
                        ));
                    }
                    frame.seen_else = true;
                    frame.current_active = frame.parent_active && !frame.branch_taken;
                    frame.branch_taken = true;
                }
                PreprocessDirective::EndIf => {
                    if conditionals.pop().is_none() {
                        return Err(Diagnostic::error_code(
                            "E_PREPROC_COND_UNEXPECTED",
                            "`!endif` without open conditional block",
                        ));
                    }
                }
                PreprocessDirective::While(expr) => {
                    let endwhile = find_matching_endwhile(&lines, i)?;
                    if active {
                        let block = lines[i + 1..endwhile].join("\n");
                        let mut iterations = 0usize;
                        while evaluate_preprocess_expr(&expr, state)? {
                            iterations += 1;
                            if iterations > MAX_PREPROC_WHILE_ITERATIONS {
                                return Err(Diagnostic::error_code(
                                    "E_PREPROC_WHILE_LIMIT",
                                    format!(
                                        "`!while` iteration limit exceeded ({MAX_PREPROC_WHILE_ITERATIONS})"
                                    ),
                                ));
                            }
                            let signal = preprocess_loop_block(
                                &block,
                                options,
                                state,
                                include_stack,
                                include_once_seen,
                                depth,
                                call_depth,
                                out,
                            )?;
                            match signal {
                                Some(PreprocLoopSignal::Break) => break,
                                Some(PreprocLoopSignal::Continue) | None => {}
                            }
                        }
                    }
                    i = endwhile + 1;
                    continue;
                }
                PreprocessDirective::EndWhile => {
                    return Err(Diagnostic::error_code(
                        "E_PREPROC_WHILE_UNEXPECTED",
                        "`!endwhile` without matching `!while`",
                    ));
                }
                PreprocessDirective::Foreach(spec) => {
                    let endfor = find_matching_endfor(&lines, i)?;
                    if active {
                        // Expected forms:
                        //   `$var in val1, val2, val3`
                        //   `$var in $listvar`
                        //   `$key, $value in $mapvar`
                        // For two loop variables, JSON objects iterate
                        // key/value pairs and JSON arrays iterate index/value
                        // pairs. This keeps common PlantUML map/list loops
                        // deterministic without adding runtime dependencies.
                        let parts: Vec<&str> = spec.splitn(2, " in ").collect();
                        if parts.len() != 2 {
                            return Err(Diagnostic::error_code(
                                "E_PREPROC_FOREACH_FORM",
                                "`!foreach` requires form `$var in val1, val2, ...`",
                            ));
                        }
                        let var_names = parts[0]
                            .split(',')
                            .map(|name| name.trim().trim_start_matches('$').to_string())
                            .filter(|name| !name.is_empty())
                            .collect::<Vec<_>>();
                        if var_names.is_empty() {
                            return Err(Diagnostic::error_code(
                                "E_PREPROC_FOREACH_FORM",
                                "`!foreach` requires at least one loop variable",
                            ));
                        }
                        let rhs = expand_preprocessor_text(parts[1].trim(), state, 0)?;
                        let bindings = preprocessor_foreach_bindings(&var_names, &rhs);
                        let block = lines[i + 1..endfor].join("\n");
                        let prev = var_names
                            .iter()
                            .map(|name| (name.clone(), state.vars.get(name).cloned()))
                            .collect::<Vec<_>>();
                        let mut should_break = false;
                        for row in bindings {
                            for (name, value) in row {
                                state.vars.insert(name, value);
                            }
                            let signal = preprocess_loop_block(
                                &block,
                                options,
                                state,
                                include_stack,
                                include_once_seen,
                                depth,
                                call_depth,
                                out,
                            )?;
                            match signal {
                                Some(PreprocLoopSignal::Break) => {
                                    should_break = true;
                                }
                                Some(PreprocLoopSignal::Continue) | None => {}
                            }
                            if should_break {
                                break;
                            }
                        }
                        for (name, value) in prev {
                            match value {
                                Some(v) => {
                                    state.vars.insert(name, v);
                                }
                                None => {
                                    state.vars.remove(&name);
                                }
                            }
                        }
                    }
                    i = endfor + 1;
                    continue;
                }
                PreprocessDirective::Break => {
                    if active {
                        if state.loop_depth == 0 {
                            return Err(Diagnostic::error_code(
                                "E_PREPROC_BREAK_OUTSIDE_LOOP",
                                "`!break` can only be used inside `!while` or `!foreach`",
                            ));
                        }
                        state.loop_signal = Some(PreprocLoopSignal::Break);
                        return Ok(());
                    }
                }
                PreprocessDirective::Continue => {
                    if active {
                        if state.loop_depth == 0 {
                            return Err(Diagnostic::error_code(
                                "E_PREPROC_CONTINUE_OUTSIDE_LOOP",
                                "`!continue` can only be used inside `!while` or `!foreach`",
                            ));
                        }
                        state.loop_signal = Some(PreprocLoopSignal::Continue);
                        return Ok(());
                    }
                }
                PreprocessDirective::EndFor => {
                    return Err(Diagnostic::error_code(
                        "E_PREPROC_FOREACH_UNEXPECTED",
                        "`!endfor` without matching `!foreach`",
                    ));
                }
                PreprocessDirective::Function => {
                    let end_idx = consume_preprocessor_block(
                        &lines,
                        i,
                        PreprocessDirective::Function,
                        PreprocessDirective::EndFunction,
                        "E_FUNCTION_UNCLOSED",
                        "!endfunction",
                    )?;
                    let header = lines[i].trim();
                    let callable = parse_callable_definition(
                        header,
                        &lines[i + 1..end_idx],
                        PreprocCallableKind::Function,
                    )?;
                    state.callables.insert(callable.0, callable.1);
                    i = end_idx + 1;
                    continue;
                }
                PreprocessDirective::Procedure => {
                    let end_idx = consume_preprocessor_block(
                        &lines,
                        i,
                        PreprocessDirective::Procedure,
                        PreprocessDirective::EndProcedure,
                        "E_PROCEDURE_UNCLOSED",
                        "!endprocedure",
                    )?;
                    let header = lines[i].trim();
                    let callable = parse_callable_definition(
                        header,
                        &lines[i + 1..end_idx],
                        PreprocCallableKind::Procedure,
                    )?;
                    state.callables.insert(callable.0, callable.1);
                    i = end_idx + 1;
                    continue;
                }
                PreprocessDirective::Assert(body) => {
                    if active && !evaluate_assert_expression(&body, state)? {
                        return Err(Diagnostic::error_code(
                            "E_PREPROC_ASSERT",
                            format!("!assert failed: {body}"),
                        ));
                    }
                }
                PreprocessDirective::Log(payload) => {
                    if active {
                        // Expand for diagnostics parity; result is discarded
                        // because we are a deterministic offline renderer.
                        let _ = expand_preprocessor_text(&payload, state, call_depth)?;
                    }
                }
                PreprocessDirective::DumpMemory(payload) => {
                    if active {
                        let _ = expand_preprocessor_text(&payload, state, call_depth)?;
                    }
                }
                PreprocessDirective::DynamicInvocation(raw) => {
                    if active {
                        invoke_dynamic_procedure(
                            &raw,
                            state,
                            options,
                            include_stack,
                            include_once_seen,
                            depth,
                            call_depth,
                            out,
                        )?;
                    }
                }
                PreprocessDirective::JsonPreproc(raw) => {
                    if active {
                        return Err(Diagnostic::error_code(
                            "E_PREPROC_JSON_UNSUPPORTED",
                            format!(
                                "JSON preprocessing is not supported in this deterministic subset: `{}`",
                                raw
                            ),
                        ));
                    }
                }
                PreprocessDirective::EndFunction | PreprocessDirective::EndProcedure => {
                    if active {
                        return Err(Diagnostic::error_code(
                            "E_PREPROC_UNEXPECTED",
                            "unexpected preprocessor block terminator",
                        ));
                    }
                }
                PreprocessDirective::Define(body) => {
                    if active {
                        if let Some((name, mac)) = parse_macro_define(&body)? {
                            state.defines.remove(&name);
                            state.macros.insert(name, mac);
                        } else {
                            let (name, value) = body.split_once(' ').unwrap_or((body.as_str(), ""));
                            let name = name.trim();
                            if !name.is_empty() {
                                state.macros.remove(name);
                                state
                                    .defines
                                    .insert(name.to_string(), value.trim().to_string());
                            }
                        }
                    }
                }
                PreprocessDirective::Undef(name) => {
                    if active {
                        let name = name.trim();
                        if !name.is_empty() {
                            let name = name.split_once('(').map_or(name, |(n, _)| n).trim();
                            state.defines.remove(name);
                            state.macros.remove(name);
                        }
                    }
                }
                PreprocessDirective::VariableAssign {
                    name,
                    value,
                    conditional,
                    scope,
                } => {
                    if active {
                        let trimmed = value.trim_start();
                        let is_json_literal = trimmed.starts_with('{') || trimmed.starts_with('[');
                        if !conditional || !state.vars.contains_key(&name) {
                            let assigned_name = name.clone();
                            if is_json_literal {
                                // Preserve JSON literal verbatim (no
                                // substitution / no trimming inside the
                                // braces) so `%get_json_attribute` can do a
                                // simple key lookup. We still trim outer
                                // whitespace for determinism.
                                state.vars.insert(name, value.trim().to_string());
                            } else {
                                let rendered = expand_preprocessor_text(&value, state, call_depth)?;
                                let resolved = rendered.trim();
                                // If the resolved value is a simple integer arithmetic
                                // expression (e.g. "0 + 1", "3 - 1"), evaluate it so
                                // that !while loop counters increment correctly.
                                let final_val = eval_simple_arithmetic(resolved)
                                    .map(|n| n.to_string())
                                    .unwrap_or_else(|| resolved.to_string());
                                state.vars.insert(name, final_val);
                            }
                            if scope == PreprocVariableScope::Global {
                                state.global_assigns.borrow_mut().insert(assigned_name);
                            }
                        }
                    }
                }
                PreprocessDirective::Include(raw_target) => {
                    if active {
                        process_include_directive(
                            &raw_target,
                            "!include",
                            false,
                            false,
                            options,
                            state,
                            include_stack,
                            include_once_seen,
                            depth,
                            call_depth,
                            out,
                        )?;
                    }
                }
                PreprocessDirective::IncludeOnce(raw_target) => {
                    if active {
                        process_include_directive(
                            &raw_target,
                            "!include_once",
                            true,
                            false,
                            options,
                            state,
                            include_stack,
                            include_once_seen,
                            depth,
                            call_depth,
                            out,
                        )?;
                    }
                }
                PreprocessDirective::IncludeMany(raw_target) => {
                    if active {
                        process_include_many_directive(
                            &raw_target,
                            options,
                            state,
                            include_stack,
                            include_once_seen,
                            depth,
                            call_depth,
                            out,
                        )?;
                    }
                }
                PreprocessDirective::IncludeSub(raw_target) => {
                    if active {
                        process_include_directive(
                            &raw_target,
                            "!includesub",
                            false,
                            true,
                            options,
                            state,
                            include_stack,
                            include_once_seen,
                            depth,
                            call_depth,
                            out,
                        )?;
                    }
                }
                PreprocessDirective::IncludeUrl(raw_target) => {
                    if active {
                        if !options.allow_url_includes {
                            return Err(Diagnostic::error_code(
                                "E_INCLUDE_URL_DISABLED",
                                format!(
                                    "!includeurl URL includes are disabled (pass --allow-url-includes to enable): {raw_target}"
                                ),
                            ));
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            let url = extract_url(&raw_target);
                            let content = fetch_url_include(url)?;
                            preprocess_text(
                                &content,
                                options,
                                state,
                                include_stack,
                                include_once_seen,
                                depth + 1,
                                call_depth,
                                out,
                            )?;
                        }
                        #[cfg(target_arch = "wasm32")]
                        {
                            return Err(Diagnostic::error_code(
                                "E_INCLUDE_URL_UNSUPPORTED",
                                format!("!includeurl URL targets are not supported in WASM: {raw_target}"),
                            ));
                        }
                    }
                }
                PreprocessDirective::Import(raw_target) => {
                    if active {
                        process_import_directive(
                            &raw_target,
                            options,
                            state,
                            include_stack,
                            include_once_seen,
                            depth,
                            call_depth,
                            out,
                        )?;
                    }
                }
                PreprocessDirective::ProcedureCall { name, args } => {
                    if active {
                        execute_procedure_call(
                            &name,
                            &args,
                            state,
                            options,
                            include_stack,
                            include_once_seen,
                            depth,
                            call_depth,
                            out,
                        )?;
                        if state.loop_signal.is_some() {
                            return Ok(());
                        }
                    }
                }
                PreprocessDirective::Unsupported(name) => {
                    if active {
                        return Err(Diagnostic::error_code(
                            "E_PREPROC_UNSUPPORTED",
                            format!("unsupported preprocessor directive `!{name}`"),
                        ));
                    }
                }
                PreprocessDirective::Passthrough(line) => {
                    if active {
                        out.push_str(&line);
                        out.push('\n');
                    }
                }
                PreprocessDirective::NoOp => {
                    // Intentionally drop: e.g. `!startsub`/`!endsub` markers.
                }
            }
            i += 1;
            continue;
        }

        if active {
            if let Some((name, args)) = parse_named_call(line) {
                if let Some(callable) = state.callables.get(&name) {
                    if callable.kind == PreprocCallableKind::Procedure {
                        execute_procedure_call(
                            &name,
                            &args,
                            state,
                            options,
                            include_stack,
                            include_once_seen,
                            depth,
                            call_depth,
                            out,
                        )?;
                        if state.loop_signal.is_some() {
                            return Ok(());
                        }
                        i += 1;
                        continue;
                    }
                }
            }
            let expanded_line = expand_preprocessor_text(raw_line, state, call_depth)?;
            out.push_str(&expanded_line);
            out.push('\n');
        }
        i += 1;
    }

    if !conditionals.is_empty() {
        return Err(Diagnostic::error_code(
            "E_PREPROC_COND_UNCLOSED",
            "missing `!endif` for conditional block",
        ));
    }

    Ok(())
}

fn is_active(conditionals: &[ConditionalFrame]) -> bool {
    conditionals.iter().all(|f| f.current_active)
}

#[allow(clippy::too_many_arguments)]
fn preprocess_loop_block(
    source: &str,
    options: &ParseOptions,
    state: &mut PreprocState,
    include_stack: &mut Vec<PathBuf>,
    include_once_seen: &mut BTreeSet<PathBuf>,
    depth: usize,
    call_depth: usize,
    out: &mut String,
) -> Result<Option<PreprocLoopSignal>, Diagnostic> {
    state.loop_depth += 1;
    let result = preprocess_text(
        source,
        options,
        state,
        include_stack,
        include_once_seen,
        depth,
        call_depth,
        out,
    );
    state.loop_depth = state.loop_depth.saturating_sub(1);
    result?;
    Ok(state.loop_signal.take())
}
