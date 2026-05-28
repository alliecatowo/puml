use super::super::builtins::extract_parenthesized_args;
use super::super::PreprocMacro;

/// Parse a `!definelong NAME(params)` header plus collected body lines into a
/// [`PreprocMacro`] whose body contains the lines joined with `\n`.
///
/// The `header` argument is everything after `!definelong ` on the opening
/// line (e.g. `"BORDER(entity, color)"`). The `body_lines` slice holds every
/// source line between the header and the `!enddefinelong` terminator.
pub fn parse_macro_definelong(
    header: &str,
    body_lines: &[&str],
) -> Result<(String, PreprocMacro), crate::diagnostic::Diagnostic> {
    let trimmed = header.trim();
    // A `!definelong` with no parameter list defines a no-arg macro.
    let (name_raw, params) = if let Some(open) = trimmed.find('(') {
        let name = trimmed[..open].trim();
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(crate::diagnostic::Diagnostic::error_code(
                "E_DEFINELONG_SYNTAX",
                format!("invalid `!definelong` macro name: `{name}`"),
            ));
        }
        let chars = trimmed.chars().collect::<Vec<_>>();
        let (params_raw, _) = extract_parenthesized_args(&chars, open)?;
        let params = super::super::builtins::parse_params(&params_raw)?;
        (name, params)
    } else {
        // No parentheses — no-argument macro.
        let name = trimmed;
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(crate::diagnostic::Diagnostic::error_code(
                "E_DEFINELONG_SYNTAX",
                format!("invalid `!definelong` macro name: `{name}`"),
            ));
        }
        (name, Vec::new())
    };
    // Join body lines preserving newlines. PlantUML trims no content from the
    // body; each line is emitted verbatim with a newline terminator.
    let body = body_lines.join("\n");
    Ok((name_raw.to_string(), PreprocMacro { params, body }))
}
