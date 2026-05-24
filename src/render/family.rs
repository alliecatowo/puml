use super::relation::{
    has_ie_endpoint_marker, normalize_relation_endpoints, render_ie_marker_defs,
};
use super::svg::{creole_text, escape_text, render_actor_stick_figure};
use crate::ast::MemberModifier;
use crate::model::{FamilyDocument, FamilyNode, FamilyNodeKind, FamilyStyle};
use crate::theme::{ActorStyle, ClassStyle};

mod box_grid;
mod box_grid_edges;
mod box_grid_frames;
mod box_grid_labels;
mod c4_nodes;
mod class_relation_labels;
mod class_relations;
mod group_frames;
mod node_shapes;
mod projections;
mod tree;

pub use self::box_grid::{render_component_svg, render_deployment_svg};
pub use self::tree::render_family_tree_svg;

use self::c4_nodes::{c4_node_height, is_c4_component_kind, is_c4_kind, render_c4_node};
use self::class_relation_labels::{
    class_build_label_overrides, relation_pair_label_lane_map, relation_source_label_lane_map,
    resolve_relation_endpoint_key,
};
use self::class_relations::{render_class_relations, ClassRelationCtx};
use self::group_frames::{
    class_group_frame_rect, collect_render_group_frames, render_class_group_frames,
    RenderGroupFrame, CLASS_GROUP_LABEL_GAP, CLASS_GROUP_TAB_HEIGHT,
};
use self::projections::{class_projection_extra_height, render_family_projection_boxes};
use self::tree::render_centered_multiline_text;
/// Backwards-compatible alias for the family stub renderer. Now delegates to
/// the real renderer.
pub fn render_family_stub_svg(document: &FamilyDocument) -> String {
    render_class_svg(document)
}

/// Box geometry for a single class/node box used by `render_class_svg`.
#[derive(Clone, Copy)]
struct ClassNodeBox {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_h: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClassPortSide {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy)]
struct ClassEndpointAnchor {
    x: i32,
    y: i32,
    side: ClassPortSide,
    is_row_port: bool,
}

impl ClassEndpointAnchor {
    fn point(self) -> (i32, i32) {
        (self.x, self.y)
    }
}

fn class_port_side_from_box_anchor(x: i32, y: i32, node_box: &ClassNodeBox) -> ClassPortSide {
    let distances = [
        (ClassPortSide::Left, (x - node_box.x).abs()),
        (ClassPortSide::Right, (x - (node_box.x + node_box.w)).abs()),
        (ClassPortSide::Top, (y - node_box.y).abs()),
        (ClassPortSide::Bottom, (y - (node_box.y + node_box.h)).abs()),
    ];
    distances
        .into_iter()
        .min_by_key(|(_, distance)| *distance)
        .map(|(side, _)| side)
        .unwrap_or(ClassPortSide::Bottom)
}

fn class_port_normal(side: ClassPortSide) -> (i32, i32) {
    match side {
        ClassPortSide::Left => (-1, 0),
        ClassPortSide::Right => (1, 0),
        ClassPortSide::Top => (0, -1),
        ClassPortSide::Bottom => (0, 1),
    }
}

fn class_box_anchor_toward_point(
    node_box: &ClassNodeBox,
    point: (i32, i32),
) -> ClassEndpointAnchor {
    let cx = node_box.x + node_box.w / 2;
    let cy = node_box.y + node_box.h / 2;
    let (px, py) = point;
    let (x, y, side) = if py < node_box.y {
        (cx, node_box.y, ClassPortSide::Top)
    } else if py > node_box.y + node_box.h {
        (cx, node_box.y + node_box.h, ClassPortSide::Bottom)
    } else if px < cx {
        (node_box.x, cy, ClassPortSide::Left)
    } else {
        (node_box.x + node_box.w, cy, ClassPortSide::Right)
    };
    ClassEndpointAnchor {
        x,
        y,
        side,
        is_row_port: false,
    }
}

fn class_row_port_stub(
    anchor: ClassEndpointAnchor,
    original_adjacent: Option<(i32, i32)>,
) -> (i32, i32) {
    const ROW_PORT_STUB: i32 = 40;
    if anchor.is_row_port {
        let (nx, ny) = class_port_normal(anchor.side);
        return (anchor.x + nx * ROW_PORT_STUB, anchor.y + ny * ROW_PORT_STUB);
    }
    if let Some((ax, ay)) = original_adjacent {
        return match anchor.side {
            ClassPortSide::Left | ClassPortSide::Right => (ax, anchor.y),
            ClassPortSide::Top | ClassPortSide::Bottom => (anchor.x, ay),
        };
    }
    let (nx, ny) = class_port_normal(anchor.side);
    (anchor.x + nx * ROW_PORT_STUB, anchor.y + ny * ROW_PORT_STUB)
}

fn class_dedup_consecutive_points(points: Vec<(i32, i32)>) -> Vec<(i32, i32)> {
    let mut deduped = Vec::with_capacity(points.len());
    for point in points {
        if deduped.last().copied() != Some(point) {
            deduped.push(point);
        }
    }
    deduped
}

fn class_route_with_row_ports(
    start: ClassEndpointAnchor,
    end: ClassEndpointAnchor,
    original_points: Option<&[(i32, i32)]>,
) -> Option<Vec<(i32, i32)>> {
    if !start.is_row_port && !end.is_row_port {
        return None;
    }
    let start_adjacent = original_points.and_then(|points| points.get(1).copied());
    let end_adjacent = original_points.and_then(|points| {
        points
            .len()
            .checked_sub(2)
            .and_then(|idx| points.get(idx).copied())
    });
    let start_stub = class_row_port_stub(start, start_adjacent);
    let end_stub = class_row_port_stub(end, end_adjacent);

    let mut points = vec![start.point(), start_stub];
    if start_stub.0 != end_stub.0 && start_stub.1 != end_stub.1 {
        let bend = match start.side {
            ClassPortSide::Left | ClassPortSide::Right => (start_stub.0, end_stub.1),
            ClassPortSide::Top | ClassPortSide::Bottom => (end_stub.0, start_stub.1),
        };
        points.push(bend);
    }
    points.push(end_stub);
    points.push(end.point());
    Some(class_dedup_consecutive_points(points))
}

/// Nudge a label's y-coordinate upward until it no longer overlaps any node box.
/// Used by `render_class_svg` for both the pre-pass and the inline placement.
fn class_nudge_label_y(
    lx: i32,
    ly: i32,
    label_half_w: i32,
    node_boxes: &std::collections::BTreeMap<String, ClassNodeBox>,
) -> (i32, i32) {
    let mut adjusted_y = ly;
    for _ in 0..8 {
        let overlap = node_boxes.values().find(|bbox| {
            lx + label_half_w >= bbox.x - 8
                && lx - label_half_w <= bbox.x + bbox.w + 8
                && adjusted_y >= bbox.y - 14
                && adjusted_y <= bbox.y + bbox.h + 6
        });
        match overlap {
            Some(bbox) => adjusted_y = bbox.y - 18,
            None => break,
        }
    }
    (lx, adjusted_y)
}

