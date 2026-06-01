use super::super::{escape_text, FamilyDocument};
use crate::output::RenderArtifact;
use crate::render_core::{
    Anchor, LabelBox, LabelRole, NodeBox, Point, Polyline, Rect, RenderScene, SceneEdge, SceneNode,
};

fn wbs_node_width(node: &super::super::FamilyNode) -> i32 {
    (crate::render::text_metrics::default_monospace_width(&node.name) + 24).clamp(80, 200)
}

/// Build a typed [`RenderScene`] from the WBS's laid-out geometry.
///
/// Each node box matches the drawn `<rect>` exactly (`cx - nw/2`, `cy - NODE_H/2`,
/// `nw`, `NODE_H`). Edges follow the same connector geometry as the SVG `<line>`
/// elements — straight for depth ≤ 1, L-shaped for depth ≥ 2 in vstack layout.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_wbs_scene(
    nodes: &[super::super::FamilyNode],
    x_positions: &[i32],
    y_positions: &[i32],
    _children_of: &[Vec<usize>],
    parent_of: &[Option<usize>],
    canvas_w: i32,
    canvas_h: i32,
    node_h: i32,
    use_plantuml_topdown_layout: bool,
) -> RenderScene {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, canvas_w as f64, canvas_h as f64));

    // Add a SceneNode for every WBS node.
    for (idx, node) in nodes.iter().enumerate() {
        let id = format!("wbs{idx}");
        let nw = wbs_node_width(node);
        let cx = x_positions[idx];
        let cy = y_positions[idx];
        let nx = cx - nw / 2;
        let ny = cy - node_h / 2;
        let bounds = Rect::new(nx as f64, ny as f64, nw as f64, node_h as f64);
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

    // Add a SceneEdge for every parent→child pair.
    for (i, &maybe_parent) in parent_of.iter().enumerate() {
        let Some(p) = maybe_parent else { continue };
        let edge_id = format!("wbs_edge{i}");
        let from_id = format!("wbs{p}");
        let to_id = format!("wbs{i}");

        // Mirror the SVG connector geometry so validate_geometry sees the same path
        // the user sees. For vstack layout at depth ≥ 2, the SVG draws an L-shaped
        // connector (vertical drop from parent's left edge, then horizontal to child's
        // left edge). Record that same L-shape in the scene; a straight center-to-center
        // line would cross intermediate nodes and trigger EdgeCrossesNode violations.
        let route = if use_plantuml_topdown_layout && nodes[i].depth >= 2 {
            // Mirror of the depth ≥ 2 arm in wbs.rs: vertical drop then horizontal.
            let parent_w = wbs_node_width(&nodes[p]);
            let child_w = wbs_node_width(&nodes[i]);
            let px = (x_positions[p] - parent_w / 2) as f64; // parent left edge
            let py = (y_positions[p] + node_h / 2) as f64; // parent bottom edge
            let cx = (x_positions[i] - child_w / 2) as f64; // child left edge
            let cy = y_positions[i] as f64; // child center y
            Polyline::from_tuples(&[(px, py), (px, cy), (cx, cy)])
        } else {
            // Root → depth-1 children (Fork-style straight line) and non-vstack layouts.
            let x1 = x_positions[p] as f64;
            let y1 = y_positions[p] as f64;
            let x2 = x_positions[i] as f64;
            let y2 = y_positions[i] as f64;
            Polyline::from_tuples(&[(x1, y1), (x2, y2)])
        };

        let (sx, sy) = route
            .points
            .first()
            .map(|pt| (pt.x, pt.y))
            .unwrap_or((0.0, 0.0));
        let (ex, ey) = route
            .points
            .last()
            .map(|pt| (pt.x, pt.y))
            .unwrap_or((0.0, 0.0));

        scene.add_edge(SceneEdge {
            id: edge_id.clone(),
            from: from_id.clone(),
            to: to_id.clone(),
            route,
            route_channel_ids: Vec::new(),
            source_anchor: Anchor {
                id: format!("{edge_id}::src"),
                owner_id: from_id,
                position: Point::new(sx, sy),
                port: None,
            },
            target_anchor: Anchor {
                id: format!("{edge_id}::tgt"),
                owner_id: to_id,
                position: Point::new(ex, ey),
                port: None,
            },
            labels: Vec::new(),
        });
    }

    scene
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_wbs_artifact(
    svg: String,
    nodes: &[super::super::FamilyNode],
    x_positions: &[i32],
    y_positions: &[i32],
    children_of: &[Vec<usize>],
    parent_of: &[Option<usize>],
    canvas_w: i32,
    canvas_h: i32,
    node_h: i32,
    use_plantuml_topdown_layout: bool,
) -> RenderArtifact {
    let scene = build_wbs_scene(
        nodes,
        x_positions,
        y_positions,
        children_of,
        parent_of,
        canvas_w,
        canvas_h,
        node_h,
        use_plantuml_topdown_layout,
    );
    RenderArtifact::with_scene(svg, scene)
}

pub(super) fn wbs_empty_svg(doc: &FamilyDocument) -> String {
    let mut out = String::new();
    out.push_str("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"300\" height=\"80\" viewBox=\"0 0 300 80\">");
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    if let Some(title) = &doc.title {
        out.push_str(&format!(
            "<text x=\"12\" y=\"28\" font-family=\"monospace\" font-size=\"14\" font-weight=\"600\">{}</text>",
            escape_text(title)
        ));
    }
    out.push_str("<text x=\"12\" y=\"52\" font-family=\"monospace\" font-size=\"12\" fill=\"#64748b\">(empty wbs)</text>");
    out.push_str("</svg>");
    out
}
