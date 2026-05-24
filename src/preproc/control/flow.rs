use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::diagnostic::Diagnostic;

use super::super::{
    ConditionalFrame, ParseOptions, PreprocLoopSignal, PreprocState, MAX_INCLUDE_DEPTH,
};
use super::preprocess_text;

pub(super) fn check_include_depth(depth: usize) -> Result<(), Diagnostic> {
    if depth > MAX_INCLUDE_DEPTH {
        return Err(Diagnostic::error(format!(
            "include depth exceeded maximum of {MAX_INCLUDE_DEPTH}"
        )));
    }
    Ok(())
}

/// Strip PlantUML block comments of the form `/' ... '/`.
/// Comments may span multiple lines. Each consumed line is replaced with an
/// empty line so that span/line-number information stays correct for
/// error reporting.
pub(super) fn strip_block_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Look for block comment open: `/'`
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'\'' {
            // Consume `/'`, then scan forward for matching `'/`
            i += 2;
            loop {
                if i >= len {
                    // Unterminated block comment — consume the rest; normalizer
                    // will handle any resulting empty input gracefully.
                    break;
                }
                if i + 1 < len && bytes[i] == b'\'' && bytes[i + 1] == b'/' {
                    // Consume closing `'/`
                    i += 2;
                    break;
                }
                // Keep newlines so that line numbers stay accurate; drop
                // all other content inside the block comment.
                if bytes[i] == b'\n' {
                    out.push('\n');
                }
                i += 1;
            }
        } else {
            // Regular character — emit as-is.
            // Advance by full char width to avoid splitting multi-byte sequences.
            let ch = source[i..].chars().next().unwrap_or('\0');
            out.push(ch);
            i += ch.len_utf8();
        }
    }

    out
}

pub(super) fn is_active(conditionals: &[ConditionalFrame]) -> bool {
    conditionals.iter().all(|f| f.current_active)
}

pub(super) fn ensure_conditionals_closed(
    conditionals: &[ConditionalFrame],
) -> Result<(), Diagnostic> {
    if conditionals.is_empty() {
        Ok(())
    } else {
        Err(Diagnostic::error_code(
            "E_PREPROC_COND_UNCLOSED",
            "missing `!endif` for conditional block",
        ))
    }
}

pub(super) fn looks_like_iso_date_value(value: &str) -> bool {
    let date = value.split_whitespace().next().unwrap_or(value).trim();
    let mut parts = date.split('-');
    let (Some(year), Some(month), Some(day)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    parts.next().is_none()
        && year.len() == 4
        && month.len() == 2
        && day.len() == 2
        && year.chars().all(|ch| ch.is_ascii_digit())
        && month.chars().all(|ch| ch.is_ascii_digit())
        && day.chars().all(|ch| ch.is_ascii_digit())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn preprocess_loop_block(
    source: &str,
    options: &ParseOptions,
    state: &mut PreprocState,
    include_stack: &mut Vec<PathBuf>,
    include_once_seen: &mut BTreeSet<PathBuf>,
    depth: usize,
    call_depth: usize,
    out: &mut String,
) -> Result<Option<PreprocLoopSignal>, Diagnostic> {
    state.loop_depth += 1;
    let result = preprocess_text(
        source,
        options,
        state,
        include_stack,
        include_once_seen,
        depth,
        call_depth,
        out,
    );
    state.loop_depth = state.loop_depth.saturating_sub(1);
    result?;
    Ok(state.loop_signal.take())
}
