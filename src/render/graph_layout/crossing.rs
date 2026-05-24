use super::{EdgeSpec, NodeSize};
use std::collections::BTreeMap;

/// Returns: rank → ordered list of node IDs (minimised crossings).
pub(super) fn minimise_crossings(
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
    let mut above_neighbors: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut below_neighbors: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
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
            let pos_above: BTreeMap<String, f64> = rank_order
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
            let pos_below: BTreeMap<String, f64> = rank_order
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

    // ── Adjacent-transposition pass (bipartite crossing refinement) ───────────
    // After barycenter sweeps, nodes that share identical barycenters (e.g. two
    // web servers both connected to the same pair of backends — a K_{2,2}
    // bipartite subgraph) converge to a stable but crossing-containing order
    // because all barycenters tie.  A pass of adjacent transpositions resolves
    // this: for every pair of adjacent nodes in a rank, try swapping them and
    // keep the swap only when it strictly reduces the number of edge crossings
    // with the neighbouring ranks.  Repeat until stable (typically 1–3 passes).
    let max_rank = rank_order.keys().copied().max().unwrap_or(0);
    let mut improved = true;
    while improved {
        improved = false;
        for r in 0..=max_rank {
            let order_len = rank_order.get(&r).map(|v| v.len()).unwrap_or(0);
            for i in 0..order_len.saturating_sub(1) {
                let before = crossings_for_rank(&rank_order, &below_neighbors, r);
                if let Some(cur) = rank_order.get_mut(&r) {
                    cur.swap(i, i + 1);
                }
                let after = crossings_for_rank(&rank_order, &below_neighbors, r);
                if after < before {
                    improved = true;
                } else {
                    // Revert — same or worse.
                    if let Some(cur) = rank_order.get_mut(&r) {
                        cur.swap(i, i + 1);
                    }
                }
            }
        }
    }

    rank_order
}

/// Count edge crossings touching rank `r`: bilayer(r-1, r) + bilayer(r, r+1).
///
/// Used by the adjacent-transposition pass to decide whether a swap improves
/// the overall crossing count.
fn crossings_for_rank(
    rank_order: &BTreeMap<usize, Vec<String>>,
    below_neighbors: &BTreeMap<&str, Vec<&str>>,
    r: usize,
) -> usize {
    let mut total = 0usize;
    if r > 0 {
        if let (Some(top), Some(bot)) = (rank_order.get(&(r - 1)), rank_order.get(&r)) {
            total += bilayer_crossings(top, bot, below_neighbors);
        }
    }
    if let (Some(top), Some(bot)) = (rank_order.get(&r), rank_order.get(&(r + 1))) {
        total += bilayer_crossings(top, bot, below_neighbors);
    }
    total
}

/// Count edge crossings between two adjacent rank layers via inversion count.
///
/// `top_order` is the upper rank; `bot_order` the lower rank.
/// `edges_down` maps upper-rank node → list of lower-rank neighbours.
fn bilayer_crossings(
    top_order: &[String],
    bot_order: &[String],
    edges_down: &BTreeMap<&str, Vec<&str>>,
) -> usize {
    let bot_pos: BTreeMap<&str, usize> = bot_order
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    let mut edge_targets: Vec<usize> = Vec::new();
    for top_id in top_order {
        if let Some(neighbors) = edges_down.get(top_id.as_str()) {
            let mut positions: Vec<usize> = neighbors
                .iter()
                .filter_map(|nb| bot_pos.get(*nb))
                .copied()
                .collect();
            positions.sort_unstable();
            edge_targets.extend(positions);
        }
    }
    count_inversions(&edge_targets)
}

/// Count inversions in a slice using merge-sort (O(n log n)).
///
/// Returns the inversion count AND the sorted version of the input so that
/// callers can use it in the merge step (standard merge-sort inversion
/// counting requires each half to be sorted before the cross-half comparison).
fn count_inversions_sorted(seq: &[usize]) -> (usize, Vec<usize>) {
    if seq.len() <= 1 {
        return (0, seq.to_vec());
    }
    let mid = seq.len() / 2;
    let (left_inv, left_sorted) = count_inversions_sorted(&seq[..mid]);
    let (right_inv, right_sorted) = count_inversions_sorted(&seq[mid..]);
    let mut inversions = left_inv + right_inv;
    // Merge step: count cross-half inversions and produce merged sorted output.
    let mut merged = Vec::with_capacity(seq.len());
    let (mut i, mut j) = (0, 0);
    while i < left_sorted.len() && j < right_sorted.len() {
        if left_sorted[i] <= right_sorted[j] {
            merged.push(left_sorted[i]);
            i += 1;
        } else {
            // All remaining elements in left are greater than right_sorted[j].
            inversions += left_sorted.len() - i;
            merged.push(right_sorted[j]);
            j += 1;
        }
    }
    merged.extend_from_slice(&left_sorted[i..]);
    merged.extend_from_slice(&right_sorted[j..]);
    (inversions, merged)
}

/// Count inversions in a slice using merge-sort (O(n log n)).
pub(super) fn count_inversions(seq: &[usize]) -> usize {
    count_inversions_sorted(seq).0
}

/// Barycenter using borrowed-str position map (for route_edges path)
#[allow(dead_code)]
fn barycenter(
    node: &str,
    neighbor_map: &BTreeMap<&str, Vec<&str>>,
    positions: &BTreeMap<&str, f64>,
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
    neighbor_map: &BTreeMap<&str, Vec<&str>>,
    positions: &BTreeMap<String, f64>,
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
