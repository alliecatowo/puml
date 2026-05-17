use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::path::PathBuf;

#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
use hex;
#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
use sha2::{Digest, Sha256};

use crate::diagnostic::Diagnostic;

const MAX_INCLUDE_DEPTH: usize = 32;
const MAX_PREPROC_WHILE_ITERATIONS: usize = 10_000;
const MAX_PREPROC_CALL_DEPTH: usize = 32;
const MAX_PREPROC_MACRO_EXPANSION_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
struct IncludeTarget {
    path: PathBuf,
    tag: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ParseOptions {
    pub include_root: Option<PathBuf>,
    /// When true (the default), `!include https://...` fetches the URL and
    /// inlines its content. Set to false via `--no-url-includes` to reject
    /// all URL include targets with a clear diagnostic.
    pub allow_url_includes: bool,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            include_root: None,
            allow_url_includes: true,
        }
    }
}

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

#[derive(Debug, Clone)]
struct ConditionalFrame {
    parent_active: bool,
    branch_taken: bool,
    current_active: bool,
    seen_else: bool,
}

#[derive(Debug, Clone)]
enum PreprocessDirective {
    Define(String),
    Undef(String),
    Include(String),
    IncludeOnce(String),
    IncludeMany(String),
    IncludeSub(String),
    IncludeUrl(String),
    Import(String),
    If(String),
    IfDef {
        name: String,
        negated: bool,
    },
    ElseIf(String),
    Else,
    EndIf,
    While(String),
    EndWhile,
    Foreach(String),
    EndFor,
    Break,
    Continue,
    Function,
    EndFunction,
    Procedure,
    EndProcedure,
    Assert(String),
    Log(String),
    DumpMemory(String),
    DynamicInvocation(String),
    JsonPreproc(String),
    Passthrough(String),
    Unsupported(String),
    NoOp,
    ProcedureCall {
        name: String,
        args: String,
    },
    VariableAssign {
        name: String,
        value: String,
        conditional: bool,
        scope: PreprocVariableScope,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PreprocCallableKind {
    Function,
    Procedure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreprocVariableScope {
    Default,
    Local,
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreprocLoopSignal {
    Break,
    Continue,
}

#[derive(Debug, Clone)]
struct PreprocParam {
    name: String,
    default: Option<String>,
}

#[derive(Debug, Clone)]
struct PreprocCallable {
    kind: PreprocCallableKind,
    params: Vec<PreprocParam>,
    body: Vec<String>,
}

#[derive(Debug, Clone)]
struct PreprocMacro {
    params: Vec<PreprocParam>,
    body: String,
}

#[derive(Debug, Clone, Default)]
struct PreprocState {
    defines: BTreeMap<String, String>,
    macros: BTreeMap<String, PreprocMacro>,
    vars: BTreeMap<String, String>,
    callables: BTreeMap<String, PreprocCallable>,
    // Counters used by the deterministic builtins `%false_then_true` /
    // `%true_then_false`. PlantUML semantics use a per-callsite latch — we
    // key by the argument value so identical sources produce identical
    // AST/render bytes. Interior mutability lets us update from
    // `expand_function_invocations` which only borrows `&PreprocState`.
    false_then_true_counts: RefCell<BTreeMap<String, u64>>,
    true_then_false_counts: RefCell<BTreeMap<String, u64>>,
    global_assigns: RefCell<BTreeSet<String>>,
    loop_depth: usize,
    loop_signal: Option<PreprocLoopSignal>,
}

#[allow(clippy::too_many_arguments)]
fn preprocess_text(
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
                                format!("!includeurl URL includes are disabled: {raw_target}"),
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

/// On `wasm32` there is no filesystem available, so the entire `!include` /
/// `!includesub` / `!include_many` / `!import` family returns a friendly error
/// rather than attempting to read files. All FS-touching code below is gated
/// with `cfg(not(target_arch = "wasm32"))`; these stubs satisfy the call sites.
#[cfg(target_arch = "wasm32")]
fn include_not_supported_in_wasm(directive_name: &str) -> Diagnostic {
    Diagnostic::error_code(
        "E_INCLUDE_NOT_SUPPORTED_WASM",
        format!(
            "{directive_name} is not available in the in-browser renderer — the WASM build has no filesystem"
        ),
    )
}

#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
fn process_include_directive(
    _raw_target: &str,
    directive_name: &str,
    _include_once: bool,
    _require_tag: bool,
    _options: &ParseOptions,
    _state: &mut PreprocState,
    _include_stack: &mut Vec<PathBuf>,
    _include_once_seen: &mut BTreeSet<PathBuf>,
    _depth: usize,
    _call_depth: usize,
    _out: &mut String,
) -> Result<(), Diagnostic> {
    Err(include_not_supported_in_wasm(directive_name))
}

#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
fn process_include_many_directive(
    _raw_target: &str,
    _options: &ParseOptions,
    _state: &mut PreprocState,
    _include_stack: &mut Vec<PathBuf>,
    _include_once_seen: &mut BTreeSet<PathBuf>,
    _depth: usize,
    _call_depth: usize,
    _out: &mut String,
) -> Result<(), Diagnostic> {
    Err(include_not_supported_in_wasm("!include_many"))
}

#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
fn process_import_directive(
    _raw_target: &str,
    _options: &ParseOptions,
    _state: &mut PreprocState,
    _include_stack: &mut Vec<PathBuf>,
    _include_once_seen: &mut BTreeSet<PathBuf>,
    _depth: usize,
    _call_depth: usize,
    _out: &mut String,
) -> Result<(), Diagnostic> {
    Err(include_not_supported_in_wasm("!import"))
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
fn process_include_directive(
    raw_target: &str,
    directive_name: &str,
    include_once: bool,
    require_tag: bool,
    options: &ParseOptions,
    state: &mut PreprocState,
    include_stack: &mut Vec<PathBuf>,
    include_once_seen: &mut BTreeSet<PathBuf>,
    depth: usize,
    call_depth: usize,
    out: &mut String,
) -> Result<(), Diagnostic> {
    if raw_target.is_empty() {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_PATH_REQUIRED",
            format!("{directive_name} requires a relative path"),
        ));
    }

    if is_url_include_target(raw_target) {
        if !options.allow_url_includes {
            return Err(Diagnostic::error_code(
                "E_INCLUDE_URL_DISABLED",
                format!(
                    "{directive_name} URL includes are disabled (pass --no-url-includes to see this error or remove the flag to enable): {}",
                    raw_target
                ),
            ));
        }
        let url = extract_url(raw_target);
        let content = fetch_url_include(url)?;
        // Preprocess the fetched content recursively (without pushing to include_stack
        // since there's no local path — use the current stack as-is).
        return preprocess_text(
            &content,
            options,
            state,
            include_stack,
            include_once_seen,
            depth + 1,
            call_depth,
            out,
        );
    }

    // Angle-bracket form `!include <Library/Module>` resolves through the stdlib root,
    // behaving like `!import` but allowing tag selection and include_once semantics.
    if is_angle_bracket_include(raw_target) {
        return process_stdlib_angle_include(
            raw_target,
            directive_name,
            include_once,
            options,
            state,
            include_stack,
            include_once_seen,
            depth,
            call_depth,
            out,
        );
    }

    let include_target = parse_include_target(raw_target);
    if require_tag && include_target.tag.is_none() {
        return Err(Diagnostic::error_code(
            "E_INCLUDESUB_TAG_REQUIRED",
            format!("{directive_name} requires a target tag (`path!TAG`)"),
        ));
    }

    if include_target.path.is_absolute() {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_ABSOLUTE_PATH",
            format!(
                "{directive_name} only supports relative paths: {}",
                include_target.path.display()
            ),
        ));
    }

    let resolved = if is_stdlib_catalog_target(raw_target) && include_target.tag.is_none() {
        let stdlib_target = parse_import_target(raw_target)?;
        let stdlib_root = resolve_stdlib_root(options, include_stack)?;
        resolve_import_path(&stdlib_root, &stdlib_target)?
    } else {
        resolve_include_path(options, include_stack, &include_target.path)?
    };
    if include_once && !include_once_seen.insert(resolved.clone()) {
        return Ok(());
    }

    if include_stack.iter().any(|p| p == &resolved) {
        let mut cycle = include_stack
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>();
        cycle.push(resolved.display().to_string());
        return Err(Diagnostic::error_code(
            "E_INCLUDE_CYCLE",
            format!("include cycle detected: {}", cycle.join(" -> ")),
        ));
    }

    let mut content = fs::read_to_string(&resolved).map_err(|e| {
        Diagnostic::error_code(
            "E_INCLUDE_READ",
            format!("failed to read include '{}': {e}", resolved.display()),
        )
    })?;
    if let Some(tag) = include_target.tag.as_deref() {
        content = extract_include_tag(&content, tag).ok_or_else(|| {
            Diagnostic::error_code(
                "E_INCLUDE_TAG_NOT_FOUND",
                format!(
                    "include tag '{}' was not found in '{}'",
                    tag,
                    resolved.display()
                ),
            )
        })?;
    }

    include_stack.push(resolved);
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
    include_stack.pop();
    Ok(())
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn is_stdlib_catalog_target(raw_target: &str) -> bool {
    let trimmed = raw_target.trim();
    trimmed.starts_with('<') && trimmed.ends_with('>')
}

/// `!include_many` with optional glob expansion. When the path contains `*`
/// or `?`, expand it to every matching file in deterministic alphabetical
/// order; otherwise behave like `!include`. Globs only match the file-name
/// segment of the path so we cannot escape the include root by accident.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
fn process_include_many_directive(
    raw_target: &str,
    options: &ParseOptions,
    state: &mut PreprocState,
    include_stack: &mut Vec<PathBuf>,
    include_once_seen: &mut BTreeSet<PathBuf>,
    depth: usize,
    call_depth: usize,
    out: &mut String,
) -> Result<(), Diagnostic> {
    if raw_target.is_empty() {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_PATH_REQUIRED",
            "!include_many requires a relative path",
        ));
    }
    if is_url_include_target(raw_target) {
        if !options.allow_url_includes {
            return Err(Diagnostic::error_code(
                "E_INCLUDE_URL_DISABLED",
                format!("!include_many URL includes are disabled: {}", raw_target),
            ));
        }
        let url = extract_url(raw_target);
        let content = fetch_url_include(url)?;
        return preprocess_text(
            &content,
            options,
            state,
            include_stack,
            include_once_seen,
            depth + 1,
            call_depth,
            out,
        );
    }

    let target = parse_include_target(raw_target);
    if target.path.is_absolute() {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_ABSOLUTE_PATH",
            format!(
                "!include_many only supports relative paths: {}",
                target.path.display()
            ),
        ));
    }

    let glob_pattern = target.path.file_name().and_then(|n| n.to_str());
    let has_glob = glob_pattern
        .map(|n| n.contains('*') || n.contains('?'))
        .unwrap_or(false);

    if !has_glob {
        return process_include_directive(
            raw_target,
            "!include_many",
            false,
            false,
            options,
            state,
            include_stack,
            include_once_seen,
            depth,
            call_depth,
            out,
        );
    }

    let pattern = glob_pattern.unwrap_or("");
    let parent = target.path.parent().unwrap_or(Path::new(""));
    // Resolve directory by going through the include-root machinery using a
    // dummy file name in the parent dir.
    let dir_probe = parent.join("__glob_probe__");
    // We need the canonical parent dir; the helper expects an existing file.
    // Manually walk the resolution: produce an absolute parent dir under the
    // configured root.
    let root_dir = options.include_root.clone().or_else(|| {
        include_stack
            .first()
            .and_then(|p| p.parent().map(Path::to_path_buf))
    });
    let Some(root_dir) = root_dir else {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_ROOT_REQUIRED",
            "!include_many from stdin requires include_root option",
        ));
    };
    let root_canon = root_dir.canonicalize().map_err(|e| {
        Diagnostic::error_code(
            "E_INCLUDE_ROOT_INVALID",
            format!(
                "failed to access include root '{}': {e}",
                root_dir.display()
            ),
        )
    })?;
    let base_dir = include_stack
        .last()
        .and_then(|curr| curr.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| root_canon.clone());
    let resolved_parent = normalize_path(base_dir.join(parent));
    let resolved_parent_canon = resolved_parent.canonicalize().map_err(|e| {
        Diagnostic::error_code(
            "E_INCLUDE_READ",
            format!(
                "failed to read include glob parent '{}': {e}",
                resolved_parent.display()
            ),
        )
    })?;
    if !resolved_parent_canon.starts_with(&root_canon) {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_ESCAPE",
            format!(
                "include path escapes include root: '{}' resolves outside '{}'",
                dir_probe.display(),
                root_canon.display()
            ),
        ));
    }

    let mut matches: Vec<PathBuf> = Vec::new();
    for entry in fs::read_dir(&resolved_parent_canon).map_err(|e| {
        Diagnostic::error_code(
            "E_INCLUDE_READ",
            format!(
                "failed to read include glob dir '{}': {e}",
                resolved_parent_canon.display()
            ),
        )
    })? {
        let entry = entry.map_err(|e| {
            Diagnostic::error_code(
                "E_INCLUDE_READ",
                format!("failed to enumerate include glob entry: {e}"),
            )
        })?;
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        let name_os = entry.file_name();
        let Some(name) = name_os.to_str() else {
            continue;
        };
        if glob_matches(pattern, name) {
            matches.push(entry.path());
        }
    }
    // Deterministic ordering — alphabetical.
    matches.sort();

    for resolved in matches {
        let resolved = resolved.canonicalize().map_err(|e| {
            Diagnostic::error_code(
                "E_INCLUDE_READ",
                format!("failed to read include '{}': {e}", resolved.display()),
            )
        })?;
        if !resolved.starts_with(&root_canon) {
            continue;
        }
        if include_stack.iter().any(|p| p == &resolved) {
            continue;
        }
        let content = fs::read_to_string(&resolved).map_err(|e| {
            Diagnostic::error_code(
                "E_INCLUDE_READ",
                format!("failed to read include '{}': {e}", resolved.display()),
            )
        })?;
        include_stack.push(resolved);
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
        include_stack.pop();
    }
    Ok(())
}

