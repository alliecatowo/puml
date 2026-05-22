use super::*;
use crate::model::StateTransition;

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
const STATE_NOTE_FILL: &str = "#fff8c4";
const STATE_NOTE_BORDER: &str = "#111111";
const STATE_NOTE_PAD_X: i32 = 10;
const STATE_NOTE_PAD_Y: i32 = 10;

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
        .filter(|n| !child_node_names.contains(n.name.as_str()) && n.kind != StateNodeKind::Note)
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
        .filter(|n| n.kind != StateNodeKind::Note)
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

    position_state_notes(nodes, transitions, &node_sizes, &mut placed);

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
    if let Some(min_y) = placed.values().map(|p| p.y).min() {
        let required_shift = (STATE_MARGIN - min_y).max(0);
        if required_shift > 0 {
            for p in placed.values_mut() {
                p.y += required_shift;
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
            if matches!(
                node_kinds.get(t.to.as_str()).copied(),
                Some(StateNodeKind::Note)
            ) {
                emit_state_note_connector(
                    &mut out,
                    t,
                    fp,
                    tp,
                    &placed,
                    &node_kinds,
                    &state_style.arrow_color,
                );
                continue;
            }
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
                    "<path class=\"state-transition\" data-state-from=\"{}\" data-state-to=\"{}\" d=\"M {x1} {y1} Q {cpx} {cpy} {x2} {y2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
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
                    "<path class=\"state-transition\" data-state-from=\"{}\" data-state-to=\"{}\" d=\"M {} {} Q {} {} {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
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
                render_state_transition_label(&mut out, &layout, label, &state_style.font_color);
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

/// Compute the rendered (w, h) of a node, recursively for composites.
/// Stores results in `sizes` map (keyed by node name).
fn compute_node_size(
    node: &StateNode,
    sizes: &mut std::collections::BTreeMap<String, (i32, i32)>,
) -> (i32, i32) {
    let result = match node.kind {
        StateNodeKind::Fork | StateNodeKind::Join => (STATE_NODE_W, 8),
        StateNodeKind::Choice => (44, 44),
        StateNodeKind::HistoryShallow | StateNodeKind::HistoryDeep => (34, 34),
        StateNodeKind::EntryPoint | StateNodeKind::ExitPoint => (26, 26),
        StateNodeKind::InputPin | StateNodeKind::OutputPin => (34, 34),
        StateNodeKind::ExpansionInput | StateNodeKind::ExpansionOutput => (46, 30),
        StateNodeKind::StartEnd | StateNodeKind::End => (26, 26),
        StateNodeKind::Note => {
            let lines = node_display_lines(node);
            let max_cols = lines
                .iter()
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(4);
            let w = (max_cols as i32 * STATE_LABEL_CHAR_W + STATE_NOTE_PAD_X * 2).max(96);
            let h = (lines.len() as i32 * STATE_LABEL_LINE_H + STATE_NOTE_PAD_Y * 2).max(44);
            (w, h)
        }
        StateNodeKind::JsonProjection => {
            let lines = node_display_lines(node);
            let max_cols = lines
                .iter()
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(6);
            let w = (max_cols as i32 * STATE_LABEL_CHAR_W + STATE_NOTE_PAD_X * 2).max(150);
            let h = (lines.len() as i32 * STATE_LABEL_LINE_H + STATE_NOTE_PAD_Y * 2).max(58);
            (w, h)
        }
        StateNodeKind::Normal => {
            let has_children = node.regions.iter().any(|r| !r.is_empty());

            if !has_children {
                // Simple state box
                let actions_h = (node.internal_actions.len() as i32) * 14;
                (STATE_NODE_W, STATE_NODE_H + actions_h)
            } else {
                // Composite state: size from children
                let n_regions = node.regions.len().max(1) as i32;
                if n_regions > 1 {
                    let (column_w, content_h) = concurrent_region_metrics(&node.regions, sizes);
                    let content_w = column_w * n_regions + REGION_DIVIDER_GAP * (n_regions - 1);
                    let w = content_w + COMPOSITE_PAD_X * 2;
                    let h = content_h + COMPOSITE_PAD_Y + COMPOSITE_PAD_BOT;
                    (w.max(STATE_NODE_W), h.max(STATE_NODE_H + 20))
                } else {
                    let mut total_w = STATE_NODE_W;
                    let mut total_h = 0i32;
                    for region in &node.regions {
                        let (rw, rh) = compute_region_size(region, sizes);
                        total_w = total_w.max(rw + COMPOSITE_PAD_X * 2);
                        total_h += rh;
                    }
                    let w = total_w;
                    let h = total_h + COMPOSITE_PAD_Y + COMPOSITE_PAD_BOT;
                    (w.max(STATE_NODE_W), h.max(STATE_NODE_H + 20))
                }
            }
        }
    };
    sizes.insert(node.name.clone(), result);
    result
}

/// Compute the (w, h) needed to lay out all nodes in a region (vertical stack).
fn compute_region_size(
    region: &[StateNode],
    sizes: &mut std::collections::BTreeMap<String, (i32, i32)>,
) -> (i32, i32) {
    let mut max_w = 0i32;
    let mut total_h = 0i32;
    for (i, child) in region.iter().enumerate() {
        let (cw, ch) = compute_node_size(child, sizes);
        max_w = max_w.max(cw);
        total_h += ch;
        if i + 1 < region.len() {
            total_h += STATE_NODE_GAP_Y;
        }
    }
    (max_w, total_h)
}

fn node_display_lines(node: &StateNode) -> Vec<String> {
    let text = node.display.as_deref().unwrap_or(&node.name);
    let lines: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect();
    if lines.is_empty() {
        vec![node.name.clone()]
    } else {
        lines
    }
}

fn concurrent_region_metrics(
    regions: &[Vec<StateNode>],
    sizes: &std::collections::BTreeMap<String, (i32, i32)>,
) -> (i32, i32) {
    let column_w = regions
        .iter()
        .flat_map(|region| region.iter())
        .filter_map(|child| sizes.get(&child.name).copied())
        .map(|(w, _)| w)
        .max()
        .unwrap_or(STATE_NODE_W);
    let content_h = regions
        .iter()
        .map(|region| {
            region
                .iter()
                .enumerate()
                .map(|(idx, child)| {
                    let (_, ch) = sizes
                        .get(&child.name)
                        .copied()
                        .unwrap_or((STATE_NODE_W, STATE_NODE_H));
                    if idx + 1 < region.len() {
                        ch + STATE_NODE_GAP_Y
                    } else {
                        ch
                    }
                })
                .sum::<i32>()
        })
        .max()
        .unwrap_or(STATE_NODE_H);
    (column_w, content_h)
}

/// Place a node and all its children into the `placed` map.
fn place_node(
    node: &StateNode,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    sizes: &std::collections::BTreeMap<String, (i32, i32)>,
    placed: &mut std::collections::BTreeMap<String, PlacedNode>,
) {
    placed.insert(node.name.clone(), PlacedNode { x, y, w, h });
    // For metadata emission
    let _ = state_node_kind_name(&node.kind);

    let has_children = node.regions.iter().any(|r| !r.is_empty());
    if node.kind == StateNodeKind::Normal && has_children {
        // Place children within the composite box.
        // Children start after the composite header label area.
        if node.regions.len() > 1 {
            let (column_w, _) = concurrent_region_metrics(&node.regions, sizes);
            let mut region_x = x + COMPOSITE_PAD_X;
            let content_top = y + COMPOSITE_PAD_Y;
            for region in &node.regions {
                let mut child_y = content_top;
                for (ci, child) in region.iter().enumerate() {
                    let (cw, ch) = sizes
                        .get(&child.name)
                        .copied()
                        .unwrap_or((STATE_NODE_W, STATE_NODE_H));
                    let cx = region_x + (column_w - cw) / 2;
                    place_node(child, cx, child_y, cw, ch, sizes, placed);
                    child_y += ch;
                    if ci + 1 < region.len() {
                        child_y += STATE_NODE_GAP_Y;
                    }
                }
                region_x += column_w + REGION_DIVIDER_GAP;
            }
        } else {
            let mut child_y = y + COMPOSITE_PAD_Y;
            for region in &node.regions {
                let region_x = x + COMPOSITE_PAD_X;
                let avail_w = w - COMPOSITE_PAD_X * 2;
                for (ci, child) in region.iter().enumerate() {
                    let (cw, ch) = sizes
                        .get(&child.name)
                        .copied()
                        .unwrap_or((STATE_NODE_W, STATE_NODE_H));
                    let cx = x + COMPOSITE_PAD_X + (avail_w - cw) / 2;
                    let cx = cx.max(region_x);
                    place_node(child, cx, child_y, cw, ch, sizes, placed);
                    child_y += ch;
                    if ci + 1 < region.len() {
                        child_y += STATE_NODE_GAP_Y;
                    }
                }
            }
        }
    }
}

fn position_state_notes(
    nodes: &[StateNode],
    transitions: &[StateTransition],
    sizes: &std::collections::BTreeMap<String, (i32, i32)>,
    placed: &mut std::collections::BTreeMap<String, PlacedNode>,
) {
    let note_names: std::collections::BTreeSet<&str> = nodes
        .iter()
        .filter(|node| node.kind == StateNodeKind::Note)
        .map(|node| node.name.as_str())
        .collect();

    for t in transitions {
        if !note_names.contains(t.to.as_str()) {
            continue;
        }
        let (note_w, note_h) = sizes
            .get(&t.to)
            .copied()
            .unwrap_or((STATE_NODE_W, STATE_NODE_H));

        let mut link_segment = None;
        let (position, anchor_x, anchor_y, target_box) = if let Some((position, target)) =
            parse_state_note_on_link_direction(t.direction.as_deref())
        {
            let Some(from_p) = placed.get(&t.from) else {
                continue;
            };
            let Some(to_p) = placed.get(target) else {
                continue;
            };
            let (x1, y1, x2, y2) = edge_anchors(from_p, to_p);
            link_segment = Some((x1, y1, x2, y2));
            (position, (x1 + x2) / 2, (y1 + y2) / 2, None)
        } else {
            let Some(target_p) = placed.get(&t.from) else {
                continue;
            };
            (
                t.direction.as_deref().unwrap_or("right"),
                target_p.x + target_p.w / 2,
                target_p.y + target_p.h / 2,
                Some(*target_p),
            )
        };

        let gap = 28;
        let (x, mut y) = if let Some(target_p) = target_box {
            match position.to_ascii_lowercase().as_str() {
                "left" => (target_p.x - note_w - gap, anchor_y - note_h / 2),
                "top" | "over" => (anchor_x - note_w / 2, target_p.y - note_h - gap),
                "bottom" => (anchor_x - note_w / 2, target_p.y + target_p.h + gap),
                _ => (target_p.x + target_p.w + gap, anchor_y - note_h / 2),
            }
        } else {
            let vertical_link = link_segment
                .map(|(x1, y1, x2, y2)| (y2 - y1).abs() >= (x2 - x1).abs())
                .unwrap_or(false);
            match position.to_ascii_lowercase().as_str() {
                "left" => (anchor_x - note_w - gap, anchor_y - note_h / 2),
                "top" | "over" if vertical_link => (anchor_x + gap, anchor_y - note_h / 2),
                "top" | "over" => (anchor_x - note_w / 2, anchor_y - note_h - gap),
                "bottom" => (anchor_x - note_w / 2, anchor_y + gap),
                _ => (anchor_x + gap, anchor_y - note_h / 2),
            }
        };
        while placed
            .iter()
            .any(|(name, other)| name != &t.to && rects_overlap(x, y, note_w, note_h, other))
        {
            y += note_h + 12;
        }
        placed.insert(
            t.to.clone(),
            PlacedNode {
                x,
                y,
                w: note_w,
                h: note_h,
            },
        );
    }
}

fn rects_overlap(x: i32, y: i32, w: i32, h: i32, other: &PlacedNode) -> bool {
    x < other.x + other.w && x + w > other.x && y < other.y + other.h && y + h > other.y
}

fn compute_top_level_depths<'a>(
    top_level_nodes: &[&'a StateNode],
    transitions: &'a [StateTransition],
    name_to_orig: &std::collections::BTreeMap<&'a str, usize>,
) -> std::collections::BTreeMap<&'a str, usize> {
    let mut depth_map: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    let top_level_names: std::collections::BTreeSet<&str> =
        top_level_nodes.iter().map(|n| n.name.as_str()).collect();
    let transition_targets: std::collections::BTreeSet<&str> = transitions
        .iter()
        .filter(|t| top_level_names.contains(t.to.as_str()))
        .map(|t| t.to.as_str())
        .collect();
    let all_node_names: Vec<&str> = top_level_nodes.iter().map(|n| n.name.as_str()).collect();
    let mut adjacency: std::collections::BTreeMap<&str, Vec<&str>> =
        std::collections::BTreeMap::new();
    for t in transitions {
        if top_level_names.contains(t.from.as_str()) && top_level_names.contains(t.to.as_str()) {
            adjacency
                .entry(t.from.as_str())
                .or_default()
                .push(t.to.as_str());
        }
    }

    fn walk_longest_depth<'a>(
        name: &'a str,
        depth: usize,
        adjacency: &std::collections::BTreeMap<&'a str, Vec<&'a str>>,
        depth_map: &mut std::collections::BTreeMap<&'a str, usize>,
        path: &mut std::collections::BTreeSet<&'a str>,
    ) {
        if depth_map.get(name).copied().unwrap_or(0) >= depth {
            return;
        }
        depth_map.insert(name, depth);
        if !path.insert(name) {
            return;
        }
        if let Some(targets) = adjacency.get(name) {
            for &target in targets {
                if !path.contains(target) {
                    walk_longest_depth(target, depth + 1, adjacency, depth_map, path);
                }
            }
        }
        path.remove(name);
    }

    let mut seeds: Vec<&str> = all_node_names
        .iter()
        .copied()
        .filter(|name| *name == "[*]" || !transition_targets.contains(name))
        .collect();
    if seeds.is_empty() {
        seeds = all_node_names.clone();
    }
    seeds.sort_by_key(|name| name_to_orig.get(name).copied().unwrap_or(usize::MAX));
    for seed in seeds {
        let mut path = std::collections::BTreeSet::new();
        walk_longest_depth(seed, 1, &adjacency, &mut depth_map, &mut path);
    }
    for &name in &all_node_names {
        depth_map.entry(name).or_insert(usize::MAX);
    }
    depth_map
}

fn place_top_level_layered<'a>(
    layout_order: &[&'a StateNode],
    depth_map: &std::collections::BTreeMap<&'a str, usize>,
    name_to_orig: &std::collections::BTreeMap<&'a str, usize>,
    transitions: &'a [StateTransition],
    node_sizes: &std::collections::BTreeMap<String, (i32, i32)>,
    placed: &mut std::collections::BTreeMap<String, PlacedNode>,
) {
    let top_level_names: std::collections::BTreeSet<&str> =
        layout_order.iter().map(|node| node.name.as_str()).collect();
    let mut predecessors: std::collections::BTreeMap<&str, Vec<&str>> =
        std::collections::BTreeMap::new();
    for t in transitions {
        if top_level_names.contains(t.from.as_str()) && top_level_names.contains(t.to.as_str()) {
            predecessors
                .entry(t.to.as_str())
                .or_default()
                .push(t.from.as_str());
        }
    }

    let mut rows: std::collections::BTreeMap<usize, Vec<&StateNode>> =
        std::collections::BTreeMap::new();
    for node in layout_order {
        rows.entry(*depth_map.get(node.name.as_str()).unwrap_or(&usize::MAX))
            .or_default()
            .push(*node);
    }

    let default_center = STATE_MARGIN + STATE_NODE_W + STATE_NODE_GAP_X;
    let mut row_y = STATE_MARGIN + 50;

    for row_nodes in rows.values_mut() {
        row_nodes.sort_by_key(|node| {
            let desired =
                desired_state_center(node.name.as_str(), &predecessors, placed, default_center);
            (
                desired,
                name_to_orig
                    .get(node.name.as_str())
                    .copied()
                    .unwrap_or(usize::MAX),
            )
        });

        let row_h = row_nodes
            .iter()
            .map(|node| {
                node_sizes
                    .get(&node.name)
                    .copied()
                    .unwrap_or((STATE_NODE_W, STATE_NODE_H))
                    .1
            })
            .max()
            .unwrap_or(STATE_NODE_H);

        let mut placements: Vec<(&StateNode, i32, i32, i32)> = Vec::new();
        let mut right_edge: Option<i32> = None;
        let mut desired_centers = Vec::new();

        for node in row_nodes.iter().copied() {
            let (w, h) = node_sizes
                .get(&node.name)
                .copied()
                .unwrap_or((STATE_NODE_W, STATE_NODE_H));
            let desired_center =
                desired_state_center(node.name.as_str(), &predecessors, placed, default_center);
            desired_centers.push(desired_center);
            let min_x = right_edge
                .map(|edge| edge + STATE_NODE_GAP_X)
                .unwrap_or(i32::MIN / 4);
            let x = (desired_center - w / 2).max(min_x);
            right_edge = Some(x + w);
            placements.push((node, x, w, h));
        }

        if placements.len() > 1 {
            let desired_cluster_center =
                desired_centers.iter().sum::<i32>() / desired_centers.len() as i32;
            let actual_left = placements.first().map(|(_, x, _, _)| *x).unwrap_or(0);
            let actual_right = placements
                .last()
                .map(|(_, x, w, _)| *x + *w)
                .unwrap_or(actual_left);
            let shift = desired_cluster_center - ((actual_left + actual_right) / 2);
            if shift != 0 {
                for (_, x, _, _) in &mut placements {
                    *x += shift;
                }
            }
        }

        for (node, x, w, h) in placements {
            let y = row_y + (row_h - h) / 2;
            place_node(node, x, y, w, h, node_sizes, placed);
        }
        row_y += row_h + STATE_NODE_GAP_Y;
    }

    adjust_fork_join_bar_widths(layout_order, transitions, placed);
}

fn desired_state_center(
    node_name: &str,
    predecessors: &std::collections::BTreeMap<&str, Vec<&str>>,
    placed: &std::collections::BTreeMap<String, PlacedNode>,
    default_center: i32,
) -> i32 {
    let Some(preds) = predecessors.get(node_name) else {
        return default_center;
    };
    let mut sum = 0i32;
    let mut count = 0i32;
    for pred in preds {
        if let Some(node) = placed.get(*pred) {
            sum += node.x + node.w / 2;
            count += 1;
        }
    }
    if count == 0 {
        default_center
    } else {
        sum / count
    }
}

fn adjust_fork_join_bar_widths<'a>(
    nodes: &[&'a StateNode],
    transitions: &'a [StateTransition],
    placed: &mut std::collections::BTreeMap<String, PlacedNode>,
) {
    for node in nodes {
        let branch_centers: Vec<i32> = match node.kind {
            StateNodeKind::Fork => transitions
                .iter()
                .filter(|t| t.from == node.name)
                .filter_map(|t| placed.get(&t.to))
                .map(|p| p.x + p.w / 2)
                .collect(),
            StateNodeKind::Join => transitions
                .iter()
                .filter(|t| t.to == node.name)
                .filter_map(|t| placed.get(&t.from))
                .map(|p| p.x + p.w / 2)
                .collect(),
            _ => continue,
        };

        if branch_centers.len() < 2 {
            continue;
        }

        let left = branch_centers.iter().min().copied().unwrap_or(0);
        let right = branch_centers.iter().max().copied().unwrap_or(left);
        if let Some(bar) = placed.get_mut(&node.name) {
            let width = (right - left).max(48);
            let center = (left + right) / 2;
            bar.w = width;
            bar.x = center - width / 2;
        }
    }
}

/// SVG style attributes bundled together for the orthogonal-path emitter.
struct StateEdgeStyle<'a> {
    stroke: &'a str,
    sw: u32,
    dash: &'a str,
    hidden: &'a str,
    dir: &'a str,
}

