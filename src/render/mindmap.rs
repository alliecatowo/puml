use super::{escape_text, FamilyDocument, MindMapSide};
use crate::output::RenderArtifact;
use crate::render_core::{
    Anchor, LabelBox, LabelRole, NodeBox, Point, Polyline, Rect, RenderScene, SceneEdge, SceneNode,
};

const MINDMAP_CHAR_PX: i32 = 7;
const MINDMAP_NODE_PAD_X: i32 = 20;

mod labels;
mod nodes;
mod style;
mod tree;
mod wbs;

pub use wbs::{render_wbs_artifact, render_wbs_svg};

use labels::{multiline_char_width, prepare_mindmap_label, render_mindmap_node_label};
use nodes::{draw_mindmap_subtree, mindmap_empty_svg};
use style::mindmap_node_fill_resolved;
use style::{mindmap_node_border_color, mindmap_node_font_color, mindmap_style};
use tree::{assign_y_positions, family_tree_child_indices};
use wbs::wbs_orientation_attr;

pub fn render_mindmap_svg(doc: &FamilyDocument) -> String {
    render_mindmap_artifact(doc).svg
}

/// Render a `@startmindmap` document into a typed [`RenderArtifact`].
///
/// The SVG is emitted unchanged (byte-identical to the legacy `render_mindmap_svg`).
/// A [`RenderScene`] is built from the same laid-out geometry the SVG draws — each
/// node box at its computed `(x, y, w, h)`, and each parent→child connector along
/// the same line segment — so the scene and SVG stay in sync.
pub fn render_mindmap_artifact(doc: &FamilyDocument) -> RenderArtifact {
    const X_STEP: i32 = 180;
    const Y_STEP: i32 = 48;
    const NODE_H: i32 = 34;
    const MARGIN: i32 = 24;
    const NODE_PAD_X: i32 = 10;

    let maximum_width = doc.maximum_width;
    let style = mindmap_style(doc);
    let display_names: Vec<String> = doc
        .nodes
        .iter()
        .map(|node| prepare_mindmap_label(&node.name, maximum_width))
        .collect();

    // Separate nodes into root, left-side, right-side subtrees.
    // Depth 0 = root. Depth 1+ inherit side from their nearest depth-1 ancestor.
    let nodes = &doc.nodes;
    if nodes.is_empty() {
        return RenderArtifact::svg_only(mindmap_empty_svg(doc));
    }

    // Build parent indices and side assignments.
    let n = nodes.len();
    let mut side = vec![MindMapSide::Right; n];
    let mut parent: Vec<Option<usize>> = vec![None; n];
    {
        let mut stack: Vec<usize> = Vec::new();
        for i in 0..n {
            let depth = nodes[i].depth;
            while stack.len() > depth {
                stack.pop();
            }
            if let Some(&p) = stack.last() {
                parent[i] = Some(p);
            }
            // Side: use the node's own side if depth >= 1
            if depth == 0 {
                side[i] = MindMapSide::Right; // root — not rendered as left/right
            } else if depth == 1 {
                side[i] = nodes[i].mindmap_side;
            } else if let Some(p) = parent[i] {
                side[i] = side[p];
            }
            stack.push(i);
        }
    }

    // Auto-balance: if all depth-1 nodes are Right (no explicit side markers),
    // distribute them evenly left/right for a balanced mindmap (#430/#532).
    // We assign the first half to Left (alternating from the last node upward so
    // the first child stays on the right per convention).
    let has_explicit_left =
        (0..n).any(|i| nodes[i].depth == 1 && nodes[i].mindmap_side == MindMapSide::Left);
    if !has_explicit_left {
        let depth1_indices: Vec<usize> = (0..n).filter(|&i| nodes[i].depth == 1).collect();
        let total = depth1_indices.len();
        if total > 1 {
            // Assign the bottom half (by index order) to Left, keeping the top half Right.
            let left_count = total / 2;
            for (rank, &node_idx) in depth1_indices.iter().enumerate() {
                if rank >= total - left_count {
                    // Bottom `left_count` children go left; propagate to their descendants.
                    let mut j = node_idx;
                    while j < n {
                        if j != node_idx && nodes[j].depth <= nodes[node_idx].depth {
                            break;
                        }
                        side[j] = MindMapSide::Left;
                        j += 1;
                    }
                }
            }
        }
    }

    // Collect left/right subtrees at depth 1+.
    let right_roots: Vec<usize> = (0..n)
        .filter(|&i| nodes[i].depth == 1 && side[i] == MindMapSide::Right)
        .collect();
    let left_roots: Vec<usize> = (0..n)
        .filter(|&i| nodes[i].depth == 1 && side[i] == MindMapSide::Left)
        .collect();

    // For each depth-1 subtree, compute total height = number of descendants + self.
    fn subtree_leaf_count(nodes: &[crate::model::FamilyNode], idx: usize) -> usize {
        let depth = nodes[idx].depth;
        let children_count: usize = (idx + 1..nodes.len())
            .take_while(|&j| nodes[j].depth > depth)
            .filter(|&j| nodes[j].depth == depth + 1)
            .count();
        if children_count == 0 {
            return 1;
        }
        let mut total = 0usize;
        let mut j = idx + 1;
        while j < nodes.len() && nodes[j].depth > depth {
            if nodes[j].depth == depth + 1 {
                total += subtree_leaf_count(nodes, j);
            }
            j += 1;
        }
        total
    }

    // Assign y positions for right-side depth-1 nodes.
    let total_right_leaves: usize = right_roots
        .iter()
        .map(|&i| subtree_leaf_count(nodes, i))
        .sum();
    let total_left_leaves: usize = left_roots
        .iter()
        .map(|&i| subtree_leaf_count(nodes, i))
        .sum();
    let max_leaves = total_right_leaves.max(total_left_leaves).max(1);
    let canvas_h = (max_leaves as i32) * Y_STEP + 2 * MARGIN + NODE_H;

    // Max text width for nodes — simple heuristic.
    fn node_width(name: &str, maximum_width: Option<i32>) -> i32 {
        let chars = multiline_char_width(name);
        let heuristic = chars * MINDMAP_CHAR_PX + MINDMAP_NODE_PAD_X;
        match maximum_width.filter(|w| *w > 0) {
            Some(max_px) => heuristic.clamp(70, max_px),
            None => heuristic.clamp(70, 220),
        }
    }

    let root_w = node_width(&display_names[0], maximum_width);
    let max_right_depth = (0..n)
        .filter(|&i| side[i] == MindMapSide::Right && nodes[i].depth >= 1)
        .map(|i| nodes[i].depth)
        .max()
        .unwrap_or(0);
    let max_left_depth = (0..n)
        .filter(|&i| side[i] == MindMapSide::Left && nodes[i].depth >= 1)
        .map(|i| nodes[i].depth)
        .max()
        .unwrap_or(0);

    let mindmap_leaf_count = (0..n)
        .filter(|&idx| family_tree_child_indices(nodes, idx).is_empty())
        .count();
    let max_mindmap_depth = max_right_depth.max(max_left_depth);
    let x_step = if max_mindmap_depth >= 4 && mindmap_leaf_count >= 12 {
        130
    } else {
        X_STEP
    };

    let root_cy = canvas_h / 2;

    // Draw nodes recursively — track y-cursors per side.
    // We assign y by a preorder traversal respecting leaf count.
    let mut y_positions = vec![0i32; n];
    {
        // Right side
        let mut y_cursor = root_cy - (total_right_leaves as i32 * Y_STEP) / 2 + Y_STEP / 2;
        assign_y_positions(nodes, &right_roots, &mut y_positions, &mut y_cursor, Y_STEP);
        // Left side
        y_cursor = root_cy - (total_left_leaves as i32 * Y_STEP) / 2 + Y_STEP / 2;
        assign_y_positions(nodes, &left_roots, &mut y_positions, &mut y_cursor, Y_STEP);
    }

    #[allow(clippy::too_many_arguments)]
    fn subtree_bounds(
        nodes: &[crate::model::FamilyNode],
        display_names: &[String],
        idx: usize,
        node_x_center: i32,
        x_step: i32,
        node_pad_x: i32,
        is_left: bool,
        maximum_width: Option<i32>,
    ) -> (i32, i32) {
        let nw = node_width(&display_names[idx], maximum_width);
        let nx = if is_left {
            node_x_center - nw
        } else {
            node_x_center
        };
        let children = family_tree_child_indices(nodes, idx);
        let next_x_center = if is_left {
            node_x_center - x_step
        } else {
            node_x_center + x_step + nw - node_pad_x
        };
        children
            .iter()
            .fold((nx, nx + nw), |(acc_min, acc_max), &c| {
                let (child_min, child_max) = subtree_bounds(
                    nodes,
                    display_names,
                    c,
                    next_x_center,
                    x_step,
                    node_pad_x,
                    is_left,
                    maximum_width,
                );
                (acc_min.min(child_min), acc_max.max(child_max))
            })
    }

    let depth_bias = (max_left_depth as i32) * x_step + 240;
    let root_cx_prelim = MARGIN + depth_bias + root_w / 2;
    let root_min_x = root_cx_prelim - root_w / 2;
    let root_max_x = root_cx_prelim + root_w / 2;
    let actual_max_right = {
        let right_start = root_cx_prelim + root_w / 2 + x_step - NODE_PAD_X;
        right_roots
            .iter()
            .map(|&i| {
                subtree_bounds(
                    nodes,
                    &display_names,
                    i,
                    right_start,
                    x_step,
                    NODE_PAD_X,
                    false,
                    maximum_width,
                )
                .1
            })
            .max()
            .unwrap_or(root_max_x)
    };
    let actual_min_left = {
        let left_start = root_cx_prelim - root_w / 2 - x_step + NODE_PAD_X;
        left_roots
            .iter()
            .map(|&i| {
                subtree_bounds(
                    nodes,
                    &display_names,
                    i,
                    left_start,
                    x_step,
                    NODE_PAD_X,
                    true,
                    maximum_width,
                )
                .0
            })
            .min()
            .unwrap_or(root_min_x)
    };
    let actual_min_x = root_min_x.min(actual_min_left);
    let actual_max_x = root_max_x.max(actual_max_right);
    let extra_left = (MARGIN - actual_min_x).max(0);
    let canvas_w = actual_max_x + extra_left + MARGIN;
    let root_cx = root_cx_prelim + extra_left;

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" data-mindmap-orientation=\"{orientation}\" data-mindmap-node-count=\"{node_count}\" data-mindmap-leaf-count=\"{leaf_count}\" data-mindmap-max-depth=\"{max_depth}\">",
        w = canvas_w,
        h = canvas_h,
        orientation = wbs_orientation_attr(doc.orientation),
        node_count = n,
        leaf_count = mindmap_leaf_count,
        max_depth = max_mindmap_depth
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");

    // Title
    let mut ty = MARGIN;
    if let Some(title) = &doc.title {
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"{cx}\" y=\"{ty}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\">{txt}</text>",
                cx = root_cx,
                ty = ty,
                txt = escape_text(line)
            ));
            ty += 20;
        }
    }

    // Draw root node
    let rx = root_cx - root_w / 2;
    let ry = root_cy - NODE_H / 2;
    let root_fill = mindmap_node_fill_resolved(&nodes[0], style);
    let root_border = mindmap_node_border_color(0, style, "#92400e");
    let root_font_color = mindmap_node_font_color(0, style, "#111827");
    out.push_str(&format!(
        "<rect class=\"mindmap-node mindmap-root mindmap-branch\" data-mindmap-depth=\"0\" data-mindmap-child-count=\"{child_count}\" data-mindmap-fill=\"{fill}\" x=\"{rx}\" y=\"{ry}\" width=\"{rw}\" height=\"{h}\" rx=\"17\" ry=\"17\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>",
        rx = rx, ry = ry, rw = root_w, h = NODE_H,
        child_count = family_tree_child_indices(nodes, 0).len(),
        fill = escape_text(&root_fill),
        stroke = escape_text(root_border)
    ));
    out.push_str(&render_mindmap_node_label(
        root_cx,
        root_cy,
        &display_names[0],
        13,
        "monospace",
        "600",
        root_font_color,
    ));

    // Draw right-side branches.
    for &i in &right_roots {
        draw_mindmap_subtree(
            &mut out,
            nodes,
            &display_names,
            i,
            root_cx + root_w / 2,
            root_cy,
            root_cx + root_w / 2 + x_step - NODE_PAD_X,
            &y_positions,
            x_step,
            NODE_H,
            NODE_PAD_X,
            false, // left=false → right
            maximum_width,
            style,
        );
    }
    // Draw left-side branches.
    for &i in &left_roots {
        draw_mindmap_subtree(
            &mut out,
            nodes,
            &display_names,
            i,
            root_cx - root_w / 2,
            root_cy,
            root_cx - root_w / 2 - x_step + NODE_PAD_X,
            &y_positions,
            x_step,
            NODE_H,
            NODE_PAD_X,
            true, // left=true
            maximum_width,
            style,
        );
    }

    // Caption
    if let Some(caption) = &doc.caption {
        out.push_str(&format!(
            "<text x=\"{cx}\" y=\"{cy}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"11\" fill=\"#475569\">{}</text>",
            escape_text(caption),
            cx = canvas_w / 2,
            cy = canvas_h - 8
        ));
    }
    // Legend
    if let Some(legend) = &doc.legend {
        let lx = canvas_w - 160;
        let ly = MARGIN + 10;
        out.push_str(&format!(
            "<rect x=\"{lx}\" y=\"{ly}\" width=\"140\" height=\"50\" rx=\"4\" ry=\"4\" fill=\"#f9fafb\" stroke=\"#94a3b8\" stroke-width=\"1\"/>",
            lx = lx, ly = ly
        ));
        out.push_str(&format!(
            "<text x=\"{tx}\" y=\"{ty}\" font-family=\"monospace\" font-size=\"10\" fill=\"#475569\">{}</text>",
            escape_text(legend),
            tx = lx + 8,
            ty = ly + 18
        ));
    }

    out.push_str("</svg>");

    let scene = build_mindmap_scene(
        nodes,
        &display_names,
        &parent,
        &side,
        &y_positions,
        &right_roots,
        &left_roots,
        root_cx,
        root_cy,
        root_w,
        NODE_H,
        NODE_PAD_X,
        maximum_width,
        x_step,
        canvas_w,
        canvas_h,
    );
    RenderArtifact::with_scene(out, scene)
}

