use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::diagnostic::Diagnostic;
use crate::source::{line_spans, MappedSpan, Span};

#[path = "control/callables.rs"]
mod callables;
#[path = "control/entrypoint.rs"]
mod entrypoint;
#[path = "control/flow.rs"]
mod flow;
#[path = "control/source_map.rs"]
mod source_map;
#[path = "control/url.rs"]
mod url;

pub(crate) use entrypoint::{preprocess, preprocess_with_map};

use callables::define_callable_block;
use flow::{
    check_include_depth, ensure_conditionals_closed, is_active, looks_like_iso_date_value,
    preprocess_foreach_directive, preprocess_while_directive,
};
use source_map::{annotate_preproc_diagnostic, preproc_source, push_mapped_line};
use url::process_include_url;

use super::builtins::{execute_procedure_call, invoke_dynamic_procedure};
use super::includes::{
    eval_simple_arithmetic, evaluate_assert_expression, evaluate_preprocess_expr,
    find_matching_enddefinelong, parse_preprocess_directive, process_import_directive,
    process_include_directive, process_include_many_directive, ImportDirectiveContext,
};
use super::macros::{
    expand_preprocessor_text, parse_macro_define, parse_macro_definelong, parse_named_call,
};
use super::{
    ConditionalFrame, ParseOptions, PreprocCallableKind, PreprocLoopSignal, PreprocState,
    PreprocVariableScope, PreprocessDirective,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn process_lines(
    source: &str,
    options: &ParseOptions,
    state: &mut PreprocState,
    include_stack: &mut Vec<PathBuf>,
    include_once_seen: &mut BTreeSet<PathBuf>,
    depth: usize,
    call_depth: usize,
    out: &mut String,
    mappings: &mut Vec<MappedSpan>,
) -> Result<(), Diagnostic> {
    check_include_depth(depth)?;

    let lines = source.lines().collect::<Vec<_>>();
    let spans = line_spans(source);
    let mut i = 0usize;
    let mut conditionals = Vec::<ConditionalFrame>::new();

    while i < lines.len() {
        let raw_line = lines[i];
        let raw_span = spans.get(i).copied().unwrap_or_else(|| Span::new(0, 0));
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
                    i = preprocess_while_directive(
                        &expr,
                        &lines,
                        i,
                        active,
                        options,
                        state,
                        include_stack,
                        include_once_seen,
                        depth,
                        call_depth,
                        out,
                        mappings,
                    )?;
                    continue;
                }
                PreprocessDirective::EndWhile => {
                    return Err(Diagnostic::error_code(
                        "E_PREPROC_WHILE_UNEXPECTED",
                        "`!endwhile` without matching `!while`",
                    ));
                }
                PreprocessDirective::Foreach(spec) => {
                    i = preprocess_foreach_directive(
                        &spec,
                        &lines,
                        i,
                        active,
                        options,
                        state,
                        include_stack,
                        include_once_seen,
                        depth,
                        call_depth,
                        out,
                        mappings,
                    )?;
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
                    let end_idx =
                        define_callable_block(&lines, i, PreprocCallableKind::Function, state)?;
                    i = end_idx + 1;
                    continue;
                }
                PreprocessDirective::Procedure => {
                    let end_idx =
                        define_callable_block(&lines, i, PreprocCallableKind::Procedure, state)?;
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
                        let _ =
                            expand_preprocessor_text(&payload, state, call_depth).map_err(|d| {
                                annotate_preproc_diagnostic(d, source, raw_span, include_stack)
                            })?;
                    }
                }
                PreprocessDirective::DumpMemory(payload) => {
                    if active {
                        let _ =
                            expand_preprocessor_text(&payload, state, call_depth).map_err(|d| {
                                annotate_preproc_diagnostic(d, source, raw_span, include_stack)
                            })?;
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
                            mappings,
                        )
                        .map_err(|d| {
                            annotate_preproc_diagnostic(d, source, raw_span, include_stack)
                        })?;
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
                PreprocessDirective::DefineLong(header) => {
                    let end_idx = find_matching_enddefinelong(&lines, i)?;
                    if active {
                        // Collect body lines between the header and !enddefinelong.
                        let body_lines = &lines[i + 1..end_idx];
                        let (name, mac) =
                            parse_macro_definelong(&header, body_lines).map_err(|d| {
                                annotate_preproc_diagnostic(d, source, raw_span, include_stack)
                            })?;
                        state.defines.remove(&name);
                        state.macros.insert(name, mac);
                    }
                    i = end_idx + 1;
                    continue;
                }
                PreprocessDirective::EndDefineLong => {
                    if active {
                        return Err(Diagnostic::error_code(
                            "E_ENDDEFINELONG_UNEXPECTED",
                            "`!enddefinelong` without matching `!definelong`",
                        ));
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
                                let rendered = expand_preprocessor_text(&value, state, call_depth)
                                    .map_err(|d| {
                                        annotate_preproc_diagnostic(
                                            d,
                                            source,
                                            raw_span,
                                            include_stack,
                                        )
                                    })?;
                                let resolved = rendered.trim();
                                // If the resolved value is a simple integer arithmetic
                                // expression (e.g. "0 + 1", "3 - 1"), evaluate it so
                                // that !while loop counters increment correctly.
                                let final_val = if looks_like_iso_date_value(resolved) {
                                    None
                                } else {
                                    eval_simple_arithmetic(resolved)
                                }
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
                            mappings,
                        )
                        .map_err(|d| {
                            annotate_preproc_diagnostic(d, source, raw_span, include_stack)
                        })?;
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
                            mappings,
                        )
                        .map_err(|d| {
                            annotate_preproc_diagnostic(d, source, raw_span, include_stack)
                        })?;
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
                            mappings,
                        )
                        .map_err(|d| {
                            annotate_preproc_diagnostic(d, source, raw_span, include_stack)
                        })?;
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
                            mappings,
                        )
                        .map_err(|d| {
                            annotate_preproc_diagnostic(d, source, raw_span, include_stack)
                        })?;
                    }
                }
                PreprocessDirective::IncludeUrl(raw_target) => {
                    if active {
                        process_include_url(
                            &raw_target,
                            options,
                            state,
                            include_stack,
                            include_once_seen,
                            depth,
                            call_depth,
                            out,
                            mappings,
                        )
                        .map_err(|d| {
                            annotate_preproc_diagnostic(d, source, raw_span, include_stack)
                        })?;
                    }
                }
                PreprocessDirective::Import(raw_target) => {
                    if active {
                        process_import_directive(
                            &raw_target,
                            ImportDirectiveContext {
                                options,
                                state,
                                include_stack,
                                include_once_seen,
                                depth,
                                call_depth,
                                out,
                                mappings,
                            },
                        )
                        .map_err(|d| {
                            annotate_preproc_diagnostic(d, source, raw_span, include_stack)
                        })?;
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
                            mappings,
                        )?;
                        if state.loop_signal.is_some() {
                            return Ok(());
                        }
                    }
                }
                PreprocessDirective::Unsupported(name) => {
                    if active {
                        let diagnostic = Diagnostic::error_code(
                            "E_PREPROC_UNSUPPORTED",
                            format!("unsupported preprocessor directive `!{name}`"),
                        )
                        .with_span(raw_span);
                        return Err(if include_stack.is_empty() {
                            diagnostic
                        } else {
                            diagnostic.with_source(preproc_source(source, raw_span, include_stack))
                        });
                    }
                }
                PreprocessDirective::Passthrough(line) => {
                    if active {
                        push_mapped_line(out, mappings, &line, source, raw_span, include_stack);
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
                            mappings,
                        )?;
                        if state.loop_signal.is_some() {
                            return Ok(());
                        }
                        i += 1;
                        continue;
                    }
                }
            }
            let expanded_line = expand_preprocessor_text(raw_line, state, call_depth)
                .map_err(|d| annotate_preproc_diagnostic(d, source, raw_span, include_stack))?;
            push_mapped_line(
                out,
                mappings,
                &expanded_line,
                source,
                raw_span,
                include_stack,
            );
        }
        i += 1;
    }

    ensure_conditionals_closed(&conditionals)
}