/// Run the hierarchical layout engine and populate `node_boxes` for `render_class_svg`.
///
/// Builds `GlNodeSize` / `GlEdgeSpec` inputs for `layout_hierarchical`, runs the
/// layout, then converts the resulting `(f64, f64)` positions into `ClassNodeBox`
/// entries.  Falls back to a simple grid for any node the engine did not place.
/// Returns `(GraphLayout, BTreeMap<String, ClassNodeBox>)`.
#[allow(clippy::too_many_arguments)]
fn class_run_layout(
    document: &FamilyDocument,
    node_heights: &[(String, i32)],
    node_width: i32,
    col_count: i32,
    col_gap: i32,
    row_gap: i32,
    margin_x: i32,
    margin_top: i32,
    title_block_height: i32,
    group_top_reserve: i32,
    header_height: i32,
) -> (
    crate::render::graph_layout::GraphLayout,
    std::collections::BTreeMap<String, ClassNodeBox>,
) {
    use crate::render::graph_layout::{
        layout_hierarchical, EdgeSpec as GlEdgeSpec, LayoutOptions as GlOptions,
        NodeSize as GlNodeSize,
    };

    // Build group membership lookup for parent assignment.
    let group_frames_for_gl = collect_render_group_frames(&document.groups);
    let mut node_to_gl_group: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for frame in &group_frames_for_gl {
        for mid in &frame.member_ids {
            node_to_gl_group
                .entry(mid.clone())
                .or_insert_with(|| frame.scope.clone());
        }
    }

    let gl_nodes: Vec<GlNodeSize> = document
        .nodes
        .iter()
        .zip(node_heights.iter())
        .map(|(node, (_key, h))| {
            let key = node.alias.clone().unwrap_or_else(|| node.name.clone());
            let parent = node_to_gl_group
                .get(&key)
                .or_else(|| node_to_gl_group.get(&node.name))
                .cloned();
            let width = class_node_width(node.kind, node_width);
            GlNodeSize {
                id: key,
                width: width as f64,
                height: *h as f64,
                parent,
            }
        })
        .collect();

    // Build a resolver from unscoped/alias names to full node IDs so edges match.
    let mut gl_name_to_id: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for n in &gl_nodes {
        gl_name_to_id
            .entry(n.id.clone())
            .or_insert_with(|| n.id.clone());
        if let Some(tail) = n.id.rsplit("::").next() {
            gl_name_to_id
                .entry(tail.to_string())
                .or_insert_with(|| n.id.clone());
        }
    }
    for node in &document.nodes {
        if let Some(alias) = &node.alias {
            let scoped = node.alias.clone().unwrap_or_else(|| node.name.clone());
            gl_name_to_id
                .entry(alias.clone())
                .or_insert_with(|| scoped.clone());
            gl_name_to_id
                .entry(node.name.clone())
                .or_insert_with(|| scoped);
        }
    }
    let resolve_gl = |name: &str| -> String {
        if let Some(id) = gl_name_to_id.get(name) {
            return id.clone();
        }
        if let Some((owner, _member)) = name.rsplit_once("::") {
            if let Some(id) = gl_name_to_id.get(owner) {
                return id.clone();
            }
        }
        name.to_string()
    };

    let gl_edges: Vec<GlEdgeSpec> = document
        .relations
        .iter()
        .enumerate()
        .map(|(i, rel)| GlEdgeSpec {
            id: format!("r{i}"),
            from: resolve_gl(&rel.from),
            to: resolve_gl(&rel.to),
        })
        .collect();

    let is_usecase = is_real_usecase_layout(document);
    let rank_separation = if is_usecase {
        // Keep usecase layouts tighter than class/object while still reserving
        // enough inter-rank clearance so actors/labels don't collide with
        // package headers in grouped examples.
        let max_node_h = node_heights.iter().map(|(_, h)| *h).max().unwrap_or(60) as f64;
        row_gap as f64 + max_node_h * 0.5
    } else {
        (row_gap + node_heights.iter().map(|(_, h)| *h).max().unwrap_or(60)) as f64
    };
    let gl_options = GlOptions {
        rank_separation,
        node_separation: col_gap as f64,
        group_padding: 16.0,
        direction: crate::render::graph_layout::Direction::TopDown,
        canvas_margin: (margin_top + title_block_height + group_top_reserve) as f64,
        // Right-side gutter only needs margin_x (32px); canvas_margin also absorbs
        // title height and group-label tabs which are only needed vertically.
        canvas_right_margin: Some(margin_x as f64),
        stack_staggered_group_collisions: false,
    };

    let mut gl_result = layout_hierarchical(&gl_nodes, &gl_edges, &gl_options);

    // Narrow #803 behavior: for object diagrams only, normalize non-parallel
    // cross-rank channels to the geometric midpoint between endpoints so forked
    // one-to-many edges don't receive incidental per-channel fan offsets.
    //
    // This keeps the fix local to object rendering instead of changing global
    // graph-layout geometry for every class-like family.
    let is_object_diagram = !document.nodes.is_empty()
        && document
            .nodes
            .iter()
            .all(|node| matches!(node.kind, FamilyNodeKind::Object));
    if is_object_diagram {
        let mut pair_counts: std::collections::BTreeMap<(String, String), usize> =
            std::collections::BTreeMap::new();
        for edge in &gl_edges {
            *pair_counts
                .entry((edge.from.clone(), edge.to.clone()))
                .or_insert(0) += 1;
        }
        for (idx, edge) in gl_edges.iter().enumerate() {
            if pair_counts
                .get(&(edge.from.clone(), edge.to.clone()))
                .copied()
                .unwrap_or(0)
                > 1
            {
                continue;
            }
            let edge_id = format!("r{idx}");
            let Some(path) = gl_result.edge_paths.get_mut(&edge_id) else {
                continue;
            };
            if path.len() < 4 {
                continue;
            }
            let src = path[0];
            let dst = *path.last().unwrap_or(&src);
            // Only adjust cross-rank routes where interior points represent a
            // channel bend between source and target rows.
            if (src.1 - dst.1).abs() < 1.0 {
                continue;
            }
            let mid_y = (src.1 + dst.1) / 2.0;
            let end = path.len().saturating_sub(1);
            for p in path.iter_mut().take(end).skip(1) {
                p.1 = mid_y;
            }
        }
    }

    // Populate node_boxes: use layout positions when available, else grid fallback.
    let mut node_boxes: std::collections::BTreeMap<String, ClassNodeBox> =
        std::collections::BTreeMap::new();
    let total_nodes = document.nodes.len() as i32;
    let row_count = if total_nodes == 0 {
        0
    } else {
        (total_nodes + col_count - 1) / col_count
    };
    let max_row_height = node_heights.iter().map(|(_, h)| *h).max().unwrap_or(60);
    let mut fallback_row_y_offsets: Vec<i32> = Vec::new();
    {
        let mut y = margin_top + title_block_height + group_top_reserve;
        for _ in 0..row_count {
            fallback_row_y_offsets.push(y);
            y += max_row_height + row_gap;
        }
    }

    for (idx, (node, (_key, h))) in document.nodes.iter().zip(node_heights.iter()).enumerate() {
        let key = node.alias.clone().unwrap_or_else(|| node.name.clone());
        let (nx, ny) = if let Some(&(lx, ly)) = gl_result.node_positions.get(&key) {
            (lx as i32, ly as i32)
        } else {
            let col = (idx as i32) % col_count;
            let row = (idx as i32) / col_count;
            let fx = margin_x + col * (node_width + col_gap);
            let fy = fallback_row_y_offsets
                .get(row as usize)
                .copied()
                .unwrap_or(margin_top + title_block_height);
            (fx, fy)
        };
        let width = class_node_width(node.kind, node_width);
        let nb = ClassNodeBox {
            x: nx,
            y: ny,
            w: width,
            h: *h,
            header_h: header_height,
        };
        node_boxes.insert(key.clone(), nb);
        if node.alias.is_some() {
            node_boxes.entry(node.name.clone()).or_insert(nb);
        }
        // Register by unscoped name for relations that reference "Browse" not
        // "Online Store::Browse" (fixes rectangle group scoping in usecase diagrams).
        if key.contains("::") {
            if let Some(unscoped) = key.rsplit("::").next() {
                node_boxes.entry(unscoped.to_string()).or_insert(nb);
            }
        }
    }

    (gl_result, node_boxes)
}

/// Output of `class_compute_canvas` — the canvas dimensions and node extents
/// needed to build the SVG header and position projections/labels.
struct ClassCanvasMetrics {
    svg_width: i32,
    svg_height: i32,
    nodes_bottom: i32,
}

/// Compute SVG canvas size and related metrics for `render_class_svg`.
///
/// Derives the canvas width/height from the bounding boxes of laid-out nodes,
/// group frames, and the layout engine floor values.  Also computes the total
/// projection extra height so the SVG is tall enough to include them.
#[allow(clippy::too_many_arguments)] // 10 distinct canvas metrics; a struct would add churn without clarity
fn class_compute_canvas(
    node_boxes: &std::collections::BTreeMap<String, ClassNodeBox>,
    group_frames: &[RenderGroupFrame],
    max_group_depth: usize,
    json_projections: &[crate::model::JsonProjection],
    relations: &[crate::model::FamilyRelation],
    gl_canvas_width: f64,
    gl_canvas_height: f64,
    margin_x: i32,
    margin_top: i32,
    title_block_height: i32,
) -> ClassCanvasMetrics {
    let nodes_right = node_boxes
        .values()
        .map(|b| b.x + b.w)
        .max()
        .unwrap_or(margin_x);
    let nodes_bottom = node_boxes
        .values()
        .map(|b| b.y + b.h)
        .max()
        .unwrap_or(margin_top + title_block_height);

    let mut groups_right = margin_x;
    let mut groups_bottom = margin_top + title_block_height;
    for group in group_frames {
        if let Some(rect) = class_group_frame_rect(group, max_group_depth, node_boxes) {
            groups_right = groups_right.max(rect.x + rect.w);
            groups_bottom = groups_bottom.max(rect.y + rect.h);
        }
    }

    let proj_extra_height = class_projection_extra_height(json_projections);

    let gl_canvas_right = gl_canvas_width as i32;
    let gl_canvas_bottom = gl_canvas_height as i32;
    let max_label_half_w = relations
        .iter()
        .map(|rel| {
            rel.label
                .as_ref()
                .map(|l| ((l.chars().count() as i32) * 6 / 2).max(18))
                .unwrap_or(0)
        })
        .max()
        .unwrap_or(0);
    // label_right_pad: the canvas right edge must be far enough that when a
    // relation label is side-cleared to `nodes_right + 8 + label_half_w` it
    // still fits within the clamping range `[..., svg_width - margin_x - 8 -
    // label_half_w]`.  Solving: svg_width >= nodes_right + 16 + 2*label_half_w
    // + margin_x, so pad = 16 + 2*max_label_half_w + margin_x.
    let label_right_pad = 16 + 2 * max_label_half_w + margin_x;
    // Drop the old 3-column grid minimum (col_count*node_width) — it inflated
    // the canvas to 700+ px even for 2-node diagrams.
    let svg_width = gl_canvas_right
        .max(nodes_right + label_right_pad)
        .max(groups_right + margin_x);
    let svg_height =
        (nodes_bottom.max(groups_bottom) + 40 + proj_extra_height).max(gl_canvas_bottom + 40);

    ClassCanvasMetrics {
        svg_width,
        svg_height,
        nodes_bottom,
    }
}

fn class_node_width(kind: FamilyNodeKind, default_width: i32) -> i32 {
    match kind {
        FamilyNodeKind::Diamond => 44,
        _ => default_width,
    }
}

fn class_node_display_name(node: &FamilyNode, namespace_separator: Option<&str>) -> String {
    let raw_name = node.label.as_deref().unwrap_or(&node.name);
    namespace_separator
        .filter(|sep| !sep.is_empty())
        .map(|sep| raw_name.replace("::", sep))
        .unwrap_or_else(|| raw_name.to_string())
}