/// Minimal `*`/`?` glob match — sufficient for `!include_many` filename
/// patterns. Backtracks on `*` to keep behaviour predictable. No character
/// classes, no recursion across path separators.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn glob_matches(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    fn rec(p: &[char], n: &[char]) -> bool {
        let mut pi = 0;
        let mut ni = 0;
        let mut star: Option<(usize, usize)> = None;
        while ni < n.len() {
            if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
                pi += 1;
                ni += 1;
            } else if pi < p.len() && p[pi] == '*' {
                star = Some((pi, ni));
                pi += 1;
            } else if let Some((sp, sn)) = star {
                pi = sp + 1;
                ni = sn + 1;
                star = Some((sp, sn + 1));
            } else {
                return false;
            }
        }
        while pi < p.len() && p[pi] == '*' {
            pi += 1;
        }
        pi == p.len()
    }
    rec(&p, &n)
}

/// Handle `!include <Library/Module>` by resolving the path through the stdlib root.
/// The angle-bracket form is a stdlib reference; it is always treated as include-once.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
fn process_stdlib_angle_include(
    raw_target: &str,
    directive_name: &str,
    _include_once: bool,
    options: &ParseOptions,
    state: &mut PreprocState,
    include_stack: &mut Vec<PathBuf>,
    include_once_seen: &mut BTreeSet<PathBuf>,
    depth: usize,
    call_depth: usize,
    out: &mut String,
) -> Result<(), Diagnostic> {
    let inner = raw_target
        .trim()
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim();
    if inner.is_empty() {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_PATH_REQUIRED",
            format!("{directive_name} angle-bracket stdlib target cannot be empty"),
        ));
    }

    let mut path = PathBuf::from(inner);
    if path.extension().is_none() {
        path.set_extension("puml");
    }

    if path.is_absolute() {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_ABSOLUTE_PATH",
            format!(
                "{directive_name} angle-bracket targets must be relative stdlib paths: {}",
                path.display()
            ),
        ));
    }

    let Some(stdlib_root) = resolve_stdlib_root_for_angle_include(options, include_stack) else {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_STDLIB_ROOT_REQUIRED",
            format!(
                "{directive_name} <{inner}> requires a stdlib root; set PUML_STDLIB_ROOT, pass \
                 --include-root pointing to a directory with a `stdlib/` sibling, or place the \
                 input file next to a `stdlib/` directory"
            ),
        ));
    };

    let resolved = resolve_import_path(&stdlib_root, &path)?;

    // Angle-bracket includes are always treated as include-once (stdlib files are idempotent).
    if !include_once_seen.insert(resolved.clone()) {
        return Ok(());
    }

    if include_stack.iter().any(|p| p == &resolved) {
        let mut cycle = include_stack
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>();
        cycle.push(resolved.display().to_string());
        return Err(Diagnostic::error_code(
            "E_INCLUDE_CYCLE",
            format!("include cycle detected: {}", cycle.join(" -> ")),
        ));
    }

    let content = fs::read_to_string(&resolved).map_err(|e| {
        Diagnostic::error_code(
            "E_INCLUDE_READ",
            format!(
                "failed to read stdlib include '{}': {e}",
                resolved.display()
            ),
        )
    })?;

    include_stack.push(resolved);
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
    include_stack.pop();
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
fn process_import_directive(
    raw_target: &str,
    options: &ParseOptions,
    state: &mut PreprocState,
    include_stack: &mut Vec<PathBuf>,
    include_once_seen: &mut BTreeSet<PathBuf>,
    depth: usize,
    call_depth: usize,
    out: &mut String,
) -> Result<(), Diagnostic> {
    if raw_target.trim().is_empty() {
        return Err(Diagnostic::error_code(
            "E_IMPORT_PATH_REQUIRED",
            "!import requires a stdlib module path",
        ));
    }
    if is_url_include_target(raw_target) {
        if !options.allow_url_includes {
            return Err(Diagnostic::error_code(
                "E_INCLUDE_URL_DISABLED",
                format!("!import URL includes are disabled: {}", raw_target),
            ));
        }
        let url = extract_url(raw_target);
        let content = fetch_url_include(url)?;
        return preprocess_text(
            &content,
            options,
            state,
            include_stack,
            include_once_seen,
            depth + 1,
            call_depth,
            out,
        );
    }

    let target = parse_import_target(raw_target)?;
    if target.is_absolute() {
        return Err(Diagnostic::error_code(
            "E_IMPORT_ABSOLUTE_PATH",
            format!(
                "!import only supports relative stdlib paths: {}",
                target.display()
            ),
        ));
    }

    let stdlib_root = resolve_stdlib_root(options, include_stack)?;
    let resolved = resolve_import_path(&stdlib_root, &target)?;
    if !include_once_seen.insert(resolved.clone()) {
        return Ok(());
    }
    if include_stack.iter().any(|p| p == &resolved) {
        let mut cycle = include_stack
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>();
        cycle.push(resolved.display().to_string());
        return Err(Diagnostic::error_code(
            "E_IMPORT_CYCLE",
            format!("import cycle detected: {}", cycle.join(" -> ")),
        ));
    }

    let content = fs::read_to_string(&resolved).map_err(|e| {
        Diagnostic::error_code(
            "E_IMPORT_READ",
            format!("failed to read import '{}': {e}", resolved.display()),
        )
    })?;
    include_stack.push(resolved);
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
    include_stack.pop();
    Ok(())
}

