use super::*;

pub(super) fn collect_composite_children<'a>(
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

pub(super) fn collect_child_to_parent<'a>(
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

pub(super) fn compute_node_size(
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
            let (alias, rows) = state_projection_layout(node);
            let max_cols = std::iter::once(alias.as_str())
                .chain(rows.iter().map(|row| row.label.as_str()))
                .map(|line| line.chars().count())
                .max()
                .unwrap_or(6);
            let w = (max_cols as i32 * STATE_LABEL_CHAR_W + STATE_NOTE_PAD_X * 2 + 32).max(170);
            let h = (22 + rows.len() as i32 * 16 + STATE_NOTE_PAD_Y * 2).max(58);
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
pub(super) fn compute_region_size(
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

pub(super) fn node_display_lines(node: &StateNode) -> Vec<String> {
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

pub(super) fn concurrent_region_metrics(
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
pub(super) fn place_node(
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
                    let cx = boundary_point_x(child, x, w, cx, cw);
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
                    let cx = boundary_point_x(child, x, w, cx, cw);
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

pub(super) fn boundary_point_x(
    child: &StateNode,
    parent_x: i32,
    parent_w: i32,
    fallback_x: i32,
    child_w: i32,
) -> i32 {
    match child.kind {
        StateNodeKind::EntryPoint => parent_x - child_w / 2,
        StateNodeKind::ExitPoint => parent_x + parent_w - child_w / 2,
        _ => fallback_x,
    }
}

pub(super) fn position_state_notes(
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

pub(super) fn rects_overlap(x: i32, y: i32, w: i32, h: i32, other: &PlacedNode) -> bool {
    x < other.x + other.w && x + w > other.x && y < other.y + other.h && y + h > other.y
}

pub(super) fn shift_layout_for_transition_labels<'a>(
    transitions: &'a [StateTransition],
    placed: &mut std::collections::BTreeMap<String, PlacedNode>,
    edge_set: &std::collections::BTreeSet<(&'a str, &'a str)>,
    node_kinds: &std::collections::BTreeMap<&'a str, &'a StateNodeKind>,
) {
    let mut dry_occupied: Vec<LabelBounds> = Vec::new();
    let mut min_label_x = placed.values().map(|p| p.x).min().unwrap_or(0);
    for t in transitions {
        // Skip transitions where either endpoint is not a top-level placed node
        // (intra-composite child->child edges are handled inside composite rendering).
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
                    state_orthogonal_label_segment(x1, y1, x2, y2)
                };
                let layout =
                    place_state_transition_label(label, lx1, ly1, lx2, ly2, placed, &dry_occupied);
                min_label_x = min_label_x.min(layout.bounds.x);
                dry_occupied.push(layout.bounds);
            }
        }
    }

    let required_shift = (STATE_MARGIN - min_label_x).max(0);
    if required_shift > 0 {
        for p in placed.values_mut() {
            p.x += required_shift;
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn expand_canvas_for_transition_labels<'a>(
    transitions: &'a [StateTransition],
    child_node_names: &std::collections::BTreeSet<&'a str>,
    child_to_parent: &std::collections::BTreeMap<&'a str, &'a str>,
    placed: &std::collections::BTreeMap<String, PlacedNode>,
    edge_set: &std::collections::BTreeSet<(&'a str, &'a str)>,
    node_kinds: &std::collections::BTreeMap<&'a str, &'a StateNodeKind>,
    max_x: &mut i32,
    max_y: &mut i32,
) {
    let mut prelim_occupied: Vec<LabelBounds> = Vec::new();
    for t in transitions {
        if child_node_names.contains(t.from.as_str()) && child_node_names.contains(t.to.as_str()) {
            continue;
        }
        account_for_transition_label(
            t,
            placed,
            placed,
            edge_set,
            node_kinds,
            max_x,
            max_y,
            &mut prelim_occupied,
        );
    }

    for t in transitions {
        if !child_node_names.contains(t.from.as_str()) || !child_node_names.contains(t.to.as_str())
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
        account_for_transition_label(
            t,
            placed,
            &inner,
            edge_set,
            node_kinds,
            max_x,
            max_y,
            &mut prelim_occupied,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn account_for_transition_label<'a>(
    transition: &'a StateTransition,
    placed: &std::collections::BTreeMap<String, PlacedNode>,
    obstacle_placed: &std::collections::BTreeMap<String, PlacedNode>,
    edge_set: &std::collections::BTreeSet<(&'a str, &'a str)>,
    node_kinds: &std::collections::BTreeMap<&'a str, &'a StateNodeKind>,
    max_x: &mut i32,
    max_y: &mut i32,
    prelim_occupied: &mut Vec<LabelBounds>,
) {
    let Some(label) = &transition.label else {
        return;
    };
    let from_p = placed.get(&transition.from);
    let to_p = placed.get(&transition.to);
    if let (Some(fp), Some(tp)) = (from_p, to_p) {
        let has_reverse = transition.from != transition.to
            && edge_set.contains(&(transition.to.as_str(), transition.from.as_str()));
        let (x1, y1, x2, y2) = edge_anchors_for_kinds(
            node_kinds.get(transition.from.as_str()).copied(),
            fp,
            node_kinds.get(transition.to.as_str()).copied(),
            tp,
        );
        let (lx1, ly1, lx2, ly2) = if has_reverse {
            offset_parallel_edge(x1, y1, x2, y2, 10)
        } else {
            state_orthogonal_label_segment(x1, y1, x2, y2)
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
}

pub(super) fn compute_top_level_depths<'a>(
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

pub(super) fn place_top_level_layered<'a>(
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

pub(super) fn desired_state_center(
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

pub(super) fn adjust_fork_join_bar_widths<'a>(
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
