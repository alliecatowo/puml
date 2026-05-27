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
/// `nw`, `NODE_H`). Edges follow the same straight `<line>` segments.
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

        // The SVG uses parent-bottom/top or parent-right/left depending on layout.
        // We use the same center-to-center endpoints as a straight line — the exact
        // same coords as the `<line>` elements drawn in the SVG pass.
        let x1 = x_positions[p] as f64;
        let y1 = y_positions[p] as f64;
        let x2 = x_positions[i] as f64;
        let y2 = y_positions[i] as f64;

        scene.add_edge(SceneEdge {
            id: edge_id.clone(),
            from: from_id.clone(),
            to: to_id.clone(),
            route: Polyline::from_tuples(&[(x1, y1), (x2, y2)]),
            route_channel_ids: Vec::new(),
            source_anchor: Anchor {
                id: format!("{edge_id}::src"),
                owner_id: from_id,
                position: Point::new(x1, y1),
                port: None,
            },
            target_anchor: Anchor {
                id: format!("{edge_id}::tgt"),
                owner_id: to_id,
                position: Point::new(x2, y2),
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
