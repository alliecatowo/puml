use super::*;
use crate::model::StateTransition;

// Layout constants
const STATE_NODE_W: i32 = 140;
const STATE_NODE_H: i32 = 40;
const STATE_NODE_GAP_X: i32 = 60;
const STATE_NODE_GAP_Y: i32 = 60;
const STATE_MARGIN: i32 = 30;
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
#[derive(Clone)]
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

    let mut placed: std::collections::BTreeMap<String, PlacedNode> =
        std::collections::BTreeMap::new();

    if has_fork_join_choice {
        place_top_level_layered(
            &layout_order,
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
            for node in &layout_order {
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

    // Compute total canvas size from placed nodes
    let max_x = placed.values().map(|p| p.x + p.w).max().unwrap_or(300);
    let max_y = placed.values().map(|p| p.y + p.h).max().unwrap_or(200);
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

    // Build a set of all (from, to) pairs to detect bidirectional edges
    let edge_set: std::collections::BTreeSet<(&str, &str)> = transitions
        .iter()
        .map(|t| (t.from.as_str(), t.to.as_str()))
        .collect();
    let node_kinds: std::collections::BTreeMap<&str, &StateNodeKind> = nodes
        .iter()
        .map(|node| (node.name.as_str(), &node.kind))
        .collect();
    let mut occupied_label_bounds: Vec<LabelBounds> = Vec::new();

    // Draw transitions first (arrows behind nodes)
    for t in transitions {
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
                out.push_str(&format!(
                    "<line class=\"state-transition\" data-state-from=\"{}\" data-state-to=\"{}\" x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
                    escape_text(&t.from), escape_text(&t.to),
                    x1, y1, x2, y2,
                    stroke, sw, dash, hidden, dir
                ));
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
        StateNodeKind::StartEnd | StateNodeKind::End => (26, 26),
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

/// Render a single state node (and its children recursively).
#[allow(clippy::too_many_arguments)]
fn render_node(
    out: &mut String,
    node: &StateNode,
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

        StateNodeKind::Normal => {
            let has_children = node.regions.iter().any(|r| !r.is_empty());
            let display = node.display.as_deref().unwrap_or(&node.name);

            if has_children {
                // ── Composite state ──────────────────────────────────────────
                // Draw the enclosing rounded-rect box
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    x, y, w, h, state_style.background_color, state_style.border_color
                ));
                // Composite name label at top-center
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                    x + w / 2, y + 20, state_style.font_color, escape_text(display)
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
                            );
                        }
                    }
                }
            } else {
                // ── Simple state box ─────────────────────────────────────────
                out.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"8\" ry=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
                    x, y, w, h, state_style.background_color, state_style.border_color
                ));
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"13\" font-weight=\"600\" text-anchor=\"middle\" fill=\"{}\">{}</text>",
                    x + w / 2, y + 24, state_style.font_color, escape_text(display)
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
                            x + 6, ay + 10, state_style.font_color, escape_text(&text)
                        ));
                    }
                }
            }
        }
    }
}