fn is_real_usecase_layout(document: &FamilyDocument) -> bool {
    if !matches!(document.kind, crate::ast::DiagramKind::UseCase) {
        return false;
    }

    document.nodes.iter().all(|node| {
        matches!(
            node.kind,
            FamilyNodeKind::UseCase
                | FamilyNodeKind::BusinessUseCase
                | FamilyNodeKind::Actor
                | FamilyNodeKind::BusinessActor
                | FamilyNodeKind::Person
                | FamilyNodeKind::Note
        ) && node.members.is_empty()
    })
}

/// Render Class/Object/UseCase documents as a real SVG with boxed nodes
/// (header + member compartment) laid out in a simple grid, plus arrows
/// for the document's relations.
pub fn render_class_svg(document: &FamilyDocument) -> String {
    // Extract class style (use defaults if not present)
    let class_style = match &document.family_style {
        Some(FamilyStyle::Class(s)) => s.clone(),
        _ => ClassStyle::default(),
    };

    // Layout constants
    let margin_x: i32 = 32;
    let margin_top: i32 = 32;
    let col_count: i32 = 3;
    let group_frames = collect_render_group_frames(&document.groups);
    let max_group_depth = group_frames
        .iter()
        .map(|frame| frame.depth)
        .max()
        .unwrap_or(0);
    // Auto-size node_width from longest member text / node name (fix #572).
    // char_width=7px (monospace), padding=24px (accounts for left+right insets).
    // Upper clamp raised to 600 so long member lines are never truncated.
    let node_width: i32 = {
        let name_px = document
            .nodes
            .iter()
            .map(|n| {
                class_node_display_name(n, document.namespace_separator.as_deref())
                    .chars()
                    .count() as i32
                    * 8
                    + 32
            })
            .max()
            .unwrap_or(200);
        let member_px = document
            .nodes
            .iter()
            .flat_map(|n| n.members.iter())
            .map(|m| m.text.chars().count() as i32 * 7 + 24)
            .max()
            .unwrap_or(0);
        name_px.max(member_px).clamp(160, 600)
    };
    // Reserve enough horizontal gap for the longest relation label so object
    // edge labels stay clear of adjacent box borders (#564, #484).
    let relation_label_gap = document
        .relations
        .iter()
        .map(|rel| {
            let label_w = rel
                .label
                .as_ref()
                .map(|label| (label.chars().count() as i32) * 7 + 24)
                .unwrap_or(0);
            let stereotype_w = rel
                .stereotype
                .as_ref()
                .map(|label| (label.chars().count() as i32) * 7 + 56)
                .unwrap_or(0);
            label_w.max(stereotype_w)
        })
        .max()
        .unwrap_or(0);
    let col_gap: i32 = 80.max(relation_label_gap);
    let is_usecase = is_real_usecase_layout(document);
    let row_gap: i32 = if is_usecase { 46 } else { 64 };
    let header_height: i32 = 30;
    let member_line_height: i32 = 16;
    let member_padding: i32 = 8;
    let empty_member_pad: i32 = 8;
    // Reserve enough top room for the deepest package frame header plus its
    // outer padding. The exact frame geometry is centralized in
    // `class_group_frame_rect`; this pre-layout value must cover the same stack.
    let group_top_reserve = if group_frames.is_empty() {
        0
    } else {
        ((max_group_depth as i32) + 1) * (CLASS_GROUP_TAB_HEIGHT + CLASS_GROUP_LABEL_GAP)
    };
    let relation_pair_label_lanes = relation_pair_label_lane_map(document);
    let relation_source_label_lanes = relation_source_label_lane_map(document);

    let title_block_height = document
        .title
        .as_deref()
        .map(|t| 12 + (t.lines().count() as i32) * 22)
        .unwrap_or(0);

    // Compute per-node heights (used both for layout and for node rendering).
    // Store as Vec<(key, h)> in declaration order.
    let node_heights: Vec<(String, i32)> = document
        .nodes
        .iter()
        .map(|node| {
            let key = node.alias.clone().unwrap_or_else(|| node.name.clone());
            let header_stereotype_count = count_header_stereotype_members(&node.members);
            let display_member_count = node.members.len().saturating_sub(header_stereotype_count);
            let stereotype_extra_h = (header_stereotype_count as i32) * 14;
            let body_h = if node.kind == FamilyNodeKind::Note {
                let lines = node
                    .label
                    .as_deref()
                    .unwrap_or(&node.name)
                    .lines()
                    .count()
                    .max(1) as i32;
                lines * 16 + 20
            } else if node.kind == FamilyNodeKind::Map {
                let rows = node
                    .members
                    .iter()
                    .filter(|member| parse_map_row(&member.text).is_some())
                    .count() as i32;
                if rows == 0 {
                    empty_member_pad
                } else {
                    rows * 18
                }
            } else if display_member_count == 0 {
                empty_member_pad
            } else {
                (display_member_count as i32) * member_line_height + 2 * member_padding
            };
            let h = c4_node_height(node.kind, header_height + stereotype_extra_h + body_h);
            (key, h)
        })
        .collect();

    // ── Hierarchical layout + node_boxes population ─────────────────────────────
    let (gl_result_class, node_boxes) = class_run_layout(
        document,
        &node_heights,
        node_width,
        col_count,
        col_gap,
        row_gap,
        margin_x,
        margin_top,
        title_block_height,
        group_top_reserve,
        header_height,
    );
    // ── Canvas dimensions from layout result ──────────────────────────────────
    let canvas = class_compute_canvas(
        &node_boxes,
        &group_frames,
        max_group_depth,
        &document.json_projections,
        &document.relations,
        gl_result_class.canvas_width,
        gl_result_class.canvas_height,
        margin_x,
        margin_top,
        title_block_height,
    );
    let svg_width = canvas.svg_width;
    let svg_height = canvas.svg_height;
    let nodes_bottom = canvas.nodes_bottom;
    // gl_canvas_right / gl_canvas_bottom consumed by class_compute_canvas
    let mut out = String::new();
    let sepia_attr = if document.style.sepia {
        " style=\"filter:sepia(1)\""
    } else {
        ""
    };
    let orientation_attr = format!(" data-orientation=\"{}\"", document.orientation.as_str());
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\"{orientation}{sepia}>",
        w = svg_width,
        h = svg_height,
        orientation = orientation_attr,
        sepia = sepia_attr,
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        escape_text(&class_style.background_color)
    ));

    // Arrowhead/diamond marker defs — use class_style.arrow_color for stroke
    let arrow_stroke = &class_style.arrow_color;
    // markerUnits="userSpaceOnUse" pins marker sizes in SVG user units so they
    // are NOT scaled by the parent element's stroke-width (fix #471 collision).
    // fill="#ffffff" instead of fill="white" avoids resvg keyword-inheritance
    // rendering the triangle filled in PNG output (fix #467).
    out.push_str("<defs>");
    out.push_str(&format!(
        "<marker id=\"arrow-open\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" \
         markerWidth=\"10\" markerHeight=\"10\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M0,0 L10,5 L0,10\" fill=\"none\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\"/>\
         </marker>",
    ));
    // Larger 16×16 marker for a clearly visible hollow triangle (fix #467).
    // refX=15 places the triangle tip exactly at the line endpoint; the white fill
    // covers the line shaft so only the triangle border is visible.
    out.push_str(&format!(
        "<marker id=\"arrow-triangle\" viewBox=\"0 0 16 16\" refX=\"15\" refY=\"8\" \
         markerWidth=\"16\" markerHeight=\"16\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <polygon points=\"0,1 14,8 0,15\" fill=\"#ffffff\" stroke=\"{arrow_stroke}\" stroke-width=\"1.5\" fill-rule=\"nonzero\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-diamond-filled\" viewBox=\"0 0 14 10\" refX=\"13\" refY=\"5\" \
         markerWidth=\"14\" markerHeight=\"10\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M0,5 L7,0 L14,5 L7,10 z\" fill=\"{arrow_stroke}\" stroke=\"{arrow_stroke}\" stroke-width=\"1\"/>\
         </marker>",
    ));
    out.push_str(&format!(
        "<marker id=\"arrow-diamond-open\" viewBox=\"0 0 14 10\" refX=\"13\" refY=\"5\" \
         markerWidth=\"14\" markerHeight=\"10\" markerUnits=\"userSpaceOnUse\" orient=\"auto-start-reverse\">\
         <path d=\"M0,5 L7,0 L14,5 L7,10 z\" fill=\"#ffffff\" stroke=\"{arrow_stroke}\" stroke-width=\"1\"/>\
         </marker>",
    ));
    if document
        .relations
        .iter()
        .any(|relation| has_ie_endpoint_marker(&relation.arrow))
    {
        render_ie_marker_defs(&mut out, arrow_stroke);
    }
    out.push_str("</defs>");

    // Title
    if let Some(title) = &document.title {
        let mut ty = margin_top;
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"{x}\" y=\"{y}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"#0f172a\">{txt}</text>",
                x = margin_x,
                y = ty,
                txt = escape_text(line)
            ));
            ty += 22;
        }
    }

    // ── Label de-collision pre-pass ───────────────────────────────────────────
    // Delegate to helper: fans labels that cluster on the same target node or
    // share the same horizontal y-band so they don't overlap (#706, #749).
    let label_override = class_build_label_overrides(
        &document.relations,
        &node_boxes,
        &gl_result_class.edge_paths,
    );

    // Build lateral-offset map for parallel edges and render all relations.
    // Delegate to helper to keep this orchestrator function concise.
    const PARALLEL_EDGE_GAP: i32 = 12;
    let mut parallel_groups: std::collections::BTreeMap<(String, String), Vec<usize>> =
        std::collections::BTreeMap::new();
    for (i, rel) in document.relations.iter().enumerate() {
        let (fn_, tn_, _) = normalize_relation_endpoints(&rel.from, &rel.to, &rel.arrow);
        let key = if fn_ <= tn_ { (fn_, tn_) } else { (tn_, fn_) };
        parallel_groups.entry(key).or_default().push(i);
    }
    let mut parallel_offset: std::collections::BTreeMap<usize, i32> =
        std::collections::BTreeMap::new();
    for group in parallel_groups.values() {
        if group.len() < 2 {
            continue;
        }
        let n = group.len() as i32;
        for (slot, &idx) in group.iter().enumerate() {
            let lane = slot as i32 - n / 2;
            parallel_offset.insert(idx, lane * PARALLEL_EDGE_GAP);
        }
    }
    render_class_relations(
        &mut out,
        &ClassRelationCtx {
            relations: &document.relations,
            nodes: &document.nodes,
            node_boxes: &node_boxes,
            edge_paths: &gl_result_class.edge_paths,
            label_override: &label_override,
            parallel_offset: &parallel_offset,
            relation_pair_label_lanes: &relation_pair_label_lanes,
            relation_source_label_lanes: &relation_source_label_lanes,
            class_style: &class_style,
            arrow_stroke: arrow_stroke.as_str(),
            margin_x,
            margin_top,
            svg_width,
        },
    );

    // Render group frames (together/package/namespace) BEFORE nodes so node
    // rectangles visually sit on top of the frame borders.
    render_class_group_frames(&mut out, &group_frames, max_group_depth, &node_boxes);

    // Render nodes
    for node in &document.nodes {
        let key = node.alias.clone().unwrap_or_else(|| node.name.clone());
        let Some(bx) = node_boxes.get(&key) else {
            continue;
        };
        render_class_node(
            &mut out,
            node,
            ClassNodeGeometry {
                x: bx.x,
                y: bx.y,
                w: bx.w,
                h: bx.h,
                header_h: bx.header_h,
            },
            &class_style,
            document.namespace_separator.as_deref(),
            document.hide_options.contains("stereotype"),
        );
    }

    // Render inline JSON/YAML projections below the node area.
    if !document.json_projections.is_empty() {
        let proj_margin_left = margin_x;
        render_family_projection_boxes(
            &mut out,
            &document.json_projections,
            proj_margin_left,
            nodes_bottom + 16,
            300,
        );
    }

    out.push_str("</svg>");
    out
}

