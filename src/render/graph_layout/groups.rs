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
    bounds
}

// ─────────────────────────────────────────────────────────────────────────────
