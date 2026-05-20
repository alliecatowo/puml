use std::collections::BTreeSet;
#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(target_arch = "wasm32")]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

use crate::diagnostic::Diagnostic;

#[cfg(not(target_arch = "wasm32"))]
use crate::preproc::control::preprocess_text;
use crate::preproc::{ParseOptions, PreprocState};

#[cfg(not(target_arch = "wasm32"))]
use super::paths::{
    extract_include_tag, extract_url, is_angle_bracket_include, is_url_include_target,
    normalize_path, parse_import_target, parse_include_target, resolve_import_path,
    resolve_include_path, resolve_stdlib_root, resolve_stdlib_root_for_angle_include,
};
#[cfg(not(target_arch = "wasm32"))]
use super::url::fetch_url_include;

/// On `wasm32` there is no filesystem available, so the entire `!include` /
/// `!includesub` / `!include_many` / `!import` family returns a friendly error
/// rather than attempting to read files. All FS-touching code below is gated
/// with `cfg(not(target_arch = "wasm32"))`; these stubs satisfy the call sites.
#[cfg(target_arch = "wasm32")]
pub(in crate::preproc) fn include_not_supported_in_wasm(directive_name: &str) -> Diagnostic {
    Diagnostic::error_code(
        "E_INCLUDE_NOT_SUPPORTED_WASM",
        format!(
            "{directive_name} is not available in the in-browser renderer — the WASM build has no filesystem"
        ),
    )
}

#[cfg(target_arch = "wasm32")]
#[allow(clippy::too_many_arguments)]
pub(in crate::preproc) fn process_include_directive(
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
pub(in crate::preproc) fn process_include_many_directive(
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
pub(in crate::preproc) fn process_import_directive(
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
pub(in crate::preproc) fn process_include_directive(
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
pub(in crate::preproc) fn is_stdlib_catalog_target(raw_target: &str) -> bool {
    let trimmed = raw_target.trim();
    trimmed.starts_with('<') && trimmed.ends_with('>')
}

/// `!include_many` with optional glob expansion. When the path contains `*`
/// or `?`, expand it to every matching file in deterministic alphabetical
/// order; otherwise behave like `!include`. Globs only match the file-name
/// segment of the path so we cannot escape the include root by accident.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub(in crate::preproc) fn process_include_many_directive(
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

    Ok(StdlibAngleIncludeTarget {
        path,
        display_name: inner.to_string(),
        tag,
    })
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub(in crate::preproc) fn process_import_directive(
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