fn parse_preprocess_directive(line: &str) -> Option<PreprocessDirective> {
    let trimmed = line.trim();
    if trimmed
        .to_ascii_lowercase()
        .starts_with("%invoke_procedure(")
        || trimmed.to_ascii_lowercase().starts_with("%call_user_func(")
    {
        return Some(PreprocessDirective::DynamicInvocation(trimmed.to_string()));
    }
    if !trimmed.starts_with('!') {
        return None;
    }
    let rest = trimmed[1..].trim_start();
    let mut split = rest.splitn(2, char::is_whitespace);
    let name = split.next().unwrap_or_default();
    let arg = split.next().unwrap_or_default().trim();
    let lower = name.to_ascii_lowercase();

    match lower.as_str() {
        "define" => Some(PreprocessDirective::Define(arg.to_string())),
        "undef" => Some(PreprocessDirective::Undef(arg.to_string())),
        "include" => Some(PreprocessDirective::Include(arg.to_string())),
        "include_once" => Some(PreprocessDirective::IncludeOnce(arg.to_string())),
        "include_many" => Some(PreprocessDirective::IncludeMany(arg.to_string())),
        "includesub" => Some(PreprocessDirective::IncludeSub(arg.to_string())),
        "includeurl" => Some(PreprocessDirective::IncludeUrl(arg.to_string())),
        "import" => Some(PreprocessDirective::Import(arg.to_string())),
        "if" => Some(PreprocessDirective::If(arg.to_string())),
        "ifdef" => Some(PreprocessDirective::IfDef {
            name: arg.to_string(),
            negated: false,
        }),
        "ifndef" => Some(PreprocessDirective::IfDef {
            name: arg.to_string(),
            negated: true,
        }),
        "elseif" => Some(PreprocessDirective::ElseIf(arg.to_string())),
        "else" => Some(PreprocessDirective::Else),
        "endif" => Some(PreprocessDirective::EndIf),
        "while" => Some(PreprocessDirective::While(arg.to_string())),
        "foreach" => Some(PreprocessDirective::Foreach(arg.to_string())),
        "endfor" => Some(PreprocessDirective::EndFor),
        "endwhile" => Some(PreprocessDirective::EndWhile),
        "break" => Some(PreprocessDirective::Break),
        "continue" => Some(PreprocessDirective::Continue),
        "function" => Some(PreprocessDirective::Function),
        "endfunction" => Some(PreprocessDirective::EndFunction),
        "procedure" => Some(PreprocessDirective::Procedure),
        "endprocedure" => Some(PreprocessDirective::EndProcedure),
        "assert" => Some(PreprocessDirective::Assert(arg.to_string())),
        "log" => Some(PreprocessDirective::Log(arg.to_string())),
        "dump_memory" => Some(PreprocessDirective::DumpMemory(arg.to_string())),
        "option" => Some(PreprocessDirective::Passthrough(trimmed.to_string())),
        "local" => parse_scoped_variable_assignment(arg, trimmed, PreprocVariableScope::Local),
        "global" => parse_scoped_variable_assignment(arg, trimmed, PreprocVariableScope::Global),
        _ if let Some((call_name, call_args)) = parse_named_call(rest) => {
            Some(PreprocessDirective::ProcedureCall {
                name: call_name,
                args: call_args,
            })
        }
        _ if name.starts_with('$') => parse_variable_assignment(name, arg, trimmed),
        "return" => Some(PreprocessDirective::Unsupported(name.to_string())),
        // `!startsub` / `!endsub` are markers used by `!includesub`. When a
        // file containing them is included directly, we silently elide the
        // marker lines and pass the body lines through.
        "startsub" | "endsub" => Some(PreprocessDirective::NoOp),
        "theme" | "pragma" => None,
        _ if !name.is_empty() => Some(PreprocessDirective::Unsupported(name.to_string())),
        _ => None,
    }
}

