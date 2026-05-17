use std::cell::RefCell;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{
    ActivityStep, ActivityStepKind, ClassDecl, ClassMember, ComponentNodeKind, DiagramKind,
    Document, FamilyRelation, Group, MemberModifier, Message, MessageStyle, Note, ObjectDecl,
    ParticipantDecl, ParticipantRole, SaltCell, StateDecl, StateInternalAction, StateTransition,
    Statement, StatementKind, TimingDeclKind, UseCaseDecl, VirtualEndpoint, VirtualEndpointKind,
    VirtualEndpointSide,
};
use crate::diagnostic::Diagnostic;
use crate::source::Span;

const MAX_INCLUDE_DEPTH: usize = 32;
const MAX_PREPROC_WHILE_ITERATIONS: usize = 10_000;
const MAX_PREPROC_CALL_DEPTH: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
struct IncludeTarget {
    path: PathBuf,
    tag: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ParseOptions {
    pub include_root: Option<PathBuf>,
}

pub fn parse(source: &str) -> Result<Document, Diagnostic> {
    parse_with_options(source, &ParseOptions::default())
}

pub fn parse_with_options(source: &str, options: &ParseOptions) -> Result<Document, Diagnostic> {
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

    parse_preprocessed(&expanded)
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
    Function,
    EndFunction,
    Procedure,
    EndProcedure,
    Assert(String),
    Log(String),
    DumpMemory(String),
    DynamicInvocation(String),
    JsonPreproc(String),
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
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PreprocCallableKind {
    Function,
    Procedure,
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

#[derive(Debug, Clone, Default)]
struct PreprocState {
    defines: BTreeMap<String, String>,
    vars: BTreeMap<String, String>,
    callables: BTreeMap<String, PreprocCallable>,
    // Counters used by the deterministic builtins `%false_then_true` /
    // `%true_then_false`. PlantUML semantics use a per-callsite latch — we
    // key by the argument value so identical sources produce identical
    // AST/render bytes. Interior mutability lets us update from
    // `expand_function_invocations` which only borrows `&PreprocState`.
    false_then_true_counts: RefCell<BTreeMap<String, u64>>,
    true_then_false_counts: RefCell<BTreeMap<String, u64>>,
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
                            preprocess_text(
                                &block,
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
                        // Expected form: `$var in val1, val2, val3` or
                        // `$var in $listvar` where $listvar is comma-separated.
                        let parts: Vec<&str> = spec.splitn(2, " in ").collect();
                        if parts.len() != 2 {
                            return Err(Diagnostic::error_code(
                                "E_PREPROC_FOREACH_FORM",
                                "`!foreach` requires form `$var in val1, val2, ...`",
                            ));
                        }
                        let var_name = parts[0].trim().trim_start_matches('$').to_string();
                        let rhs = expand_preprocessor_text(parts[1].trim(), state, 0)?;
                        let items = preprocessor_list_items(&rhs);
                        let block = lines[i + 1..endfor].join("\n");
                        let prev = state.vars.get(&var_name).cloned();
                        for item in items {
                            state.vars.insert(var_name.clone(), item);
                            preprocess_text(
                                &block,
                                options,
                                state,
                                include_stack,
                                include_once_seen,
                                depth,
                                call_depth,
                                out,
                            )?;
                        }
                        match prev {
                            Some(v) => {
                                state.vars.insert(var_name, v);
                            }
                            None => {
                                state.vars.remove(&var_name);
                            }
                        }
                    }
                    i = endfor + 1;
                    continue;
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
                        let (name, value) = body.split_once(' ').unwrap_or((body.as_str(), ""));
                        let name = name.trim();
                        if !name.is_empty() {
                            state
                                .defines
                                .insert(name.to_string(), value.trim().to_string());
                        }
                    }
                }
                PreprocessDirective::Undef(name) => {
                    if active {
                        let name = name.trim();
                        if !name.is_empty() {
                            state.defines.remove(name);
                        }
                    }
                }
                PreprocessDirective::VariableAssign {
                    name,
                    value,
                    conditional,
                } => {
                    if active {
                        let trimmed = value.trim_start();
                        let is_json_literal = trimmed.starts_with('{') || trimmed.starts_with('[');
                        if !conditional || !state.vars.contains_key(&name) {
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
                        return Err(Diagnostic::error_code(
                            "E_INCLUDE_URL_UNSUPPORTED",
                            format!("!includeurl URL targets are not supported: {raw_target}"),
                        ));
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
        return Err(Diagnostic::error_code(
            "E_INCLUDE_URL_UNSUPPORTED",
            format!("{directive_name} URL targets are not supported: {raw_target}"),
        ));
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

fn is_stdlib_catalog_target(raw_target: &str) -> bool {
    let trimmed = raw_target.trim();
    trimmed.starts_with('<') && trimmed.ends_with('>')
}

/// `!include_many` with optional glob expansion. When the path contains `*`
/// or `?`, expand it to every matching file in deterministic alphabetical
/// order; otherwise behave like `!include`. Globs only match the file-name
/// segment of the path so we cannot escape the include root by accident.
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
        return Err(Diagnostic::error_code(
            "E_INCLUDE_URL_UNSUPPORTED",
            format!("!include_many URL targets are not supported: {raw_target}"),
        ));
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
        return Err(Diagnostic::error_code(
            "E_IMPORT_URL_UNSUPPORTED",
            format!("!import URL targets are not supported: {raw_target}"),
        ));
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
        "function" => Some(PreprocessDirective::Function),
        "endfunction" => Some(PreprocessDirective::EndFunction),
        "procedure" => Some(PreprocessDirective::Procedure),
        "endprocedure" => Some(PreprocessDirective::EndProcedure),
        "assert" => Some(PreprocessDirective::Assert(arg.to_string())),
        "log" => Some(PreprocessDirective::Log(arg.to_string())),
        "dump_memory" => Some(PreprocessDirective::DumpMemory(arg.to_string())),
        _ if name.starts_with('$') => parse_variable_assignment(name, arg, trimmed),
        "return" => Some(PreprocessDirective::Unsupported(name.to_string())),
        // `!startsub` / `!endsub` are markers used by `!includesub`. When a
        // file containing them is included directly, we silently elide the
        // marker lines and pass the body lines through.
        "startsub" | "endsub" => Some(PreprocessDirective::NoOp),
        "theme" | "pragma" => None,
        _ if let Some((call_name, call_args)) = parse_named_call(rest) => {
            Some(PreprocessDirective::ProcedureCall {
                name: call_name,
                args: call_args,
            })
        }
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

    if let Some((lhs, rhs)) = split_top_level(trimmed, "==") {
        return Ok(normalize_expr_value(&lhs) == normalize_expr_value(&rhs));
    }
    if let Some((lhs, rhs)) = split_top_level(trimmed, "!=") {
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

fn is_url_include_target(raw_target: &str) -> bool {
    let trimmed = raw_target
        .trim()
        .trim_matches('"')
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim();
    let lower = trimmed.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

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
fn is_angle_bracket_include(raw_target: &str) -> bool {
    let trimmed = raw_target.trim();
    trimmed.starts_with('<') && trimmed.ends_with('>')
}

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

fn substitute_tokens(line: &str, defines: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(line.len());
    let mut token = String::new();
    let mut in_quotes = false;

    let flush_token = |token: &mut String, out: &mut String, defines: &BTreeMap<String, String>| {
        if token.is_empty() {
            return;
        }
        if let Some(v) = defines.get(token.as_str()) {
            out.push_str(v);
        } else {
            out.push_str(token);
        }
        token.clear();
    };

    for ch in line.chars() {
        if ch == '"' {
            flush_token(&mut token, &mut out, defines);
            in_quotes = !in_quotes;
            out.push(ch);
            continue;
        }

        if !in_quotes && (ch.is_ascii_alphanumeric() || ch == '_') {
            token.push(ch);
            continue;
        }

        flush_token(&mut token, &mut out, defines);
        out.push(ch);
    }

    flush_token(&mut token, &mut out, defines);
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

fn substitute_tokens_and_vars(line: &str, state: &PreprocState) -> String {
    let with_tokens = substitute_tokens(line, &state.defines);
    substitute_vars(&with_tokens, &state.vars)
}

fn parse_variable_assignment(name: &str, arg: &str, raw: &str) -> Option<PreprocessDirective> {
    let var = name.strip_prefix('$')?.trim().to_string();
    if var.is_empty() {
        return Some(PreprocessDirective::JsonPreproc(raw.to_string()));
    }
    if let Some(value) = arg.strip_prefix("?=") {
        return Some(PreprocessDirective::VariableAssign {
            name: var,
            value: value.trim().to_string(),
            conditional: true,
        });
    }
    if let Some(value) = arg.strip_prefix('=') {
        return Some(PreprocessDirective::VariableAssign {
            name: var,
            value: value.trim().to_string(),
            conditional: false,
        });
    }
    Some(PreprocessDirective::JsonPreproc(raw.to_string()))
}

fn parse_named_call(rest: &str) -> Option<(String, String)> {
    let rest = rest.trim();
    let open = rest.find('(')?;
    let close = rest.rfind(')')?;
    if close <= open || close != rest.len() - 1 {
        return None;
    }
    let name = rest[..open].trim();
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
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
    let substituted = collapse_macro_concat(&substitute_tokens_and_vars(raw_line, state));
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
    while i < chars.len() {
        if chars[i] == '#' && i + 1 < chars.len() && chars[i + 1] == '#' {
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
        "strlen" => Some(arg(0).chars().count().to_string()),
        "size" => Some(preprocessor_size(&arg(0)).to_string()),
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
        "splitstr_regex" => Some(split_preprocessor_regex(&arg(0), &arg(1)).join(",")),
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
        "list" | "array" => Some(preprocessor_list_literal(&expanded_args)),
        "list_size" | "array_size" | "map_size" => Some(preprocessor_size(&arg(0)).to_string()),
        "list_contains" | "contains_list" => Some(
            preprocessor_list_items(&arg(0))
                .contains(&arg(1))
                .to_string(),
        ),
        "list_get" => Some(
            preprocessor_list_items(&arg(0))
                .get(parse_int_lenient(&arg(1)).max(0) as usize)
                .cloned()
                .unwrap_or_default(),
        ),
        "list_add" => {
            let mut items = preprocessor_list_items(&arg(0));
            items.push(arg(1));
            Some(preprocessor_list_literal(&items))
        }
        "map" | "dict" => Some(preprocessor_map_literal(&expanded_args)),
        "map_contains_key" | "contains_key" => {
            Some(json_contains_key(&arg(0), &arg(1)).to_string())
        }
        "get" | "map_get" | "json_get" => Some(preprocessor_get(&arg(0), &arg(1))),
        "set" | "put" | "json_set" | "map_put" => Some(preprocessor_set(&arg(0), &arg(1), &arg(2))),
        "keys" | "map_keys" => Some(preprocessor_json_keys(&arg(0)).join(",")),
        "values" | "map_values" => Some(preprocessor_json_values(&arg(0)).join(",")),
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
        "startswith" | "starts_with" => Some(arg(0).starts_with(&arg(1)).to_string()),
        "endswith" | "ends_with" => Some(arg(0).ends_with(&arg(1)).to_string()),
        "contains" => Some(arg(0).contains(&arg(1)).to_string()),
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
        "date" | "getenv" => Some(String::new()),
        // Random-sensitive builtins: fixed deterministic value.
        "random" | "rand" => Some("0".to_string()),
        // Local/remote IO helpers are disabled rather than implicitly reading
        // host state during preprocessing.
        "load_file" | "load_data" | "load_json" | "load_yaml" | "load_csv" => {
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

fn preprocessor_list_literal(items: &[String]) -> String {
    let values = items
        .iter()
        .map(|item| serde_json::Value::String(item.clone()))
        .collect::<Vec<_>>();
    serde_json::Value::Array(values).to_string()
}

fn preprocessor_map_literal(args: &[String]) -> String {
    let mut obj = serde_json::Map::new();
    for chunk in args.chunks(2) {
        if let [key, value] = chunk {
            obj.insert(key.clone(), serde_json::Value::String(value.clone()));
        }
    }
    serde_json::Value::Object(obj).to_string()
}

fn preprocessor_get(container: &str, key: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(container.trim()) {
        if let Ok(idx) = key.trim().parse::<usize>() {
            return value
                .as_array()
                .and_then(|items| items.get(idx))
                .map(json_value_to_preproc_string)
                .unwrap_or_default();
        }
        return json_value_at_path(&value, key)
            .map(json_value_to_preproc_string)
            .unwrap_or_default();
    }
    preprocessor_list_items(container)
        .get(key.trim().parse::<usize>().unwrap_or(usize::MAX))
        .cloned()
        .unwrap_or_default()
}

fn preprocessor_set(container: &str, key: &str, replacement: &str) -> String {
    if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(container.trim()) {
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
    if in_quotes || paren_depth != 0 {
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
    for raw in &callable.body {
        let line = raw.trim();
        if !line.to_ascii_lowercase().starts_with("!return") {
            continue;
        }
        let trimmed_return = raw.trim_start();
        let expr = trimmed_return
            .trim_start_matches("!return")
            .trim_start()
            .to_string();
        let mut local_state = state.clone();
        for (k, v) in &bindings {
            local_state.vars.insert(k.clone(), v.clone());
        }
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
    let mut local = String::new();
    for raw in &callable.body {
        if raw.trim().to_ascii_lowercase().starts_with("!return") {
            return Err(Diagnostic::error_code(
                "E_PREPROC_RETURN_UNEXPECTED",
                format!("procedure `{name}` cannot contain `!return`"),
            ));
        }
        let mut local_state = state.clone();
        for (k, v) in &bindings {
            local_state.vars.insert(k.clone(), v.clone());
        }
        local.push_str(&expand_preprocessor_text(
            raw,
            &local_state,
            call_depth + 1,
        )?);
        local.push('\n');
    }
    preprocess_text(
        &local,
        options,
        state,
        include_stack,
        include_once_seen,
        depth,
        call_depth + 1,
        out,
    )
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

fn parse_preprocessed(source: &str) -> Result<Document, Diagnostic> {
    let mut statements = Vec::new();
    let mut lines = Vec::new();
    let mut offset = 0usize;
    for raw_line in source.lines() {
        let span = Span::new(offset, offset + raw_line.len());
        lines.push((raw_line, span));
        offset += raw_line.len() + 1;
    }

    let mut detected_kind: Option<DiagramKind> = None;
    let mut in_block = false;
    let mut block_kind: Option<BlockKind> = None;
    let mut block_start_span: Option<Span> = None;
    let mut i = 0usize;
    while i < lines.len() {
        let (raw_line, span) = lines[i];
        let line = strip_inline_plantuml_comment(raw_line).trim();

        // In raw-body family blocks we never strip empty lines or interpret comments.
        // Check for the closing marker first; otherwise capture verbatim.
        if let Some(bk) = block_kind {
            if is_raw_body_block(bk) || block_kind_is_raw_body(bk) {
                if let Some(end_kind) = parse_end_block_kind(raw_line.trim()) {
                    if block_kind == Some(end_kind) {
                        in_block = false;
                        block_kind = None;
                        block_start_span = None;
                        i += 1;
                        continue;
                    } else {
                        return Err(Diagnostic::error(format!(
                            "[E_BLOCK_MISMATCH] closing marker `@end{}` does not match opening `@start{}`",
                            block_kind_name(end_kind),
                            block_kind_name(bk)
                        ))
                        .with_span(span));
                    }
                }
                statements.push(Statement {
                    span,
                    kind: StatementKind::RawBody(raw_line.to_string()),
                });
                i += 1;
                continue;
            }
        }

        if line.is_empty() || line.starts_with('"') {
            i += 1;
            continue;
        }
        if let Some(start_kind) = parse_start_block_kind(line) {
            if in_block {
                return Err(Diagnostic::error(
                    "unmatched @startuml/@enduml boundary: found new @start marker before closing previous block",
                )
                .with_span(span));
            }
            in_block = true;
            block_kind = Some(start_kind);
            block_start_span = Some(span);
            if let Some(candidate) = start_block_family(start_kind) {
                detected_kind = Some(select_diagram_kind(detected_kind, candidate, span)?);
            }
            i += 1;
            continue;
        }
        if let Some(end_kind) = parse_end_block_kind(line) {
            if !in_block {
                return Err(Diagnostic::error(
                    "unmatched @startuml/@enduml boundary: found @end marker without a preceding @startuml",
                )
                .with_span(span));
            }
            if block_kind != Some(end_kind) {
                return Err(Diagnostic::error(format!(
                    "[E_BLOCK_MISMATCH] closing marker `@end{}` does not match opening `@start{}`",
                    block_kind_name(end_kind),
                    block_kind_name(block_kind.unwrap_or(BlockKind::Uml))
                ))
                .with_span(span));
            }
            in_block = false;
            block_kind = None;
            block_start_span = None;
            i += 1;
            continue;
        }

        if let Some(kind) = parse_keyword(line) {
            let multiline_note_head =
                matches!(&kind, StatementKind::Note(_)) && note_block_continues(&lines, i, line);
            let multiline_text_head = matches!(
                &kind,
                StatementKind::Title(_)
                    | StatementKind::Header(_)
                    | StatementKind::Footer(_)
                    | StatementKind::Caption(_)
                    | StatementKind::Legend(_)
            ) && text_block_continues(&lines, i, line);
            if detected_kind.is_some()
                && is_family_common_keyword(&kind)
                && !multiline_note_head
                && !multiline_text_head
            {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }

        if matches!(
            detected_kind,
            Some(DiagramKind::Component | DiagramKind::Deployment)
        ) {
            if let Some((kind, end_idx)) = parse_component_scoping_block(&lines, i, line)? {
                let family = if matches!(detected_kind, Some(DiagramKind::Deployment)) {
                    DiagramKind::Deployment
                } else {
                    DiagramKind::Component
                };
                detected_kind = Some(select_diagram_kind(detected_kind, family, span)?);
                statements.push(Statement { span, kind });
                i = end_idx + 1;
                continue;
            }
            if let Some(kind) = parse_component_decl(line) {
                let family = match &kind {
                    StatementKind::ComponentDecl {
                        kind:
                            ComponentNodeKind::Node
                            | ComponentNodeKind::Artifact
                            | ComponentNodeKind::Cloud
                            | ComponentNodeKind::Frame
                            | ComponentNodeKind::Storage
                            | ComponentNodeKind::Database
                            | ComponentNodeKind::Package
                            | ComponentNodeKind::Rectangle
                            | ComponentNodeKind::Folder
                            | ComponentNodeKind::File
                            | ComponentNodeKind::Card,
                        ..
                    } => DiagramKind::Deployment,
                    _ => DiagramKind::Component,
                };
                detected_kind = Some(select_diagram_kind(detected_kind, family, span)?);
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }

        if matches!(
            detected_kind,
            None | Some(DiagramKind::Class | DiagramKind::Object | DiagramKind::UseCase)
        ) && !(detected_kind.is_none()
            && in_block
            && block_kind == Some(BlockKind::Uml)
            && ((line.starts_with("interface ")
                && !later_lines_contain_class_family_declaration(&lines, i))
                || (line.starts_with("actor ")
                    && !line.contains("<<")
                    && !later_lines_contain_usecase_family_declaration(&lines, i))))
        {
            if let Some((kind, end_idx)) = parse_family_declaration(&lines, i, line)? {
                let family = family_for_declaration(&kind);
                detected_kind = Some(select_diagram_kind(detected_kind, family, span)?);
                let block_span = Span::new(span.start, lines[end_idx].1.end);
                statements.push(Statement {
                    span: block_span,
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
        }

        if let Some(kind) = parse_family_member_row(line, detected_kind) {
            statements.push(Statement { span, kind });
            i += 1;
            continue;
        }

        if let Some(kind) = parse_family_relation(line, detected_kind) {
            statements.push(Statement { span, kind });
            i += 1;
            continue;
        }

        if matches!(detected_kind, None | Some(DiagramKind::Class)) {
            if let Some((kind, end_idx)) = parse_class_scoping_block(&lines, i, line)? {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Class,
                    span,
                )?);
                let block_span = Span::new(span.start, lines[end_idx].1.end);
                statements.push(Statement {
                    span: block_span,
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
        }

        if detected_kind.is_none() && detect_non_sequence_family(line) != Some(DiagramKind::State) {
            if let Some(kind) = parse_message(line) {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Sequence,
                    span,
                )?);
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }

        if detected_kind.is_none()
            && in_block
            && block_kind == Some(BlockKind::Uml)
            && !(line.starts_with("actor ") && line.contains("<<"))
        {
            if let Some(kind) = parse_participant(line) {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Sequence,
                    span,
                )?);
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }

        if detected_kind.is_none() {
            if let Some(kind) = detect_non_sequence_family(line) {
                detected_kind = Some(kind);
            } else if parse_component_decl(line).is_some() {
                detected_kind = Some(DiagramKind::Component);
            } else if looks_like_unsupported_family_syntax(line) {
                detected_kind = Some(DiagramKind::Unknown);
            }
        }

        // Family-specific inline parsing for the newly-implemented families.
        if matches!(
            detected_kind,
            Some(DiagramKind::Component) | Some(DiagramKind::Deployment)
        ) {
            if let Some((kind, end_idx)) = parse_component_scoping_block(&lines, i, line)? {
                statements.push(Statement { span, kind });
                i = end_idx + 1;
                continue;
            }
            if let Some(kind) = parse_component_decl(line) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
            // Try a relation again now that detection settled.
            if let Some(kind) = parse_family_relation(line, detected_kind) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }

        if matches!(detected_kind, Some(DiagramKind::Activity)) {
            if let Some(kind) = parse_activity_step(line) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }

        if matches!(detected_kind, Some(DiagramKind::Timing)) {
            if let Some(kind) = parse_timing_decl(line) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
            if let Some(kind) = parse_timing_event(line) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }

        let allow_sequence_parse =
            detected_kind.is_none() || matches!(detected_kind, Some(DiagramKind::Sequence));
        let allow_gantt_parse = matches!(detected_kind, Some(DiagramKind::Gantt));
        let allow_chronology_parse = matches!(detected_kind, Some(DiagramKind::Chronology));
        let allow_state_parse = matches!(detected_kind, Some(DiagramKind::State));
        // MindMap and WBS also support multiline legend/title/caption/header/footer blocks.
        let allow_family_keyword_block =
            matches!(detected_kind, Some(DiagramKind::MindMap | DiagramKind::Wbs));

        if allow_sequence_parse {
            if let Some((kind, end_idx)) = parse_multiline_keyword_block(&lines, i, line) {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Sequence,
                    span,
                )?);
                let block_span = Span::new(span.start, lines[end_idx].1.end);
                statements.push(Statement {
                    span: block_span,
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
        }

        if allow_family_keyword_block {
            if let Some((kind, end_idx)) = parse_multiline_keyword_block(&lines, i, line) {
                let block_span = Span::new(span.start, lines[end_idx].1.end);
                statements.push(Statement {
                    span: block_span,
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
        }

        if allow_gantt_parse {
            if let Some(kind) = parse_gantt_baseline_statement(line) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
            if let Some((kind, end_idx)) = parse_multiline_keyword_block(&lines, i, line) {
                statements.push(Statement {
                    span: Span::new(span.start, lines[end_idx].1.end),
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
            if let Some(kind) = parse_keyword(line) {
                if is_timeline_metadata_statement(&kind) {
                    statements.push(Statement { span, kind });
                    i += 1;
                    continue;
                }
            }
            statements.push(Statement {
                span,
                kind: StatementKind::Unknown(format!(
                    "[E_GANTT_UNSUPPORTED] unsupported gantt baseline syntax: `{line}`"
                )),
            });
            i += 1;
            continue;
        }

        if allow_chronology_parse {
            if let Some(kind) = parse_chronology_baseline_statement(line) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
            if let Some((kind, end_idx)) = parse_multiline_keyword_block(&lines, i, line) {
                statements.push(Statement {
                    span: Span::new(span.start, lines[end_idx].1.end),
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
            if let Some(kind) = parse_keyword(line) {
                if is_timeline_metadata_statement(&kind) {
                    statements.push(Statement { span, kind });
                    i += 1;
                    continue;
                }
            }
            statements.push(Statement {
                span,
                kind: StatementKind::Unknown(format!(
                    "[E_CHRONOLOGY_UNSUPPORTED] unsupported chronology baseline syntax: `{line}`"
                )),
            });
            i += 1;
            continue;
        }

        if allow_state_parse {
            if let Some((kind, end_idx)) = parse_state_statement(&lines, i, line)? {
                let block_span = if end_idx > i {
                    Span::new(span.start, lines[end_idx].1.end)
                } else {
                    span
                };
                statements.push(Statement {
                    span: block_span,
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
            // Any non-empty line in a state diagram that wasn't recognised above
            // is stored as Unknown for normalizer to reject gracefully.
            statements.push(Statement {
                span,
                kind: StatementKind::Unknown(line.to_string()),
            });
            i += 1;
            continue;
        }

        if allow_sequence_parse {
            if let Some((kind, end_idx)) = parse_multiline_note_block(&lines, i, line) {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Sequence,
                    span,
                )?);
                let block_span = Span::new(span.start, lines[end_idx].1.end);
                statements.push(Statement {
                    span: block_span,
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
        }
        if allow_sequence_parse {
            if let Some((kind, end_idx)) = parse_multiline_ref_block(&lines, i, line) {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Sequence,
                    span,
                )?);
                let block_span = Span::new(span.start, lines[end_idx].1.end);
                statements.push(Statement {
                    span: block_span,
                    kind,
                });
                i = end_idx + 1;
                continue;
            }
        }

        if allow_sequence_parse {
            if let Some(kind) = parse_participant(line) {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Sequence,
                    span,
                )?);
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }
        if allow_sequence_parse {
            if let Some(kind) = parse_message(line) {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Sequence,
                    span,
                )?);
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
        }
        if allow_sequence_parse && looks_like_arrow_syntax(line) {
            return Err(Diagnostic::error(format!(
                "[E_ARROW_INVALID] malformed sequence arrow syntax: `{}`",
                line
            ))
            .with_span(span));
        }

        if let Some(kind) = parse_keyword(line) {
            if is_sequence_keyword(&kind) {
                detected_kind = Some(select_diagram_kind(
                    detected_kind,
                    DiagramKind::Sequence,
                    span,
                )?);
            }
            statements.push(Statement { span, kind });
            i += 1;
            continue;
        }

        // Inline JSON/YAML projection: `json $alias {` / `yaml $alias {` ... `}`
        // Valid inside @startuml/@enduml blocks for object-diagram-like use.
        // Marks the document as Class family (rendered via render_class_svg).
        if let Some((kind, end_idx)) = parse_json_projection_block(&lines, i, line)? {
            detected_kind = Some(select_diagram_kind(
                detected_kind,
                DiagramKind::Class,
                span,
            )?);
            let block_span = Span::new(span.start, lines[end_idx].1.end);
            statements.push(Statement {
                span: block_span,
                kind,
            });
            i = end_idx + 1;
            continue;
        }

        // Salt wireframe grid row parsing: `|cell|cell|cell|`
        if matches!(detected_kind, Some(DiagramKind::Salt)) {
            if let Some(kind) = parse_salt_grid_row(line) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
            }
            // Skip `{`, `}`, `{+`, `{-`, `---` markers inside salt blocks
            let trimmed = line.trim();
            if matches!(trimmed, "{" | "}" | "{+" | "{-" | "---") || trimmed.is_empty() {
                i += 1;
                continue;
            }
        }

        statements.push(Statement {
            span,
            kind: StatementKind::Unknown(line.to_string()),
        });
        i += 1;
    }

    if in_block {
        return Err(Diagnostic::error(
            "unmatched @startuml/@enduml boundary: opening @start marker is missing a closing @enduml",
        )
        .with_span(block_start_span.unwrap_or(Span::new(0, 0))));
    }

    Ok(Document {
        kind: detected_kind.unwrap_or(DiagramKind::Unknown),
        statements,
    })
}

fn strip_inline_plantuml_comment(line: &str) -> &str {
    let mut in_quotes = false;
    for (idx, ch) in line.char_indices() {
        if ch == '"' {
            in_quotes = !in_quotes;
            continue;
        }
        if ch == '\'' && !in_quotes {
            return &line[..idx];
        }
    }
    line
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Uml,
    Salt,
    MindMap,
    Wbs,
    Gantt,
    Chronology,
    Json,
    Yaml,
    Nwdiag,
    Archimate,
    Regex,
    Ebnf,
    Math,
    Sdl,
    Ditaa,
    Chart,
}

fn parse_start_block_kind(line: &str) -> Option<BlockKind> {
    parse_block_marker_kind(line, true)
}

fn parse_end_block_kind(line: &str) -> Option<BlockKind> {
    parse_block_marker_kind(line, false)
}

fn parse_block_marker_kind(line: &str, start: bool) -> Option<BlockKind> {
    let lower = line.to_ascii_lowercase();
    // NOTE: longer markers must come before shorter prefixes that they share.
    let markers: &[(&str, BlockKind)] = if start {
        &[
            ("@startmindmap", BlockKind::MindMap),
            ("@startchronology", BlockKind::Chronology),
            ("@startjson", BlockKind::Json),
            ("@startyaml", BlockKind::Yaml),
            ("@startnwdiag", BlockKind::Nwdiag),
            ("@startarchimate", BlockKind::Archimate),
            ("@startregex", BlockKind::Regex),
            ("@startebnf", BlockKind::Ebnf),
            ("@startlatex", BlockKind::Math),
            ("@startmath", BlockKind::Math),
            ("@startditaa", BlockKind::Ditaa),
            ("@startchart", BlockKind::Chart),
            ("@startsdl", BlockKind::Sdl),
            ("@startgantt", BlockKind::Gantt),
            ("@startwbs", BlockKind::Wbs),
            ("@startsalt", BlockKind::Salt),
            ("@startuml", BlockKind::Uml),
        ]
    } else {
        &[
            ("@endmindmap", BlockKind::MindMap),
            ("@endchronology", BlockKind::Chronology),
            ("@endjson", BlockKind::Json),
            ("@endyaml", BlockKind::Yaml),
            ("@endnwdiag", BlockKind::Nwdiag),
            ("@endarchimate", BlockKind::Archimate),
            ("@endregex", BlockKind::Regex),
            ("@endebnf", BlockKind::Ebnf),
            ("@endlatex", BlockKind::Math),
            ("@endmath", BlockKind::Math),
            ("@endditaa", BlockKind::Ditaa),
            ("@endchart", BlockKind::Chart),
            ("@endsdl", BlockKind::Sdl),
            ("@endgantt", BlockKind::Gantt),
            ("@endwbs", BlockKind::Wbs),
            ("@endsalt", BlockKind::Salt),
            ("@enduml", BlockKind::Uml),
        ]
    };
    for (marker, kind) in markers {
        if lower.starts_with(marker) {
            let rest = &line[marker.len()..];
            if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                return Some(*kind);
            }
        }
    }
    None
}

fn start_block_family(kind: BlockKind) -> Option<DiagramKind> {
    match kind {
        BlockKind::Uml => None,
        BlockKind::Salt => Some(DiagramKind::Salt),
        BlockKind::MindMap => Some(DiagramKind::MindMap),
        BlockKind::Wbs => Some(DiagramKind::Wbs),
        BlockKind::Gantt => Some(DiagramKind::Gantt),
        BlockKind::Chronology => Some(DiagramKind::Chronology),
        BlockKind::Json => Some(DiagramKind::Json),
        BlockKind::Yaml => Some(DiagramKind::Yaml),
        BlockKind::Nwdiag => Some(DiagramKind::Nwdiag),
        BlockKind::Archimate => Some(DiagramKind::Archimate),
        BlockKind::Regex => Some(DiagramKind::Regex),
        BlockKind::Ebnf => Some(DiagramKind::Ebnf),
        BlockKind::Math => Some(DiagramKind::Math),
        BlockKind::Sdl => Some(DiagramKind::Sdl),
        BlockKind::Ditaa => Some(DiagramKind::Ditaa),
        BlockKind::Chart => Some(DiagramKind::Chart),
    }
}

fn block_kind_name(kind: BlockKind) -> &'static str {
    match kind {
        BlockKind::Uml => "uml",
        BlockKind::Salt => "salt",
        BlockKind::MindMap => "mindmap",
        BlockKind::Wbs => "wbs",
        BlockKind::Gantt => "gantt",
        BlockKind::Chronology => "chronology",
        BlockKind::Json => "json",
        BlockKind::Yaml => "yaml",
        BlockKind::Nwdiag => "nwdiag",
        BlockKind::Archimate => "archimate",
        BlockKind::Regex => "regex",
        BlockKind::Ebnf => "ebnf",
        BlockKind::Math => "math",
        BlockKind::Sdl => "sdl",
        BlockKind::Ditaa => "ditaa",
        BlockKind::Chart => "chart",
    }
}

fn is_raw_body_block(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Json | BlockKind::Yaml | BlockKind::Nwdiag | BlockKind::Archimate
    )
}

fn block_kind_is_raw_body(kind: BlockKind) -> bool {
    matches!(
        kind,
        BlockKind::Regex
            | BlockKind::Ebnf
            | BlockKind::Math
            | BlockKind::Sdl
            | BlockKind::Ditaa
            | BlockKind::Chart
    )
}

fn select_diagram_kind(
    current: Option<DiagramKind>,
    candidate: DiagramKind,
    span: Span,
) -> Result<DiagramKind, Diagnostic> {
    let Some(current) = current else {
        return Ok(candidate);
    };
    if current == candidate {
        return Ok(current);
    }
    if current == DiagramKind::Unknown || candidate == DiagramKind::Unknown {
        return Ok(DiagramKind::Unknown);
    }
    Err(Diagnostic::error(format!(
        "[E_FAMILY_MIXED] mixed diagram families are not supported: found `{}` syntax in `{}` diagram",
        diagram_kind_name(candidate),
        diagram_kind_name(current)
    ))
    .with_span(span))
}

fn looks_like_unsupported_family_syntax(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.starts_with("state ")
        || lower.starts_with("component ")
        || lower.starts_with("activity ")
        || lower.starts_with("deployment ")
        || lower.starts_with('*')
        || lower.starts_with("mindmap")
        || lower.starts_with("wbs")
        || lower.starts_with("node ")
        || lower.starts_with("clock ")
        || lower.starts_with("binary ")
        || lower.starts_with("robust ")
        || lower.starts_with("concise ")
}

fn diagram_kind_name(kind: DiagramKind) -> &'static str {
    match kind {
        DiagramKind::Sequence => "sequence",
        DiagramKind::Class => "class",
        DiagramKind::Object => "object",
        DiagramKind::UseCase => "usecase",
        DiagramKind::Salt => "salt",
        DiagramKind::MindMap => "mindmap",
        DiagramKind::Wbs => "wbs",
        DiagramKind::Gantt => "gantt",
        DiagramKind::Chronology => "chronology",
        DiagramKind::Component => "component",
        DiagramKind::Deployment => "deployment",
        DiagramKind::State => "state",
        DiagramKind::Activity => "activity",
        DiagramKind::Timing => "timing",
        DiagramKind::Json => "json",
        DiagramKind::Yaml => "yaml",
        DiagramKind::Nwdiag => "nwdiag",
        DiagramKind::Archimate => "archimate",
        DiagramKind::Regex => "regex",
        DiagramKind::Ebnf => "ebnf",
        DiagramKind::Math => "math",
        DiagramKind::Sdl => "sdl",
        DiagramKind::Ditaa => "ditaa",
        DiagramKind::Chart => "chart",
        DiagramKind::Unknown => "unknown",
    }
}

fn family_for_declaration(kind: &StatementKind) -> DiagramKind {
    match kind {
        StatementKind::ClassDecl(_) => DiagramKind::Class,
        StatementKind::ObjectDecl(_) => DiagramKind::Object,
        StatementKind::UseCaseDecl(_) => DiagramKind::UseCase,
        _ => DiagramKind::Unknown,
    }
}

fn parse_family_declaration(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    for (keyword, marker) in [
        ("abstract class", Some("<<abstract class>>")),
        ("interface", Some("<<interface>>")),
        ("enum", Some("<<enum>>")),
        ("annotation", Some("<<annotation>>")),
        ("protocol", Some("<<protocol>>")),
        ("struct", Some("<<struct>>")),
        ("abstract", Some("<<abstract>>")),
        ("class", None),
    ] {
        let Some((name, alias, has_block, stereotypes)) = parse_named_family_decl(line, keyword)
        else {
            continue;
        };
        let members = if has_block {
            let mut members = parse_family_decl_members(lines, start, keyword, &name)?;
            if let Some(marker) = marker {
                members.insert(
                    0,
                    ClassMember {
                        text: marker.to_string(),
                        modifier: None,
                    },
                );
            }
            for stereotype in stereotypes.iter().rev() {
                members.insert(
                    0,
                    ClassMember {
                        text: format!("<<{stereotype}>>"),
                        modifier: None,
                    },
                );
            }
            members
        } else {
            declaration_marker_members(marker, stereotypes)
        };
        return Ok(Some((
            StatementKind::ClassDecl(ClassDecl {
                name,
                alias,
                members,
            }),
            if has_block {
                find_family_decl_end(lines, start)
            } else {
                start
            },
        )));
    }

    for (keyword, marker) in [("map", Some("<<map>>")), ("object", None)] {
        let Some((name, alias, has_block, stereotypes)) = parse_named_family_decl(line, keyword)
        else {
            continue;
        };
        let members = if has_block {
            let mut members = parse_family_decl_members(lines, start, keyword, &name)?;
            if let Some(marker) = marker {
                members.insert(
                    0,
                    ClassMember {
                        text: marker.to_string(),
                        modifier: None,
                    },
                );
            }
            for stereotype in stereotypes.iter().rev() {
                members.insert(
                    0,
                    ClassMember {
                        text: format!("<<{stereotype}>>"),
                        modifier: None,
                    },
                );
            }
            members
        } else {
            declaration_marker_members(marker, stereotypes)
        };
        return Ok(Some((
            StatementKind::ObjectDecl(ObjectDecl {
                name,
                alias,
                members,
            }),
            if has_block {
                find_family_decl_end(lines, start)
            } else {
                start
            },
        )));
    }

    if let Some((name, alias, has_block)) = parse_parenthesized_usecase_decl(line) {
        return Ok(Some((
            StatementKind::UseCaseDecl(UseCaseDecl {
                name,
                alias,
                members: Vec::new(),
            }),
            if has_block {
                find_family_decl_end(lines, start)
            } else {
                start
            },
        )));
    }

    for (keyword, marker) in [("actor", Some("<<actor>>")), ("usecase", None)] {
        let Some((name, alias, has_block, stereotypes)) = parse_named_family_decl(line, keyword)
        else {
            continue;
        };
        let members = if has_block {
            let mut members = parse_family_decl_members(lines, start, keyword, &name)?;
            if let Some(marker) = marker {
                members.insert(
                    0,
                    ClassMember {
                        text: marker.to_string(),
                        modifier: None,
                    },
                );
            }
            for stereotype in stereotypes.iter().rev() {
                members.insert(
                    0,
                    ClassMember {
                        text: format!("<<{stereotype}>>"),
                        modifier: None,
                    },
                );
            }
            members
        } else {
            declaration_marker_members(marker, stereotypes)
        };
        return Ok(Some((
            StatementKind::UseCaseDecl(UseCaseDecl {
                name,
                alias,
                members,
            }),
            if has_block {
                find_family_decl_end(lines, start)
            } else {
                start
            },
        )));
    }
    Ok(None)
}

fn later_lines_contain_class_family_declaration(lines: &[(&str, Span)], start: usize) -> bool {
    lines.iter().skip(start + 1).any(|(raw, _)| {
        let line = raw.trim();
        line.starts_with("abstract class ")
            || line.starts_with("abstract ")
            || line.starts_with("annotation ")
            || line.starts_with("class ")
            || line.starts_with("enum ")
            || line.starts_with("protocol ")
            || line.starts_with("struct ")
    })
}

fn later_lines_contain_usecase_family_declaration(lines: &[(&str, Span)], start: usize) -> bool {
    lines.iter().skip(start + 1).any(|(raw, _)| {
        let line = raw.trim();
        line.starts_with("usecase ") || line.starts_with("usecase(")
    })
}

fn parse_named_family_decl(
    line: &str,
    keyword: &str,
) -> Option<(String, Option<String>, bool, Vec<String>)> {
    if !line.starts_with(keyword) {
        return None;
    }
    if line.len() > keyword.len()
        && !line[keyword.len()..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
    {
        return None;
    }
    let rest = line[keyword.len()..].trim();
    if rest.is_empty() {
        return None;
    }

    let has_block = rest.ends_with('{');
    let trimmed = if has_block {
        rest.trim_end_matches('{').trim()
    } else {
        rest
    };

    let (name_raw, alias_raw) = if let Some((lhs, rhs)) = trimmed.split_once(" as ") {
        (lhs.trim(), Some(rhs.trim()))
    } else {
        (trimmed, None)
    };

    let (name_without_stereotypes, stereotypes) = strip_declaration_stereotypes(name_raw);
    let name = clean_ident(&name_without_stereotypes);
    if name.is_empty() {
        return None;
    }
    let alias = alias_raw.map(clean_ident).filter(|v| !v.is_empty());
    Some((name, alias, has_block, stereotypes))
}

fn declaration_marker_members(marker: Option<&str>, stereotypes: Vec<String>) -> Vec<ClassMember> {
    let mut members = Vec::new();
    if let Some(marker) = marker {
        members.push(ClassMember {
            text: marker.to_string(),
            modifier: None,
        });
    }
    for stereotype in stereotypes {
        members.push(ClassMember {
            text: format!("<<{stereotype}>>"),
            modifier: None,
        });
    }
    members
}

fn strip_declaration_stereotypes(input: &str) -> (String, Vec<String>) {
    let mut remaining = input.trim().to_string();
    let mut stereotypes = Vec::new();
    while let Some(start) = remaining.find("<<") {
        let Some(end_rel) = remaining[start + 2..].find(">>") else {
            break;
        };
        let end = start + 2 + end_rel;
        let value = remaining[start + 2..end].trim();
        if !value.is_empty() {
            stereotypes.push(value.to_string());
        }
        remaining.replace_range(start..end + 2, "");
    }
    (remaining.trim().to_string(), stereotypes)
}

fn parse_parenthesized_usecase_decl(line: &str) -> Option<(String, Option<String>, bool)> {
    let trimmed = line.trim();
    let trimmed = trimmed.strip_prefix("usecase ").unwrap_or(trimmed).trim();
    if !trimmed.starts_with('(') {
        return None;
    }
    let close = trimmed.find(')')?;
    let name_raw = trimmed[1..close].trim();
    if name_raw.is_empty() {
        return None;
    }
    let rest = trimmed[close + 1..].trim();
    let has_block = rest.ends_with('{');
    let rest = if has_block {
        rest.trim_end_matches('{').trim()
    } else {
        rest
    };
    let alias = rest
        .strip_prefix("as ")
        .map(str::trim)
        .map(clean_ident)
        .filter(|v| !v.is_empty());
    Some((clean_ident(name_raw), alias, has_block))
}

fn parse_family_decl_members(
    lines: &[(&str, Span)],
    start: usize,
    keyword: &str,
    name: &str,
) -> Result<Vec<ClassMember>, Diagnostic> {
    let end_idx = find_family_decl_end(lines, start);
    if end_idx == start {
        return Err(Diagnostic::error(format!(
            "[E_FAMILY_DECL_BLOCK_UNCLOSED] unclosed {keyword} declaration block for `{name}`: missing `}}`",
        ))
        .with_span(lines[start].1));
    }
    let mut members = Vec::new();
    for (raw, _) in lines.iter().take(end_idx).skip(start + 1) {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            members.push(parse_class_member(trimmed));
        }
    }
    Ok(members)
}

/// Parse a single member line, extracting any `{field}`, `{method}`, `{abstract}`,
/// `{static}`, or `{class}` modifier token (trailing or leading), as well as
/// `<<abstract>>` and `<<static>>` stereotype tokens.
fn parse_class_member(raw: &str) -> ClassMember {
    // Check for leading brace modifier: `{field} +id: UUID`
    if let Some(rest) = try_strip_leading_brace_modifier(raw) {
        let modifier = parse_brace_modifier_word(leading_brace_word(raw));
        return ClassMember {
            text: rest.trim().to_string(),
            modifier,
        };
    }

    // Check for trailing brace modifier: `+id: UUID {field}`
    if let Some((text_part, mod_word)) = try_strip_trailing_brace_modifier(raw) {
        let modifier = parse_brace_modifier_word(mod_word);
        return ClassMember {
            text: text_part.trim().to_string(),
            modifier,
        };
    }

    // Check for leading `<<abstract>>` or `<<static>>` stereotype
    if let Some((modifier, rest)) = try_strip_leading_stereotype_modifier(raw) {
        return ClassMember {
            text: rest.trim().to_string(),
            modifier: Some(modifier),
        };
    }

    // Check for trailing `<<abstract>>` or `<<static>>` stereotype
    if let Some((text_part, modifier)) = try_strip_trailing_stereotype_modifier(raw) {
        return ClassMember {
            text: text_part.trim().to_string(),
            modifier: Some(modifier),
        };
    }

    ClassMember {
        text: raw.to_string(),
        modifier: None,
    }
}

fn leading_brace_word(s: &str) -> &str {
    // returns the content between the first { and }
    if let Some(rest) = s.strip_prefix('{') {
        if let Some(end) = rest.find('}') {
            return rest[..end].trim();
        }
    }
    ""
}

fn try_strip_leading_brace_modifier(s: &str) -> Option<&str> {
    let s = s.trim_start();
    if !s.starts_with('{') {
        return None;
    }
    let rest = &s[1..];
    let end = rest.find('}')?;
    let word = rest[..end].trim();
    if is_member_modifier_word(word) {
        Some(rest[end + 1..].trim())
    } else {
        None
    }
}

fn try_strip_trailing_brace_modifier(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_end();
    if !s.ends_with('}') {
        return None;
    }
    let start = s.rfind('{')?;
    let word = s[start + 1..s.len() - 1].trim();
    if is_member_modifier_word(word) {
        Some((&s[..start], word))
    } else {
        None
    }
}

fn try_strip_leading_stereotype_modifier(s: &str) -> Option<(MemberModifier, &str)> {
    let s = s.trim_start();
    if !s.starts_with("<<") {
        return None;
    }
    let rest = &s[2..];
    let end = rest.find(">>")?;
    let word = rest[..end].trim();
    let modifier = match word.to_ascii_lowercase().as_str() {
        "abstract" => MemberModifier::Abstract,
        "static" => MemberModifier::Static,
        _ => return None,
    };
    Some((modifier, rest[end + 2..].trim()))
}

fn try_strip_trailing_stereotype_modifier(s: &str) -> Option<(&str, MemberModifier)> {
    let s = s.trim_end();
    if !s.ends_with(">>") {
        return None;
    }
    let start = s.rfind("<<")?;
    let word = s[start + 2..s.len() - 2].trim();
    let modifier = match word.to_ascii_lowercase().as_str() {
        "abstract" => MemberModifier::Abstract,
        "static" => MemberModifier::Static,
        _ => return None,
    };
    Some((&s[..start], modifier))
}

fn is_member_modifier_word(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "field" | "method" | "abstract" | "static" | "class"
    )
}

fn parse_brace_modifier_word(word: &str) -> Option<MemberModifier> {
    match word.to_ascii_lowercase().as_str() {
        "field" => Some(MemberModifier::Field),
        "method" => Some(MemberModifier::Method),
        "abstract" => Some(MemberModifier::Abstract),
        "static" | "class" => Some(MemberModifier::Static),
        _ => None,
    }
}

fn find_family_decl_end(lines: &[(&str, Span)], start: usize) -> usize {
    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        if raw.trim() == "}" {
            return idx;
        }
    }
    start
}

fn parse_family_relation(line: &str, family: Option<DiagramKind>) -> Option<StatementKind> {
    match family {
        Some(DiagramKind::Class)
        | Some(DiagramKind::Object)
        | Some(DiagramKind::UseCase)
        | Some(DiagramKind::Salt)
        | Some(DiagramKind::Component)
        | Some(DiagramKind::Deployment) => {}
        _ => return None,
    }

    let (core, label) = split_family_relation_label(line);
    let (lhs, arrow, rhs) = split_family_arrow(core)?;
    let (lhs_core, left_cardinality, left_role) = parse_relation_side_annotations(lhs, true);
    let (rhs_core, right_cardinality, right_role) = parse_relation_side_annotations(rhs, false);
    if normalize_virtual_endpoint(&lhs_core).is_some()
        || normalize_virtual_endpoint(&rhs_core).is_some()
        || looks_like_virtual_endpoint_syntax(&lhs_core)
        || looks_like_virtual_endpoint_syntax(&rhs_core)
    {
        return None;
    }
    let from = clean_bracketed_ident(&lhs_core);
    let to = clean_bracketed_ident(&rhs_core);
    if from.is_empty() || to.is_empty() {
        return None;
    }
    Some(StatementKind::FamilyRelation(FamilyRelation {
        from,
        to,
        arrow,
        label,
        left_cardinality,
        right_cardinality,
        left_role,
        right_role,
    }))
}

fn parse_family_member_row(line: &str, family: Option<DiagramKind>) -> Option<StatementKind> {
    let family = match family {
        Some(DiagramKind::Class | DiagramKind::Object | DiagramKind::UseCase) => family?,
        _ => return None,
    };
    if split_family_arrow(line).is_some() {
        return None;
    }
    let (owner, member) = line.split_once(':')?;
    if owner.contains("--") || owner.contains("..") || owner.contains("->") || owner.contains("<-")
    {
        return None;
    }
    let owner = clean_bracketed_ident(owner);
    let member = member.trim();
    if owner.is_empty() || member.is_empty() {
        return None;
    }
    let members = vec![parse_class_member(member)];
    Some(match family {
        DiagramKind::Object => StatementKind::ObjectDecl(ObjectDecl {
            name: owner,
            alias: None,
            members,
        }),
        DiagramKind::UseCase => StatementKind::UseCaseDecl(UseCaseDecl {
            name: owner,
            alias: None,
            members,
        }),
        _ => StatementKind::ClassDecl(ClassDecl {
            name: owner,
            alias: None,
            members,
        }),
    })
}

fn parse_relation_side_annotations(
    side: &str,
    is_left: bool,
) -> (String, Option<String>, Option<String>) {
    let trimmed = side.trim();
    if trimmed.is_empty() {
        return (String::new(), None, None);
    }

    let mut rem = trimmed.to_string();
    let mut cardinality: Option<String> = None;
    let mut role: Option<String> = None;

    if is_left {
        loop {
            let t = rem.trim_end();
            if t.ends_with(']') {
                if let Some(start_bracket) = t.rfind('[') {
                    let value = t[start_bracket + 1..t.len() - 1].trim();
                    let endpoint = t[..start_bracket].trim_end();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if role.is_none() {
                            role = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            if let Some(stripped) = t.strip_suffix('"') {
                if let Some(start_quote) = stripped.rfind('"') {
                    let value = stripped[start_quote + 1..].trim();
                    let endpoint = t[..start_quote].trim_end();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if cardinality.is_none() {
                            cardinality = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            if let Some(colon) = t.rfind(" :") {
                let value = t[colon + 2..].trim();
                let endpoint = t[..colon].trim_end();
                if !value.is_empty() && !endpoint.is_empty() {
                    if role.is_none() {
                        role = Some(value.to_string());
                    }
                    rem = endpoint.to_string();
                    continue;
                }
            }
            break;
        }
    } else {
        loop {
            let t = rem.trim_start();
            if let Some(rest) = t.strip_prefix('"') {
                if let Some(end_quote_rel) = rest.find('"') {
                    let value = rest[..end_quote_rel].trim();
                    let endpoint = rest[end_quote_rel + 1..].trim_start();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if cardinality.is_none() {
                            cardinality = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            if let Some(rest) = t.strip_prefix('[') {
                if let Some(end_bracket_rel) = rest.find(']') {
                    let value = rest[..end_bracket_rel].trim();
                    let endpoint = rest[end_bracket_rel + 1..].trim_start();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if role.is_none() {
                            role = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            if let Some(rest) = t.strip_prefix(':') {
                let value_len = rest
                    .char_indices()
                    .take_while(|(_, ch)| !ch.is_whitespace())
                    .map(|(idx, ch)| idx + ch.len_utf8())
                    .last()
                    .unwrap_or(0);
                if value_len > 0 {
                    let value = rest[..value_len].trim();
                    let endpoint = rest[value_len..].trim_start();
                    if !value.is_empty() && !endpoint.is_empty() {
                        if role.is_none() {
                            role = Some(value.to_string());
                        }
                        rem = endpoint.to_string();
                        continue;
                    }
                }
            }
            break;
        }
    }

    (rem.trim().to_string(), cardinality, role)
}

fn split_family_arrow(core: &str) -> Option<(&str, String, &str)> {
    for (idx, ch) in core.char_indices() {
        if !matches!(ch, '-' | '.' | '<' | '*' | 'o' | '+') {
            continue;
        }
        let rest = &core[idx..];
        let Some(len) = family_arrow_token_len(rest) else {
            continue;
        };
        let lhs = core[..idx].trim();
        let rhs = core[idx + len..].trim();
        if lhs.is_empty() || rhs.is_empty() {
            continue;
        }
        let arrow = normalize_family_arrow_token(&rest[..len]);
        if arrow.is_empty() {
            continue;
        }
        return Some((lhs, arrow, rhs));
    }
    None
}

fn family_arrow_token_len(s: &str) -> Option<usize> {
    if let Some(len) = directional_family_arrow_token_len(s) {
        return Some(len);
    }

    let len = s
        .char_indices()
        .take_while(|(_, ch)| matches!(ch, '-' | '.' | '<' | '>' | '|' | '*' | 'o' | '+'))
        .map(|(idx, ch)| idx + ch.len_utf8())
        .last()?;
    let token = &s[..len];
    if is_family_arrow_token(token) {
        Some(len)
    } else {
        None
    }
}

fn directional_family_arrow_token_len(s: &str) -> Option<usize> {
    let dirs = ["left", "right", "up", "down", "l", "r", "u", "d"];
    for prefix_len in 1..=2 {
        let prefix = s.get(..prefix_len)?;
        if !prefix.chars().all(|ch| matches!(ch, '-' | '.')) {
            continue;
        }
        let after_prefix = &s[prefix_len..];
        if let Some(after_directive) = after_prefix.strip_prefix('[') {
            if let Some(close) = after_directive.find(']') {
                let after = &after_directive[close + 1..];
                let suffix_len = after
                    .char_indices()
                    .take_while(|(_, ch)| matches!(ch, '-' | '.' | '<' | '>' | '|'))
                    .map(|(idx, ch)| idx + ch.len_utf8())
                    .last()
                    .unwrap_or(0);
                if suffix_len > 0 {
                    return Some(prefix_len + close + 2 + suffix_len);
                }
            }
        }
        for dir in dirs {
            if let Some(after_dir) = after_prefix.strip_prefix(dir) {
                let suffix_len = after_dir
                    .char_indices()
                    .take_while(|(_, ch)| matches!(ch, '-' | '.' | '<' | '>' | '|'))
                    .map(|(idx, ch)| idx + ch.len_utf8())
                    .last()
                    .unwrap_or(0);
                if suffix_len > 0 {
                    return Some(prefix_len + dir.len() + suffix_len);
                }
            }
        }
    }
    None
}

fn is_family_arrow_token(token: &str) -> bool {
    token.contains('-') || token.contains('<') || token.contains('>') || token.contains("..")
}

fn normalize_family_arrow_token(token: &str) -> String {
    let mut out = String::new();
    let mut chars = token.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '[' {
            for next in chars.by_ref() {
                if next == ']' {
                    break;
                }
            }
            continue;
        }
        if ch.is_ascii_alphabetic() {
            continue;
        }
        out.push(ch);
    }
    out
}

fn clean_bracketed_ident(s: &str) -> String {
    let trimmed = s.trim();
    // Preserve special state markers like [*] verbatim.
    if trimmed == "[*]" || trimmed == "[H]" || trimmed == "[H*]" {
        return trimmed.to_string();
    }
    // Allow `[Name]` shorthand: strip the surrounding brackets if balanced and no interior bracket.
    if let Some(inner) = trimmed.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        if !inner.contains('[') && !inner.contains(']') && !inner.is_empty() {
            return clean_ident(inner);
        }
    }
    // Strip `()` interface-style prefix `() Name`.
    if let Some(rest) = trimmed.strip_prefix("()") {
        return clean_ident(rest.trim());
    }
    clean_ident(trimmed)
}

/// Parse `together { ... }`, `package "name" { ... }`, `namespace ns { ... }` blocks.
/// Returns (StatementKind, end_line_index) where end_line_index points to the closing `}`.
fn parse_class_scoping_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    let lower = line.to_ascii_lowercase();

    // together { ... }
    if lower == "together {" || lower.starts_with("together {") {
        let end_idx = find_family_decl_end(lines, start);
        if end_idx == start {
            return Err(Diagnostic::error(
                "[E_CLASS_TOGETHER_UNCLOSED] unclosed `together` block: missing `}`",
            )
            .with_span(lines[start].1));
        }
        let members: Vec<String> = lines[start + 1..end_idx]
            .iter()
            .map(|(raw, _)| raw.trim())
            .filter(|s| !s.is_empty())
            .map(clean_ident)
            .filter(|s| !s.is_empty())
            .collect();
        return Ok(Some((
            StatementKind::ClassGroup {
                kind: "together".to_string(),
                label: None,
                members,
            },
            end_idx,
        )));
    }

    // package "label" { ... } or package label { ... }
    if lower.starts_with("package ") && line.trim_end().ends_with('{') {
        let rest = line.strip_prefix("package ").unwrap_or("").trim();
        let label_raw = rest.trim_end_matches('{').trim();
        let label = clean_ident(label_raw.trim_matches('"'));
        let end_idx = find_scoping_block_end(lines, start);
        if end_idx == start {
            return Err(Diagnostic::error(
                "[E_CLASS_PACKAGE_UNCLOSED] unclosed `package` block: missing `}`",
            )
            .with_span(lines[start].1));
        }
        if group_body_contains_component_family(lines, start, end_idx) {
            return Ok(None);
        }
        let members =
            collect_scoped_class_group_members(lines, start, end_idx, std::slice::from_ref(&label));
        return Ok(Some((
            StatementKind::ClassGroup {
                kind: "package".to_string(),
                label: if label.is_empty() { None } else { Some(label) },
                members,
            },
            end_idx,
        )));
    }

    // namespace ns { ... }
    if lower.starts_with("namespace ") && line.trim_end().ends_with('{') {
        let rest = line.strip_prefix("namespace ").unwrap_or("").trim();
        let label_raw = rest.trim_end_matches('{').trim();
        let label = clean_ident(label_raw.trim_matches('"'));
        let end_idx = find_scoping_block_end(lines, start);
        if end_idx == start {
            return Err(Diagnostic::error(
                "[E_CLASS_NAMESPACE_UNCLOSED] unclosed `namespace` block: missing `}`",
            )
            .with_span(lines[start].1));
        }
        let members =
            collect_scoped_class_group_members(lines, start, end_idx, std::slice::from_ref(&label));
        return Ok(Some((
            StatementKind::ClassGroup {
                kind: "namespace".to_string(),
                label: if label.is_empty() { None } else { Some(label) },
                members,
            },
            end_idx,
        )));
    }

    Ok(None)
}

fn collect_scoped_class_group_members(
    lines: &[(&str, Span)],
    start: usize,
    end_idx: usize,
    scope: &[String],
) -> Vec<String> {
    let mut members = Vec::new();
    let mut idx = start + 1;
    while idx < end_idx {
        let line = lines[idx].0.trim();
        let lower = line.to_ascii_lowercase();
        if line.is_empty() || line == "}" {
            idx += 1;
            continue;
        }
        if (lower.starts_with("package ") || lower.starts_with("namespace "))
            && line.trim_end().ends_with('{')
        {
            let keyword = if lower.starts_with("package ") {
                "package"
            } else {
                "namespace"
            };
            let label = clean_ident(
                line[keyword.len()..]
                    .trim()
                    .trim_end_matches('{')
                    .trim()
                    .trim_matches('"'),
            );
            let nested_end = find_scoping_block_end(lines, idx);
            if nested_end > idx {
                let mut nested_scope = scope.to_vec();
                if !label.is_empty() {
                    nested_scope.push(label);
                }
                members.extend(collect_scoped_class_group_members(
                    lines,
                    idx,
                    nested_end,
                    &nested_scope,
                ));
                idx = nested_end + 1;
                continue;
            }
        }
        for keyword in [
            "abstract class",
            "annotation",
            "interface",
            "abstract",
            "enum",
            "class",
        ] {
            if let Some((name, _alias, true, _stereotypes)) = parse_named_family_decl(line, keyword)
            {
                let scoped_name = if scope.iter().all(|s| s.is_empty()) {
                    name
                } else {
                    format!(
                        "{}::{}",
                        scope
                            .iter()
                            .filter(|s| !s.is_empty())
                            .cloned()
                            .collect::<Vec<_>>()
                            .join("::"),
                        name
                    )
                };
                let nested_end = find_family_decl_end(lines, idx);
                let members_text = parse_family_decl_members(lines, idx, keyword, &scoped_name)
                    .map(|members| {
                        members
                            .into_iter()
                            .map(|member| member.text)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                let mut encoded = scoped_name;
                for member in members_text {
                    encoded.push('\t');
                    encoded.push_str(&member);
                }
                members.push(encoded);
                if nested_end > idx {
                    idx = nested_end + 1;
                    continue;
                }
            }
        }
        let name = extract_class_member_name(line);
        if !name.is_empty() {
            let scoped = if scope.iter().all(|s| s.is_empty()) {
                name
            } else {
                format!(
                    "{}::{}",
                    scope
                        .iter()
                        .filter(|s| !s.is_empty())
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("::"),
                    name
                )
            };
            members.push(scoped);
        }
        if line.ends_with('{') {
            let nested_end = find_family_decl_end(lines, idx);
            if nested_end > idx {
                idx = nested_end + 1;
                continue;
            }
        }
        idx += 1;
    }
    members
}

fn group_body_contains_component_family(
    lines: &[(&str, Span)],
    start: usize,
    end_idx: usize,
) -> bool {
    lines[start + 1..end_idx].iter().any(|(raw, _)| {
        let line = strip_inline_plantuml_comment(raw).trim();
        let lower = line.to_ascii_lowercase();
        lower.starts_with("component ")
            || lower.starts_with("node ")
            || lower.starts_with("artifact ")
            || lower.starts_with("database ")
            || lower.starts_with("cloud ")
            || lower.starts_with("frame ")
            || lower.starts_with("storage ")
            || lower.starts_with("rectangle ")
            || lower.starts_with("folder ")
            || lower.starts_with("file ")
            || lower.starts_with("card ")
            || lower.starts_with("actor ")
            || lower.starts_with("port ")
            || lower.starts_with("portin ")
            || lower.starts_with("portout ")
    })
}

fn parse_component_scoping_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let Some((kind, label_raw)) = lower
        .starts_with("package ")
        .then(|| {
            (
                "package",
                trimmed.strip_prefix("package ").unwrap_or("").trim(),
            )
        })
        .or_else(|| {
            lower
                .starts_with("node ")
                .then(|| ("node", trimmed.strip_prefix("node ").unwrap_or("").trim()))
        })
        .or_else(|| {
            lower
                .starts_with("frame ")
                .then(|| ("frame", trimmed.strip_prefix("frame ").unwrap_or("").trim()))
        })
        .or_else(|| {
            lower
                .starts_with("cloud ")
                .then(|| ("cloud", trimmed.strip_prefix("cloud ").unwrap_or("").trim()))
        })
        .or_else(|| {
            lower.starts_with("rectangle ").then(|| {
                (
                    "rectangle",
                    trimmed.strip_prefix("rectangle ").unwrap_or("").trim(),
                )
            })
        })
    else {
        return Ok(None);
    };
    if !trimmed.ends_with('{') {
        return Ok(None);
    }
    let end_idx = find_scoping_block_end(lines, start);
    if end_idx == start {
        return Err(Diagnostic::error(format!(
            "[E_COMPONENT_GROUP_UNCLOSED] unclosed `{kind}` block: missing `}}`",
        ))
        .with_span(lines[start].1));
    }
    let label = clean_ident(label_raw.trim_end_matches('{').trim().trim_matches('"'));
    let members = lines[start + 1..end_idx]
        .iter()
        .map(|(raw, _)| raw.trim())
        .filter(|s| !s.is_empty() && *s != "}")
        .map(extract_component_group_member_name)
        .filter(|s| !s.is_empty())
        .collect();
    Ok(Some((
        StatementKind::ClassGroup {
            kind: kind.to_string(),
            label: if label.is_empty() { None } else { Some(label) },
            members,
        },
        end_idx,
    )))
}

fn find_scoping_block_end(lines: &[(&str, Span)], start: usize) -> usize {
    let mut depth = 0usize;
    for (idx, (raw, _)) in lines.iter().enumerate().skip(start) {
        let trimmed = strip_inline_plantuml_comment(raw).trim();
        if trimmed.ends_with('{') {
            depth += 1;
        }
        if trimmed == "}" {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return idx;
            }
        }
    }
    start
}

fn detect_non_sequence_family(line: &str) -> Option<DiagramKind> {
    if line.starts_with("component ")
        || line.starts_with("interface ")
        || line.starts_with("port ")
        || line.starts_with("portin ")
        || line.starts_with("portout ")
        || line.starts_with("package ")
        || line.starts_with("rectangle ")
        || line.starts_with("folder ")
        || line.starts_with("file ")
        || line.starts_with("card ")
        || line.starts_with("actor ")
    {
        return Some(DiagramKind::Component);
    }

    if line.starts_with("node ")
        || line.starts_with("artifact ")
        || line.starts_with("cloud ")
        || line.starts_with("frame ")
        || line.starts_with("storage ")
        || line.starts_with("database ")
    {
        return Some(DiagramKind::Deployment);
    }

    if line.starts_with("state ") || line == "[*]" || line == "[H]" || line == "[H*]" {
        return Some(DiagramKind::State);
    }
    // State transitions involving pseudo-states
    if (line.starts_with("[*]") || line.starts_with("[H]") || line.starts_with("[H*]"))
        && line.contains("-->")
    {
        return Some(DiagramKind::State);
    }
    // Any line that is `X --> Y` where Y is `[*]`, `[H]`, or `[H*]`
    if line.contains("-->") {
        if let Some(idx) = line.find("-->") {
            let rhs = line[idx + 3..].trim();
            // Strip label part
            let rhs_base = rhs.split(':').next().unwrap_or(rhs).trim();
            if matches!(rhs_base, "[*]" | "[H]" | "[H*]") {
                return Some(DiagramKind::State);
            }
        }
    }

    if line.starts_with('*')
        || line.starts_with('+')
        || line.starts_with('-')
        || line.starts_with('#')
    {
        return Some(DiagramKind::MindMap);
    }

    if line.starts_with("wbs ") {
        return Some(DiagramKind::Wbs);
    }

    if line.starts_with("start")
        || line.starts_with("stop")
        || line.starts_with(':')
        || line.starts_with("(*)")
        || line.starts_with("if ")
        || line.starts_with("elseif ")
        || line == "else"
        || line.starts_with("endif")
        || line.starts_with("repeat")
        || line.starts_with("while ")
        || line.starts_with("fork")
        || line.starts_with("partition ")
        || line.starts_with("swimlane ")
        || line.starts_with('|')
        || line.starts_with("detach")
    {
        return Some(DiagramKind::Activity);
    }

    if line.starts_with("robust ")
        || line.starts_with("concise ")
        || line.starts_with("clock ")
        || line.starts_with("binary ")
        || line.starts_with('@')
        // Timing-specific scale syntax: "scale N as N" (maps clock units to pixels).
        // Plain "scale 1.5" / "scale 800*600" / "scale max N" is the output-scale
        // directive and should not be classified as a timing diagram.
        || (line.starts_with("scale ") && line.contains(" as "))
    {
        return Some(DiagramKind::Timing);
    }
    if line.starts_with("salt ") {
        return Some(DiagramKind::Salt);
    }

    if line.starts_with("salt ") {
        return Some(DiagramKind::Salt);
    }

    None
}

fn parse_gantt_baseline_statement(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if let Some(kind) = parse_keyword(trimmed) {
        return Some(kind);
    }

    if let Some(rest) = trimmed.strip_prefix("Project starts ") {
        let date = rest
            .trim()
            .strip_prefix("on ")
            .or_else(|| rest.trim().strip_prefix("the "))
            .unwrap_or_else(|| rest.trim())
            .trim();
        if is_iso_date_literal(date) {
            return Some(StatementKind::GanttConstraint {
                subject: "Project".to_string(),
                kind: "starts".to_string(),
                target: date.to_string(),
            });
        }
    }
    if let Some((start_date, end_date)) = parse_gantt_closed_date_range(trimmed) {
        return Some(StatementKind::GanttCalendarClosedDateRange {
            start_date,
            end_date,
        });
    }
    if let Some(day) = parse_gantt_closed_weekday(trimmed) {
        return Some(StatementKind::GanttCalendarClosed { day });
    }
    let (subject, rest) = parse_bracket_subject(trimmed)?;
    if rest.is_empty() {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources: Vec::new(),
        });
    }
    let rest = rest.trim();
    let (rest_without_resources, resources) = extract_gantt_resources(rest);
    let rest = rest_without_resources.trim();
    if let Some(rest) = rest.strip_prefix(':') {
        if subject.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            let label = rest.trim();
            if !label.is_empty() {
                return Some(StatementKind::GanttMilestoneDecl {
                    name: label.to_string(),
                    happens_on: Some(subject),
                });
            }
        }
        return Some(StatementKind::GanttTaskDecl {
            name: rest.trim().to_string(),
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources,
        });
    }
    if let Some((start_date, duration_days)) = parse_gantt_start_and_duration(rest) {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            start_date: Some(start_date),
            duration_days: Some(duration_days),
            depends_on: Vec::new(),
            resources,
        });
    }
    if let Some(duration_days) = parse_gantt_duration_clause(rest) {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            start_date: None,
            duration_days: Some(duration_days),
            depends_on: Vec::new(),
            resources,
        });
    }
    if let Some(start_date) = parse_gantt_start_date_clause(rest) {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            start_date: Some(start_date),
            duration_days: None,
            depends_on: Vec::new(),
            resources,
        });
    }
    if !resources.is_empty() {
        return Some(StatementKind::GanttTaskDecl {
            name: subject,
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources,
        });
    }
    let rest = rest.trim();
    if let Some(rest) = rest.strip_prefix(':') {
        return Some(StatementKind::GanttTaskDecl {
            name: rest.trim().to_string(),
            start_date: None,
            duration_days: None,
            depends_on: Vec::new(),
            resources: Vec::new(),
        });
    }
    let lower = rest.to_ascii_lowercase();
    if lower.starts_with("happens") {
        return Some(StatementKind::GanttMilestoneDecl {
            name: subject,
            happens_on: parse_gantt_happens_target(rest),
        });
    }
    for kind in ["starts", "ends", "requires"] {
        if lower.starts_with(kind) {
            let target = rest[kind.len()..]
                .trim()
                .strip_prefix("at ")
                .unwrap_or_else(|| rest[kind.len()..].trim())
                .trim()
                .to_string();
            return Some(StatementKind::GanttConstraint {
                subject,
                kind: kind.to_string(),
                target,
            });
        }
    }
    None
}

fn parse_gantt_closed_weekday(line: &str) -> Option<String> {
    let lower = line.trim().to_ascii_lowercase();
    let day = [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ]
    .into_iter()
    .find(|day| {
        lower == format!("{day} is closed")
            || lower == format!("{day} are closed")
            || lower == format!("{day}s are closed")
    })?;
    Some(day.to_string())
}

fn parse_gantt_closed_date_range(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let suffix_len = if lower.ends_with(" is closed") {
        " is closed".len()
    } else if lower.ends_with(" are closed") {
        " are closed".len()
    } else {
        return None;
    };
    let range = trimmed[..trimmed.len().saturating_sub(suffix_len)].trim();
    let lower_range = lower[..lower.len().saturating_sub(suffix_len)].trim();
    let sep = " to ";
    let idx = lower_range.find(sep)?;
    let start_date = range[..idx].trim();
    let end_date = range[idx + sep.len()..].trim();
    if !is_iso_date_literal(start_date) || !is_iso_date_literal(end_date) {
        return None;
    }
    Some((start_date.to_string(), end_date.to_string()))
}

fn parse_gantt_start_and_duration(rest: &str) -> Option<(String, u32)> {
    let lower = rest.to_ascii_lowercase();
    let (idx, marker_len) = lower
        .find(" and lasts ")
        .map(|idx| (idx, " and lasts ".len()))
        .or_else(|| {
            lower
                .find(" and requires ")
                .map(|idx| (idx, " and requires ".len()))
        })?;
    let start_clause = rest[..idx].trim();
    let duration_clause = rest[idx + marker_len..].trim();
    let start_date = parse_gantt_start_date_clause(start_clause)?;
    Some((start_date, parse_gantt_duration_clause(duration_clause)?))
}

fn parse_gantt_start_date_clause(rest: &str) -> Option<String> {
    let start_date = rest
        .trim()
        .strip_prefix("starts ")?
        .trim()
        .strip_prefix("at ")
        .unwrap_or_else(|| rest.trim().strip_prefix("starts ").unwrap().trim())
        .trim();
    if !is_iso_date_literal(start_date) {
        return None;
    }
    Some(start_date.to_string())
}

fn parse_gantt_duration_clause(rest: &str) -> Option<u32> {
    let trimmed = rest.trim();
    let clause = trimmed
        .strip_prefix("lasts ")
        .or_else(|| trimmed.strip_prefix("requires "))
        .map(str::trim)
        .unwrap_or(trimmed);
    let mut total = 0u32;
    let mut parts = clause.split_whitespace().peekable();
    while parts.peek().is_some() {
        if parts.peek().copied() == Some("and") {
            parts.next();
            continue;
        }
        let n = parts.next()?.parse::<u32>().ok()?;
        let unit = parts.next()?.to_ascii_lowercase();
        let days = match unit.as_str() {
            "day" | "days" => n,
            "week" | "weeks" => n.saturating_mul(7),
            _ => return None,
        };
        total = total.saturating_add(days);
    }
    if total == 0 {
        None
    } else {
        Some(total)
    }
}

fn extract_gantt_resources(rest: &str) -> (String, Vec<String>) {
    let lower = rest.to_ascii_lowercase();
    let Some(on_idx) = lower
        .find(" on {")
        .or_else(|| lower.strip_prefix("on {").map(|_| 0))
    else {
        return (rest.to_string(), Vec::new());
    };
    let mut cursor = if on_idx == 0 {
        "on ".len()
    } else {
        on_idx + " on ".len()
    };
    let mut resources = Vec::new();
    while cursor < rest.len() {
        let skipped = rest[cursor..].len() - rest[cursor..].trim_start().len();
        cursor += skipped;
        if !rest[cursor..].starts_with('{') {
            break;
        }
        let Some(end_rel) = rest[cursor + 1..].find('}') else {
            break;
        };
        let end = cursor + 1 + end_rel;
        let resource = rest[cursor + 1..end].trim();
        if !resource.is_empty() {
            resources.push(resource.to_string());
        }
        cursor = end + 1;
    }
    if resources.is_empty() {
        return (rest.to_string(), Vec::new());
    }
    let prefix = rest[..on_idx].trim_end();
    let suffix = rest[cursor..]
        .trim_start()
        .strip_prefix("and ")
        .unwrap_or_else(|| rest[cursor..].trim_start())
        .trim_start();
    let cleaned = if prefix.is_empty() {
        suffix.to_string()
    } else if suffix.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix} {suffix}")
    };
    (cleaned, resources)
}

fn parse_gantt_happens_target(rest: &str) -> Option<String> {
    let lower = rest.to_ascii_lowercase();
    let target = lower
        .strip_prefix("happens on ")
        .and_then(|_| rest.get("happens on ".len()..))
        .or_else(|| {
            lower
                .strip_prefix("happens at ")
                .and_then(|_| rest.get("happens at ".len()..))
        })?
        .trim();
    if target.is_empty() {
        None
    } else {
        Some(target.to_string())
    }
}

fn is_iso_date_literal(raw: &str) -> bool {
    let mut parts = raw.trim().split('-');
    let Some(y) = parts.next() else {
        return false;
    };
    let Some(m) = parts.next() else {
        return false;
    };
    let Some(d) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    if y.len() != 4 || m.len() != 2 || d.len() != 2 {
        return false;
    }
    y.chars().all(|c| c.is_ascii_digit())
        && m.chars().all(|c| c.is_ascii_digit())
        && d.chars().all(|c| c.is_ascii_digit())
}

fn parse_component_decl(line: &str) -> Option<StatementKind> {
    use crate::ast::ComponentNodeKind as K;
    let keywords: &[(&str, K)] = &[
        ("component", K::Component),
        ("interface", K::Interface),
        ("portin", K::Port),
        ("portout", K::Port),
        ("port", K::Port),
        ("node", K::Node),
        ("database", K::Database),
        ("cloud", K::Cloud),
        ("frame", K::Frame),
        ("storage", K::Storage),
        ("package", K::Package),
        ("rectangle", K::Rectangle),
        ("folder", K::Folder),
        ("file", K::File),
        ("card", K::Card),
        ("artifact", K::Artifact),
        ("actor", K::Actor),
    ];
    for (kw, kind) in keywords.iter().copied() {
        let trimmed = line.trim();
        if !trimmed.starts_with(kw) {
            continue;
        }
        let rest_raw = trimmed[kw.len()..].trim();
        if rest_raw.is_empty() {
            return None;
        }
        // Must be followed by whitespace OR the rest is a non-identifier prefix; require space.
        if !line
            .as_bytes()
            .get(kw.len())
            .copied()
            .is_some_and(|b| b == b' ' || b == b'\t')
        {
            // For the very first char after kw, ensure it's whitespace.
            // (line is already trimmed by caller; recompute on trimmed)
            let bytes = trimmed.as_bytes();
            if let Some(&b) = bytes.get(kw.len()) {
                if !(b == b' ' || b == b'\t') {
                    continue;
                }
            }
        }
        let rest = rest_raw.trim_end_matches('{').trim();
        let (label, rest_after_label) = if rest.starts_with('"') {
            let stripped = rest.strip_prefix('"')?;
            let end = stripped.find('"')?;
            (
                Some(stripped[..end].to_string()),
                stripped[end + 1..].trim(),
            )
        } else {
            (None, rest)
        };
        let (name_raw, alias_raw) = if let Some(alias) = rest_after_label.strip_prefix("as ") {
            (label.as_deref().unwrap_or("").trim(), Some(alias.trim()))
        } else if let Some((lhs, rhs)) = rest_after_label.split_once(" as ") {
            (lhs.trim(), Some(rhs.trim()))
        } else {
            (rest_after_label, None)
        };
        let name = clean_bracketed_ident(name_raw);
        if name.is_empty() {
            return None;
        }
        let alias = alias_raw.map(clean_ident).filter(|v| !v.is_empty());
        return Some(StatementKind::ComponentDecl {
            kind,
            name,
            alias,
            label,
        });
    }
    // Anonymous shorthand: `[Name]` declares a component, `() Name` declares an interface.
    let trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let inner = rest[..end].trim();
            let suffix = rest[end + 1..].trim();
            let alias = suffix
                .strip_prefix("as ")
                .map(str::trim)
                .map(clean_ident)
                .filter(|v| !v.is_empty());
            if !inner.is_empty() && !inner.contains('[') && !inner.contains(']') {
                let name = alias.clone().unwrap_or_else(|| clean_ident(inner));
                let label = alias.as_ref().map(|_| inner.to_string());
                return Some(StatementKind::ComponentDecl {
                    kind: ComponentNodeKind::Component,
                    name,
                    alias,
                    label,
                });
            }
        }
    }
    if let Some(rest) = trimmed.strip_prefix("()") {
        let rest = rest.trim();
        if !rest.is_empty() {
            let (label, rest_after_label) = if rest.starts_with('"') {
                let stripped = rest.strip_prefix('"')?;
                let end = stripped.find('"')?;
                (
                    Some(stripped[..end].to_string()),
                    stripped[end + 1..].trim(),
                )
            } else {
                (None, rest)
            };
            let (name_raw, alias) = if let Some(alias) = rest_after_label.strip_prefix("as ") {
                (
                    label.as_deref().unwrap_or("").trim(),
                    Some(clean_ident(alias.trim())),
                )
            } else if let Some((lhs, rhs)) = rest_after_label.split_once(" as ") {
                (lhs.trim(), Some(clean_ident(rhs.trim())))
            } else {
                (rest_after_label, None)
            };
            let name = alias
                .clone()
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| clean_ident(name_raw));
            if !name.is_empty() {
                return Some(StatementKind::ComponentDecl {
                    kind: ComponentNodeKind::Interface,
                    name,
                    alias: alias.filter(|v| !v.is_empty()),
                    label,
                });
            }
        }
    }
    if let Some(inner) = trimmed.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        if !inner.is_empty() && !inner.contains('[') && !inner.contains(']') {
            return Some(StatementKind::ComponentDecl {
                kind: ComponentNodeKind::Component,
                name: clean_ident(inner),
                alias: None,
                label: None,
            });
        }
    }
    if let Some(rest) = trimmed.strip_prefix("()") {
        let rest = rest.trim();
        if !rest.is_empty() {
            return Some(StatementKind::ComponentDecl {
                kind: ComponentNodeKind::Interface,
                name: clean_ident(rest),
                alias: None,
                label: None,
            });
        }
    }
    None
}

fn parse_activity_step(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    // `:action;` or `:action` form
    if let Some(rest) = trimmed.strip_prefix(':') {
        let body = rest.trim_end_matches(';').trim();
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: if body.is_empty() {
                None
            } else {
                Some(body.to_string())
            },
        }));
    }
    if trimmed == "start" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Start,
            label: None,
        }));
    }
    if trimmed == "stop" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Stop,
            label: None,
        }));
    }
    if trimmed == "end" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::End,
            label: None,
        }));
    }
    if trimmed == "else" || trimmed.starts_with("else ") || trimmed.starts_with("else(") {
        let label = if trimmed == "else" {
            None
        } else {
            extract_paren_label(trimmed.trim_start_matches("else").trim())
        };
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Else,
            label,
        }));
    }
    if trimmed == "endif" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndIf,
            label: None,
        }));
    }
    if trimmed == "fork" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Fork,
            label: None,
        }));
    }
    if trimmed == "fork again" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::ForkAgain,
            label: None,
        }));
    }
    if trimmed == "end fork" || trimmed == "endfork" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndFork,
            label: None,
        }));
    }
    if trimmed == "split" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Fork,
            label: Some("split".to_string()),
        }));
    }
    if trimmed == "split again" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::ForkAgain,
            label: Some("split again".to_string()),
        }));
    }
    if trimmed == "end split" || trimmed == "endsplit" || trimmed == "end merge" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndFork,
            label: Some("end split".to_string()),
        }));
    }
    if trimmed == "endwhile" || trimmed == "end while" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndWhile,
            label: None,
        }));
    }
    if trimmed == "repeat" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::RepeatStart,
            label: None,
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("if ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::IfStart,
            label: Some(parse_activity_if_label(rest.trim())),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("switch ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::IfStart,
            label: Some(format!(
                "switch {}",
                extract_paren_label(rest.trim()).unwrap_or_else(|| rest.trim().to_string())
            )),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("case ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Else,
            label: extract_paren_label(rest.trim()).or_else(|| Some(rest.trim().to_string())),
        }));
    }
    if trimmed == "endswitch" || trimmed == "end switch" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::EndIf,
            label: Some("endswitch".to_string()),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("while ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::WhileStart,
            label: Some(
                extract_paren_label(rest.trim()).unwrap_or_else(|| rest.trim().to_string()),
            ),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("repeat while") {
        let r = rest.trim();
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::RepeatWhile,
            label: extract_paren_label(r).or_else(|| {
                if r.is_empty() {
                    None
                } else {
                    Some(r.to_string())
                }
            }),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("partition ") {
        let label = rest.trim().trim_end_matches('{').trim();
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::PartitionStart,
            label: Some(label.to_string()),
        }));
    }
    if trimmed == "}" {
        // Treat lone `}` inside activity as partition close.
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::PartitionEnd,
            label: None,
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("label ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: Some(format!("label {}", rest.trim())),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("goto ") {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: Some(format!("goto {}", rest.trim())),
        }));
    }
    if let Some(rest) = trimmed.strip_prefix("backward") {
        let label = rest
            .trim()
            .trim_start_matches(':')
            .trim_end_matches(';')
            .trim();
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Action,
            label: Some(if label.is_empty() {
                "backward".to_string()
            } else {
                format!("backward {label}")
            }),
        }));
    }
    if trimmed == "kill" || trimmed == "detach" {
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::Stop,
            label: Some(trimmed.to_string()),
        }));
    }
    None
}