/// Emit an SVG `<path>` element that routes a state transition orthogonally
/// (L-shaped / Z-shaped elbow) rather than as a straight diagonal.
///
/// Routing rules (same logic as the activity renderer):
/// - Same X or same Y: emit a straight line segment.
/// - Otherwise: route via a symmetric mid-point bend
///   `(x1,y1) → (x1,mid_y) → (x2,mid_y) → (x2,y2)`.
///
/// The path carries the same SVG attributes (stroke, stroke-width, dash, hidden,
/// direction, data-* labels, marker-end) as the old `<line>` element.
// Style attrs are already grouped into `StateEdgeStyle`; the remaining args are
// the mandatory out-buffer, two name strings, and four coordinate scalars — there
// is no meaningful grouping that would reduce the count further without obfuscating
// the call sites.
#[allow(clippy::too_many_arguments)]
fn emit_state_orthogonal_path(
    out: &mut String,
    from_name: &str,
    to_name: &str,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    style: &StateEdgeStyle<'_>,
) {
    let d = if x1 == x2 || y1 == y2 {
        format!("M {x1} {y1} L {x2} {y2}")
    } else {
        let mid_y = y1 + (y2 - y1) / 2;
        format!("M {x1} {y1} L {x1} {mid_y} L {x2} {mid_y} L {x2} {y2}")
    };
    out.push_str(&format!(
        "<path class=\"state-transition\" data-state-from=\"{}\" data-state-to=\"{}\" d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
        escape_text(from_name),
        escape_text(to_name),
        d,
        style.stroke,
        style.sw,
        style.dash,
        style.hidden,
        style.dir
    ));
}

