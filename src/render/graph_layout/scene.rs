use super::{EdgeSpec, NodeSize};
use crate::render::layout_constants::PKG_TAB_HEIGHT;
use crate::render_core::{
    Anchor, GroupFrame, LabelBox, LabelRole, NodeBox, Point, Polyline, Port, PortSide, Rect,
    RenderScene, SceneEdge, SceneGroup, SceneNode,
};
use std::collections::BTreeMap;

pub(super) fn build_render_scene(
    nodes: &[NodeSize],
    edges: &[EdgeSpec],
    node_positions: &BTreeMap<String, (f64, f64)>,
    edge_paths: &BTreeMap<String, Vec<(f64, f64)>>,
    group_bounds: &BTreeMap<String, (f64, f64, f64, f64)>,
    canvas_width: f64,
    canvas_height: f64,
) -> RenderScene {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, canvas_width, canvas_height));
    let node_by_id: BTreeMap<&str, &NodeSize> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut children_by_group: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for node in nodes {
        if let Some(parent) = &node.parent {
            children_by_group
                .entry(parent.clone())
                .or_default()
                .push(node.id.clone());
        }
        let Some(&(x, y)) = node_positions.get(&node.id) else {
            continue;
        };
        let bounds = Rect::new(x, y, node.width, node.height);
        let label = centered_label(
            format!("node:{}:label", node.id),
            node.id.clone(),
            bounds,
            Some(node.id.clone()),
            LabelRole::Node,
        );
        let node_box = NodeBox {
            id: node.id.clone(),
            bounds,
            ports: default_node_ports(&node.id, bounds),
            labels: vec![label],
        };
        scene.add_node(SceneNode {
            id: node.id.clone(),
            node_box,
        });
    }

    for (group_id, &(x, y, width, height)) in group_bounds {
        let bounds = Rect::new(x, y, width, height);
        let header = Some(Rect::new(x, y, width, (PKG_TAB_HEIGHT as f64).min(height)));
        let mut child_node_ids = children_by_group.remove(group_id).unwrap_or_default();
        child_node_ids.sort();
        let labels = vec![LabelBox {
            id: format!("group:{group_id}:label"),
            text: group_id.clone(),
            bounds: Rect::new(
                x + 8.0,
                y + 4.0,
                (group_id.chars().count() as f64 * 7.0 + 8.0).max(12.0),
                14.0,
            ),
            owner_id: Some(group_id.clone()),
            role: LabelRole::Group,
        }];
        scene.add_group(SceneGroup {
            id: group_id.clone(),
            frame: GroupFrame {
                id: group_id.clone(),
                bounds,
                header,
                child_node_ids,
                labels,
            },
        });
    }

    for edge in edges {
        let points = edge_paths
            .get(&edge.id)
            .map(|path| {
                path.iter()
                    .map(|(x, y)| Point::new(*x, *y))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let route = Polyline::new(points);
        let source_point = route.first().unwrap_or_else(|| {
            node_rect(&edge.from, &node_by_id, node_positions)
                .map(Rect::center)
                .unwrap_or(Point::new(0.0, 0.0))
        });
        let target_point = route.last().unwrap_or_else(|| {
            node_rect(&edge.to, &node_by_id, node_positions)
                .map(Rect::center)
                .unwrap_or(Point::new(0.0, 0.0))
        });
        let source_rect = node_rect(&edge.from, &node_by_id, node_positions);
        let target_rect = node_rect(&edge.to, &node_by_id, node_positions);
        scene.add_edge(SceneEdge {
            id: edge.id.clone(),
            from: edge.from.clone(),
            to: edge.to.clone(),
            route,
            source_anchor: anchor_for_endpoint(
                &edge.id,
                "source",
                &edge.from,
                source_point,
                source_rect,
            ),
            target_anchor: anchor_for_endpoint(
                &edge.id,
                "target",
                &edge.to,
                target_point,
                target_rect,
            ),
            labels: Vec::new(),
        });
    }

    scene.fit_viewport_to_visible_bounds();
    scene
}

fn node_rect(
    node_id: &str,
    node_by_id: &BTreeMap<&str, &NodeSize>,
    node_positions: &BTreeMap<String, (f64, f64)>,
) -> Option<Rect> {
    let node = node_by_id.get(node_id)?;
    let &(x, y) = node_positions.get(node_id)?;
    Some(Rect::new(x, y, node.width, node.height))
}

fn centered_label(
    id: String,
    text: String,
    owner_bounds: Rect,
    owner_id: Option<String>,
    role: LabelRole,
) -> LabelBox {
    let width = (text.chars().count() as f64 * 7.0).max(8.0);
    let height = 14.0;
    let center = owner_bounds.center();
    LabelBox {
        id,
        text,
        bounds: Rect::new(
            center.x - width / 2.0,
            center.y - height / 2.0,
            width,
            height,
        ),
        owner_id,
        role,
    }
}

fn default_node_ports(node_id: &str, bounds: Rect) -> Vec<Port> {
    vec![
        port(
            node_id,
            PortSide::Top,
            Point::new(bounds.center().x, bounds.min_y()),
        ),
        port(
            node_id,
            PortSide::Right,
            Point::new(bounds.max_x(), bounds.center().y),
        ),
        port(
            node_id,
            PortSide::Bottom,
            Point::new(bounds.center().x, bounds.max_y()),
        ),
        port(
            node_id,
            PortSide::Left,
            Point::new(bounds.min_x(), bounds.center().y),
        ),
        port(node_id, PortSide::Center, bounds.center()),
    ]
}

fn port(node_id: &str, side: PortSide, position: Point) -> Port {
    Port {
        id: format!("{node_id}:{}", port_side_name(side)),
        node_id: node_id.to_string(),
        side,
        position,
    }
}

fn anchor_for_endpoint(
    edge_id: &str,
    role: &str,
    node_id: &str,
    position: Point,
    node_bounds: Option<Rect>,
) -> Anchor {
    let matched_port = node_bounds.and_then(|bounds| {
        default_node_ports(node_id, bounds)
            .into_iter()
            .find(|candidate| candidate.position.distance_to(position) <= 0.5)
    });
    Anchor {
        id: format!("{edge_id}:{role}"),
        owner_id: node_id.to_string(),
        position,
        port: matched_port,
    }
}

fn port_side_name(side: PortSide) -> &'static str {
    match side {
        PortSide::Top => "top",
        PortSide::Right => "right",
        PortSide::Bottom => "bottom",
        PortSide::Left => "left",
        PortSide::Center => "center",
    }
}

// ─────────────────────────────────────────────────────────────────────────────
