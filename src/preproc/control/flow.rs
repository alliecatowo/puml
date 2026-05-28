use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::diagnostic::Diagnostic;
use crate::source::MappedSpan;

use super::super::builtins::preprocessor_foreach_bindings;
use super::super::includes::{
    evaluate_preprocess_expr, find_matching_endfor, find_matching_endwhile,
};
use super::super::{
    ConditionalFrame, ParseOptions, PreprocLoopSignal, PreprocState, MAX_INCLUDE_DEPTH,
    MAX_PREPROC_WHILE_ITERATIONS,
};
use super::process_lines;

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
    mappings: &mut Vec<MappedSpan>,
) -> Result<Option<PreprocLoopSignal>, Diagnostic> {
    state.loop_depth += 1;
    let result = process_lines(
        source,
        options,
        state,
        include_stack,
        include_once_seen,
        depth,
        call_depth,
        out,
        mappings,
    );
    state.loop_depth = state.loop_depth.saturating_sub(1);
    result?;
    Ok(state.loop_signal.take())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn preprocess_while_directive(
    expr: &str,
    lines: &[&str],
    while_idx: usize,
    active: bool,
    options: &ParseOptions,
    state: &mut PreprocState,
    include_stack: &mut Vec<PathBuf>,
    include_once_seen: &mut BTreeSet<PathBuf>,
    depth: usize,
    call_depth: usize,
    out: &mut String,
    mappings: &mut Vec<MappedSpan>,
) -> Result<usize, Diagnostic> {
    let endwhile = find_matching_endwhile(lines, while_idx)?;
    if active {
        let block = lines[while_idx + 1..endwhile].join("\n");
        let mut iterations = 0usize;
        while evaluate_preprocess_expr(expr, state)? {
            iterations += 1;
            if iterations > MAX_PREPROC_WHILE_ITERATIONS {
                return Err(Diagnostic::error_code(
                    "E_PREPROC_WHILE_LIMIT",
                    format!("`!while` iteration limit exceeded ({MAX_PREPROC_WHILE_ITERATIONS})"),
                ));
            }
            let signal = preprocess_loop_block(
                &block,
                options,
                state,
                include_stack,
                include_once_seen,
                depth,
                call_depth,
                out,
                mappings,
            )?;
            match signal {
                Some(PreprocLoopSignal::Break) => break,
                Some(PreprocLoopSignal::Continue) | None => {}
            }
        }
    }
    Ok(endwhile + 1)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn preprocess_foreach_directive(
    spec: &str,
    lines: &[&str],
    foreach_idx: usize,
    active: bool,
    options: &ParseOptions,
    state: &mut PreprocState,
    include_stack: &mut Vec<PathBuf>,
    include_once_seen: &mut BTreeSet<PathBuf>,
    depth: usize,
    call_depth: usize,
    out: &mut String,
    mappings: &mut Vec<MappedSpan>,
) -> Result<usize, Diagnostic> {
    let endfor = find_matching_endfor(lines, foreach_idx)?;
    if !active {
        return Ok(endfor + 1);
    }

    let parts: Vec<&str> = spec.splitn(2, " in ").collect();
    if parts.len() != 2 {
        return Err(Diagnostic::error_code(
            "E_PREPROC_FOREACH_FORM",
            "`!foreach` requires form `$var in val1, val2, ...`",
        ));
    }
    let var_names = parts[0]
        .split(',')
        .map(|name| name.trim().trim_start_matches('$').to_string())
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    if var_names.is_empty() {
        return Err(Diagnostic::error_code(
            "E_PREPROC_FOREACH_FORM",
            "`!foreach` requires at least one loop variable",
        ));
    }
    let rhs = crate::preproc::macros::expand_preprocessor_text(parts[1].trim(), state, 0)?;
    let bindings = preprocessor_foreach_bindings(&var_names, &rhs);
    let block = lines[foreach_idx + 1..endfor].join("\n");
    let prev = var_names
        .iter()
        .map(|name| (name.clone(), state.vars.get(name).cloned()))
        .collect::<Vec<_>>();
    let mut should_break = false;
    for row in bindings {
        for (name, value) in row {
            state.vars.insert(name, value);
        }
        let signal = preprocess_loop_block(
            &block,
            options,
            state,
            include_stack,
            include_once_seen,
            depth,
            call_depth,
            out,
            mappings,
        )?;
        match signal {
            Some(PreprocLoopSignal::Break) => {
                should_break = true;
            }
            Some(PreprocLoopSignal::Continue) | None => {}
        }
        if should_break {
            break;
        }
    }
    for (name, value) in prev {
        match value {
            Some(v) => {
                state.vars.insert(name, v);
            }
            None => {
                state.vars.remove(&name);
            }
        }
    }
    Ok(endfor + 1)
}