fn parse_state_note_on_link_direction(direction: Option<&str>) -> Option<(&str, &str)> {
    let direction = direction?;
    let mut parts = direction.splitn(3, '|');
    if parts.next()? != "on-link" {
        return None;
    }
    let position = parts.next().unwrap_or("over");
    let target = parts.next()?;
    Some((position, target))
}

fn emit_state_note_connector(
    out: &mut String,
    transition: &StateTransition,
    from_p: &PlacedNode,
    note_p: &PlacedNode,
    placed: &std::collections::BTreeMap<String, PlacedNode>,
    node_kinds: &std::collections::BTreeMap<&str, &StateNodeKind>,
    fallback_stroke: &str,
) {
    let stroke = escape_text(transition.line_color.as_deref().unwrap_or(fallback_stroke));
    let sw = transition.thickness.unwrap_or(1).clamp(1, 8);
    let (x1, y1) = if let Some((_, target)) =
        parse_state_note_on_link_direction(transition.direction.as_deref())
    {
        if let Some(target_p) = placed.get(target) {
            let (ex1, ey1, ex2, ey2) = edge_anchors_for_kinds(
                node_kinds.get(transition.from.as_str()).copied(),
                from_p,
                node_kinds.get(target).copied(),
                target_p,
            );
            ((ex1 + ex2) / 2, (ey1 + ey2) / 2)
        } else {
            (from_p.x + from_p.w / 2, from_p.y + from_p.h / 2)
        }
    } else {
        (from_p.x + from_p.w / 2, from_p.y + from_p.h / 2)
    };
    let (_, _, x2, y2) = edge_anchors(
        &PlacedNode {
            x: x1,
            y: y1,
            w: 1,
            h: 1,
        },
        note_p,
    );
    out.push_str(&format!(
        "<path class=\"state-note-connector\" data-state-from=\"{}\" data-state-to=\"{}\" d=\"M {} {} L {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-dasharray=\"5 3\"/>",
        escape_text(&transition.from),
        escape_text(&transition.to),
        x1,
        y1,
        x2,
        y2,
        stroke,
        sw
    ));
}

