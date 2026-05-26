use std::collections::BTreeMap;

use crate::model::{VirtualEndpoint, VirtualEndpointSide};
use crate::render_core::{
    Anchor, GroupFrame, LaneFrame, NodeBox, Point, Polyline, Port, PortSide, Rect, RenderScene,
    SceneEdge, SceneGroup, SceneNode,
};
use crate::scene::{ActivationBox, LifecycleMarker, MessageLine, ParticipantBox, Scene};

mod labels;

use labels::{
    group_header_bounds, group_label_boxes, message_label_boxes, metadata_label_boxes,
    note_label_boxes, participant_label_boxes,
};

const SELF_LOOP_DROP: f64 = 32.0;
const LIFELINE_WIDTH: f64 = 1.0;

pub(super) fn build_render_scene(sequence: &Scene) -> RenderScene {
    let mut builder = SequenceSceneBuilder::new(sequence);
    builder.collect_message_endpoint_ports();
    builder.add_participants();
    builder.add_footboxes();
    builder.add_lifeline_lanes();
    builder.add_activations();
    builder.add_virtual_endpoint_nodes();
    builder.add_lifecycle_markers();
    builder.add_notes();
    builder.add_groups();
    builder.add_metadata_labels();
    builder.add_messages();
    builder.scene
}

struct SequenceSceneBuilder<'a> {
    sequence: &'a Scene,
    scene: RenderScene,
    ports_by_owner: BTreeMap<String, BTreeMap<String, Port>>,
    participant_nodes: BTreeMap<String, String>,
    activation_nodes: Vec<ActivationSceneNode>,
}

#[derive(Debug, Clone)]
struct ActivationSceneNode {
    node_id: String,
    participant_id: String,
    bounds: Rect,
}

#[derive(Debug, Clone)]
struct EndpointNode {
    node_id: String,
    port_id: String,
    side: PortSide,
    position: Point,
}

impl<'a> SequenceSceneBuilder<'a> {
    fn new(sequence: &'a Scene) -> Self {
        Self {
            sequence,
            scene: RenderScene::new(Rect::new(
                0.0,
                0.0,
                f64::from(sequence.width),
                f64::from(sequence.height),
            )),
            ports_by_owner: BTreeMap::new(),
            participant_nodes: sequence
                .participants
                .iter()
                .map(|participant| (participant.id.clone(), participant_node_id(&participant.id)))
                .collect(),
            activation_nodes: Vec::new(),
        }
    }

    fn collect_message_endpoint_ports(&mut self) {
        for activation in &self.sequence.activations {
            self.activation_nodes.push(activation_scene_node(
                self.activation_nodes.len(),
                activation,
            ));
        }

        for (index, message) in self.sequence.messages.iter().enumerate() {
            let source = self.endpoint_for_message(index, message, true);
            let target = self.endpoint_for_message(index, message, false);
            self.ensure_port(source);
            self.ensure_port(target);
        }
    }

    fn add_participants(&mut self) {
        for participant in &self.sequence.participants {
            let node_id = participant_node_id(&participant.id);
            let bounds = rect_from_participant(participant);
            let labels = participant_label_boxes(&node_id, participant);
            let ports = self.take_ports(&node_id);
            self.scene.add_node(SceneNode {
                id: node_id.clone(),
                node_box: NodeBox {
                    id: node_id.clone(),
                    bounds,
                    ports,
                    labels,
                },
            });
        }
    }

    fn add_footboxes(&mut self) {
        for participant in &self.sequence.footboxes {
            let node_id = format!("footbox:{}", participant.id);
            self.scene.add_node(SceneNode {
                id: node_id.clone(),
                node_box: NodeBox {
                    id: node_id.clone(),
                    bounds: rect_from_participant(participant),
                    ports: Vec::new(),
                    labels: participant_label_boxes(&node_id, participant),
                },
            });
        }
    }

