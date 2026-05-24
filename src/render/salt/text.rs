use super::super::text_metrics::default_monospace_width;
use super::super::{creole_text, escape_text};

pub(super) fn estimate_text_width(text: &str) -> i32 {
    default_monospace_width(text)
}

pub(super) fn estimate_salt_text_width(text: &str) -> i32 {
    let lines = crate::creole::tokenize_creole(text);
    let max_chars = lines
        .iter()
        .map(|line| {
            line.iter()
                .map(|span| span.text.chars().count() as i32)
                .sum()
        })
        .max()
        .unwrap_or(0);
    max_chars * 7
}

pub(super) fn salt_text_line_count(text: &str) -> usize {
    crate::creole::tokenize_creole(text).len().max(1)
}

pub(super) fn salt_input_width(text: &str) -> i32 {
    estimate_text_width(text) + 29
}

pub(super) fn salt_button_width(text: &str) -> i32 {
    (estimate_text_width(text) + 16).max(36)
}

pub(super) fn salt_combo_width(text: &str) -> i32 {
    estimate_text_width(text) + 23
}

pub(super) fn salt_text(out: &mut String, x: i32, y: i32, attrs: &str, text: &str, color: &str) {
    let icon_names = extract_salt_icon_names(text);
    let mut extra_attrs = attrs.to_string();
    if salt_text_has_creole(text) {
        extra_attrs.push_str(" data-salt-creole=\"true\"");
    }
    if !icon_names.is_empty() {
        extra_attrs.push_str(&format!(
            " data-salt-icons=\"{}\"",
            escape_text(&icon_names.join(","))
        ));
    }
    out.push_str(&creole_text(x, y, &extra_attrs, text, color));
}

pub(super) fn salt_text_has_creole(text: &str) -> bool {
    text.contains("**")
        || text.contains("//")
        || text.contains("\"\"")
        || text.contains("__")
        || text.contains("--")
        || text.contains("[[")
        || text.contains("<color")
        || text.contains("<size")
        || text.contains("<b>")
        || text.contains("<B>")
        || text.contains("<i>")
        || text.contains("<I>")
        || text.contains("<u>")
        || text.contains("<U>")
        || text.contains("<&")
}

fn extract_salt_icon_names(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("<&") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find('>') else {
            break;
        };
        let name = rest[..end].trim();
        if !name.is_empty() {
            names.push(name.to_string());
        }
        rest = &rest[end + 1..];
    }
    names
}
