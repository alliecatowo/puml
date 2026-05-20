use std::collections::{BTreeMap, BTreeSet};

use super::{EdgeSpec, NodeSize};

/// Returns (rank map, set of edge ids that were reversed to break cycles).
pub(super) fn assign_ranks(
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
        adj_fwd
            .entry(e.from.as_str())
            .or_default()
            .push(e.to.as_str());
        adj_rev
            .entry(e.to.as_str())
            .or_default()
            .push(e.from.as_str());
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
            // Reverse this edge: remove (u->v), add (v->u)
            working_edges.retain(|&(a, b)| !(a == *u && b == *v));
            working_edges.push((v, u));
            // Mark the original edge as reversed.
            for e in edges {
                if e.from.as_str() == *u && e.to.as_str() == *v {
                    reversed.insert(e.id.clone());
                }
            }
        }
    }

    // Rebuild adjacency from the possibly cycle-broken edge list.
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

    // Longest-path rank assignment: rank[n] = 1 + max(rank[pred]).
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

    // Nodes not reached by topo_sort (disconnected) get rank 0.
    for n in nodes {
        ranks.entry(n.id.clone()).or_insert(0);
    }

    snap_group_roots_to_sibling_median(nodes, &mut ranks, &dag_fwd, &dag_rev);

    (ranks, reversed)
}

fn snap_group_roots_to_sibling_median(
    nodes: &[NodeSize],
    ranks: &mut BTreeMap<String, usize>,
    dag_fwd: &BTreeMap<&str, BTreeSet<&str>>,
    dag_rev: &BTreeMap<&str, BTreeSet<&str>>,
) {
    // When a root node (no DAG predecessors) lives in a declared parent group
    // whose other members are at a different rank, snap the root to the median
    // sibling rank provided the snap doesn't violate DAG constraints:
    //   median_rank < min(rank[successors])
    // This keeps Theme Engine inside Shared Services instead of floating to
    // rank-0 alongside Transports (CLI/LSP/WASM).
    let mut group_members: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for n in nodes {
        if let Some(parent) = &n.parent {
            group_members
                .entry(parent.clone())
                .or_default()
                .push(n.id.clone());
        }
    }

    for members in group_members.values() {
        if members.len() < 2 {
            continue;
        }
        let mut member_ranks: Vec<usize> = members
            .iter()
            .filter_map(|id| ranks.get(id))
            .copied()
            .collect();
        if member_ranks.is_empty() {
            continue;
        }
        member_ranks.sort_unstable();
        let median_rank = member_ranks[member_ranks.len() / 2];

        for id in members {
            let current_rank = match ranks.get(id) {
                Some(&r) => r,
                None => continue,
            };
            if current_rank == median_rank {
                continue;
            }
            let has_preds = dag_rev
                .get(id.as_str())
                .map(|preds| !preds.is_empty())
                .unwrap_or(false);
            if has_preds {
                continue;
            }
            let min_succ_rank: Option<usize> = dag_fwd
                .get(id.as_str())
                .and_then(|succs| succs.iter().filter_map(|s| ranks.get(*s)).copied().min());
            let ok = match min_succ_rank {
                Some(min_s) => median_rank < min_s,
                None => true,
            };
            if ok {
                ranks.insert(id.clone(), median_rank);
            }
        }
    }
}

/// DFS-based back-edge detection. Returns list of (u, v) back edges.
fn find_back_edges<'a>(nodes: &[&'a str], edges: &[(&'a str, &'a str)]) -> Vec<(&'a str, &'a str)> {
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
            neighbors.sort_unstable();
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
    for succs in dag_fwd.values() {
        for &v in succs {
            *in_degree.entry(v).or_insert(0) += 1;
        }
    }

    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, &d)| d == 0)
        .map(|(&n, _)| n)
        .collect();
    queue.sort_unstable();

    let mut result: Vec<String> = Vec::new();
    while !queue.is_empty() {
        queue.sort_unstable();
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