    fn add_lifeline_lanes(&mut self) {
        for lifeline in &self.sequence.lifelines {
            let lane_id = format!("lifeline:{}", lifeline.participant_id);
            let mut child_node_ids = vec![participant_node_id(&lifeline.participant_id)];
            child_node_ids.extend(
                self.activation_nodes
                    .iter()
                    .filter(|node| node.participant_id == lifeline.participant_id)
                    .map(|node| node.node_id.clone()),
            );
            self.scene.add_lane(LaneFrame {
                id: lane_id,
                bounds: Rect::new(
                    f64::from(lifeline.x),
                    f64::from(lifeline.y1),
                    LIFELINE_WIDTH,
                    f64::from(lifeline.y2 - lifeline.y1).max(0.0),
                ),
                header: None,
                child_node_ids,
                labels: Vec::new(),
            });
        }
    }

    fn add_activations(&mut self) {
        for activation in self.activation_nodes.clone() {
            let ports = self.take_ports(&activation.node_id);
            self.scene.add_node(SceneNode {
                id: activation.node_id.clone(),
                node_box: NodeBox {
                    id: activation.node_id.clone(),
                    bounds: activation.bounds,
                    ports,
                    labels: Vec::new(),
                },
            });
        }
    }

    fn add_virtual_endpoint_nodes(&mut self) {
        let virtual_node_ids = self
            .ports_by_owner
            .keys()
            .filter(|id| id.starts_with("virtual:"))
            .cloned()
            .collect::<Vec<_>>();
        for node_id in virtual_node_ids {
            let position = self
                .ports_by_owner
                .get(&node_id)
                .and_then(|ports| ports.values().next())
                .map(|port| port.position)
                .unwrap_or(Point::new(0.0, 0.0));
            let ports = self.take_ports(&node_id);
            self.scene.add_node(SceneNode {
                id: node_id.clone(),
                node_box: NodeBox {
                    id: node_id.clone(),
                    bounds: Rect::new(position.x - 4.0, position.y - 4.0, 8.0, 8.0),
                    ports,
                    labels: Vec::new(),
                },
            });
        }
    }

    fn add_lifecycle_markers(&mut self) {
        for (index, marker) in self.sequence.lifecycle_markers.iter().enumerate() {
            let node_id = format!("lifecycle:{index}:{}", marker.participant_id);
            self.scene.add_node(SceneNode {
                id: node_id.clone(),
                node_box: NodeBox {
                    id: node_id.clone(),
                    bounds: lifecycle_marker_bounds(marker),
                    ports: Vec::new(),
                    labels: Vec::new(),
                },
            });
        }
    }

    fn add_notes(&mut self) {
        for (index, note) in self.sequence.notes.iter().enumerate() {
            let node_id = format!("note:{index}");
            self.scene.add_node(SceneNode {
                id: node_id.clone(),
                node_box: NodeBox {
                    id: node_id.clone(),
                    bounds: Rect::new(
                        f64::from(note.x),
                        f64::from(note.y),
                        f64::from(note.width),
                        f64::from(note.height),
                    ),
                    ports: Vec::new(),
                    labels: note_label_boxes(&node_id, note),
                },
            });
        }
    }

    fn add_groups(&mut self) {
        for (index, group) in self.sequence.groups.iter().enumerate() {
            let group_id = format!("group:{index}:{}", group.kind);
            let bounds = Rect::new(
                f64::from(group.x),
                f64::from(group.y),
                f64::from(group.width),
                f64::from(group.height),
            );
            self.scene.add_group(SceneGroup {
                id: group_id.clone(),
                frame: GroupFrame {
                    id: group_id.clone(),
                    bounds,
                    header: group_header_bounds(group),
                    child_node_ids: child_nodes_in_bounds(&self.scene, bounds),
                    labels: group_label_boxes(&group_id, group),
                },
            });
        }
    }

