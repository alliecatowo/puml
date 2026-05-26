use std::collections::BTreeMap;

use crate::model::{ChenAttribute, ChenDocument, ChenNode, ChenNodeKind};
use crate::output::RenderArtifact;
use crate::render::graph_layout::{layout_hierarchical, EdgeSpec, LayoutOptions, NodeSize};
use crate::render::svg::escape_text;
use crate::render_core::{
    Anchor, LabelBox, LabelRole, NodeBox, Point, Polyline, Rect, RenderScene, SceneEdge, SceneNode,
};

#[derive(Debug, Clone, Copy)]
enum ChenShape {
    Entity,
    Relationship,
    Attribute,
}

#[derive(Debug, Clone)]
struct RenderNode {
    id: String,
    label: String,
    detail: Option<String>,
    shape: ChenShape,
    weak: bool,
    identifying: bool,
    key: bool,
    derived: bool,
    multivalued: bool,
    width: f64,
    height: f64,
}

pub fn render_chen_svg(document: &ChenDocument) -> String {
    render_chen_artifact(document).svg
}

/// Render a Chen ER diagram into a typed [`RenderArtifact`].
///
/// The SVG is still emitted directly (Chen draws bespoke entity/relationship/
/// attribute shapes and straight border-to-border edges), but we also build a
/// [`RenderScene`] from the *actual* drawn geometry — node boxes at their laid-out
/// positions and edges along the same anchor lines the SVG uses — so the scene
/// stays consistent with the output (no box_grid-style scene/SVG drift). Output
/// is byte-identical to the legacy `render_chen_svg`; the scene is attached for
/// the typed-geometry validation path.
pub fn render_chen_artifact(document: &ChenDocument) -> RenderArtifact {
    let mut render_nodes = Vec::new();
    let mut edges = Vec::new();

    for node in &document.nodes {
        push_chen_node(&mut render_nodes, node);
        for attr in &node.attributes {
            push_chen_attribute(&mut render_nodes, &mut edges, &node.id, attr);
        }
    }

    for (idx, rel) in document.relations.iter().enumerate() {
        edges.push(EdgeSpec {
            id: format!("rel{idx}"),
            from: rel.from.clone(),
            to: rel.to.clone(),
            label: None,
        });
    }
    for (idx, inheritance) in document.inheritances.iter().enumerate() {
        let set_id = format!("chen_set_{idx}");
        let label = inheritance
            .discriminator
            .clone()
            .unwrap_or_else(|| inheritance.connector.clone());
        render_nodes.push(RenderNode {
            id: set_id.clone(),
            label,
            detail: Some("EER".to_string()),
            shape: ChenShape::Relationship,
            weak: false,
            identifying: false,
            key: false,
            derived: false,
            multivalued: false,
            width: 96.0,
            height: 62.0,
        });
        edges.push(EdgeSpec {
            id: format!("inh{idx}_parent"),
            from: inheritance.parent.clone(),
            to: set_id.clone(),
            label: None,
        });
        for (child_idx, child) in inheritance.children.iter().enumerate() {
            edges.push(EdgeSpec {
                id: format!("inh{idx}_child{child_idx}"),
                from: set_id.clone(),
                to: child.clone(),
                label: None,
            });
        }
    }

    let node_sizes = render_nodes
        .iter()
        .map(|node| NodeSize {
            id: node.id.clone(),
            width: node.width,
            height: node.height,
            parent: None,
        })
        .collect::<Vec<_>>();
    let layout = layout_hierarchical(
        &node_sizes,
        &edges,
        &LayoutOptions {
            rank_separation: 74.0,
            node_separation: 80.0,
            canvas_margin: 34.0 + title_height(document),
            canvas_right_margin: Some(34.0),
            ..LayoutOptions::default()
        },
    );
    let node_by_id = render_nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let pos = &layout.node_positions;
    let mut width = layout.canvas_width.ceil() as i32 + 36;
    let mut height = layout.canvas_height.ceil() as i32 + 40;
    if width < 280 {
        width = 280;
    }
    if height < 160 {
        height = 160;
    }

    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\" data-diagram-family=\"chen\" data-orientation=\"{}\">",
        document.orientation.as_str()
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>");

    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"34\" y=\"28\" font-family=\"monospace\" font-size=\"18\" font-weight=\"700\" fill=\"#111827\">{}</text>",
            escape_text(title)
        ));
    }

    for rel in &document.relations {
        render_edge(
            &mut out,
            pos,
            &node_by_id,
            &rel.from,
            &rel.to,
            Some(&rel.cardinality),
            rel.total_participation,
        );
    }
    for edge in edges.iter().filter(|edge| edge.id.starts_with("attr")) {
        render_edge(
            &mut out,
            pos,
            &node_by_id,
            &edge.from,
            &edge.to,
            None,
            false,
        );
    }
    for edge in edges.iter().filter(|edge| edge.id.starts_with("inh")) {
        render_edge(
            &mut out,
            pos,
            &node_by_id,
            &edge.from,
            &edge.to,
            None,
            false,
        );
    }

    for node in &render_nodes {
        if let Some(&(x, y)) = pos.get(&node.id) {
            render_node(&mut out, node, x, y);
        }
    }

    if let Some(caption) = &document.caption {
        out.push_str(&format!(
            "<text x=\"34\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">{}</text>",
            height - 18,
            escape_text(caption)
        ));
    }
    if let Some(legend) = &document.legend {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" text-anchor=\"end\" font-family=\"monospace\" font-size=\"12\" fill=\"#475569\">{}</text>",
            width - 34,
            height - 18,
            escape_text(legend)
        ));
    }
    out.push_str("</svg>");

    let scene = build_chen_scene(
        &render_nodes,
        &node_by_id,
        pos,
        &edges,
        document,
        width as f64,
        height as f64,
    );
    RenderArtifact::with_scene(out, scene)
}

