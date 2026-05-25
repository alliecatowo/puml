use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use crate::diagnostic::Diagnostic;
use crate::preproc::control::preprocess_text;
use crate::preproc::{ParseOptions, PreprocState};

use super::diagnostics::stack_cycle;
use super::paths::{
    import_escape_diagnostic, import_path_escapes_root, parent_component_import_escape_diagnostic,
    path_contains_parent_component, resolve_import_path,
};

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(in crate::preproc) fn is_stdlib_catalog_target(raw_target: &str) -> bool {
    let trimmed = raw_target.trim();
    trimmed.starts_with('<') && trimmed.ends_with('>')
}

/// Handle `!include <Library/Module>` by resolving the path through the stdlib root.
/// The angle-bracket form is a stdlib reference; it is always treated as include-once.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub(in crate::preproc) fn process_stdlib_angle_include(
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
    if path_contains_parent_component(&path) {
        return Err(parent_component_import_escape_diagnostic(&path));
    }

    if let Some(builtin) = crate::stdlib::resolve_builtin_stdlib_include(&path) {
        return process_builtin_stdlib_include(
            builtin,
            target.tag.as_deref(),
            directive_name,
            options,
            state,
            include_stack,
            include_once_seen,
            depth,
            call_depth,
            out,
        );
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
    if import_path_escapes_root(&stdlib_root, &path) {
        return Err(import_escape_diagnostic(&stdlib_root, &path));
    }

    let resolved = if stdlib_root.join(&path).exists() {
        resolve_import_path(&stdlib_root, &path)?
    } else {
        return Err(stdlib_not_found_diagnostic(
            &stdlib_root,
            target.requested_pack.as_deref(),
            inner,
            &path,
        ));
    };

    // Angle-bracket includes are always treated as include-once (stdlib files are idempotent).
    if !include_once_seen.insert(resolved.clone()) {
        return Ok(());
    }

    if include_stack.iter().any(|p| p == &resolved) {
        return Err(stack_cycle(
            "E_INCLUDE_CYCLE",
            "include",
            include_stack,
            &resolved,
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

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub(in crate::preproc) fn process_builtin_stdlib_include(
    builtin: crate::stdlib::StdlibBuiltinInclude,
    tag: Option<&str>,
    _directive_name: &str,
    options: &ParseOptions,
    state: &mut PreprocState,
    include_stack: &mut Vec<PathBuf>,
    include_once_seen: &mut BTreeSet<PathBuf>,
    depth: usize,
    call_depth: usize,
    out: &mut String,
) -> Result<(), Diagnostic> {
    if let Some(tag) = tag {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_TAG_NOT_FOUND",
            format!(
                "include tag '{tag}' was not found in built-in stdlib include '<{}>'",
                builtin.logical_path
            ),
        ));
    }

    let include_key = PathBuf::from(format!("builtin-stdlib://{}", builtin.logical_path));
    if !include_once_seen.insert(include_key.clone()) {
        return Ok(());
    }
    if include_stack.iter().any(|path| path == &include_key) {
        return Err(stack_cycle(
            "E_INCLUDE_CYCLE",
            "include",
            include_stack,
            &include_key,
        ));
    }

    include_stack.push(include_key);
    preprocess_text(
        &builtin.content,
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
    requested_pack: Option<String>,
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

    let requested_pack = crate::stdlib::stdlib_path_pack(inner).map(str::to_string);
    let mut path = PathBuf::from(inner);
    if path.extension().is_none() {
        path.set_extension("puml");
    }
    let path = crate::stdlib::apply_stdlib_path_alias(path);

    Ok(StdlibAngleIncludeTarget {
        path,
        display_name: inner.to_string(),
        requested_pack,
        tag,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::preproc) fn stdlib_not_found_diagnostic(
    stdlib_root: &std::path::Path,
    requested_pack: Option<&str>,
    display_name: &str,
    resolved_path: &std::path::Path,
) -> Diagnostic {
    let entries = crate::stdlib::inventory_from_root(stdlib_root).unwrap_or_default();
    let available = crate::stdlib::available_stdlib_packs(&entries).join(", ");
    let unavailable = crate::stdlib::sorted_missing_stdlib_packs().join(", ");
    let pack = requested_pack
        .or_else(|| {
            resolved_path
                .to_str()
                .and_then(crate::stdlib::stdlib_path_pack)
        })
        .unwrap_or(display_name);

    if crate::stdlib::is_known_missing_stdlib_pack(pack) {
        return Diagnostic::error_code(
            "E_INCLUDE_STDLIB_PACK_UNAVAILABLE",
            format!(
                "stdlib pack '{pack}' is not bundled; available packs: {available}; known unavailable upstream packs: {unavailable}"
            ),
        );
    }

    Diagnostic::error_code(
        "E_INCLUDE_STDLIB_NOT_FOUND",
        format!(
            "stdlib include '<{display_name}>' was not found as '{}'; available packs: {available}; known unavailable upstream packs: {unavailable}",
            resolved_path.display()
        ),
    )
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::preproc) fn resolve_stdlib_root(
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
pub(in crate::preproc) fn is_angle_bracket_include(raw_target: &str) -> bool {
    let trimmed = raw_target.trim();
    trimmed.starts_with('<')
}
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(in crate::preproc) fn extract_include_tag(content: &str, tag: &str) -> Option<String> {
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
