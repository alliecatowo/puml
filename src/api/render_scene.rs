use serde_json::{json, Value};

use super::render::render_family_document_artifact;
use super::render_summary::{family_model_summary_to_json, normalized_model_summary_to_json};
use crate::model::{FamilyDocument, NormalizedDocument};
use crate::output::{RenderArtifact, RenderSceneContract};
use crate::render_core::SceneAvailability;

pub fn normalized_scene_summary_to_json(model: &NormalizedDocument) -> Value {
    match model {
        NormalizedDocument::Sequence(_) => {
            let artifacts = super::render::render_artifact_pages_from_model(model);
            sequence_artifact_scene_summary_to_json(&artifacts)
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

pub fn normalized_artifact_scene_summary_to_json(
    model: &NormalizedDocument,
    artifacts: &[RenderArtifact],
) -> Value {
    match model {
        NormalizedDocument::Sequence(_) => sequence_artifact_scene_summary_to_json(artifacts),
        NormalizedDocument::Family(family) => {
            family_artifact_scene_summary_to_json(family, artifacts.first())
        }
        NormalizedDocument::FamilyPages(pages) => json!({
            "kind": "FamilyPages",
            "typed": false,
            "pageCount": pages.len(),
            "pages": pages.iter().zip(artifacts.iter()).map(|(family, artifact)| {
                family_artifact_scene_summary_to_json(family, Some(artifact))
            }).collect::<Vec<_>>()
        }),
        _ => {
            let summary = normalized_model_summary_to_json(model);
            json!({
                "kind": summary["kind"].clone(),
                "typed": false,
                "available": false,
                "sceneAvailability": artifacts
                    .first()
                    .map(scene_availability_to_str)
                    .unwrap_or("NotMigrated"),
                "summary": summary
            })
        }
    }
}

fn sequence_artifact_scene_summary_to_json(artifacts: &[RenderArtifact]) -> Value {
    let pages = artifacts
        .iter()
        .map(|artifact| match artifact.scene_contract() {
            RenderSceneContract::Typed(scene) => render_core_scene_to_json(scene),
            RenderSceneContract::NotMigrated
            | RenderSceneContract::Unsupported
            | RenderSceneContract::Inconsistent => json!({
                "kind": "Sequence",
                "typed": false,
                "available": false,
                "sceneAvailability": scene_availability_to_str(artifact)
            }),
        })
        .collect::<Vec<_>>();
    let typed = artifacts
        .iter()
        .all(|artifact| artifact.typed_scene().is_some())
        && !artifacts.is_empty();
    json!({
        "kind": "Sequence",
        "typed": typed,
        "sceneAvailability": artifacts
            .first()
            .map(scene_availability_to_str)
            .unwrap_or("NotMigrated"),
        "pageCount": artifacts.len(),
        "pages": pages,
    })
}

fn family_scene_summary_to_json(family: &FamilyDocument) -> Value {
    let artifact = render_family_document_artifact(family);
    family_artifact_scene_summary_to_json(family, Some(&artifact))
}

fn family_artifact_scene_summary_to_json(
    family: &FamilyDocument,
    artifact: Option<&RenderArtifact>,
) -> Value {
    match artifact.map(RenderArtifact::scene_contract) {
        Some(RenderSceneContract::Typed(scene)) => render_core_scene_to_json(scene),
        Some(RenderSceneContract::NotMigrated)
        | Some(RenderSceneContract::Unsupported)
        | Some(RenderSceneContract::Inconsistent)
        | None => json!({
            "kind": format!("{:?}", family.kind),
            "typed": false,
            "available": false,
            "sceneAvailability": artifact
                .map(scene_availability_to_str)
                .unwrap_or("NotMigrated"),
            "summary": family_model_summary_to_json(family)
        }),
    }
}

fn render_core_scene_to_json(scene: &crate::render_core::RenderScene) -> Value {
    json!({
        "kind": "RenderScene",
        "typed": true,
        "sceneAvailability": "TypedScene",
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

fn scene_availability_to_str(artifact: &RenderArtifact) -> &'static str {
    match artifact.scene_availability {
        SceneAvailability::TypedScene => "TypedScene",
        SceneAvailability::NotMigrated => "NotMigrated",
        SceneAvailability::Unsupported => "Unsupported",
    }
}

fn rect_to_json(rect: crate::render_core::Rect) -> Value {
    json!({
        "x": rect.origin.x,
        "y": rect.origin.y,
        "width": rect.size.width,
        "height": rect.size.height
    })
}
