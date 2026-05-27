use crate::model::{StateNode, StateNodeKind, StateTransition};
use crate::render_core::{
    Anchor, GroupFrame, LabelBox, LabelRole, NodeBox, Point, Polyline, Rect, RenderScene,
    SceneEdge, SceneGroup, SceneNode,
};

use super::edges::state_upward_elbow_x;
use super::labels::edge_anchors_for_kinds;
use super::types::{PlacedNode, COMPOSITE_PAD_Y};

/// Build a typed [`RenderScene`] from the laid-out geometry of a state diagram.
///
/// Every placed node becomes a [`SceneNode`] at its exact `placed` rect.
/// Composite states additionally become [`SceneGroup`]s so child containment
/// is captured. Every top-level transition (both endpoints in `placed`,
/// excluding note connectors) becomes a [`SceneEdge`] whose polyline follows
/// the same orthogonal route the SVG draws.
pub(super) fn build_state_scene(
    nodes: &[StateNode],
    transitions: &[StateTransition],
    placed: &std::collections::BTreeMap<String, PlacedNode>,
    child_node_names: &std::collections::BTreeSet<&str>,
    width: f64,
    height: f64,
) -> RenderScene {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, width, height));

    // Add all placed nodes as SceneNodes (including composite children).
    for (name, p) in placed {
        let bounds = Rect::new(p.x as f64, p.y as f64, p.w as f64, p.h as f64);
        let label = LabelBox {
            id: format!("{name}::label"),
            text: name.clone(),
            bounds,
            owner_id: Some(name.clone()),
            role: LabelRole::Node,
        };
        scene.add_node(SceneNode {
            id: name.clone(),
            node_box: NodeBox {
                id: name.clone(),
                bounds,
                ports: Vec::new(),
                labels: vec![label],
            },
        });
    }

    // Add composite states as SceneGroups so child containment is captured.
    for node in nodes {
        if child_node_names.contains(node.name.as_str()) {
            continue;
        }
        add_composite_groups_recursive(node, placed, &mut scene);
    }

    // Add top-level transitions as SceneEdges.
    // Skip note connectors and intra-composite transitions (both endpoints
    // are children); those are structural decoration, not typed graph edges.
    let node_kinds: std::collections::BTreeMap<&str, &StateNodeKind> =
        nodes.iter().map(|n| (n.name.as_str(), &n.kind)).collect();
    // Also gather child node kinds for composite-internal transitions
    fn collect_node_kinds<'a>(
        node: &'a StateNode,
        map: &mut std::collections::BTreeMap<&'a str, &'a StateNodeKind>,
    ) {
        map.insert(node.name.as_str(), &node.kind);
        for region in &node.regions {
            for child in region {
                collect_node_kinds(child, map);
            }
        }
    }
    let mut all_node_kinds: std::collections::BTreeMap<&str, &StateNodeKind> =
        std::collections::BTreeMap::new();
    for node in nodes {
        collect_node_kinds(node, &mut all_node_kinds);
    }

    for (idx, t) in transitions.iter().enumerate() {
        // Skip if either endpoint is not in placed (defensive)
        let (Some(fp), Some(tp)) = (placed.get(&t.from), placed.get(&t.to)) else {
            continue;
        };
        // Skip note connectors
        if matches!(
            all_node_kinds.get(t.to.as_str()).copied(),
            Some(StateNodeKind::Note)
        ) {
            continue;
        }

        let (x1, y1, x2, y2) = edge_anchors_for_kinds(
            node_kinds.get(t.from.as_str()).copied(),
            fp,
            node_kinds.get(t.to.as_str()).copied(),
            tp,
        );

        let route_tuples = if t.from == t.to {
            // Self-loop: use a simple two-point route (the SVG draws a bezier
            // but the scene just needs start/end anchors at the same point)
            let loop_rx = 18i32;
            let loop_ry = 14i32;
            let cpx = x1 + loop_rx;
            let cpy = y1 - loop_ry;
            vec![
                (x1 as f64, y1 as f64),
                (cpx as f64, cpy as f64),
                (x2 as f64, y2 as f64),
            ]
        } else {
            // Orthogonal path — reconstruct the same waypoints as the SVG
            state_orthogonal_polyline_tuples(x1, y1, x2, y2)
        };

        let src_pt = Point::new(x1 as f64, y1 as f64);
        let tgt_pt = Point::new(x2 as f64, y2 as f64);
        let edge_id = format!("t{idx}:{}:{}", t.from, t.to);
        scene.add_edge(SceneEdge {
            id: edge_id.clone(),
            from: t.from.clone(),
            to: t.to.clone(),
            route: Polyline::from_tuples(&route_tuples),
            route_channel_ids: Vec::new(),
            source_anchor: Anchor {
                id: format!("{edge_id}::src"),
                owner_id: t.from.clone(),
                position: src_pt,
                port: None,
            },
            target_anchor: Anchor {
                id: format!("{edge_id}::tgt"),
                owner_id: t.to.clone(),
                position: tgt_pt,
                port: None,
            },
            labels: Vec::new(),
        });
    }

    scene
}

