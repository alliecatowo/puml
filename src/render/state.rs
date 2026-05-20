mod layout;
mod support;

use super::scene_graph::Rect as SceneRect;
use super::*;
use crate::model::StateTransition;
use layout::*;
use support::*;

// Layout constants
const STATE_NODE_W: i32 = 140;
const STATE_NODE_H: i32 = 40;
const STATE_NODE_GAP_X: i32 = 60;
const STATE_NODE_GAP_Y: i32 = 60;
const STATE_MARGIN: i32 = 30;
// Removed STATE_LEFT_GUTTER: initial node placement no longer adds a fixed
// left offset. The left-edge gutter pre-pass (below) conditionally shifts all
// nodes right only when a transition label would actually clip the left edge.
// X-offset of the right-side gutter column used for sink states.
const STATE_SINK_GUTTER_GAP: i32 = 80;
const COMPOSITE_PAD_X: i32 = 16;
const COMPOSITE_PAD_Y: i32 = 36; // extra space for composite header label
const COMPOSITE_PAD_BOT: i32 = 12;
const REGION_DIVIDER_GAP: i32 = 24; // gap between concurrent regions / divider clearance
const STATE_LABEL_LINE_H: i32 = 14;
const STATE_LABEL_CHAR_W: i32 = 7;
const STATE_LABEL_NODE_CLEARANCE: i32 = 12;
const STATE_LABEL_LABEL_CLEARANCE: i32 = 8;
const STATE_LABEL_WRAP_COLS: usize = 24;

/// A placed node entry in the flat coord map.
/// Stores the node's top-left (x, y) and its full rendered size (w, h).
#[derive(Clone, Copy)]
struct PlacedNode {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

#[derive(Clone)]
struct StateLabelLayout {
    cx: i32,
    top: i32,
    lines: Vec<String>,
    bounds: LabelBounds,
}

#[derive(Clone, Copy)]
struct LabelBounds {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

fn state_geometry_bbox(x: i32, y: i32, w: i32, h: i32) -> SceneRect {
    SceneRect::new(x as f64, y as f64, w as f64, h as f64)
}

fn state_label_bounds_centered(
    cx: i32,
    baseline_y: i32,
    label: &str,
    font_size: i32,
) -> LabelBounds {
    let char_w = (font_size * 3 / 5).max(6);
    let w = (label.chars().count() as i32 * char_w).max(char_w);
    let h = font_size + 3;
    LabelBounds {
        x: cx - w / 2,
        y: baseline_y - font_size,
        w,
        h,
    }
}

fn state_label_bounds_left(x: i32, baseline_y: i32, label: &str, font_size: i32) -> LabelBounds {
    let char_w = (font_size * 3 / 5).max(6);
    LabelBounds {
        x,
        y: baseline_y - font_size,
        w: (label.chars().count() as i32 * char_w).max(char_w),
        h: font_size + 3,
    }
}

fn state_label_bbox(bounds: LabelBounds) -> SceneRect {
    state_geometry_bbox(bounds.x, bounds.y, bounds.w, bounds.h)
}

fn puml_state_node_attrs(node: &StateNode, x: i32, y: i32, w: i32, h: i32) -> String {
    puml_node_attrs(
        &node.name,
        "state",
        state_node_kind_name(&node.kind),
        state_geometry_bbox(x, y, w, h),
    )
}

fn puml_state_edge_attrs(from: &str, to: &str) -> String {
    let id = state_edge_id(from, to);
    puml_edge_attrs(&id, "state", "transition", from, to)
}

fn state_edge_id(from: &str, to: &str) -> String {
    format!("state:{from}->{to}")
}

fn puml_state_label_attrs(owner: &str, kind: &str, bounds: LabelBounds) -> String {
    puml_label_attrs(owner, kind, state_label_bbox(bounds))
}

pub fn render_state_svg(document: &StateDocument) -> String {
    let nodes = &document.nodes;
    let transitions = &document.transitions;
    let state_style = &document.state_style;

    // ── Phase 1: compute recursive layout ───────────────────────────────────
    // We use a two-column top-level layout for the outer nodes, then compute
    // each composite's size bottom-up from its children.

    // Pre-compute the set of all node names that appear as children inside
    // composite states. These nodes are positioned and rendered by their parent
    // and must be excluded from the top-level layout and rendering loops.
    // (The normalizer may add them to the flat nodes list to ensure edge routing
    // has valid endpoint coordinates, but their placement is owned by the parent.)
    fn collect_composite_children<'a>(
        node: &'a StateNode,
        set: &mut std::collections::BTreeSet<&'a str>,
    ) {
        for region in &node.regions {
            for child in region {
                set.insert(child.name.as_str());
                collect_composite_children(child, set);
            }
        }
    }
    let mut child_node_names: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for node in nodes {
        collect_composite_children(node, &mut child_node_names);
    }

