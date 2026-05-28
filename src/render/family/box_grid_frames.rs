use std::collections::BTreeMap;

use crate::model::FamilyDocument;
use crate::render::svg::escape_text;
use crate::theme::ComponentStyle;

use super::box_grid::PackageLayout;
use super::group_frames::collect_render_group_frames;

pub(super) struct BoxGridPackageFrameInputs<'a> {
    pub(super) doc: &'a FamilyDocument,
    pub(super) pkg_layouts: &'a [PackageLayout],
    pub(super) pkg_frame_widths: &'a [i32],
    pub(super) pkg_frame_heights: &'a [i32],
    pub(super) pkg_tab: i32,
    pub(super) comp_style: &'a ComponentStyle,
    pub(super) positions: &'a BTreeMap<String, (i32, i32, i32, i32)>,
}

pub(super) fn render_box_grid_package_frames(
    out: &mut String,
    inputs: BoxGridPackageFrameInputs<'_>,
) {
    let BoxGridPackageFrameInputs {
        doc,
        pkg_layouts,
        pkg_frame_widths,
        pkg_frame_heights,
        pkg_tab,
        comp_style,
        positions,
    } = inputs;

    // ─────────────────────────────────────────────────────────────────────────
    // Phase 1e: Render package frames (BEFORE nodes, so nodes sit on top)
    // ─────────────────────────────────────────────────────────────────────────
    for (i, pkg) in pkg_layouts.iter().enumerate() {
        let fw = pkg_frame_widths[i];
        let fh = pkg_frame_heights[i];
        let fx = pkg.abs_x;
        let fy = pkg.abs_y;

        // Draw the outer frame first (light fill, dark border, rounded corners).
        // Use a per-frame fill color if the group header specified one (e.g. `frame "X" #LightYellow`).
        let frame_fill = pkg
            .fill_color
            .as_deref()
            .and_then(|c| {
                let token = c.trim_start_matches('#');
                crate::theme::color::parse_color_value(token)
                    .or_else(|| crate::theme::color::parse_color_value(c))
            })
            .unwrap_or_else(|| "#f8faff".to_string());
        out.push_str(&format!(
        "<rect class=\"uml-group-frame\" data-uml-group=\"{}\" x=\"{fx}\" y=\"{fy}\" width=\"{fw}\" height=\"{fh}\" rx=\"8\" ry=\"8\" fill=\"{frame_fill}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        escape_text(&pkg.scope),
        comp_style.border_color
    ));
        // Full-width header band inset into the frame top.  The band uses
        // rounded corners at the top-left and top-right (matching the outer
        // frame), then flat square corners at the bottom via a cover rect.
        // This makes the dark band look like an inset header, not a floating tab.
        out.push_str(&format!(
        "<rect x=\"{fx}\" y=\"{fy}\" width=\"{fw}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"{}\" stroke=\"none\"/>",
        pkg_tab,
        comp_style.border_color,
    ));
        // Flatten only the bottom 8px of the header band (square the bottom corners)
        out.push_str(&format!(
            "<rect x=\"{fx}\" y=\"{}\" width=\"{fw}\" height=\"8\" fill=\"{}\" stroke=\"none\"/>",
            fy + pkg_tab - 8,
            comp_style.border_color
        ));
        // Package label text in the header band (left-aligned, vertically centred)
        out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" font-weight=\"600\" fill=\"#ffffff\">{}</text>",
        fx + 8,
        fy + pkg_tab - 8,
        escape_text(&pkg.label)
    ));
        // Horizontal separator line between header band and content area
        out.push_str(&format!(
            "<line x1=\"{fx}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            fy + pkg_tab,
            fx + fw,
            fy + pkg_tab,
            comp_style.border_color
        ));
    }

    // ── Nested sub-group frames (from collect_render_group_frames, depth > 0) ──
    // These handle nested packages like `node Rack { ... }` inside `package Edge { ... }`.
    // We draw them after top-level packages (so they appear inside), before nodes.
    {
        let all_group_frames = collect_render_group_frames(&doc.groups);
        let max_group_depth = all_group_frames.iter().map(|f| f.depth).max().unwrap_or(0);
        for frame in &all_group_frames {
            if frame.depth == 0 {
                // Top-level frames are already drawn above
                continue;
            }
            // Compute bounding box of all member nodes in this sub-frame
            let mut gx_min = i32::MAX;
            let mut gy_min = i32::MAX;
            let mut gx_max = i32::MIN;
            let mut gy_max = i32::MIN;
            let mut found_any = false;
            for mid in &frame.member_ids {
                // Try direct lookup, or strip namespace prefix
                let lookup_key = mid.rsplit("::").next().unwrap_or(mid.as_str()).to_string();
                let found = positions
                    .get(mid.as_str())
                    .or_else(|| positions.get(lookup_key.as_str()));
                if let Some(&(bx, by, bw, bh)) = found {
                    gx_min = gx_min.min(bx);
                    gy_min = gy_min.min(by);
                    gx_max = gx_max.max(bx + bw);
                    gy_max = gy_max.max(by + bh);
                    found_any = true;
                }
            }
            if !found_any {
                continue;
            }
            let depth_outset = (max_group_depth.saturating_sub(frame.depth) as i32) * 8;
            let pad = 10 + depth_outset;
            let label_h = 20 + depth_outset;
            let fx = gx_min - pad;
            let fy = gy_min - pad - label_h;
            let fw = gx_max - gx_min + pad * 2;
            let fh = gy_max - gy_min + pad * 2 + label_h;
            let sub_label = frame.display_label();
            out.push_str(&format!(
            "<rect class=\"uml-group-frame\" data-uml-group=\"{}\" x=\"{fx}\" y=\"{fy}\" width=\"{fw}\" height=\"{fh}\" rx=\"6\" ry=\"6\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.5\" stroke-dasharray=\"4 3\"/>",
            escape_text(&frame.scope),
            comp_style.border_color
        ));
            out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" font-weight=\"600\" fill=\"{}\">{}</text>",
            fx + 6,
            fy + 13,
            comp_style.border_color,
            escape_text(&sub_label)
        ));
        }
    }
}
