use super::{LayoutOptions, NodeSize};
use crate::render::layout_constants::{COMPONENT_BOX_HEIGHT, COMPONENT_BOX_WIDTH, PKG_TAB_HEIGHT};
use std::collections::BTreeMap;

pub(super) fn compute_group_bounds(
    nodes: &[NodeSize],
    positions: &BTreeMap<String, (f64, f64)>,
    options: &LayoutOptions,
) -> BTreeMap<String, (f64, f64, f64, f64)> {
    // Collect parent → children
    let mut children_by_group: BTreeMap<String, Vec<(&str, f64, f64)>> = BTreeMap::new();
    for n in nodes {
        if let Some(parent) = &n.parent {
            if let Some(&(x, y)) = positions.get(n.id.as_str()) {
                children_by_group
                    .entry(parent.clone())
                    .or_default()
                    .push((n.id.as_str(), x, y));
            }
        }
    }

    let node_by_id: BTreeMap<&str, &NodeSize> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    let pad = options.group_padding;
    // label_reserve must match PKG_TAB_HEIGHT used by the component renderer.
    // Using the same constant ensures group_bounds.gy accurately reflects the
    // rendered frame top so the package-header avoidance check fires correctly.
    let label_reserve = PKG_TAB_HEIGHT as f64;

    let mut bounds: BTreeMap<String, (f64, f64, f64, f64)> = BTreeMap::new();
    for (group_id, children) in &children_by_group {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for &(id, cx, cy) in children {
            let (nw, nh) = node_by_id
                .get(id)
                .map(|n| (n.width, n.height))
                .unwrap_or((COMPONENT_BOX_WIDTH as f64, COMPONENT_BOX_HEIGHT as f64));
            min_x = min_x.min(cx);
            min_y = min_y.min(cy);
            max_x = max_x.max(cx + nw);
            max_y = max_y.max(cy + nh);
        }
        if min_x == f64::MAX {
            continue;
        }
        let gx = min_x - pad;
        let gy = min_y - pad - label_reserve;
        let gw = (max_x - min_x) + pad * 2.0;
        let gh = (max_y - min_y) + pad * 2.0 + label_reserve;
        bounds.insert(group_id.clone(), (gx, gy, gw, gh));
    }

    // Recursive nesting pass (#1287, #1290): when leaf nodes are parented to
    // their DEEPEST scope (e.g. "com::acme::hr"), ancestor scopes ("com",
    // "com::acme") have no direct children in `children_by_group` and would
    // otherwise be missing from `bounds`.  Infer the parent/child relation
    // from `::` scope prefixes and grow each ancestor's bbox to enclose all
    // descendant bboxes plus an outset proportional to the nesting depth.
    //
    // We work from deepest scope outward, accumulating each child's outer
    // rectangle into its prefix-parent.
    let mut scopes: Vec<String> = bounds.keys().cloned().collect();
    // Also seed ancestor scopes that are NOT themselves currently in `bounds`
    // (their leaves all live in deeper sub-scopes).
    let mut ancestor_seeds: std::collections::BTreeSet<String> = Default::default();
    for scope in &scopes {
        let parts: Vec<&str> = scope.split("::").filter(|p| !p.is_empty()).collect();
        for i in 1..parts.len() {
            ancestor_seeds.insert(parts[..i].join("::"));
        }
    }
    for ancestor in &ancestor_seeds {
        if !bounds.contains_key(ancestor) {
            scopes.push(ancestor.clone());
        }
    }
    // Deepest-first iteration so that when we grow a parent, the children's
    // bboxes are already final.
    scopes.sort_by(|a, b| {
        let depth_a = a.split("::").filter(|p| !p.is_empty()).count();
        let depth_b = b.split("::").filter(|p| !p.is_empty()).count();
        depth_b.cmp(&depth_a).then_with(|| a.cmp(b))
    });

    for scope in &scopes {
        let parts: Vec<&str> = scope.split("::").filter(|p| !p.is_empty()).collect();
        // Walk every strict ancestor prefix.
        for i in 1..parts.len() {
            let parent = parts[..i].join("::");
            let child = bounds.get(scope).copied();
            let Some((cx, cy, cw, ch)) = child else { break };
            // Outset reserves space for each level's label tab so frames don't
            // visually share the same top edge as a child frame.
            let outset = label_reserve;
            let cx_left = cx - outset;
            let cy_top = cy - outset;
            let cx_right = cx + cw + outset;
            let cy_bottom = cy + ch + outset;
            bounds
                .entry(parent.clone())
                .and_modify(|prev| {
                    let (px, py, pw, ph) = *prev;
                    let new_left = px.min(cx_left);
                    let new_top = py.min(cy_top);
                    let new_right = (px + pw).max(cx_right);
                    let new_bottom = (py + ph).max(cy_bottom);
                    *prev = (
                        new_left,
                        new_top,
                        new_right - new_left,
                        new_bottom - new_top,
                    );
                })
                .or_insert((cx_left, cy_top, cx_right - cx_left, cy_bottom - cy_top));
        }
    }

    bounds
}

// ─────────────────────────────────────────────────────────────────────────────