/// Recursively add composite state nodes as [`SceneGroup`]s.
/// Each composite gets a group frame whose bounds mirror the composite's
/// placed rect and whose `child_node_ids` lists the direct children.
pub(super) fn add_composite_groups_recursive(
    node: &StateNode,
    placed: &std::collections::BTreeMap<String, PlacedNode>,
    scene: &mut RenderScene,
) {
    let has_children = node.regions.iter().any(|r| !r.is_empty());
    if node.kind == StateNodeKind::Normal && has_children {
        if let Some(p) = placed.get(&node.name) {
            let bounds = Rect::new(p.x as f64, p.y as f64, p.w as f64, p.h as f64);
            // Header label rect spans the top of the composite box.
            let header = Rect::new(p.x as f64, p.y as f64, p.w as f64, COMPOSITE_PAD_Y as f64);
            let display = node
                .display
                .as_deref()
                .unwrap_or(node.name.as_str())
                .to_string();
            let header_label = LabelBox {
                id: format!("{}::group::label", node.name),
                text: display,
                bounds: header,
                owner_id: Some(node.name.clone()),
                role: LabelRole::Group,
            };
            let child_node_ids: Vec<String> = node
                .regions
                .iter()
                .flat_map(|r| r.iter())
                .map(|child| child.name.clone())
                .collect();
            scene.add_group(SceneGroup {
                id: node.name.clone(),
                frame: GroupFrame {
                    id: node.name.clone(),
                    bounds,
                    header: Some(header),
                    child_node_ids,
                    labels: vec![header_label],
                },
            });
        }
    }
    // Recurse into children
    for region in &node.regions {
        for child in region {
            add_composite_groups_recursive(child, placed, scene);
        }
    }
}

/// Reconstruct the orthogonal polyline waypoints that the SVG uses for a
/// transition edge. Must match [`state_orthogonal_path_data`] exactly.
pub(super) fn state_orthogonal_polyline_tuples(
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
) -> Vec<(f64, f64)> {
    if x1 == x2 || y1 == y2 {
        vec![(x1 as f64, y1 as f64), (x2 as f64, y2 as f64)]
    } else if y2 < y1 {
        let mid_x = state_upward_elbow_x(x1, x2);
        vec![
            (x1 as f64, y1 as f64),
            (mid_x as f64, y1 as f64),
            (mid_x as f64, y2 as f64),
            (x2 as f64, y2 as f64),
        ]
    } else {
        let mid_y = y1 + (y2 - y1) / 2;
        vec![
            (x1 as f64, y1 as f64),
            (x1 as f64, mid_y as f64),
            (x2 as f64, mid_y as f64),
            (x2 as f64, y2 as f64),
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::model::StateDocument;
    use crate::{normalize, parser};

    fn parse_state(src: &str) -> StateDocument {
        let ast = parser::parse(src).expect("parse failed");
        match normalize::normalize_family(ast).expect("normalize failed") {
            crate::model::NormalizedDocument::State(doc) => doc,
            other => panic!("expected state document, got {other:?}"),
        }
    }

    use super::super::render_state_artifact;

    #[test]
    fn render_state_artifact_basic_scene_counts() {
        // 3 state nodes: [*]__start, Active, [*]__end; 2 transitions.
        let doc = parse_state("@startuml\nstate Active\n[*] --> Active\nActive --> [*]\n@enduml\n");
        let artifact = render_state_artifact(&doc);

        // SVG must still look like an SVG
        assert!(artifact.svg.starts_with("<svg"), "expected SVG output");

        let scene = artifact.typed_scene().expect("expected typed scene");

        // Count non-pseudo, non-note placed nodes.  We just assert the scene
        // has at least as many nodes as document nodes (some may be absent if
        // they had no placement).
        let doc_node_count = doc.nodes.len();
        assert!(
            scene.nodes.len() >= doc_node_count,
            "scene should have at least {} nodes, got {}",
            doc_node_count,
            scene.nodes.len()
        );

        // Transition count: one SceneEdge per transition (note connectors
        // excluded, but this simple diagram has none).
        let expected_edges = doc.transitions.len();
        assert_eq!(
            scene.edges.len(),
            expected_edges,
            "expected {} edges in scene, got {}",
            expected_edges,
            scene.edges.len()
        );

        // Validate scene geometry: report any issues but don't hard-fail on
        // issues the validator catches as warnings only.
        let issues = scene.validate_geometry();
        assert!(issues.is_empty(), "scene geometry issues: {issues:?}");
    }

    #[test]
    fn render_state_artifact_composite_adds_groups() {
        let src = "@startuml\nstate Outer {\n  state Inner\n  [*] --> Inner\n}\n[*] --> Outer\nOuter --> [*]\n@enduml\n";
        let doc = parse_state(src);
        let artifact = render_state_artifact(&doc);
        let scene = artifact.typed_scene().expect("expected typed scene");

        // Outer is a composite → must appear in scene.groups
        assert!(
            scene.groups.contains_key("Outer"),
            "composite state Outer should be a SceneGroup"
        );
    }
}
