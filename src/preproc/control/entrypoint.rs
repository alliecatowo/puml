use std::collections::BTreeSet;

use crate::diagnostic::Diagnostic;
use crate::source::SourceMap;

use super::super::{ParseOptions, PreprocState, PreprocessResult};
use super::flow::strip_block_comments;

pub(crate) fn preprocess(source: &str, options: &ParseOptions) -> Result<String, Diagnostic> {
    preprocess_with_map(source, options).map(|result| result.source)
}

pub(crate) fn preprocess_with_map(
    source: &str,
    options: &ParseOptions,
) -> Result<PreprocessResult, Diagnostic> {
    let mut state = PreprocState::default();
    state.vars.extend(options.inject_vars.clone());
    let mut include_stack = Vec::new();
    let mut include_once_seen = BTreeSet::new();
    let mut expanded = String::new();
    let mut mappings = Vec::new();

    // Strip PlantUML block comments (`/' ... '/`) before line-by-line processing.
    // Block comments can span multiple lines; we replace consumed content with
    // spaces/newlines to preserve line numbers for span/diagnostic accuracy.
    let stripped = strip_block_comments(source);

    super::process_lines(
        &stripped,
        options,
        &mut state,
        &mut include_stack,
        &mut include_once_seen,
        0,
        0,
        &mut expanded,
        &mut mappings,
    )?;

    Ok(PreprocessResult {
        source: expanded,
        source_map: SourceMap::new(mappings),
    })
}