/// Compute the node box `(nx, ny_top, nw, nh)` for a subtree, mirroring
/// the geometry from `draw_mindmap_subtree`. Called at module level so it can
/// use `labels::multiline_char_width` and `labels::multiline_line_count`.
#[allow(clippy::too_many_arguments)]
fn mindmap_node_box(
    display_names: &[String],
    idx: usize,
    node_x_center: i32,
    y_positions: &[i32],
    base_node_h: i32,
    maximum_width: Option<i32>,
    is_left: bool,
) -> (i32, i32, i32, i32) {
    let label = &display_names[idx];
    let ny = y_positions[idx];
    let chars = labels::multiline_char_width(label);
    let heuristic = chars * MINDMAP_CHAR_PX + MINDMAP_NODE_PAD_X;
    let nw = match maximum_width.filter(|w| *w > 0) {
        Some(max_px) => heuristic.clamp(70, max_px),
        None => heuristic.clamp(70, 220),
    };
    let lines = labels::multiline_line_count(label);
    let nh = if lines > 1 {
        (base_node_h + (lines - 1) * 16).min(base_node_h * lines.max(1))
    } else {
        base_node_h
    };
    let nx = if is_left {
        node_x_center - nw
    } else {
        node_x_center
    };
    let ny_top = ny - nh / 2;
    (nx, ny_top, nw, nh)
}

