use crate::diagnostic::Diagnostic;
use crate::preproc::{PreprocState, PreprocVariableScope, PreprocessDirective};

use super::expr::evaluate_preprocess_expr;
use crate::preproc::macros::{
    parse_named_call, parse_scoped_variable_assignment, parse_variable_assignment,
};

pub(in crate::preproc) fn parse_preprocess_directive(line: &str) -> Option<PreprocessDirective> {
    let trimmed = line.trim();
    if trimmed
        .to_ascii_lowercase()
        .starts_with("%invoke_procedure(")
        || trimmed.to_ascii_lowercase().starts_with("%call_user_func(")
    {
        return Some(PreprocessDirective::DynamicInvocation(trimmed.to_string()));
    }
    if !trimmed.starts_with('!') {
        return None;
    }
    let rest = trimmed[1..].trim_start();
    let mut split = rest.splitn(2, char::is_whitespace);
    let name = split.next().unwrap_or_default();
    let arg = split.next().unwrap_or_default().trim();
    let lower = name.to_ascii_lowercase();

    match lower.as_str() {
        "define" => Some(PreprocessDirective::Define(arg.to_string())),
        "undef" => Some(PreprocessDirective::Undef(arg.to_string())),
        "include" => Some(PreprocessDirective::Include(arg.to_string())),
        "include_once" => Some(PreprocessDirective::IncludeOnce(arg.to_string())),
        "include_many" => Some(PreprocessDirective::IncludeMany(arg.to_string())),
        "includesub" => Some(PreprocessDirective::IncludeSub(arg.to_string())),
        "includeurl" => Some(PreprocessDirective::IncludeUrl(arg.to_string())),
        "import" => Some(PreprocessDirective::Import(arg.to_string())),
        "if" => Some(PreprocessDirective::If(arg.to_string())),
        "ifdef" => Some(PreprocessDirective::IfDef {
            name: arg.to_string(),
            negated: false,
        }),
        "ifndef" => Some(PreprocessDirective::IfDef {
            name: arg.to_string(),
            negated: true,
        }),
        "elseif" => Some(PreprocessDirective::ElseIf(arg.to_string())),
        "else" => Some(PreprocessDirective::Else),
        "endif" => Some(PreprocessDirective::EndIf),
        "while" => Some(PreprocessDirective::While(arg.to_string())),
        "foreach" => Some(PreprocessDirective::Foreach(arg.to_string())),
        "endfor" => Some(PreprocessDirective::EndFor),
        "endwhile" => Some(PreprocessDirective::EndWhile),
        "break" => Some(PreprocessDirective::Break),
        "continue" => Some(PreprocessDirective::Continue),
        "function" => Some(PreprocessDirective::Function),
        "endfunction" => Some(PreprocessDirective::EndFunction),
        "procedure" => Some(PreprocessDirective::Procedure),
        "endprocedure" => Some(PreprocessDirective::EndProcedure),
        "assert" => Some(PreprocessDirective::Assert(arg.to_string())),
        "log" => Some(PreprocessDirective::Log(arg.to_string())),
        "dump_memory" => Some(PreprocessDirective::DumpMemory(arg.to_string())),
        "option" => Some(PreprocessDirective::Passthrough(trimmed.to_string())),
        "local" => parse_scoped_variable_assignment(arg, trimmed, PreprocVariableScope::Local),
        "global" => parse_scoped_variable_assignment(arg, trimmed, PreprocVariableScope::Global),
        _ if let Some((call_name, call_args)) = parse_named_call(rest) => {
            Some(PreprocessDirective::ProcedureCall {
                name: call_name,
                args: call_args,
            })
        }
        _ if name.starts_with('$') => parse_variable_assignment(name, arg, trimmed),
        "return" => Some(PreprocessDirective::Unsupported(name.to_string())),
        // `!startsub` / `!endsub` are markers used by `!includesub`. When a
        // file containing them is included directly, we silently elide the
        // marker lines and pass the body lines through.
        "startsub" | "endsub" => Some(PreprocessDirective::NoOp),
        "theme" | "pragma" => None,
        _ if !name.is_empty() => Some(PreprocessDirective::Unsupported(name.to_string())),
        _ => None,
    }
}

pub(in crate::preproc) fn find_matching_endwhile(
    lines: &[&str],
    while_idx: usize,
) -> Result<usize, Diagnostic> {
    let mut depth = 0usize;
    for (idx, raw) in lines.iter().enumerate().skip(while_idx + 1) {
        let line = raw.trim();
        match parse_preprocess_directive(line) {
            Some(PreprocessDirective::While(_)) => depth += 1,
            Some(PreprocessDirective::EndWhile) => {
                if depth == 0 {
                    return Ok(idx);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    Err(Diagnostic::error_code(
        "E_PREPROC_WHILE_UNCLOSED",
        "missing `!endwhile` for `!while` block",
    ))
}

pub(in crate::preproc) fn find_matching_endfor(
    lines: &[&str],
    foreach_idx: usize,
) -> Result<usize, Diagnostic> {
    let mut depth = 0usize;
    for (idx, raw) in lines.iter().enumerate().skip(foreach_idx + 1) {
        let line = raw.trim();
        match parse_preprocess_directive(line) {
            Some(PreprocessDirective::Foreach(_)) => depth += 1,
            Some(PreprocessDirective::EndFor) => {
                if depth == 0 {
                    return Ok(idx);
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    Err(Diagnostic::error_code(
        "E_PREPROC_FOREACH_UNCLOSED",
        "missing `!endfor` for `!foreach` block",
    ))
}

pub(in crate::preproc) fn consume_preprocessor_block(
    lines: &[&str],
    start_idx: usize,
    start_directive: PreprocessDirective,
    end_directive: PreprocessDirective,
    error_code: &str,
    end_name: &str,
) -> Result<usize, Diagnostic> {
    let mut depth = 0usize;
    for (idx, raw) in lines.iter().enumerate().skip(start_idx + 1) {
        let line = raw.trim();
        if let Some(directive) = parse_preprocess_directive(line) {
            if std::mem::discriminant(&directive) == std::mem::discriminant(&start_directive) {
                depth += 1;
            } else if std::mem::discriminant(&directive) == std::mem::discriminant(&end_directive) {
                if depth == 0 {
                    return Ok(idx);
                }
                depth -= 1;
            }
        }
    }
    Err(Diagnostic::error_code(
        error_code,
        format!("missing `{end_name}` for preprocessor block"),
    ))
}

pub(in crate::preproc) fn evaluate_assert_expression(
    body: &str,
    state: &PreprocState,
) -> Result<bool, Diagnostic> {
    let expression = body.split_once(':').map_or(body, |(expr, _)| expr).trim();
    if expression.is_empty() {
        return Err(Diagnostic::error_code(
            "E_PREPROC_ASSERT_EXPR_REQUIRED",
            "!assert requires a non-empty expression before optional `:` message",
        ));
    }
    evaluate_preprocess_expr(expression, state)
}
