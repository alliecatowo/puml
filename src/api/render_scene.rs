use serde_json::{json, Value};

use crate::model::{FamilyDocument, NormalizedDocument};
use crate::{layout, LayoutOptions};

use super::render::render_family_document_artifact;
use super::render_summary::{family_model_summary_to_json, normalized_model_summary_to_json};

pub fn normalized_scene_summary_to_json(model: &NormalizedDocument) -> Value {
    match model {
        NormalizedDocument::Sequence(sequence) => {
            let pages = layout::layout_pages(sequence, LayoutOptions::default());
            json!({
                "kind": "Sequence",
                "typed": false,
                "pageCount": pages.len(),
                "pages": pages.iter().map(sequence_scene_to_json).collect::<Vec<_>>()
            })
        }
        NormalizedDocument::Family(family) => family_scene_summary_to_json(family),
        NormalizedDocument::FamilyPages(pages) => json!({
            "kind": "FamilyPages",
            "typed": false,
            "pageCount": pages.len(),
            "pages": pages.iter().map(family_scene_summary_to_json).collect::<Vec<_>>()
        }),
        _ => {
            let summary = normalized_model_summary_to_json(model);
            json!({
                "kind": summary["kind"].clone(),
                "typed": false,
                "available": false,
                "summary": summary
            })
        }
    }
}

fn sequence_scene_to_json(scene: &crate::Scene) -> Value {
    json!({
        "size": {"width": scene.width, "height": scene.height},
        "participants": scene.participants.iter().map(|participant| {
            json!({
                "id": participant.id,
                "display": participant.display_lines.join("\n"),
                "role": format!("{:?}", participant.role),
                "bounds": {
                    "x": participant.x,
                    "y": participant.y,
                    "width": participant.width,
                    "height": participant.height
                }
            })
        }).collect::<Vec<_>>(),
        "messages": scene.messages.iter().map(|message| {
            json!({
                "from": message.from_id,
                "to": message.to_id,
                "arrow": message.arrow,
                "label": message.label,
                "route": {
                    "x1": message.x1,
                    "y": message.y,
                    "routeY": message.route_y,
                    "x2": message.x2
                }
            })
        }).collect::<Vec<_>>(),
        "notes": scene.notes.len(),
        "groups": scene.groups.len(),
        "structures": scene.structures.len()
    })
}

fn family_scene_summary_to_json(family: &FamilyDocument) -> Value {
    let artifact = render_family_document_artifact(family);
    match artifact.scene {
        Some(scene) => render_core_scene_to_json(&scene),
        None => json!({
            "kind": format!("{:?}", family.kind),
            "typed": false,
            "available": false,
            "summary": family_model_summary_to_json(family)
        }),
    }
}

fn render_core_scene_to_json(scene: &crate::render_core::RenderScene) -> Value {
    json!({
        "kind": "RenderScene",
        "typed": true,
        "viewport": rect_to_json(scene.viewport),
        "nodes": scene.nodes.values().map(|node| {
            json!({
                "id": node.id,
                "bounds": rect_to_json(node.node_box.bounds),
                "ports": node.node_box.ports.len(),
                "labels": node.node_box.labels.len()
            })
        }).collect::<Vec<_>>(),
        "edges": scene.edges.values().map(|edge| {
            json!({
                "id": edge.id,
                "from": edge.from,
                "to": edge.to,
                "points": edge.route.points.iter().map(|point| {
                    json!({"x": point.x, "y": point.y})
                }).collect::<Vec<_>>(),
                "labels": edge.labels.len()
            })
        }).collect::<Vec<_>>(),
        "groups": scene.groups.values().map(|group| {
            json!({
                "id": group.id,
                "bounds": rect_to_json(group.frame.bounds),
                "children": group.frame.child_node_ids
            })
        }).collect::<Vec<_>>(),
        "lanes": scene.lanes.values().map(|lane| {
            json!({
                "id": lane.id,
                "bounds": rect_to_json(lane.bounds),
                "children": lane.child_node_ids
            })
        }).collect::<Vec<_>>(),
        "labels": scene.labels.len(),
        "routeChannels": scene.route_channels.len()
    })
}

fn rect_to_json(rect: crate::render_core::Rect) -> Value {
    json!({
        "x": rect.origin.x,
        "y": rect.origin.y,
        "width": rect.size.width,
        "height": rect.size.height
    })
}
