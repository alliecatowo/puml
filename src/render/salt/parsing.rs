use super::model::SaltCellRender;

pub(super) fn parse_salt_sprite_def(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.ends_with(">>") {
        return None;
    }
    let inner = trimmed.strip_prefix("<<")?;
    let name = inner
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches(">>")
        .trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

pub(super) fn parse_salt_sprite_ref(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let inner = trimmed.strip_prefix("<<")?.strip_suffix(">>")?.trim();
    if inner.is_empty() || inner.contains(char::is_whitespace) {
        None
    } else {
        Some(inner.to_string())
    }
}

pub(super) fn parse_salt_tree_line(line: &str) -> Option<(usize, String)> {
    // PlantUML `{T ... }` tree syntax: lines starting with `+` characters.
    // Example: `+ Root` (depth 0), `++ Child` (depth 1), `+++ Grandchild` (depth 2).
    let plus_depth = line.chars().take_while(|&ch| ch == '+').count();
    if plus_depth > 0 {
        let label = line[plus_depth..].trim().trim_matches('"').to_string();
        return if label.is_empty() {
            None
        } else {
            Some((plus_depth.saturating_sub(1), label))
        };
    }
    // Alternate outline/tree syntax using `**`/`***` markers (used inside `{.` containers).
    // Example: `** Folder A` (depth 0), `*** File 1` (depth 1).
    // Single `*` is NOT a tree marker — it is used for table spans elsewhere.
    let star_count = line.chars().take_while(|&ch| ch == '*').count();
    if star_count >= 2 {
        let label = line[star_count..].trim().trim_matches('"').to_string();
        return if label.is_empty() {
            None
        } else {
            // `**` → depth 0, `***` → depth 1, `****` → depth 2, etc.
            Some((star_count.saturating_sub(2), label))
        };
    }
    None
}

pub(super) fn parse_salt_items(line: &str, prefixes: &[&str]) -> Option<Vec<String>> {
    let lower = line.to_ascii_lowercase();
    let mut rest = None;
    for prefix in prefixes {
        if lower.starts_with(prefix)
            && (prefix.starts_with('{')
                || lower.len() == prefix.len()
                || lower
                    .as_bytes()
                    .get(prefix.len())
                    .is_some_and(|ch| ch.is_ascii_whitespace()))
        {
            rest = Some(line[prefix.len()..].trim());
            break;
        }
    }
    let rest = rest?;
    let rest = rest.trim_matches('{').trim_matches('}').trim();
    let items: Vec<String> = rest
        .split(['|', ','])
        .map(|item| item.trim().trim_matches('"').to_string())
        .filter(|item| !item.is_empty())
        .collect();
    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

/// Parse a `{/ Tab1 | **Tab2** | Tab3 }` tab-bar declaration.
/// Returns `(labels, active_index)` where active tab is detected from `**...**` markup.
/// Falls back to index 0 when no tab is marked.
pub(super) fn parse_salt_tab_bar(line: &str) -> Option<(Vec<String>, usize)> {
    let lower = line.to_ascii_lowercase();
    // Must start with `{/` (PlantUML tab syntax).
    // Bare `tab`/`tabs` keywords are handled by `parse_salt_items` elsewhere;
    // we deliberately do not match them here to avoid false-positives on lines
    // like "Tab content here" which begin with the word "tab".
    let rest = if lower.starts_with("{/") {
        line[2..].trim()
    } else {
        return None;
    };
    // Strip surrounding braces left over after the prefix
    let rest = rest.trim_start_matches('{').trim_end_matches('}').trim();
    if rest.is_empty() {
        return None;
    }
    let mut active = 0usize;
    let tabs: Vec<String> = rest
        .split('|')
        .enumerate()
        .filter_map(|(idx, raw)| {
            let t = raw.trim();
            if t.is_empty() {
                return None;
            }
            // `**label**` → active tab; strip markup and record index.
            if t.starts_with("**") && t.ends_with("**") && t.len() > 4 {
                active = idx;
                Some(t[2..t.len() - 2].trim().to_string())
            } else {
                Some(t.trim_matches('"').to_string())
            }
        })
        .collect();
    if tabs.is_empty() {
        None
    } else {
        Some((tabs, active))
    }
}

pub(super) fn parse_salt_scrollbar(line: &str) -> Option<(bool, u8)> {
    let lower = line.to_ascii_lowercase();
    if !(lower.starts_with("{s") || lower.starts_with("scroll") || lower.contains("scrollbar")) {
        return None;
    }
    let vertical = !lower.contains("horizontal");
    let percent = lower
        .split(|ch: char| !ch.is_ascii_digit())
        .find_map(|part| part.parse::<u8>().ok())
        .unwrap_or(40)
        .min(100);
    Some((vertical, percent))
}

pub(super) fn parse_salt_scroll_container(line: &str) -> Option<(bool, bool)> {
    let lower = line.to_ascii_lowercase();
    if !lower.starts_with("{s") || lower.starts_with("{*") {
        return None;
    }
    let marker = lower.trim_matches('{').trim_matches('}').trim();
    if marker.starts_with("si") {
        Some((true, false))
    } else if marker.starts_with("s-") {
        Some((false, true))
    } else {
        Some((true, true))
    }
}

/// Parse `^label^^item1^^item2^` (open / expanded droplist).
/// Returns `(label, items)` if the pattern matches a multi-item droplist,
/// or `None` for a plain closed `^label^` combo or a non-combo string.
pub(super) fn parse_salt_open_combo(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    if !trimmed.starts_with('^') {
        return None;
    }
    parse_salt_open_combo_payload(trimmed.trim_start_matches('^').trim_end_matches('^'))
}

pub(super) fn parse_salt_open_combo_payload(inner: &str) -> Option<(String, Vec<String>)> {
    let (label, raw_items) = inner.split_once("^^")?;
    let label = label.trim().to_string();
    let items: Vec<String> = raw_items
        .split("^^")
        .map(|part| part.trim())
        .filter(|item| !item.is_empty())
        .map(ToString::to_string)
        .collect();
    if label.is_empty() || items.is_empty() {
        return None;
    }
    Some((label, items))
}

/// Parse a password field: `"*hint*"` or `"****"` — a quoted string where
/// the content consists entirely of `*` characters or is wrapped in `*...*`.
/// Returns the hint label (empty string for fully-masked `"****"` input).
pub(super) fn parse_salt_password(line: &str) -> Option<String> {
    let trimmed = line.trim();
    // Must be a quoted string.
    let inner = trimmed.strip_prefix('"')?.strip_suffix('"')?;
    if inner.is_empty() {
        return None;
    }
    // Pattern 1: all asterisks inside quotes → masked input with no hint.
    if inner.chars().all(|c| c == '*') && inner.len() >= 2 {
        return Some(String::new());
    }
    // Pattern 2: `*hint*` — wrapped in asterisks with non-asterisk content inside.
    if let Some(hint) = inner.strip_prefix('*').and_then(|r| r.strip_suffix('*')) {
        if !hint.is_empty() && !hint.contains('"') {
            return Some(hint.to_string());
        }
    }
    None
}

/// Parse a slider: `{slider:min,max,value}`, `{slider:value}`, or bare `{slider}`.
/// Returns `(min, max, value)`. Defaults: min=0, max=100, value=50.
pub(super) fn parse_salt_slider(line: &str) -> Option<(i32, i32, i32)> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    if !lower.starts_with("{slider") && !lower.starts_with("slider") {
        return None;
    }
    // Strip surrounding braces if present.
    let inner = trimmed.trim_start_matches('{').trim_end_matches('}').trim();
    let lower_inner = inner.to_ascii_lowercase();
    let payload = lower_inner
        .strip_prefix("slider")?
        .trim_start_matches(':')
        .trim();
    if payload.is_empty() {
        // Bare `{slider}` — use defaults.
        return Some((0, 100, 50));
    }
    // Parse up to three comma-separated integers.
    let nums: Vec<i32> = payload
        .split(',')
        .filter_map(|p| p.trim().parse::<i32>().ok())
        .collect();
    match nums.as_slice() {
        [val] => Some((0, 100, (*val).clamp(0, 100))),
        [min, max] => {
            let lo = (*min).min(*max);
            let hi = (*min).max(*max);
            Some((lo, hi, (lo + hi) / 2))
        }
        [min, max, val] => {
            let lo = (*min).min(*max);
            let hi = (*min).max(*max);
            Some((lo, hi, (*val).clamp(lo, hi)))
        }
        _ => Some((0, 100, 50)),
    }
}

/// Parse a progress bar: `[=====   ]` or `[========]`.
/// Returns fill ratio [0.0, 1.0] if the cell looks like a progress bar.
pub(super) fn parse_salt_progress_bar(line: &str) -> Option<f32> {
    let trimmed = line.trim();
    // Must be enclosed in `[...]`
    let inner = trimmed.strip_prefix('[')?.strip_suffix(']')?;
    if inner.is_empty() {
        return None;
    }
    // Content must consist only of `=` and space characters (at least one `=`).
    if !inner.chars().all(|c| c == '=' || c == ' ') {
        return None;
    }
    let filled = inner.chars().filter(|&c| c == '=').count();
    let total = inner.len();
    if total == 0 {
        return None;
    }
    Some(filled as f32 / total as f32)
}

/// Decode a salt cell from the encoded string `"X:text"`.
pub(super) fn decode_salt_cell(s: &str) -> SaltCellRender {
    if let Some(rest) = s.strip_prefix("I:") {
        SaltCellRender::Input(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("B:") {
        SaltCellRender::Button(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("C:") {
        SaltCellRender::Combo(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("CX:") {
        SaltCellRender::CheckboxChecked(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("CU:") {
        SaltCellRender::CheckboxUnchecked(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("RO:") {
        SaltCellRender::RadioOn(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("RF:") {
        SaltCellRender::RadioOff(rest.to_string())
    } else if let Some(rest) = s.strip_prefix("L:") {
        SaltCellRender::Label(rest.to_string())
    } else {
        SaltCellRender::Label(s.to_string())
    }
}

/// Unit tests for Salt widget parsers added in #1503.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_password_all_stars_returns_empty_hint() {
        // `"****"` → password with no hint label.
        let result = parse_salt_password("\"****\"").expect("all-star quoted string");
        assert_eq!(result, "", "all-star password should have empty hint");
    }

    #[test]
    fn parse_password_starred_hint_returns_hint_text() {
        let result = parse_salt_password("\"*current password*\"").expect("starred hint");
        assert_eq!(result, "current password");
    }

    #[test]
    fn parse_password_plain_quoted_string_returns_none() {
        // Plain quoted string (no asterisk wrapping) must NOT parse as password.
        assert!(
            parse_salt_password("\"Enter your name\"").is_none(),
            "plain quoted string must not be a password field"
        );
    }

    #[test]
    fn parse_slider_bare_returns_defaults() {
        let (min, max, val) = parse_salt_slider("{slider}").expect("bare slider");
        assert_eq!((min, max, val), (0, 100, 50));
    }

    #[test]
    fn parse_slider_single_value() {
        let (min, max, val) = parse_salt_slider("{slider:60}").expect("single-value slider");
        assert_eq!((min, max, val), (0, 100, 60));
    }

    #[test]
    fn parse_slider_min_max_value() {
        let (min, max, val) = parse_salt_slider("{slider:0,100,75}").expect("full slider spec");
        assert_eq!((min, max, val), (0, 100, 75));
    }

    #[test]
    fn parse_slider_clamps_value_to_range() {
        let (min, max, val) = parse_salt_slider("{slider:10,50,200}").expect("out-of-range value");
        assert_eq!(min, 10);
        assert_eq!(max, 50);
        assert_eq!(val, 50, "value must be clamped to max");
    }

    #[test]
    fn parse_slider_non_slider_returns_none() {
        assert!(parse_salt_slider("\"some text\"").is_none());
        assert!(parse_salt_slider("[====   ]").is_none());
        assert!(parse_salt_slider("Name").is_none());
    }

    #[test]
    fn parse_slider_without_braces() {
        // Bare `slider:0,100,40` (no braces) also parses.
        let (min, max, val) = parse_salt_slider("slider:0,100,40").expect("bare syntax");
        assert_eq!((min, max, val), (0, 100, 40));
    }
}
