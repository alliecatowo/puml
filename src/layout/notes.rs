use std::collections::BTreeMap;

use crate::scene::LayoutOptions;

use super::metrics::default_center;

fn parse_target_ids(spec: &str) -> Vec<String> {
    spec.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn note_target_centers(
    target_spec: &str,
    centers_by_id: &BTreeMap<String, i32>,
    options: &LayoutOptions,
) -> Vec<i32> {
    let default = default_center(options);
    parse_target_ids(target_spec)
        .into_iter()
        .map(|id| centers_by_id.get(&id).copied().unwrap_or(default))
        .collect::<Vec<_>>()
}

pub(super) fn note_target_bounds(
    target_spec: &str,
    bounds_by_id: &BTreeMap<String, (i32, i32)>,
    options: &LayoutOptions,
) -> Vec<(i32, i32)> {
    let default_center = default_center(options);
    let default_bounds = (
        default_center - (options.participant_width / 2),
        default_center + (options.participant_width / 2),
    );
    parse_target_ids(target_spec)
        .into_iter()
        .map(|id| bounds_by_id.get(&id).copied().unwrap_or(default_bounds))
        .collect::<Vec<_>>()
}

pub(super) fn note_horizontal_bounds(
    position: &str,
    target_spec: Option<&str>,
    centers_by_id: &BTreeMap<String, i32>,
    bounds_by_id: &BTreeMap<String, (i32, i32)>,
    max_participant_right: i32,
    width: i32,
    options: &LayoutOptions,
) -> (i32, i32) {
    if position.eq_ignore_ascii_case("across") {
        let span_width = (max_participant_right - options.margin).max(options.note_width);
        return (options.margin, span_width.max(width));
    }

    let x = if let Some(target_spec) = target_spec {
        let bounds = note_target_bounds(target_spec, bounds_by_id, options);
        let min_left = bounds
            .iter()
            .map(|(left, _)| *left)
            .min()
            .unwrap_or(options.margin);
        let max_right = bounds
            .iter()
            .map(|(_, right)| *right)
            .max()
            .unwrap_or(max_participant_right);
        let centers = note_target_centers(target_spec, centers_by_id, options);
        let min_center = *centers.iter().min().unwrap_or(&default_center(options));
        let max_center = *centers.iter().max().unwrap_or(&default_center(options));
        let mid_center = (min_center + max_center) / 2;
        if position.eq_ignore_ascii_case("left") {
            min_left - width - 12
        } else if position.eq_ignore_ascii_case("right") {
            max_right + 12
        } else if bounds.len() > 1 {
            return (
                min_left.max(options.margin),
                width.max(max_right - min_left),
            );
        } else {
            mid_center - (width / 2)
        }
    } else {
        options.margin
    };

    (x, width)
}

pub(super) fn note_vertical_position_y(
    position: &str,
    row_y: i32,
    height: i32,
    events_top: i32,
) -> i32 {
    if position.eq_ignore_ascii_case("top") {
        return (row_y - height - 8).max(events_top - height - 8);
    }
    if position.eq_ignore_ascii_case("bottom") {
        return row_y + 8;
    }
    row_y
}
