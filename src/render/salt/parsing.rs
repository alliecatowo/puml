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
    let depth = line.chars().take_while(|&ch| ch == '+').count();
    if depth == 0 {
        return None;
    }
    let label = line[depth..].trim().trim_matches('"').to_string();
    if label.is_empty() {
        None
    } else {
        Some((depth.saturating_sub(1), label))
    }
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