/// Build a typed [`RenderScene`] from Chen's laid-out geometry. Node boxes use
/// the same positions/sizes the SVG draws; edge routes use the same
/// `anchor_line` border-to-border segments, so scene and SVG never diverge.
fn build_chen_scene(
    render_nodes: &[RenderNode],
    node_by_id: &BTreeMap<&str, &RenderNode>,
    pos: &BTreeMap<String, (f64, f64)>,
    edges: &[EdgeSpec],
    document: &ChenDocument,
    width: f64,
    height: f64,
) -> RenderScene {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, width, height));

    for node in render_nodes {
        if let Some(&(x, y)) = pos.get(&node.id) {
            let bounds = Rect::new(x, y, node.width, node.height);
            let label = LabelBox {
                id: format!("{}::label", node.id),
                text: node.label.clone(),
                bounds,
                owner_id: Some(node.id.clone()),
                role: LabelRole::Node,
            };
            scene.add_node(SceneNode {
                id: node.id.clone(),
                node_box: NodeBox {
                    id: node.id.clone(),
                    bounds,
                    ports: Vec::new(),
                    labels: vec![label],
                },
            });
        }
    }

    for (idx, rel) in document.relations.iter().enumerate() {
        push_scene_edge(&mut scene, pos, node_by_id, format!("rel{idx}"), &rel.from, &rel.to);
    }
    for edge in edges.iter().filter(|edge| edge.id.starts_with("attr")) {
        push_scene_edge(&mut scene, pos, node_by_id, edge.id.clone(), &edge.from, &edge.to);
    }
    for edge in edges.iter().filter(|edge| edge.id.starts_with("inh")) {
        push_scene_edge(&mut scene, pos, node_by_id, edge.id.clone(), &edge.from, &edge.to);
    }

    scene
}

fn push_scene_edge(
    scene: &mut RenderScene,
    pos: &BTreeMap<String, (f64, f64)>,
    node_by_id: &BTreeMap<&str, &RenderNode>,
    id: String,
    from: &str,
    to: &str,
) {
    let (Some(src_rect), Some(tgt_rect)) =
        (node_rect(pos, node_by_id, from), node_rect(pos, node_by_id, to))
    else {
        return;
    };
    let (x1, y1, x2, y2) = anchor_line(src_rect, tgt_rect);
    let source_anchor = Anchor {
        id: format!("{id}::src"),
        owner_id: from.to_string(),
        position: Point::new(x1, y1),
        port: None,
    };
    let target_anchor = Anchor {
        id: format!("{id}::tgt"),
        owner_id: to.to_string(),
        position: Point::new(x2, y2),
        port: None,
    };
    scene.add_edge(SceneEdge {
        id,
        from: from.to_string(),
        to: to.to_string(),
        route: Polyline::from_tuples(&[(x1, y1), (x2, y2)]),
        route_channel_ids: Vec::new(),
        source_anchor,
        target_anchor,
        labels: Vec::new(),
    });
}

fn push_chen_node(render_nodes: &mut Vec<RenderNode>, node: &ChenNode) {
    let label_width = (node.label.chars().count() as f64 * 8.0 + 36.0).max(136.0);
    render_nodes.push(RenderNode {
        id: node.id.clone(),
        label: node.label.clone(),
        detail: None,
        shape: match node.kind {
            ChenNodeKind::Entity => ChenShape::Entity,
            ChenNodeKind::Relationship => ChenShape::Relationship,
        },
        weak: node.weak,
        identifying: node.identifying,
        key: false,
        derived: false,
        multivalued: false,
        width: label_width,
        height: if node.kind == ChenNodeKind::Relationship {
            76.0
        } else {
            58.0
        },
    });
}

