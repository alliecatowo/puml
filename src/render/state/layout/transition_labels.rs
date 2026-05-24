use super::super::*;

pub(in crate::render::state) fn shift_layout_for_transition_labels<'a>(
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
pub(in crate::render::state) fn expand_canvas_for_transition_labels<'a>(
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
