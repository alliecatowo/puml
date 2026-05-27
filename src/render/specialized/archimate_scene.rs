use super::*;
use crate::render_core::{
    Anchor, LabelBox, LabelRole, LaneFrame, NodeBox, Point, Polyline, Rect, RenderScene, SceneEdge,
    SceneNode,
};

/// Build a typed `RenderScene` that mirrors the geometry emitted by
/// `render_archimate_svg_inner`.
///
/// Nodes are keyed by element **name** (the same string used in
/// `element_bounds.insert(elem.name.clone(), ...)`).  Alias → name resolution
/// is applied so that edge `from`/`to` always refer to a real node id even
/// when the relation was written using an alias.
pub(super) fn build_archimate_scene(document: &ArchimateDocument) -> RenderScene {
    let width = 760;
    let layers = [
        "strategy",
        "business",
        "application",
        "technology",
        "motivation",
        "junction",
    ];
    let lane_height = 80;
    let populated_layer_count = layers
        .iter()
        .filter(|&&layer| document.elements.iter().any(|e| e.layer == layer))
        .count()
        .max(1);
    let height = 80 + (populated_layer_count as i32) * lane_height;

    let viewport = Rect::new(0.0, 0.0, width as f64, height as f64);
    let mut scene = RenderScene::new(viewport);

    // alias → element name resolution map
    let mut alias_to_name: BTreeMap<String, String> = BTreeMap::new();
    for elem in &document.elements {
        if let Some(alias) = &elem.alias {
            alias_to_name.insert(alias.clone(), elem.name.clone());
        }
    }

    // geometry mirrors render_archimate_svg_inner exactly
    let mut y = 44i32; // 28 + 16 as in the SVG emitter
    let mut element_bounds: BTreeMap<String, (i32, i32, i32, i32)> = BTreeMap::new();

    for layer in layers.iter() {
        let layer_elements: Vec<_> = document
            .elements
            .iter()
            .filter(|e| e.layer == *layer)
            .collect();
        if layer_elements.is_empty() {
            continue;
        }
        let layer_y = y;

        // Lane for this layer band
        scene.add_lane(LaneFrame {
            id: format!("lane:{layer}"),
            bounds: Rect::new(24.0, layer_y as f64, 712.0, lane_height as f64),
            header: Some(Rect::new(24.0, layer_y as f64, 712.0, 14.0)),
            child_node_ids: layer_elements.iter().map(|e| e.name.clone()).collect(),
            labels: vec![LabelBox {
                id: format!("lane:{layer}:label"),
                text: layer.to_string(),
                bounds: Rect::new(32.0, (layer_y + 14) as f64, 80.0, 12.0),
                owner_id: Some(format!("lane:{layer}")),
                role: LabelRole::Lane,
            }],
        });

        let mut x = 100i32;
        for elem in layer_elements {
            let elem_y = layer_y + 22;
            let bounds = Rect::new(x as f64, elem_y as f64, 140.0, 40.0);
            let label = LabelBox {
                id: format!("{}:label", elem.name),
                text: elem.name.clone(),
                bounds: Rect::new((x + 8) as f64, (layer_y + 46) as f64, 100.0, 14.0),
                owner_id: Some(elem.name.clone()),
                role: LabelRole::Node,
            };
            scene.add_node(SceneNode {
                id: elem.name.clone(),
                node_box: NodeBox {
                    id: elem.name.clone(),
                    bounds,
                    ports: vec![],
                    labels: vec![label],
                },
            });
            element_bounds.insert(elem.name.clone(), (x, elem_y, 140, 40));
            if let Some(alias) = &elem.alias {
                element_bounds.insert(alias.clone(), (x, elem_y, 140, 40));
            }
            x += 150;
            if x + 140 > 736 {
                break;
            }
        }
        y += lane_height;
    }

    // Resolve alias or name to node id (element name).
    let resolve_id = |id: &str| -> String {
        alias_to_name
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_string())
    };

    for (rel_idx, rel) in document.relations.iter().enumerate() {
        let Some(&from_rect) = element_bounds.get(&rel.from) else {
            continue;
        };
        let Some(&to_rect) = element_bounds.get(&rel.to) else {
            continue;
        };
        let (x1, y1, x2, y2) =
            compute_edge_anchors_for_direction(from_rect, to_rect, rel.direction.as_deref());
        let from_id = resolve_id(&rel.from);
        let to_id = resolve_id(&rel.to);
        let edge_id = format!("rel:{rel_idx}");
        let src_pos = Point::new(x1 as f64, y1 as f64);
        let tgt_pos = Point::new(x2 as f64, y2 as f64);
        scene.add_edge(SceneEdge {
            id: edge_id.clone(),
            from: from_id.clone(),
            to: to_id.clone(),
            route: Polyline::from_tuples(&[(x1 as f64, y1 as f64), (x2 as f64, y2 as f64)]),
            route_channel_ids: vec![],
            source_anchor: Anchor {
                id: format!("{edge_id}:source"),
                owner_id: from_id,
                position: src_pos,
                port: None,
            },
            target_anchor: Anchor {
                id: format!("{edge_id}:target"),
                owner_id: to_id,
                position: tgt_pos,
                port: None,
            },
            labels: vec![],
        });
    }

    scene
}

