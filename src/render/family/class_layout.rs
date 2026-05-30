use crate::model::{FamilyDocument, FamilyNode, FamilyNodeKind};

use super::class_types::{ClassCanvasMetrics, ClassNodeBox};
use super::group_frames::{class_group_frame_rect, collect_render_group_frames, RenderGroupFrame};
use super::projections::class_projection_extra_height;

#[allow(clippy::too_many_arguments)]
pub(super) fn class_run_layout(
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
    //
    // For deeply-nested packages (#1287) a leaf class belongs to multiple
    // ancestor frames simultaneously; use the DEEPEST scope so the layout
    // groups siblings within their innermost frame instead of letting a
    // distant outer frame's bbox swallow them.  Frames are sorted ascending
    // by depth, so we overwrite earlier (shallower) parents as we go.
    let group_frames_for_gl = collect_render_group_frames(&document.groups);
    let mut node_to_gl_group_depth: std::collections::BTreeMap<String, (usize, String)> =
        std::collections::BTreeMap::new();
    for frame in &group_frames_for_gl {
        for mid in &frame.member_ids {
            node_to_gl_group_depth
                .entry(mid.clone())
                .and_modify(|prev| {
                    if frame.depth > prev.0 {
                        *prev = (frame.depth, frame.scope.clone());
                    }
                })
                .or_insert_with(|| (frame.depth, frame.scope.clone()));
        }
    }
    let node_to_gl_group: std::collections::BTreeMap<String, String> = node_to_gl_group_depth
        .into_iter()
        .map(|(k, (_d, scope))| (k, scope))
        .collect();

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
            label: rel.label.clone(),
        })
        .collect();

    let is_usecase = is_real_usecase_layout(document);
    let rank_separation = if is_usecase {
        // Retune for PlantUML-equivalent density (#1359): rank_separation =
        // max_node_height + row_gap ensures adjacent ranks never overlap
        // (the layout engine requires gap > 8px between rank bottom and next
        // rank top).  row_gap=20 gives ~20px inter-rank clearance.
        let max_node_h = node_heights.iter().map(|(_, h)| *h).max().unwrap_or(60) as f64;
        max_node_h + row_gap as f64
    } else {
        (row_gap + node_heights.iter().map(|(_, h)| *h).max().unwrap_or(60)) as f64
    };
    let gl_options = GlOptions {
        rank_separation,
        node_separation: col_gap as f64,
        group_padding: if is_usecase { 8.0 } else { 16.0 },
        direction: crate::render::graph_layout::Direction::TopDown,
        canvas_margin: (margin_top + title_block_height + group_top_reserve) as f64,
        // Right-side gutter only needs margin_x (32px); canvas_margin also absorbs
        // title height and group-label tabs which are only needed vertically.
        canvas_right_margin: Some(margin_x as f64),
        // Usecase groups (system boundaries) can stack vertically instead of
        // spreading horizontally when groups collide at different vertical positions.
        stack_staggered_group_collisions: false,
        // Skip group collision resolution for usecase: multi-rank groups cause
        // excessive horizontal spread when collision resolver pushes them apart.
        // PlantUML allows boundary groups to overlap at edges (#1359).
        skip_group_collision_resolution: is_usecase,
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
        gl_result.rebuild_scene(&gl_nodes, &gl_edges);
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
#[allow(clippy::too_many_arguments)] // 10 distinct canvas metrics; a struct would add churn without clarity
pub(super) fn class_compute_canvas(
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

pub(super) fn class_node_width(kind: FamilyNodeKind, default_width: i32) -> i32 {
    match kind {
        FamilyNodeKind::Diamond => 44,
        _ => default_width,
    }
}

pub(super) fn class_node_display_name(
    node: &FamilyNode,
    namespace_separator: Option<&str>,
) -> String {
    let raw_name = node.label.as_deref().unwrap_or(&node.name);
    namespace_separator
        .filter(|sep| !sep.is_empty())
        .map(|sep| raw_name.replace("::", sep))
        .unwrap_or_else(|| raw_name.to_string())
}

pub(super) fn is_real_usecase_layout(document: &FamilyDocument) -> bool {
    if !matches!(document.kind, crate::ast::DiagramKind::UseCase) {
        return false;
    }
    // A usecase diagram is "real" (uses actor stick-figures and oval shapes)
    // when every node is an Actor/UseCase/Note kind.  We intentionally do NOT
    // check members: UseCase nodes commonly store a display-label member (e.g.
    // "Browse Catalog" for `usecase "Browse Catalog" as UC1`), which is valid
    // in usecase diagrams.
    document.nodes.iter().all(|node| {
        matches!(
            node.kind,
            FamilyNodeKind::UseCase
                | FamilyNodeKind::BusinessUseCase
                | FamilyNodeKind::Actor
                | FamilyNodeKind::BusinessActor
                | FamilyNodeKind::Person
                | FamilyNodeKind::Note
        )
    })
}