/// Render a `@startsalt` wireframe grid as an SVG.
/// Nodes in the FamilyDocument whose `name` starts with `"SALT_ROW\x1f"` are
/// decoded back into cell lists and drawn as a proper wireframe table.
fn parse_visibility_member(member: &str) -> (Option<&'static str>, &'static str, &str) {
    let trimmed = member.trim();
    if let Some(rest) = trimmed.strip_prefix('\\') {
        if matches!(rest.chars().next(), Some('+' | '-' | '#' | '~')) {
            return (None, "#334155", rest);
        }
    }
    match trimmed.chars().next() {
        Some('+') => (Some("+"), "#16a34a", trimmed[1..].trim_start()),
        Some('-') => (Some("-"), "#dc2626", trimmed[1..].trim_start()),
        Some('#') => (Some("#"), "#d97706", trimmed[1..].trim_start()),
        Some('~') => (Some("~"), "#7c3aed", trimmed[1..].trim_start()),
        _ => (None, "#334155", trimmed),
    }
}

fn uml_visibility_name(symbol: &str) -> &'static str {
    match symbol {
        "+" => "public",
        "-" => "private",
        "#" => "protected",
        "~" => "package",
        _ => "unknown",
    }
}

fn member_modifier_name(modifier: Option<&MemberModifier>) -> Option<&'static str> {
    match modifier {
        Some(MemberModifier::Field) => Some("field"),
        Some(MemberModifier::Method) => Some("method"),
        Some(MemberModifier::Abstract) => Some("abstract"),
        Some(MemberModifier::Static) => Some("static"),
        None => None,
    }
}

/// Parse {abstract} / {static} modifiers from member text.
/// Returns (SVG style attrs string, cleaned text without modifiers).
fn parse_member_modifiers(text: &str) -> (&'static str, &str) {
    let t = text.trim();
    if let Some(rest) = t.strip_prefix("{abstract}") {
        (" font-style=\"italic\"", rest.trim_start())
    } else if let Some(rest) = t.strip_prefix("{static}") {
        (" text-decoration=\"underline\"", rest.trim_start())
    } else {
        ("", t)
    }
}

pub(crate) fn family_node_label(kind: FamilyNodeKind) -> &'static str {
    if let Some(spec) = crate::registry::graph_element_for_family_node_kind(kind) {
        return spec.renderer_label;
    }
    match kind {
        FamilyNodeKind::Class => "class",
        FamilyNodeKind::Object => "object",
        FamilyNodeKind::Map => "map",
        FamilyNodeKind::Diamond => "diamond",
        FamilyNodeKind::UseCase => "usecase",
        FamilyNodeKind::Salt => "widget",
        FamilyNodeKind::MindMap => "mindmap",
        FamilyNodeKind::Wbs => "wbs",
        FamilyNodeKind::Component => "component",
        FamilyNodeKind::Interface => "interface",
        FamilyNodeKind::Port => "port",
        FamilyNodeKind::Action => "action",
        FamilyNodeKind::Agent => "agent",
        FamilyNodeKind::Node => "node",
        FamilyNodeKind::Artifact => "artifact",
        FamilyNodeKind::Boundary => "boundary",
        FamilyNodeKind::Cloud => "cloud",
        FamilyNodeKind::Circle => "circle",
        FamilyNodeKind::Collections => "collections",
        FamilyNodeKind::Frame => "frame",
        FamilyNodeKind::Storage => "storage",
        FamilyNodeKind::Container => "container",
        FamilyNodeKind::Control => "control",
        FamilyNodeKind::Database => "database",
        FamilyNodeKind::Entity => "entity",
        FamilyNodeKind::Package => "package",
        FamilyNodeKind::Rectangle => "rectangle",
        FamilyNodeKind::Folder => "folder",
        FamilyNodeKind::File => "file",
        FamilyNodeKind::Card => "card",
        FamilyNodeKind::Actor => "actor",
        FamilyNodeKind::BusinessActor => "business-actor",
        FamilyNodeKind::BusinessUseCase => "business-usecase",
        FamilyNodeKind::Hexagon => "hexagon",
        FamilyNodeKind::Label => "label",
        FamilyNodeKind::Person => "person",
        FamilyNodeKind::Process => "process",
        FamilyNodeKind::Queue => "queue",
        FamilyNodeKind::Stack => "stack",
        FamilyNodeKind::UseCaseDeployment => "usecase",
        FamilyNodeKind::State => "state",
        FamilyNodeKind::StateInitial => "initial",
        FamilyNodeKind::StateFinal => "final",
        FamilyNodeKind::StateHistory => "history",
        FamilyNodeKind::ActivityStart => "start",
        FamilyNodeKind::ActivityStop => "stop",
        FamilyNodeKind::ActivityAction => "action",
        FamilyNodeKind::ActivityDecision => "decision",
        FamilyNodeKind::ActivityFork => "fork",
        FamilyNodeKind::ActivityForkEnd => "end fork",
        FamilyNodeKind::ActivityMerge => "merge",
        FamilyNodeKind::ActivityPartition => "partition",
        FamilyNodeKind::TimingConcise => "concise",
        FamilyNodeKind::TimingRobust => "robust",
        FamilyNodeKind::TimingClock => "clock",
        FamilyNodeKind::TimingBinary => "binary",
        FamilyNodeKind::TimingEvent => "event",
        FamilyNodeKind::Note => "note",
        // C4 family
        FamilyNodeKind::C4Person => "person",
        FamilyNodeKind::C4PersonExt => "person_ext",
        FamilyNodeKind::C4System => "system",
        FamilyNodeKind::C4SystemExt => "system_ext",
        FamilyNodeKind::C4SystemDb => "system_db",
        FamilyNodeKind::C4SystemQueue => "system_queue",
        FamilyNodeKind::C4Container => "container",
        FamilyNodeKind::C4ContainerExt => "container_ext",
        FamilyNodeKind::C4ContainerDb => "container_db",
        FamilyNodeKind::C4ContainerQueue => "container_queue",
        FamilyNodeKind::C4Component => "component",
        FamilyNodeKind::C4ComponentExt => "component_ext",
        FamilyNodeKind::C4ComponentDb => "component_db",
        FamilyNodeKind::C4ComponentQueue => "component_queue",
        FamilyNodeKind::C4Boundary => "boundary",
    }
}

struct ClassNodeGeometry {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    header_h: i32,
}

/// Return the recognised kind-stereotype label for a type-marker member
/// (e.g. `"<<enum>>"` → `Some("«enumeration»")`).  Only the built-in
/// keyword markers produced by the parser qualify; user-defined stereotypes
/// like `"<<controller>>"` are NOT covered here (they are handled separately).
fn builtin_type_stereotype_label(text: &str) -> Option<&'static str> {
    match text {
        "<<enum>>" => Some("\u{ab}enumeration\u{bb}"),
        "<<interface>>" => Some("\u{ab}interface\u{bb}"),
        "<<abstract>>" | "<<abstract class>>" => Some("\u{ab}abstract\u{bb}"),
        "<<annotation>>" => Some("\u{ab}annotation\u{bb}"),
        "<<protocol>>" => Some("\u{ab}protocol\u{bb}"),
        "<<struct>>" => Some("\u{ab}struct\u{bb}"),
        _ => None,
    }
}