/// Offset a line segment by `d` pixels perpendicular to its direction (to the right).
/// Used to separate bidirectional parallel edges so both arrows are visible.
fn offset_parallel_edge(x1: i32, y1: i32, x2: i32, y2: i32, d: i32) -> (i32, i32, i32, i32) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0 {
        return (x1, y1, x2, y2);
    }
    // Perpendicular unit vector (rotated 90° clockwise): (dy, -dx) / |len|
    let len = (len_sq as f64).sqrt();
    let ox = ((dy as f64 / len) * d as f64).round() as i32;
    let oy = ((-dx as f64 / len) * d as f64).round() as i32;
    (x1 + ox, y1 + oy, x2 + ox, y2 + oy)
}

/// Compute the edge anchor points between two placed nodes.
fn edge_anchors(from: &PlacedNode, to: &PlacedNode) -> (i32, i32, i32, i32) {
    let fcx = from.x + from.w / 2;
    let fcy = from.y + from.h / 2;
    let tcx = to.x + to.w / 2;
    let tcy = to.y + to.h / 2;

    let dx = tcx - fcx;
    let dy = tcy - fcy;

    // Use half-sizes for boundary computation
    let fhw = from.w / 2;
    let fhh = from.h / 2;
    let thw = to.w / 2;
    let thh = to.h / 2;

    if dx == 0 && dy == 0 {
        return (fcx, fcy, tcx, tcy);
    }

    // Determine exit/entry side based on dominant direction
    if dx.abs() >= dy.abs() {
        if dx >= 0 {
            (fcx + fhw, fcy, tcx - thw, tcy)
        } else {
            (fcx - fhw, fcy, tcx + thw, tcy)
        }
    } else if dy >= 0 {
        (fcx, fcy + fhh, tcx, tcy - thh)
    } else {
        (fcx, fcy - fhh, tcx, tcy + thh)
    }
}

