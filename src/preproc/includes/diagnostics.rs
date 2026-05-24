use std::path::PathBuf;

use crate::diagnostic::Diagnostic;

pub(in crate::preproc) fn include_path_required(directive_name: &str) -> Diagnostic {
    Diagnostic::error_code(
        "E_INCLUDE_PATH_REQUIRED",
        format!("{directive_name} requires a relative path"),
    )
}

pub(in crate::preproc) fn url_includes_disabled(
    directive_name: &str,
    raw_target: &str,
) -> Diagnostic {
    Diagnostic::error_code(
        "E_INCLUDE_URL_DISABLED",
        format!(
            "{directive_name} URL includes are disabled (pass --allow-url-includes to enable): {raw_target}"
        ),
    )
}

pub(in crate::preproc) fn stack_cycle(
    code: &str,
    label: &str,
    include_stack: &[PathBuf],
    resolved: &std::path::Path,
) -> Diagnostic {
    let mut cycle = include_stack
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>();
    cycle.push(resolved.display().to_string());
    Diagnostic::error_code(
        code,
        format!("{label} cycle detected: {}", cycle.join(" -> ")),
    )
}