fn parse_activity_if_label(input: &str) -> String {
    let lower = input.to_ascii_lowercase();
    if let Some(idx) = lower.find(" then ") {
        let condition_raw = input[..idx].trim();
        let then_raw = input[idx + " then ".len()..].trim();
        let condition =
            extract_paren_label(condition_raw).unwrap_or_else(|| condition_raw.to_string());
        if let Some(branch) = extract_paren_label(then_raw) {
            if !branch.is_empty() {
                return format!("{condition} / {branch}");
            }
        }
        return condition;
    }
    let body = input.trim_end_matches("then").trim();
    extract_paren_label(body).unwrap_or_else(|| body.to_string())
}

fn extract_paren_label(input: &str) -> Option<String> {
    let s = input.trim();
    let open = s.find('(')?;
    let close = s.rfind(')')?;
    if close <= open {
        return None;
    }
    Some(s[open + 1..close].trim().to_string())
}

fn parse_timing_decl(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    let kinds: &[(&str, TimingDeclKind)] = &[
        ("concise", TimingDeclKind::Concise),
        ("robust", TimingDeclKind::Robust),
        ("clock", TimingDeclKind::Clock),
        ("binary", TimingDeclKind::Binary),
    ];
    for (kw, kind) in kinds.iter().copied() {
        if let Some(rest) = trimmed.strip_prefix(kw) {
            if !rest.starts_with(char::is_whitespace) {
                continue;
            }
            let rest = rest.trim();
            if rest.is_empty() {
                return None;
            }
            let (label, name_raw) = if rest.starts_with('"') {
                let stripped = rest.strip_prefix('"')?;
                let end = stripped.find('"')?;
                let rem = stripped[end + 1..].trim();
                let name = rem.strip_prefix("as ").map(str::trim).unwrap_or(rem).trim();
                (Some(stripped[..end].to_string()), name)
            } else if let Some((lhs, rhs)) = rest.split_once(" as ") {
                (Some(lhs.trim().to_string()), rhs.trim())
            } else {
                (None, rest)
            };
            let (name_raw, controls) = split_timing_decl_controls(name_raw);
            let name = clean_ident(&name_raw);
            if name.is_empty() {
                return None;
            }
            return Some(StatementKind::TimingDecl {
                kind,
                name,
                label,
                controls,
            });
        }
    }
    None
}

