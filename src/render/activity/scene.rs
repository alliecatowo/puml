use crate::model::{FamilyDocument, FamilyNodeKind};
use crate::render_core::{
    Anchor, LabelBox, LabelRole, LaneFrame, NodeBox, Point, Polyline, Rect, RenderScene, SceneEdge,
    SceneNode,
};

use super::layout;
use super::nodes;

/// Build a typed [`RenderScene`] mirroring the activity SVG's geometry.
///
/// Every node box uses the *exact* bounds the SVG renders (center x from
/// `NodeLayout.cx`, top from `NodeLayout.slot_y`, same w/h constants the SVG
/// shapes use).  Every edge is a polyline through the same (x1,y1)→(x2,y2)
/// coordinates used to draw the arrow.  Swimlanes are captured as
/// [`LaneFrame`]s from the lane span data.  This guarantees scene ↔ SVG
/// consistency with no separate layout pass.
#[allow(clippy::too_many_arguments)]
pub(super) fn build_activity_scene(
    doc: &FamilyDocument,
    metas: &[layout::NodeMeta],
    node_layouts: &[layout::NodeLayout],
    hidden_nodes: &std::collections::BTreeSet<usize>,
    fork_bar_half_widths: &std::collections::BTreeMap<usize, i32>,
    redirected_extra_arrows: &[layout::ActivityRoute],
    direct_arrows: &[layout::ActivityRoute],
    suppress_prev_arrow: &std::collections::BTreeSet<usize>,
    lanes: &[String],
    lane_spans: &[Option<(i32, i32)>],
    sequential_partition_lanes: bool,
    lane_area_x: i32,
    lane_w: i32,
    stacked_partition_blocks: bool,
    header_h: i32,
    lane_header_h: i32,
    width: i32,
    height: i32,
    box_w: i32,
) -> RenderScene {
    let mut scene = RenderScene::new(Rect::new(0.0, 0.0, width as f64, height as f64));

    // ---- Nodes ----
    for (i, (node, layout)) in doc.nodes.iter().zip(node_layouts.iter()).enumerate() {
        if hidden_nodes.contains(&i) {
            continue;
        }
        let step_kind = &metas[i].step_kind;
        let cx = layout.cx as f64;
        let y = layout.slot_y as f64;

        let bounds: Option<Rect> = match node.kind {
            FamilyNodeKind::ActivityStart => {
                // Filled circle: cx, y+20, r=12
                Some(Rect::new(cx - 12.0, y + 8.0, 24.0, 24.0))
            }
            FamilyNodeKind::ActivityStop => {
                match step_kind.as_str() {
                    "Kill" => Some(Rect::new(cx - 12.0, y + 8.0, 24.0, 24.0)),
                    "Detach" => Some(Rect::new(cx - 12.0, y + 14.0, 24.0, 12.0)),
                    _ => {
                        // Bull's-eye: outer r=14
                        Some(Rect::new(cx - 14.0, y + 6.0, 28.0, 28.0))
                    }
                }
            }
            FamilyNodeKind::ActivityAction => {
                if step_kind == "Connector" {
                    // Circle connector r=16
                    Some(Rect::new(cx - 16.0, y + 6.0, 32.0, 32.0))
                } else {
                    // Action box: x = cx - box_w/2, y+4, w=box_w, h=36
                    let bw = box_w as f64;
                    Some(Rect::new(cx - bw / 2.0, y + 4.0, bw, 36.0))
                }
            }
            FamilyNodeKind::Note => {
                let bw = box_w as f64;
                let note_h =
                    nodes::activity_note_card_height(node.label.as_deref().unwrap_or_default())
                        as f64;
                Some(Rect::new(cx - bw / 2.0, y + 2.0, bw, note_h))
            }
            FamilyNodeKind::ActivityDecision => {
                // Diamond: cx±100, y+2 .. y+46
                Some(Rect::new(cx - 100.0, y + 2.0, 200.0, 44.0))
            }
            FamilyNodeKind::ActivityFork | FamilyNodeKind::ActivityForkEnd => {
                if step_kind.contains("ForkAgain") {
                    None // layout bookmark only
                } else {
                    let bar_half = fork_bar_half_widths.get(&i).copied().unwrap_or(box_w / 2);
                    if bar_half <= 0 {
                        None
                    } else {
                        let bw = (bar_half * 2).max(box_w) as f64;
                        // Sync bar: x = cx - bw/2, y+24, height=8
                        Some(Rect::new(cx - bw / 2.0, y + 24.0, bw, 8.0))
                    }
                }
            }
            FamilyNodeKind::ActivityMerge => {
                // Merge nodes are invisible merge points; give them a zero-area anchor
                if step_kind.contains("Else")
                    || step_kind.contains("EndIf")
                    || step_kind.contains("EndWhile")
                    || step_kind.contains("RepeatStart")
                {
                    None
                } else {
                    Some(Rect::new(cx - 4.0, y + 20.0, 8.0, 8.0))
                }
            }
            FamilyNodeKind::ActivityPartition => None,
            _ => None,
        };

        let Some(bounds) = bounds else {
            continue;
        };

        let node_id = format!("activity-node-{i}");
        let label_text = node.label.clone().unwrap_or_default();
        let label = LabelBox {
            id: format!("{node_id}::label"),
            text: label_text,
            bounds,
            owner_id: Some(node_id.clone()),
            role: LabelRole::Node,
        };
        scene.add_node(SceneNode {
            id: node_id.clone(),
            node_box: NodeBox {
                id: node_id,
                bounds,
                ports: Vec::new(),
                labels: vec![label],
            },
        });
    }

    // ---- Predecessor edges (the main sequential flow) ----
    let mut edge_idx: usize = 0;
    for i in 1..doc.nodes.len() {
        if suppress_prev_arrow.contains(&i) {
            continue;
        }
        if matches!(
            metas[i - 1].step_kind.as_str(),
            "Else" | "EndIf" | "EndWhile"
        ) {
            continue;
        }
        // Walk back to find the real visible predecessor (same logic as nodes.rs)
        let mut prev_idx = i - 1;
        while prev_idx > 0 && layout::is_activity_flow_neutral_node(doc, metas, prev_idx) {
            prev_idx -= 1;
        }
        if matches!(
            metas[prev_idx].step_kind.as_str(),
            "Stop" | "End" | "Kill" | "Detach"
        ) && !matches!(doc.nodes[i].kind, FamilyNodeKind::Note)
        {
            continue;
        }
        let prev = &node_layouts[prev_idx];
        let cur = &node_layouts[i];
        let (x1, y1) = if metas[prev_idx].step_kind == "IfStart" && prev.cx != cur.cx {
            let side_x = if cur.cx < prev.cx {
                prev.cx - 100
            } else {
                prev.cx + 100
            };
            (side_x, prev.slot_y + 24)
        } else {
            (prev.cx, prev.arrow_out_y)
        };
        let (x2, y2) = (cur.cx, cur.slot_y);
        if x1 == x2 && y1 == y2 {
            continue;
        }
        let from_node_id = format!("activity-node-{prev_idx}");
        let to_node_id = format!("activity-node-{i}");
        let edge_id = format!("activity-edge-pred-{edge_idx}");
        edge_idx += 1;
        push_activity_edge(
            &mut scene,
            edge_id,
            from_node_id,
            to_node_id,
            x1,
            y1,
            x2,
            y2,
        );
    }

    // ---- Extra/branch edges (if/while branching) ----
    for (k, route) in redirected_extra_arrows.iter().enumerate() {
        // Skip while-loop back-edges and exit-path arrows: the SVG renderer
        // detours them around obstacle nodes using multi-segment paths.  A
        // straight 2-point approximation in the scene would cross intermediate
        // nodes and produce spurious EdgeCrossesNode geometry violations.
        if route.skip_in_scene {
            continue;
        }
        // Find which node this extra arrow targets (same dst matching as emit_extra_arrows)
        let to_idx = doc
            .nodes
            .iter()
            .zip(node_layouts.iter())
            .position(|(_, layout)| layout.cx == route.x2 && layout.slot_y == route.y2);
        let to_node_id = to_idx
            .map(|i| format!("activity-node-{i}"))
            .unwrap_or_else(|| format!("activity-extra-dst-{k}"));
        let from_node_id = format!("activity-extra-src-{k}");
        let edge_id = format!("activity-edge-extra-{k}");
        push_activity_edge(
            &mut scene,
            edge_id,
            from_node_id,
            to_node_id,
            route.x1,
            route.y1,
            route.x2,
            route.y2,
        );
    }

    // ---- Direct edges (fork-bar→branch, branch→join-bar) ----
    for (k, route) in direct_arrows.iter().enumerate() {
        let edge_id = format!("activity-edge-direct-{k}");
        push_activity_edge(
            &mut scene,
            edge_id,
            format!("activity-direct-src-{k}"),
            format!("activity-direct-dst-{k}"),
            route.x1,
            route.y1,
            route.x2,
            route.y2,
        );
    }

    // ---- Swimlanes ----
    for (idx, lane_name) in lanes.iter().enumerate() {
        let lx = if stacked_partition_blocks {
            lane_area_x
        } else {
            lane_area_x + idx as i32 * lane_w
        };

        if sequential_partition_lanes {
            let Some((span_top, span_bottom)) = lane_spans.get(idx).and_then(|s| *s) else {
                continue;
            };
            let body_y = span_top + lane_header_h;
            let body_h = (span_bottom - body_y).max(24);
            let header_rect = Rect::new(
                lx as f64,
                span_top as f64,
                lane_w as f64,
                lane_header_h as f64,
            );
            let body_rect = Rect::new(lx as f64, body_y as f64, lane_w as f64, body_h as f64);
            let lane_bounds = header_rect.union(body_rect);
            let lane_id = format!("activity-lane-{idx}");
            let label = LabelBox {
                id: format!("{lane_id}::label"),
                text: lane_name.clone(),
                bounds: header_rect,
                owner_id: Some(lane_id.clone()),
                role: LabelRole::Lane,
            };
            scene.add_lane(LaneFrame {
                id: lane_id,
                bounds: lane_bounds,
                header: Some(header_rect),
                child_node_ids: Vec::new(),
                labels: vec![label],
            });
        } else if lane_name != "default" {
            let body_y = header_h + lane_header_h;
            let body_h = height - header_h - lane_header_h - 20;
            let header_rect = Rect::new(
                lx as f64,
                header_h as f64,
                lane_w as f64,
                lane_header_h as f64,
            );
            let body_rect = Rect::new(
                lx as f64,
                body_y as f64,
                lane_w as f64,
                body_h.max(0) as f64,
            );
            let lane_bounds = header_rect.union(body_rect);
            let lane_id = format!("activity-lane-{idx}");
            let label = LabelBox {
                id: format!("{lane_id}::label"),
                text: lane_name.clone(),
                bounds: header_rect,
                owner_id: Some(lane_id.clone()),
                role: LabelRole::Lane,
            };
            scene.add_lane(LaneFrame {
                id: lane_id,
                bounds: lane_bounds,
                header: Some(header_rect),
                child_node_ids: Vec::new(),
                labels: vec![label],
            });
        }
    }

    scene
}

