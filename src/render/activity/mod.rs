use super::layout_constants::{
    ACTIVITY_BASE_LANE_WIDTH, ACTIVITY_BRANCH_X_OFFSET, ACTIVITY_LANE_AREA_X, ACTIVITY_STEP_HEIGHT,
};
use super::svg::escape_text;
use crate::model::{FamilyDocument, FamilyNodeKind, FamilyStyle};
use crate::output::RenderArtifact;
use crate::render_core::{
    Anchor, LabelBox, LabelRole, LaneFrame, NodeBox, Point, Polyline, Rect, RenderScene, SceneEdge,
    SceneNode,
};
use crate::theme::ActivityStyle;

mod arrows;
mod branches;
mod layout;
mod nodes;
mod swimlanes;

pub fn render_activity_svg(doc: &FamilyDocument) -> String {
    render_activity_artifact(doc).svg
}

/// Render an activity diagram into a typed [`RenderArtifact`].
///
/// The SVG output is byte-identical to the legacy `render_activity_svg`. We
/// additionally build a [`RenderScene`] from the *exact* geometry the SVG uses
/// — node boxes at their drawn positions/sizes, edges along the same routing
/// coordinates the SVG draws — so the scene and the visual output never diverge.
pub fn render_activity_artifact(doc: &FamilyDocument) -> RenderArtifact {
    // -----------------------------------------------------------------------
    // 1. Style + global metrics
    // -----------------------------------------------------------------------
    let act_style = match &doc.family_style {
        Some(FamilyStyle::Activity(s)) => s.clone(),
        _ => ActivityStyle::default(),
    };

    let step_h = ACTIVITY_STEP_HEIGHT;
    let title_lines = doc
        .title
        .as_deref()
        .map(|v| v.lines().count() as i32)
        .unwrap_or(0);
    let header_h = 40 + title_lines * 22;

    // -----------------------------------------------------------------------
    // 2. Pass 0 — parse node metadata
    // -----------------------------------------------------------------------
    let metas = layout::parse_node_metas(doc);

    // -----------------------------------------------------------------------
    // 3. Collect swim-lanes
    // -----------------------------------------------------------------------
    let mut lanes: Vec<String> = Vec::new();
    for meta in &metas {
        if meta.lane_name != "default" && !lanes.iter().any(|l| l == &meta.lane_name) {
            lanes.push(meta.lane_name.clone());
        }
    }
    if lanes.is_empty() {
        lanes.push("default".to_string());
    }
    let mut lane_fills = std::collections::BTreeMap::new();
    for (node, meta) in doc.nodes.iter().zip(metas.iter()) {
        if meta.step_kind == "PartitionStart" && meta.lane_name != "default" {
            if let Some(fill) = &node.fill_color {
                lane_fills
                    .entry(meta.lane_name.clone())
                    .or_insert(fill.clone());
            }
        }
    }

    // -----------------------------------------------------------------------
    // 4. Canvas sizing
    // -----------------------------------------------------------------------
    // Count max nesting depth of if/else to estimate extra width needed.
    let mut max_if_depth: i32 = 0;
    {
        let mut depth: i32 = 0;
        for meta in &metas {
            if meta.step_kind == "IfStart" {
                depth += 1;
                max_if_depth = max_if_depth.max(depth);
            } else if meta.step_kind == "EndIf" {
                depth = depth.saturating_sub(1);
            }
        }
    }
    // Count max fork branch count to size canvas for parallel columns.
    let mut max_fork_branches: i32 = 0;
    {
        let mut count: i32 = 0;
        for meta in &metas {
            if meta.step_kind == "Fork" {
                count = 1;
            } else if meta.step_kind == "ForkAgain" {
                count += 1;
                max_fork_branches = max_fork_branches.max(count);
            } else if meta.step_kind == "EndFork" {
                count = 0;
            }
        }
    }
    let branch_x_offset = ACTIVITY_BRANCH_X_OFFSET;
    let extra_branch_width = 2 * branch_x_offset * max_if_depth;
    let extra_fork_width = (max_fork_branches * ACTIVITY_BRANCH_X_OFFSET).max(0);

    let has_left_notes = metas
        .iter()
        .any(|meta| meta.step_kind == "Note" && meta.note_side.as_deref() == Some("left"));
    let has_right_notes = metas
        .iter()
        .any(|meta| meta.step_kind == "Note" && meta.note_side.as_deref() != Some("left"));
    let side_note_margin = 260;
    let lane_area_x = ACTIVITY_LANE_AREA_X + if has_left_notes { side_note_margin } else { 0 };
    let base_lane_area_w = ACTIVITY_BASE_LANE_WIDTH;
    let lane_area_w = base_lane_area_w + extra_branch_width + extra_fork_width;
    let width = lane_area_x + lane_area_w + 32 + if has_right_notes { side_note_margin } else { 0 };
    let has_named_lanes = lanes.iter().any(|l| l != "default");
    let has_partition_markers = metas.iter().any(|meta| meta.step_kind == "PartitionStart");
    let has_partition_blocks = metas.iter().any(|meta| meta.step_kind == "PartitionEnd");
    // `partition Name { ... }` is a stacked group, while open-ended `|Lane|`
    // markers keep their existing lane-column behavior.
    let stacked_partition_blocks = has_named_lanes && has_partition_blocks;
    let lane_w = if stacked_partition_blocks {
        lane_area_w
    } else {
        (lane_area_w / (lanes.len() as i32)).max(120)
    };

    let lane_index = |name: &str| -> i32 {
        lanes
            .iter()
            .position(|l| l == name)
            .map(|i| i as i32)
            .unwrap_or(0)
    };
    let lane_center_x = |lane_name: &str| -> i32 {
        if stacked_partition_blocks {
            return lane_area_x + lane_area_w / 2;
        }
        let idx = lane_index(lane_name);
        lane_area_x + idx * lane_w + lane_w / 2
    };

    let lane_header_h = if has_named_lanes { 24i32 } else { 0i32 };
    let sequential_partition_lanes = has_named_lanes && has_partition_markers;

    let fork_col_w = (lane_w / 2).max(160i32);
    let box_w = (lane_w - 24).clamp(120, 220);

    // -----------------------------------------------------------------------
    // 5. Pass 1 — layout
    // -----------------------------------------------------------------------
    let layout_result = layout::compute_layout(
        doc,
        &metas,
        &layout::LayoutParams {
            header_h,
            lane_header_h,
            step_h,
            branch_x_offset,
            fork_col_w,
            lane_w,
            lane_center_x: &lane_center_x,
        },
    );
    let layout::LayoutResult {
        mut node_layouts,
        fork_bar_half_widths,
        extra_arrows,
        direct_arrows,
        mut suppress_prev_arrow,
    } = layout_result;

    // -----------------------------------------------------------------------
    // 6. Hidden-node deduplication pass
    // -----------------------------------------------------------------------
    let hidden_nodes =
        branches::compute_hidden_nodes(doc, &metas, &mut node_layouts, &mut suppress_prev_arrow);

    // -----------------------------------------------------------------------
    // 7. Extra-arrow redirect pass
    // -----------------------------------------------------------------------
    let redirected_extra_arrows =
        branches::redirect_extra_arrows(doc, &metas, &node_layouts, extra_arrows, &hidden_nodes);

    // -----------------------------------------------------------------------
    // 8. Canvas height + lane spans
    // -----------------------------------------------------------------------
    let height = node_layouts
        .iter()
        .map(|l| l.next_slot_y)
        .max()
        .unwrap_or(header_h + step_h)
        + 60;

    let lane_spans = if sequential_partition_lanes {
        swimlanes::compute_lane_spans(
            doc,
            &metas,
            &node_layouts,
            &lanes,
            &lane_index,
            lane_header_h,
            header_h,
            height,
        )
    } else {
        vec![None; lanes.len()]
    };

    // -----------------------------------------------------------------------
    // 9. Build obstacle bboxes for arrow routing (#734).
    //
    // Collect the bounding boxes of every visible node so that
    // emit_activity_arrow can choose a mid_y that does not cross any node body.
    // -----------------------------------------------------------------------
    let node_bboxes: Vec<arrows::NodeBbox> = doc
        .nodes
        .iter()
        .zip(node_layouts.iter())
        .zip(metas.iter())
        .filter_map(|((node, layout), meta)| {
            let cx = layout.cx;
            let y = layout.slot_y;
            match node.kind {
                FamilyNodeKind::ActivityAction => Some(arrows::NodeBbox {
                    left: cx - box_w / 2,
                    top: y + 4,
                    right: cx + box_w / 2,
                    bottom: y + 40,
                }),
                FamilyNodeKind::Note => Some(arrows::NodeBbox {
                    left: cx - box_w / 2,
                    top: y + 2,
                    right: cx + box_w / 2,
                    bottom: y
                        + 2
                        + nodes::activity_note_card_height(
                            node.label.as_deref().unwrap_or_default(),
                        ),
                }),
                FamilyNodeKind::ActivityDecision => Some(arrows::NodeBbox {
                    left: cx - 100,
                    top: y + 2,
                    right: cx + 100,
                    bottom: y + 46,
                }),
                FamilyNodeKind::ActivityStart => Some(arrows::NodeBbox {
                    left: cx - 12,
                    top: y + 8,
                    right: cx + 12,
                    bottom: y + 32,
                }),
                FamilyNodeKind::ActivityStop => Some(arrows::NodeBbox {
                    left: cx - 14,
                    top: y + 6,
                    right: cx + 14,
                    bottom: y + 34,
                }),
                FamilyNodeKind::ActivityFork | FamilyNodeKind::ActivityForkEnd => {
                    if meta.step_kind.contains("ForkAgain") {
                        None
                    } else {
                        Some(arrows::NodeBbox {
                            left: cx - box_w / 2,
                            top: y + 24,
                            right: cx + box_w / 2,
                            bottom: y + 32,
                        })
                    }
                }
                _ => None,
            }
        })
        .collect();

    // -----------------------------------------------------------------------
    // 10. Emit SVG
    // -----------------------------------------------------------------------
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str(&format!(
        "<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>",
        escape_text(&act_style.background_color)
    ));

    // Title block
    let mut y_cursor = 28;
    if let Some(title) = &doc.title {
        for line in title.lines() {
            out.push_str(&format!(
                "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"18\" font-weight=\"600\" fill=\"{}\">{}</text>",
                y_cursor,
                escape_text(&act_style.font_color),
                escape_text(line)
            ));
            y_cursor += 22;
        }
    }
    out.push_str(&format!(
        "<text x=\"32\" y=\"{}\" font-family=\"monospace\" font-size=\"12\" fill=\"{}\">activity diagram</text>",
        y_cursor + 2,
        escape_text(&act_style.font_color)
    ));

    // Swim-lane backgrounds + headers
    swimlanes::emit_lanes(
        &mut out,
        &lanes,
        &lane_spans,
        sequential_partition_lanes,
        lane_area_x,
        lane_w,
        stacked_partition_blocks,
        header_h,
        lane_header_h,
        height,
        &act_style,
        &lane_fills,
    );

    // Pass 2: nodes + arrows
    for i in 0..doc.nodes.len() {
        nodes::render_node(
            &mut out,
            doc,
            i,
            &node_layouts,
            &metas,
            &hidden_nodes,
            &fork_bar_half_widths,
            &act_style,
            box_w,
        );

        nodes::emit_predecessor_arrow(
            &mut out,
            doc,
            i,
            &node_layouts,
            &metas,
            &suppress_prev_arrow,
            &act_style,
            &node_bboxes,
        );

        // Extra arrows for if-branching that target this node
        let layout = &node_layouts[i];
        arrows::emit_extra_arrows(
            &mut out,
            &redirected_extra_arrows,
            layout.cx,
            layout.slot_y,
            &act_style.arrow_color,
            &node_bboxes,
        );
    }

    // Direct arrows: fork-bar→branch and branch→join-bar
    arrows::emit_direct_arrows(
        &mut out,
        &direct_arrows,
        &act_style.arrow_color,
        &node_bboxes,
    );

    out.push_str("</svg>");

    // -----------------------------------------------------------------------
    // 11. Build typed RenderScene from the SAME geometry the SVG uses
    // -----------------------------------------------------------------------
    let scene = build_activity_scene(
        doc,
        &metas,
        &node_layouts,
        &hidden_nodes,
        &fork_bar_half_widths,
        &redirected_extra_arrows,
        &direct_arrows,
        &suppress_prev_arrow,
        &lanes,
        &lane_spans,
        sequential_partition_lanes,
        lane_area_x,
        lane_w,
        stacked_partition_blocks,
        header_h,
        lane_header_h,
        width,
        height,
        box_w,
    );

    RenderArtifact::with_scene(out, scene)
}

/// Build a typed [`RenderScene`] mirroring the activity SVG's geometry.
///
/// Every node box uses the *exact* bounds the SVG renders (center x from
/// `NodeLayout.cx`, top from `NodeLayout.slot_y`, same w/h constants the SVG
/// shapes use).  Every edge is a polyline through the same (x1,y1)→(x2,y2)
/// coordinates used to draw the arrow.  Swimlanes are captured as
/// [`LaneFrame`]s from the lane span data.  This guarantees scene ↔ SVG
/// consistency with no separate layout pass.
#[allow(clippy::too_many_arguments)]
fn build_activity_scene(
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
fn push_activity_edge(
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
    use super::*;
    use crate::model::NormalizedDocument;

    fn parse_activity(source: &str) -> FamilyDocument {
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
