#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::path::PathBuf;

use crate::diagnostic::Diagnostic;

use crate::preproc::{IncludeTarget, ParseOptions};

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

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(in crate::preproc) fn parse_include_target(raw_target: &str) -> IncludeTarget {
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
pub(in crate::preproc) fn parse_import_target(raw_target: &str) -> Result<PathBuf, Diagnostic> {
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
pub(in crate::preproc) fn is_url_include_target(raw_target: &str) -> bool {
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
pub(in crate::preproc) fn extract_url(raw_target: &str) -> &str {
    raw_target
        .trim()
        .trim_matches('"')
        .trim_start_matches('<')
        .trim_end_matches('>')
        .trim()
}

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
pub(in crate::preproc) fn resolve_stdlib_root_for_angle_include(
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
pub(in crate::preproc) fn is_angle_bracket_include(raw_target: &str) -> bool {
    let trimmed = raw_target.trim();
    trimmed.starts_with('<')
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
