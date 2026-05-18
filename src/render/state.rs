use super::*;

// Layout constants
const STATE_NODE_W: i32 = 140;
const STATE_NODE_H: i32 = 40;
const STATE_NODE_GAP_X: i32 = 60;
const STATE_NODE_GAP_Y: i32 = 60;
const STATE_MARGIN: i32 = 30;
const COMPOSITE_PAD_X: i32 = 16;
const COMPOSITE_PAD_Y: i32 = 36; // extra space for composite header label
const COMPOSITE_PAD_BOT: i32 = 12;
const REGION_DIVIDER_GAP: i32 = 10; // gap between concurrent regions (horizontal divider)
const STATE_LABEL_CLEARANCE: i32 = 18;

/// A placed node entry in the flat coord map.
/// Stores the node's top-left (x, y) and its full rendered size (w, h).
#[derive(Clone)]
struct PlacedNode {
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
    let top_level_count = nodes
        .iter()
        .filter(|n| !child_node_names.contains(n.name.as_str()))
        .count();
    let has_fork_join_choice = nodes.iter().any(|n| {
        !child_node_names.contains(n.name.as_str())
            && matches!(
                n.kind,
                StateNodeKind::Fork | StateNodeKind::Join | StateNodeKind::Choice
            )
    });
    let has_top_level_composite = nodes.iter().any(|n| {
        !child_node_names.contains(n.name.as_str()) && n.regions.iter().any(|region| !region.is_empty())
    });
    let cols: i32 = if has_fork_join_choice || has_top_level_composite || top_level_count <= 3 {
        1
    } else {
        2
    };