fn evaluate_preprocess_expr(expr: &str, state: &PreprocState) -> Result<bool, Diagnostic> {
    let raw = expr.trim();
    if raw.is_empty() {
        return Err(Diagnostic::error_code(
            "E_PREPROC_EXPR_REQUIRED",
            "preprocessor condition requires an expression",
        ));
    }
    // Compound boolean: split top-level || then && and recurse on each half
    if let Some((lhs, rhs)) = split_top_level(raw, "||") {
        return Ok(evaluate_preprocess_expr(&lhs, state)? || evaluate_preprocess_expr(&rhs, state)?);
    }
    if let Some((lhs, rhs)) = split_top_level_word(raw, "or") {
        return Ok(evaluate_preprocess_expr(&lhs, state)? || evaluate_preprocess_expr(&rhs, state)?);
    }
    if let Some((lhs, rhs)) = split_top_level(raw, "&&") {
        return Ok(evaluate_preprocess_expr(&lhs, state)? && evaluate_preprocess_expr(&rhs, state)?);
    }
    if let Some((lhs, rhs)) = split_top_level_word(raw, "and") {
        return Ok(evaluate_preprocess_expr(&lhs, state)? && evaluate_preprocess_expr(&rhs, state)?);
    }
    if let Some((lhs, rhs)) = split_top_level_word(raw, "xor") {
        return Ok(evaluate_preprocess_expr(&lhs, state)? ^ evaluate_preprocess_expr(&rhs, state)?);
    }

    if let Some((negated, name)) = parse_defined_call(raw) {
        let defined = state.defines.contains_key(name) || state.vars.contains_key(name);
        return Ok(if negated { !defined } else { defined });
    }

    let substituted = expand_preprocessor_text(raw, state, 0)?;
    evaluate_scalar_expr(substituted.trim())
}

fn parse_defined_call(expr: &str) -> Option<(bool, &str)> {
    let trimmed = expr.trim();
    let (negated, rest) = if let Some(rem) = trimmed.strip_prefix('!') {
        (true, rem.trim_start())
    } else {
        (false, trimmed)
    };
    let lower = rest.to_ascii_lowercase();
    if !lower.starts_with("defined") {
        return None;
    }
    let rest = &rest["defined".len()..];
    let name = rest
        .trim_start()
        .strip_prefix('(')?
        .strip_suffix(')')?
        .trim();
    if name.is_empty() {
        return None;
    }
    Some((negated, name))
}

