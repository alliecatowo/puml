use crate::diagnostic::Diagnostic;

use super::super::builtins::parse_callable_definition;
use super::super::includes::consume_preprocessor_block;
use super::super::{PreprocCallableKind, PreprocState, PreprocessDirective};

pub(super) fn define_callable_block(
    lines: &[&str],
    start_idx: usize,
    kind: PreprocCallableKind,
    state: &mut PreprocState,
) -> Result<usize, Diagnostic> {
    let (start, end, code, terminator) = match kind {
        PreprocCallableKind::Function => (
            PreprocessDirective::Function,
            PreprocessDirective::EndFunction,
            "E_FUNCTION_UNCLOSED",
            "!endfunction",
        ),
        PreprocCallableKind::Procedure => (
            PreprocessDirective::Procedure,
            PreprocessDirective::EndProcedure,
            "E_PROCEDURE_UNCLOSED",
            "!endprocedure",
        ),
    };
    let end_idx = consume_preprocessor_block(lines, start_idx, start, end, code, terminator)?;
    let header = lines[start_idx].trim();
    let callable = parse_callable_definition(header, &lines[start_idx + 1..end_idx], kind)?;
    state.callables.insert(callable.0, callable.1);
    Ok(end_idx)
}