    fn add_metadata_labels(&mut self) {
        for (role, label) in [
            ("header", self.sequence.header.as_ref()),
            ("title", self.sequence.title.as_ref()),
            ("caption", self.sequence.caption.as_ref()),
            ("footer", self.sequence.footer.as_ref()),
        ] {
            if let Some(label) = label {
                for label_box in metadata_label_boxes(role, label, self.sequence.width) {
                    self.scene.add_label_box(label_box);
                }
            }
        }
    }

    fn add_messages(&mut self) {
        let mut parallel_label_lanes: BTreeMap<i32, i32> = BTreeMap::new();
        for (index, message) in self.sequence.messages.iter().enumerate() {
            let source = self.endpoint_for_message(index, message, true);
            let target = self.endpoint_for_message(index, message, false);
            let edge_id = format!("message:{index}");
            self.scene.add_edge(SceneEdge {
                id: edge_id.clone(),
                from: source.node_id.clone(),
                to: target.node_id.clone(),
                route: message_route(message),
                route_channel_ids: Vec::new(),
                source_anchor: self.anchor_for_endpoint(&edge_id, "source", source),
                target_anchor: self.anchor_for_endpoint(&edge_id, "target", target),
                labels: message_label_boxes(
                    &edge_id,
                    message,
                    self.sequence.style.message_align,
                    self.sequence.style.response_message_below_arrow,
                    &mut parallel_label_lanes,
                ),
            });
        }
    }

    fn endpoint_for_message(
        &self,
        message_index: usize,
        message: &MessageLine,
        source: bool,
    ) -> EndpointNode {
        let (participant_id, virtual_endpoint, x, y) = if source {
            (
                &message.from_id,
                message.from_virtual,
                message.x1,
                message.route_y,
            )
        } else if message.from_id == message.to_id
            && message.from_virtual.is_none()
            && message.to_virtual.is_none()
        {
            (
                &message.to_id,
                message.to_virtual,
                message.x1,
                message.route_y + SELF_LOOP_DROP as i32,
            )
        } else {
            (
                &message.to_id,
                message.to_virtual,
                message.x2,
                message.route_y,
            )
        };
        let position = Point::new(f64::from(x), f64::from(y));

        if let Some(virtual_endpoint) = virtual_endpoint {
            return EndpointNode {
                node_id: format!(
                    "virtual:{message_index}:{}",
                    if source { "source" } else { "target" }
                ),
                port_id: format!(
                    "virtual:{message_index}:{}:port",
                    if source { "source" } else { "target" }
                ),
                side: side_for_virtual(virtual_endpoint),
                position,
            };
        }

        if let Some(activation) = self.activation_at(participant_id, position) {
            return EndpointNode {
                node_id: activation.node_id.clone(),
                port_id: format!(
                    "{}:{}:{}",
                    activation.node_id,
                    if source { "source" } else { "target" },
                    message_index
                ),
                side: side_for_activation(activation.bounds, position),
                position,
            };
        }

        let node_id = self
            .participant_nodes
            .get(participant_id)
            .cloned()
            .unwrap_or_else(|| participant_node_id(participant_id));
        EndpointNode {
            node_id,
            port_id: format!(
                "participant:{participant_id}:{}:{}",
                if source { "source" } else { "target" },
                message_index
            ),
            side: side_for_participant_message(message.x1, message.x2, source),
            position,
        }
    }

    fn activation_at(&self, participant_id: &str, position: Point) -> Option<&ActivationSceneNode> {
        self.activation_nodes.iter().find(|activation| {
            activation.participant_id == participant_id
                && position.y >= activation.bounds.min_y() - 0.5
                && position.y <= activation.bounds.max_y() + 0.5
                && ((position.x - activation.bounds.min_x()).abs() <= 0.5
                    || (position.x - activation.bounds.max_x()).abs() <= 0.5)
        })
    }

    fn ensure_port(&mut self, endpoint: EndpointNode) {
        self.ports_by_owner
            .entry(endpoint.node_id.clone())
            .or_default()
            .entry(endpoint.port_id.clone())
            .or_insert_with(|| Port {
                id: endpoint.port_id,
                node_id: endpoint.node_id,
                side: endpoint.side,
                position: endpoint.position,
            });
    }