/// Return true if `text` is an arbitrary user-defined stereotype marker
/// (any `<<…>>` value that is NOT one of the built-in type keywords).
fn is_user_stereotype(text: &str) -> bool {
    text.starts_with("<<") && text.ends_with(">>") && builtin_type_stereotype_label(text).is_none()
}

/// Count how many leading members of `members` are header stereotypes that
/// should be rendered in the class-box header rather than as member rows.
/// This includes the optional built-in type marker (first position) plus any
/// consecutive user-defined stereotype markers that immediately follow it.
fn count_header_stereotype_members(members: &[crate::ast::ClassMember]) -> usize {
    let mut skip = 0;
    // First member may be a built-in type marker (e.g. <<enum>>).
    if members
        .first()
        .is_some_and(|m| builtin_type_stereotype_label(&m.text).is_some())
    {
        skip += 1;
    }
    // Any consecutive user-defined <<…>> members directly after the type marker
    // (or at the start if there was no type marker) are also header stereotypes.
    while skip < members.len() && is_user_stereotype(&members[skip].text) {
        skip += 1;
    }
    skip
}

fn first_user_stereotype_key(node: &crate::model::FamilyNode) -> Option<String> {
    node.members.iter().find_map(|member| {
        let text = member.text.trim();
        is_user_stereotype(text).then(|| {
            text.trim_start_matches("<<")
                .trim_end_matches(">>")
                .trim()
                .to_ascii_lowercase()
        })
    })
}

#[derive(Debug, Clone, Copy)]
struct MapRow<'a> {
    key: &'a str,
    value: &'a str,
}

fn parse_map_row(text: &str) -> Option<MapRow<'_>> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }
    for sep in ["<=>", "=>"] {
        if let Some((key, value)) = trimmed.split_once(sep) {
            return Some(MapRow {
                key: key.trim(),
                value: value.trim(),
            });
        }
    }
    for marker in [
        "*--->", "*-->", "*---", "*--", "*->", "-->", "---", "--", "..>", "...", "..",
    ] {
        if let Some((lhs, rhs)) = trimmed.split_once(marker) {
            return Some(MapRow {
                key: lhs.trim(),
                value: rhs.trim(),
            });
        }
    }
    Some(MapRow {
        key: trimmed,
        value: "",
    })
}

fn map_row_anchor_y(
    node: &crate::model::FamilyNode,
    key: &str,
    y: i32,
    header_h: i32,
) -> Option<i32> {
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    let mut row_idx = 0;
    for member in &node.members {
        let Some(row) = parse_map_row(&member.text) else {
            continue;
        };
        if row.key == key {
            return Some(y + header_h + 12 + row_idx * 18);
        }
        row_idx += 1;
    }
    None
}

fn qualified_member_anchor_y(
    node: &crate::model::FamilyNode,
    member_key: &str,
    y: i32,
    header_h: i32,
) -> Option<i32> {
    let member_key = member_key.trim();
    if member_key.is_empty() {
        return None;
    }
    if node.kind == FamilyNodeKind::Map {
        return map_row_anchor_y(node, member_key, y, header_h);
    }

    let header_skip = count_header_stereotype_members(&node.members);
    let mut row_idx = 0;
    for member in node.members.iter().skip(header_skip) {
        let text = member.text.trim();
        if text == "--" || text == ".." {
            continue;
        }
        if text.is_empty() {
            row_idx += 1;
            continue;
        }
        if member_anchor_matches(text, member_key) {
            return Some(y + header_h + 16 + row_idx * 16);
        }
        row_idx += 1;
    }
    None
}

fn member_anchor_matches(member_text: &str, member_key: &str) -> bool {
    let (_visibility, _color, after_visibility) = parse_visibility_member(member_text.trim());
    let (_style, clean_text) = parse_member_modifiers(after_visibility.trim());
    let clean_text = clean_text.trim();
    if clean_text == member_key {
        return true;
    }
    let name = clean_text
        .split(['(', ':', '='])
        .next()
        .unwrap_or(clean_text)
        .trim();
    name == member_key
}

fn qualified_row_anchor(
    endpoint: &str,
    nodes: &[crate::model::FamilyNode],
    node_boxes: &std::collections::BTreeMap<String, ClassNodeBox>,
    other: &ClassNodeBox,
) -> Option<ClassEndpointAnchor> {
    let (owner, row_key) = endpoint.rsplit_once("::")?;
    let owner_key = resolve_relation_endpoint_key(owner, node_boxes);
    let owner_box = node_boxes.get(&owner_key)?;
    let owner_node = nodes.iter().find(|node| {
        node.name == owner
            || node.alias.as_deref() == Some(owner)
            || node.name == owner_key
            || node.alias.as_deref() == Some(owner_key.as_str())
    })?;
    let y = qualified_member_anchor_y(owner_node, row_key, owner_box.y, owner_box.header_h)?;
    let owner_cx = owner_box.x + owner_box.w / 2;
    let other_cx = other.x + other.w / 2;
    let (x, side) = if other_cx < owner_cx {
        (owner_box.x, ClassPortSide::Left)
    } else {
        (owner_box.x + owner_box.w, ClassPortSide::Right)
    };
    Some(ClassEndpointAnchor {
        x,
        y,
        side,
        is_row_port: true,
    })
}

struct MapRenderCtx<'a> {
    font_family: &'a str,
    member_font_size: u32,
    member_color: &'a str,
    stroke: &'a str,
}

fn render_map_rows(
    out: &mut String,
    node: &crate::model::FamilyNode,
    x: i32,
    y: i32,
    w: i32,
    header_h: i32,
    ctx: &MapRenderCtx<'_>,
) {
    let divider_x = x + (w * 45 / 100);
    let rows: Vec<_> = node
        .members
        .iter()
        .filter_map(|member| parse_map_row(&member.text))
        .collect();
    if rows.is_empty() {
        return;
    }
    out.push_str(&format!(
        "<line class=\"uml-map-divider\" x1=\"{divider_x}\" y1=\"{}\" x2=\"{divider_x}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        y + header_h,
        y + header_h + rows.len() as i32 * 18,
        ctx.stroke
    ));
    for (idx, row) in rows.iter().enumerate() {
        let row_top = y + header_h + idx as i32 * 18;
        let text_y = row_top + 12;
        if idx > 0 {
            out.push_str(&format!(
                "<line class=\"uml-map-row\" x1=\"{x}\" y1=\"{row_top}\" x2=\"{}\" y2=\"{row_top}\" stroke=\"{}\" stroke-width=\"1\"/>",
                x + w,
                ctx.stroke
            ));
        }
        let anchor = format!(
            "{}::{}",
            node.alias.as_deref().unwrap_or(&node.name),
            row.key
        );
        out.push_str(&format!(
            "<text class=\"uml-map-key\" data-uml-anchor=\"{}\" x=\"{}\" y=\"{text_y}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            escape_text(&anchor),
            x + 10,
            escape_text(ctx.font_family),
            ctx.member_font_size,
            escape_text(ctx.member_color),
            escape_text(row.key)
        ));
        out.push_str(&format!(
            "<text class=\"uml-map-value\" x=\"{}\" y=\"{text_y}\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\">{}</text>",
            divider_x + 10,
            escape_text(ctx.font_family),
            ctx.member_font_size,
            escape_text(ctx.member_color),
            escape_text(row.value)
        ));
    }
}

#[derive(Default)]
struct FamilyNodeInlineStyle {
    border_color: Option<String>,
    text_color: Option<String>,
    border_dashed: bool,
    border_thickness: Option<f32>,
}

fn family_node_inline_style(node: &crate::model::FamilyNode) -> FamilyNodeInlineStyle {
    let mut style = FamilyNodeInlineStyle::default();
    for member in &node.members {
        let text = member.text.trim();
        if let Some(color) = text.strip_prefix("\x1fstyle:border:") {
            style.border_color = Some(color.trim().to_string());
        } else if let Some(color) = text.strip_prefix("\x1fstyle:text:") {
            style.text_color = Some(color.trim().to_string());
        } else if text == "\x1fstyle:border-dashed" {
            style.border_dashed = true;
        } else if let Some(width) = text.strip_prefix("\x1fstyle:border-thickness:") {
            if let Ok(width) = width.trim().parse::<f32>() {
                style.border_thickness = Some(width.clamp(1.0, 8.0));
            }
        }
    }
    style
}

fn is_family_style_member(text: &str) -> bool {
    text.starts_with("\x1fstyle:")
        || text.starts_with("\x1fclass:")
        || text.starts_with("\x1ffamily:tag:")
}

fn class_node_visibility_symbol(node: &crate::model::FamilyNode) -> Option<&'static str> {
    node.members.iter().find_map(|member| {
        let symbol = member.text.strip_prefix("\x1fclass:visibility:")?;
        match symbol.trim() {
            "+" => Some("+"),
            "-" => Some("-"),
            "#" => Some("#"),
            "~" => Some("~"),
            _ => None,
        }
    })
}