#[cfg(test)]
mod tests {
    use crate::model::{ArchimateElement, ArchimateRelation};

    use super::super::archimate::{render_archimate_artifact, render_archimate_svg};

    fn make_doc() -> crate::model::ArchimateDocument {
        crate::model::ArchimateDocument {
            title: Some("Test".to_string()),
            elements: vec![
                ArchimateElement {
                    name: "Order Service".to_string(),
                    alias: Some("svc".to_string()),
                    layer: "application".to_string(),
                    kind: "component".to_string(),
                    fill: None,
                    stroke: None,
                },
                ArchimateElement {
                    name: "Customer".to_string(),
                    alias: Some("cust".to_string()),
                    layer: "business".to_string(),
                    kind: "actor".to_string(),
                    fill: None,
                    stroke: None,
                },
            ],
            relations: vec![ArchimateRelation {
                from: "svc".to_string(),
                to: "cust".to_string(),
                kind: "serving".to_string(),
                label: None,
                direction: None,
                style: None,
            }],
            warnings: vec![],
        }
    }

    #[test]
    fn archimate_artifact_scene_has_no_geometry_issues() {
        let doc = make_doc();
        let artifact = render_archimate_artifact(&doc);
        let scene = artifact.scene.expect("archimate scene must be present");

        // Two elements → two nodes keyed by display name
        assert_eq!(scene.nodes.len(), 2, "expected 2 nodes");
        assert!(
            scene.nodes.contains_key("Order Service"),
            "node 'Order Service' must be present"
        );
        assert!(
            scene.nodes.contains_key("Customer"),
            "node 'Customer' must be present"
        );

        // One relation → one edge
        assert_eq!(scene.edges.len(), 1, "expected 1 edge");
        let edge = scene.edges.values().next().unwrap();
        assert_eq!(
            edge.from, "Order Service",
            "edge.from must resolve alias 'svc' → 'Order Service'"
        );
        assert_eq!(
            edge.to, "Customer",
            "edge.to must resolve alias 'cust' → 'Customer'"
        );

        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "archimate scene must have no geometry issues: {issues:?}"
        );
    }

    #[test]
    fn archimate_svg_and_artifact_svg_are_byte_identical() {
        let doc = make_doc();
        let svg_direct = render_archimate_svg(&doc);
        let artifact = render_archimate_artifact(&doc);
        assert_eq!(
            svg_direct, artifact.svg,
            "render_archimate_svg and render_archimate_artifact must produce identical SVG"
        );
    }
}
