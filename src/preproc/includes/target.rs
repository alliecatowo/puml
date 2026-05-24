use std::path::PathBuf;

use crate::diagnostic::Diagnostic;
use crate::preproc::IncludeTarget;

/// Minimal `*`/`?` glob match — sufficient for `!include_many` filename
/// patterns. Backtracks on `*` to keep behaviour predictable. No character
/// classes, no recursion across path separators.
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(in crate::preproc) fn glob_matches(pattern: &str, name: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    fn rec(p: &[char], n: &[char]) -> bool {
        let mut pi = 0;
        let mut ni = 0;
        let mut star: Option<(usize, usize)> = None;
        while ni < n.len() {
            if pi < p.len() && (p[pi] == '?' || p[pi] == n[ni]) {
                pi += 1;
                ni += 1;
            } else if pi < p.len() && p[pi] == '*' {
                star = Some((pi, ni));
                pi += 1;
            } else if let Some((sp, sn)) = star {
                pi = sp + 1;
                ni = sn + 1;
                star = Some((sp, sn + 1));
            } else {
                return false;
            }
        }
        while pi < p.len() && p[pi] == '*' {
            pi += 1;
        }
        pi == p.len()
    }
    rec(&p, &n)
}
#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(in crate::preproc) fn parse_include_target(raw_target: &str) -> IncludeTarget {
    let trimmed = raw_target.trim();
    let unwrapped = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| trimmed.strip_prefix('<').and_then(|s| s.strip_suffix('>')))
        .unwrap_or(trimmed);
    let (path, tag) = if unwrapped.contains("://") {
        (unwrapped, None)
    } else if let Some((path, tag)) = unwrapped.rsplit_once('!') {
        let clean_tag = tag.trim();
        if path.trim().is_empty() || clean_tag.is_empty() {
            (unwrapped, None)
        } else {
            (path.trim(), Some(clean_tag.to_string()))
        }
    } else {
        (unwrapped, None)
    };

    IncludeTarget {
        path: PathBuf::from(path),
        tag,
    }
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
pub(in crate::preproc) fn parse_import_target(raw_target: &str) -> Result<PathBuf, Diagnostic> {
    let trimmed = raw_target.trim();
    let unwrapped = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| trimmed.strip_prefix('<').and_then(|s| s.strip_suffix('>')))
        .unwrap_or(trimmed)
        .trim();
    if unwrapped.is_empty() {
        return Err(Diagnostic::error_code(
            "E_IMPORT_PATH_REQUIRED",
            "!import requires a stdlib module path",
        ));
    }
    if unwrapped.contains('!') {
        return Err(Diagnostic::error_code(
            "E_IMPORT_INVALID_FORM",
            format!("!import does not support tag selection (`path!TAG`): {raw_target}"),
        ));
    }

    let mut path = PathBuf::from(unwrapped);
    if path.extension().is_none() {
        path.set_extension("puml");
    }
    Ok(crate::stdlib::apply_stdlib_path_alias(path))
}