fn render_class_node(
    out: &mut String,
    node: &crate::model::FamilyNode,
    geometry: ClassNodeGeometry,
    class_style: &ClassStyle,
    namespace_separator: Option<&str>,
    hide_stereotype: bool,
) {
    let ClassNodeGeometry {
        x,
        y,
        w,
        h,
        header_h,
    } = geometry;

    // ── C4 node rendering ─────────────────────────────────────────────────────
    if is_c4_kind(node.kind) {
        render_c4_node(out, node, x, y, w, h);
        return;
    }

    if node.kind == FamilyNodeKind::Note {
        render_note_card(out, x, y, w, h, node.label.as_deref().unwrap_or(&node.name));
        return;
    }

    let scoped_style =
        first_user_stereotype_key(node).and_then(|key| class_style.stereotype_styles.get(&key));
    let fill = node
        .fill_color
        .as_deref()
        .or_else(|| scoped_style.and_then(|style| style.background_color.as_deref()))
        .unwrap_or(&class_style.background_color);
    let inline_style = family_node_inline_style(node);
    let stroke = inline_style
        .border_color
        .as_deref()
        .or_else(|| scoped_style.and_then(|style| style.border_color.as_deref()))
        .unwrap_or(&class_style.border_color);
    let scoped_font_color = scoped_style
        .and_then(|style| style.font_color.as_deref())
        .filter(|color| !color.is_empty());
    let font_color = inline_style
        .text_color
        .as_deref()
        .or(scoped_font_color)
        .unwrap_or(&class_style.font_color);
    let member_color = inline_style
        .text_color
        .as_deref()
        .or(scoped_font_color)
        .unwrap_or(class_style.member_color.as_str());
    let stroke_dash = if inline_style.border_dashed {
        " stroke-dasharray=\"5 3\""
    } else {
        ""
    };
    let stroke_width = inline_style.border_thickness.unwrap_or(1.5);
    let font_family = class_style.font_name.as_deref().unwrap_or("monospace");
    let title_font_size = class_style.font_size.unwrap_or(13);
    let member_font_size = title_font_size.saturating_sub(2).max(9);
    // Determine the header fill colour.  For classes we also inspect the
    // leading type-marker member so that enum / annotation / interface / abstract
    // classes each get a visually distinct header (fix #769).
    let builtin_type_marker = node
        .members
        .first()
        .and_then(|m| builtin_type_stereotype_label(&m.text));
    let header_fill = match node.kind {
        FamilyNodeKind::Class => match builtin_type_marker {
            Some("\u{ab}enumeration\u{bb}") => "#ffffcc", // lemon — PlantUML enum convention
            Some("\u{ab}annotation\u{bb}") => "#fff0cc",  // warm amber for @annotation
            Some("\u{ab}interface\u{bb}") => "#dae8fc",   // light blue for interface
            Some("\u{ab}abstract\u{bb}") => "#f0e6ff",    // light lavender for abstract
            _ => scoped_style
                .and_then(|style| style.header_color.as_deref())
                .unwrap_or(class_style.header_color.as_str()),
        },
        FamilyNodeKind::Object => "#fef3c7",
        FamilyNodeKind::Map => "#fef3c7",
        FamilyNodeKind::UseCase | FamilyNodeKind::BusinessUseCase => "#dcfce7",
        _ => "#f1f5f9",
    };

    if matches!(node.kind, FamilyNodeKind::Diamond) {
        let cx = x + w / 2;
        let cy = y + h / 2;
        let r = (w.min(h) / 2).saturating_sub(3).max(12);
        out.push_str(&format!(
            "<polygon class=\"uml-node uml-diamond\" data-uml-kind=\"diamond\" data-uml-id=\"{}\" points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            escape_text(node.alias.as_deref().unwrap_or(&node.name)),
            cx,
            cy - r,
            cx + r,
            cy,
            cx,
            cy + r,
            cx - r,
            cy,
            fill,
            stroke
        ));
        return;
    }

    if matches!(
        node.kind,
        FamilyNodeKind::Actor | FamilyNodeKind::BusinessActor
    ) {
        let cx = x + w / 2;
        let fig_cy = y + 21;
        if matches!(node.kind, FamilyNodeKind::BusinessActor) {
            out.push_str(&format!(
                "<rect class=\"uml-business-actor\" data-uml-kind=\"business-actor\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"10\" ry=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                fill,
                stroke,
                stroke_width,
                stroke_dash
            ));
        }
        match class_style.actor_style {
            ActorStyle::Stick => {
                // Canonical stick-figure rendering for actors (issue #715).
                // Proportions are shared with the sequence renderer via render_actor_stick_figure.
                render_actor_stick_figure(out, cx, fig_cy, stroke);
            }
            ActorStyle::Awesome => render_actor_awesome_figure(out, cx, fig_cy, stroke),
            ActorStyle::Hollow => render_actor_hollow_figure(out, cx, fig_cy, stroke),
        }
        let name_y = match class_style.actor_style {
            // Stick-figure feet end at fig_cy + 23; keep the historical 4px gap.
            ActorStyle::Stick => fig_cy + 27,
            // The alternative PlantUML actor glyphs are bulkier silhouettes.
            ActorStyle::Awesome | ActorStyle::Hollow => fig_cy + 42,
        };
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{name_y}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" font-weight=\"600\" fill=\"{}\">{name}</text>",
            escape_text(font_family),
            title_font_size,
            escape_text(font_color),
            name = escape_text(&node.name)
        ));
        // Stereotype / extra members below name
        let mut member_y = name_y + 14;
        for member in &node.members {
            let text = member.text.trim();
            if text.is_empty() || is_family_style_member(text) {
                continue;
            }
            if hide_stereotype && is_user_stereotype(text) {
                continue;
            }
            out.push_str(&format!(
                "<text x=\"{cx}\" y=\"{member_y}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"11\" fill=\"{}\">{}</text>",
                escape_text(font_family),
                escape_text(member_color),
                escape_text(text)
            ));
            member_y += 14;
        }
        return;
    }

    if matches!(
        node.kind,
        FamilyNodeKind::UseCase | FamilyNodeKind::BusinessUseCase
    ) {
        let cx = x + w / 2;
        let cy = y + h / 2;
        let rx = w / 2;
        let ry = h / 2;
        if matches!(node.kind, FamilyNodeKind::BusinessUseCase) {
            out.push_str(&format!(
                "<rect class=\"uml-business-usecase\" data-uml-kind=\"business-usecase\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"18\" ry=\"18\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{stroke_dash}/>",
            ));
        } else {
            out.push_str(&format!(
                "<ellipse cx=\"{cx}\" cy=\"{cy}\" rx=\"{rx}\" ry=\"{ry}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{stroke_dash}/>",
            ));
        }
        // Resolve display name: namespace-qualified nodes (e.g. "Package::MP") encode
        // the human-readable label as members[0] when the parser embeds `as DisplayName`
        // inside a group. Detect this by checking that members[0] is plain text (not a
        // UML modifier line) and use it as the displayed label (fix #578).
        let (uc_display_name, uc_member_skip): (&str, usize) = if node.name.contains("::") {
            let first_member_is_label = node.members.first().is_some_and(|m| {
                let t = m.text.trim();
                !t.is_empty()
                    && !t.starts_with("<<")
                    && !t.starts_with('+')
                    && !t.starts_with('-')
                    && !t.starts_with('#')
                    && !t.starts_with('~')
                    && !t.starts_with('{')
                    && !t.starts_with('\x1f')
                    && !t.contains(':')
                    && !t.contains('(')
            });
            if first_member_is_label {
                (node.members[0].text.trim(), 1)
            } else {
                let short = node.name.rsplit("::").next().unwrap_or(&node.name);
                (short, 0)
            }
        } else {
            (node.name.as_str(), 0)
        };
        // Name centered — the alias is the internal id only; do NOT display it (fix #478)
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" font-weight=\"600\" fill=\"{}\">{name}</text>",
            escape_text(font_family),
            title_font_size,
            escape_text(font_color),
            ty = cy + 4,
            name = escape_text(uc_display_name)
        ));
        // Members rendered below the ellipse (rare for usecases), skipping display-label slot
        let mut my = y + h + 14;
        for member in node.members.iter().skip(uc_member_skip) {
            let text = member.text.trim();
            if is_family_style_member(text) || (hide_stereotype && is_user_stereotype(text)) {
                continue;
            }
            out.push_str(&format!(
                "<text x=\"{tx}\" y=\"{my}\" text-anchor=\"middle\" font-family=\"{}\" font-size=\"{}\" fill=\"{mc}\">{m}</text>",
                escape_text(font_family),
                member_font_size,
                tx = x + w / 2,
                mc = member_color,
                m = escape_text(&member.text)
            ));
            my += 14;
        }
        return;
    }

    // Collect all leading header stereotype labels (built-in type markers + user-defined
    // <<…>> markers — fix #470 for built-in types, fix #551 for user stereotypes).
    // These are rendered as guillemet labels in the header, NOT as ordinary member rows.
    let header_skip = count_header_stereotype_members(&node.members);
    // Build the list of guillemet labels to show in the header (top → bottom).
    let mut header_stereotype_labels: Vec<String> = Vec::new();
    if !hide_stereotype {
        for m in &node.members[..header_skip] {
            if let Some(builtin) = builtin_type_stereotype_label(&m.text) {
                header_stereotype_labels.push(builtin.to_string());
            } else if is_user_stereotype(&m.text) {
                // Convert <<foo>> → «foo»
                let inner = m.text.trim_start_matches("<<").trim_end_matches(">>");
                header_stereotype_labels.push(format!("\u{ab}{inner}\u{bb}"));
            }
        }
    }
    // Members to display: skip all header stereotype members
    let display_members = &node.members[header_skip..];

    // Outer rect
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"4\" ry=\"4\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{stroke_dash}/>",
    ));
    // Header band — taller when we display stereotype labels (14px per label — fix #470, #551)
    let stereotype_extra = (header_stereotype_labels.len() as i32) * 14;
    let effective_header_h = header_h + stereotype_extra;
    out.push_str(&format!(
        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{hh}\" rx=\"4\" ry=\"4\" fill=\"{header_fill}\" stroke=\"{stroke}\" stroke-width=\"{stroke_width}\"{stroke_dash}/>",
        hh = effective_header_h
    ));
    // Header separator line
    out.push_str(&format!(
        "<line x1=\"{x}\" y1=\"{ly}\" x2=\"{x2}\" y2=\"{ly}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
        ly = y + effective_header_h,
        x2 = x + w
    ));

    // Render each stereotype label above the class name in the header (fix #470, #551)
    for (i, label) in header_stereotype_labels.iter().enumerate() {
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"10\" fill=\"{fc}\">{lbl}</text>",
            tx = x + w / 2,
            ty = y + 13 + (i as i32) * 14,
            ff = escape_text(font_family),
            fc = escape_text(font_color),
            lbl = escape_text(label)
        ));
    }

    // Header text: class name or object instance label (`name : Type`).
    let header_text = class_node_display_name(node, namespace_separator);
    let class_visibility = class_node_visibility_symbol(node);
    let header_text = class_visibility
        .map(|symbol| format!("{symbol}{header_text}"))
        .unwrap_or(header_text);
    let class_visibility_attr = class_visibility
        .map(uml_visibility_name)
        .map(|name| format!(" data-uml-class-visibility=\"{name}\""))
        .unwrap_or_default();
    // Underline for objects (PlantUML convention — fix #486)
    let text_decoration = if matches!(node.kind, FamilyNodeKind::Object) {
        " text-decoration=\"underline\" text-decoration-thickness=\"1\""
    } else {
        ""
    };
    // Italic name for abstract classes and interfaces (fix #767 — PlantUML UML convention)
    let is_abstract_node = matches!(
        builtin_type_marker,
        Some("\u{ab}abstract\u{bb}") | Some("\u{ab}interface\u{bb}")
    );
    let name_font_style = if is_abstract_node {
        " font-style=\"italic\""
    } else {
        ""
    };
    let name_ty = y + effective_header_h - 9;
    out.push_str(&format!(
        "<text x=\"{tx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"{ff}\" font-size=\"{fs}\" font-weight=\"600\" fill=\"{fc}\"{td}{fi}{cv}>{txt}</text>",
        ff = escape_text(font_family),
        fs = title_font_size,
        fc = escape_text(font_color),
        tx = x + w / 2,
        ty = name_ty,
        td = text_decoration,
        fi = name_font_style,
        cv = class_visibility_attr,
        txt = escape_text(&header_text)
    ));

    if matches!(node.kind, FamilyNodeKind::Map) {
        render_map_rows(
            out,
            node,
            x,
            y,
            w,
            effective_header_h,
            &MapRenderCtx {
                font_family,
                member_font_size,
                member_color,
                stroke,
            },
        );
        return;
    }

    // Members — split by `--` / `..` divider tokens to draw compartment lines (fix #468).
    // We also auto-insert a divider between the last attribute and the first operation
    // when there is no explicit divider in the source (fix #468 second compartment).
    //
    // Pre-scan: detect whether there are both attributes and operations in display_members
    // so we know to auto-insert a divider at the transition boundary.
    let has_explicit_divider = display_members
        .iter()
        .any(|m| m.text.trim() == "--" || m.text.trim() == "..");
    let auto_divider = if !has_explicit_divider {
        // Determine the index of the first operation (text containing '(') after at least one attribute.
        let mut first_op_idx: Option<usize> = None;
        let mut seen_attr = false;
        for (i, m) in display_members.iter().enumerate() {
            let t = m.text.trim();
            if t == "--" || t == ".." || t.is_empty() {
                continue;
            }
            // Strip visibility prefix before checking for '('
            let (_vis, _col, rest) = parse_visibility_member(t);
            if rest.contains('(') {
                if seen_attr {
                    first_op_idx = Some(i);
                }
                break;
            } else {
                seen_attr = true;
            }
        }
        first_op_idx
    } else {
        None
    };

    let mut my = y + effective_header_h + 16;
    let mut section_started = false; // tracks if we've seen at least one non-divider member
    for (midx, member) in display_members.iter().enumerate() {
        let raw_text = member.text.trim();
        if is_family_style_member(raw_text) {
            continue;
        }
        // Auto-insert divider before the first operation when no explicit divider exists (fix #468)
        if auto_divider == Some(midx) {
            let div_y = my - 8;
            out.push_str(&format!(
                "<line x1=\"{x}\" y1=\"{div_y}\" x2=\"{x2}\" y2=\"{div_y}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
                x2 = x + w
            ));
            section_started = false;
        }
        // Detect explicit divider tokens (`--` or `..` compartment separator)
        if raw_text == "--" || raw_text == ".." {
            // Draw a horizontal divider line (fix #468)
            let div_y = my - 8;
            out.push_str(&format!(
                "<line x1=\"{x}\" y1=\"{div_y}\" x2=\"{x2}\" y2=\"{div_y}\" stroke=\"{stroke}\" stroke-width=\"1\"/>",
                x2 = x + w
            ));
            section_started = false;
            continue;
        }
        // Skip blank display lines
        if raw_text.is_empty() {
            my += 16;
            continue;
        }
        let _ = section_started;
        section_started = true;
        let (vis_sym, vis_color, rest_after_vis) = parse_visibility_member(raw_text);
        let (base_style, text_after_mod) = parse_member_modifiers(rest_after_vis);
        let mut style_attrs = String::from(base_style);
        match &member.modifier {
            Some(MemberModifier::Abstract) | Some(MemberModifier::Field) => {
                if !style_attrs.contains("font-style") {
                    style_attrs.push_str(" font-style=\"italic\"");
                }
            }
            Some(MemberModifier::Static) => {
                if !style_attrs.contains("text-decoration") {
                    style_attrs.push_str(" text-decoration=\"underline\"");
                }
            }
            Some(MemberModifier::Method) | None => {
                // Interface members are implicitly abstract — render in italic (fix #767)
                if is_abstract_node && !style_attrs.contains("font-style") {
                    style_attrs.push_str(" font-style=\"italic\"");
                }
            }
        }
        let render_visibility_icons = class_style.attribute_icons;
        // If no explicit visibility color, fall back to member_color from style.
        let effective_color = if vis_sym.is_some() && render_visibility_icons {
            vis_color
        } else {
            member_color
        };
        // Reconstruct display text: keep visibility prefix + remaining text
        let display_text = if vis_sym.is_some() {
            format!("{}{}", vis_sym.unwrap_or(""), text_after_mod)
        } else {
            text_after_mod.to_string()
        };
        let visibility_attr = if render_visibility_icons {
            vis_sym
                .map(uml_visibility_name)
                .map(|name| format!(" data-uml-visibility=\"{name}\""))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let modifier_attr = member_modifier_name(member.modifier.as_ref())
            .map(|name| format!(" data-uml-modifier=\"{name}\""))
            .unwrap_or_default();
        if let Some(required_text) = display_text.strip_prefix('*') {
            out.push_str(&format!(
                "<text class=\"uml-member uml-ie-member\" data-uml-ie-mandatory=\"true\"{visibility_attr}{modifier_attr} x=\"{tx}\" y=\"{my}\" font-family=\"{ff}\" font-size=\"{fs}\" fill=\"{vc}\"{sa}>\
                 <tspan font-weight=\"700\">*</tspan><tspan dx=\"4\">{m}</tspan></text>",
                ff = escape_text(font_family),
                fs = member_font_size,
                tx = x + 10,
                vc = effective_color,
                sa = style_attrs,
                m = escape_text(required_text.trim_start())
            ));
        } else {
            if display_text.contains("<$") {
                out.push_str(&creole_text(
                    x + 10,
                    my,
                    &format!(
                        "class=\"uml-member\"{visibility_attr}{modifier_attr} font-family=\"{}\" font-size=\"{}\" fill=\"{}\"{}",
                        escape_text(font_family),
                        member_font_size,
                        effective_color,
                        style_attrs
                    ),
                    &display_text,
                    effective_color,
                ));
            } else {
                out.push_str(&format!(
                    "<text class=\"uml-member\"{visibility_attr}{modifier_attr} x=\"{tx}\" y=\"{my}\" font-family=\"{ff}\" font-size=\"{fs}\" fill=\"{vc}\"{sa}>{m}</text>",
                    ff = escape_text(font_family),
                    fs = member_font_size,
                    tx = x + 10,
                    vc = effective_color,
                    sa = style_attrs,
                    m = escape_text(&display_text)
                ));
            }
        }
        my += 16;
    }
}

