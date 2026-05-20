pub(super) fn parse_participant_group_label(raw: Option<&str>) -> (Option<String>, Option<String>) {
    let Some(raw) = raw.map(str::trim).filter(|raw| !raw.is_empty()) else {
        return (None, None);
    };

    let mut label = raw;
    let mut color = None;
    if let Some(last) = raw.split_whitespace().last() {
        if last.starts_with('#') && last.len() > 1 {
            color = Some(last.to_string());
            label = raw[..raw.len() - last.len()].trim_end();
        }
    }

    let label = label
        .trim()
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .unwrap_or(label)
        .trim();

    ((!label.is_empty()).then_some(label.to_string()), color)
}

#[derive(Debug, Clone)]
pub(super) struct GroupFrame {
    pub(super) kind: String,
    pub(super) span: crate::source::Span,
    pub(super) branch_has_content: bool,
}

pub(super) fn mark_group_content(group_stack: &mut [GroupFrame]) {
    for frame in group_stack {
        frame.branch_has_content = true;
    }
}

pub(super) fn allows_else(kind: &str) -> bool {
    matches!(kind, "alt" | "par" | "critical")
}

/// Returns `true` if `separator` is a valid branch-separator keyword inside a
/// group of type `group_kind`.  `else` works in `alt`, `par`, and `critical`.
/// `also` is the `par`-specific parallel-branch continuation (PlantUML parity,
/// fixes #780).
pub(super) fn allows_branch_separator(group_kind: &str, separator: &str) -> bool {
    match separator {
        "also" => matches!(group_kind, "par"),
        _ => allows_else(group_kind),
    }
}

pub(super) fn rejects_empty_group(kind: &str) -> bool {
    matches!(kind, "alt" | "par" | "critical")
}
