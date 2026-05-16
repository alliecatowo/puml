use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{
    ClassDecl, DiagramKind, Document, FamilyRelation, Group, Message, Note, ObjectDecl,
    ParticipantDecl, ParticipantRole, Statement, StatementKind, UseCaseDecl, VirtualEndpoint,
    VirtualEndpointKind, VirtualEndpointSide,
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
                        ensure_no_unsupported_builtin_payload(&payload)?;
                    }
                }
                PreprocessDirective::DumpMemory(payload) => {
                    if active {
                        ensure_no_unsupported_builtin_payload(&payload)?;
                    }
                }
                PreprocessDirective::DynamicInvocation(raw) => {
                    if active {
                        return Err(Diagnostic::error_code(
                            "E_PREPROC_DYNAMIC_UNSUPPORTED",
                            format!(
                                "dynamic preprocessor invocation is not supported in this deterministic subset: `{}`",
                                raw
                            ),
                        ));
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
                        if trimmed.starts_with('{') || trimmed.starts_with('[') {
                            return Err(Diagnostic::error_code(
                                "E_PREPROC_JSON_UNSUPPORTED",
                                format!(
                                    "JSON preprocessing is not supported in this deterministic subset: `!${} = {}`",
                                    name, value
                                ),
                            ));
                        }
                        if !conditional || !state.vars.contains_key(&name) {
                            let rendered = substitute_tokens_and_vars(&value, state);
                            state.vars.insert(name, rendered.trim().to_string());
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
                        process_include_directive(
                            &raw_target,
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
        "return" | "foreach" | "endfor" | "startsub" | "endsub" => {
            Some(PreprocessDirective::Unsupported(name.to_string()))
        }
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
    if contains_builtin_invocation(expression) {
        return Err(Diagnostic::error_code(
            "E_PREPROC_BUILTIN_UNSUPPORTED",
            "preprocessor builtin functions (`%...`) are not supported in this deterministic subset",
        ));
    }
    evaluate_preprocess_expr(expression, state)
}

fn ensure_no_unsupported_builtin_payload(payload: &str) -> Result<(), Diagnostic> {
    if contains_builtin_invocation(payload) {
        return Err(Diagnostic::error_code(
            "E_PREPROC_BUILTIN_UNSUPPORTED",
            "preprocessor builtin functions (`%...`) are not supported in this deterministic subset",
        ));
    }
    Ok(())
}

fn contains_builtin_invocation(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] != b'%' {
            continue;
        }
        let mut j = i + 1;
        let mut saw_ident = false;
        while j < bytes.len() {
            let ch = bytes[j] as char;
            if ch.is_ascii_alphanumeric() || ch == '_' {
                saw_ident = true;
                j += 1;
                continue;
            }
            break;
        }
        if saw_ident && j < bytes.len() && bytes[j] == b'(' {
            return true;
        }
    }
    false
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
                let callable = state.callables.get(&call_name).ok_or_else(|| {
                    Diagnostic::error_code(
                        "E_PREPROC_BUILTIN_UNSUPPORTED",
                        format!(
                            "preprocessor builtin or unknown function `%{}(...)` is not supported in this deterministic subset",
                            call_name
                        ),
                    )
                })?;
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
        }
        out.push(chars[i]);
        i += 1;
    }
    Ok(out)
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
    let mut depth = 0usize;
    let mut in_quotes = false;
    for ch in raw.chars() {
        if ch == '"' {
            in_quotes = !in_quotes;
            curr.push(ch);
            continue;
        }
        if !in_quotes {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                if depth == 0 {
                    return Err(Diagnostic::error_code(
                        "E_PREPROC_CALL_SYNTAX",
                        "unbalanced `)` in argument list",
                    ));
                }
                depth -= 1;
            } else if ch == ',' && depth == 0 {
                out.push(curr.trim().to_string());
                curr.clear();
                continue;
            }
        }
        curr.push(ch);
    }
    if in_quotes || depth != 0 {
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

        if detected_kind.is_none() {
            if let Some(kind) = detect_non_sequence_family(line) {
                detected_kind = Some(kind);
            } else if looks_like_unsupported_family_syntax(line) {
                detected_kind = Some(DiagramKind::Unknown);
            }
        }

        let allow_sequence_parse =
            detected_kind.is_none() || matches!(detected_kind, Some(DiagramKind::Sequence));
        let allow_gantt_parse = matches!(detected_kind, Some(DiagramKind::Gantt));
        let allow_chronology_parse = matches!(detected_kind, Some(DiagramKind::Chronology));

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
    MindMap,
    Wbs,
    Gantt,
    Chronology,
}

fn parse_start_block_kind(line: &str) -> Option<BlockKind> {
    parse_block_marker_kind(line, true)
}

fn parse_end_block_kind(line: &str) -> Option<BlockKind> {
    parse_block_marker_kind(line, false)
}

