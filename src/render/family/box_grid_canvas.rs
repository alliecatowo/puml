//! Canvas-size computation for the box-grid (component / deployment) renderer.
//!
//! Extracted from `box_grid.rs` to keep that file under the 600-LOC target.

use crate::model::{FamilyDocument, FamilyNodeKind};

use super::box_grid::PackageLayout;

/// All edge/size values derived from the hierarchical layout pass that
/// downstream SVG-generation phases need.
pub(super) struct BoxGridCanvasBounds {
    pub(super) svg_width: i32,
    pub(super) svg_height: i32,
    pub(super) all_pkg_bottom: i32,
    pub(super) ungrouped_bottom: i32,
    pub(super) gl_canvas_bottom: i32,
    pub(super) projection_extra_height: i32,
    pub(super) caption_block_h: i32,
}

/// Inputs for [`compute_canvas_bounds`].
pub(super) struct CanvasBoundsInput<'a> {
    pub(super) doc: &'a FamilyDocument,
    pub(super) family: &'a str,
    pub(super) pkg_layouts: &'a [PackageLayout],
    pub(super) pkg_frame_widths: &'a [i32],
    pub(super) pkg_frame_heights: &'a [i32],
    pub(super) positions: &'a std::collections::BTreeMap<String, (i32, i32, i32, i32)>,
    pub(super) ungrouped_len: usize,
    pub(super) gl_canvas_width: f64,
    pub(super) gl_canvas_height: f64,
    pub(super) inner_cols: i32,
    pub(super) cell_w: i32,
    pub(super) cell_h: i32,
    pub(super) inner_gap: i32,
    pub(super) canvas_margin: i32,
    pub(super) pkg_bottom: i32,
    pub(super) header_h: i32,
}

/// Compute SVG canvas dimensions and key vertical anchors.
///
/// This is Phase 2 of the box-grid rendering pipeline (after hierarchical
/// layout and position resolution).  All values are in i32 pixel coordinates.
pub(super) fn compute_canvas_bounds(inp: CanvasBoundsInput<'_>) -> BoxGridCanvasBounds {
    const CUBE_OFFSET: i32 = 12;
    let has_3d_node = inp
        .doc
        .nodes
        .iter()
        .any(|n| matches!(n.kind, FamilyNodeKind::Node | FamilyNodeKind::Frame));
    let shape_right_extra = if has_3d_node { CUBE_OFFSET } else { 0 };

    let all_pkg_bottom = inp
        .pkg_layouts
        .iter()
        .enumerate()
        .map(|(i, pkg)| pkg.abs_y + inp.pkg_frame_heights[i])
        .max()
        .unwrap_or(inp.canvas_margin + inp.header_h);
    let all_pkg_right = inp
        .pkg_layouts
        .iter()
        .enumerate()
        .map(|(i, pkg)| pkg.abs_x + inp.pkg_frame_widths[i])
        .max()
        .unwrap_or(inp.canvas_margin);

    let max_node_drawn_right = inp
        .positions
        .values()
        .map(|&(nx, _, nw, _)| nx + nw + shape_right_extra)
        .max()
        .unwrap_or(inp.canvas_margin);

    let ungrouped_right = if inp.ungrouped_len == 0 {
        0
    } else {
        let last_col = (inp.ungrouped_len as i32 - 1) % inp.inner_cols;
        inp.canvas_margin + last_col * (inp.cell_w + inp.inner_gap) + inp.cell_w + shape_right_extra
    };
    let ungrouped_bottom = if inp.ungrouped_len == 0 {
        0
    } else {
        let ungrouped_rows = (inp.ungrouped_len as i32 + inp.inner_cols - 1) / inp.inner_cols;
        inp.pkg_bottom + ungrouped_rows * (inp.cell_h + inp.inner_gap)
    };

    let gl_canvas_right = inp.gl_canvas_width as i32;
    let gl_canvas_bottom = inp.gl_canvas_height as i32;

    let projection_extra_height =
        super::projections::family_projection_extra_height(&inp.doc.json_projections);
    let relation_label_half_width = inp
        .doc
        .relations
        .iter()
        .filter_map(|rel| rel.label.as_ref())
        .map(|label| (crate::render::text_metrics::monospace_width(label, 7) + 12) / 2)
        .max()
        .unwrap_or(0);
    let right_gutter = if inp.family == "deployment" {
        inp.canvas_margin.max(12 + relation_label_half_width)
    } else {
        inp.canvas_margin
    };

    let svg_width = all_pkg_right
        .max(gl_canvas_right)
        .max(max_node_drawn_right)
        .max(ungrouped_right)
        .max(inp.canvas_margin)
        + right_gutter;
    let svg_width = svg_width.max(400);

    let caption_block_h =
        super::class_metadata::family_metadata_label_height(inp.doc.caption.as_deref());
    let svg_height = all_pkg_bottom.max(ungrouped_bottom).max(gl_canvas_bottom)
        + inp.canvas_margin
        + projection_extra_height
        + caption_block_h
        + super::class_metadata::family_metadata_label_height(inp.doc.footer.as_deref());

    BoxGridCanvasBounds {
        svg_width,
        svg_height,
        all_pkg_bottom,
        ungrouped_bottom,
        gl_canvas_bottom,
        projection_extra_height,
        caption_block_h,
    }
}