fn push_chen_attribute(
    render_nodes: &mut Vec<RenderNode>,
    edges: &mut Vec<EdgeSpec>,
    owner: &str,
    attr: &ChenAttribute,
) {
    let id = format!("{owner}::{}", attr.id);
    let label_width = (attr.label.chars().count() as f64 * 8.0 + 42.0).max(108.0);
    render_nodes.push(RenderNode {
        id: id.clone(),
        label: attr.label.clone(),
        detail: attr.data_type.clone(),
        shape: ChenShape::Attribute,
        weak: false,
        identifying: false,
        key: attr.key,
        derived: attr.derived,
        multivalued: attr.multivalued,
        width: label_width,
        height: 48.0,
    });
    edges.push(EdgeSpec {
        id: format!("attr_{}_{}", edges.len(), sanitize_edge_id(&id)),
        from: owner.to_string(),
        to: id.clone(),
        label: None,
    });
    for child in &attr.children {
        push_chen_attribute(render_nodes, edges, &id, child);
    }
}

fn render_node(out: &mut String, node: &RenderNode, x: f64, y: f64) {
    match node.shape {
        ChenShape::Entity => {
            render_entity(out, node, x, y);
        }
        ChenShape::Relationship => {
            render_relationship(out, node, x, y);
        }
        ChenShape::Attribute => {
            render_attribute(out, node, x, y);
        }
    }
}

fn render_entity(out: &mut String, node: &RenderNode, x: f64, y: f64) {
    out.push_str(&format!(
        "<rect class=\"chen-entity\" x=\"{x:.1}\" y=\"{y:.1}\" width=\"{w:.1}\" height=\"{h:.1}\" fill=\"#f8fafc\" stroke=\"#111827\" stroke-width=\"1.5\"/>",
        w = node.width,
        h = node.height
    ));
    if node.weak {
        out.push_str(&format!(
            "<rect class=\"chen-weak-entity\" x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"#111827\" stroke-width=\"1\"/>",
            x + 5.0,
            y + 5.0,
            node.width - 10.0,
            node.height - 10.0
        ));
    }
    render_center_label(out, node, x, y, "#111827");
}

fn render_relationship(out: &mut String, node: &RenderNode, x: f64, y: f64) {
    let cx = x + node.width / 2.0;
    let cy = y + node.height / 2.0;
    let points = diamond_points(cx, cy, node.width / 2.0, node.height / 2.0);
    out.push_str(&format!(
        "<polygon class=\"chen-relationship\" points=\"{}\" fill=\"#ecfeff\" stroke=\"#155e75\" stroke-width=\"1.5\"/>",
        points
    ));
    if node.identifying {
        out.push_str(&format!(
            "<polygon class=\"chen-identifying\" points=\"{}\" fill=\"none\" stroke=\"#155e75\" stroke-width=\"1\"/>",
            diamond_points(cx, cy, node.width / 2.0 - 6.0, node.height / 2.0 - 6.0)
        ));
    }
    render_center_label(out, node, x, y, "#164e63");
}

fn render_attribute(out: &mut String, node: &RenderNode, x: f64, y: f64) {
    let dash = if node.derived {
        " stroke-dasharray=\"5 4\""
    } else {
        ""
    };
    out.push_str(&format!(
        "<ellipse class=\"chen-attribute\" cx=\"{cx:.1}\" cy=\"{cy:.1}\" rx=\"{rx:.1}\" ry=\"{ry:.1}\" fill=\"#fff7ed\" stroke=\"#9a3412\" stroke-width=\"1.3\"{dash}/>",
        cx = x + node.width / 2.0,
        cy = y + node.height / 2.0,
        rx = node.width / 2.0,
        ry = node.height / 2.0
    ));
    if node.multivalued {
        out.push_str(&format!(
            "<ellipse class=\"chen-multivalued\" cx=\"{cx:.1}\" cy=\"{cy:.1}\" rx=\"{rx:.1}\" ry=\"{ry:.1}\" fill=\"none\" stroke=\"#9a3412\" stroke-width=\"1\"/>",
            cx = x + node.width / 2.0,
            cy = y + node.height / 2.0,
            rx = node.width / 2.0 - 5.0,
            ry = node.height / 2.0 - 5.0
        ));
    }
    render_center_label(out, node, x, y, "#7c2d12");
    if node.key {
        let text_w = node.label.chars().count() as f64 * 7.2;
        let cx = x + node.width / 2.0;
        out.push_str(&format!(
            "<line class=\"chen-key-underline\" x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"#7c2d12\" stroke-width=\"1\"/>",
            cx - text_w / 2.0,
            y + node.height / 2.0 + 6.0,
            cx + text_w / 2.0,
            y + node.height / 2.0 + 6.0
        ));
    }
}

