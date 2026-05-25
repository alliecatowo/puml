use super::*;
use crate::render_core::{
    Anchor, GroupFrame, LabelBox, LabelRole, LaneFrame, NodeBox, Point, Polyline, Port, PortSide,
    Rect, RenderScene, RouteChannel, SceneEdge, SceneGroup, SceneNode,
};
use std::collections::BTreeMap;

pub(super) fn build_nwdiag_scene(
    width: i32,
    height: i32,
    node_rects: &BTreeMap<String, Vec<NodeRect>>,
    overlays: &[GroupOverlay],
    lanes: &[NetworkLaneGeom],
    peer_routes: &[PeerRouteGeom],
) -> RenderScene {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, width as f64, height as f64));

    for lane in lanes {
        scene.add_lane(LaneFrame {
            id: lane.id.clone(),
            bounds: rect_from_tuple(lane.bounds),
            header: Some(rect_from_tuple(lane.bus)),
            child_node_ids: nodes_in_lane(node_rects, &lane.id),
            labels: vec![LabelBox {
                id: format!("{}:label", lane.id),
                text: lane.label.clone(),
                bounds: Rect::new(
                    lane.bounds.0 as f64 + 8.0,
                    lane.bounds.1 as f64 + 4.0,
                    text_box_width(&lane.label, 13.0),
                    16.0,
                ),
                owner_id: Some(lane.id.clone()),
                role: LabelRole::Lane,
            }],
        });
        scene.route_channels.insert(
            format!("{}:bus", lane.id),
            RouteChannel::new(format!("{}:bus", lane.id), rect_from_tuple(lane.bus)),
        );
    }

    for (name, rects) in node_rects {
        let Some(rect) = rects.iter().find(|rect| rect.physical) else {
            continue;
        };
        let bounds = Rect::new(rect.x as f64, rect.y as f64, rect.w as f64, rect.h as f64);
        scene.add_node(SceneNode {
            id: name.clone(),
            node_box: NodeBox {
                id: name.clone(),
                bounds,
                ports: node_ports(name, bounds, rects),
                labels: vec![LabelBox {
                    id: format!("nwdiag:node:{name}:label"),
                    text: name.clone(),
                    bounds: centered_label_bounds(bounds, name, 12.0),
                    owner_id: Some(name.clone()),
                    role: LabelRole::Node,
                }],
            },
        });
    }

    for overlay in overlays {
        let bounds = Rect::new(
            overlay.x as f64,
            overlay.y as f64,
            overlay.w as f64,
            overlay.h as f64,
        );
        scene.add_group(SceneGroup {
            id: overlay.id.clone(),
            frame: GroupFrame {
                id: overlay.id.clone(),
                bounds,
                header: Some(Rect::new(
                    overlay.x as f64,
                    overlay.y as f64,
                    overlay.w as f64,
                    30.0,
                )),
                child_node_ids: overlay.child_node_ids.clone(),
                labels: vec![LabelBox {
                    id: format!("{}:label", overlay.id),
                    text: overlay.label.clone(),
                    bounds: Rect::new(
                        overlay.x as f64 + 4.0,
                        overlay.y as f64 + 5.0,
                        text_box_width(&format!("group {}", overlay.label), 10.0),
                        14.0,
                    ),
                    owner_id: Some(overlay.id.clone()),
                    role: LabelRole::Group,
                }],
            },
        });
    }

    for route in peer_routes {
        scene.route_channels.insert(
            format!("nwdiag:peer-channel:{}", route.id),
            RouteChannel::new(
                format!("nwdiag:peer-channel:{}", route.id),
                route_channel_bounds(&route.path),
            )
            .with_graph_channel_metadata(0, 0, 0.0, vec![route.id.clone()], Vec::new()),
        );
        let points = route
            .path
            .iter()
            .map(|(x, y)| Point::new(*x as f64, *y as f64))
            .collect::<Vec<_>>();
        let polyline = Polyline::new(points);
        let source = polyline.first().unwrap_or(Point::new(0.0, 0.0));
        let target = polyline.last().unwrap_or(Point::new(0.0, 0.0));
        scene.add_edge(SceneEdge {
            id: route.id.clone(),
            from: route.from.clone(),
            to: route.to.clone(),
            route: polyline,
            route_channel_ids: vec![format!("nwdiag:peer-channel:{}", route.id)],
            source_anchor: anchor_for_point(&route.id, "source", &route.from, source, &scene),
            target_anchor: anchor_for_point(&route.id, "target", &route.to, target, &scene),
            labels: Vec::new(),
        });
    }

    scene
}