fn wrap_state_label(label: &str, max_cols: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();

    for word in label.split_whitespace() {
        if word.len() > max_cols {
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
            }
            let mut start = 0usize;
            while start < word.len() {
                let end = (start + max_cols).min(word.len());
                lines.push(word[start..end].to_string());
                start = end;
            }
            continue;
        }

        let next_len = if current.is_empty() {
            word.len()
        } else {
            current.len() + 1 + word.len()
        };
        if next_len > max_cols && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }

    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn measure_state_label(lines: &[String]) -> (i32, i32) {
    let max_cols = lines
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(0) as i32;
    let width = (max_cols * STATE_LABEL_CHAR_W).max(24);
    let height = (lines.len() as i32 * STATE_LABEL_LINE_H).max(STATE_LABEL_LINE_H);
    (width, height)
}

fn place_state_transition_label(
    label: &str,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    placed: &std::collections::BTreeMap<String, PlacedNode>,
    occupied: &[LabelBounds],
) -> StateLabelLayout {
    let lines = wrap_state_label(label, STATE_LABEL_WRAP_COLS);
    let (w, h) = measure_state_label(&lines);
    let mx = (x1 + x2) as f64 / 2.0;
    let my = (y1 + y2) as f64 / 2.0;
    let dx = (x2 - x1) as f64;
    let dy = (y2 - y1) as f64;
    let len = (dx * dx + dy * dy).sqrt();
    let (tx, ty, nx, ny) = if len <= f64::EPSILON {
        (1.0, 0.0, 0.0, -1.0)
    } else {
        let tx = dx / len;
        let ty = dy / len;
        (tx, ty, -ty, tx)
    };

    let mut best = label_bounds_from_center(mx.round() as i32, (my - 18.0).round() as i32, w, h);
    let t_positions = [0.3, 0.4, 0.5, 0.6, 0.7];
    let along_offsets = [
        0.0, -18.0, 18.0, -36.0, 36.0, -56.0, 56.0, -76.0, 76.0, -96.0, 96.0, -120.0, 120.0,
    ];
    let normal_offsets = [18.0, 30.0, 42.0, 56.0, 72.0, 92.0, 116.0, 140.0, 168.0];

    for t in t_positions {
        let base_x = x1 as f64 + dx * t;
        let base_y = y1 as f64 + dy * t;
        for normal_sign in [1.0, -1.0] {
            for normal in normal_offsets {
                for along in along_offsets {
                    let cx = base_x + nx * normal * normal_sign + tx * along;
                    let cy = base_y + ny * normal * normal_sign + ty * along;
                    let candidate =
                        label_bounds_from_center(cx.round() as i32, cy.round() as i32, w, h);
                    if !state_label_hits_node(candidate, placed)
                        && !state_label_hits_other_label(candidate, occupied)
                    {
                        return StateLabelLayout {
                            cx: candidate.x + candidate.w / 2,
                            top: candidate.y,
                            lines,
                            bounds: candidate,
                        };
                    }
                    if state_label_candidate_score(candidate, placed, occupied)
                        > state_label_candidate_score(best, placed, occupied)
                    {
                        best = candidate;
                    }
                }
            }
        }
    }

    StateLabelLayout {
        cx: best.x + best.w / 2,
        top: best.y,
        lines,
        bounds: best,
    }
}

fn label_bounds_from_center(cx: i32, cy: i32, w: i32, h: i32) -> LabelBounds {
    LabelBounds {
        x: cx - w / 2,
        y: cy - h / 2,
        w,
        h,
    }
}

fn state_label_hits_node(
    label: LabelBounds,
    placed: &std::collections::BTreeMap<String, PlacedNode>,
) -> bool {
    placed
        .values()
        .any(|node| bounds_overlap(label, node_bounds(node), STATE_LABEL_NODE_CLEARANCE))
}

fn state_label_hits_other_label(label: LabelBounds, occupied: &[LabelBounds]) -> bool {
    occupied
        .iter()
        .copied()
        .any(|other| bounds_overlap(label, other, STATE_LABEL_LABEL_CLEARANCE))
}

fn state_label_candidate_score(
    label: LabelBounds,
    placed: &std::collections::BTreeMap<String, PlacedNode>,
    occupied: &[LabelBounds],
) -> i32 {
    let node_hits = placed
        .values()
        .filter(|node| bounds_overlap(label, node_bounds(node), STATE_LABEL_NODE_CLEARANCE))
        .count() as i32;
    let label_hits = occupied
        .iter()
        .filter(|other| bounds_overlap(label, **other, STATE_LABEL_LABEL_CLEARANCE))
        .count() as i32;
    -(node_hits * 100 + label_hits * 150)
}

fn node_bounds(node: &PlacedNode) -> LabelBounds {
    LabelBounds {
        x: node.x,
        y: node.y,
        w: node.w,
        h: node.h,
    }
}

fn bounds_overlap(a: LabelBounds, b: LabelBounds, padding: i32) -> bool {
    let ax1 = a.x - padding;
    let ay1 = a.y - padding;
    let ax2 = a.x + a.w + padding;
    let ay2 = a.y + a.h + padding;
    let bx1 = b.x;
    let by1 = b.y;
    let bx2 = b.x + b.w;
    let by2 = b.y + b.h;
    ax1 < bx2 && ax2 > bx1 && ay1 < by2 && ay2 > by1
}

fn render_state_transition_label(
    out: &mut String,
    layout: &StateLabelLayout,
    original_label: &str,
    font_color: &str,
) {
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\" text-anchor=\"middle\" data-state-label=\"{}\">",
        layout.cx,
        layout.top + 11,
        escape_text(font_color),
        escape_text(original_label)
    ));
    for (idx, line) in layout.lines.iter().enumerate() {
        out.push_str(&format!(
            "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
            layout.cx,
            layout.top + 11 + idx as i32 * STATE_LABEL_LINE_H,
            escape_text(line)
        ));
    }
    out.push_str("</text>");
}