fn parse_block_marker_kind(line: &str, start: bool) -> Option<BlockKind> {
    let lower = line.to_ascii_lowercase();
    let markers = if start {
        [
            ("@startuml", BlockKind::Uml),
            ("@startmindmap", BlockKind::MindMap),
            ("@startwbs", BlockKind::Wbs),
            ("@startgantt", BlockKind::Gantt),
            ("@startchronology", BlockKind::Chronology),
        ]
    } else {
        [
            ("@enduml", BlockKind::Uml),
            ("@endmindmap", BlockKind::MindMap),
            ("@endwbs", BlockKind::Wbs),
            ("@endgantt", BlockKind::Gantt),
            ("@endchronology", BlockKind::Chronology),
        ]
    };
    for (marker, kind) in markers {
        if lower.starts_with(marker) {
            let rest = &line[marker.len()..];
            if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                return Some(kind);
            }
        }
    }
    None
}

fn start_block_family(kind: BlockKind) -> Option<DiagramKind> {
    match kind {
        BlockKind::Uml => None,
        BlockKind::MindMap => Some(DiagramKind::MindMap),
        BlockKind::Wbs => Some(DiagramKind::Wbs),
        BlockKind::Gantt => Some(DiagramKind::Gantt),
        BlockKind::Chronology => Some(DiagramKind::Chronology),
    }
}

fn block_kind_name(kind: BlockKind) -> &'static str {
    match kind {
        BlockKind::Uml => "uml",
        BlockKind::MindMap => "mindmap",
        BlockKind::Wbs => "wbs",
        BlockKind::Gantt => "gantt",
        BlockKind::Chronology => "chronology",
    }
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
        DiagramKind::MindMap => "mindmap",
        DiagramKind::Wbs => "wbs",
        DiagramKind::Gantt => "gantt",
        DiagramKind::Chronology => "chronology",
        DiagramKind::Component => "component",
        DiagramKind::Deployment => "deployment",
        DiagramKind::State => "state",
        DiagramKind::Activity => "activity",
        DiagramKind::Timing => "timing",
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
        Some(DiagramKind::Class) | Some(DiagramKind::Object) | Some(DiagramKind::UseCase) => {}
        _ => return None,
    }

    let (core, label) = split_message_label(line);
    let (lhs, arrow, rhs) = split_arrow(core)?;
    let from = clean_ident(lhs);
    let to = clean_ident(rhs);
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
        || line.starts_with("scale ")
    {
        return Some(DiagramKind::Timing);
    }

    if line.starts_with("gantt ") {
        return Some(DiagramKind::Gantt);
    }

    if line.starts_with("chronology ") {
        return Some(DiagramKind::Chronology);
    }

    None
}

fn parse_gantt_baseline_statement(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    let (subject, rest) = parse_bracket_subject(trimmed)?;
    if rest.is_empty() {
        return Some(StatementKind::GanttTaskDecl { name: subject });
    }
    let rest = rest.trim();
    if let Some(rest) = rest.strip_prefix(':') {
        return Some(StatementKind::GanttTaskDecl {
            name: rest.trim().to_string(),
        });
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

fn parse_chronology_baseline_statement(line: &str) -> Option<StatementKind> {
    let trimmed = line.trim();
    if let Some((subject, when)) = trimmed.split_once(':') {
        if !when.trim().is_empty() && !subject.trim().is_empty() {
            return Some(StatementKind::ChronologyHappensOn {
                subject: subject.trim().trim_matches('"').to_string(),
                when: when.trim().to_string(),
            });
        }
    }
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
    let key = ["title", "header", "footer", "caption", "legend"]
        .into_iter()
        .find(|k| line.eq_ignore_ascii_case(k))?;
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
                _ => StatementKind::Legend(text),
            };
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

    for g in ["alt", "opt", "loop", "par", "critical", "break", "group"] {
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
            "alt" | "opt" | "loop" | "par" | "critical" | "break" | "group" | "ref"
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
    fn preprocessor_builtin_dynamic_and_json_edges_are_deterministic() {
        for (src, code) in [
            (
                "@startuml\n!assert %true() : no\nA -> B: hi\n@enduml\n",
                "E_PREPROC_BUILTIN_UNSUPPORTED",
            ),
            (
                "@startuml\n!log %boolval(\"x\")\nA -> B: hi\n@enduml\n",
                "E_PREPROC_BUILTIN_UNSUPPORTED",
            ),
            (
                "@startuml\n%invoke_procedure(\"$go\")\nA -> B: hi\n@enduml\n",
                "E_PREPROC_DYNAMIC_UNSUPPORTED",
            ),
            (
                "@startuml\n!$foo = { \"k\": 1 }\nA -> B: hi\n@enduml\n",
                "E_PREPROC_JSON_UNSUPPORTED",
            ),
        ] {
            let err = parse_with_options(src, &ParseOptions::default()).unwrap_err();
            assert!(
                err.message.contains(code),
                "missing {code}: {}",
                err.message
            );
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
    fn unsupported_family_keyword_is_tagged_for_family_routing() {
        let doc = parse_with_options("state Running\n", &ParseOptions::default()).unwrap();
        assert_eq!(doc.kind, DiagramKind::State);
        assert!(matches!(doc.statements[0].kind, StatementKind::Unknown(_)));
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