fn render_center_label(out: &mut String, node: &RenderNode, x: f64, y: f64, color: &str) {
    let label_y = if node.detail.is_some() {
        y + node.height / 2.0 - 2.0
    } else {
        y + node.height / 2.0 + 5.0
    };
    out.push_str(&format!(
        "<text x=\"{:.1}\" y=\"{label_y:.1}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" fill=\"{color}\">{}</text>",
        x + node.width / 2.0,
        escape_text(&node.label)
    ));
    if let Some(detail) = &node.detail {
        out.push_str(&format!(
            "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"10\" fill=\"#64748b\">{}</text>",
            x + node.width / 2.0,
            y + node.height / 2.0 + 13.0,
            escape_text(detail)
        ));
    }
}

fn render_edge(
    out: &mut String,
    pos: &BTreeMap<String, (f64, f64)>,
    nodes: &BTreeMap<&str, &RenderNode>,
    from: &str,
    to: &str,
    label: Option<&str>,
    total_participation: bool,
) {
    let Some((sx, sy, sw, sh)) = node_rect(pos, nodes, from) else {
        return;
    };
    let Some((tx, ty, tw, th)) = node_rect(pos, nodes, to) else {
        return;
    };
    let (x1, y1, x2, y2) = anchor_line((sx, sy, sw, sh), (tx, ty, tw, th));
    let stroke_width = if total_participation { 3.0 } else { 1.4 };
    out.push_str(&format!(
        "<line class=\"chen-edge\" x1=\"{x1:.1}\" y1=\"{y1:.1}\" x2=\"{x2:.1}\" y2=\"{y2:.1}\" stroke=\"#334155\" stroke-width=\"{stroke_width:.1}\"/>"
    ));
    if let Some(label) = label {
        let mx = (x1 + x2) / 2.0;
        let my = (y1 + y2) / 2.0 - 5.0;
        let label_w = label.chars().count() as f64 * 7.0 + 10.0;
        out.push_str(&format!(
            "<rect class=\"chen-cardinality-bg\" x=\"{:.1}\" y=\"{:.1}\" width=\"{label_w:.1}\" height=\"16\" rx=\"3\" fill=\"#ffffff\"/>",
            mx - label_w / 2.0,
            my - 11.0
        ));
        out.push_str(&format!(
            "<text class=\"chen-cardinality\" x=\"{mx:.1}\" y=\"{my:.1}\" text-anchor=\"middle\" font-family=\"monospace\" font-size=\"12\" fill=\"#0f172a\">{}</text>",
            escape_text(label)
        ));
    }
}

fn node_rect(
    pos: &BTreeMap<String, (f64, f64)>,
    nodes: &BTreeMap<&str, &RenderNode>,
    id: &str,
) -> Option<(f64, f64, f64, f64)> {
    let node = nodes.get(id)?;
    let (x, y) = *pos.get(id)?;
    Some((x, y, node.width, node.height))
}

fn anchor_line(
    (sx, sy, sw, sh): (f64, f64, f64, f64),
    (tx, ty, tw, th): (f64, f64, f64, f64),
) -> (f64, f64, f64, f64) {
    let scx = sx + sw / 2.0;
    let scy = sy + sh / 2.0;
    let tcx = tx + tw / 2.0;
    let tcy = ty + th / 2.0;
    if (tcx - scx).abs() > (tcy - scy).abs() {
        if tcx >= scx {
            (sx + sw, scy, tx, tcy)
        } else {
            (sx, scy, tx + tw, tcy)
        }
    } else if tcy >= scy {
        (scx, sy + sh, tcx, ty)
    } else {
        (scx, sy, tcx, ty + th)
    }
}

fn diamond_points(cx: f64, cy: f64, rx: f64, ry: f64) -> String {
    format!(
        "{cx:.1},{:.1} {:.1},{cy:.1} {cx:.1},{:.1} {:.1},{cy:.1}",
        cy - ry,
        cx + rx,
        cy + ry,
        cx - rx
    )
}

fn title_height(document: &ChenDocument) -> f64 {
    if document.title.is_some() {
        24.0
    } else {
        0.0
    }
}

fn sanitize_edge_id(input: &str) -> String {
    input
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}
