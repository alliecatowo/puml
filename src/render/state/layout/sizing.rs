use super::super::*;

pub(in crate::render::state) fn collect_composite_children<'a>(
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

pub(in crate::render::state) fn collect_child_to_parent<'a>(
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

pub(in crate::render::state) fn compute_node_size(
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
        StateNodeKind::Terminate => (26, 26),
        StateNodeKind::SdlReceive | StateNodeKind::SdlSend => (STATE_NODE_W, STATE_NODE_H),
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
                // Composite state: size from children.
                // If the composite has internal actions (entry/exit/do), allocate
                // extra header height so the action text does not overlap children
                // (closes #1304).
                let actions_h = (node.internal_actions.len() as i32) * 14;
                let n_regions = node.regions.len().max(1) as i32;
                if n_regions > 1 {
                    // Compute children's sizes first so concurrent_region_metrics
                    // has accurate per-child dimensions.  (compute_region_size
                    // populates `sizes` as a side-effect via recursive calls to
                    // compute_node_size.)
                    for region in &node.regions {
                        compute_region_size(region, sizes);
                    }
                    let (column_w, content_h) = concurrent_region_metrics(&node.regions, sizes);
                    let content_w = column_w * n_regions + REGION_DIVIDER_GAP * (n_regions - 1);
                    let w = content_w + COMPOSITE_PAD_X * 2;
                    let h = content_h + COMPOSITE_PAD_Y + actions_h + COMPOSITE_PAD_BOT;
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
                    let h = total_h + COMPOSITE_PAD_Y + actions_h + COMPOSITE_PAD_BOT;
                    (w.max(STATE_NODE_W), h.max(STATE_NODE_H + 20))
                }
            }
        }
    };
    sizes.insert(node.name.clone(), result);
    result
}

/// Compute the (w, h) needed to lay out all nodes in a region (vertical stack).
pub(in crate::render::state) fn compute_region_size(
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

pub(in crate::render::state) fn node_display_lines(node: &StateNode) -> Vec<String> {
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

pub(in crate::render::state) fn concurrent_region_metrics(
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
pub(in crate::render::state) fn place_node(
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
        // Children start after the composite header label area plus any
        // internal action lines (entry/exit/do actions, closes #1304).
        let actions_h = (node.internal_actions.len() as i32) * 14;
        if node.regions.len() > 1 {
            let (column_w, _) = concurrent_region_metrics(&node.regions, sizes);
            let mut region_x = x + COMPOSITE_PAD_X;
            let content_top = y + COMPOSITE_PAD_Y + actions_h;
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
            let mut child_y = y + COMPOSITE_PAD_Y + actions_h;
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

pub(in crate::render::state) fn boundary_point_x(
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
