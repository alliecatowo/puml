//! Hierarchical (Sugiyama-style) graph layout for component, deployment, and C4 diagrams.
//!
//! Stage 2 of the layout engine refactor (#592, child of #590).
//!
//! Pipeline:
//!   1. Rank assignment   — longest-path topological sort; cycle detection via DFS
//!   2. Crossing minim.  — barycenter heuristic, up to 12 alternating sweeps
//!   3. Coord assignment  — simple: nodes left-to-right per rank, ranks top-to-bottom
//!   4. Group bounds      — bounding box over all children + padding
//!   5. Edge routing      — straight line using pick_port anchor logic

use std::collections::{BTreeMap, BTreeSet, HashMap};

// ─────────────────────────────────────────────────────────────────────────────
// Public API types
// ─────────────────────────────────────────────────────────────────────────────

/// Size specification for one node.
#[derive(Debug, Clone)]
pub struct NodeSize {
    pub id: String,
    pub width: f64,
    pub height: f64,
    /// Optional parent group id; nodes with the same parent are enclosed together.
    pub parent: Option<String>,
}

/// One directed edge in the graph.
#[derive(Debug, Clone)]
pub struct EdgeSpec {
    pub id: String,
    pub from: String,
    pub to: String,
}

/// Result of the layout pass.
#[derive(Debug, Clone, Default)]
pub struct GraphLayout {
    /// Map node id → (x, y) of top-left corner.
    pub node_positions: BTreeMap<String, (f64, f64)>,
    /// Map node id → assigned rank (0 = top).
    /// Exposed for diagnostics and Stage 3 orthogonal routing.
    #[allow(dead_code)]
    pub node_ranks: BTreeMap<String, usize>,
    /// Map edge id → two-point path (straight line anchor points).
    /// Will be used by Stage 3 orthogonal routing.
    #[allow(dead_code)]
    pub edge_paths: BTreeMap<String, Vec<(f64, f64)>>,
    /// Map group id → bounding rect (x, y, w, h).
    pub group_bounds: BTreeMap<String, (f64, f64, f64, f64)>,
    /// Total canvas width.
    pub canvas_width: f64,
    /// Total canvas height.
    pub canvas_height: f64,
}

/// Layout direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    TopDown,
    /// Planned for Stage 3 (left-to-right flow for wide diagrams).
    #[allow(dead_code)]
    LeftRight,
}