fn split_timing_decl_controls(input: &str) -> (String, Vec<String>) {
    let trimmed = input.trim();
    let lower = trimmed.to_ascii_lowercase();
    if let Some(idx) = lower.find(" with ") {
        let name = trimmed[..idx].trim().to_string();
        let controls = trimmed[idx + " with ".len()..]
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
        return (name, controls);
    }
    (trimmed.to_string(), Vec::new())
}

fn parse_timing_event(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    // `@<time>` standalone, or `<signal> is <state>` or `@<time> <signal> is <state>`
    if let Some(rest) = trimmed.strip_prefix('@') {
        let rest = rest.trim();
        if rest.is_empty() {
            return Some(StatementKind::TimingEvent {
                time: String::new(),
                signal: None,
                state: None,
                note: None,
            });
        }
        // split at first whitespace
        let (time, after) = rest
            .split_once(char::is_whitespace)
            .map(|(a, b)| (a.trim().to_string(), b.trim()))
            .unwrap_or_else(|| (rest.to_string(), ""));
        if after.is_empty() {
            return Some(StatementKind::TimingEvent {
                time,
                signal: None,
                state: None,
                note: None,
            });
        }
        // after may contain "signal is state"
        if let Some((sig, state)) = split_is(after) {
            return Some(StatementKind::TimingEvent {
                time,
                signal: Some(sig),
                state: Some(state),
                note: None,
            });
        }
        return Some(StatementKind::TimingEvent {
            time,
            signal: None,
            state: None,
            note: Some(after.to_string()),
        });
    }
    if let Some((sig, state)) = split_is(trimmed) {
        return Some(StatementKind::TimingEvent {
            time: String::new(),
            signal: Some(sig),
            state: Some(state),
            note: None,
        });
    }
    None
}