fn evaluate_scalar_expr(expr: &str) -> Result<bool, Diagnostic> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return Ok(false);
    }

    if let Some(inner) = strip_outer_balanced_parens(trimmed) {
        return evaluate_scalar_expr(inner);
    }

    // Compound boolean: try top-level || then && (split outside quotes/parens)
    if let Some((lhs, rhs)) = split_top_level(trimmed, "||") {
        return Ok(evaluate_scalar_expr(&lhs)? || evaluate_scalar_expr(&rhs)?);
    }
    if let Some((lhs, rhs)) = split_top_level_word(trimmed, "or") {
        return Ok(evaluate_scalar_expr(&lhs)? || evaluate_scalar_expr(&rhs)?);
    }
    if let Some((lhs, rhs)) = split_top_level(trimmed, "&&") {
        return Ok(evaluate_scalar_expr(&lhs)? && evaluate_scalar_expr(&rhs)?);
    }
    if let Some((lhs, rhs)) = split_top_level_word(trimmed, "and") {
        return Ok(evaluate_scalar_expr(&lhs)? && evaluate_scalar_expr(&rhs)?);
    }
    if let Some((lhs, rhs)) = split_top_level_word(trimmed, "xor") {
        return Ok(evaluate_scalar_expr(&lhs)? ^ evaluate_scalar_expr(&rhs)?);
    }

    let lower_trimmed = trimmed.to_ascii_lowercase();
    if lower_trimmed.starts_with("not ") {
        return evaluate_scalar_expr(trimmed[3..].trim_start()).map(|v| !v);
    }
    if let Some((lhs, rhs)) = split_top_level(trimmed, "==") {
        return Ok(normalize_expr_value(&lhs) == normalize_expr_value(&rhs));
    }
    if let Some((lhs, rhs)) = split_top_level(trimmed, "!=") {
        return Ok(normalize_expr_value(&lhs) != normalize_expr_value(&rhs));
    }
    if let Some((lhs, rhs)) = split_top_level(trimmed, "<>") {
        return Ok(normalize_expr_value(&lhs) != normalize_expr_value(&rhs));
    }
    // Numeric comparisons: check two-char operators before one-char to avoid splitting <=/>= wrong.
    if let Some((lhs, rhs)) = split_top_level(trimmed, "<=") {
        let a = normalize_expr_value(&lhs)
            .parse::<i64>()
            .unwrap_or(i64::MIN);
        let b = normalize_expr_value(&rhs)
            .parse::<i64>()
            .unwrap_or(i64::MAX);
        return Ok(a <= b);
    }
    if let Some((lhs, rhs)) = split_top_level(trimmed, ">=") {
        let a = normalize_expr_value(&lhs)
            .parse::<i64>()
            .unwrap_or(i64::MAX);
        let b = normalize_expr_value(&rhs)
            .parse::<i64>()
            .unwrap_or(i64::MIN);
        return Ok(a >= b);
    }
    if let Some((lhs, rhs)) = split_top_level(trimmed, "<") {
        let a = normalize_expr_value(&lhs)
            .parse::<i64>()
            .unwrap_or(i64::MIN);
        let b = normalize_expr_value(&rhs)
            .parse::<i64>()
            .unwrap_or(i64::MAX);
        return Ok(a < b);
    }
    if let Some((lhs, rhs)) = split_top_level(trimmed, ">") {
        let a = normalize_expr_value(&lhs)
            .parse::<i64>()
            .unwrap_or(i64::MAX);
        let b = normalize_expr_value(&rhs)
            .parse::<i64>()
            .unwrap_or(i64::MIN);
        return Ok(a > b);
    }
    if let Some(inner) = trimmed.strip_prefix('!') {
        return evaluate_scalar_expr(inner).map(|v| !v);
    }
    if trimmed.contains('(') || trimmed.contains(')') {
        return Err(Diagnostic::error_code(
            "E_PREPROC_EXPR_UNSUPPORTED",
            "only simple conditions are supported in this preprocessor slice",
        ));
    }

    let normalized = normalize_expr_value(trimmed);
    if normalized.is_empty() {
        return Ok(false);
    }
    match normalized.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => return Ok(true),
        "false" | "no" | "off" => return Ok(false),
        _ => {}
    }
    if let Ok(n) = normalized.parse::<i64>() {
        return Ok(n != 0);
    }
    Ok(false)
}

fn strip_outer_balanced_parens(expr: &str) -> Option<&str> {
    if !expr.starts_with('(') || !expr.ends_with(')') {
        return None;
    }
    let bytes = expr.as_bytes();
    let mut depth: i32 = 0;
    let mut in_str = false;
    for (idx, b) in bytes.iter().enumerate() {
        if in_str {
            if *b == b'"' {
                in_str = false;
            }
            continue;
        }
        match *b {
            b'"' => in_str = true,
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
                if depth == 0 && idx != bytes.len() - 1 {
                    return None;
                }
            }
            _ => {}
        }
    }
    if depth == 0 {
        Some(expr[1..expr.len() - 1].trim())
    } else {
        None
    }
}

/// Split `expr` on the first top-level occurrence of `sep`, respecting
/// parentheses depth and double-quoted strings. Returns None if `sep`
/// is absent at depth zero (which keeps short-circuit chains correct).
fn split_top_level(expr: &str, sep: &str) -> Option<(String, String)> {
    let bytes = expr.as_bytes();
    let sep_bytes = sep.as_bytes();
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut i = 0;
    while i + sep_bytes.len() <= bytes.len() {
        let b = bytes[i];
        if in_str {
            if b == b'"' {
                in_str = false;
            }
        } else {
            match b {
                b'"' => in_str = true,
                b'(' => depth += 1,
                b')' => depth = depth.saturating_sub(1),
                _ => {}
            }
            if depth == 0 && bytes[i..].starts_with(sep_bytes) {
                let lhs = expr[..i].trim().to_string();
                let rhs = expr[i + sep_bytes.len()..].trim().to_string();
                if !lhs.is_empty() && !rhs.is_empty() {
                    return Some((lhs, rhs));
                }
            }
        }
        i += 1;
    }
    None
}

fn split_top_level_word(expr: &str, sep: &str) -> Option<(String, String)> {
    let sep_lower = sep.to_ascii_lowercase();
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut token_start: Option<usize> = None;
    for (idx, ch) in expr.char_indices() {
        if in_str {
            if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => {
                if let Some(start) = token_start.take() {
                    if is_top_level_word_match(expr, start, idx, &sep_lower, depth) {
                        return split_word_at(expr, start, idx);
                    }
                }
                in_str = true;
            }
            '(' => {
                if let Some(start) = token_start.take() {
                    if is_top_level_word_match(expr, start, idx, &sep_lower, depth) {
                        return split_word_at(expr, start, idx);
                    }
                }
                depth += 1;
            }
            ')' => {
                if let Some(start) = token_start.take() {
                    if is_top_level_word_match(expr, start, idx, &sep_lower, depth) {
                        return split_word_at(expr, start, idx);
                    }
                }
                depth = depth.saturating_sub(1);
            }
            c if c.is_whitespace() => {
                if let Some(start) = token_start.take() {
                    if is_top_level_word_match(expr, start, idx, &sep_lower, depth) {
                        return split_word_at(expr, start, idx);
                    }
                }
            }
            _ => {
                if token_start.is_none() {
                    token_start = Some(idx);
                }
            }
        }
    }
    if let Some(start) = token_start {
        if is_top_level_word_match(expr, start, expr.len(), &sep_lower, depth) {
            return split_word_at(expr, start, expr.len());
        }
    }
    None
}

fn is_top_level_word_match(
    expr: &str,
    start: usize,
    end: usize,
    sep_lower: &str,
    depth: i32,
) -> bool {
    depth == 0 && expr[start..end].eq_ignore_ascii_case(sep_lower)
}

fn split_word_at(expr: &str, start: usize, end: usize) -> Option<(String, String)> {
    let lhs = expr[..start].trim().to_string();
    let rhs = expr[end..].trim().to_string();
    if lhs.is_empty() || rhs.is_empty() {
        None
    } else {
        Some((lhs, rhs))
    }
}

fn normalize_expr_value(value: &str) -> String {
    let normalized = value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string();
    if let Some(n) = eval_int_expr(&normalized) {
        n.to_string()
    } else {
        normalized
    }
}

/// Evaluate a simple two-operand integer arithmetic expression such as "3 + 1"
/// or "5 - 2".  Only +, -, *, / operators on integer literals are handled.
/// Returns `None` if the expression is not in this form (non-integer values or
/// more complex expressions), so the caller can fall back to the raw string.
fn eval_simple_arithmetic(expr: &str) -> Option<i64> {
    eval_int_expr(expr)
}

