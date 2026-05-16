use std::cell::RefCell;
#![allow(clippy::manual_strip)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::manual_map)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{
    ActivityStep, ActivityStepKind, ClassDecl, ComponentNodeKind, DiagramKind, Document,
    FamilyRelation, Group, Message, Note, ObjectDecl, ParticipantDecl, ParticipantRole, StateDecl,
    StateInternalAction, StateTransition, Statement, StatementKind, TimingDeclKind, UseCaseDecl,
    VirtualEndpoint, VirtualEndpointKind, VirtualEndpointSide,
};
use crate::diagnostic::Diagnostic;
use crate::source::Span;

const MAX_INCLUDE_DEPTH: usize = 32;
const MAX_PREPROC_WHILE_ITERATIONS: usize = 256;
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
                                let rendered = substitute_tokens_and_vars(&value, state);
                                state.vars.insert(name, rendered.trim().to_string());
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

    let resolved = resolve_include_path(options, include_stack, &include_target.path)?;
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
        "endwhile" => Some(PreprocessDirective::EndWhile),
        "function" => Some(PreprocessDirective::Function),
        "endfunction" => Some(PreprocessDirective::EndFunction),
        "procedure" => Some(PreprocessDirective::Procedure),
        "endprocedure" => Some(PreprocessDirective::EndProcedure),
        "assert" => Some(PreprocessDirective::Assert(arg.to_string())),
        "log" => Some(PreprocessDirective::Log(arg.to_string())),
        "dump_memory" => Some(PreprocessDirective::DumpMemory(arg.to_string())),
        _ if name.starts_with('$') => parse_variable_assignment(name, arg, trimmed),
        "return" | "foreach" | "endfor" => Some(PreprocessDirective::Unsupported(name.to_string())),
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
    if raw.contains("&&") || raw.contains("||") {
        return Err(Diagnostic::error_code(
            "E_PREPROC_EXPR_UNSUPPORTED",
            "only simple conditions are supported in this preprocessor slice",
        ));
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

    if let Some((lhs, rhs)) = trimmed.split_once("==") {
        return Ok(normalize_expr_value(lhs) == normalize_expr_value(rhs));
    }
    if let Some((lhs, rhs)) = trimmed.split_once("!=") {
        return Ok(normalize_expr_value(lhs) != normalize_expr_value(rhs));
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

fn normalize_expr_value(value: &str) -> String {
    value
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
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
    let substituted = substitute_tokens_and_vars(raw_line, state);
    expand_function_invocations(&substituted, state, call_depth)
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
        "strlen" | "size" => Some(arg(0).chars().count().to_string()),
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
        "str" => Some(arg(0)),
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
        "feature" => Some(String::new()),
        "get_variable_value" => {
            let key = arg(0);
            Some(state.vars.get(&key).cloned().unwrap_or_default())
        }
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
            return Err(Diagnostic::error_code(
                "E_PREPROC_DYNAMIC_UNSUPPORTED",
                format!(
                    "dynamic preprocessor invocation `%{}(...)` is not supported in this deterministic subset",
                    name
                ),
            ));
        }
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

/// PlantUML-ish truthiness for `%boolval`/`%not`.
fn boolval(s: &str) -> bool {
    let t = s.trim();
    if t.is_empty() {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    !matches!(lower.as_str(), "0" | "false" | "no" | "off")
}

/// Minimal top-level JSON key lookup so `%get_json_attribute` can serve the
/// common case `!$cfg = { "name": "X" }` → `%get_json_attribute($cfg, "name")`.
/// Returns the value for the first matching key as a string (with quotes
/// stripped for string values; numeric/boolean/null left verbatim). Returns
/// an empty string when the input is not an object or the key is missing.
/// This intentionally avoids pulling a full JSON dependency.
fn get_json_attribute(json: &str, key: &str) -> String {
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

fn json_contains_key(json: &str, key: &str) -> bool {
    // Cheap reuse: re-run lookup; non-empty result OR explicit empty-but-present
    // would require deeper parsing. Treat presence as "key appears as a JSON
    // key in the object". For our minimal model, empty-string returns mean
    // either missing or empty-value; PlantUML callers nearly always pair this
    // with a non-empty value so we equate "has non-empty value" with present.
    // This deterministic simplification is documented above.
    !get_json_attribute(json, key).is_empty()
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
    if params_raw.contains("##") {
        return Err(Diagnostic::error_code(
            "E_PREPROC_CONCAT_UNSUPPORTED",
            "macro argument concatenation (`##`) is not supported in this deterministic subset",
        ));
    }
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
    for piece in split_args(raw)? {
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
    if args_raw.contains("##") {
        return Err(Diagnostic::error_code(
            "E_PREPROC_CONCAT_UNSUPPORTED",
            "macro argument concatenation (`##`) is not supported in this deterministic subset",
        ));
    }
    let mut bound = BTreeMap::new();
    let mut positional = Vec::new();
    let mut keyword = BTreeMap::new();
    for arg in split_args(args_raw)? {
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

        if detected_kind.is_none() {
            if let Some(kind) = detect_non_sequence_family(line) {
                detected_kind = Some(kind);
            } else if looks_like_unsupported_family_syntax(line) {
                detected_kind = Some(DiagramKind::Unknown);
            }
        }

        // Family-specific inline parsing for the newly-implemented families.
        if matches!(
            detected_kind,
            Some(DiagramKind::Component) | Some(DiagramKind::Deployment)
        ) {
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

        if allow_gantt_parse {
            if let Some(kind) = parse_gantt_baseline_statement(line) {
                statements.push(Statement { span, kind });
                i += 1;
                continue;
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
    if let Some((name, alias, has_block)) = parse_named_family_decl(line, "class") {
        let members = if has_block {
            parse_family_decl_members(lines, start, "class", &name)?
        } else {
            Vec::new()
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
    if let Some((name, alias, has_block)) = parse_named_family_decl(line, "object") {
        let members = if has_block {
            parse_family_decl_members(lines, start, "object", &name)?
        } else {
            Vec::new()
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
    if let Some((name, alias, has_block)) = parse_named_family_decl(line, "usecase") {
        let members = if has_block {
            parse_family_decl_members(lines, start, "usecase", &name)?
        } else {
            Vec::new()
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

fn parse_named_family_decl(line: &str, keyword: &str) -> Option<(String, Option<String>, bool)> {
    if !line.starts_with(keyword) {
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

    let name = clean_ident(name_raw);
    if name.is_empty() {
        return None;
    }
    let alias = alias_raw.map(clean_ident).filter(|v| !v.is_empty());
    Some((name, alias, has_block))
}

fn parse_family_decl_members(
    lines: &[(&str, Span)],
    start: usize,
    keyword: &str,
    name: &str,
) -> Result<Vec<String>, Diagnostic> {
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
            members.push(trimmed.to_string());
        }
    }
    Ok(members)
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

    let (core, label) = split_message_label(line);
    let (lhs, arrow, rhs) = split_arrow(core)?;
    let from = clean_bracketed_ident(lhs);
    let to = clean_bracketed_ident(rhs);
    if from.is_empty() || to.is_empty() {
        return None;
    }
    Some(StatementKind::FamilyRelation(FamilyRelation {
        from,
        to,
        arrow: arrow.to_string(),
        label,
    }))
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
            .map(|s| clean_ident(s))
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
        let rest = line["package ".len()..].trim();
        let label_raw = rest.trim_end_matches('{').trim();
        let label = clean_ident(label_raw.trim_matches('"'));
        let end_idx = find_family_decl_end(lines, start);
        if end_idx == start {
            return Err(Diagnostic::error(
                "[E_CLASS_PACKAGE_UNCLOSED] unclosed `package` block: missing `}`",
            )
            .with_span(lines[start].1));
        }
        let members: Vec<String> = lines[start + 1..end_idx]
            .iter()
            .map(|(raw, _)| raw.trim())
            .filter(|s| !s.is_empty())
            .map(|s| extract_class_member_name(s))
            .filter(|s| !s.is_empty())
            .collect();
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
        let rest = line["namespace ".len()..].trim();
        let label_raw = rest.trim_end_matches('{').trim();
        let label = clean_ident(label_raw.trim_matches('"'));
        let end_idx = find_family_decl_end(lines, start);
        if end_idx == start {
            return Err(Diagnostic::error(
                "[E_CLASS_NAMESPACE_UNCLOSED] unclosed `namespace` block: missing `}`",
            )
            .with_span(lines[start].1));
        }
        let members: Vec<String> = lines[start + 1..end_idx]
            .iter()
            .map(|(raw, _)| raw.trim())
            .filter(|s| !s.is_empty())
            .map(|s| extract_class_member_name(s))
            .filter(|s| !s.is_empty())
            .collect();
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

/// Parse single-line class options: `set namespaceSeparator`, `hide circle`, etc.
fn parse_class_option_statement(line: &str) -> Option<StatementKind> {
    let lower = line.to_ascii_lowercase();

    // set namespaceSeparator <sep>
    if lower.starts_with("set namespaceseparator") {
        let rest = line["set namespaceSeparator".len()..].trim();
        return Some(StatementKind::SetOption {
            key: "namespaceSeparator".to_string(),
            value: rest.to_string(),
        });
    }

    // hide options: hide circle, hide stereotype, hide empty members, hide empty methods, hide empty fields
    if lower.starts_with("hide ") {
        let rest = lower["hide ".len()..].trim();
        let known = [
            "circle",
            "stereotype",
            "empty members",
            "empty methods",
            "empty fields",
        ];
        for opt in known {
            if rest == opt {
                return Some(StatementKind::HideOption(rest.to_string()));
            }
        }
    }

    None
}

fn detect_non_sequence_family(line: &str) -> Option<DiagramKind> {
    if line.starts_with("component ")
        || line.starts_with("interface ")
        || line.starts_with("port ")
        || line.starts_with("portin ")
        || line.starts_with("portout ")
    {
        return Some(DiagramKind::Component);
    }

    if line.starts_with("node ")
        || line.starts_with("artifact ")
        || line.starts_with("cloud ")
        || line.starts_with("frame ")
        || line.starts_with("storage ")
    {
        return Some(DiagramKind::Deployment);
    }

    if line.starts_with("state ") || line == "[*]" || line == "[H]" || line == "[H*]" {
        return Some(DiagramKind::State);
    }
    // State transitions involving pseudo-states
    if line.starts_with("[*]") || line.starts_with("[H]") || line.starts_with("[H*]") {
        if line.contains("-->") {
            return Some(DiagramKind::State);
        }
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
        || line.starts_with("if ")
        || line.starts_with("elseif ")
        || line == "else"
        || line.starts_with("endif")
        || line.starts_with("repeat")
        || line.starts_with("while ")
        || line.starts_with("fork")
        || line.starts_with("partition ")
        || line.starts_with("swimlane ")
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

    None
}

fn parse_gantt_baseline_statement(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    let (subject, rest) = parse_bracket_subject(trimmed)?;
    if rest.is_empty() {
        return Some(StatementKind::GanttTaskDecl { name: subject });
    }
    let lower = rest.to_ascii_lowercase();
    if lower.starts_with("happens") {
        return Some(StatementKind::GanttMilestoneDecl { name: subject });
    }
    for kind in ["starts", "ends", "requires"] {
        if lower.starts_with(kind) {
            return Some(StatementKind::GanttConstraint {
                subject,
                kind: kind.to_string(),
                target: rest.to_string(),
            });
        }
    }
    None
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
            .map_or(false, |b| b == b' ' || b == b'\t')
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
            let stripped = &rest[1..];
            let end = stripped.find('"')?;
            (
                Some(stripped[..end].to_string()),
                stripped[end + 1..].trim(),
            )
        } else {
            (None, rest)
        };
        let (name_raw, alias_raw) = if let Some((lhs, rhs)) = rest_after_label.split_once(" as ") {
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
        let body = rest.trim_end_matches("then").trim();
        return Some(StatementKind::ActivityStep(ActivityStep {
            kind: ActivityStepKind::IfStart,
            label: Some(extract_paren_label(body).unwrap_or_else(|| body.to_string())),
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
    None
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
                let stripped = &rest[1..];
                let end = stripped.find('"')?;
                let rem = stripped[end + 1..].trim();
                let name = rem.strip_prefix("as ").map(str::trim).unwrap_or(rem).trim();
                (Some(stripped[..end].to_string()), name)
            } else if let Some((lhs, rhs)) = rest.split_once(" as ") {
                (Some(lhs.trim().to_string()), rhs.trim())
            } else {
                (None, rest)
            };
            let name = clean_ident(name_raw);
            if name.is_empty() {
                return None;
            }
            return Some(StatementKind::TimingDecl { kind, name, label });
        }
    }
    None
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
    let lower = line.to_ascii_lowercase();
    let marker = " happens on ";
    let idx = lower.find(marker)?;
    let subject = line[..idx].trim().trim_matches('"').to_string();
    let when = line[idx + marker.len()..].trim().to_string();
    if subject.is_empty() || when.is_empty() {
        return None;
    }
    Some(StatementKind::ChronologyHappensOn { subject, when })
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
        let rest = line["state ".len()..].trim();
        if rest.is_empty() {
            return Ok(None);
        }

        // Extract optional stereotype `<<...>>`
        let (name_part, stereotype) = if let Some(idx) = rest.find("<<") {
            let name = rest[..idx].trim();
            let after = &rest[idx + 2..];
            let stereo = if let Some(end) = after.find(">>") {
                Some(after[..end].trim().to_string())
            } else {
                None
            };
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
    // Look for `-->` arrow
    let arrow = "-->";
    let idx = line.find(arrow)?;
    let from_raw = line[..idx].trim();
    let rest = line[idx + arrow.len()..].trim();

    // Split `To : label`
    let (to_raw, label) = if let Some((to_part, lbl)) = rest.split_once(':') {
        (to_part.trim(), Some(lbl.trim().to_string()))
    } else {
        (rest, None)
    };

    if from_raw.is_empty() || to_raw.is_empty() {
        return None;
    }

    Some(StateTransition {
        from: from_raw.to_string(),
        to: to_raw.to_string(),
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
        from_virtual,
        to_virtual,
    }))
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
        // Treat `hide unlinked` as a skinparam-style flag; the normalizer
        // pipeline records it as a pragma so the layout layer can later
        // filter out participants that never appear on a message.
        return Some(StatementKind::SkinParam {
            key: "hideUnlinked".to_string(),
            value: "true".to_string(),
        });
    }

    // scale directive: "scale <factor>", "scale <w>*<h>", "scale max <n>"
    if lower.starts_with("scale ") {
        let body = line[6..].trim();
        return Some(StatementKind::Scale(body.to_string()));
    }

    // Class-diagram hide options (parsed here so they work before any class decl sets detected_kind)
    if lower.starts_with("hide ") {
        let rest = lower["hide ".len()..].trim();
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
        if pos.eq_ignore_ascii_case("of")
            || !is_valid_note_position(&pos)
            || (matches!(pos.to_ascii_lowercase().as_str(), "left" | "right") && target.is_none())
        {
            return Some(StatementKind::Unknown(format!(
                "[E_NOTE_INVALID] malformed note syntax: `{}`",
                line
            )));
        }
        return Some(StatementKind::Note(Note {
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

fn is_valid_note_position(position: &str) -> bool {
    matches!(
        position.to_ascii_lowercase().as_str(),
        "left" | "right" | "over" | "across"
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
    for (idx, ch) in core.char_indices() {
        if is_arrow_char(ch) {
            if run_start.is_none() {
                run_start = Some(idx);
            }
            continue;
        }
        if let Some(start) = run_start.take() {
            let candidate = &core[start..idx];
            if !candidate.contains('-') {
                continue;
            }
            let lhs = core[..start].trim();
            let rhs = core[idx..].trim();
            if !lhs.is_empty() && !rhs.is_empty() {
                return Some((lhs, candidate.trim(), rhs));
            }
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
        StatementKind::Note(_)
            | StatementKind::Group(_)
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

#[cfg(test)]
mod tests {
    use super::{parse_with_options, ParseOptions};
    use crate::ast::{DiagramKind, StatementKind};
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
    fn preprocessor_concat_and_arg_errors_are_deterministic() {
        let concat = parse_with_options(
            "@startuml\n!function Join($a##$b)\n!return $a\n!endfunction\nA -> B\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap_err();
        assert!(concat.message.contains("E_PREPROC_CONCAT_UNSUPPORTED"));

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
                assert_eq!(n.position, "over");
                assert_eq!(n.target.as_deref(), Some("A"));
                assert_eq!(n.text, "alias form");
            }
            other => panic!("unexpected statement: {other:?}"),
        }
        match &doc.statements[1].kind {
            StatementKind::Note(n) => {
                assert_eq!(n.position, "right");
                assert_eq!(n.target.as_deref(), Some("A"));
                assert_eq!(n.text, "rounded alias");
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
                assert_eq!(
                    decl.members,
                    vec!["+id: UUID".to_string(), "+name: String".to_string()]
                );
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
            StatementKind::GanttMilestoneDecl { .. }
        ));
        assert!(matches!(
            doc.statements[2].kind,
            StatementKind::GanttConstraint { .. }
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