fn edge_anchors_for_kinds(
    from_kind: Option<&StateNodeKind>,
    from: &PlacedNode,
    to_kind: Option<&StateNodeKind>,
    to: &PlacedNode,
) -> (i32, i32, i32, i32) {
    let mut anchors = edge_anchors(from, to);
    let from_center_x = from.x + from.w / 2;
    let from_center_y = from.y + from.h / 2;
    let to_center_x = to.x + to.w / 2;
    let to_center_y = to.y + to.h / 2;

    if matches!(
        from_kind,
        Some(&StateNodeKind::Fork) | Some(&StateNodeKind::Join)
    ) {
        let target_below = to_center_y >= from_center_y;
        anchors.0 = to_center_x.clamp(from.x, from.x + from.w);
        anchors.1 = if target_below {
            from.y + from.h
        } else {
            from.y
        };
        anchors.2 = to_center_x;
        anchors.3 = if target_below { to.y } else { to.y + to.h };
    }

    if matches!(
        to_kind,
        Some(&StateNodeKind::Fork) | Some(&StateNodeKind::Join)
    ) {
        let source_above = from_center_y <= to_center_y;
        anchors.0 = from_center_x;
        anchors.1 = if source_above {
            from.y + from.h
        } else {
            from.y
        };
        anchors.2 = from_center_x.clamp(to.x, to.x + to.w);
        anchors.3 = if source_above { to.y } else { to.y + to.h };
    }

    if matches!(from_kind, Some(&StateNodeKind::Choice)) {
        (anchors.0, anchors.1) = diamond_anchor(from, to_center_x, to_center_y);
    }

    if matches!(to_kind, Some(&StateNodeKind::Choice)) {
        (anchors.2, anchors.3) = diamond_anchor(to, from_center_x, from_center_y);
    }

    anchors
}

fn diamond_anchor(node: &PlacedNode, toward_x: i32, toward_y: i32) -> (i32, i32) {
    let cx = node.x + node.w / 2;
    let cy = node.y + node.h / 2;
    let dx = toward_x - cx;
    let dy = toward_y - cy;
    if dx == 0 && dy == 0 {
        return (cx, cy);
    }

    let half_w = (node.w / 2).max(1) as f64;
    let half_h = (node.h / 2).max(1) as f64;
    let scale =
        1.0 / (((dx.abs() as f64) / half_w) + ((dy.abs() as f64) / half_h)).max(f64::EPSILON);
    (
        cx + ((dx as f64) * scale).round() as i32,
        cy + ((dy as f64) * scale).round() as i32,
    )
}

fn state_node_kind_name(kind: &StateNodeKind) -> &'static str {
    match kind {
        StateNodeKind::Normal => "normal",
        StateNodeKind::StartEnd => "start-end",
        StateNodeKind::HistoryShallow => "history-shallow",
        StateNodeKind::HistoryDeep => "history-deep",
        StateNodeKind::Fork => "fork",
        StateNodeKind::Join => "join",
        StateNodeKind::Choice => "choice",
        StateNodeKind::End => "end",
        StateNodeKind::EntryPoint => "entry-point",
        StateNodeKind::ExitPoint => "exit-point",
        StateNodeKind::InputPin => "input-pin",
        StateNodeKind::OutputPin => "output-pin",
        StateNodeKind::ExpansionInput => "expansion-input",
        StateNodeKind::ExpansionOutput => "expansion-output",
        StateNodeKind::Note => "note",
        StateNodeKind::JsonProjection => "json-projection",
    }
}

fn state_dash_attr(dashed: bool) -> &'static str {
    if dashed {
        " stroke-dasharray=\"5 3\""
    } else {
        ""
    }
}

fn state_hidden_attr(hidden: bool) -> &'static str {
    if hidden {
        " visibility=\"hidden\""
    } else {
        ""
    }
}

fn state_direction_attr(direction: Option<&str>) -> String {
    direction
        .map(|d| format!(" data-state-direction=\"{}\"", escape_text(d)))
        .unwrap_or_default()
}

fn state_node_fill(node: &StateNode, state_style: &crate::theme::StateStyle) -> String {
    escape_text(
        node.style
            .fill_color
            .as_deref()
            .unwrap_or(&state_style.background_color),
    )
}

fn state_node_border(node: &StateNode, state_style: &crate::theme::StateStyle) -> String {
    escape_text(
        node.style
            .border_color
            .as_deref()
            .unwrap_or(&state_style.border_color),
    )
}

fn state_node_text(node: &StateNode, state_style: &crate::theme::StateStyle) -> String {
    escape_text(
        node.style
            .text_color
            .as_deref()
            .unwrap_or(&state_style.font_color),
    )
}

fn state_node_border_dash(node: &StateNode) -> &'static str {
    if node.style.border_dashed {
        " stroke-dasharray=\"5 3\""
    } else {
        ""
    }
}

fn state_node_stroke_width(node: &StateNode, fallback: f32) -> String {
    node.style
        .border_thickness
        .map(|value| value.clamp(1, 8).to_string())
        .unwrap_or_else(|| {
            if fallback.fract() == 0.0 {
                format!("{}", fallback as i32)
            } else {
                fallback.to_string()
            }
        })
}

fn render_state_note(out: &mut String, node: &StateNode, x: i32, y: i32, w: i32, h: i32) {
    let fold = 12;
    out.push_str(&format!(
        "<path class=\"state-note\" d=\"M {x} {y} H {} L {} {} V {} H {x} Z\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
        x + w - fold,
        x + w,
        y + fold,
        y + h,
        STATE_NOTE_FILL,
        STATE_NOTE_BORDER
    ));
    out.push_str(&format!(
        "<path class=\"state-note-fold\" d=\"M {} {y} V {} H {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"1\"/>",
        x + w - fold,
        y + fold,
        x + w,
        STATE_NOTE_BORDER
    ));
    for (idx, line) in node_display_lines(node).iter().enumerate() {
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"#111111\">{}</text>",
            x + STATE_NOTE_PAD_X,
            y + STATE_NOTE_PAD_Y + 11 + idx as i32 * STATE_LABEL_LINE_H,
            escape_text(line)
        ));
    }
}

fn render_state_json_projection(
    out: &mut String,
    node: &StateNode,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    state_style: &crate::theme::StateStyle,
) {
    let fill = node
        .style
        .fill_color
        .as_deref()
        .map(escape_text)
        .unwrap_or_else(|| "#eef6ff".to_string());
    let border = state_node_border(node, state_style);
    out.push_str(&format!(
        "<rect class=\"state-json-projection\" data-state-json=\"true\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"6\" ry=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"{} />",
        x,
        y,
        w,
        h,
        fill,
        border,
        state_node_border_dash(node)
    ));
    let lines = node_display_lines(node);
    for (idx, line) in lines.iter().enumerate() {
        let weight = if idx == 0 { "600" } else { "400" };
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" font-weight=\"{}\" fill=\"{}\">{}</text>",
            x + STATE_NOTE_PAD_X,
            y + STATE_NOTE_PAD_Y + 11 + idx as i32 * STATE_LABEL_LINE_H,
            weight,
            state_node_text(node, state_style),
            escape_text(line)
        ));
    }
}

