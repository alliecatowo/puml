use crate::theme::styles::MonochromeMode;

pub(super) fn parse_bool_value(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => Some(true),
        "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

pub(super) fn parse_monochrome_value(value: &str) -> Option<Option<MonochromeMode>> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" => Some(Some(MonochromeMode::True)),
        "reverse" => Some(Some(MonochromeMode::Reverse)),
        "false" | "no" | "off" => Some(None),
        _ => None,
    }
}

pub(super) fn split_stereotype_scope(key: &str) -> (String, Option<String>) {
    let trimmed = key.trim();
    let Some(prefix) = trimmed.strip_suffix(">>") else {
        return (trimmed.to_ascii_lowercase(), None);
    };
    let Some(start) = prefix.rfind("<<") else {
        return (trimmed.to_ascii_lowercase(), None);
    };
    let base = prefix[..start].trim().to_ascii_lowercase();
    let stereotype = prefix[start + 2..].trim();
    if base.is_empty() || stereotype.is_empty() {
        return (trimmed.to_ascii_lowercase(), None);
    }
    (base, Some(stereotype.to_ascii_lowercase()))
}