fn nodes_in_lane(node_rects: &BTreeMap<String, Vec<NodeRect>>, lane_id: &str) -> Vec<String> {
    let network = lane_id.strip_prefix("nwdiag:network:").unwrap_or(lane_id);
    let mut nodes = Vec::new();
    for (name, rects) in node_rects {
        if rects
            .iter()
            .any(|rect| rect.network.as_deref() == Some(network))
        {
            nodes.push(name.clone());
        }
    }
    nodes
}

fn rect_from_tuple((x, y, w, h): (i32, i32, i32, i32)) -> Rect {
    Rect::new(x as f64, y as f64, w as f64, h as f64)
}

fn node_ports(node_id: &str, bounds: Rect, rects: &[NodeRect]) -> Vec<Port> {
    let mut ports = vec![
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
    ];
    for rect in rects.iter().filter(|rect| !rect.physical) {
        let center_x = rect.x as f64 + rect.w as f64 / 2.0;
        let scope = rect.network.as_deref().unwrap_or("shared");
        ports.push(port_with_id(
            format!("nwdiag:node:{node_id}:{scope}:top"),
            node_id,
            PortSide::Top,
            Point::new(center_x, rect.y as f64),
        ));
        ports.push(port_with_id(
            format!("nwdiag:node:{node_id}:{scope}:bottom"),
            node_id,
            PortSide::Bottom,
            Point::new(center_x, (rect.y + rect.h) as f64),
        ));
    }
    ports
}

fn port(node_id: &str, side: PortSide, position: Point) -> Port {
    port_with_id(
        format!("nwdiag:node:{node_id}:{}", port_side_name(side)),
        node_id,
        side,
        position,
    )
}

fn port_with_id(id: String, node_id: &str, side: PortSide, position: Point) -> Port {
    Port {
        id,
        node_id: node_id.to_string(),
        side,
        position,
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

fn centered_label_bounds(bounds: Rect, text: &str, font_size: f64) -> Rect {
    let width = text_box_width(text, font_size);
    let center = bounds.center();
    Rect::new(center.x - width / 2.0, center.y - 7.0, width, 14.0)
}

fn text_box_width(text: &str, font_size: f64) -> f64 {
    (text.chars().count() as f64 * font_size * 0.62).max(8.0)
}

fn route_channel_bounds(path: &[(i32, i32)]) -> Rect {
    let Some((first_x, first_y)) = path.first().copied() else {
        return Rect::new(0.0, 0.0, 0.0, 0.0);
    };
    let mut min_x = first_x;
    let mut max_x = first_x;
    let mut min_y = first_y;
    let mut max_y = first_y;
    for &(x, y) in path {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }
    Rect::new(
        min_x as f64,
        min_y as f64 - 4.0,
        (max_x - min_x).max(1) as f64,
        (max_y - min_y).max(1) as f64 + 8.0,
    )
}

fn anchor_for_point(
    edge_id: &str,
    role: &str,
    node_id: &str,
    position: Point,
    scene: &RenderScene,
) -> Anchor {
    let port = scene.nodes.get(node_id).and_then(|node| {
        node.node_box
            .ports
            .iter()
            .find(|port| port.position.distance_to(position) <= 0.5)
            .cloned()
    });
    Anchor {
        id: format!("{edge_id}:{role}"),
        owner_id: node_id.to_string(),
        position,
        port,
    }
}