/// Render a single state node (and its children recursively).
#[allow(clippy::too_many_arguments)]
fn render_node<'a>(
    out: &mut String,
    node: &'a StateNode,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    state_style: &crate::theme::StateStyle,
    incoming_count: usize,
    outgoing_count: usize,
    placed: &std::collections::BTreeMap<String, PlacedNode>,
    incoming: &std::collections::BTreeMap<&str, usize>,
    outgoing: &std::collections::BTreeMap<&str, usize>,
    all_transitions: &'a [StateTransition],
    edge_set: &std::collections::BTreeSet<(&'a str, &'a str)>,
    node_kinds: &std::collections::BTreeMap<&'a str, &'a StateNodeKind>,
    occupied_label_bounds: &mut Vec<LabelBounds>,
) {
    out.push_str(&format!(
        "<metadata data-state-node=\"{}\" data-state-kind=\"{}\"{} />",
        escape_text(&node.name),
        state_node_kind_name(&node.kind),
        node.stereotype
            .as_deref()
            .map(|s| format!(" data-state-stereotype=\"{}\"", escape_text(s)))
            .unwrap_or_default()
    ));

    match node.kind {
        StateNodeKind::StartEnd => {
            let cx = x + w / 2;
            let cy = y + h / 2;
            let r = 12i32;
            if incoming_count > 0 && outgoing_count == 0 {
                // End variant: outer ring + inner dot
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                    cx, cy, r, state_style.background_color, state_style.border_color
                ));
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"7\" fill=\"{}\"/>",
                    cx, cy, state_style.start_color
                ));
            } else {
                // Start variant: filled circle
                out.push_str(&format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>",
                    cx, cy, r, state_style.start_color
                ));
            }
        }

        StateNodeKind::End => {
            let cx = x + w / 2;
            let cy = y + h / 2;
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"12\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx, cy, state_style.background_color, state_style.border_color
            ));
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"8\" fill=\"{}\"/>",
                cx, cy, state_style.start_color
            ));
        }

        StateNodeKind::HistoryShallow | StateNodeKind::HistoryDeep => {
            let cx = x + w / 2;
            let cy = y + h / 2;
            let label = node.display.as_deref().unwrap_or("H");
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"16\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx, cy, state_style.background_color, state_style.border_color
            ));
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\" dominant-baseline=\"middle\" fill=\"{}\">{}</text>",
                cx, cy, state_style.font_color, escape_text(label)
            ));
        }

        StateNodeKind::Fork | StateNodeKind::Join => {
            // UML spec: thick horizontal bar; no text label
            let bar_h = 8i32;
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                x,
                y + h / 2 - bar_h / 2,
                w,
                bar_h,
                state_style.start_color
            ));
            // No "fork"/"join" text — UML spec shows only the bar
        }

        StateNodeKind::Choice => {
            let cx = x + w / 2;
            let cy = y + h / 2;
            let r = (w / 2).min(h / 2) - 2;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx, cy - r,
                cx + r, cy,
                cx, cy + r,
                cx - r, cy,
                state_style.background_color, state_style.border_color
            ));
        }

        StateNodeKind::EntryPoint | StateNodeKind::ExitPoint => {
            let cx = x + w / 2;
            let cy = y + h / 2;
            let border = state_node_border(node, state_style);
            let fill = state_node_fill(node, state_style);
            out.push_str(&format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"10\" fill=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                cx, cy, fill, border
            ));
            if node.kind == StateNodeKind::ExitPoint {
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                    cx - 5, cy - 5, cx + 5, cy + 5, border
                ));
                out.push_str(&format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"2\"/>",
                    cx + 5, cy - 5, cx - 5, cy + 5, border
                ));
            }
        }

        StateNodeKind::InputPin | StateNodeKind::OutputPin => {
            let fill = state_node_fill(node, state_style);
            let border = state_node_border(node, state_style);
            let sw = state_node_stroke_width(node, 1.5);
            out.push_str(&format!(
                "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{} />",
                x + 5,
                y + 5,
                w - 10,
                h - 10,
                fill,
                border,
                sw,
                state_node_border_dash(node)
            ));
        }

        StateNodeKind::ExpansionInput | StateNodeKind::ExpansionOutput => {
            let fill = state_node_fill(node, state_style);
            let border = state_node_border(node, state_style);
            let sw = state_node_stroke_width(node, 1.5);
            let segment_w = (w - 8) / 3;
            for idx in 0..3 {
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{} />",
                    x + 4 + idx * segment_w,
                    y + 5,
                    segment_w,
                    h - 10,
                    fill,
                    border,
                    sw,
                    state_node_border_dash(node)
                ));
            }
        }

        StateNodeKind::Note => {
            render_state_note(out, node, x, y, w, h);
        }

        StateNodeKind::JsonProjection => {
            render_state_json_projection(out, node, x, y, w, h, state_style);
        }

        StateNodeKind::Normal => {
            let has_children = node.regions.iter().any(|r| !r.is_empty());
            let display = node.display.as_deref().unwrap_or(&node.name);

            if has_children {
                // ── Composite state ──────────────────────────────────────────
                // Draw the enclosing rounded-rect box
                let fill = state_node_fill(node, state_style);
                let border = state_node_border(node, state_style);
                let text = state_node_text(node, state_style);
                let sw = state_node_stroke_width(node, 1.5);
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                    x, y, w, h, fill, border, sw, state_node_border_dash(node)
                ));
                // Composite name label at top-center
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                    x + w / 2, y + 20, text, escape_text(display)
                ));

                // Draw concurrent region dividers (dashed vertical lines)
                if node.regions.len() > 1 {
                    for ri in 0..node.regions.len() - 1 {
                        let prev_right = node.regions[ri]
                            .iter()
                            .filter_map(|child| placed.get(&child.name))
                            .map(|child| child.x + child.w)
                            .max();
                        let next_left = node.regions[ri + 1]
                            .iter()
                            .filter_map(|child| placed.get(&child.name))
                            .map(|child| child.x)
                            .min();
                        if let (Some(prev_right), Some(next_left)) = (prev_right, next_left) {
                            let div_x = (prev_right + next_left) / 2;
                            let div_top = y + COMPOSITE_PAD_Y - 8;
                            let div_bot = y + h - COMPOSITE_PAD_BOT + 4;
                            if div_top < div_bot {
                                out.push_str(&format!(
                                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"5 3\"/>",
                                    div_x, div_top, div_x, div_bot, state_style.border_color
                                ));
                            }
                        }
                    }
                }

                // Collect names of all direct children across all regions of this
                // composite, so we can draw intra-composite transitions above the
                // background rect but below child node boxes.
                let child_names: std::collections::BTreeSet<&str> = node
                    .regions
                    .iter()
                    .flat_map(|r| r.iter())
                    .map(|c| c.name.as_str())
                    .collect();

                // For label placement within the composite, replace the composite
                // parent's full bounding box with thin "wall" slabs along its
                // content edges.  The parent's full bounding box covers the entire
                // interior, so keeping it would force every intra-composite label
                // outside the composite box (#709).  The walls prevent labels from
                // drifting into the header / footer / side margins while still
                // allowing the algorithm to find positions in the interior gap.
                let composite_p = PlacedNode { x, y, w, h };
                let header_wall = PlacedNode {
                    x: composite_p.x,
                    y: composite_p.y,
                    w: composite_p.w,
                    h: COMPOSITE_PAD_Y, // covers the title bar
                };
                let footer_wall = PlacedNode {
                    x: composite_p.x,
                    y: composite_p.y + composite_p.h - COMPOSITE_PAD_BOT,
                    w: composite_p.w,
                    h: COMPOSITE_PAD_BOT,
                };
                let left_wall = PlacedNode {
                    x: composite_p.x,
                    y: composite_p.y,
                    w: COMPOSITE_PAD_X,
                    h: composite_p.h,
                };
                let right_wall = PlacedNode {
                    x: composite_p.x + composite_p.w - COMPOSITE_PAD_X,
                    y: composite_p.y,
                    w: COMPOSITE_PAD_X,
                    h: composite_p.h,
                };
                let mut inner_placed: std::collections::BTreeMap<String, PlacedNode> = placed
                    .iter()
                    .filter(|(k, _)| k.as_str() != node.name.as_str())
                    .map(|(k, v)| (k.clone(), *v))
                    .collect();
                inner_placed.insert(format!("__wall_header_{}", node.name), header_wall);
                inner_placed.insert(format!("__wall_footer_{}", node.name), footer_wall);
                inner_placed.insert(format!("__wall_left_{}", node.name), left_wall);
                inner_placed.insert(format!("__wall_right_{}", node.name), right_wall);

                // Draw intra-composite transitions (both endpoints are direct children).
                // These were skipped in the outer transition loop so they appear above
                // the composite background rect rather than hidden behind it.
                for t in all_transitions {
                    if !child_names.contains(t.from.as_str())
                        || !child_names.contains(t.to.as_str())
                    {
                        continue;
                    }
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
                        let stroke = escape_text(
                            t.line_color.as_deref().unwrap_or(&state_style.arrow_color),
                        );
                        let sw = t.thickness.unwrap_or(2).clamp(1, 8);
                        let dash = state_dash_attr(t.dashed);
                        let hidden = state_hidden_attr(t.hidden);
                        let dir = state_direction_attr(t.direction.as_deref());

                        if t.from == t.to {
                            let cpx = x1 + 18;
                            let cpy = y1 - 14;
                            out.push_str(&format!(
                                "<path class=\"state-transition\" data-state-from=\"{}\" data-state-to=\"{}\" d=\"M {x1} {y1} Q {cpx} {cpy} {x2} {y2}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
                                escape_text(&t.from), escape_text(&t.to), stroke, sw, dash, hidden, dir
                            ));
                        } else if has_reverse {
                            let (ox1, oy1, ox2, oy2) = offset_parallel_edge(x1, y1, x2, y2, 10);
                            let cpx = (ox1 + ox2) / 2;
                            let cpy = (oy1 + oy2) / 2;
                            out.push_str(&format!(
                                "<path class=\"state-transition\" data-state-from=\"{}\" data-state-to=\"{}\" d=\"M {} {} Q {} {} {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
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
                                    &inner_placed,
                                    occupied_label_bounds,
                                );
                                render_state_transition_label(
                                    out,
                                    &layout,
                                    label,
                                    &state_style.font_color,
                                );
                                occupied_label_bounds.push(layout.bounds);
                            }
                            continue;
                        } else {
                            emit_state_orthogonal_path(
                                out,
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
                                &inner_placed,
                                occupied_label_bounds,
                            );
                            render_state_transition_label(
                                out,
                                &layout,
                                label,
                                &state_style.font_color,
                            );
                            occupied_label_bounds.push(layout.bounds);
                        }
                    }
                }

                // Draw children recursively
                for region in &node.regions {
                    for child in region {
                        if let Some(cp) = placed.get(&child.name) {
                            let c_inc = *incoming.get(child.name.as_str()).unwrap_or(&0);
                            let c_out = *outgoing.get(child.name.as_str()).unwrap_or(&0);
                            render_node(
                                out,
                                child,
                                cp.x,
                                cp.y,
                                cp.w,
                                cp.h,
                                state_style,
                                c_inc,
                                c_out,
                                placed,
                                incoming,
                                outgoing,
                                all_transitions,
                                edge_set,
                                node_kinds,
                                occupied_label_bounds,
                            );
                        }
                    }
                }
            } else {
                // ── Simple state box ─────────────────────────────────────────
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                    x,
                    y,
                    w,
                    h,
                    state_node_fill(node, state_style),
                    state_node_border(node, state_style),
                    state_node_stroke_width(node, 1.5),
                    state_node_border_dash(node)
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                    x + w / 2, y + 24, state_node_text(node, state_style), escape_text(display)
                ));
                // Internal actions
                if !node.internal_actions.is_empty() {
                    out.push_str(&format!(
                        "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                        x, y + STATE_NODE_H - 4, x + w, y + STATE_NODE_H - 4, state_style.border_color
                    ));
                    for (ai, action) in node.internal_actions.iter().enumerate() {
                        let ay = y + STATE_NODE_H + ai as i32 * 14;
                        let text = if action.action.is_empty() {
                            action.kind.clone()
                        } else {
                            format!("{} / {}", action.kind, action.action)
                        };
                        out.push_str(&format!(
                            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" font-style=\"italic\" fill=\"{}\">{}</text>",
                            x + 6, ay + 10, state_node_text(node, state_style), escape_text(&text)
                        ));
                    }
                }
            }
        }
    }
}