fn render_actor_awesome_figure(out: &mut String, cx: i32, cy: i32, stroke: &str) {
    let head_cy = cy - 15;
    out.push_str(&format!(
        "<circle class=\"uml-actor-glyph\" data-uml-actor-style=\"awesome\" cx=\"{cx}\" cy=\"{head_cy}\" r=\"7\" fill=\"{stroke}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
    ));
    let shoulder_y = head_cy + 11;
    let body_top = head_cy + 16;
    let body_bottom = head_cy + 37;
    out.push_str(&format!(
        "<path class=\"uml-actor-glyph\" data-uml-actor-style=\"awesome\" d=\"M{} {} Q{} {} {} {} L{} {} Q{} {} {} {} L{} {} Q{} {} {} {} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        cx - 15,
        body_bottom,
        cx - 13,
        shoulder_y,
        cx,
        body_top,
        cx + 15,
        body_bottom,
        cx + 8,
        body_bottom + 4,
        cx,
        body_bottom + 4,
        cx - 8,
        body_bottom + 4,
        cx - 15,
        body_bottom,
        cx - 15,
        body_bottom,
        stroke,
        stroke
    ));
}

fn render_actor_hollow_figure(out: &mut String, cx: i32, cy: i32, stroke: &str) {
    let head_cy = cy - 15;
    out.push_str(&format!(
        "<circle class=\"uml-actor-glyph\" data-uml-actor-style=\"hollow\" cx=\"{cx}\" cy=\"{head_cy}\" r=\"7\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.8\"/>"
    ));
    let shoulder_y = head_cy + 11;
    let body_bottom = head_cy + 38;
    out.push_str(&format!(
        "<path class=\"uml-actor-glyph\" data-uml-actor-style=\"hollow\" d=\"M{} {} Q{} {} {} {} Q{} {} {} {} Q{} {} {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>",
        cx - 16,
        body_bottom,
        cx - 13,
        shoulder_y,
        cx,
        shoulder_y,
        cx + 13,
        shoulder_y,
        cx + 16,
        body_bottom,
        cx,
        body_bottom + 6,
        cx - 16,
        body_bottom,
        stroke
    ));
}