/// Push a simple 2-point edge (straight line) into the scene.
// Threads the edge id, endpoints, and four route coordinates straight into the
// scene; a context struct would only mirror these fields one-to-one.
#[allow(clippy::too_many_arguments)]
pub(super) fn push_activity_edge(
    scene: &mut RenderScene,
    id: String,
    from: String,
    to: String,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
) {
    let source_anchor = Anchor {
        id: format!("{id}::src"),
        owner_id: from.clone(),
        position: Point::new(x1 as f64, y1 as f64),
        port: None,
    };
    let target_anchor = Anchor {
        id: format!("{id}::tgt"),
        owner_id: to.clone(),
        position: Point::new(x2 as f64, y2 as f64),
        port: None,
    };
    scene.add_edge(SceneEdge {
        id,
        from,
        to,
        route: Polyline::from_tuples(&[(x1 as f64, y1 as f64), (x2 as f64, y2 as f64)]),
        route_channel_ids: Vec::new(),
        source_anchor,
        target_anchor,
        labels: Vec::new(),
    });
}

#[cfg(test)]
mod tests {
    use crate::model::NormalizedDocument;

    use super::super::{render_activity_artifact, render_activity_svg};

    fn parse_activity(source: &str) -> crate::model::FamilyDocument {
        let doc = crate::parse(source).expect("parse ok");
        let model = crate::normalize_family(doc).expect("normalize ok");
        match model {
            NormalizedDocument::Family(f) => f,
            other => panic!("expected NormalizedDocument::Family, got {:?}", other),
        }
    }