/// Tuneable layout parameters.
#[derive(Debug, Clone)]
pub struct LayoutOptions {
    /// Vertical gap between ranks (pixels).
    pub rank_separation: f64,
    /// Horizontal gap between nodes in the same rank.
    pub node_separation: f64,
    /// Padding inside group containers.
    pub group_padding: f64,
    /// Flow direction (TopDown or LeftRight).
    #[allow(dead_code)]
    pub direction: Direction,
    /// Left/top margin around the full canvas.
    pub canvas_margin: f64,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            rank_separation: 80.0,
            node_separation: 60.0,
            group_padding: 28.0,
            direction: Direction::TopDown,
            canvas_margin: 40.0,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Main entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Run the full Sugiyama-style hierarchical layout pipeline.
pub fn layout_hierarchical(
    nodes: &[NodeSize],
    edges: &[EdgeSpec],
    options: &LayoutOptions,
) -> GraphLayout {
    if nodes.is_empty() {
        return GraphLayout::default();
    }

    // Step 1 — Build adjacency + rank assignment
    let (ranks, reversed_edges) = assign_ranks(nodes, edges);

    // Step 2 — Crossing minimisation (barycenter, 12 sweeps)
    let rank_order = minimise_crossings(nodes, edges, &ranks, 12);

    // Step 3 — Coordinate assignment
    let (node_positions, canvas_width, canvas_height) =
        assign_coordinates(nodes, &ranks, &rank_order, options);

    // Step 4 — Group bounding boxes
    let group_bounds = compute_group_bounds(nodes, &node_positions, options);

    // Step 5 — Edge routing (straight lines via port logic)
    let edge_paths = route_edges(nodes, edges, &node_positions, &reversed_edges);

    GraphLayout {
        node_positions,
        node_ranks: ranks,
        edge_paths,
        group_bounds,
        canvas_width,
        canvas_height,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Step 1: Rank assignment
// ─────────────────────────────────────────────────────────────────────────────

/// Returns (rank map, set of edge ids that were reversed to break cycles).
fn assign_ranks(
    nodes: &[NodeSize],
    edges: &[EdgeSpec],
) -> (BTreeMap<String, usize>, BTreeSet<String>) {
    // Build adjacency list (forward and reverse) as sorted Vecs for determinism.
    let mut adj_fwd: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut adj_rev: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for n in nodes {
        adj_fwd.entry(n.id.as_str()).or_default();
        adj_rev.entry(n.id.as_str()).or_default();
    }
    for e in edges {
        adj_fwd.entry(e.from.as_str()).or_default().push(e.to.as_str());
        adj_rev.entry(e.to.as_str()).or_default().push(e.from.as_str());
    }

    // Cycle detection + breaking via DFS (reverse one back-edge per cycle).
    let mut reversed: BTreeSet<String> = BTreeSet::new();
    let mut working_edges: Vec<(&str, &str)> = edges
        .iter()
        .map(|e| (e.from.as_str(), e.to.as_str()))
        .collect();

    {
        let node_ids: Vec<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
        let back_edges = find_back_edges(&node_ids, &working_edges);
        for (u, v) in &back_edges {
            // Reverse this edge: remove (u→v), add (v→u)
            working_edges.retain(|&(a, b)| !(a == *u && b == *v));
            working_edges.push((v, u));
            // Mark the original edge as reversed
            for e in edges {
                if e.from.as_str() == *u && e.to.as_str() == *v {
                    reversed.insert(e.id.clone());
                }
            }
        }
    }

    // Rebuild adjacency from (possibly cycle-broken) edge list
    let mut dag_fwd: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    let mut dag_rev: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for n in nodes {
        dag_fwd.entry(n.id.as_str()).or_default();
        dag_rev.entry(n.id.as_str()).or_default();
    }
    for &(u, v) in &working_edges {
        dag_fwd.entry(u).or_default().insert(v);
        dag_rev.entry(v).or_default().insert(u);
    }

    // Longest-path rank assignment: rank[n] = 1 + max(rank[pred])
    // Process in topological order.
    let topo = topo_sort(nodes, &dag_fwd);
    let mut ranks: BTreeMap<String, usize> = BTreeMap::new();
    for node_id in &topo {
        let max_pred_rank = dag_rev
            .get(node_id.as_str())
            .map(|preds| {
                preds
                    .iter()
                    .filter_map(|p| ranks.get(*p))
                    .copied()
                    .max()
                    .map(|r| r + 1)
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        ranks.insert(node_id.clone(), max_pred_rank);
    }

    // Nodes not reached by topo_sort (disconnected) get rank 0
    for n in nodes {
        ranks.entry(n.id.clone()).or_insert(0);
    }

    (ranks, reversed)
}

/// DFS-based back-edge detection. Returns list of (u, v) back edges.
fn find_back_edges<'a>(
    nodes: &[&'a str],
    edges: &[(&'a str, &'a str)],
) -> Vec<(&'a str, &'a str)> {
    let mut adj: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for &n in nodes {
        adj.entry(n).or_default();
    }
    for &(u, v) in edges {
        adj.entry(u).or_default().push(v);
    }

    let mut visited: BTreeSet<&str> = BTreeSet::new();
    let mut on_stack: BTreeSet<&str> = BTreeSet::new();
    let mut back_edges: Vec<(&str, &str)> = Vec::new();

    fn dfs<'a>(
        node: &'a str,
        adj: &BTreeMap<&'a str, Vec<&'a str>>,
        visited: &mut BTreeSet<&'a str>,
        on_stack: &mut BTreeSet<&'a str>,
        back_edges: &mut Vec<(&'a str, &'a str)>,
    ) {
        visited.insert(node);
        on_stack.insert(node);
        if let Some(neighbors) = adj.get(node) {
            let mut neighbors = neighbors.clone();
            neighbors.sort_unstable(); // determinism
            for nb in neighbors {
                if !visited.contains(nb) {
                    dfs(nb, adj, visited, on_stack, back_edges);
                } else if on_stack.contains(nb) {
                    back_edges.push((node, nb));
                }
            }
        }
        on_stack.remove(node);
    }

    // Sort nodes for deterministic traversal
    let mut sorted_nodes = nodes.to_vec();
    sorted_nodes.sort_unstable();
    for &n in &sorted_nodes {
        if !visited.contains(n) {
            dfs(n, &adj, &mut visited, &mut on_stack, &mut back_edges);
        }
    }
    back_edges
}

/// Topological sort using Kahn's algorithm.
fn topo_sort(nodes: &[NodeSize], dag_fwd: &BTreeMap<&str, BTreeSet<&str>>) -> Vec<String> {
    let mut in_degree: BTreeMap<&str, usize> = BTreeMap::new();
    for n in nodes {
        in_degree.entry(n.id.as_str()).or_insert(0);
    }
    for (_, succs) in dag_fwd {
        for &v in succs {
            *in_degree.entry(v).or_insert(0) += 1;
        }
    }

    // Use sorted queue for determinism
    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(&n, _)| n)
        .collect();
    queue.sort_unstable();

    let mut result: Vec<String> = Vec::new();
    while !queue.is_empty() {
        queue.sort_unstable(); // keep sorted for determinism
        let u = queue.remove(0);
        result.push(u.to_string());
        if let Some(succs) = dag_fwd.get(u) {
            let mut succs: Vec<&str> = succs.iter().copied().collect();
            succs.sort_unstable();
            for v in succs {
                let d = in_degree.entry(v).or_insert(0);
                *d = d.saturating_sub(1);
                if *d == 0 {
                    queue.push(v);
                }
            }
        }
    }
    result
}

// ─────────────────────────────────────────────────────────────────────────────
// Step 2: Crossing minimisation (barycenter heuristic)
// ─────────────────────────────────────────────────────────────────────────────

/// Returns: rank → ordered list of node IDs (minimised crossings).
fn minimise_crossings(
    nodes: &[NodeSize],
    edges: &[EdgeSpec],
    ranks: &BTreeMap<String, usize>,
    max_iters: usize,
) -> BTreeMap<usize, Vec<String>> {
    // Group nodes by rank, initial order = declaration order (stable)
    let mut rank_order: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for n in nodes {
        let r = ranks.get(&n.id).copied().unwrap_or(0);
        rank_order.entry(r).or_default().push(n.id.clone());
    }

    // Build neighbour maps for barycenter calculation
    let mut above_neighbors: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut below_neighbors: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in edges {
        below_neighbors
            .entry(e.from.as_str())
            .or_default()
            .push(e.to.as_str());
        above_neighbors
            .entry(e.to.as_str())
            .or_default()
            .push(e.from.as_str());
    }

    let max_rank = ranks.values().copied().max().unwrap_or(0);

    for _iter in 0..max_iters {
        let changed_before = rank_order.clone();

        // Downward sweep: sort each rank by barycenter of rank-above neighbors
        for r in 1..=max_rank {
            // Clone the above-rank ordering so we can mutably borrow the current rank.
            let pos_above: HashMap<String, f64> = rank_order
                .get(&(r - 1))
                .map(|above| {
                    above
                        .iter()
                        .enumerate()
                        .map(|(i, id)| (id.clone(), i as f64))
                        .collect()
                })
                .unwrap_or_default();
            if let Some(cur) = rank_order.get_mut(&r) {
                cur.sort_by(|a, b| {
                    let ba = barycenter_owned(a.as_str(), &above_neighbors, &pos_above);
                    let bb = barycenter_owned(b.as_str(), &above_neighbors, &pos_above);
                    ba.partial_cmp(&bb)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.cmp(b))
                });
            }
        }

        // Upward sweep: sort each rank by barycenter of rank-below neighbors
        for r in (0..max_rank).rev() {
            let pos_below: HashMap<String, f64> = rank_order
                .get(&(r + 1))
                .map(|below| {
                    below
                        .iter()
                        .enumerate()
                        .map(|(i, id)| (id.clone(), i as f64))
                        .collect()
                })
                .unwrap_or_default();
            if let Some(cur) = rank_order.get_mut(&r) {
                cur.sort_by(|a, b| {
                    let ba = barycenter_owned(a.as_str(), &below_neighbors, &pos_below);
                    let bb = barycenter_owned(b.as_str(), &below_neighbors, &pos_below);
                    ba.partial_cmp(&bb)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| a.cmp(b))
                });
            }
        }

        if rank_order == changed_before {
            break; // stable
        }
    }

    rank_order
}

/// Barycenter using borrowed-str position map (for route_edges path)
#[allow(dead_code)]
fn barycenter(
    node: &str,
    neighbor_map: &HashMap<&str, Vec<&str>>,
    positions: &HashMap<&str, f64>,
) -> f64 {
    let Some(neighbors) = neighbor_map.get(node) else {
        return f64::MAX;
    };
    let known: Vec<f64> = neighbors
        .iter()
        .filter_map(|nb| positions.get(nb))
        .copied()
        .collect();
    if known.is_empty() {
        return f64::MAX;
    }
    known.iter().sum::<f64>() / known.len() as f64
}

/// Barycenter using owned-String position map (avoids borrow conflicts in sweeps).
fn barycenter_owned(
    node: &str,
    neighbor_map: &HashMap<&str, Vec<&str>>,
    positions: &HashMap<String, f64>,
) -> f64 {
    let Some(neighbors) = neighbor_map.get(node) else {
        return f64::MAX;
    };
    let known: Vec<f64> = neighbors
        .iter()
        .filter_map(|nb| positions.get(*nb))
        .copied()
        .collect();
    if known.is_empty() {
        return f64::MAX;
    }
    known.iter().sum::<f64>() / known.len() as f64
}

// ─────────────────────────────────────────────────────────────────────────────
// Step 3: Coordinate assignment
// ─────────────────────────────────────────────────────────────────────────────

fn assign_coordinates(
    nodes: &[NodeSize],
    ranks: &BTreeMap<String, usize>,
    rank_order: &BTreeMap<usize, Vec<String>>,
    options: &LayoutOptions,
) -> (BTreeMap<String, (f64, f64)>, f64, f64) {
    let node_by_id: BTreeMap<&str, &NodeSize> =
        nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    let max_rank = ranks.values().copied().max().unwrap_or(0);

    // Compute per-rank y positions (top of rank = y of tallest node in rank)
    let mut rank_y: Vec<f64> = vec![0.0; max_rank + 1];
    {
        let mut y = options.canvas_margin;
        for r in 0..=max_rank {
            rank_y[r] = y;
            let max_h = rank_order
                .get(&r)
                .map(|ids| {
                    ids.iter()
                        .filter_map(|id| node_by_id.get(id.as_str()))
                        .map(|n| n.height)
                        .fold(0.0_f64, f64::max)
                })
                .unwrap_or(0.0);
            y += max_h + options.rank_separation;
        }
    }

    // For each rank, compute x positions centred around the widest rank
    // First, compute total width of each rank
    let rank_widths: Vec<f64> = (0..=max_rank)
        .map(|r| {
            rank_order
                .get(&r)
                .map(|ids| {
                    let n_nodes = ids.len() as f64;
                    let total_node_w: f64 = ids
                        .iter()
                        .filter_map(|id| node_by_id.get(id.as_str()))
                        .map(|n| n.width)
                        .sum();
                    total_node_w + (n_nodes - 1.0).max(0.0) * options.node_separation
                })
                .unwrap_or(0.0)
        })
        .collect();

    let max_rank_width = rank_widths.iter().cloned().fold(0.0_f64, f64::max);
    let canvas_content_width = max_rank_width + 2.0 * options.canvas_margin;

    let mut positions: BTreeMap<String, (f64, f64)> = BTreeMap::new();

    for r in 0..=max_rank {
        let Some(ids) = rank_order.get(&r) else {
            continue;
        };
        let rw = rank_widths[r];
        // Centre this rank horizontally
        let rank_start_x = options.canvas_margin + (max_rank_width - rw) / 2.0;
        let ry = rank_y[r];

        let mut x = rank_start_x;
        for id in ids {
            let w = node_by_id
                .get(id.as_str())
                .map(|n| n.width)
                .unwrap_or(200.0);
            positions.insert(id.clone(), (x, ry));
            x += w + options.node_separation;
        }
    }

    // Canvas size
    let canvas_height = {
        let bottom = rank_y[max_rank]
            + rank_order
                .get(&max_rank)
                .map(|ids| {
                    ids.iter()
                        .filter_map(|id| node_by_id.get(id.as_str()))
                        .map(|n| n.height)
                        .fold(0.0_f64, f64::max)
                })
                .unwrap_or(0.0);
        bottom + options.canvas_margin
    };

    (positions, canvas_content_width, canvas_height)
}

// ─────────────────────────────────────────────────────────────────────────────
// Step 4: Group bounding boxes
// ─────────────────────────────────────────────────────────────────────────────

fn compute_group_bounds(
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

    let node_by_id: BTreeMap<&str, &NodeSize> =
        nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    let pad = options.group_padding;
    let label_reserve = 28.0; // space for the group label tab

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
                .unwrap_or((200.0, 80.0));
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
// Step 5: Edge routing (straight lines, port-snapped)
// ─────────────────────────────────────────────────────────────────────────────

fn route_edges(
    nodes: &[NodeSize],
    edges: &[EdgeSpec],
    positions: &BTreeMap<String, (f64, f64)>,
    reversed_edges: &BTreeSet<String>,
) -> BTreeMap<String, Vec<(f64, f64)>> {
    let node_by_id: BTreeMap<&str, &NodeSize> =
        nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    let mut paths: BTreeMap<String, Vec<(f64, f64)>> = BTreeMap::new();

    for e in edges {
        let (src_id, tgt_id) = if reversed_edges.contains(&e.id) {
            (e.to.as_str(), e.from.as_str())
        } else {
            (e.from.as_str(), e.to.as_str())
        };

        let src_pos = positions.get(src_id);
        let tgt_pos = positions.get(tgt_id);
        let (Some(&(sx, sy)), Some(&(tx, ty))) = (src_pos, tgt_pos) else {
            continue;
        };

        let (sw, sh) = node_by_id
            .get(src_id)
            .map(|n| (n.width, n.height))
            .unwrap_or((200.0, 80.0));
        let (tw, th) = node_by_id
            .get(tgt_id)
            .map(|n| (n.width, n.height))
            .unwrap_or((200.0, 80.0));

        // Port-snapping: use same logic as pick_port in geometry.rs
        let (x1, y1, x2, y2) = pick_port_f64(
            (sx, sy, sw, sh),
            (tx, ty, tw, th),
        );

        paths.insert(e.id.clone(), vec![(x1, y1), (x2, y2)]);
    }
    paths
}

/// f64 version of pick_port from geometry.rs
fn pick_port_f64(
    src: (f64, f64, f64, f64),
    tgt: (f64, f64, f64, f64),
) -> (f64, f64, f64, f64) {
    let (sx, sy, sw, sh) = src;
    let (tx, ty, tw, th) = tgt;
    let scx = sx + sw / 2.0;
    let scy = sy + sh / 2.0;
    let tcx = tx + tw / 2.0;
    let tcy = ty + th / 2.0;
    let dx = tcx - scx;
    let dy = tcy - scy;
    if dx.abs() > dy.abs() {
        if dx > 0.0 {
            (sx + sw, scy, tx, tcy)
        } else {
            (sx, scy, tx + tw, tcy)
        }
    } else {
        if dy > 0.0 {
            (scx, sy + sh, tcx, ty)
        } else {
            (scx, sy, tcx, ty + th)
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Utility: convert GraphLayout positions (f64) to i32 for the SVG renderer
// ─────────────────────────────────────────────────────────────────────────────

/// Convert a GraphLayout position map from f64 to i32 tuples (x, y, w, h)
/// given the node size list for width/height lookup.
/// Used by Stage 3 (orthogonal routing integration).
#[allow(dead_code)]
pub fn layout_to_i32_positions(
    layout: &GraphLayout,
    nodes: &[NodeSize],
) -> BTreeMap<String, (i32, i32, i32, i32)> {
    let node_by_id: BTreeMap<&str, &NodeSize> =
        nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut out: BTreeMap<String, (i32, i32, i32, i32)> = BTreeMap::new();
    for (id, &(x, y)) in &layout.node_positions {
        let (w, h) = node_by_id
            .get(id.as_str())
            .map(|n| (n.width as i32, n.height as i32))
            .unwrap_or((200, 80));
        out.insert(id.clone(), (x as i32, y as i32, w, h));
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, parent: Option<&str>) -> NodeSize {
        NodeSize {
            id: id.to_string(),
            width: 200.0,
            height: 80.0,
            parent: parent.map(|s| s.to_string()),
        }
    }

    fn make_edge(id: &str, from: &str, to: &str) -> EdgeSpec {
        EdgeSpec {
            id: id.to_string(),
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    #[test]
    fn empty_graph_returns_default() {
        let layout = layout_hierarchical(&[], &[], &LayoutOptions::default());
        assert!(layout.node_positions.is_empty());
    }

    #[test]
    fn single_node_is_placed() {
        let nodes = vec![make_node("A", None)];
        let layout = layout_hierarchical(&nodes, &[], &LayoutOptions::default());
        assert!(layout.node_positions.contains_key("A"));
        let &(x, y) = layout.node_positions.get("A").unwrap();
        assert!(x >= 0.0);
        assert!(y >= 0.0);
    }

    #[test]
    fn linear_chain_assigns_increasing_ranks() {
        // A → B → C should get ranks 0, 1, 2
        let nodes = vec![make_node("A", None), make_node("B", None), make_node("C", None)];
        let edges = vec![make_edge("e1", "A", "B"), make_edge("e2", "B", "C")];
        let layout = layout_hierarchical(&nodes, &edges, &LayoutOptions::default());
        let ra = layout.node_ranks["A"];
        let rb = layout.node_ranks["B"];
        let rc = layout.node_ranks["C"];
        assert!(ra < rb, "A rank ({ra}) should be < B rank ({rb})");
        assert!(rb < rc, "B rank ({rb}) should be < C rank ({rc})");
    }

    #[test]
    fn cycle_is_broken_gracefully() {
        // A → B → A is a cycle; layout should not panic
        let nodes = vec![make_node("A", None), make_node("B", None)];
        let edges = vec![make_edge("e1", "A", "B"), make_edge("e2", "B", "A")];
        let layout = layout_hierarchical(&nodes, &edges, &LayoutOptions::default());
        // Both nodes must be placed
        assert!(layout.node_positions.contains_key("A"));
        assert!(layout.node_positions.contains_key("B"));
    }

    #[test]
    fn group_bounds_are_computed() {
        let nodes = vec![
            make_node("A", Some("G1")),
            make_node("B", Some("G1")),
        ];
        let layout = layout_hierarchical(&nodes, &[], &LayoutOptions::default());
        assert!(layout.group_bounds.contains_key("G1"));
        let (_, _, w, h) = layout.group_bounds["G1"];
        assert!(w > 0.0);
        assert!(h > 0.0);
    }

    #[test]
    fn top_node_is_above_bottom_node() {
        // A → B: A should have smaller y than B (TopDown)
        let nodes = vec![make_node("A", None), make_node("B", None)];
        let edges = vec![make_edge("e1", "A", "B")];
        let layout = layout_hierarchical(&nodes, &edges, &LayoutOptions::default());
        let ya = layout.node_positions["A"].1;
        let yb = layout.node_positions["B"].1;
        assert!(ya < yb, "A (y={ya}) should be above B (y={yb}) in TopDown layout");
    }

    #[test]
    fn diamond_graph_no_panic() {
        // A → B, A → C, B → D, C → D  (diamond)
        let nodes = vec![
            make_node("A", None),
            make_node("B", None),
            make_node("C", None),
            make_node("D", None),
        ];
        let edges = vec![
            make_edge("e1", "A", "B"),
            make_edge("e2", "A", "C"),
            make_edge("e3", "B", "D"),
            make_edge("e4", "C", "D"),
        ];
        let layout = layout_hierarchical(&nodes, &edges, &LayoutOptions::default());
        assert_eq!(layout.node_positions.len(), 4);
        // D should be at the highest rank
        let rd = layout.node_ranks["D"];
        let ra = layout.node_ranks["A"];
        assert!(rd > ra);
    }

    #[test]
    fn architecture_overview_shape() {
        // Mirrors the architecture-overview.puml structure:
        // 5 packages, 18 nodes, ~20 edges
        let nodes = vec![
            make_node("PlantumlFE", Some("Frontends")),
            make_node("PicoumlFE", Some("Frontends")),
            make_node("MermaidFE", Some("Frontends")),
            make_node("Parser", Some("PipelineCore")),
            make_node("AST", Some("PipelineCore")),
            make_node("Normalizer", Some("PipelineCore")),
            make_node("Renderer", Some("PipelineCore")),
            make_node("Preproc", Some("SharedServices")),
            make_node("LangSvc", Some("SharedServices")),
            make_node("Diag", Some("SharedServices")),
            make_node("Theme", Some("SharedServices")),
            make_node("CLI", Some("Transports")),
            make_node("LSP", Some("Transports")),
            make_node("WASM", Some("Transports")),
            make_node("SVG", Some("OutputFormats")),
            make_node("Raster", Some("OutputFormats")),
            make_node("Text", Some("OutputFormats")),
        ];
        let edges = vec![
            make_edge("e1", "PlantumlFE", "Parser"),
            make_edge("e2", "PicoumlFE", "Parser"),
            make_edge("e3", "MermaidFE", "Parser"),
            make_edge("e4", "Preproc", "Parser"),
            make_edge("e5", "Parser", "AST"),
            make_edge("e6", "AST", "Normalizer"),
            make_edge("e7", "Normalizer", "Renderer"),
            make_edge("e8", "Theme", "Renderer"),
            make_edge("e9", "Diag", "Renderer"),
            make_edge("e10", "Renderer", "SVG"),
            make_edge("e11", "Renderer", "Raster"),
            make_edge("e12", "Renderer", "Text"),
            make_edge("e13", "CLI", "PlantumlFE"),
            make_edge("e14", "CLI", "PicoumlFE"),
            make_edge("e15", "CLI", "MermaidFE"),
            make_edge("e16", "CLI", "Preproc"),
            make_edge("e17", "LSP", "LangSvc"),
            make_edge("e18", "LangSvc", "Parser"),
            make_edge("e19", "LangSvc", "Diag"),
            make_edge("e20", "WASM", "LangSvc"),
        ];
        let layout = layout_hierarchical(&nodes, &edges, &LayoutOptions::default());
        assert_eq!(layout.node_positions.len(), 17);
        // Renderer should have higher rank than Parser
        let r_parser = layout.node_ranks["Parser"];
        let r_renderer = layout.node_ranks["Renderer"];
        assert!(r_parser < r_renderer, "Parser rank {r_parser} < Renderer rank {r_renderer}");
        // SVG should be below Renderer
        let r_svg = layout.node_ranks["SVG"];
        assert!(r_renderer < r_svg, "Renderer rank {r_renderer} < SVG rank {r_svg}");
    }
}
