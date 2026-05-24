use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::diagnostic::Diagnostic;
use crate::preproc::{ParseOptions, PreprocState};

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