    // First pass: compute sizes of all nodes recursively.
    // We build a flat map: name → PlacedNode (x, y computed in second pass).
    let mut node_sizes: std::collections::BTreeMap<String, (i32, i32)> =
        std::collections::BTreeMap::new();
    for node in nodes {
        compute_node_size(node, &mut node_sizes);
    }

    // Second pass: assign positions to top-level nodes, then recurse to assign
    // child positions relative to their parent.
    // Only position nodes that are not children of a composite.
    //
    // Layout policy:
    // - Use a single column when fork/join/choice nodes are present (linear flow).
    // - Use a single column when there are ≤ 3 top-level nodes (avoids side-by-side
    //   placement of [*] and a single composite state, fix #555).
    // - Otherwise use a 2-column grid for denser layouts.
    // In all cases, sort nodes by BFS depth from initial states.
    let top_level_nodes: Vec<&StateNode> = nodes
        .iter()
        .filter(|n| !child_node_names.contains(n.name.as_str()))
        .collect();
    let top_level_count = top_level_nodes.len();
    let has_fork_join_choice = nodes.iter().any(|n| {
        !child_node_names.contains(n.name.as_str())
            && matches!(
                n.kind,
                StateNodeKind::Fork | StateNodeKind::Join | StateNodeKind::Choice
            )
    });
    let has_top_level_composite = nodes.iter().any(|n| {
        !child_node_names.contains(n.name.as_str())
            && n.regions.iter().any(|region| !region.is_empty())
    });
    let cols: i32 = if has_fork_join_choice || has_top_level_composite || top_level_count <= 3 {
        1
    } else {
        2
    };

    // ── Sink-state heuristic ────────────────────────────────────────────────
    // Top-level nodes that have ONLY incoming error transitions (no outgoing
    // transitions that lead to non-terminal nodes) are "sink" states.  These
    // should be placed in a right-side gutter column rather than in the main
    // vertical flow, so they don't interrupt the happy path.
    //
    // A node is a sink if:
    //   - it is NOT a StartEnd / End pseudo-state (those go at top/bottom)
    //   - its out-degree is 0 OR all outgoing transitions go to [*]__end / [*]
    //   - its in-degree > 0 (at least one incoming transition)
    {
        // (computed later; pre-compute inline for sink detection)
    }
    // Build a set of explicit End-stereotype node names so the out-degree
    // computation can treat them as terminal (same as [*] pseudo-states).
    // Without this, a node whose only outgoing edge targets a `<<end>>` state
    // would be counted as having non-terminal outflow and would not be
    // classified as a sink, contradicting the heuristic's intent.
    let end_node_names: std::collections::BTreeSet<&str> = nodes
        .iter()
        .filter(|n| matches!(n.kind, StateNodeKind::End))
        .map(|n| n.name.as_str())
        .collect();
    let top_level_out_degree: std::collections::BTreeMap<&str, usize> = {
        let mut m: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
        for t in transitions {
            // Count outgoing edges that go somewhere non-terminal.
            // Both [*] pseudo-states and explicit <<end>> stereotype nodes are
            // terminal — transitions to them do not disqualify a node from being
            // a sink.
            if !t.to.starts_with("[*]") && !end_node_names.contains(t.to.as_str()) {
                *m.entry(t.from.as_str()).or_insert(0) += 1;
            }
        }
        m
    };
    let top_level_in_degree: std::collections::BTreeMap<&str, usize> = {
        let mut m: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
        for t in transitions {
            *m.entry(t.to.as_str()).or_insert(0) += 1;
        }
        m
    };
    let sink_names: std::collections::BTreeSet<&str> = top_level_nodes
        .iter()
        .filter(|n| {
            // Must be a normal state (not a pseudo-state)
            matches!(n.kind, StateNodeKind::Normal)
                // Must have incoming transitions from at least 2 different sources
                // (single-predecessor terminals like "Output" are on the happy path)
                && top_level_in_degree.get(n.name.as_str()).copied().unwrap_or(0) >= 2
                // Must have NO outgoing transitions to non-terminal nodes
                && top_level_out_degree.get(n.name.as_str()).copied().unwrap_or(0) == 0
        })
        .map(|n| n.name.as_str())
        .collect();

