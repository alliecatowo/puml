use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::diagnostic::Diagnostic;
use crate::preproc::{ParseOptions, PreprocState};
use crate::source::MappedSpan;

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
    _mappings: &mut Vec<MappedSpan>,
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
    _mappings: &mut Vec<MappedSpan>,
) -> Result<(), Diagnostic> {
    Err(include_not_supported_in_wasm("!include_many"))
}

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
pub(in crate::preproc) fn process_import_directive(
    _raw_target: &str,
    ctx: ImportDirectiveContext<'_>,
) -> Result<(), Diagnostic> {
    let ImportDirectiveContext {
        options,
        state,
        include_stack,
        include_once_seen,
        depth,
        call_depth,
        out,
        mappings,
    } = ctx;
    let _ = (
        options,
        state,
        include_stack,
        include_once_seen,
        depth,
        call_depth,
        out,
        mappings,
    );
    Err(include_not_supported_in_wasm("!import"))
}