/// Recursively collect node box geometry for a subtree, mirroring
/// `draw_mindmap_subtree`'s layout decisions.
#[allow(clippy::too_many_arguments)]
fn mindmap_collect_subtree_boxes(
    nodes: &[crate::model::FamilyNode],
    display_names: &[String],
    idx: usize,
    node_x_center: i32,
    y_positions: &[i32],
    x_step: i32,
    base_node_h: i32,
    node_pad_x: i32,
    is_left: bool,
    maximum_width: Option<i32>,
    node_boxes: &mut Vec<Option<(i32, i32, i32, i32)>>,
) {
    let (nx, ny_top, nw, nh) = mindmap_node_box(
        display_names,
        idx,
        node_x_center,
        y_positions,
        base_node_h,
        maximum_width,
        is_left,
    );
    node_boxes[idx] = Some((nx, ny_top, nw, nh));

    let depth = nodes[idx].depth;
    let children: Vec<usize> = (idx + 1..nodes.len())
        .take_while(|&j| nodes[j].depth > depth)
        .filter(|&j| nodes[j].depth == depth + 1)
        .collect();

    let next_x_center = if is_left {
        node_x_center - x_step
    } else {
        node_x_center + x_step + nw - node_pad_x
    };
    for &child_idx in &children {
        mindmap_collect_subtree_boxes(
            nodes,
            display_names,
            child_idx,
            next_x_center,
            y_positions,
            x_step,
            base_node_h,
            node_pad_x,
            is_left,
            maximum_width,
            node_boxes,
        );
    }
}

