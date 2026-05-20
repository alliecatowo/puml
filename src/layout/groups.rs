use std::collections::BTreeMap;

use crate::scene::LayoutOptions;

use super::metrics::{
    estimate_text_px_width, GROUP_BOTTOM_PADDING, GROUP_HEADER_BASELINE_Y,
    GROUP_REF_BODY_BASELINE_Y, GROUP_TEXT_INSET_X, TEXT_LINE_HEIGHT,
};
use super::notes::note_target_bounds;

pub(super) fn group_horizontal_bounds(
    kind: &str,
    label: Option<&str>,
    bounds_by_id: &BTreeMap<String, (i32, i32)>,
    options: &LayoutOptions,
) -> (i32, i32) {
    let (min_content_width, _) = group_content_min_size(kind, label);
    if let Some(raw) = label {
        let header = raw.lines().next().unwrap_or(raw);
        if let Some(target_spec) = header.strip_prefix("over ") {
            let bounds = note_target_bounds(target_spec.trim(), bounds_by_id, options);
            if !bounds.is_empty() {
                let min_left = bounds
                    .iter()
                    .map(|(left, _)| *left)
                    .min()
                    .unwrap_or(options.margin);
                let max_right = bounds
                    .iter()
                    .map(|(_, right)| *right)
                    .max()
                    .unwrap_or(options.margin + options.participant_width);
                let target_width = (max_right - min_left).max(options.participant_width);
                let width = target_width.max(min_content_width);
                let x = (min_left - ((width - target_width) / 2)).max(options.margin);
                return (x, width);
            }
        }
    }
    let width = (bounds_by_id.len() as i32 * options.participant_spacing)
        .max(options.participant_width + 64)
        .max(min_content_width);
    (options.margin, width)
}

pub(super) fn group_content_min_size(kind: &str, label: Option<&str>) -> (i32, i32) {
    if kind.eq_ignore_ascii_case("box") {
        let min_width = label
            .map(|label| estimate_text_px_width(label) + (GROUP_TEXT_INSET_X * 2))
            .unwrap_or(0);
        return (min_width, 0);
    }
    let Some(label) = label else {
        return (0, 0);
    };
    let mut lines = label.split('\n');
    let header = lines.next().unwrap_or("");
    let header_text = format!("{kind} {header}");
    let mut max_width = estimate_text_px_width(header_text.trim());
    let mut height = GROUP_HEADER_BASELINE_Y + GROUP_BOTTOM_PADDING;

    if kind.eq_ignore_ascii_case("ref") {
        // For ref boxes all label lines (including the first "over ..." line)
        // appear in the body.  Count the header line too.
        let mut body_lines = 1; // the first line already consumed above
        for line in lines {
            max_width = max_width.max(estimate_text_px_width(line));
            body_lines += 1;
        }
        height = GROUP_REF_BODY_BASELINE_Y
            + ((body_lines - 1) * TEXT_LINE_HEIGHT)
            + GROUP_BOTTOM_PADDING;
    }

    (max_width + (GROUP_TEXT_INSET_X * 2), height)
}

pub(super) fn else_separator_label(label: Option<&str>) -> String {
    match label.map(str::trim).filter(|label| !label.is_empty()) {
        Some(label) => format!("else {label}"),
        None => "else".to_string(),
    }
}
