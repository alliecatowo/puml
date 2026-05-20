use crate::scene::{Label, LayoutOptions};

pub(super) const TEXT_LINE_HEIGHT: i32 = 16;
pub(super) const GROUP_TEXT_INSET_X: i32 = 8;
pub(super) const GROUP_HEADER_BASELINE_Y: i32 = 16;
pub(super) const GROUP_REF_BODY_BASELINE_Y: i32 = 32;
pub(super) const GROUP_BOTTOM_PADDING: i32 = 8;
pub(super) const NOTE_TEXT_WIDTH_GUARD_PX: i32 = 8;
pub(super) const METADATA_LINE_HEIGHT: i32 = 16;
pub(super) const METADATA_BLOCK_PADDING: i32 = 8;
pub(super) const TEOZ_ROUTE_LANE_HEIGHT: i32 = 14;
/// Height of the rendered self-loop U-shape below the message's `y` coordinate.
/// Must match `loop_h` in `render/sequence.rs`.
pub(super) const SELF_LOOP_DROP: i32 = 32;

pub(super) fn metadata_label_block_height(label: Option<&Label>) -> i32 {
    label
        .map(|label| metadata_lines_block_height(Some(&label.lines)))
        .unwrap_or(0)
}

pub(super) fn metadata_block_right_edge(label: &Option<Label>, margin: i32) -> i32 {
    label
        .as_ref()
        .map(|label| {
            label
                .lines
                .iter()
                .map(|line| label.x + estimate_text_px_width(line) + margin)
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

pub(super) fn metadata_lines_right_edge(lines: Option<&Vec<String>>, margin: i32) -> i32 {
    lines
        .map(|lines| {
            lines
                .iter()
                .map(|line| margin + estimate_text_px_width(line) + margin)
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0)
}

pub(super) fn metadata_lines_block_height(lines: Option<&Vec<String>>) -> i32 {
    lines
        .map(|lines| (lines.len() as i32 * METADATA_LINE_HEIGHT) + METADATA_BLOCK_PADDING)
        .unwrap_or(0)
}

pub(super) fn structure_bounds(
    centers_by_id: &std::collections::BTreeMap<String, i32>,
    options: &LayoutOptions,
) -> (i32, i32) {
    let x1 = options.margin;
    let width = (centers_by_id.len() as i32 * options.participant_spacing)
        .max(options.participant_width + 64);
    (x1, x1 + width)
}

pub(super) fn default_center(options: &LayoutOptions) -> i32 {
    options.margin + options.participant_width / 2
}

pub(super) fn multiline_metrics(text: &str) -> (i32, i32) {
    let mut max_width = 0;
    let mut lines = 0;
    for line in text.split('\n') {
        max_width = max_width.max(estimate_text_px_width(line));
        lines += 1;
    }
    (max_width, lines)
}

pub(super) fn estimate_text_px_width(line: &str) -> i32 {
    (line.chars().count() as i32) * 7
}

pub(super) fn legend_box_size(text: &str) -> (i32, i32) {
    let lines = text.lines().collect::<Vec<_>>();
    let line_count = lines.len().max(1) as i32;
    let max_line_width = lines
        .iter()
        .map(|line| estimate_text_px_width(line))
        .max()
        .unwrap_or(0);
    let width = (max_line_width + 16).max(200);
    let height = 24 + (line_count * 16);
    (width, height)
}

pub(super) fn row_units_for_height(height: i32, row_height: i32) -> i32 {
    if row_height <= 0 {
        return 1;
    }
    ((height + row_height - 1) / row_height).max(1)
}