/// Build a typed [`RenderScene`] from the mindmap's laid-out geometry.
///
/// Node boxes mirror the positions/sizes the SVG draws; edge routes follow the
/// same straight line segments the `<line>` elements use, so scene and SVG stay
/// consistent.
#[allow(clippy::too_many_arguments)]
fn build_mindmap_scene(
    nodes: &[crate::model::FamilyNode],
    display_names: &[String],
    parent: &[Option<usize>],
    side: &[MindMapSide],
    y_positions: &[i32],
    right_roots: &[usize],
    left_roots: &[usize],
    root_cx: i32,
    root_cy: i32,
    root_w: i32,
    node_h: i32,
    node_pad_x: i32,
    maximum_width: Option<i32>,
    x_step: i32,
    canvas_w: i32,
    canvas_h: i32,
) -> RenderScene {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, canvas_w as f64, canvas_h as f64));

    let n = nodes.len();
    // Collect node box positions: index → (nx, ny_top, nw, nh)
    let mut node_boxes: Vec<Option<(i32, i32, i32, i32)>> = vec![None; n];

    // Root node box
    if n > 0 {
        let rx = root_cx - root_w / 2;
        let ry = root_cy - node_h / 2;
        node_boxes[0] = Some((rx, ry, root_w, node_h));
    }

    // Subtree boxes — same geometry as the SVG draw path.
    for &i in right_roots {
        let x_center = root_cx + root_w / 2 + x_step - node_pad_x;
        mindmap_collect_subtree_boxes(
            nodes,
            display_names,
            i,
            x_center,
            y_positions,
            x_step,
            node_h,
            node_pad_x,
            false,
            maximum_width,
            &mut node_boxes,
        );
    }
    for &i in left_roots {
        let x_center = root_cx - root_w / 2 - x_step + node_pad_x;
        mindmap_collect_subtree_boxes(
            nodes,
            display_names,
            i,
            x_center,
            y_positions,
            x_step,
            node_h,
            node_pad_x,
            true,
            maximum_width,
            &mut node_boxes,
        );
    }

    // Add SceneNodes from collected boxes.
    for (idx, node) in nodes.iter().enumerate() {
        let id = format!("mm{idx}");
        if let Some((nx, ny_top, nw, nh)) = node_boxes[idx] {
            let bounds = Rect::new(nx as f64, ny_top as f64, nw as f64, nh as f64);
            let label_box = LabelBox {
                id: format!("{id}::label"),
                text: node.name.clone(),
                bounds,
                owner_id: Some(id.clone()),
                role: LabelRole::Node,
            };
            scene.add_node(SceneNode {
                id: id.clone(),
                node_box: NodeBox {
                    id: id.clone(),
                    bounds,
                    ports: Vec::new(),
                    labels: vec![label_box],
                },
            });
        }
    }

    // Add SceneEdges: each parent→child connector mirrors the SVG <line> segment.
    for idx in 0..n {
        let edge_id = format!("mm_edge{idx}");
        let Some(pidx) = parent[idx] else { continue };
        let Some((pnx, pny_top, pnw, pnh)) = node_boxes[pidx] else {
            continue;
        };
        let Some((cnx, cny_top, cnw, cnh)) = node_boxes[idx] else {
            continue;
        };

        let child_is_left = matches!(side[idx], MindMapSide::Left);
        // Parent attach: right edge for right-side children, left edge for left-side.
        let attach_px = if child_is_left { pnx } else { pnx + pnw };
        let attach_py = pny_top + pnh / 2;
        // Child attach: the edge facing toward the parent.
        let attach_cx = if child_is_left { cnx + cnw } else { cnx };
        let attach_cy = cny_top + cnh / 2;

        let from_id = format!("mm{pidx}");
        let to_id = format!("mm{idx}");
        scene.add_edge(SceneEdge {
            id: edge_id.clone(),
            from: from_id.clone(),
            to: to_id.clone(),
            route: Polyline::from_tuples(&[
                (attach_px as f64, attach_py as f64),
                (attach_cx as f64, attach_cy as f64),
            ]),
            route_channel_ids: Vec::new(),
            source_anchor: Anchor {
                id: format!("{edge_id}::src"),
                owner_id: from_id,
                position: Point::new(attach_px as f64, attach_py as f64),
                port: None,
            },
            target_anchor: Anchor {
                id: format!("{edge_id}::tgt"),
                owner_id: to_id,
                position: Point::new(attach_cx as f64, attach_cy as f64),
                port: None,
            },
            labels: Vec::new(),
        });
    }

    scene
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::output::RenderSceneContract;

    /// Helper: parse `@startmindmap` source, render to artifact, return it.
    fn render_mindmap(src: &str) -> RenderArtifact {
        let model = crate::normalize_family(crate::parse(src).unwrap()).unwrap();
        match model {
            crate::model::NormalizedDocument::Family(ref family) => render_mindmap_artifact(family),
            _ => panic!("expected Family document"),
        }
    }

    /// Helper: parse `@startwbs` source, render to artifact, return it.
    fn render_wbs(src: &str) -> crate::output::RenderArtifact {
        use wbs::render_wbs_artifact;
        let model = crate::normalize_family(crate::parse(src).unwrap()).unwrap();
        match model {
            crate::model::NormalizedDocument::Family(ref family) => render_wbs_artifact(family),
            _ => panic!("expected Family document"),
        }
    }

    #[test]
    fn mindmap_artifact_scene_node_count_equals_tree_node_count() {
        // 3-node mindmap: root + 2 children
        let src = "@startmindmap\n* Root\n** Alpha\n** Beta\n@endmindmap\n";
        let artifact = render_mindmap(src);
        let RenderSceneContract::Typed(scene) = artifact.scene_contract() else {
            panic!(
                "expected TypedScene availability, got {:?}",
                artifact.scene_availability
            );
        };
        // Root + Alpha + Beta = 3 nodes
        assert_eq!(
            scene.nodes.len(),
            3,
            "scene should have 3 nodes (root + 2 children), got {}",
            scene.nodes.len()
        );
        // 2 edges (root→Alpha, root→Beta)
        assert_eq!(
            scene.edges.len(),
            2,
            "scene should have 2 edges, got {}",
            scene.edges.len()
        );
        // Geometry validation: no issues (bounds are consistent with the SVG coords)
        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "scene.validate_geometry() returned issues: {:?}",
            issues
        );
    }

    #[test]
    fn mindmap_artifact_svg_is_byte_identical_to_render_svg() {
        let src = "@startmindmap\n* Center\n** Left\n** Right\n@endmindmap\n";
        let model = crate::normalize_family(crate::parse(src).unwrap()).unwrap();
        let crate::model::NormalizedDocument::Family(ref family) = model else {
            panic!("expected Family");
        };
        let svg_via_artifact = render_mindmap_artifact(family).svg;
        let svg_direct = render_mindmap_svg(family);
        assert_eq!(
            svg_via_artifact, svg_direct,
            "render_mindmap_artifact.svg must be byte-identical to render_mindmap_svg"
        );
    }

    #[test]
    fn wbs_artifact_scene_node_count_equals_tree_node_count() {
        // 4-node WBS: root + 3 children at varying depths
        let src = "@startwbs\n* Project\n** Planning\n*** Task A\n** Delivery\n@endwbs\n";
        let artifact = render_wbs(src);
        let RenderSceneContract::Typed(scene) = artifact.scene_contract() else {
            panic!(
                "expected TypedScene availability, got {:?}",
                artifact.scene_availability
            );
        };
        // Project + Planning + Task A + Delivery = 4 nodes
        assert_eq!(
            scene.nodes.len(),
            4,
            "scene should have 4 nodes, got {}",
            scene.nodes.len()
        );
        // 3 edges (parent→child for each non-root)
        assert_eq!(
            scene.edges.len(),
            3,
            "scene should have 3 edges, got {}",
            scene.edges.len()
        );
        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "scene.validate_geometry() returned issues: {:?}",
            issues
        );
    }

    #[test]
    fn wbs_artifact_svg_is_byte_identical_to_render_svg() {
        use wbs::render_wbs_artifact;
        let src = "@startwbs\n* Root\n** A\n** B\n@endwbs\n";
        let model = crate::normalize_family(crate::parse(src).unwrap()).unwrap();
        let crate::model::NormalizedDocument::Family(ref family) = model else {
            panic!("expected Family");
        };
        let svg_via_artifact = render_wbs_artifact(family).svg;
        let svg_direct = render_wbs_svg(family);
        assert_eq!(
            svg_via_artifact, svg_direct,
            "render_wbs_artifact.svg must be byte-identical to render_wbs_svg"
        );
    }
}