fn split_is(s: &str) -> Option<(String, String)> {
    let needle = " is ";
    let idx = s.find(needle)?;
    let lhs = s[..idx].trim();
    let rhs = s[idx + needle.len()..].trim().trim_matches('"');
    if lhs.is_empty() || rhs.is_empty() {
        return None;
    }
    Some((lhs.to_string(), rhs.to_string()))
}

fn parse_chronology_baseline_statement(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if let Some(kind) = parse_keyword(trimmed) {
        return Some(kind);
    }
    let lower = trimmed.to_ascii_lowercase();
    let marker = " happens on ";
    if let Some(idx) = lower.find(marker) {
        let subject = trimmed[..idx].trim().trim_matches('"').to_string();
        let when = trimmed[idx + marker.len()..].trim().to_string();
        if subject.is_empty() || when.is_empty() {
            return None;
        }
        return Some(StatementKind::ChronologyHappensOn { subject, when });
    }
    // Accept ISO `YYYY-MM-DD : Label` shorthand
    if let Some((lhs, rhs)) = trimmed.split_once(':') {
        let when = lhs.trim();
        let subject = rhs.trim().trim_matches('"');
        if !when.is_empty()
            && !subject.is_empty()
            && when.chars().next().is_some_and(|c| c.is_ascii_digit())
        {
            return Some(StatementKind::ChronologyHappensOn {
                subject: subject.to_string(),
                when: when.to_string(),
            });
        }
    }
    None
}