    fn take_ports(&mut self, node_id: &str) -> Vec<Port> {
        self.ports_by_owner
            .remove(node_id)
            .map(|ports| ports.into_values().collect())
            .unwrap_or_default()
    }

    fn anchor_for_endpoint(&self, edge_id: &str, role: &str, endpoint: EndpointNode) -> Anchor {
        let port = self
            .ports_by_owner
            .get(&endpoint.node_id)
            .and_then(|ports| ports.get(&endpoint.port_id))
            .cloned()
            .or_else(|| {
                self.scene.nodes.get(&endpoint.node_id).and_then(|node| {
                    node.node_box
                        .ports
                        .iter()
                        .find(|port| port.id == endpoint.port_id)
                        .cloned()
                })
            });
        Anchor {
            id: format!("{edge_id}:{role}"),
            owner_id: endpoint.node_id,
            position: endpoint.position,
            port,
        }
    }
}

fn participant_node_id(id: &str) -> String {
    format!("participant:{id}")
}

fn activation_scene_node(index: usize, activation: &ActivationBox) -> ActivationSceneNode {
    let offset = (activation.depth as i32) * 6;
    let x = activation.x + offset - 5;
    let y = activation.y1.min(activation.y2);
    let height = (activation.y2 - activation.y1).abs().max(12);
    ActivationSceneNode {
        node_id: format!("activation:{index}:{}", activation.participant_id),
        participant_id: activation.participant_id.clone(),
        bounds: Rect::new(f64::from(x), f64::from(y), 10.0, f64::from(height)),
    }
}

fn rect_from_participant(participant: &ParticipantBox) -> Rect {
    Rect::new(
        f64::from(participant.x),
        f64::from(participant.y),
        f64::from(participant.width),
        f64::from(participant.height),
    )
}

fn lifecycle_marker_bounds(marker: &LifecycleMarker) -> Rect {
    Rect::new(f64::from(marker.x - 6), f64::from(marker.y - 6), 12.0, 12.0)
}

fn message_route(message: &MessageLine) -> Polyline {
    if message.from_id == message.to_id
        && message.from_virtual.is_none()
        && message.to_virtual.is_none()
    {
        return Polyline::new(vec![
            Point::new(f64::from(message.x1), f64::from(message.route_y)),
            Point::new(f64::from(message.x2), f64::from(message.route_y)),
            Point::new(
                f64::from(message.x2),
                f64::from(message.route_y) + SELF_LOOP_DROP,
            ),
            Point::new(
                f64::from(message.x1),
                f64::from(message.route_y) + SELF_LOOP_DROP,
            ),
        ]);
    }
    Polyline::new(vec![
        Point::new(f64::from(message.x1), f64::from(message.route_y)),
        Point::new(f64::from(message.x2), f64::from(message.route_y)),
    ])
}

fn child_nodes_in_bounds(scene: &RenderScene, bounds: Rect) -> Vec<String> {
    scene
        .nodes
        .values()
        .filter(|node| bounds.contains_rect(node.node_box.bounds))
        .map(|node| node.id.clone())
        .collect()
}

fn side_for_virtual(endpoint: VirtualEndpoint) -> PortSide {
    match endpoint.side {
        VirtualEndpointSide::Left => PortSide::Left,
        VirtualEndpointSide::Right => PortSide::Right,
    }
}

fn side_for_activation(bounds: Rect, position: Point) -> PortSide {
    if (position.x - bounds.min_x()).abs() <= (position.x - bounds.max_x()).abs() {
        PortSide::Left
    } else {
        PortSide::Right
    }
}

fn side_for_participant_message(x1: i32, x2: i32, source: bool) -> PortSide {
    if x1 == x2 {
        return PortSide::Center;
    }
    let left_to_right = x2 >= x1;
    match (source, left_to_right) {
        (true, true) | (false, false) => PortSide::Right,
        _ => PortSide::Left,
    }
}
