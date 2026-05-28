use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::diagnostic::Diagnostic;
use crate::source::MappedSpan;

use super::super::{ParseOptions, PreprocState};
use super::process_lines;

#[allow(clippy::too_many_arguments)]
pub(super) fn process_include_url(
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
    if !options.allow_url_includes {
        return Err(Diagnostic::error_code(
            "E_INCLUDE_URL_DISABLED",
            format!(
                "!includeurl URL includes are disabled (pass --allow-url-includes to enable): {raw_target}"
            ),
        ));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use super::super::includes::{extract_url, fetch_url_include};

        let url = extract_url(raw_target);
        let content = fetch_url_include(url)?;
        process_lines(
            &content,
            options,
            state,
            include_stack,
            include_once_seen,
            depth + 1,
            call_depth,
            out,
            mappings,
        )
    }
    #[cfg(target_arch = "wasm32")]
    {
        Err(Diagnostic::error_code(
            "E_INCLUDE_URL_UNSUPPORTED",
            format!("!includeurl URL targets are not supported in WASM: {raw_target}"),
        ))
    }
}