/// Parse a state diagram statement from the current line.
/// Returns `Some((kind, end_index))` where `end_index` is the last consumed line.
fn parse_state_statement(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    // Handle common keywords that are valid in any diagram
    if let Some(kind) = parse_keyword(line) {
        return Ok(Some((kind, start)));
    }

    // `[H]` or `[H*]` — history pseudo-states
    if line == "[H]" {
        return Ok(Some((StatementKind::StateHistory { deep: false }, start)));
    }
    if line == "[H*]" {
        return Ok(Some((StatementKind::StateHistory { deep: true }, start)));
    }

    // `state Name` or `state Name <<stereotype>>` or `state Name { ... }`
    if line.starts_with("state ") {
        let rest = line.strip_prefix("state ").unwrap_or("").trim();
        if rest.is_empty() {
            return Ok(None);
        }

        // Extract optional stereotype `<<...>>`
        let (name_part, stereotype) = if let Some(idx) = rest.find("<<") {
            let name = rest[..idx].trim();
            let after = &rest[idx + 2..];
            let stereo = after.find(">>").map(|end| after[..end].trim().to_string());
            (name, stereo)
        } else {
            (rest, None)
        };

        // Check if there's a block
        let (name_alias_part, has_block) = if name_part.ends_with('{') {
            (name_part.trim_end_matches('{').trim(), true)
        } else {
            (name_part, false)
        };

        // Extract alias
        let (name_raw, alias) = if let Some((lhs, rhs)) = name_alias_part.split_once(" as ") {
            let name = clean_ident(lhs.trim());
            let alias = clean_ident(rhs.trim());
            (name, if alias.is_empty() { None } else { Some(alias) })
        } else {
            (clean_ident(name_alias_part), None)
        };

        if name_raw.is_empty() {
            return Ok(None);
        }

        if has_block {
            // Parse nested children until matching `}`
            let (children, region_dividers, end_idx) = parse_state_block(lines, start)?;
            let decl = StateDecl {
                name: name_raw,
                alias,
                stereotype,
                children,
                region_dividers,
            };
            return Ok(Some((StatementKind::StateDecl(decl), end_idx)));
        } else {
            let decl = StateDecl {
                name: name_raw,
                alias,
                stereotype,
                children: Vec::new(),
                region_dividers: Vec::new(),
            };
            return Ok(Some((StatementKind::StateDecl(decl), start)));
        }
    }

    // Transition: `From --> To` or `From --> To : label`
    // Also handles `[*] --> X` and `X --> [*]`
    if let Some(transition) = parse_state_transition(line) {
        return Ok(Some((StatementKind::StateTransition(transition), start)));
    }

    // Internal action: `State : entry / action` or `State : exit / action` or `State : event / action`
    if let Some(action) = parse_state_internal_action(line) {
        return Ok(Some((StatementKind::StateInternalAction(action), start)));
    }

    Ok(None)
}

/// Parse the body of a `state X { ... }` block.
/// Returns (children, region_divider_indices, end_line_index).
fn parse_state_block(
    lines: &[(&str, Span)],
    start: usize,
) -> Result<(Vec<Statement>, Vec<usize>, usize), Diagnostic> {
    let mut children: Vec<Statement> = Vec::new();
    let mut region_dividers: Vec<usize> = Vec::new();
    let mut depth = 1i32;
    let mut j = start + 1;

    while j < lines.len() {
        let (raw, span) = lines[j];
        let inner = raw.trim();

        if inner.ends_with('{') || inner == "{" {
            depth += 1;
        }
        if inner == "}" {
            depth -= 1;
            if depth == 0 {
                return Ok((children, region_dividers, j));
            }
        }

        // `||` region divider
        if inner == "||" && depth == 1 {
            region_dividers.push(children.len());
            j += 1;
            continue;
        }

        // Recurse for nested state declarations inside a block
        if depth == 1 {
            if inner.is_empty() || inner.starts_with('\'') {
                j += 1;
                continue;
            }
            if inner == "[H]" {
                children.push(Statement {
                    span,
                    kind: StatementKind::StateHistory { deep: false },
                });
                j += 1;
                continue;
            }
            if inner == "[H*]" {
                children.push(Statement {
                    span,
                    kind: StatementKind::StateHistory { deep: true },
                });
                j += 1;
                continue;
            }
            if let Some(transition) = parse_state_transition(inner) {
                children.push(Statement {
                    span,
                    kind: StatementKind::StateTransition(transition),
                });
                j += 1;
                continue;
            }
            if let Some(action) = parse_state_internal_action(inner) {
                children.push(Statement {
                    span,
                    kind: StatementKind::StateInternalAction(action),
                });
                j += 1;
                continue;
            }
            if inner.starts_with("state ") {
                if let Some((kind, end_idx)) = parse_state_statement(lines, j, inner)? {
                    let block_span = if end_idx > j {
                        Span::new(span.start, lines[end_idx].1.end)
                    } else {
                        span
                    };
                    children.push(Statement {
                        span: block_span,
                        kind,
                    });
                    j = end_idx + 1;
                    continue;
                }
            }
            if let Some(kind) = parse_keyword(inner) {
                children.push(Statement { span, kind });
                j += 1;
                continue;
            }
            // Unknown line inside block — store for normalizer
            children.push(Statement {
                span,
                kind: StatementKind::Unknown(inner.to_string()),
            });
        }
        j += 1;
    }

    // Unclosed block — treat as if closed at EOF
    Ok((children, region_dividers, lines.len().saturating_sub(1)))
}

/// Parse `From --> To` or `From --> To : label`
fn parse_state_transition(line: &str) -> Option<StateTransition> {
    let (core, label) = split_message_label(line);
    let (from_raw, arrow, to_raw) = split_family_arrow(core)?;

    if !arrow.contains('>') || from_raw.is_empty() || to_raw.is_empty() {
        return None;
    }

    Some(StateTransition {
        from: clean_bracketed_ident(from_raw),
        to: clean_bracketed_ident(to_raw),
        label,
    })
}

/// Parse `State : entry / action` or `State : exit / action` or `State : event / action`
fn parse_state_internal_action(line: &str) -> Option<StateInternalAction> {
    let (state_part, rest) = line.split_once(':')?;
    let state = state_part.trim();
    if state.is_empty() || state.contains("-->") {
        return None;
    }
    // Rest should have form `kind / action` or `kind`
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    let (kind, action) = if let Some((k, a)) = rest.split_once('/') {
        (k.trim().to_string(), a.trim().to_string())
    } else {
        (rest.to_string(), String::new())
    };
    if kind.is_empty() {
        return None;
    }
    Some(StateInternalAction {
        state: state.to_string(),
        kind,
        action,
    })
}

fn is_timeline_metadata_statement(kind: &StatementKind) -> bool {
    matches!(
        kind,
        StatementKind::Title(_)
            | StatementKind::Header(_)
            | StatementKind::Footer(_)
            | StatementKind::Caption(_)
            | StatementKind::Legend(_)
            | StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Pragma(_)
    )
}

fn parse_bracket_subject(line: &str) -> Option<(String, &str)> {
    let trimmed = line.trim();
    let stripped = trimmed.strip_prefix('[')?;
    let end = stripped.find(']')?;
    let name = stripped[..end].trim().to_string();
    if name.is_empty() {
        return None;
    }
    let rest = stripped[end + 1..].trim();
    Some((name, rest))
}
fn parse_multiline_keyword_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Option<(StatementKind, usize)> {
    let lower = line.to_ascii_lowercase();
    // Check for "legend" (alone or with positioning qualifiers: "legend left", etc.)
    let (key, legend_pos) = if lower == "legend" {
        ("legend", None)
    } else if lower.starts_with("legend ") {
        // Collect any position tokens after "legend"
        let pos_part = line[7..].trim();
        let pos_lower = pos_part.to_ascii_lowercase();
        // Verify all tokens are valid positioning keywords
        let all_pos = pos_lower
            .split_whitespace()
            .all(|t| matches!(t, "left" | "right" | "center" | "top" | "bottom"));
        if all_pos && !pos_part.is_empty() {
            ("legend", Some(pos_part.to_string()))
        } else {
            return None;
        }
    } else {
        let k = ["title", "header", "footer", "caption"]
            .into_iter()
            .find(|k| lower.as_str().eq(*k))?;
        (k, None)
    };

    let end_marker = format!("end {key}");
    let mut body = Vec::new();

    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case(&end_marker) {
            let text = body.join("\n");
            let kind = match key {
                "title" => StatementKind::Title(text),
                "header" => StatementKind::Header(text),
                "footer" => StatementKind::Footer(text),
                "caption" => StatementKind::Caption(text),
                "legend" => {
                    // Emit Legend first; if there's position info emit LegendPos separately.
                    // We return the Legend text here; the LegendPos is handled by returning
                    // the Legend kind with position info embedded for the caller.
                    // Since we can only return one StatementKind, we pack the pos into the
                    // legend_pos field and handle it via a special kind.
                    let _ = legend_pos; // used below
                    StatementKind::Legend(text)
                }
                _ => StatementKind::Legend(text),
            };
            // If there was a position qualifier alongside the legend text, we need to
            // emit both. We return the Legend kind (which the caller will handle) and
            // separately emit a LegendPos. But since we can only return one statement,
            // we encode the position in a specially-prefixed Legend value when present.
            // Convention: if legend_pos is Some, we prefix the text with "LEGEND_POS:<pos>\n".
            // The normalizer detects and splits this prefix.
            if key == "legend" {
                if let Some(ref pos) = legend_pos {
                    let packed = format!("LEGEND_POS:{}\n{}", pos, body.join("\n"));
                    return Some((StatementKind::Legend(packed), idx));
                }
            }
            return Some((kind, idx));
        }
        body.push(trimmed.to_string());
    }

    None
}

fn parse_multiline_note_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Option<(StatementKind, usize)> {
    let lower = line.to_ascii_lowercase();
    let note_kw = if lower.starts_with("note ") {
        "note"
    } else if lower.starts_with("hnote ") {
        "hnote"
    } else if lower.starts_with("rnote ") {
        "rnote"
    } else {
        return None;
    };

    let tail = line[note_kw.len()..].trim();
    if tail.is_empty() {
        return None;
    }
    let (head, inline) = tail.split_once(':').unwrap_or((tail, ""));
    let (position, target) = parse_note_head(head.trim());
    if matches!(position.to_ascii_lowercase().as_str(), "left" | "right") && target.is_none() {
        return None;
    }
    let mut body = Vec::new();
    if !inline.trim().is_empty() {
        body.push(inline.trim().to_string());
    }

    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("end note") {
            return Some((
                StatementKind::Note(Note {
                    kind: note_kind_from_keyword(note_kw),
                    position,
                    target,
                    text: body.join("\n"),
                }),
                idx,
            ));
        }
        if note_end_matches(trimmed, note_kw) {
            return Some((
                StatementKind::Note(Note {
                    kind: note_kind_from_keyword(note_kw),
                    position,
                    target,
                    text: body.join("\n"),
                }),
                idx,
            ));
        }
        body.push(trimmed.to_string());
    }

    None
}

fn parse_multiline_ref_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Option<(StatementKind, usize)> {
    if !line.to_ascii_lowercase().starts_with("ref ") {
        return None;
    }
    let tail = line[4..].trim();
    let (head, inline) = tail.split_once(':').unwrap_or((tail, ""));
    let head = head.trim();
    if head.is_empty() {
        return None;
    }

    let mut body = Vec::new();
    let mut has_non_empty_body = false;
    if !inline.trim().is_empty() {
        body.push(inline.trim().to_string());
        has_non_empty_body = true;
    }
    for (idx, (raw, _)) in lines.iter().enumerate().skip(start + 1) {
        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case("end ref") {
            if !has_non_empty_body {
                return None;
            }
            let mut label = head.to_string();
            label.push('\n');
            label.push_str(&body.join("\n"));
            return Some((
                StatementKind::Group(Group {
                    kind: "ref".to_string(),
                    label: Some(label),
                }),
                idx,
            ));
        }
        if !trimmed.is_empty() {
            has_non_empty_body = true;
        }
        body.push(trimmed.to_string());
    }
    None
}

fn parse_participant(line: &str) -> Option<StatementKind> {
    let roles = [
        ("participant", ParticipantRole::Participant),
        ("actor", ParticipantRole::Actor),
        ("boundary", ParticipantRole::Boundary),
        ("control", ParticipantRole::Control),
        ("entity", ParticipantRole::Entity),
        ("database", ParticipantRole::Database),
        ("collections", ParticipantRole::Collections),
        ("queue", ParticipantRole::Queue),
    ];

    for (kw, role) in roles {
        if !line.starts_with(kw) {
            continue;
        }
        let rest = line[kw.len()..].trim();
        if rest.is_empty() {
            return None;
        }
        let (display, rem) = if let Some(stripped) = rest.strip_prefix('"') {
            let end = stripped.find('"')?;
            (
                Some(stripped[..end].to_string()),
                stripped[end + 1..].trim(),
            )
        } else {
            (None, rest)
        };

        let mut alias = None;
        let mut name = rem.to_string();
        if let Some((lhs, rhs)) = rem.split_once(" as ") {
            let lhs = lhs.trim();
            let rhs = rhs.trim();
            if display.is_none() {
                name = lhs.to_string();
            }
            alias = Some(clean_ident(rhs));
        }

        if name.is_empty() {
            name = alias.clone().unwrap_or_default();
        }
        let name = clean_ident(&name);
        let display = display.or_else(|| Some(name.clone()));

        return Some(StatementKind::Participant(ParticipantDecl {
            role,
            name,
            alias,
            display,
        }));
    }
    None
}

fn parse_message(line: &str) -> Option<StatementKind> {
    let (core, label) = split_message_label(line);
    let (lhs_raw, arrow, rhs_raw) = split_arrow(core)?;
    let style = parse_arrow_style(arrow);
    let parsed_arrow = parse_arrow(arrow)?;
    let (from_id_raw, from_modifier) = split_lifecycle_modifier(lhs_raw);
    let (to_id_raw, to_modifier) = split_lifecycle_modifier(rhs_raw);

    let from = if let Some(v) = normalize_virtual_endpoint(from_id_raw) {
        v
    } else {
        if looks_like_virtual_endpoint_syntax(from_id_raw) {
            return None;
        }
        clean_ident(from_id_raw)
    };
    let to = if let Some(v) = normalize_virtual_endpoint(to_id_raw) {
        v
    } else {
        if looks_like_virtual_endpoint_syntax(to_id_raw) {
            return None;
        }
        clean_ident(to_id_raw)
    };

    if from.is_empty() || to.is_empty() {
        return None;
    }

    let mut arrow_encoded = parsed_arrow.to_string();
    if let Some(modifier) = from_modifier {
        arrow_encoded.push_str("@L");
        arrow_encoded.push_str(modifier);
    }
    if let Some(modifier) = to_modifier {
        arrow_encoded.push_str("@R");
        arrow_encoded.push_str(modifier);
    }

    let from_virtual = ast_virtual_endpoint_from_id(&from, true);
    let to_virtual = ast_virtual_endpoint_from_id(&to, false);
    Some(StatementKind::Message(Message {
        from,
        to,
        arrow: arrow_encoded,
        label,
        style,
        from_virtual,
        to_virtual,
    }))
}

fn parse_arrow_style(arrow: &str) -> MessageStyle {
    let mut style = MessageStyle::default();
    let mut chars = arrow.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '[' {
            continue;
        }
        let mut body = String::new();
        for inner in chars.by_ref() {
            if inner == ']' {
                break;
            }
            body.push(inner);
        }
        for token in body.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            let lower = token.to_ascii_lowercase();
            match lower.as_str() {
                "hidden" => style.hidden = true,
                "dashed" => style.dashed = true,
                "dotted" => style.dotted = true,
                _ if token.starts_with('#')
                    && matches!(token.len(), 4 | 5 | 7 | 9)
                    && token[1..].bytes().all(|b| b.is_ascii_hexdigit()) =>
                {
                    style.color = Some(format!("#{}", token[1..].to_ascii_lowercase()));
                }
                _ if token.starts_with('#')
                    && token[1..].bytes().all(|b| b.is_ascii_alphabetic()) =>
                {
                    style.color = Some(token[1..].to_ascii_lowercase());
                }
                _ if token.bytes().all(|b| b.is_ascii_alphabetic()) => {
                    style.color = Some(lower);
                }
                _ => {}
            }
        }
    }
    style
}

fn ast_virtual_endpoint_from_id(id: &str, is_from: bool) -> Option<VirtualEndpoint> {
    let (side, kind) = match id {
        "[" => (VirtualEndpointSide::Left, VirtualEndpointKind::Plain),
        "]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Plain),
        "[o" => (VirtualEndpointSide::Left, VirtualEndpointKind::Circle),
        "o]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Circle),
        "[x" => (VirtualEndpointSide::Left, VirtualEndpointKind::Cross),
        "x]" => (VirtualEndpointSide::Right, VirtualEndpointKind::Cross),
        "[*]" => (
            if is_from {
                VirtualEndpointSide::Left
            } else {
                VirtualEndpointSide::Right
            },
            VirtualEndpointKind::Filled,
        ),
        _ => return None,
    };
    Some(VirtualEndpoint { side, kind })
}