    // Longest-path reachability sort of top-level nodes from initial states.
    // Using the maximum depth instead of the minimum keeps sinks/final states below
    // all of their incoming branches, which avoids clipped/crossing terminal arrows.
    let name_to_orig: std::collections::BTreeMap<&str, usize> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.name.as_str(), i))
        .collect();
    let depth_map = compute_top_level_depths(&top_level_nodes, transitions, &name_to_orig);
    let mut layout_order = top_level_nodes.clone();
    layout_order.sort_by_key(|n| {
        (
            depth_map
                .get(n.name.as_str())
                .copied()
                .unwrap_or(usize::MAX),
            name_to_orig
                .get(n.name.as_str())
                .copied()
                .unwrap_or(usize::MAX),
        )
    });

    // Split into main flow and sink gutter
    let main_layout_order: Vec<&StateNode> = layout_order
        .iter()
        .copied()
        .filter(|n| !sink_names.contains(n.name.as_str()))
        .collect();
    let sink_layout_order: Vec<&StateNode> = layout_order
        .iter()
        .copied()
        .filter(|n| sink_names.contains(n.name.as_str()))
        .collect();

    let mut placed: std::collections::BTreeMap<String, PlacedNode> =
        std::collections::BTreeMap::new();

    if has_fork_join_choice {
        place_top_level_layered(
            &main_layout_order,
            &depth_map,
            &name_to_orig,
            transitions,
            &node_sizes,
            &mut placed,
        );
    } else {
        // Place top-level nodes in column order, using the BFS-sorted layout_order.
        let mut col_y = [STATE_MARGIN + 50, STATE_MARGIN + 50];
        #[allow(clippy::explicit_counter_loop)]
        {
            let mut col_idx = 0usize;
            for node in &main_layout_order {
                let col = (col_idx as i32) % cols;
                col_idx += 1;
                let x = STATE_MARGIN + col * (STATE_NODE_W + STATE_NODE_GAP_X + 80);
                let y = col_y[col as usize];
                let (w, h) = *node_sizes
                    .get(&node.name)
                    .unwrap_or(&(STATE_NODE_W, STATE_NODE_H));
                place_node(node, x, y, w, h, &node_sizes, &mut placed);
                col_y[col as usize] = y + h + STATE_NODE_GAP_Y;
            }
        }
    }

    // Place sink nodes in a right-side gutter column.
    // Each sink's Y origin is anchored to its predecessor depth so that
    // arrows from lower main-flow states don't point upward into the sink
    // gutter (which caused crossing/clipping before this fix).  We compute
    // a per-sink Y from the bottom edge of the placed predecessors that feed
    // into it; if none are placed yet we fall back to STATE_MARGIN + 50.
    if !sink_layout_order.is_empty() {
        let main_max_x = placed
            .values()
            .map(|p| p.x + p.w)
            .max()
            .unwrap_or(STATE_MARGIN + STATE_NODE_W);
        let sink_x = main_max_x + STATE_SINK_GUTTER_GAP;
        let mut sink_y = STATE_MARGIN + 50;
        for node in &sink_layout_order {
            // Find the maximum bottom-Y among all placed predecessors of this sink.
            let pred_max_bottom: Option<i32> = transitions
                .iter()
                .filter(|t| t.to == node.name)
                .filter_map(|t| placed.get(&t.from))
                .map(|p| p.y + p.h)
                .max();
            // Anchor this sink's top to the deepest predecessor's bottom (+ gap),
            // but never above the current watermark (sink_y) so sequential sinks
            // don't overlap each other.
            if let Some(pred_bottom) = pred_max_bottom {
                sink_y = sink_y.max(pred_bottom + STATE_NODE_GAP_Y);
            }
            let (w, h) = *node_sizes
                .get(&node.name)
                .unwrap_or(&(STATE_NODE_W, STATE_NODE_H));
            place_node(node, sink_x, sink_y, w, h, &node_sizes, &mut placed);
            sink_y += h + STATE_NODE_GAP_Y;
        }
        // Re-run fork/join bar-width adjustment now that sink nodes are in `placed`.
        // The earlier call inside place_top_level_layered ran before sink nodes were
        // placed, so fork/join bars whose branches include sink targets were sized
        // from an incomplete set of branch centers and could end up too narrow.
        if has_fork_join_choice {
            adjust_fork_join_bar_widths(&main_layout_order, transitions, &mut placed);
        }
    }

    // Build edge/kind lookup tables needed for both the gutter pre-pass and Phase 2.
    // Build a set of all (from, to) pairs to detect bidirectional edges
    let edge_set: std::collections::BTreeSet<(&str, &str)> = transitions
        .iter()
        .map(|t| (t.from.as_str(), t.to.as_str()))
        .collect();
    let node_kinds: std::collections::BTreeMap<&str, &StateNodeKind> = nodes
        .iter()
        .map(|node| (node.name.as_str(), &node.kind))
        .collect();

    // ── Left-edge gutter pre-pass ────────────────────────────────────────────
    // Transition labels may be placed to the left of their source/target nodes.
    // Do a dry-run of label placement to find the leftmost label x, then shift
    // all placed nodes right so no label falls outside the viewBox.
    // Only considers top-level transitions (both endpoints in `placed`); skips
    // intra-composite child→child edges which are not in the top-level placed map.
    // Uses the same anchor/offset geometry as the actual render pass so the
    // estimate of min_label_x is accurate for bidirectional and kind-specific edges.
    {
        let mut dry_occupied: Vec<LabelBounds> = Vec::new();
        let mut min_label_x = placed.values().map(|p| p.x).min().unwrap_or(0);
        for t in transitions {
            // Skip transitions where either endpoint is not a top-level placed node
            // (intra-composite child→child edges handled inside composite rendering).
            if let (Some(fp), Some(tp)) = (placed.get(&t.from), placed.get(&t.to)) {
                if let Some(label) = &t.label {
                    // Match render-pass geometry: use kind-aware anchors and offset
                    // bidirectional edges, so min_label_x is not underestimated.
                    let (x1, y1, x2, y2) = edge_anchors_for_kinds(
                        node_kinds.get(t.from.as_str()).copied(),
                        fp,
                        node_kinds.get(t.to.as_str()).copied(),
                        tp,
                    );
                    let has_reverse =
                        t.from != t.to && edge_set.contains(&(t.to.as_str(), t.from.as_str()));
                    let (lx1, ly1, lx2, ly2) = if has_reverse {
                        offset_parallel_edge(x1, y1, x2, y2, 10)
                    } else {
                        (x1, y1, x2, y2)
                    };
                    let layout = place_state_transition_label(
                        label,
                        lx1,
                        ly1,
                        lx2,
                        ly2,
                        &placed,
                        &dry_occupied,
                    );
                    min_label_x = min_label_x.min(layout.bounds.x);
                    dry_occupied.push(layout.bounds);
                }
            }
        }
        // If anything extends left of STATE_MARGIN, shift the whole coordinate
        // system right to compensate.
        let required_shift = (STATE_MARGIN - min_label_x).max(0);
        if required_shift > 0 {
            let names: Vec<String> = placed.keys().cloned().collect();
            for name in names {
                if let Some(p) = placed.get_mut(&name) {
                    p.x += required_shift;
                }
            }
        }
    }

    // Compute total canvas size from placed nodes
    let mut max_x = placed.values().map(|p| p.x + p.w).max().unwrap_or(300);
    let mut max_y = placed.values().map(|p| p.y + p.h).max().unwrap_or(200);

    // Build a map from child-node name → immediate composite-parent name.
    // This is used in the label-extent prepass to exclude the composite parent
    // from the obstacle set when simulating label placement for intra-composite
    // transitions (the parent's bounding box covers the entire interior, so
    // including it would force labels outside the composite — #709).
    fn collect_child_to_parent<'a>(
        node: &'a StateNode,
        map: &mut std::collections::BTreeMap<&'a str, &'a str>,
    ) {
        for region in &node.regions {
            for child in region {
                map.insert(child.name.as_str(), node.name.as_str());
                collect_child_to_parent(child, map);
            }
        }
    }
    let mut child_to_parent: std::collections::BTreeMap<&str, &str> =
        std::collections::BTreeMap::new();
    for node in nodes {
        collect_child_to_parent(node, &mut child_to_parent);
    }

    // Pre-pass: expand canvas to include transition label extents.
    // Labels are placed in Phase 2 but the canvas must account for their
    // right/bottom edges *before* the SVG viewBox is fixed. Without this
    // pre-pass, labels whose centers fall near the right edge are clipped
    // because only node bounding boxes contribute to max_x/max_y (#745).
    //
    // We must process transitions in the same order that Phase 2 does so that
    // `prelim_occupied` mirrors `occupied_label_bounds` at every step and the
    // simulated label positions match the real ones:
    //   1. Non-intra-composite transitions (drawn in the outer loop, before nodes)
    //   2. Intra-composite transitions (drawn inside render_node, after the outer loop)
    // Without this ordering, composite-internal labels like "done" (Working → Idle)
    // are simulated with a different occupied-set than the one used in Phase 2, so
    // their estimated position diverges and the canvas is under-sized (#709).
    {
        let mut prelim_occupied: Vec<LabelBounds> = Vec::new();
        // Helper: simulate placing one transition's label, expanding max_x/max_y.
        // `obstacle_placed` is the placed-node map to use for collision checks
        // (callers may exclude the composite parent for intra-composite transitions).
        let account_for_label =
            |t: &StateTransition,
             obstacle_placed: &std::collections::BTreeMap<String, PlacedNode>,
             max_x: &mut i32,
             max_y: &mut i32,
             prelim_occupied: &mut Vec<LabelBounds>| {
                let Some(label) = &t.label else { return };
                let from_p = placed.get(&t.from);
                let to_p = placed.get(&t.to);
                if let (Some(fp), Some(tp)) = (from_p, to_p) {
                    let has_reverse =
                        t.from != t.to && edge_set.contains(&(t.to.as_str(), t.from.as_str()));
                    let (x1, y1, x2, y2) = edge_anchors_for_kinds(
                        node_kinds.get(t.from.as_str()).copied(),
                        fp,
                        node_kinds.get(t.to.as_str()).copied(),
                        tp,
                    );
                    let (lx1, ly1, lx2, ly2) = if has_reverse {
                        offset_parallel_edge(x1, y1, x2, y2, 10)
                    } else {
                        (x1, y1, x2, y2)
                    };
                    let layout = place_state_transition_label(
                        label,
                        lx1,
                        ly1,
                        lx2,
                        ly2,
                        obstacle_placed,
                        prelim_occupied,
                    );
                    let b = layout.bounds;
                    *max_x = (*max_x).max(b.x + b.w);
                    *max_y = (*max_y).max(b.y + b.h);
                    prelim_occupied.push(b);
                }
            };
        // Pass 1: non-intra-composite transitions (outer loop order in Phase 2).
        // These use the full placed map (no parent composite to exclude).
        for t in transitions {
            if child_node_names.contains(t.from.as_str())
                && child_node_names.contains(t.to.as_str())
            {
                continue;
            }
            account_for_label(t, &placed, &mut max_x, &mut max_y, &mut prelim_occupied);
        }
        // Pass 2: intra-composite transitions (render_node order in Phase 2).
        // Replace the composite parent with thin wall slabs so labels are
        // constrained to the interior content area without being pushed outside
        // the composite boundary (#709).
        for t in transitions {
            if !child_node_names.contains(t.from.as_str())
                || !child_node_names.contains(t.to.as_str())
            {
                continue;
            }
            let parent_name = child_to_parent.get(t.from.as_str()).copied();
            let mut inner: std::collections::BTreeMap<String, PlacedNode> = placed
                .iter()
                .filter(|(k, _)| Some(k.as_str()) != parent_name)
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            if let Some(pname) = parent_name {
                if let Some(cp) = placed.get(pname) {
                    inner.insert(
                        format!("__wall_header_{}", pname),
                        PlacedNode {
                            x: cp.x,
                            y: cp.y,
                            w: cp.w,
                            h: COMPOSITE_PAD_Y,
                        },
                    );
                    inner.insert(
                        format!("__wall_footer_{}", pname),
                        PlacedNode {
                            x: cp.x,
                            y: cp.y + cp.h - COMPOSITE_PAD_BOT,
                            w: cp.w,
                            h: COMPOSITE_PAD_BOT,
                        },
                    );
                    inner.insert(
                        format!("__wall_left_{}", pname),
                        PlacedNode {
                            x: cp.x,
                            y: cp.y,
                            w: COMPOSITE_PAD_X,
                            h: cp.h,
                        },
                    );
                    inner.insert(
                        format!("__wall_right_{}", pname),
                        PlacedNode {
                            x: cp.x + cp.w - COMPOSITE_PAD_X,
                            y: cp.y,
                            w: COMPOSITE_PAD_X,
                            h: cp.h,
                        },
                    );
                }
            }
            account_for_label(t, &inner, &mut max_x, &mut max_y, &mut prelim_occupied);
        }
    }

    let width = max_x + STATE_MARGIN;
    let height = max_y + STATE_MARGIN + 12;

    // ── Phase 2: emit SVG ────────────────────────────────────────────────────
    let mut out = String::new();
    out.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        width, height, width, height
    ));
    out.push_str("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>");
    out.push_str(&format!(
        "<defs><marker id=\"arrow\" markerWidth=\"8\" markerHeight=\"8\" refX=\"6\" refY=\"3\" orient=\"auto\"><path d=\"M0,0 L0,6 L8,3 z\" fill=\"{}\"/></marker></defs>",
        state_style.arrow_color
    ));

    // Title
    let mut y_header = 28i32;
    if let Some(title) = &document.title {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"16\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
            width / 2,
            y_header,
            escape_text(&state_style.font_color),
            escape_text(title)
        ));
        y_header += 20;
    }
    let _ = y_header;

    // Compute incoming/outgoing counts for all placed nodes (for StartEnd rendering variant)
    let mut incoming: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    let mut outgoing: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for t in transitions {
        *incoming.entry(t.to.as_str()).or_insert(0) += 1;
        *outgoing.entry(t.from.as_str()).or_insert(0) += 1;
    }

    let mut occupied_label_bounds: Vec<LabelBounds> = Vec::new();

    // Draw transitions first (arrows behind nodes).
    // Intra-composite transitions (both endpoints inside the same composite) are
    // deferred to render_node so they appear above the composite background rect.
    for t in transitions {
        // Skip transitions where both endpoints are children inside composites —
        // they will be drawn by render_node after the composite background is laid.
        if child_node_names.contains(t.from.as_str()) && child_node_names.contains(t.to.as_str()) {
            continue;
        }
        let from_p = placed.get(&t.from);
        let to_p = placed.get(&t.to);
        if let (Some(fp), Some(tp)) = (from_p, to_p) {
            // Check if the reverse edge also exists (bidirectional pair)
            let has_reverse =
                t.from != t.to && edge_set.contains(&(t.to.as_str(), t.from.as_str()));
            let (x1, y1, x2, y2) = edge_anchors_for_kinds(
                node_kinds.get(t.from.as_str()).copied(),
                fp,
                node_kinds.get(t.to.as_str()).copied(),
                tp,
            );
            let stroke = escape_text(t.line_color.as_deref().unwrap_or(&state_style.arrow_color));
            let sw = t.thickness.unwrap_or(2).clamp(1, 8);
            let dash = state_dash_attr(t.dashed);
            let hidden = state_hidden_attr(t.hidden);
            let dir = state_direction_attr(t.direction.as_deref());

            if t.from == t.to {
                // Self-loop
                let loop_rx = 18;
                let loop_ry = 14;
                let cpx = x1 + loop_rx;
                let cpy = y1 - loop_ry;
                out.push_str(&format!(
                    "<path class=\"state-transition puml-edge\" {} data-state-from=\"{}\" data-state-to=\"{}\" d=\"M {x1} {y1} Q {cpx} {cpy} {x2} {y2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
                    puml_state_edge_attrs(&t.from, &t.to),
                    escape_text(&t.from), escape_text(&t.to), stroke, sw, dash, hidden, dir
                ));
                if let Some(label) = &t.label {
                    let layout = place_state_transition_label(
                        label,
                        x1,
                        y1,
                        x2,
                        y2,
                        &placed,
                        &occupied_label_bounds,
                    );
                    render_state_transition_label(
                        &mut out,
                        &layout,
                        &state_edge_id(&t.from, &t.to),
                        label,
                        &state_style.font_color,
                    );
                    occupied_label_bounds.push(layout.bounds);
                }
            } else if has_reverse {
                // Bidirectional pair: use a curved path offset to the right of the line
                // so both arrows are visible without overlapping.
                let (ox1, oy1, ox2, oy2) = offset_parallel_edge(x1, y1, x2, y2, 10);
                let cpx = (ox1 + ox2) / 2;
                let cpy = (oy1 + oy2) / 2;
                out.push_str(&format!(
                    "<path class=\"state-transition puml-edge\" {} data-state-from=\"{}\" data-state-to=\"{}\" d=\"M {} {} Q {} {} {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
                    puml_state_edge_attrs(&t.from, &t.to),
                    escape_text(&t.from), escape_text(&t.to),
                    ox1, oy1, cpx, cpy, ox2, oy2,
                    stroke, sw, dash, hidden, dir
                ));
                if let Some(label) = &t.label {
                    let layout = place_state_transition_label(
                        label,
                        ox1,
                        oy1,
                        ox2,
                        oy2,
                        &placed,
                        &occupied_label_bounds,
                    );
                    render_state_transition_label(
                        &mut out,
                        &layout,
                        &state_edge_id(&t.from, &t.to),
                        label,
                        &state_style.font_color,
                    );
                    occupied_label_bounds.push(layout.bounds);
                }
                continue;
            } else {
                emit_state_orthogonal_path(
                    &mut out,
                    &t.from,
                    &t.to,
                    x1,
                    y1,
                    x2,
                    y2,
                    &StateEdgeStyle {
                        stroke: &stroke,
                        sw: sw as u32,
                        dash,
                        hidden,
                        dir: &dir,
                    },
                );
            }

            if let Some(label) = &t.label {
                let layout = place_state_transition_label(
                    label,
                    x1,
                    y1,
                    x2,
                    y2,
                    &placed,
                    &occupied_label_bounds,
                );
                render_state_transition_label(
                    &mut out,
                    &layout,
                    &state_edge_id(&t.from, &t.to),
                    label,
                    &state_style.font_color,
                );
                occupied_label_bounds.push(layout.bounds);
            }
        }
    }

    // Draw nodes (composites drawn recursively, children inside parent box)
    for node in nodes {
        // Skip nodes that are rendered as children of a composite
        // (child_node_names was computed before placement and rendering loops)
        if child_node_names.contains(node.name.as_str()) {
            continue;
        }
        if let Some(p) = placed.get(&node.name) {
            let inc = *incoming.get(node.name.as_str()).unwrap_or(&0);
            let out_c = *outgoing.get(node.name.as_str()).unwrap_or(&0);
            render_node(
                &mut out,
                node,
                p.x,
                p.y,
                p.w,
                p.h,
                state_style,
                inc,
                out_c,
                &placed,
                &incoming,
                &outgoing,
                transitions,
                &edge_set,
                &node_kinds,
                &mut occupied_label_bounds,
            );
        }
    }

    out.push_str("</svg>");
    out
}
