use crate::model::MindMapSide;
use crate::output::RenderArtifact;
use crate::render_core::{
    Anchor, LabelBox, LabelRole, NodeBox, Point, Polyline, Rect, RenderScene, SceneEdge, SceneNode,
};

use super::labels;

/// Compute the node box `(nx, ny_top, nw, nh)` for a subtree, mirroring
/// the geometry from `draw_mindmap_subtree`. Called at module level so it can
/// use `labels::multiline_char_width` and `labels::multiline_line_count`.
#[allow(clippy::too_many_arguments)]
pub(super) fn mindmap_node_box(
    display_names: &[String],
    idx: usize,
    node_x_center: i32,
    y_positions: &[i32],
    base_node_h: i32,
    maximum_width: Option<i32>,
    is_left: bool,
) -> (i32, i32, i32, i32) {
    use super::{MINDMAP_CHAR_PX, MINDMAP_NODE_PAD_X};
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
pub(super) fn mindmap_collect_subtree_boxes(
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
pub(super) fn build_mindmap_scene(
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

/// Build a [`RenderArtifact`] containing both the SVG and the typed scene.
/// This is the entry point called from `mindmap.rs`.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_mindmap_artifact(
    svg: String,
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
) -> RenderArtifact {
    let scene = build_mindmap_scene(
        nodes,
        display_names,
        parent,
        side,
        y_positions,
        right_roots,
        left_roots,
        root_cx,
        root_cy,
        root_w,
        node_h,
        node_pad_x,
        maximum_width,
        x_step,
        canvas_w,
        canvas_h,
    );
    RenderArtifact::with_scene(svg, scene)
}

#[cfg(test)]
mod tests {
    use crate::output::RenderSceneContract;

    use super::super::wbs::{render_wbs_artifact, render_wbs_svg};
    use super::super::{render_mindmap_artifact, render_mindmap_svg};

    /// Helper: parse `@startmindmap` source, render to artifact, return it.
    fn render_mindmap(src: &str) -> crate::output::RenderArtifact {
        let model = crate::normalize_family(crate::parse(src).unwrap()).unwrap();
        match model {
            crate::model::NormalizedDocument::Family(ref family) => render_mindmap_artifact(family),
            _ => panic!("expected Family document"),
        }
    }

    /// Helper: parse `@startwbs` source, render to artifact, return it.
    fn render_wbs(src: &str) -> crate::output::RenderArtifact {
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
