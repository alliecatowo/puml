use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostic::Diagnostic;
use crate::preproc::control::preprocess_text;
use crate::preproc::{ParseOptions, PreprocState};
use crate::source::MappedSpan;

use super::diagnostics::{include_path_required, stack_cycle, url_includes_disabled};
use super::paths::{
    import_escape_diagnostic, import_path_escapes_root, normalize_path,
    parent_component_import_escape_diagnostic, path_contains_parent_component, resolve_import_path,
    resolve_include_path,
};
use super::stdlib::{
    extract_include_tag, is_angle_bracket_include, is_stdlib_catalog_target,
    process_builtin_stdlib_include, process_stdlib_angle_include, resolve_stdlib_root,
    stdlib_not_found_diagnostic,
};
use super::target::{glob_matches, parse_import_target, parse_include_target};
use super::url::{extract_url, fetch_url_include, is_url_include_target};

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
    mappings: &mut Vec<MappedSpan>,
) -> Result<(), Diagnostic> {
    if raw_target.is_empty() {
        return Err(include_path_required(directive_name));
    }

    if is_url_include_target(raw_target) {
        if !options.allow_url_includes {
            return Err(url_includes_disabled(directive_name, raw_target));
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
            mappings,
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
            mappings,
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
        mappings,
    )?;
    include_stack.pop();
    Ok(())
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
    mappings: &mut Vec<MappedSpan>,
) -> Result<(), Diagnostic> {
    if raw_target.is_empty() {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_PATH_REQUIRED",
            "!include_many requires a relative path",
        ));
    }
    if is_url_include_target(raw_target) {
        if !options.allow_url_includes {
            return Err(url_includes_disabled("!include_many", raw_target));
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
            mappings,
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
            mappings,
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
            mappings,
        )?;
        include_stack.pop();
    }
    Ok(())
}
pub(in crate::preproc) struct ImportDirectiveContext<'a> {
    pub(in crate::preproc) options: &'a ParseOptions,
    pub(in crate::preproc) state: &'a mut PreprocState,
    pub(in crate::preproc) include_stack: &'a mut Vec<PathBuf>,
    pub(in crate::preproc) include_once_seen: &'a mut BTreeSet<PathBuf>,
    pub(in crate::preproc) depth: usize,
    pub(in crate::preproc) call_depth: usize,
    pub(in crate::preproc) out: &'a mut String,
    pub(in crate::preproc) mappings: &'a mut Vec<MappedSpan>,
}

pub(in crate::preproc) fn process_import_directive(
    raw_target: &str,
    ctx: ImportDirectiveContext<'_>,
) -> Result<(), Diagnostic> {
    if raw_target.trim().is_empty() {
        return Err(Diagnostic::error_code(
            "E_IMPORT_PATH_REQUIRED",
            "!import requires a stdlib module path",
        ));
    }
    if is_url_include_target(raw_target) {
        if !ctx.options.allow_url_includes {
            return Err(url_includes_disabled("!import", raw_target));
        }
        let url = extract_url(raw_target);
        let content = fetch_url_include(url)?;
        return preprocess_text(
            &content,
            ctx.options,
            ctx.state,
            ctx.include_stack,
            ctx.include_once_seen,
            ctx.depth + 1,
            ctx.call_depth,
            ctx.out,
            ctx.mappings,
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
    if path_contains_parent_component(&target) {
        return Err(parent_component_import_escape_diagnostic(&target));
    }

    if let Some(builtin) = crate::stdlib::resolve_builtin_stdlib_include(&target) {
        return process_builtin_stdlib_include(
            builtin,
            None,
            "!import",
            ctx.options,
            ctx.state,
            ctx.include_stack,
            ctx.include_once_seen,
            ctx.depth,
            ctx.call_depth,
            ctx.out,
            ctx.mappings,
        );
    }

    let stdlib_root = resolve_stdlib_root(ctx.options, ctx.include_stack)?;
    if import_path_escapes_root(&stdlib_root, &target) {
        return Err(import_escape_diagnostic(&stdlib_root, &target));
    }
    if !stdlib_root.join(&target).exists() {
        let requested_pack = target.to_str().and_then(crate::stdlib::stdlib_path_pack);
        if requested_pack.is_some_and(crate::stdlib::is_known_missing_stdlib_pack) {
            return Err(stdlib_not_found_diagnostic(
                &stdlib_root,
                requested_pack,
                &target.display().to_string(),
                &target,
            ));
        }
        return Err(Diagnostic::error_code(
            "E_IMPORT_STDLIB_NOT_FOUND",
            format!("stdlib import not found '{}'", target.display()),
        ));
    }
    let resolved = resolve_import_path(&stdlib_root, &target)?;
    if !ctx.include_once_seen.insert(resolved.clone()) {
        return Ok(());
    }
    if ctx.include_stack.iter().any(|p| p == &resolved) {
        return Err(stack_cycle(
            "E_IMPORT_CYCLE",
            "import",
            ctx.include_stack,
            &resolved,
        ));
    }

    let content = fs::read_to_string(&resolved).map_err(|e| {
        Diagnostic::error_code(
            "E_IMPORT_READ",
            format!("failed to read import '{}': {e}", resolved.display()),
        )
    })?;
    ctx.include_stack.push(resolved);
    preprocess_text(
        &content,
        ctx.options,
        ctx.state,
        ctx.include_stack,
        ctx.include_once_seen,
        ctx.depth + 1,
        ctx.call_depth,
        ctx.out,
        ctx.mappings,
    )?;
    ctx.include_stack.pop();
    Ok(())
}