fn parse_keyword(line: &str) -> Option<StatementKind> {
    let lower = line.to_ascii_lowercase();

    for k in ["title", "header", "footer", "caption", "legend"] {
        if lower.starts_with(&(k.to_string() + " ")) {
            let v = line[k.len()..].trim().to_string();
            return Some(match k {
                "title" => StatementKind::Title(v),
                "header" => StatementKind::Header(v),
                "footer" => StatementKind::Footer(v),
                "caption" => StatementKind::Caption(v),
                _ => StatementKind::Legend(v),
            });
        }
    }

    if lower.starts_with("skinparam ") {
        let body = line[9..].trim();
        let (key, value) = body.split_once(' ').unwrap_or((body, ""));
        return Some(StatementKind::SkinParam {
            key: key.trim().to_string(),
            value: value.trim().to_string(),
        });
    }
    if lower.starts_with("!theme") {
        return Some(StatementKind::Theme(line[6..].trim().to_string()));
    }
    if lower.starts_with("!pragma") {
        let body = line[7..].trim();
        if body.is_empty() {
            return Some(StatementKind::Unknown(
                "[E_PRAGMA_INVALID] malformed pragma syntax: missing pragma body".to_string(),
            ));
        }
        return Some(StatementKind::Pragma(body.to_string()));
    }

    if lower == "hide footbox" {
        return Some(StatementKind::Footbox(false));
    }
    if lower == "show footbox" {
        return Some(StatementKind::Footbox(true));
    }
    if lower == "hide unlinked" {
        return Some(StatementKind::HideUnlinked);
    }

    // scale directive: "scale <factor>", "scale <w>*<h>", "scale max <n>"
    if lower.starts_with("scale ") {
        let body = line[6..].trim();
        return Some(StatementKind::Scale(body.to_string()));
    }

    // Class-diagram hide options (parsed here so they work before any class decl sets detected_kind)
    if lower.starts_with("hide ") {
        let rest = lower.strip_prefix("hide ").unwrap_or("").trim();
        let class_hide_opts = [
            "circle",
            "stereotype",
            "empty members",
            "empty methods",
            "empty fields",
        ];
        for opt in class_hide_opts {
            if rest == opt {
                return Some(StatementKind::HideOption(rest.to_string()));
            }
        }
    }

    // set namespaceSeparator <sep>
    if lower.starts_with("set namespaceseparator") {
        let rest = line["set namespaceSeparator".len()..].trim();
        return Some(StatementKind::SetOption {
            key: "namespaceSeparator".to_string(),
            value: rest.to_string(),
        });
    }

    let note_kw = if lower.starts_with("note ") {
        Some("note")
    } else if lower.starts_with("hnote ") {
        Some("hnote")
    } else if lower.starts_with("rnote ") {
        Some("rnote")
    } else {
        None
    };
    if let Some(note_kw) = note_kw {
        let tail = line[note_kw.len()..].trim();
        if tail.is_empty() {
            return Some(StatementKind::Unknown(
                "[E_NOTE_INVALID] malformed note syntax: missing note head".to_string(),
            ));
        }
        let (head, text) = tail.split_once(':').unwrap_or((tail, ""));
        let (pos, target) = parse_note_head(head);
        if pos.eq_ignore_ascii_case("of") || !is_valid_note_position(&pos) {
            return Some(StatementKind::Unknown(format!(
                "[E_NOTE_INVALID] malformed note syntax: `{}`",
                line
            )));
        }
        return Some(StatementKind::Note(Note {
            kind: note_kind_from_keyword(note_kw),
            position: pos,
            target,
            text: text.trim().to_string(),
        }));
    }
    if lower.starts_with("ref ") {
        let tail = line[4..].trim();
        let (head, text) = tail.split_once(':').unwrap_or((tail, ""));
        if head.is_empty() || text.trim().is_empty() {
            return Some(StatementKind::Unknown(format!(
                "[E_REF_INVALID] malformed ref syntax: `{}`",
                line
            )));
        }
        let label = format!("{}\n{}", head.trim(), text.trim());
        return Some(StatementKind::Group(Group {
            kind: "ref".to_string(),
            label: Some(label),
        }));
    }

    for g in [
        "alt", "opt", "loop", "par", "critical", "break", "group", "box",
    ] {
        if lower == g || lower.starts_with(&(g.to_string() + " ")) {
            let label = line[g.len()..].trim();
            return Some(StatementKind::Group(Group {
                kind: g.to_string(),
                label: if label.is_empty() {
                    None
                } else {
                    Some(label.to_string())
                },
            }));
        }
    }

    if lower == "else" || lower.starts_with("else ") {
        return Some(StatementKind::Group(Group {
            kind: "else".to_string(),
            label: Some(line[4..].trim().to_string()).filter(|s| !s.is_empty()),
        }));
    }

    if lower == "end" {
        return Some(StatementKind::Group(Group {
            kind: "end".to_string(),
            label: None,
        }));
    }
    if let Some(stripped) = lower.strip_prefix("end ") {
        let tail = stripped.trim();
        if matches!(
            tail,
            "alt" | "opt" | "loop" | "par" | "critical" | "break" | "group" | "ref" | "box"
        ) {
            return Some(StatementKind::Group(Group {
                kind: "end".to_string(),
                label: Some(tail.to_string()),
            }));
        }
    }

    if line == "..." {
        return Some(StatementKind::Spacer);
    }
    if lower.starts_with("...") && line.ends_with("...") && line.len() >= 6 {
        return Some(StatementKind::Divider(Some(
            line.trim_matches('.').trim().to_string(),
        )));
    }
    if lower.starts_with("||") && line.ends_with("||") && line.len() >= 4 {
        return Some(StatementKind::Delay(Some(
            line.trim_matches('|').trim().to_string(),
        )));
    }
    if lower == "||" {
        return Some(StatementKind::Delay(None));
    }
    if line.starts_with("==") && line.ends_with("==") && line.len() >= 4 {
        let label = line[2..line.len() - 2].trim().to_string();
        return Some(if label.is_empty() {
            StatementKind::Separator(None)
        } else {
            StatementKind::Separator(Some(label))
        });
    }
    if lower.starts_with("newpage") {
        return Some(StatementKind::NewPage(line[7..].trim().to_string().into()));
    }
    if lower == "ignore newpage" {
        return Some(StatementKind::IgnoreNewPage);
    }
    if lower.starts_with("autonumber") {
        return Some(StatementKind::Autonumber(
            line[10..].trim().to_string().into(),
        ));
    }

    for (kw, ctor) in [
        (
            "activate",
            StatementKind::Activate as fn(String) -> StatementKind,
        ),
        ("deactivate", StatementKind::Deactivate),
        ("destroy", StatementKind::Destroy),
        ("create", StatementKind::Create),
    ] {
        if lower.starts_with(&(kw.to_string() + " ")) {
            return Some(ctor(clean_ident(line[kw.len()..].trim())));
        }
    }

    if lower == "return" || lower.starts_with("return ") {
        return Some(StatementKind::Return(
            Some(line[6..].trim().to_string()).filter(|s| !s.is_empty()),
        ));
    }

    if lower.starts_with("!include") {
        return Some(StatementKind::Include(line[8..].trim().to_string()));
    }
    if lower.starts_with("!define") {
        let body = line[7..].trim();
        let (name, value) = body.split_once(' ').unwrap_or((body, ""));
        return Some(StatementKind::Define {
            name: name.trim().to_string(),
            value: Some(value.trim().to_string()).filter(|s| !s.is_empty()),
        });
    }
    if lower.starts_with("!undef") {
        return Some(StatementKind::Undef(line[6..].trim().to_string()));
    }

    None
}

fn parse_note_head(head: &str) -> (String, Option<String>) {
    let mut bits = head.split_whitespace();
    let position = bits.next().unwrap_or("over").to_string();
    let rest = bits.collect::<Vec<_>>();
    if rest.is_empty() {
        return (position, None);
    }
    if rest[0].eq_ignore_ascii_case("of") {
        let target = rest[1..].join(" ");
        return (
            position,
            (!target.trim().is_empty()).then(|| clean_ident(target.trim())),
        );
    }
    let target = rest.join(" ");
    (
        position,
        (!target.trim().is_empty()).then(|| clean_ident(target.trim())),
    )
}

fn note_kind_from_keyword(keyword: &str) -> crate::ast::NoteKind {
    match keyword.to_ascii_lowercase().as_str() {
        "hnote" => crate::ast::NoteKind::Hexagonal,
        "rnote" => crate::ast::NoteKind::Rectangle,
        _ => crate::ast::NoteKind::Folded,
    }
}

fn note_end_matches(line: &str, note_keyword: &str) -> bool {
    line.eq_ignore_ascii_case("end note")
        || (note_keyword.eq_ignore_ascii_case("hnote") && line.eq_ignore_ascii_case("endhnote"))
        || (note_keyword.eq_ignore_ascii_case("rnote") && line.eq_ignore_ascii_case("endrnote"))
}

fn is_valid_note_position(position: &str) -> bool {
    matches!(
        position.to_ascii_lowercase().as_str(),
        "left" | "right" | "top" | "bottom" | "over" | "across"
    )
}

fn clean_ident(s: &str) -> String {
    let mut out = s.trim().trim_matches('"').to_string();
    for suffix in ["++", "--", "**", "!!"] {
        out = out
            .strip_suffix(suffix)
            .map(str::trim_end)
            .unwrap_or(&out)
            .to_string();
    }
    out
}

/// Extract the class/interface/enum name from a member line inside a package/namespace block.
/// E.g. "class Service" → "Service", "interface IRepo" → "IRepo", "MyClass" → "MyClass".
fn extract_class_member_name(s: &str) -> String {
    let t = s.trim();
    let lower = t.to_ascii_lowercase();
    for kw in &[
        "abstract class ",
        "annotation ",
        "interface ",
        "abstract ",
        "enum ",
        "class ",
    ] {
        if lower.starts_with(kw) {
            // Extract the first identifier token from the original (case-preserved) text
            let name_part = t[kw.len()..].trim();
            let name = name_part
                .split(|c: char| c.is_whitespace() || c == '{')
                .next()
                .unwrap_or("")
                .trim_matches('"');
            return clean_ident(name);
        }
    }
    // Plain identifier (like in a together block)
    clean_ident(t)
}

fn extract_component_group_member_name(s: &str) -> String {
    if let Some(StatementKind::ComponentDecl { name, alias, .. }) = parse_component_decl(s) {
        return alias.unwrap_or(name);
    }
    extract_class_member_name(s)
}

fn split_family_relation_label(line: &str) -> (&str, Option<String>) {
    if split_family_arrow(line).is_none() {
        return split_message_label(line);
    }
    if let Some(colon) = line.rfind(" :") {
        let suffix = line[colon + 2..].trim();
        if !suffix_has_family_relation_arrow(suffix) {
            let text = line[colon + 2..].trim();
            if !text.is_empty() {
                return (line[..colon].trim_end(), Some(text.to_string()));
            }
        }
    }
    (line.trim_end(), None)
}

fn suffix_has_family_relation_arrow(suffix: &str) -> bool {
    suffix.contains("--")
        || suffix.contains("..")
        || suffix.contains("->")
        || suffix.contains("<-")
        || suffix.contains("|>")
        || suffix.contains("<|")
}

fn split_message_label(line: &str) -> (&str, Option<String>) {
    if let Some(colon) = line.find(':') {
        let text = line[colon + 1..].trim();
        (
            line[..colon].trim_end(),
            Some(text.to_string()).filter(|s| !s.is_empty()),
        )
    } else {
        (line.trim_end(), None)
    }
}

fn split_arrow(core: &str) -> Option<(&str, &str, &str)> {
    fn is_arrow_char(c: char) -> bool {
        matches!(c, '-' | '<' | '>' | '[' | ']' | 'o' | 'x' | '/' | '\\')
    }

    let mut run_start: Option<usize> = None;
    let mut in_bracket = false;
    let mut skip_until = 0usize;
    for (idx, ch) in core.char_indices() {
        if idx < skip_until {
            continue;
        }
        if let Some(start) = run_start {
            if in_bracket {
                if ch == ']' {
                    in_bracket = false;
                }
                continue;
            }
            if ch == '[' {
                in_bracket = true;
                continue;
            }
            if is_arrow_char(ch) {
                continue;
            }
            let candidate = &core[start..idx];
            if !candidate.contains('-') {
                run_start = None;
                continue;
            }
            let lhs = core[..start].trim();
            let rhs = core[idx..].trim();
            if !lhs.is_empty() && !rhs.is_empty() {
                return Some((lhs, candidate.trim(), rhs));
            }
            run_start = None;
            continue;
        }
        if ch == '[' && core[..idx].trim().is_empty() {
            let mut skipped_open_endpoint = false;
            for endpoint in ["[o", "[x"] {
                if core[idx..].starts_with(endpoint)
                    && core[idx + endpoint.len()..]
                        .chars()
                        .next()
                        .is_some_and(char::is_whitespace)
                {
                    skip_until = idx + endpoint.len();
                    skipped_open_endpoint = true;
                    break;
                }
            }
            if skipped_open_endpoint {
                continue;
            }
            if let Some(close_rel) = core[idx..].find(']') {
                let bracket_body = &core[idx + ch.len_utf8()..idx + close_rel];
                if bracket_body.contains('-') {
                    continue;
                }
                let after_idx = idx + close_rel + 1;
                if core[after_idx..]
                    .chars()
                    .next()
                    .is_some_and(char::is_whitespace)
                {
                    skip_until = after_idx;
                    continue;
                }
            } else if core[idx + ch.len_utf8()..]
                .chars()
                .next()
                .is_some_and(char::is_whitespace)
            {
                continue;
            }
        }
        if is_arrow_char(ch) {
            if run_start.is_none() {
                run_start = Some(idx);
            }
            if ch == '[' {
                in_bracket = true;
            }
            continue;
        }
    }
    if let Some(start) = run_start {
        let candidate = &core[start..];
        if !candidate.contains('-') {
            return None;
        }
        let lhs = core[..start].trim();
        if lhs.is_empty() {
            return None;
        }
        return Some((lhs, candidate.trim(), ""));
    }
    None
}

fn parse_arrow(arrow: &str) -> Option<String> {
    const VALID_BASE_ARROWS: &[&str] = &[
        "->", "-->", "->>", "-->>", "<-", "<--", "<<-", "<<--", "<->", "<-->", "<<->>", "<<-->>",
    ];
    let arrow = strip_sequence_arrow_brackets(arrow);
    let mut squashed = String::with_capacity(arrow.len());
    let mut last_slash: Option<char> = None;
    let mut slash_run_len = 0usize;
    for ch in arrow.chars() {
        if matches!(ch, '/' | '\\') {
            if last_slash == Some(ch) {
                slash_run_len += 1;
            } else {
                last_slash = Some(ch);
                slash_run_len = 1;
            }
            if ch == '/' && slash_run_len > 1 {
                // Portable slash forms allow a single slash marker only.
                return None;
            }
            if slash_run_len == 1 {
                squashed.push(ch);
            }
            continue;
        }
        last_slash = None;
        slash_run_len = 0;
        squashed.push(ch);
    }

    let canonical = squashed.replace(['/', '\\'], "");
    if canonical.is_empty()
        || !canonical
            .chars()
            .all(|c| matches!(c, '-' | '<' | '>' | 'o' | 'x'))
        || !squashed
            .chars()
            .all(|c| matches!(c, '-' | '<' | '>' | 'o' | 'x' | '/' | '\\'))
    {
        return None;
    }
    let has_slash_marker = squashed.contains('/') || squashed.contains('\\');
    let expanded_marker = squashed.contains("-/") || squashed.contains("-\\");

    if VALID_BASE_ARROWS.contains(&canonical.as_str()) {
        if has_slash_marker && !expanded_marker {
            return Some(canonical);
        }
        if expanded_marker
            && squashed.contains("-\\")
            && canonical == "-->>"
            && squashed.contains("->>")
        {
            return Some(squashed.replacen("->>", "-->>", 1));
        }
        return Some(squashed);
    }
    let with_left_trimmed = canonical
        .strip_prefix('o')
        .or_else(|| canonical.strip_prefix('x'))
        .unwrap_or(&canonical);
    let (core, right_marker_removed) = if let Some(stripped) = with_left_trimmed.strip_suffix('o') {
        (stripped, true)
    } else if let Some(stripped) = with_left_trimmed.strip_suffix('x') {
        (stripped, true)
    } else {
        (with_left_trimmed, false)
    };
    if core.is_empty() {
        return None;
    }
    if VALID_BASE_ARROWS.contains(&core) && (right_marker_removed || core != canonical) {
        if has_slash_marker && !expanded_marker {
            let mut out = core.to_string();
            if let Some(ch) = with_left_trimmed.chars().last() {
                if matches!(ch, 'o' | 'x') && right_marker_removed {
                    out.push(ch);
                }
            }
            return Some(out);
        }
        if expanded_marker && canonical.contains("-->>") && squashed.contains("->>") {
            return Some(squashed.replacen("->>", "-->>", 1));
        }
        return Some(squashed);
    }
    if let Some(stripped_core) = core.strip_prefix('-') {
        if VALID_BASE_ARROWS.contains(&stripped_core) && (right_marker_removed || core != canonical)
        {
            if has_slash_marker && !expanded_marker {
                let mut out = stripped_core.to_string();
                if let Some(ch) = with_left_trimmed.chars().last() {
                    if matches!(ch, 'o' | 'x') && right_marker_removed {
                        out.push(ch);
                    }
                }
                return Some(out);
            }
            if expanded_marker && canonical.contains("-->>") && squashed.contains("->>") {
                return Some(squashed.replacen("->>", "-->>", 1));
            }
            return Some(squashed);
        }
    }
    None
}

fn strip_sequence_arrow_brackets(arrow: &str) -> String {
    let mut out = String::with_capacity(arrow.len());
    let mut chars = arrow.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '[' {
            for next in chars.by_ref() {
                if next == ']' {
                    break;
                }
            }
            continue;
        }
        out.push(ch);
    }
    out
}

fn split_lifecycle_modifier(endpoint: &str) -> (&str, Option<&'static str>) {
    for suffix in ["++", "--", "**", "!!"] {
        if let Some(base) = endpoint.trim_end().strip_suffix(suffix) {
            return (base.trim_end(), Some(suffix));
        }
    }
    (endpoint, None)
}

fn normalize_virtual_endpoint(raw: &str) -> Option<String> {
    let t = raw.trim().trim_matches('"');
    let lower = t.to_ascii_lowercase();
    match lower.as_str() {
        "[*]" => Some("[*]".to_string()),
        "[" => Some("[".to_string()),
        "]" => Some("]".to_string()),
        "[o" | "o[" => Some("[o".to_string()),
        "o]" | "]o" => Some("o]".to_string()),
        "[x" | "x[" => Some("[x".to_string()),
        "x]" | "]x" => Some("x]".to_string()),
        _ => None,
    }
}

fn looks_like_virtual_endpoint_syntax(raw: &str) -> bool {
    let t = raw.trim().trim_matches('"').to_ascii_lowercase();
    t.contains('[') || t.contains(']')
}

fn looks_like_arrow_syntax(line: &str) -> bool {
    if line.starts_with('!') || line.starts_with('@') {
        return false;
    }
    line.contains("->")
        || line.contains("-->")
        || line.contains("<-")
        || line.contains("<--")
        || line.contains("<->")
        || line.contains("<-->")
        || line.contains("->>")
        || line.contains("-->>")
        || line.contains("-x")
        || line.contains("x-")
        || line.contains("-o")
        || line.contains("o-")
}

fn is_sequence_keyword(kind: &StatementKind) -> bool {
    matches!(
        kind,
        StatementKind::Group(_)
            | StatementKind::Footbox(_)
            | StatementKind::Delay(_)
            | StatementKind::Divider(_)
            | StatementKind::Separator(_)
            | StatementKind::Spacer
            | StatementKind::NewPage(_)
            | StatementKind::IgnoreNewPage
            | StatementKind::Autonumber(_)
            | StatementKind::Activate(_)
            | StatementKind::Deactivate(_)
            | StatementKind::Destroy(_)
            | StatementKind::Create(_)
            | StatementKind::Return(_)
    )
}

fn note_block_continues(lines: &[(&str, Span)], idx: usize, line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    if !(lower.starts_with("note ") || lower.starts_with("hnote ") || lower.starts_with("rnote ")) {
        return false;
    }
    for (candidate, _) in lines.iter().skip(idx + 1) {
        let trimmed = candidate.trim();
        if trimmed.eq_ignore_ascii_case("end note")
            || trimmed.eq_ignore_ascii_case("endnote")
            || trimmed.eq_ignore_ascii_case("endhnote")
            || trimmed.eq_ignore_ascii_case("endrnote")
        {
            return true;
        }
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('@') {
            return false;
        }
    }
    !line.contains(':')
}

fn text_block_continues(lines: &[(&str, Span)], idx: usize, line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    let keyword = ["title", "header", "footer", "caption", "legend"]
        .into_iter()
        .find(|keyword| lower.starts_with(&format!("{keyword} ")));
    let Some(keyword) = keyword else {
        return false;
    };
    let end_marker = format!("end {keyword}");
    for (candidate, _) in lines.iter().skip(idx + 1) {
        let trimmed = candidate.trim();
        if trimmed.eq_ignore_ascii_case(&end_marker) {
            return true;
        }
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('@') {
            return false;
        }
    }
    false
}