    // Longest-path reachability sort of top-level nodes from initial states.
    // Using the maximum depth instead of the minimum keeps sinks/final states below
    // all of their incoming branches, which avoids clipped/crossing terminal arrows.
    let layout_order: Vec<&StateNode> = {
        let mut depth_map: std::collections::BTreeMap<&str, usize> =
            std::collections::BTreeMap::new();
        let top_level_names: std::collections::BTreeSet<&str> = nodes
            .iter()
            .filter(|n| !child_node_names.contains(n.name.as_str()))
            .map(|n| n.name.as_str())
            .collect();
        let transition_targets: std::collections::BTreeSet<&str> =
            transitions
                .iter()
                .filter(|t| top_level_names.contains(t.to.as_str()))
                .map(|t| t.to.as_str())
                .collect();
        let all_node_names: Vec<&str> = nodes
            .iter()
            .filter(|n| !child_node_names.contains(n.name.as_str()))
            .map(|n| n.name.as_str())
            .collect();
        let mut adjacency: std::collections::BTreeMap<&str, Vec<&str>> =
            std::collections::BTreeMap::new();
        for t in transitions {
            if top_level_names.contains(t.from.as_str()) && top_level_names.contains(t.to.as_str()) {
                adjacency.entry(t.from.as_str()).or_default().push(t.to.as_str());
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
        for seed in seeds {
            let mut path = std::collections::BTreeSet::new();
            walk_longest_depth(seed, 1, &adjacency, &mut depth_map, &mut path);
        }
        // Unreachable nodes get MAX depth (appear at bottom)
        for &name in &all_node_names {
            depth_map.entry(name).or_insert(usize::MAX);
        }
        // Sort by (reachability depth, original doc order) so layout matches diagram flow
        let name_to_orig: std::collections::BTreeMap<&str, usize> = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.name.as_str(), i))
            .collect();
        let mut ordered: Vec<&StateNode> = nodes
            .iter()
            .filter(|n| !child_node_names.contains(n.name.as_str()))
            .collect();
        ordered.sort_by_key(|n| {
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
        ordered
    };

    let mut placed: std::collections::BTreeMap<String, PlacedNode> =
        std::collections::BTreeMap::new();

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
    let mut edge_labels: Vec<(i32, i32, String)> = Vec::new();

    // Draw transitions first (arrows behind nodes)
    for t in transitions {
        let from_p = placed.get(&t.from);
        let to_p = placed.get(&t.to);
        if let (Some(fp), Some(tp)) = (from_p, to_p) {
            // Check if the reverse edge also exists (bidirectional pair)
            let has_reverse =
                t.from != t.to && edge_set.contains(&(t.to.as_str(), t.from.as_str()));
            let (x1, y1, x2, y2) = edge_anchors(fp, tp);
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
                let (label_x, label_y) = edge_label_position(x1, y1, x2, y2);
                let label_y = label_y.min(cpy - 6);
                if let Some(label) = &t.label {
                    edge_labels.push((label_x, label_y, label.clone()));
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
                    let (label_x, label_y) = edge_label_position(ox1, oy1, ox2, oy2);
                    edge_labels.push((label_x, label_y, label.clone()));
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
                let (label_x, label_y) = edge_label_position(x1, y1, x2, y2);
                edge_labels.push((label_x, label_y, label.clone()));
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

    for (x, y, label) in edge_labels {
        render_state_label(&mut out, &label, x, y, &state_style.font_color);
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
                // For concurrent states (multi-region), stack regions vertically
                // with a dashed divider line between them.
                let mut total_w = STATE_NODE_W;
                let mut total_h = 0i32;
                for region in &node.regions {
                    let (rw, rh) = compute_region_size(region, sizes);
                    total_w = total_w.max(rw + COMPOSITE_PAD_X * 2);
                    total_h += rh;
                }
                // Add region divider gaps
                if n_regions > 1 {
                    total_h += (n_regions - 1) * REGION_DIVIDER_GAP;
                }
                let w = total_w;
                let h = total_h + COMPOSITE_PAD_Y + COMPOSITE_PAD_BOT;
                (w.max(STATE_NODE_W), h.max(STATE_NODE_H + 20))
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
        let mut child_y = y + COMPOSITE_PAD_Y;
        for (ri, region) in node.regions.iter().enumerate() {
            // Place each child in this region as a vertical stack
            // centered horizontally within the composite.
            let region_x = x + COMPOSITE_PAD_X;
            let avail_w = w - COMPOSITE_PAD_X * 2;
            let _ = avail_w;

            for (ci, child) in region.iter().enumerate() {
                let (cw, ch) = sizes
                    .get(&child.name)
                    .copied()
                    .unwrap_or((STATE_NODE_W, STATE_NODE_H));
                // Center child horizontally within parent
                let cx = x + COMPOSITE_PAD_X + (avail_w - cw) / 2;
                let cx = cx.max(region_x); // don't go left of padding
                place_node(child, cx, child_y, cw, ch, sizes, placed);
                child_y += ch;
                // Gap between children within a region (not after last)
                if ci + 1 < region.len() {
                    child_y += STATE_NODE_GAP_Y;
                }
            }
            // After each region (except last), leave room for the dashed divider
            if ri + 1 < node.regions.len() {
                child_y += REGION_DIVIDER_GAP;
            }
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

fn edge_label_position(x1: i32, y1: i32, x2: i32, y2: i32) -> (i32, i32) {
    let mx = (x1 + x2) / 2;
    let my = (y1 + y2) / 2;
    let dx = x2 - x1;
    let dy = y2 - y1;

    if dx == 0 && dy == 0 {
        return (mx, my - STATE_LABEL_CLEARANCE);
    }
    if dx.abs() < dy.abs() {
        return (mx + STATE_LABEL_CLEARANCE, my);
    }

    let len = ((dx * dx + dy * dy) as f64).sqrt();
    if len == 0.0 {
        return (mx, my - STATE_LABEL_CLEARANCE);
    }
    let offset_x = ((-(dy as f64) / len) * STATE_LABEL_CLEARANCE as f64).round() as i32;
    let offset_y = (((dx as f64) / len) * STATE_LABEL_CLEARANCE as f64).round() as i32;
    (mx + offset_x, my + offset_y)
}

fn render_state_label(out: &mut String, label: &str, x: i32, y: i32, color: &str) {
    let lines: Vec<&str> = label.lines().collect();
    out.push_str(&format!(
        "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"11\" fill=\"{}\" text-anchor=\"middle\">",
        x,
        y,
        escape_text(color)
    ));
    if lines.len() <= 1 {
        out.push_str(&escape_text(label));
    } else {
        for (idx, line) in lines.iter().enumerate() {
            let dy = if idx == 0 { 0 } else { 12 };
            out.push_str(&format!(
                "<tspan x=\"{}\" dy=\"{}\">{}</tspan>",
                x,
                dy,
                escape_text(line)
            ));
        }
    }
    out.push_str("</text>");
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

                // Draw concurrent region dividers (dashed horizontal lines)
                // We need to find where each region ends to place the divider.
                // We use the placed coords of the last child in each non-final region.
                if node.regions.len() > 1 {
                    for ri in 0..node.regions.len() - 1 {
                        // Find the bottom of the last child in region ri
                        if let Some(last_child) = node.regions[ri].last() {
                            if let Some(lp) = placed.get(&last_child.name) {
                                let div_y = lp.y + lp.h + REGION_DIVIDER_GAP / 2;
                                out.push_str(&format!(
                                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1\" stroke-dasharray=\"5 3\"/>",
                                    x + 4, div_y, x + w - 4, div_y, state_style.border_color
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