fn eval_int_expr(expr: &str) -> Option<i64> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some(inner) = strip_outer_balanced_parens(trimmed) {
        return eval_int_expr(inner);
    }
    if let Some((lhs, op, rhs)) = split_top_level_arithmetic(trimmed, &['+', '-']) {
        let a = eval_int_expr(lhs)?;
        let b = eval_int_expr(rhs)?;
        return Some(if op == '+' { a + b } else { a - b });
    }
    if let Some((lhs, op, rhs)) = split_top_level_arithmetic(trimmed, &['*', '/', '%']) {
        let a = eval_int_expr(lhs)?;
        let b = eval_int_expr(rhs)?;
        return match op {
            '*' => Some(a * b),
            '/' if b != 0 => Some(a / b),
            '%' if b != 0 => Some(a % b),
            _ => None,
        };
    }
    trimmed.parse::<i64>().ok()
}

fn split_top_level_arithmetic<'a>(expr: &'a str, ops: &[char]) -> Option<(&'a str, char, &'a str)> {
    let mut depth = 0i32;
    let mut in_str = false;
    let mut last = None;
    for (idx, ch) in expr.char_indices() {
        if in_str {
            if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 && ops.contains(&ch) => {
                if ch == '-' {
                    let prev = expr[..idx].chars().rev().find(|c| !c.is_whitespace());
                    if prev.is_none() || matches!(prev, Some('(' | '+' | '-' | '*' | '/' | '%')) {
                        continue;
                    }
                }
                last = Some((idx, ch));
            }
            _ => {}
        }
    }
    let (idx, op) = last?;
    let lhs = expr[..idx].trim();
    let rhs = expr[idx + op.len_utf8()..].trim();
    if lhs.is_empty() || rhs.is_empty() {
        None
    } else {
        Some((lhs, op, rhs))
    }
}

