use std::collections::BTreeSet;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
use std::io::Read;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::path::PathBuf;

#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
use sha2::{Digest, Sha256};

use crate::diagnostic::Diagnostic;

use super::{IncludeTarget, ParseOptions, PreprocState, PreprocVariableScope, PreprocessDirective};
#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
use super::{URL_INCLUDE_MAX_BYTES, URL_INCLUDE_TIMEOUT};

use super::macros::{
    expand_preprocessor_text, parse_named_call, parse_scoped_variable_assignment,
    parse_variable_assignment,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::preproc::control::preprocess_text;

/// On `wasm32` there is no filesystem available, so the entire `!include` /
/// `!includesub` / `!include_many` / `!import` family returns a friendly error
/// rather than attempting to read files. All FS-touching code below is gated
/// with `cfg(not(target_arch = "wasm32"))`; these stubs satisfy the call sites.
#[cfg(target_arch = "wasm32")]
pub(super) fn include_not_supported_in_wasm(directive_name: &str) -> Diagnostic {
    Diagnostic::error_code(
        "E_INCLUDE_NOT_SUPPORTED_WASM",
        format!(
            "{directive_name} is not available in the in-browser renderer — the WASM build has no filesystem"
        ),
    )
}

#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
pub(super) fn process_include_directive(
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
pub(super) fn process_include_many_directive(
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
pub(super) fn process_import_directive(
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
pub(super) fn process_include_directive(
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
                    "{directive_name} URL includes are disabled (pass --allow-url-includes to enable): {}",
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
pub(super) fn is_stdlib_catalog_target(raw_target: &str) -> bool {
    let trimmed = raw_target.trim();
    trimmed.starts_with('<') && trimmed.ends_with('>')
}

/// `!include_many` with optional glob expansion. When the path contains `*`
/// or `?`, expand it to every matching file in deterministic alphabetical
/// order; otherwise behave like `!include`. Globs only match the file-name
/// segment of the path so we cannot escape the include root by accident.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub(super) fn process_include_many_directive(
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
                format!(
                    "!include_many URL includes are disabled (pass --allow-url-includes to enable): {}",
                    raw_target
                ),
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
    let target = parse_stdlib_angle_include_target(raw_target, directive_name)?;
    let inner = target.display_name.as_str();
    let path = target.path;

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

    let mut content = fs::read_to_string(&resolved).map_err(|e| {
        Diagnostic::error_code(
            "E_INCLUDE_READ",
            format!(
                "failed to read stdlib include '{}': {e}",
                resolved.display()
            ),
        )
    })?;
    if let Some(tag) = target.tag.as_deref() {
        content = extract_include_tag(&content, tag).ok_or_else(|| {
            Diagnostic::error_code(
                "E_INCLUDE_TAG_NOT_FOUND",
                format!(
                    "include tag '{}' was not found in stdlib include '{}'",
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
struct StdlibAngleIncludeTarget {
    path: PathBuf,
    display_name: String,
    tag: Option<String>,
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn parse_stdlib_angle_include_target(
    raw_target: &str,
    directive_name: &str,
) -> Result<StdlibAngleIncludeTarget, Diagnostic> {
    let trimmed = raw_target.trim();
    let Some(after_open) = trimmed.strip_prefix('<') else {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_INVALID_FORM",
            format!("{directive_name} stdlib target must start with `<`: {raw_target}"),
        ));
    };
    let Some(close_index) = after_open.find('>') else {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_INVALID_FORM",
            format!("{directive_name} stdlib target is missing closing `>`: {raw_target}"),
        ));
    };

    let inner = after_open[..close_index].trim();
    if inner.is_empty() {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_PATH_REQUIRED",
            format!("{directive_name} angle-bracket stdlib target cannot be empty"),
        ));
    }

    let suffix = after_open[close_index + 1..].trim();
    let tag = if suffix.is_empty() {
        None
    } else if let Some(tag) = suffix.strip_prefix('!') {
        let tag = tag.trim();
        if tag.is_empty() || tag.contains(char::is_whitespace) {
            return Err(Diagnostic::error_code(
                "E_INCLUDE_INVALID_FORM",
                format!("{directive_name} has an invalid stdlib include tag: {raw_target}"),
            ));
        }
        Some(tag.to_string())
    } else {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_INVALID_FORM",
            format!(
                "{directive_name} stdlib target only supports optional `!TAG` after `>`: {raw_target}"
            ),
        ));
    };

    let mut path = PathBuf::from(inner);
    if path.extension().is_none() {
        path.set_extension("puml");
    }
    let path = crate::stdlib::apply_stdlib_path_alias(path);

    Ok(StdlibAngleIncludeTarget {
        path,
        display_name: inner.to_string(),
        tag,
    })
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub(super) fn process_import_directive(
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
                format!(
                    "!import URL includes are disabled (pass --allow-url-includes to enable): {}",
                    raw_target
                ),
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

pub(super) fn parse_preprocess_directive(line: &str) -> Option<PreprocessDirective> {
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

pub(super) fn evaluate_preprocess_expr(
    expr: &str,
    state: &PreprocState,
) -> Result<bool, Diagnostic> {
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

pub(super) fn evaluate_scalar_expr(expr: &str) -> Result<bool, Diagnostic> {
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
pub(super) fn eval_simple_arithmetic(expr: &str) -> Option<i64> {
    eval_int_expr(expr)
}

pub(super) fn eval_int_expr(expr: &str) -> Option<i64> {
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

pub(super) fn find_matching_endwhile(
    lines: &[&str],
    while_idx: usize,
) -> Result<usize, Diagnostic> {
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

pub(super) fn find_matching_endfor(
    lines: &[&str],
    foreach_idx: usize,
) -> Result<usize, Diagnostic> {
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

pub(super) fn consume_preprocessor_block(
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

pub(super) fn evaluate_assert_expression(
    body: &str,
    state: &PreprocState,
) -> Result<bool, Diagnostic> {
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
pub(super) fn resolve_include_path(
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
pub(super) fn parse_include_target(raw_target: &str) -> IncludeTarget {
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
pub(super) fn parse_import_target(raw_target: &str) -> Result<PathBuf, Diagnostic> {
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
    Ok(crate::stdlib::apply_stdlib_path_alias(path))
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(super) fn is_url_include_target(raw_target: &str) -> bool {
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
pub(super) fn extract_url(raw_target: &str) -> &str {
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
pub(super) fn fetch_url_include(url: &str) -> Result<String, Diagnostic> {
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
        let content = fetch_http_url_include(url)?;

        // Write to cache (best-effort; failures are non-fatal).
        if let Some(parent) = cache_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::write(&cache_path, &content);

        Ok(content)
    } else {
        // No cache path available; fetch directly without caching.
        fetch_http_url_include(url)
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
fn fetch_http_url_include(url: &str) -> Result<String, Diagnostic> {
    let response = ureq::builder()
        .redirects(0)
        .timeout_connect(URL_INCLUDE_TIMEOUT)
        .timeout_read(URL_INCLUDE_TIMEOUT)
        .timeout_write(URL_INCLUDE_TIMEOUT)
        .build()
        .get(url)
        .call()
        .map_err(|e| {
            Diagnostic::error_code(
                "E_INCLUDE_URL_FETCH",
                format!("failed to fetch '{}': {e}", url),
            )
        })?;

    if (300..400).contains(&response.status()) {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_URL_REDIRECT",
            format!(
                "redirects are not followed for URL include '{}': HTTP {} {}",
                url,
                response.status(),
                response.status_text()
            ),
        ));
    }

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

    read_limited_url_include_body(url, response)
}

#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
fn read_limited_url_include_body(
    url: &str,
    response: ureq::Response,
) -> Result<String, Diagnostic> {
    if let Some(length) = response
        .header("content-length")
        .and_then(|value| value.parse::<usize>().ok())
    {
        if length > URL_INCLUDE_MAX_BYTES {
            return Err(url_include_too_large(url, length));
        }
    }

    let mut bytes = Vec::new();
    let mut reader = response
        .into_reader()
        .take((URL_INCLUDE_MAX_BYTES + 1) as u64);
    reader.read_to_end(&mut bytes).map_err(|e| {
        Diagnostic::error_code(
            "E_INCLUDE_URL_FETCH",
            format!("failed to read response body from '{}': {e}", url),
        )
    })?;

    if bytes.len() > URL_INCLUDE_MAX_BYTES {
        return Err(url_include_too_large(url, bytes.len()));
    }

    String::from_utf8(bytes).map_err(|e| {
        Diagnostic::error_code(
            "E_INCLUDE_URL_FETCH",
            format!("failed to decode response body from '{}': {e}", url),
        )
    })
}

#[cfg(all(not(target_arch = "wasm32"), feature = "url-includes"))]
fn url_include_too_large(url: &str, bytes: usize) -> Diagnostic {
    Diagnostic::error_code(
        "E_INCLUDE_URL_TOO_LARGE",
        format!(
            "URL include '{}' is too large: {bytes} bytes exceeds the {URL_INCLUDE_MAX_BYTES} byte limit",
            url
        ),
    )
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "url-includes")))]
#[allow(dead_code)]
pub(super) fn fetch_url_include(url: &str) -> Result<String, Diagnostic> {
    Err(Diagnostic::error_code(
        "E_INCLUDE_URL_UNSUPPORTED",
        format!("URL includes are not supported in this build: {url}"),
    ))
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn resolve_stdlib_root(
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
/// such as `<C4/C4_Context>` or `<awslib/Compute/EC2>`.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(super) fn is_angle_bracket_include(raw_target: &str) -> bool {
    let trimmed = raw_target.trim();
    trimmed.starts_with('<')
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn resolve_import_path(
    stdlib_root: &Path,
    import_path: &Path,
) -> Result<PathBuf, Diagnostic> {
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
pub(super) fn extract_include_tag(content: &str, tag: &str) -> Option<String> {
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
pub(super) fn normalize_path(path: PathBuf) -> PathBuf {
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
