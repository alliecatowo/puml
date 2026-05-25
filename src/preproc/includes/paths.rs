use std::path::{Path, PathBuf};

use crate::diagnostic::Diagnostic;
use crate::preproc::ParseOptions;

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::preproc) fn resolve_include_path(
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
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::preproc) fn resolve_import_path(
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

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::preproc) fn import_path_escapes_root(stdlib_root: &Path, import_path: &Path) -> bool {
    let resolved = normalize_path(stdlib_root.join(import_path));
    !resolved.starts_with(stdlib_root)
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(in crate::preproc) fn path_contains_parent_component(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::preproc) fn import_escape_diagnostic(
    stdlib_root: &Path,
    import_path: &Path,
) -> Diagnostic {
    Diagnostic::error_code(
        "E_IMPORT_ESCAPE",
        format!(
            "import path escapes stdlib root: '{}' resolves outside '{}'",
            import_path.display(),
            stdlib_root.display()
        ),
    )
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(in crate::preproc) fn parent_component_import_escape_diagnostic(
    import_path: &Path,
) -> Diagnostic {
    Diagnostic::error_code(
        "E_IMPORT_ESCAPE",
        format!(
            "stdlib import path contains parent traversal: '{}'",
            import_path.display()
        ),
    )
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(in crate::preproc) fn normalize_path(path: PathBuf) -> PathBuf {
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
