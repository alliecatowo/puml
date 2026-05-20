use std::collections::BTreeMap;

use super::{EdgeSpec, NodeSize};

type NeighborMap<'a> = BTreeMap<&'a str, Vec<&'a str>>;

/// Returns rank -> ordered list of node IDs with crossings minimised.
pub(super) fn minimise_crossings(
    nodes: &[NodeSize],
    edges: &[EdgeSpec],
    ranks: &BTreeMap<String, usize>,
    max_iters: usize,
) -> BTreeMap<usize, Vec<String>> {
    // Group nodes by rank, initial order = declaration order (stable).
    let mut rank_order: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for n in nodes {
        let r = ranks.get(&n.id).copied().unwrap_or(0);
        rank_order.entry(r).or_default().push(n.id.clone());
    }

    let (above_neighbors, below_neighbors) = build_neighbor_maps(edges);
    let max_rank = ranks.values().copied().max().unwrap_or(0);

    for _iter in 0..max_iters {
        let changed_before = rank_order.clone();

        // Downward sweep: sort each rank by barycenter of rank-above neighbors.
        for r in 1..=max_rank {
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

        // Upward sweep: sort each rank by barycenter of rank-below neighbors.
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
            break;
        }
    }

    refine_with_adjacent_transpositions(&mut rank_order, &below_neighbors);
    rank_order
}

fn build_neighbor_maps(edges: &[EdgeSpec]) -> (NeighborMap<'_>, NeighborMap<'_>) {
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
    (above_neighbors, below_neighbors)
}

fn refine_with_adjacent_transpositions(
    rank_order: &mut BTreeMap<usize, Vec<String>>,
    below_neighbors: &BTreeMap<&str, Vec<&str>>,
) {
    // Nodes that share identical barycenters can converge to a stable but
    // crossing-containing order. Try adjacent swaps and keep only strict wins.
    let max_rank = rank_order.keys().copied().max().unwrap_or(0);
    let mut improved = true;
    while improved {
        improved = false;
        for r in 0..=max_rank {
            let order_len = rank_order.get(&r).map(|v| v.len()).unwrap_or(0);
            for i in 0..order_len.saturating_sub(1) {
                let before = crossings_for_rank(rank_order, below_neighbors, r);
                if let Some(cur) = rank_order.get_mut(&r) {
                    cur.swap(i, i + 1);
                }
                let after = crossings_for_rank(rank_order, below_neighbors, r);
                if after < before {
                    improved = true;
                } else if let Some(cur) = rank_order.get_mut(&r) {
                    cur.swap(i, i + 1);
                }
            }
        }
    }
}

/// Count edge crossings touching rank `r`: bilayer(r-1, r) + bilayer(r, r+1).
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
/// Returns the inversion count and the sorted version of the input so callers
/// can use it in the merge step.
fn count_inversions_sorted(seq: &[usize]) -> (usize, Vec<usize>) {
    if seq.len() <= 1 {
        return (0, seq.to_vec());
    }
    let mid = seq.len() / 2;
    let (left_inv, left_sorted) = count_inversions_sorted(&seq[..mid]);
    let (right_inv, right_sorted) = count_inversions_sorted(&seq[mid..]);
    let mut inversions = left_inv + right_inv;
    let mut merged = Vec::with_capacity(seq.len());
    let (mut i, mut j) = (0, 0);
    while i < left_sorted.len() && j < right_sorted.len() {
        if left_sorted[i] <= right_sorted[j] {
            merged.push(left_sorted[i]);
            i += 1;
        } else {
            inversions += left_sorted.len() - i;
            merged.push(right_sorted[j]);
            j += 1;
        }
    }
    merged.extend_from_slice(&left_sorted[i..]);
    merged.extend_from_slice(&right_sorted[j..]);
    (inversions, merged)
}

pub(super) fn count_inversions(seq: &[usize]) -> usize {
    count_inversions_sorted(seq).0
}

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