fn render_family_node_shape(
    out: &mut String,
    node: &FamilyNode,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    hide_stereotype: bool,
) {
    let cx = x + w / 2;
    let cy = y + h / 2;
    let display = node.label.clone().unwrap_or_else(|| node.name.clone());
    let kind_label = family_node_label(node.kind);
    out.push_str(&format!(
        "<desc data-uml-id=\"{}\">{}</desc>",
        escape_text(&node.name),
        escape_text(&node.name)
    ));

    match node.kind {
        FamilyNodeKind::Interface => {
            // small circle interface
            let r = 18;
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#f1f5f9\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                cx, cy, r
            ));
        }
        FamilyNodeKind::Component => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"#fefce8\" stroke=\"#a16207\" stroke-width=\"1\"/>",
                x, y, w, h
            ));
            // component badges (two small rectangles on the left edge)
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"8\" fill=\"#fefce8\" stroke=\"#a16207\" stroke-width=\"1\"/>",
                x - 4,
                y + 12
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"16\" height=\"8\" fill=\"#fefce8\" stroke=\"#a16207\" stroke-width=\"1\"/>",
                x - 4,
                y + h - 20
            ));
        }
        FamilyNodeKind::Node | FamilyNodeKind::Frame => {
            // 3D cube: top face (parallelogram) + right face + front face (fix #571)
            let offset = 12i32;
            // Top face
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"#d4dff7\" stroke=\"#3730a3\" stroke-width=\"1\"/>",
                x, y,
                x + offset, y - offset,
                x + w + offset, y - offset,
                x + w, y
            ));
            // Right face
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"#b8c8ef\" stroke=\"#3730a3\" stroke-width=\"1\"/>",
                x + w, y,
                x + w + offset, y - offset,
                x + w + offset, y + h - offset,
                x + w, y + h
            ));
            // Front face
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#eef2ff\" stroke=\"#3730a3\" stroke-width=\"1.5\"/>",
                x, y, w, h
            ));
        }
        FamilyNodeKind::Cloud => {
            // cloud-ish: rounded with several arcs
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"#f0f9ff\" stroke=\"#0369a1\" stroke-width=\"1.5\"/>",
                cx,
                cy,
                w / 2 - 4,
                h / 2 - 4
            ));
        }
        FamilyNodeKind::Database => {
            // database cylinder
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"8\" fill=\"#ecfeff\" stroke=\"#0e7490\" stroke-width=\"1.5\"/>",
                cx,
                y + 10,
                w / 2 - 6
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#ecfeff\" stroke=\"#0e7490\" stroke-width=\"1.5\"/>",
                x + 6,
                y + 10,
                w - 12,
                h - 20
            ));
            out.push_str(&format!(
                "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"8\" fill=\"#ecfeff\" stroke=\"#0e7490\" stroke-width=\"1.5\"/>",
                cx,
                y + h - 10,
                w / 2 - 6
            ));
        }
        FamilyNodeKind::Artifact | FamilyNodeKind::File => {
            // folded-corner rectangle
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{} {},{}\" fill=\"#fff7ed\" stroke=\"#9a3412\" stroke-width=\"1.5\"/>",
                x,
                y,
                x + w - 18,
                y,
                x + w,
                y + 18,
                x + w,
                y + h,
                x,
                y + h
            ));
        }
        FamilyNodeKind::Folder | FamilyNodeKind::Package => {
            let fill = node.fill_color.as_deref().unwrap_or("#fef3c7");
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"60\" height=\"14\" fill=\"{}\" stroke=\"#92400e\" stroke-width=\"1\"/>",
                x, y, fill
            ));
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"#92400e\" stroke-width=\"1\"/>",
                x,
                y + 14,
                w,
                h - 14,
                fill
            ));
        }
        FamilyNodeKind::Storage => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"16\" ry=\"16\" fill=\"#fff1f2\" stroke=\"#9f1239\" stroke-width=\"1.5\"/>",
                x, y, w, h
            ));
        }
        FamilyNodeKind::Rectangle
        | FamilyNodeKind::Card
        | FamilyNodeKind::Actor
        | FamilyNodeKind::Port => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"#f8fafc\" stroke=\"#475569\" stroke-width=\"1\"/>",
                x, y, w, h
            ));
        }
        FamilyNodeKind::State => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"14\" ry=\"14\" fill=\"#ecfccb\" stroke=\"#3f6212\" stroke-width=\"1.5\"/>",
                x, y, w, h
            ));
        }
        FamilyNodeKind::StateInitial => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"#0f172a\"/>",
                cx, cy
            ));
        }
        FamilyNodeKind::StateFinal => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"#ffffff\" stroke=\"#0f172a\" stroke-width=\"1.5\"/>",
                cx, cy
            ));
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"6\" fill=\"#0f172a\"/>",
                cx, cy
            ));
        }
        FamilyNodeKind::StateHistory => {
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"14\" fill=\"#fef3c7\" stroke=\"#92400e\" stroke-width=\"1.5\"/>",
                cx, cy
            ));
        }
        FamilyNodeKind::Note => {
            render_note_card(out, x, y, w, h, &display);
            return;
        }
        FamilyNodeKind::Class | FamilyNodeKind::Object | FamilyNodeKind::UseCase => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"#f1f5f9\" stroke=\"#64748b\" stroke-width=\"1\"/>",
                x, y, w, h
            ));
        }
        _ => {
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"4\" ry=\"4\" fill=\"#f8fafc\" stroke=\"#64748b\" stroke-width=\"1\"/>",
                x, y, w, h
            ));
        }
    }

    // For interface/initial/final we render label below the marker.
    let (label_x, label_y) = match node.kind {
        FamilyNodeKind::Interface
        | FamilyNodeKind::StateInitial
        | FamilyNodeKind::StateFinal
        | FamilyNodeKind::StateHistory => (cx, cy + 28),
        _ => (cx, cy + 6),
    };
    let label_last_y =
        render_centered_multiline_text(out, label_x, label_y, 13, "600", None, &display);
    let kind_tag_y = match node.kind {
        FamilyNodeKind::Interface
        | FamilyNodeKind::StateInitial
        | FamilyNodeKind::StateFinal
        | FamilyNodeKind::StateHistory => label_last_y + 14,
        _ => y + 14,
    };
    // Suppress the kind-tag for package/rectangle/folder container nodes — they already
    // show their label in a visual header/tab (fix #549).
    let is_package_container = matches!(
        node.kind,
        FamilyNodeKind::Package | FamilyNodeKind::Rectangle | FamilyNodeKind::Folder
    );
    if !is_package_container && !hide_stereotype {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">{}</text>",
            cx,
            kind_tag_y,
            kind_label
        ));
    }
    if !hide_stereotype {
        render_node_stereotype_rows(out, node, cx, kind_tag_y + 13);
    }
}

fn render_node_stereotype_rows(out: &mut String, node: &FamilyNode, cx: i32, start_y: i32) {
    for (idx, member) in node
        .members
        .iter()
        .filter(|member| {
            let text = member.text.trim();
            text.starts_with("<<") && text.ends_with(">>")
        })
        .take(4)
        .enumerate()
    {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#64748b\">{}</text>",
            cx,
            start_y + idx as i32 * 12,
            escape_text(member.text.trim())
        ));
    }
}

pub(crate) fn render_note_card(out: &mut String, x: i32, y: i32, w: i32, h: i32, text: &str) {
    out.push_str(&format!(
        "<path d=\"M{x},{y} H{} L{} {} V{} H{x} Z\" fill=\"#fff8c4\" stroke=\"#8a6d00\" stroke-width=\"1.2\"/>",
        x + w - 16,
        x + w,
        y + 16,
        y + h
    ));
    out.push_str(&format!(
        "<path d=\"M{} {y} V{} H{}\" fill=\"none\" stroke=\"#8a6d00\" stroke-width=\"1\"/>",
        x + w - 16,
        y + 16,
        x + w
    ));
    let mut ty = y + 22;
    for line in text.lines().take(5) {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#3b2f00\">{}</text>",
            x + 10,
            ty,
            escape_text(line)
        ));
        ty += 15;
    }
}
