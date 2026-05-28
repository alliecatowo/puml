use super::{parse_int_lenient, strip_quotes};

/// Dispatch string/character builtins that require multi-line logic.
///
/// Returns `Some(result)` when `name` is handled, `None` when the name is not
/// in this group (so the caller can fall through to other dispatchers).
///
/// `arg0`/`arg1`/`arg2` are the already-expanded arguments (empty string when
/// not supplied). `argc` is the number of arguments the caller actually saw.
pub(super) fn dispatch_string_builtin(
    name: &str,
    arg0: &str,
    arg1: &str,
    arg2: &str,
    argc: usize,
) -> Option<String> {
    match name {
        "strpos" => {
            let s = arg0;
            let sub = arg1;
            Some(match s.find(sub) {
                Some(byte_idx) => {
                    // Return char index (PlantUML semantics).
                    let char_idx = s[..byte_idx].chars().count();
                    char_idx.to_string()
                }
                None => "-1".to_string(),
            })
        }
        "substr" => {
            let s = arg0;
            let start = parse_int_lenient(arg1).max(0) as usize;
            let chars: Vec<char> = s.chars().collect();
            let start = start.min(chars.len());
            let end = if argc >= 3 {
                let len = parse_int_lenient(arg2);
                if len < 0 {
                    chars.len()
                } else {
                    (start + len as usize).min(chars.len())
                }
            } else {
                chars.len()
            };
            Some(chars[start..end].iter().collect())
        }
        "chr" => {
            let n = parse_int_lenient(arg0);
            if n < 0 {
                Some(String::new())
            } else if let Some(c) = u32::try_from(n).ok().and_then(char::from_u32) {
                Some(c.to_string())
            } else {
                Some(String::new())
            }
        }
        "dec2hex" => {
            let n = parse_int_lenient(arg0);
            if n < 0 {
                Some(String::new())
            } else {
                Some(format!("{:x}", n))
            }
        }
        "hex2dec" => {
            let s = arg0;
            let cleaned = s.trim().trim_start_matches("0x").trim_start_matches("0X");
            Some(
                i64::from_str_radix(cleaned, 16)
                    .map(|n| n.to_string())
                    .unwrap_or_else(|_| "0".to_string()),
            )
        }
        "ord" => Some(
            arg0.chars()
                .next()
                .map(|c| (c as u32).to_string())
                .unwrap_or_else(|| "0".to_string()),
        ),
        "equals" | "eq" | "strcmp" => Some((arg0 == arg1).to_string()),
        "equals_ignore_case" | "eq_ignore_case" | "strcmp_ignore_case" => {
            Some(arg0.eq_ignore_ascii_case(arg1).to_string())
        }
        "startswith" | "starts_with" => Some(arg0.starts_with(arg1).to_string()),
        "startswith_ignore_case" | "starts_with_ignore_case" => Some(
            arg0.to_ascii_lowercase()
                .starts_with(&arg1.to_ascii_lowercase())
                .to_string(),
        ),
        "endswith" | "ends_with" => Some(arg0.ends_with(arg1).to_string()),
        "endswith_ignore_case" | "ends_with_ignore_case" => Some(
            arg0.to_ascii_lowercase()
                .ends_with(&arg1.to_ascii_lowercase())
                .to_string(),
        ),
        "contains" => Some(arg0.contains(arg1).to_string()),
        "contains_ignore_case" => Some(
            arg0.to_ascii_lowercase()
                .contains(&arg1.to_ascii_lowercase())
                .to_string(),
        ),
        "quote" => Some(format!("\"{}\"", arg0.replace('"', "\\\""))),
        "unquote" => Some(strip_quotes(arg0)),
        _ => None,
    }
}
