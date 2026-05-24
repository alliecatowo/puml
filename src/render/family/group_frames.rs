use crate::model::FamilyGroup;
use crate::render::svg::escape_text;

use super::class_types::ClassNodeBox;

#[derive(Clone, Copy)]
pub(super) struct ClassGroupFrameRect {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
}

pub(super) const CLASS_GROUP_DEPTH_OUTSET: i32 = 18;
pub(super) const CLASS_GROUP_BASE_PAD: i32 = 20;
pub(super) const CLASS_GROUP_TAB_HEIGHT: i32 = 24;
pub(super) const CLASS_GROUP_LABEL_GAP: i32 = 28;

pub(super) fn class_group_frame_rect(
    group: &RenderGroupFrame,
    max_group_depth: usize,
    node_boxes: &std::collections::BTreeMap<String, ClassNodeBox>,
) -> Option<ClassGroupFrameRect> {
    let mut gx_min = i32::MAX;
    let mut gy_min = i32::MAX;
    let mut gx_max = i32::MIN;
    let mut gy_max = i32::MIN;
    let mut found_any = false;
    for member_id in &group.member_ids {
        if let Some(bx) = node_boxes.get(member_id.as_str()) {
            gx_min = gx_min.min(bx.x);
            gy_min = gy_min.min(bx.y);
            gx_max = gx_max.max(bx.x + bx.w);
            gy_max = gy_max.max(bx.y + bx.h);
            found_any = true;
        }
    }
    if !found_any {
        return None;
    }

    let depth_outset =
        (max_group_depth.saturating_sub(group.depth) as i32) * CLASS_GROUP_DEPTH_OUTSET;
    let pad = CLASS_GROUP_BASE_PAD + depth_outset;
    let label_header = CLASS_GROUP_TAB_HEIGHT + CLASS_GROUP_LABEL_GAP + depth_outset;
    let x = gx_min - pad;
    let y = gy_min - pad - label_header;
    let w = (gx_max - gx_min) + pad * 2;
    let h = (gy_max - gy_min) + pad * 2 + label_header;

    Some(ClassGroupFrameRect { x, y, w, h })
}

/// Render the group/package/namespace frames for a class diagram.
///
/// Draws labeled frame rectangles (with optional tab headers) behind all node
/// boxes so that node rectangles visually sit on top of the frame borders.
pub(super) fn render_class_group_frames(
    out: &mut String,
    group_frames: &[RenderGroupFrame],
    max_group_depth: usize,
    node_boxes: &std::collections::BTreeMap<String, ClassNodeBox>,
) {
    for group in group_frames {
        let Some(rect) = class_group_frame_rect(group, max_group_depth, node_boxes) else {
            continue;
        };
        let fx = rect.x;
        let fy = rect.y;
        let fw = rect.w;
        let fh = rect.h;

        let group_label = group.display_label();
        let uses_tab_header = matches!(group.kind.as_str(), "rectangle" | "package");

        out.push_str(&format!(
            "<rect class=\"uml-group-frame\" data-uml-group=\"{}\" x=\"{fx}\" y=\"{fy}\" width=\"{fw}\" height=\"{fh}\" rx=\"6\" ry=\"6\" fill=\"none\" stroke=\"#6366f1\" stroke-width=\"1.5\" stroke-dasharray=\"5 3\"/>",
            escape_text(&group.scope)
        ));
        if uses_tab_header {
            let tab_h = CLASS_GROUP_TAB_HEIGHT;
            let tab_w = ((group_label.len() as i32) * 8 + 16).max(60).min(fw);
            out.push_str(&format!(
                "<rect x=\"{fx}\" y=\"{fy}\" width=\"{tab_w}\" height=\"{tab_h}\" rx=\"6\" ry=\"6\" fill=\"#ffffff\" stroke=\"#6366f1\" stroke-width=\"1.5\"/>"
            ));
            out.push_str(&format!(
                "<rect x=\"{fx}\" y=\"{}\" width=\"{tab_w}\" height=\"8\" fill=\"#ffffff\" stroke=\"none\"/>",
                fy + tab_h - 8
            ));
            out.push_str(&format!(
                "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"#4338ca\">{label}</text>",
                tx = fx + 8,
                ty = fy + 16,
                label = escape_text(&group_label)
            ));
            out.push_str(&format!(
                "<line x1=\"{fx}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#6366f1\" stroke-width=\"1\"/>",
                fy + tab_h,
                fx + fw,
                fy + tab_h
            ));
        } else {
            out.push_str(&format!(
                "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"#4338ca\">{label}</text>",
                tx = fx + 8,
                ty = fy + 14,
                label = escape_text(&group_label)
            ));
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct RenderGroupFrame {
    pub(super) kind: String,
    pub(super) label: Option<String>,
    pub(super) scope: String,
    pub(super) member_ids: Vec<String>,
    pub(super) depth: usize,
}

impl RenderGroupFrame {
    pub(super) fn display_label(&self) -> String {
        match self.label.as_deref() {
            Some(label) if !label.is_empty() => {
                // For boundary keywords like `rectangle` (used in usecase diagrams as
                // system-boundary frames, fix #553), the label alone is the display
                // name — the keyword is structural, not part of the visible text.
                if self.kind == "rectangle" {
                    label.to_string()
                } else {
                    format!("{} {}", self.kind, label)
                }
            }
            _ => self.kind.clone(),
        }
    }
}

pub(super) fn collect_render_group_frames(groups: &[FamilyGroup]) -> Vec<RenderGroupFrame> {
    let mut frames: std::collections::BTreeMap<String, RenderGroupFrame> =
        std::collections::BTreeMap::new();

    for group in groups {
        let explicit_scope = group
            .label
            .as_deref()
            .filter(|label| !label.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| group.kind.clone());
        if !group.member_ids.is_empty() {
            let scope = explicit_scope;
            let depth = scope.split("::").filter(|part| !part.is_empty()).count();
            let key = format!("{}\x1f{}", group.kind, scope);
            let entry = frames.entry(key).or_insert_with(|| RenderGroupFrame {
                kind: group.kind.clone(),
                label: group.label.clone(),
                scope: scope.clone(),
                member_ids: Vec::new(),
                depth: depth.saturating_sub(1),
            });
            entry.member_ids.extend(group.member_ids.iter().cloned());
        }

        for member_id in &group.member_ids {
            let node_id = member_id
                .split('\t')
                .next()
                .unwrap_or(member_id.as_str())
                .trim();
            if node_id.is_empty() {
                continue;
            }
            let parts = node_id
                .split("::")
                .filter(|part| !part.is_empty())
                .collect::<Vec<_>>();
            if parts.len() < 2 {
                continue;
            }
            for prefix_len in 1..parts.len() {
                let scope = parts[..prefix_len].join("::");
                let key = format!("{}\x1f{}", group.kind, scope);
                let label = parts.get(prefix_len - 1).map(|value| (*value).to_string());
                let entry = frames.entry(key).or_insert_with(|| RenderGroupFrame {
                    kind: group.kind.clone(),
                    label,
                    scope: scope.clone(),
                    member_ids: Vec::new(),
                    depth: prefix_len.saturating_sub(1),
                });
                entry.member_ids.push(node_id.to_string());
            }
        }
    }

    let mut frames = frames.into_values().collect::<Vec<_>>();
    for frame in &mut frames {
        frame.member_ids.sort();
        frame.member_ids.dedup();
    }
    frames.sort_by(|a, b| {
        (a.depth, a.scope.as_str(), a.kind.as_str()).cmp(&(
            b.depth,
            b.scope.as_str(),
            b.kind.as_str(),
        ))
    });
    frames
}