fn find_matching_endwhile(lines: &[&str], while_idx: usize) -> Result<usize, Diagnostic> {
    let mut depth = 0usize;
    for (idx, raw) in lines.iter().enumerate().skip(while_idx + 1) {
        let line = raw.trim();
        match parse_preprocess_directive(line) {
            Some(PreprocessDirective::While(_)) => depth += 1,
            Some(PreprocessDirective::EndWhile) => {
                if depth == 0 {
                    return Ok(idx);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    Err(Diagnostic::error_code(
        "E_PREPROC_WHILE_UNCLOSED",
        "missing `!endwhile` for `!while` block",
    ))
}

fn find_matching_endfor(lines: &[&str], foreach_idx: usize) -> Result<usize, Diagnostic> {
    let mut depth = 0usize;
    for (idx, raw) in lines.iter().enumerate().skip(foreach_idx + 1) {
        let line = raw.trim();
        match parse_preprocess_directive(line) {
            Some(PreprocessDirective::Foreach(_)) => depth += 1,
            Some(PreprocessDirective::EndFor) => {
                if depth == 0 {
                    return Ok(idx);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    Err(Diagnostic::error_code(
        "E_PREPROC_FOREACH_UNCLOSED",
        "missing `!endfor` for `!foreach` block",
    ))
}

fn consume_preprocessor_block(
    lines: &[&str],
    start_idx: usize,
    start_directive: PreprocessDirective,
    end_directive: PreprocessDirective,
    error_code: &str,
    end_name: &str,
) -> Result<usize, Diagnostic> {
    let mut depth = 0usize;
    for (idx, raw) in lines.iter().enumerate().skip(start_idx + 1) {
        let line = raw.trim();
        if let Some(directive) = parse_preprocess_directive(line) {
            if std::mem::discriminant(&directive) == std::mem::discriminant(&start_directive) {
                depth += 1;
            } else if std::mem::discriminant(&directive) == std::mem::discriminant(&end_directive) {
                if depth == 0 {
                    return Ok(idx);
                }
                depth -= 1;
            }
        }
    }
    Err(Diagnostic::error_code(
        error_code,
        format!("missing `{end_name}` for preprocessor block"),
    ))
}

fn evaluate_assert_expression(body: &str, state: &PreprocState) -> Result<bool, Diagnostic> {
    let expression = body.split_once(':').map_or(body, |(expr, _)| expr).trim();
    if expression.is_empty() {
        return Err(Diagnostic::error_code(
            "E_PREPROC_ASSERT_EXPR_REQUIRED",
            "!assert requires a non-empty expression before optional `:` message",
        ));
    }
    evaluate_preprocess_expr(expression, state)
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_include_path(
    options: &ParseOptions,
    include_stack: &[PathBuf],
    include_path: &Path,
) -> Result<PathBuf, Diagnostic> {
    let root_dir = options.include_root.clone().or_else(|| {
        include_stack
            .first()
            .and_then(|p| p.parent().map(Path::to_path_buf))
    });

    let Some(root_dir) = root_dir else {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_ROOT_REQUIRED",
            "!include from stdin requires include_root option",
        ));
    };

    let root_canon = root_dir.canonicalize().map_err(|e| {
        Diagnostic::error_code(
            "E_INCLUDE_ROOT_INVALID",
            format!(
                "failed to access include root '{}': {e}",
                root_dir.display()
            ),
        )
    })?;

    let base_dir = include_stack
        .last()
        .and_then(|curr| curr.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| root_canon.clone());
    let resolved = normalize_path(base_dir.join(include_path));
    let resolved_canon = resolved.canonicalize().map_err(|e| {
        Diagnostic::error_code(
            "E_INCLUDE_READ",
            format!("failed to read include '{}': {e}", resolved.display()),
        )
    })?;

    if !resolved_canon.starts_with(&root_canon) {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_ESCAPE",
            format!(
                "include path escapes include root: '{}' resolves outside '{}'",
                include_path.display(),
                root_canon.display()
            ),
        ));
    }

    Ok(resolved_canon)
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn parse_include_target(raw_target: &str) -> IncludeTarget {
    let trimmed = raw_target.trim();
    let unwrapped = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| trimmed.strip_prefix('<').and_then(|s| s.strip_suffix('>')))
        .unwrap_or(trimmed);
    let (path, tag) = if unwrapped.contains("://") {
        (unwrapped, None)
    } else if let Some((path, tag)) = unwrapped.rsplit_once('!') {
        let clean_tag = tag.trim();
        if path.trim().is_empty() || clean_tag.is_empty() {
            (unwrapped, None)
        } else {
            (path.trim(), Some(clean_tag.to_string()))
        }
    } else {
        (unwrapped, None)
    };

    IncludeTarget {
        path: PathBuf::from(path),
        tag,
    }
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn parse_import_target(raw_target: &str) -> Result<PathBuf, Diagnostic> {
    let trimmed = raw_target.trim();
    let unwrapped = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| trimmed.strip_prefix('<').and_then(|s| s.strip_suffix('>')))
        .unwrap_or(trimmed)
        .trim();
    if unwrapped.is_empty() {
        return Err(Diagnostic::error_code(
            "E_IMPORT_PATH_REQUIRED",
            "!import requires a stdlib module path",
        ));
    }
    if unwrapped.contains('!') {
        return Err(Diagnostic::error_code(
            "E_IMPORT_INVALID_FORM",
            format!("!import does not support tag selection (`path!TAG`): {raw_target}"),
        ));
    }

    let mut path = PathBuf::from(unwrapped);
    if path.extension().is_none() {
        path.set_extension("puml");
    }
    Ok(path)
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn is_url_include_target(raw_target: &str) -> bool {
    let trimmed = raw_target
        .trim()
        .trim_matches('"')
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim();
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("file://")
}

/// Extract the canonical URL string from a raw include target (strips quotes/angle brackets).
#[cfg_attr(
    not(all(not(target_arch = "wasm32"), feature = "url-includes")),
    allow(dead_code)
)]
fn extract_url(raw_target: &str) -> &str {
    raw_target
        .trim()
        .trim_matches('"')
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim()
}

/// Resolve the on-disk cache path for a URL include.
/// Uses `~/.cache/puml/includes/<sha256-of-url>`.
#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
fn url_cache_path(url: &str) -> Option<std::path::PathBuf> {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    let hash = hex::encode(hasher.finalize());

    let cache_base = std::env::var_os("XDG_CACHE_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".cache")))?;

    Some(cache_base.join("puml").join("includes").join(hash))
}

/// Fetch a URL include, using a local disk cache keyed by SHA-256 of the URL.
/// Returns the fetched content as a string.
/// Handles `file://` URLs by reading from the local filesystem directly.
#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
fn fetch_url_include(url: &str) -> Result<String, Diagnostic> {
    // Handle file:// URLs by stripping the scheme and reading from the local fs.
    if url.to_ascii_lowercase().starts_with("file://") {
        let path_str = &url["file://".len()..];
        return fs::read_to_string(path_str).map_err(|e| {
            Diagnostic::error_code(
                "E_INCLUDE_URL_FETCH",
                format!("failed to read file URL '{}': {e}", url),
            )
        });
    }

    // Check cache first.
    if let Some(cache_path) = url_cache_path(url) {
        if cache_path.exists() {
            return fs::read_to_string(&cache_path).map_err(|e| {
                Diagnostic::error_code(
                    "E_INCLUDE_URL_CACHE_READ",
                    format!("failed to read cache for '{}': {e}", url),
                )
            });
        }

        // Fetch via HTTP(S).
        let response = ureq::get(url).call().map_err(|e| {
            Diagnostic::error_code(
                "E_INCLUDE_URL_FETCH",
                format!("failed to fetch '{}': {e}", url),
            )
        })?;

        if response.status() < 200 || response.status() >= 300 {
            return Err(Diagnostic::error_code(
                "E_INCLUDE_URL_FETCH",
                format!(
                    "HTTP {} fetching '{}': {}",
                    response.status(),
                    url,
                    response.status_text()
                ),
            ));
        }

        let content = response.into_string().map_err(|e| {
            Diagnostic::error_code(
                "E_INCLUDE_URL_FETCH",
                format!("failed to read response body from '{}': {e}", url),
            )
        })?;

        // Write to cache (best-effort; failures are non-fatal).
        if let Some(parent) = cache_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&cache_path, &content);

        Ok(content)
    } else {
        // No cache path available; fetch directly without caching.
        let response = ureq::get(url).call().map_err(|e| {
            Diagnostic::error_code(
                "E_INCLUDE_URL_FETCH",
                format!("failed to fetch '{}': {e}", url),
            )
        })?;

        if response.status() < 200 || response.status() >= 300 {
            return Err(Diagnostic::error_code(
                "E_INCLUDE_URL_FETCH",
                format!(
                    "HTTP {} fetching '{}': {}",
                    response.status(),
                    url,
                    response.status_text()
                ),
            ));
        }

        response.into_string().map_err(|e| {
            Diagnostic::error_code(
                "E_INCLUDE_URL_FETCH",
                format!("failed to read response body from '{}': {e}", url),
            )
        })
    }
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "url-includes")))]
#[allow(dead_code)]
fn fetch_url_include(url: &str) -> Result<String, Diagnostic> {
    Err(Diagnostic::error_code(
        "E_INCLUDE_URL_UNSUPPORTED",
        format!("URL includes are not supported in this build: {url}"),
    ))
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_stdlib_root(
    options: &ParseOptions,
    include_stack: &[PathBuf],
) -> Result<PathBuf, Diagnostic> {
    let candidate = options
        .include_root
        .clone()
        .map(|r| r.join("stdlib"))
        .or_else(|| {
            include_stack
                .first()
                .and_then(|p| p.parent().map(|dir| dir.join("stdlib")))
        });

    let Some(root) = candidate else {
        return Err(Diagnostic::error_code(
            "E_IMPORT_ROOT_REQUIRED",
            "!import requires a stdlib root; pass --include-root or use file input with a sibling `stdlib/` directory",
        ));
    };

    let root_canon = root.canonicalize().map_err(|e| {
        Diagnostic::error_code(
            "E_IMPORT_STDLIB_ROOT_INVALID",
            format!("failed to access stdlib root '{}': {e}", root.display()),
        )
    })?;

    if !root_canon.is_dir() {
        return Err(Diagnostic::error_code(
            "E_IMPORT_STDLIB_ROOT_INVALID",
            format!("stdlib root is not a directory: '{}'", root_canon.display()),
        ));
    }
    Ok(root_canon)
}

/// Resolve the stdlib root for angle-bracket `!include <Library/Module>` references.
///
/// Search order:
/// 1. `PUML_STDLIB_ROOT` environment variable override.
/// 2. `include_root/stdlib/` from `ParseOptions`.
/// 3. `stdlib/` adjacent to the first file on the include stack.
/// 4. `CARGO_MANIFEST_DIR/stdlib/` at compile time (dev and test builds).
#[cfg(not(target_arch = "wasm32"))]
fn resolve_stdlib_root_for_angle_include(
    options: &ParseOptions,
    include_stack: &[PathBuf],
) -> Option<PathBuf> {
    // Priority 1: PUML_STDLIB_ROOT env var override.
    if let Ok(env_root) = std::env::var("PUML_STDLIB_ROOT") {
        let root = PathBuf::from(env_root);
        if let Ok(canon) = root.canonicalize() {
            if canon.is_dir() {
                return Some(canon);
            }
        }
    }

    // Priority 2: stdlib/ sibling to the include_root option.
    if let Some(ref root) = options.include_root {
        let candidate = root.join("stdlib");
        if let Ok(canon) = candidate.canonicalize() {
            if canon.is_dir() {
                return Some(canon);
            }
        }
    }

    // Priority 3: stdlib/ sibling to the first file on the include stack.
    if let Some(first) = include_stack.first() {
        if let Some(parent) = first.parent() {
            let candidate = parent.join("stdlib");
            if let Ok(canon) = candidate.canonicalize() {
                if canon.is_dir() {
                    return Some(canon);
                }
            }
        }
    }

    // Priority 4: CARGO_MANIFEST_DIR/stdlib/ compiled in (dev/test builds only).
    if let Some(manifest_dir) = option_env!("CARGO_MANIFEST_DIR") {
        let candidate = PathBuf::from(manifest_dir).join("stdlib");
        if let Ok(canon) = candidate.canonicalize() {
            if canon.is_dir() {
                return Some(canon);
            }
        }
    }

    None
}

/// Returns true when the raw include target is an angle-bracket stdlib reference
/// such as `<C4/C4_Context>` or `<awslib14/Compute/EC2>`.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn is_angle_bracket_include(raw_target: &str) -> bool {
    let trimmed = raw_target.trim();
    trimmed.starts_with('<') && trimmed.ends_with('>')
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_import_path(stdlib_root: &Path, import_path: &Path) -> Result<PathBuf, Diagnostic> {
    let resolved = normalize_path(stdlib_root.join(import_path));
    if !resolved.starts_with(stdlib_root) {
        return Err(Diagnostic::error_code(
            "E_IMPORT_ESCAPE",
            format!(
                "import path escapes stdlib root: '{}' resolves outside '{}'",
                import_path.display(),
                stdlib_root.display()
            ),
        ));
    }
    let resolved_canon = resolved.canonicalize().map_err(|e| {
        Diagnostic::error_code(
            "E_IMPORT_STDLIB_NOT_FOUND",
            format!("stdlib import not found '{}': {e}", import_path.display()),
        )
    })?;
    if !resolved_canon.starts_with(stdlib_root) {
        return Err(Diagnostic::error_code(
            "E_IMPORT_ESCAPE",
            format!(
                "import path escapes stdlib root: '{}' resolves outside '{}'",
                import_path.display(),
                stdlib_root.display()
            ),
        ));
    }
    Ok(resolved_canon)
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn extract_include_tag(content: &str, tag: &str) -> Option<String> {
    let mut collecting = false;
    let mut lines = Vec::new();
    let tag_lower = tag.to_ascii_lowercase();

    for raw_line in content.lines() {
        let line = raw_line.trim();
        let lower = line.to_ascii_lowercase();

        if lower.starts_with("!startsub") {
            let candidate = line[9..].trim().to_ascii_lowercase();
            if candidate == tag_lower {
                collecting = true;
            }
            continue;
        }

        if lower.starts_with("!endsub") {
            if collecting {
                return Some(lines.join("\n"));
            }
            continue;
        }

        if collecting {
            lines.push(raw_line);
        }
    }

    None
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn normalize_path(path: PathBuf) -> PathBuf {
    let mut parts = Vec::new();
    let is_abs = path.is_absolute();

    for comp in path.components() {
        use std::path::Component;
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if parts
                    .last()
                    .is_some_and(|c: &Component<'_>| !matches!(c, Component::ParentDir))
                {
                    parts.pop();
                } else if !is_abs {
                    parts.push(comp);
                }
            }
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => parts.push(comp),
        }
    }

    let mut out = PathBuf::new();
    for c in parts {
        out.push(c.as_os_str());
    }
    out
}

fn parse_macro_define(body: &str) -> Result<Option<(String, PreprocMacro)>, Diagnostic> {
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
    let params = parse_params(&params_raw)?;
    Ok(Some((
        name.to_string(),
        PreprocMacro {
            params,
            body: rest.to_string(),
        },
    )))
}

fn substitute_defines(
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

fn expand_macro_body(mac: &PreprocMacro, args: &[String]) -> String {
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

fn substitute_macro_params(body: &str, bindings: &BTreeMap<String, String>) -> String {
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

fn substitute_vars(line: &str, vars: &BTreeMap<String, String>) -> String {
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
                    out.push_str(&get_json_attribute(value, &json_path));
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

fn collect_json_path_suffix(chars: &[char], start: usize) -> (String, usize) {
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

fn substitute_tokens_and_vars(line: &str, state: &PreprocState) -> Result<String, Diagnostic> {
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

fn parse_variable_assignment(name: &str, arg: &str, raw: &str) -> Option<PreprocessDirective> {
    parse_variable_assignment_with_scope(name, arg, raw, PreprocVariableScope::Default)
}

fn parse_variable_assignment_with_scope(
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

fn parse_scoped_variable_assignment(
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

fn parse_named_call(rest: &str) -> Option<(String, String)> {
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

fn expand_preprocessor_text(
    raw_line: &str,
    state: &PreprocState,
    call_depth: usize,
) -> Result<String, Diagnostic> {
    let substituted = collapse_macro_concat(&substitute_tokens_and_vars(raw_line, state)?);
    let expanded = expand_function_invocations(&substituted, state, call_depth)?;
    Ok(collapse_macro_concat(&expanded))
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

/// Dispatch a known preprocessor builtin. Returns `Ok(Some(result))` if the
/// name maps to a builtin, `Ok(None)` if the name is not recognised so the
/// caller can fall through to its unknown-function diagnostic.
///
/// Time/IO-sensitive builtins (`%date`, `%getenv`) deliberately return an
/// empty string. PlantUML's defaults inject the current wall-clock or process
/// environment, which would defeat determinism: identical source must yield
/// identical bytes for `cargo test`/`puml --check` to be useful.
fn dispatch_builtin(
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
            eval_int_expr(&arg(0))
                .map(|n| n.to_string())
                .unwrap_or_else(|| arg(0)),
        ),
        "eval_bool" | "eval_boolean" => Some(evaluate_scalar_expr(&arg(0))?.to_string()),
        "if" | "ternary" | "iif" => Some(if evaluate_scalar_expr(&arg(0))? {
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
            Some(eval_int_expr(arg(0).trim()).is_some().to_string())
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
fn strip_quotes(s: &str) -> String {
    let trimmed = s.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_int_lenient(s: &str) -> i64 {
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
fn get_json_attribute(json: &str, key: &str) -> String {
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

fn preprocessor_list_items(raw: &str) -> Vec<String> {
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

fn preprocessor_foreach_bindings(var_names: &[String], rhs: &str) -> Vec<Vec<(String, String)>> {
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

fn preprocessor_size(raw: &str) -> usize {
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

fn preprocessor_list_literal(items: &[String]) -> String {
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

fn json_value_from_preproc(raw: &str) -> serde_json::Value {
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

fn extract_parenthesized_args(
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

fn parse_callable_definition(
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

fn parse_params(raw: &str) -> Result<Vec<PreprocParam>, Diagnostic> {
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

fn split_args(raw: &str) -> Result<Vec<String>, Diagnostic> {
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

fn bind_callable_args(
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

fn execute_function_call(
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
fn execute_procedure_call(
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
fn invoke_dynamic_procedure(
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