fn is_family_common_keyword(kind: &StatementKind) -> bool {
    matches!(
        kind,
        StatementKind::Note(_)
            | StatementKind::Title(_)
            | StatementKind::Caption(_)
            | StatementKind::Header(_)
            | StatementKind::Footer(_)
            | StatementKind::Legend(_)
            | StatementKind::LegendPos(_)
            | StatementKind::SkinParam { .. }
            | StatementKind::Theme(_)
            | StatementKind::Scale(_)
            | StatementKind::SetOption { .. }
            | StatementKind::HideOption(_)
            | StatementKind::Pragma(_)
    )
}

/// Parse an inline `json $alias { ... }` or `yaml $alias { ... }` block.
/// Returns the projection statement and closing line index if found, else `None`.
/// Errors if a projection block is found but no matching closing `}` appears.
fn parse_json_projection_block(
    lines: &[(&str, Span)],
    start: usize,
    line: &str,
) -> Result<Option<(StatementKind, usize)>, Diagnostic> {
    // Match: `json|yaml` <whitespace> <identifier starting with optional $> `{`
    let lower = line.to_ascii_lowercase();
    let (keyword, is_yaml) = if lower.starts_with("json ") {
        ("json", false)
    } else if lower.starts_with("yaml ") {
        ("yaml", true)
    } else {
        return Ok(None);
    };
    let rest = line[keyword.len() + 1..].trim();
    if rest.is_empty() {
        return Ok(None);
    }

    // Parse alias (identifier, optionally starting with `$`)
    let (alias, after_alias) = {
        let mut end = 0;
        let chars: Vec<char> = rest.chars().collect();
        if chars.is_empty() {
            return Ok(None);
        }
        // Allow `$identifier` or plain `identifier`
        if chars[0] == '$' {
            end += 1;
        }
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }
        if end == 0 || (end == 1 && rest.starts_with('$')) {
            return Ok(None);
        }
        let alias = rest[..end].to_string();
        let after = rest[end..].trim();
        (alias, after)
    };

    // Must be followed by `{`
    if !after_alias.starts_with('{') {
        return Ok(None);
    }

    // Accumulate body lines until the matching `}` (depth-tracked).
    let mut body_lines: Vec<&str> = Vec::new();
    // The opening `{` may have content after it on the same line.
    let inline_after_brace = after_alias[1..].trim();
    let mut depth: i32 = 1;

    // If everything is on one line: `json $alias { ... }`
    if !inline_after_brace.is_empty() {
        let mut in_quotes = false;
        let mut prev_escape = false;
        for (j, ch) in inline_after_brace.char_indices() {
            if in_quotes {
                if ch == '"' && !prev_escape {
                    in_quotes = false;
                }
                prev_escape = ch == '\\' && !prev_escape;
                continue;
            }
            match ch {
                '"' => in_quotes = true,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let body = inline_after_brace[..j].trim().to_string();
                        let kind = if is_yaml {
                            StatementKind::YamlProjection { alias, body }
                        } else {
                            StatementKind::JsonProjection { alias, body }
                        };
                        return Ok(Some((kind, start)));
                    }
                }
                _ => {}
            }
            prev_escape = false;
        }
        // Depth > 0: content continues on next lines.
        body_lines.push(inline_after_brace);
    }

    // Continue scanning subsequent lines.
    let mut i = start + 1;
    while i < lines.len() {
        let (raw, _span) = lines[i];
        let trimmed = raw.trim();
        // Check for matching closing brace.
        let mut consumed_close = false;
        let mut close_pos = 0;
        let mut in_quotes = false;
        let mut prev_escape = false;
        for (pos, ch) in trimmed.char_indices() {
            if in_quotes {
                if ch == '"' && !prev_escape {
                    in_quotes = false;
                }
                prev_escape = ch == '\\' && !prev_escape;
                continue;
            }
            match ch {
                '"' => in_quotes = true,
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        consumed_close = true;
                        close_pos = pos;
                        break;
                    }
                }
                _ => {}
            }
            prev_escape = false;
        }
        if consumed_close {
            // Everything before the closing `}` is part of the body.
            let last_body = trimmed[..close_pos].trim();
            if !last_body.is_empty() {
                body_lines.push(last_body);
            }
            let body = body_lines.join("\n");
            let kind = if is_yaml {
                StatementKind::YamlProjection { alias, body }
            } else {
                StatementKind::JsonProjection { alias, body }
            };
            return Ok(Some((kind, i)));
        }
        body_lines.push(trimmed);
        i += 1;
    }

    // No closing brace found.
    Err(Diagnostic::error(format!(
        "[E_PROJECTION_UNCLOSED] `{keyword} {alias}` block has no matching closing `}}`"
    ))
    .with_span(lines[start].1))
}

/// Parse a single salt wireframe row line into a `SaltGridRow` statement.
/// A row is a `|`-delimited sequence of cell tokens.
/// Returns `None` if the line does not start with `|`.
fn parse_salt_grid_row(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let whole_line_widget = lower.starts_with("{*")
        || lower.starts_with("{/")
        || lower.starts_with("{s")
        || lower.starts_with("{t")
        || lower == "tree"
        || lower.starts_with("tree ")
        || lower == "menu"
        || lower.starts_with("menu ")
        || lower == "tab"
        || lower.starts_with("tab ")
        || lower == "tabs"
        || lower.starts_with("tabs ")
        || lower.starts_with("scroll")
        || lower.contains("scrollbar");
    if whole_line_widget {
        return Some(StatementKind::SaltGridRow {
            cells: vec![SaltCell::Label(trimmed.to_string())],
        });
    }
    if !trimmed.contains('|') {
        return None;
    }
    // Split on `|` and parse each cell token.
    let parts: Vec<&str> = trimmed.split('|').collect();
    let mut cells = Vec::new();
    for part in parts {
        let cell_text = part.trim();
        if cell_text.is_empty() {
            continue;
        }
        cells.push(parse_salt_cell(cell_text));
    }
    if cells.is_empty() {
        return None;
    }
    Some(StatementKind::SaltGridRow { cells })
}

