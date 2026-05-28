/// Standalone helper utilities for the inline creole parser.
///
/// These functions handle link syntax parsing, character classification, and
/// tag pattern matching. Extracted from `inline.rs` to keep that file under
/// the 600-line guardrail.

/// Parse the interior of a `[[...]]` link token into `(url, tooltip, label)`.
pub(super) fn parse_link_inner(inner: &str) -> (String, Option<String>, String) {
    if let Some(open) = inner.find('{') {
        if let Some(relative_close) = inner[open + 1..].find('}') {
            let close = open + 1 + relative_close;
            let url = inner[..open].trim().to_string();
            if !url.is_empty() {
                let tooltip = inner[open + 1..close].to_string();
                let label = inner[close + 1..].trim();
                let label = if label.is_empty() {
                    url.clone()
                } else {
                    label.to_string()
                };
                return (url, Some(tooltip), label);
            }
        }
    }

    let (target, label) = if let Some(sp) = inner.find(char::is_whitespace) {
        (&inner[..sp], inner[sp..].trim_start().to_string())
    } else {
        (inner, inner.to_string())
    };

    (target.to_string(), None, label)
}

pub(super) fn find_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())
}

pub(super) fn is_creole_pair(a: char, b: char) -> bool {
    matches!(
        (a, b),
        ('*', '*') | ('/', '/') | ('"', '"') | ('_', '_') | ('-', '-') | ('~', '~') | ('[', '[')
    )
}

pub(super) fn pair_exists_after(chars: &[char], start: usize, a: char, b: char) -> bool {
    chars
        .get(start..)
        .is_some_and(|tail| tail.windows(2).any(|pair| pair[0] == a && pair[1] == b))
}

/// Try to match `<tagname:value>` at the start of `s` (case-insensitive).
/// Returns `Some((value, consumed_bytes))` on success.
pub(super) fn parse_open_tag_with_value<'a>(s: &'a str, tagname: &str) -> Option<(&'a str, usize)> {
    let lower = s.to_ascii_lowercase();
    let prefix = format!("<{}:", tagname);
    if !lower.starts_with(&prefix) {
        return None;
    }
    let value_start = prefix.len();
    let close = s[value_start..].find('>')?;
    let value = &s[value_start..value_start + close];
    let consumed = value_start + close + 1; // includes '>'
    Some((value, consumed))
}

/// Match a literal prefix + suffix pattern (e.g. `<&` ... `>`).
pub(super) fn strip_tag_prefix<'a>(s: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    let lower_prefix = s
        .chars()
        .take(prefix.len())
        .collect::<String>()
        .to_ascii_lowercase();
    if lower_prefix != prefix.to_ascii_lowercase() {
        return None;
    }
    let inner_start = prefix.len();
    let close = s[inner_start..].find(suffix)?;
    Some(&s[inner_start..inner_start + close])
}