    /// Parse a minimal activity diagram and verify the scene node count matches
    /// the visible action/control elements, and that geometry validation is clean.
    #[test]
    fn render_activity_artifact_scene_node_count_and_geometry() {
        // Simple flow: start → action → stop  (3 visible nodes)
        let page = parse_activity("@startuml\nstart\n:Do Something;\nstop\n@enduml\n");
        let artifact = render_activity_artifact(&page);
        let scene = artifact
            .typed_scene()
            .expect("activity renderer must produce a typed scene");

        // start + action + stop = 3 visible nodes
        assert_eq!(
            scene.nodes.len(),
            3,
            "expected 3 scene nodes (start, action, stop), got {}",
            scene.nodes.len()
        );

        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "geometry validation found issues: {issues:?}"
        );
    }

    /// Verify that a diagram with swimlanes populates the scene's lanes.
    #[test]
    fn render_activity_artifact_swimlanes_in_scene() {
        let page = parse_activity(
            "@startuml\n|Lane A|\nstart\n:Step A;\n|Lane B|\n:Step B;\nstop\n@enduml\n",
        );
        let artifact = render_activity_artifact(&page);
        let scene = artifact
            .typed_scene()
            .expect("activity renderer must produce a typed scene");

        assert!(
            !scene.lanes.is_empty(),
            "expected at least one lane in scene, got none"
        );

        let issues = scene.validate_geometry();
        assert!(
            issues.is_empty(),
            "geometry validation found issues: {issues:?}"
        );
    }

    /// SVG output from render_activity_svg and render_activity_artifact must be identical.
    #[test]
    fn render_activity_svg_and_artifact_are_byte_identical() {
        let page = parse_activity(
            "@startuml\nstart\n:Step 1;\nif (condition?) then (yes)\n  :Branch A;\nelse (no)\n  :Branch B;\nendif\nstop\n@enduml\n",
        );
        let svg_direct = render_activity_svg(&page);
        let artifact = render_activity_artifact(&page);
        assert_eq!(
            svg_direct, artifact.svg,
            "render_activity_svg and render_activity_artifact must produce identical SVG"
        );
    }
}