/// Parse a single salt cell token into a `SaltCell` variant.
fn parse_salt_cell(text: &str) -> SaltCell {
    // `"placeholder"` → Input
    if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
        let inner = &text[1..text.len() - 1];
        return SaltCell::Input(inner.to_string());
    }
    // `[X] label` or `[ ] label` → Checkbox
    if text.starts_with("[X]") || text.starts_with("[x]") {
        let label = text[3..].trim().to_string();
        return SaltCell::CheckboxChecked(label);
    }
    if let Some(rest) = text.strip_prefix("[ ]") {
        return SaltCell::CheckboxUnchecked(rest.trim().to_string());
    }
    // `(X) label` or `( ) label` → Radio
    if text.starts_with("(X)") || text.starts_with("(x)") {
        let label = text[3..].trim().to_string();
        return SaltCell::RadioOn(label);
    }
    if let Some(rest) = text.strip_prefix("( )") {
        return SaltCell::RadioOff(rest.trim().to_string());
    }
    // `[button text]` → Button
    if text.starts_with('[') && text.ends_with(']') && text.len() >= 2 {
        let inner = &text[1..text.len() - 1];
        return SaltCell::Button(inner.to_string());
    }
    // `^combo text^` → Combo
    if text.starts_with('^') && text.ends_with('^') && text.len() >= 2 {
        let inner = &text[1..text.len() - 1];
        return SaltCell::Combo(inner.to_string());
    }
    // Plain text → Label
    SaltCell::Label(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::{parse_with_options, ParseOptions};
    use crate::ast::{ActivityStepKind, DiagramKind, StatementKind};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn define_substitution_skips_quoted_strings() {
        let doc = parse_with_options(
            "!define NAME Alice\nparticipant NAME\nnote over NAME: \"NAME\"\n",
            &ParseOptions::default(),
        )
        .unwrap();

        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::Participant(_)
        ));
        match &doc.statements[1].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.target.as_deref(), Some("Alice"));
                assert_eq!(n.text, "\"NAME\"");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn pragma_directives_with_arguments_are_preserved_as_statements() {
        let doc = parse_with_options(
            "!pragma teoz true\nparticipant A\nparticipant B\nA -> B: hi\n",
            &ParseOptions::default(),
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 4);
        assert!(matches!(doc.statements[0].kind, StatementKind::Pragma(_)));
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::Participant(_)
        ));
        assert!(matches!(
            doc.statements[2].kind,
            StatementKind::Participant(_)
        ));
        assert!(matches!(doc.statements[3].kind, StatementKind::Message(_)));
    }

    #[test]
    fn include_resolves_relative_to_root() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("inc.puml"), "A -> B\n").unwrap();

        let doc = parse_with_options(
            "!include inc.puml",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
            },
        )
        .unwrap();

        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
    }

    #[test]
    fn include_cycle_errors() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.puml"), "!include b.puml\n").unwrap();
        fs::write(dir.path().join("b.puml"), "!include a.puml\n").unwrap();

        let err = parse_with_options(
            "!include a.puml",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
            },
        )
        .unwrap_err();

        assert!(err.message.contains("include cycle detected"));
    }

    #[test]
    fn include_from_stdin_requires_root() {
        let err = parse_with_options("!include x.puml", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_INCLUDE_ROOT_REQUIRED"));
    }

    #[test]
    fn include_rejects_parent_escape_outside_root() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        let outside = dir.path().join("outside.puml");
        fs::create_dir_all(&root).unwrap();
        fs::write(&outside, "A -> B\n").unwrap();

        let err = parse_with_options(
            "!include ../outside.puml",
            &ParseOptions {
                include_root: Some(root),
            },
        )
        .unwrap_err();

        assert!(err.message.contains("E_INCLUDE_ESCAPE"));
    }

    #[cfg(unix)]
    #[test]
    fn include_rejects_symlink_target_outside_root() {
        use std::os::unix::fs::symlink;

        let dir = tempdir().unwrap();
        let root = dir.path().join("root");
        let outside = dir.path().join("outside.puml");
        let link = root.join("link_outside.puml");

        fs::create_dir_all(&root).unwrap();
        fs::write(&outside, "A -> B\n").unwrap();
        symlink(&outside, &link).unwrap();

        let err = parse_with_options(
            "!include link_outside.puml",
            &ParseOptions {
                include_root: Some(root),
            },
        )
        .unwrap_err();

        assert!(err.message.contains("E_INCLUDE_ESCAPE"));
    }

    #[test]
    fn include_id_extracts_startsub_block() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("inc.puml"),
            "!startsub FLOW\nA -> B : one\n!endsub\n",
        )
        .unwrap();

        let doc = parse_with_options(
            "!include inc.puml!FLOW",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
            },
        )
        .unwrap();

        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
    }

    #[test]
    fn include_id_missing_tag_errors() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("inc.puml"),
            "!startsub FLOW\nA -> B : one\n!endsub\n",
        )
        .unwrap();

        let err = parse_with_options(
            "!include inc.puml!MISSING",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
            },
        )
        .unwrap_err();

        assert!(err.message.contains("E_INCLUDE_TAG_NOT_FOUND"));
    }

    #[test]
    fn include_url_errors() {
        let err = parse_with_options(
            "!include https://example.com/a.puml",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_INCLUDE_URL_UNSUPPORTED"));
    }

    #[test]
    fn import_resolves_stdlib_module_from_include_root() {
        let dir = tempdir().unwrap();
        let stdlib = dir.path().join("stdlib");
        fs::create_dir_all(stdlib.join("nested")).unwrap();
        fs::write(stdlib.join("core.puml"), "A -> B : core\n").unwrap();
        fs::write(
            stdlib.join("nested").join("extra.puml"),
            "B -> A : nested\n",
        )
        .unwrap();

        let doc = parse_with_options(
            "!import core\n!import nested/extra\n!import core\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
            },
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 2);
    }

    #[test]
    fn include_angle_bracket_targets_resolve_from_stdlib_catalog() {
        let dir = tempdir().unwrap();
        let stdlib = dir.path().join("stdlib");
        fs::create_dir_all(stdlib.join("C4")).unwrap();
        fs::write(
            stdlib.join("C4").join("C4_Container.puml"),
            "!procedure Container($alias,$label)\n$alias -> $alias : [C4] $label\n!endprocedure\n",
        )
        .unwrap();

        let doc = parse_with_options(
            "!include <C4/C4_Container>\nContainer(Api, \"API\")\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
            },
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 1);
    }

    #[test]
    fn import_and_include_catalog_support_aws_shape_stub_surface() {
        let dir = tempdir().unwrap();
        let stdlib = dir.path().join("stdlib");
        fs::create_dir_all(stdlib.join("awslib14").join("Compute")).unwrap();
        fs::write(
            stdlib.join("awslib14").join("AWSCommon.puml"),
            "!procedure AWSIcon($alias,$service,$label=\"\")\n$alias -> $alias : [AWS $service] $label\n!endprocedure\n",
        )
        .unwrap();
        fs::write(
            stdlib.join("awslib14").join("Compute").join("EC2.puml"),
            "!include <awslib14/AWSCommon>\n!procedure EC2($alias,$label=\"\")\nAWSIcon($alias,EC2,$label)\n!endprocedure\n",
        )
        .unwrap();

        let doc = parse_with_options(
            "!import awslib14/AWSCommon\n!include <awslib14/Compute/EC2>\nEC2(NodeA, \"ingress\")\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
            },
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 1);
    }

    #[test]
    fn import_and_include_catalog_support_azure_and_gcp_shape_stub_surface() {
        let dir = tempdir().unwrap();
        let stdlib = dir.path().join("stdlib");

        fs::create_dir_all(stdlib.join("azure")).unwrap();
        fs::write(
            stdlib.join("azure").join("AzureCommon.puml"),
            "!procedure AzureIcon($alias,$service,$label=\"\")\n$alias -> $alias : [AZURE $service] $label\n!endprocedure\n",
        )
        .unwrap();
        fs::write(
            stdlib.join("azure").join("StorageAccount.puml"),
            "!include <azure/AzureCommon>\n!procedure AzureStorageAccount($alias,$label=\"\")\nAzureIcon($alias,StorageAccount,$label)\n!endprocedure\n",
        )
        .unwrap();

        fs::create_dir_all(stdlib.join("gcp")).unwrap();
        fs::write(
            stdlib.join("gcp").join("GCPCommon.puml"),
            "!procedure GCPIcon($alias,$service,$label=\"\")\n$alias -> $alias : [GCP $service] $label\n!endprocedure\n",
        )
        .unwrap();
        fs::write(
            stdlib.join("gcp").join("ComputeEngine.puml"),
            "!include <gcp/GCPCommon>\n!procedure GCPComputeEngine($alias,$label=\"\")\nGCPIcon($alias,ComputeEngine,$label)\n!endprocedure\n",
        )
        .unwrap();

        let doc = parse_with_options(
            "!import azure/AzureCommon\n!include <azure/StorageAccount>\nAzureStorageAccount(AzStore, \"assets\")\n!import gcp/GCPCommon\n!include <gcp/ComputeEngine>\nGCPComputeEngine(GceNode, \"ingress\")\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
            },
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 2);
    }

    #[test]
    fn import_requires_stdlib_root_when_no_include_root_is_available() {
        let err = parse_with_options("!import core\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_IMPORT_ROOT_REQUIRED"));
    }

    #[test]
    fn import_security_and_shape_errors_are_deterministic() {
        let dir = tempdir().unwrap();
        let stdlib = dir.path().join("stdlib");
        fs::create_dir_all(&stdlib).unwrap();
        fs::write(stdlib.join("ok.puml"), "A -> B\n").unwrap();

        let cases = [
            ("!import\n", "E_IMPORT_PATH_REQUIRED"),
            (
                "!import https://example.com/lib.puml\n",
                "E_IMPORT_URL_UNSUPPORTED",
            ),
            ("!import /tmp/abs.puml\n", "E_IMPORT_ABSOLUTE_PATH"),
            ("!import bad!TAG\n", "E_IMPORT_INVALID_FORM"),
            ("!import ../outside\n", "E_IMPORT_ESCAPE"),
            ("!import does/not/exist\n", "E_IMPORT_STDLIB_NOT_FOUND"),
        ];

        for (src, code) in cases {
            let err = parse_with_options(
                src,
                &ParseOptions {
                    include_root: Some(dir.path().to_path_buf()),
                },
            )
            .unwrap_err();
            assert!(
                err.message.contains(code),
                "missing {code}: {}",
                err.message
            );
        }
    }

    #[test]
    fn include_once_only_expands_first_occurrence() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("inc.puml"), "A -> B : once\n").unwrap();

        let doc = parse_with_options(
            "!include_once inc.puml\n!include_once inc.puml\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
            },
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 1);
    }

    #[test]
    fn include_many_expands_each_occurrence() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("inc.puml"), "A -> B : many\n").unwrap();

        let doc = parse_with_options(
            "!include_many inc.puml\n!include_many inc.puml\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
            },
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 2);
    }

    #[test]
    fn include_once_deduplicates_canonical_path_aliases() {
        let dir = tempdir().unwrap();
        fs::create_dir_all(dir.path().join("nested")).unwrap();
        fs::write(dir.path().join("inc.puml"), "A -> B : once\n").unwrap();

        let doc = parse_with_options(
            "!include_once ./inc.puml\n!include_once nested/../inc.puml\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
            },
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 1);
    }

    #[test]
    fn includesub_requires_tag() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("inc.puml"), "A -> B : body\n").unwrap();

        let err = parse_with_options(
            "!includesub inc.puml\n",
            &ParseOptions {
                include_root: Some(dir.path().to_path_buf()),
            },
        )
        .unwrap_err();

        assert!(err.message.contains("E_INCLUDESUB_TAG_REQUIRED"));
    }

    #[test]
    fn include_many_url_errors() {
        let err = parse_with_options(
            "!include_many https://example.com/a.puml",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_INCLUDE_URL_UNSUPPORTED"));
    }

    #[test]
    fn include_url_directive_errors_deterministically() {
        let err = parse_with_options(
            "!includeurl https://example.com/a.puml",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_INCLUDE_URL_UNSUPPORTED"));
        assert!(err
            .message
            .contains("!includeurl URL targets are not supported"));
    }

    #[test]
    fn conditional_if_elseif_else_selects_first_matching_branch() {
        let doc = parse_with_options(
            "!define FLAG 1\n!if FLAG == 1\nA -> B: first\n!elseif 1\nA -> B: second\n!else\nA -> B: third\n!endif\n",
            &ParseOptions::default(),
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("first")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn ifdef_and_ifndef_follow_define_state() {
        let doc = parse_with_options(
            "!define ENABLED 1\n!ifdef ENABLED\nA -> B: yes\n!endif\n!ifndef ENABLED\nA -> B: no\n!endif\n",
            &ParseOptions::default(),
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("yes")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn while_loops_execute_with_define_updates() {
        let doc = parse_with_options(
            "!define COUNT 2\n!while COUNT != 0\nA -> B: loop\n!if COUNT == 2\n!define COUNT 1\n!elseif COUNT == 1\n!define COUNT 0\n!endif\n!endwhile\n",
            &ParseOptions::default(),
        )
        .unwrap();

        assert_eq!(doc.statements.len(), 2);
        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
        assert!(matches!(doc.statements[1].kind, StatementKind::Message(_)));
    }

    #[test]
    fn preprocessor_function_and_procedure_blocks_are_accepted() {
        let doc = parse_with_options(
            "@startuml\n!function Echo($x)\n!return $x\n!endfunction\n!procedure Emit($x)\n!log $x\n!endprocedure\nA -> B: hi\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 1);
        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
    }

    #[test]
    fn preprocessor_variables_and_callable_args_are_applied() {
        let doc = parse_with_options(
            "@startuml\n!$from = Alice\n!$to ?= Bob\n!function F($x,$y=\"B\")\n!return $x + $y\n!endfunction\n!procedure P($a,$b=\"B\")\n$a -> $b: via-proc\n!endprocedure\n!P($from,$to)\n$from -> $to: %F(\"A\")\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 2);
        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
        assert!(matches!(doc.statements[1].kind, StatementKind::Message(_)));
    }

    #[test]
    fn preprocessor_concat_signature_and_arg_errors_are_deterministic() {
        let doc = parse_with_options(
            "@startuml\n!function Join($a##$b)\n!return $a ## $b\n!endfunction\nA -> B: %Join(Al, ice)\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("Alice")),
            other => panic!("unexpected statement: {other:?}"),
        }

        let missing = parse_with_options(
            "@startuml\n!function Need($a,$b)\n!return $a\n!endfunction\nA -> B: %Need(\"x\")\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(missing.message.contains("E_PREPROC_ARG_REQUIRED"));
    }

    #[test]
    fn preprocessor_assert_false_is_rejected() {
        let err = parse_with_options(
            "@startuml\n!assert false : expected failure\nA -> B: hi\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_PREPROC_ASSERT"));
    }

    #[test]
    fn preprocessor_assert_requires_non_empty_expression() {
        let err = parse_with_options(
            "@startuml\n!assert\nA -> B: hi\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_PREPROC_ASSERT_EXPR_REQUIRED"));
    }

    #[test]
    fn preprocessor_unknown_builtin_is_rejected_deterministically() {
        // Truly-unknown `%xyz(...)` invocations must surface a deterministic
        // diagnostic so that drift in PlantUML's builtin surface fails fast
        // instead of silently going through.
        let err = parse_with_options(
            "@startuml\n!assert %nosuchbuiltin() : no\nA -> B: hi\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(
            err.message.contains("E_PREPROC_BUILTIN_UNSUPPORTED"),
            "expected E_PREPROC_BUILTIN_UNSUPPORTED, got: {}",
            err.message
        );
    }

    #[test]
    fn preprocessor_builtin_basics_expand_inline() {
        // strlen, upper/lower, substr, intval, boolval — these used to error
        // out via E_PREPROC_BUILTIN_UNSUPPORTED. They now expand inline.
        let doc = parse_with_options(
            "@startuml\nA -> B : %strlen(\"hello\")=%upper(\"ab\")/%substr(\"plantuml\", 0, 5)\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.label.as_deref(), Some("5=AB/plant"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_json_variable_round_trips_via_get_json_attribute() {
        // JSON variable assignment is now accepted; `%get_json_attribute`
        // reads a single top-level string value.
        let doc = parse_with_options(
            "@startuml\n!$cfg = { \"name\": \"alpha\", \"v\": 2 }\nA -> B : %get_json_attribute($cfg, \"name\")\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("alpha")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_invoke_procedure_dynamically_dispatches_to_callable() {
        // `%invoke_procedure("$Say", ...)` resolves to a previously declared
        // `!procedure` and executes its body deterministically.
        let doc = parse_with_options(
            "@startuml\n!procedure $Say($who)\nA -> $who : hi\n!endprocedure\n%invoke_procedure(\"$Say\", \"Bob\")\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.to, "Bob");
                assert_eq!(m.label.as_deref(), Some("hi"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_call_user_func_supports_dynamic_function_invocation() {
        let doc = parse_with_options(
            "@startuml\n!function F($x,$y)\n!return $x + $y\n!endfunction\nA -> B : %call_user_func(\"F\", \"A\", \"B\")\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.label.as_deref(), Some("\"A\" + \"B\""));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_unclosed_function_is_rejected() {
        let err = parse_with_options(
            "@startuml\n!function Echo($x)\nA -> B: hi\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_FUNCTION_UNCLOSED"));
    }

    #[test]
    fn unknown_preprocessor_directive_errors_deterministically() {
        let err = parse_with_options("!totallynew thing\nA -> B\n", &ParseOptions::default())
            .unwrap_err();
        assert!(err.message.contains("E_PREPROC_UNSUPPORTED"));
        assert!(err.message.contains("!totallynew"));
    }

    #[test]
    fn conditional_requires_balancing_and_order() {
        let err = parse_with_options("!endif\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_PREPROC_COND_UNEXPECTED"));

        let err = parse_with_options("!if 1\nA -> B\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_PREPROC_COND_UNCLOSED"));

        let err = parse_with_options(
            "!if 1\n!else\n!elseif 1\n!endif\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_PREPROC_COND_ORDER"));
    }

    #[test]
    fn preprocessor_parenthesized_logical_conditions_are_supported() {
        let doc = parse_with_options(
            "@startuml\n!if (1 && (0 || 1))\nA -> B : yes\n!endif\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 1);
        assert!(matches!(doc.statements[0].kind, StatementKind::Message(_)));
    }

    #[test]
    fn preprocessor_conditions_support_nested_integer_arithmetic() {
        let doc = parse_with_options(
            "@startuml\n!if (2 + 3 * (4 - 1)) == 11\nA -> B : math\n!endif\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("math")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_macro_concat_collapses_expanded_function_body_tokens() {
        let doc = parse_with_options(
            "@startuml\n!function Join($a,$b)\n!return $a ## $b\n!endfunction\nA -> B : %Join(Al, ice)\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("Alice")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_json_helpers_return_nested_objects_and_empty_keys() {
        let doc = parse_with_options(
            "@startuml\n!$cfg = { \"users\": [{ \"name\": \"Ada\", \"meta\": { \"team\": \"core\" }}], \"empty\": \"\" }\n!if %json_key_exists($cfg, \"empty\")\nA -> B : %get_json_attribute($cfg, \"users[0].meta\")\n!endif\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.label.as_deref(), Some("{\"team\":\"core\"}"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn preprocessor_list_map_helpers_and_modulo_expand_inline() {
        let doc = parse_with_options(
            "@startuml\n!$cfg = { \"name\": \"Ada\", \"role\": \"core\" }\n!foreach $item in %split(\"red|blue\", \"|\")\nA -> B : $item\n!endfor\n!if 7 % 4 == 3\nA -> B : %get($cfg, \"name\")/%join([\"x\",\"y\"], \"-\")/%quote(ok)\n!endif\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        let labels = doc
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StatementKind::Message(m) => m.label.as_deref(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["red", "blue", "Ada/x-y/\"ok\""]);
    }

    #[test]
    fn while_requires_balancing() {
        let err = parse_with_options("!endwhile\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_PREPROC_WHILE_UNEXPECTED"));

        let err = parse_with_options("!while 1\nA -> B\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_PREPROC_WHILE_UNCLOSED"));
    }

    #[test]
    fn parses_multiline_title_and_legend_blocks() {
        let doc = parse_with_options(
            "title\nLine 1\nLine 2\nend title\nlegend\nAlpha\nBeta\nend legend\nA -> B\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Title(v) => assert_eq!(v, "Line 1\nLine 2"),
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Legend(v) => assert_eq!(v, "Alpha\nBeta"),
            other => panic!("unexpected statement: {other:?}"),
        }
        assert!(matches!(doc.statements[2].kind, StatementKind::Message(_)));
    }

    #[test]
    fn parses_multiline_note_block() {
        let doc = parse_with_options(
            "A -> B\nnote right of B\nline 1\nline 2\nend note\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[1].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.position, "right");
                assert_eq!(n.target.as_deref(), Some("B"));
                assert_eq!(n.text, "line 1\nline 2");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_note_across_without_target() {
        let doc =
            parse_with_options("note across: shared context\n", &ParseOptions::default()).unwrap();

        match &doc.statements[0].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.position, "across");
                assert!(n.target.is_none());
                assert_eq!(n.text, "shared context");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_multiline_note_with_inline_head_text() {
        let doc = parse_with_options(
            "note over A, B: summary\nline 2\nend note\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.position, "over");
                assert_eq!(n.target.as_deref(), Some("A, B"));
                assert_eq!(n.text, "summary\nline 2");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_hnote_and_rnote_aliases_as_note() {
        let doc = parse_with_options(
            "hnote over A: alias form\nrnote right of A: rounded alias\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.kind, crate::ast::NoteKind::Hexagonal);
                assert_eq!(n.position, "over");
                assert_eq!(n.target.as_deref(), Some("A"));
                assert_eq!(n.text, "alias form");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.kind, crate::ast::NoteKind::Rectangle);
                assert_eq!(n.position, "right");
                assert_eq!(n.target.as_deref(), Some("A"));
                assert_eq!(n.text, "rounded alias");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_hnote_and_rnote_multiline_terminators() {
        let doc = parse_with_options(
            "hnote over A\nhex body\nendhnote\nrnote over B\nrect body\nendrnote\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.kind, crate::ast::NoteKind::Hexagonal);
                assert_eq!(n.text, "hex body");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.kind, crate::ast::NoteKind::Rectangle);
                assert_eq!(n.text, "rect body");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_multiline_ref_with_inline_head_text() {
        let doc = parse_with_options(
            "ref over A, B: summary\nline 2\nend ref\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Group(g) => {
                assert_eq!(g.kind, "ref");
                assert_eq!(g.label.as_deref(), Some("over A, B\nsummary\nline 2"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn rejects_malformed_arrow_syntax() {
        let err = parse_with_options("A -x B", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_ARROW_INVALID"));
    }

    #[test]
    fn parses_lifecycle_shortcut_suffixes() {
        let doc = parse_with_options("A -> B++: inc", &ParseOptions::default()).unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.arrow, "->@R++");
                assert_eq!(m.to, "B");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_expanded_slanted_arrow_tokens() {
        let doc = parse_with_options("A -/-> B\nB -\\\\->> A\n", &ParseOptions::default()).unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => assert_eq!(m.arrow, "-/->"),
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Message(m) => assert_eq!(m.arrow, "-\\-->>"),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_filled_virtual_endpoint_side_from_message_context() {
        let doc = parse_with_options("[*] -> A\nA -> [*]\n", &ParseOptions::default()).unwrap();
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                let from_virtual = m.from_virtual.expect("from virtual");
                assert_eq!(from_virtual.side, crate::ast::VirtualEndpointSide::Left);
                assert_eq!(from_virtual.kind, crate::ast::VirtualEndpointKind::Filled);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Message(m) => {
                let to_virtual = m.to_virtual.expect("to virtual");
                assert_eq!(to_virtual.side, crate::ast::VirtualEndpointSide::Right);
                assert_eq!(to_virtual.kind, crate::ast::VirtualEndpointKind::Filled);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_queue_participant_and_separator() {
        let doc = parse_with_options(
            "queue Jobs as Q\n== Processing ==\n",
            &ParseOptions::default(),
        )
        .unwrap();

        match &doc.statements[0].kind {
            StatementKind::Participant(p) => {
                assert_eq!(p.name, "Jobs");
                assert_eq!(p.alias.as_deref(), Some("Q"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Separator(v) => assert_eq!(v.as_deref(), Some("Processing")),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_typed_group_end_keyword() {
        let doc =
            parse_with_options("alt branch\nA -> B\nend alt\n", &ParseOptions::default()).unwrap();

        match &doc.statements[2].kind {
            StatementKind::Group(g) => {
                assert_eq!(g.kind, "end");
                assert_eq!(g.label.as_deref(), Some("alt"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_class_bootstrap_declarations_and_relations() {
        let doc = parse_with_options(
            "class User\nclass Account as Acct\nUser --> Acct : owns\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Class);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::ClassDecl(_)
        ));
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::ClassDecl(_)
        ));
        match &doc.statements[2].kind {
            StatementKind::FamilyRelation(rel) => {
                assert_eq!(rel.from, "User");
                assert_eq!(rel.to, "Acct");
                assert_eq!(rel.arrow, "-->");
                assert_eq!(rel.label.as_deref(), Some("owns"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_object_and_usecase_bootstrap_kinds() {
        let object_doc =
            parse_with_options("object Order\nobject Customer\n", &ParseOptions::default())
                .unwrap();
        assert_eq!(object_doc.kind, DiagramKind::Object);

        let usecase_doc = parse_with_options(
            "usecase Authenticate\nusecase Authorize\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(usecase_doc.kind, DiagramKind::UseCase);
    }

    #[test]
    fn parses_core_uml_broad_partial_declaration_forms() {
        let class_doc = parse_with_options(
            "interface Gateway\nabstract class Shape\nannotation Trace\nstruct Payload\nGateway -[#blue,dashed]-> Shape : adapts\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(class_doc.kind, DiagramKind::Class);
        match &class_doc.statements[0].kind {
            StatementKind::ClassDecl(decl) => {
                assert_eq!(decl.name, "Gateway");
                assert_eq!(decl.members[0].text, "<<interface>>");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &class_doc.statements[1].kind {
            StatementKind::ClassDecl(decl) => {
                assert_eq!(decl.name, "Shape");
                assert_eq!(decl.members[0].text, "<<abstract class>>");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        assert!(matches!(
            class_doc.statements[4].kind,
            StatementKind::FamilyRelation(_)
        ));
        match &class_doc.statements[4].kind {
            StatementKind::FamilyRelation(rel) => assert_eq!(rel.arrow, "-->"),
            other => panic!("unexpected statement: {other:?}"),
        }

        let object_doc = parse_with_options(
            "map Settings {\n  theme => light\n}\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(object_doc.kind, DiagramKind::Object);
        match &object_doc.statements[0].kind {
            StatementKind::ObjectDecl(decl) => {
                assert_eq!(decl.name, "Settings");
                assert_eq!(decl.members[0].text, "<<map>>");
                assert_eq!(decl.members[1].text, "theme => light");
            }
            other => panic!("unexpected statement: {other:?}"),
        }

        let usecase_doc = parse_with_options(
            "actor Customer as C\nusecase (Login) as UC1\nC ..> UC1 : <<include>>\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(usecase_doc.kind, DiagramKind::UseCase);
        match &usecase_doc.statements[0].kind {
            StatementKind::UseCaseDecl(decl) => {
                assert_eq!(decl.name, "Customer");
                assert_eq!(decl.alias.as_deref(), Some("C"));
                assert_eq!(decl.members[0].text, "<<actor>>");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &usecase_doc.statements[1].kind {
            StatementKind::UseCaseDecl(decl) => {
                assert_eq!(decl.name, "Login");
                assert_eq!(decl.alias.as_deref(), Some("UC1"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &usecase_doc.statements[2].kind {
            StatementKind::FamilyRelation(rel) => {
                assert_eq!(rel.arrow, "..>");
                assert_eq!(rel.label.as_deref(), Some("<<include>>"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_sequence_decorated_arrow_styles_as_portable_arrow_core() {
        let doc = parse_with_options(
            "participant A\nparticipant B\nA -[#red,dashed]> B : styled\nB -[hidden]-> A : hidden\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Sequence);
        match &doc.statements[2].kind {
            StatementKind::Message(m) => assert_eq!(m.arrow, "->"),
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[3].kind {
            StatementKind::Message(m) => assert_eq!(m.arrow, "-->"),
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn parses_activity_switch_split_goto_and_terminal_controls() {
        let doc = parse_with_options(
            "@startuml\nstart\nswitch (kind?)\ncase (A)\n:Do A;\ncase (B)\ngoto retry\nendswitch\nsplit\n:one;\nsplit again\n:two;\nend split\nlabel retry\nbackward: retry path;\ndetach\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Activity);
        let steps = doc
            .statements
            .iter()
            .filter_map(|stmt| match &stmt.kind {
                StatementKind::ActivityStep(step) => Some(step),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::IfStart
                && step.label.as_deref() == Some("switch kind?")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Else && step.label.as_deref() == Some("A")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Fork
                && step.label.as_deref() == Some("split")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Action
                && step.label.as_deref() == Some("goto retry")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Action
                && step.label.as_deref() == Some("backward retry path")));
        assert!(steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Stop
                && step.label.as_deref() == Some("detach")));
    }

    #[test]
    fn parses_family_declaration_blocks_with_members() {
        let doc = parse_with_options(
            "class User {\n  +id: UUID\n  +name: String\n}\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Class);
        match &doc.statements[0].kind {
            StatementKind::ClassDecl(decl) => {
                assert_eq!(decl.name, "User");
                assert_eq!(decl.members.len(), 2);
                assert_eq!(decl.members[0].text, "+id: UUID");
                assert_eq!(decl.members[0].modifier, None);
                assert_eq!(decl.members[1].text, "+name: String");
                assert_eq!(decl.members[1].modifier, None);
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn unclosed_family_declaration_block_reports_deterministic_error() {
        let err = parse_with_options(
            "object Config {\nkey = \"value\"\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(err.message.contains("E_FAMILY_DECL_BLOCK_UNCLOSED"));
    }

    #[test]
    fn parses_gantt_baseline_statements() {
        let doc = parse_with_options(
            "@startgantt\n[Build]\n[Milestone] happens on 2026-05-01\n[Build] starts 2026-04-01\n[Build] requires [Design]\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Gantt);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::GanttTaskDecl { .. }
        ));
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::GanttMilestoneDecl {
                happens_on: Some(_),
                ..
            }
        ));
        assert!(doc
            .statements
            .iter()
            .any(|stmt| matches!(stmt.kind, StatementKind::GanttConstraint { .. })));
    }

    #[test]
    fn parses_gantt_dates_and_duration_baseline_statements() {
        let doc = parse_with_options(
            "@startgantt\nProject starts 2026-05-01\n[Build] lasts 5 days\n[Test] starts 2026-05-06 and lasts 2 weeks\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Gantt);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::GanttConstraint {
                ref subject,
                ref kind,
                ref target
            } if subject == "Project" && kind == "starts" && target == "2026-05-01"
        ));
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::GanttTaskDecl {
                ref name,
                duration_days: Some(5),
                ..
            } if name == "Build"
        ));
        assert!(matches!(
            doc.statements[2].kind,
            StatementKind::GanttTaskDecl {
                ref name,
                start_date: Some(ref d),
                duration_days: Some(14),
                ..
            } if name == "Test" && d == "2026-05-06"
        ));
    }

    #[test]
    fn parses_gantt_closed_weekday_calendar_statements() {
        let doc = parse_with_options(
            "@startgantt\nProject starts 2026-05-01\nsaturday are closed\nsundays are closed\n[Build] lasts 2 days\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Gantt);
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::GanttCalendarClosed { ref day } if day == "saturday"
        ));
        assert!(matches!(
            doc.statements[2].kind,
            StatementKind::GanttCalendarClosed { ref day } if day == "sunday"
        ));
    }

    #[test]
    fn parses_gantt_closed_date_range_calendar_statement() {
        let doc = parse_with_options(
            "@startgantt\nProject starts 2026-05-01\n2026-05-04 to 2026-05-05 is closed\n[Build] lasts 2 days\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Gantt);
        assert!(matches!(
            doc.statements[1].kind,
            StatementKind::GanttCalendarClosedDateRange {
                ref start_date,
                ref end_date
            } if start_date == "2026-05-04" && end_date == "2026-05-05"
        ));
    }

    #[test]
    fn parses_chronology_happens_on_baseline_statement() {
        let doc = parse_with_options(
            "@startchronology\nRelease happens on 2026-05-15\n@endchronology\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Chronology);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::ChronologyHappensOn { .. }
        ));
    }

    #[test]
    fn parses_usecase_relations_with_alias_and_label() {
        let doc = parse_with_options(
            "usecase Authenticate as Auth\nusecase User\nAuth --> User : validates\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::UseCase);
        match &doc.statements[2].kind {
            StatementKind::FamilyRelation(rel) => {
                assert_eq!(rel.from, "Auth");
                assert_eq!(rel.to, "User");
                assert_eq!(rel.arrow, "-->");
                assert_eq!(rel.label.as_deref(), Some("validates"));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }

    #[test]
    fn malformed_family_relation_is_preserved_as_unknown_statement() {
        let doc = parse_with_options("class User\nUser -->\n", &ParseOptions::default()).unwrap();
        assert_eq!(doc.kind, DiagramKind::Class);
        assert!(matches!(doc.statements[1].kind, StatementKind::Unknown(_)));
    }

    #[test]
    fn state_keyword_is_parsed_as_state_decl() {
        let doc = parse_with_options("state Running\n", &ParseOptions::default()).unwrap();
        assert_eq!(doc.kind, DiagramKind::State);
        assert!(matches!(
            doc.statements[0].kind,
            StatementKind::StateDecl(_)
        ));
    }

    #[test]
    fn mixed_family_input_reports_deterministic_error() {
        let err = parse_with_options("class A\nnewpage\n", &ParseOptions::default()).unwrap_err();
        assert!(err.message.contains("E_FAMILY_MIXED"));
    }

    #[test]
    fn start_enduml_markers_accept_optional_block_suffixes() {
        let doc = parse_with_options(
            "@startuml \"Primary\"\nA -> B: one\n@enduml anything\n@startuml Second\nB -> A: two\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Sequence);
        let labels = doc
            .statements
            .iter()
            .filter_map(|s| match &s.kind {
                StatementKind::Message(m) => m.label.as_deref(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(labels, vec!["one", "two"]);
    }

    #[test]
    fn start_end_timeline_markers_accept_optional_block_suffixes() {
        let gantt = parse_with_options(
            "@startgantt \"Gantt\"\n[2026-01] : one\n@endgantt anything\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(gantt.kind, DiagramKind::Gantt);

        let chronology = parse_with_options(
            "@startchronology\nEvent\n@endchronology now\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(chronology.kind, DiagramKind::Chronology);
    }

    #[test]
    fn startmindmap_and_startwbs_markers_set_family_kind() {
        let mindmap = parse_with_options(
            "@startmindmap\n* Root\n** Child\n@endmindmap\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(mindmap.kind, DiagramKind::MindMap);

        let wbs =
            parse_with_options("@startwbs\n* Scope\n@endwbs\n", &ParseOptions::default()).unwrap();
        assert_eq!(wbs.kind, DiagramKind::Wbs);

        let gantt = parse_with_options(
            "@startgantt\n[2026-01-01] : Kickoff\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(gantt.kind, DiagramKind::Gantt);

        let chronology = parse_with_options(
            "@startchronology\n2026-01-01 : Event\n@endchronology\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(chronology.kind, DiagramKind::Chronology);
    }

    #[test]
    fn parses_activity_oldstyle_baseline_statements() {
        let doc = parse_with_options(
            "@startuml\n|Build|\n(*) --> \"Init\"\n#gold:Compile;\n-->[next] right of \"Test\"\n\"Test\" --> (*)\ndetach\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Activity);
        assert!(!doc.statements.is_empty());
    }

    #[test]
    fn mismatched_start_end_family_markers_report_deterministic_error() {
        let err = parse_with_options("@startmindmap\n* Root\n@endwbs\n", &ParseOptions::default())
            .unwrap_err();
        assert!(err.message.contains("E_BLOCK_MISMATCH"));
    }

    #[test]
    fn apostrophe_comments_are_ignored_but_preserved_inside_quotes() {
        let doc = parse_with_options(
            "@startuml\n' full line comment\nA -> B: \"don't split\" ' trailing comment\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
        assert_eq!(doc.kind, DiagramKind::Sequence);
        assert_eq!(doc.statements.len(), 1);
        match &doc.statements[0].kind {
            StatementKind::Message(m) => {
                assert_eq!(m.label.as_deref(), Some("\"don't split\""));
            }
            other => panic!("unexpected statement: {other:?}"),
        }
    }
}
