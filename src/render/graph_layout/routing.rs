use std::collections::{BTreeMap, BTreeSet};

use super::{EdgeSpec, NodeSize};

/// Vertical spacing between tracks within a channel (px).
const TRACK_SPACING: f64 = 8.0;
/// Number of tracks available per channel before we wrap (soft cap).
const MAX_TRACKS: usize = 12;

/// Height of the package header band that should not be crossed by edge
/// routing. Matches the `label_header` constant in family.rs (40px) plus
/// a small safety margin.
const PKG_HEADER_HEIGHT: f64 = 48.0;

#[derive(Debug)]
struct EdgeInfo {
    edge_id: String,
    src_id: String,
    tgt_id: String,
    src_rank: usize,
    tgt_rank: usize,
    src_x: f64,
}

struct RoutingContext<'a> {
    positions: &'a BTreeMap<String, (f64, f64)>,
    node_by_id: BTreeMap<&'a str, &'a NodeSize>,
    group_bounds: &'a BTreeMap<String, (f64, f64, f64, f64)>,
    rank_bottom_y: BTreeMap<usize, f64>,
    rank_top_y: BTreeMap<usize, f64>,
    edge_track: BTreeMap<String, usize>,
    channel_max_track: BTreeMap<usize, usize>,
}

#[derive(Clone, Copy)]
struct NodeFrame {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

pub(super) fn route_edges(
    nodes: &[NodeSize],
    edges: &[EdgeSpec],
    positions: &BTreeMap<String, (f64, f64)>,
    reversed_edges: &BTreeSet<String>,
    group_bounds: &BTreeMap<String, (f64, f64, f64, f64)>,
) -> BTreeMap<String, Vec<(f64, f64)>> {
    let node_by_id: BTreeMap<&str, &NodeSize> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let y_to_rank = build_y_to_rank(positions);
    let node_rank = build_node_ranks(nodes, positions, &y_to_rank);
    let rank_bottom_y = build_rank_bottoms(nodes, positions, &y_to_rank);
    let rank_top_y = build_rank_tops(nodes, positions, &y_to_rank);

    let edge_infos = collect_edge_infos(edges, positions, reversed_edges, &node_rank);
    let edge_track = assign_edge_tracks(&edge_infos);
    let channel_max_track = compute_channel_max_tracks(&edge_infos, &edge_track);

    let context = RoutingContext {
        positions,
        node_by_id,
        group_bounds,
        rank_bottom_y,
        rank_top_y,
        edge_track,
        channel_max_track,
    };

    let mut paths: BTreeMap<String, Vec<(f64, f64)>> = BTreeMap::new();
    for ei in &edge_infos {
        if let Some(path) = route_edge(ei, &context) {
            paths.insert(ei.edge_id.clone(), path);
        }
    }
    paths
}

fn build_y_to_rank(positions: &BTreeMap<String, (f64, f64)>) -> BTreeMap<i64, usize> {
    let mut sorted_ys: Vec<i64> = positions
        .values()
        .map(|&(_, y)| y as i64)
        .collect::<BTreeSet<i64>>()
        .into_iter()
        .collect();
    sorted_ys.sort_unstable();
    sorted_ys
        .into_iter()
        .enumerate()
        .map(|(rank_idx, y_key)| (y_key, rank_idx))
        .collect()
}

fn build_node_ranks<'a>(
    nodes: &'a [NodeSize],
    positions: &BTreeMap<String, (f64, f64)>,
    y_to_rank: &BTreeMap<i64, usize>,
) -> BTreeMap<&'a str, usize> {
    nodes
        .iter()
        .filter_map(|n| {
            positions.get(n.id.as_str()).map(|&(_, y)| {
                let rank = *y_to_rank.get(&(y as i64)).unwrap_or(&0);
                (n.id.as_str(), rank)
            })
        })
        .collect()
}

fn build_rank_bottoms(
    nodes: &[NodeSize],
    positions: &BTreeMap<String, (f64, f64)>,
    y_to_rank: &BTreeMap<i64, usize>,
) -> BTreeMap<usize, f64> {
    let mut bottoms: BTreeMap<usize, f64> = BTreeMap::new();
    for n in nodes {
        if let Some(&(_, y)) = positions.get(n.id.as_str()) {
            let r = *y_to_rank.get(&(y as i64)).unwrap_or(&0);
            let bot = y + n.height;
            let e = bottoms.entry(r).or_insert(bot);
            if bot > *e {
                *e = bot;
            }
        }
    }
    bottoms
}

fn build_rank_tops(
    nodes: &[NodeSize],
    positions: &BTreeMap<String, (f64, f64)>,
    y_to_rank: &BTreeMap<i64, usize>,
) -> BTreeMap<usize, f64> {
    let mut tops: BTreeMap<usize, f64> = BTreeMap::new();
    for n in nodes {
        if let Some(&(_, y)) = positions.get(n.id.as_str()) {
            let r = *y_to_rank.get(&(y as i64)).unwrap_or(&0);
            let e = tops.entry(r).or_insert(y);
            if y < *e {
                *e = y;
            }
        }
    }
    tops
}

fn collect_edge_infos(
    edges: &[EdgeSpec],
    positions: &BTreeMap<String, (f64, f64)>,
    reversed_edges: &BTreeSet<String>,
    node_rank: &BTreeMap<&str, usize>,
) -> Vec<EdgeInfo> {
    let mut edge_infos: Vec<EdgeInfo> = Vec::new();
    for e in edges {
        let (src_id, tgt_id) = if reversed_edges.contains(&e.id) {
            (e.to.as_str(), e.from.as_str())
        } else {
            (e.from.as_str(), e.to.as_str())
        };
        let Some(&(sx, _)) = positions.get(src_id) else {
            continue;
        };
        let src_rank = *node_rank.get(src_id).unwrap_or(&0);
        let tgt_rank = *node_rank.get(tgt_id).unwrap_or(&0);
        edge_infos.push(EdgeInfo {
            edge_id: e.id.clone(),
            src_id: src_id.to_string(),
            tgt_id: tgt_id.to_string(),
            src_rank,
            tgt_rank,
            src_x: sx,
        });
    }
    edge_infos.sort_by(|a, b| {
        a.src_rank
            .cmp(&b.src_rank)
            .then_with(|| {
                a.src_x
                    .partial_cmp(&b.src_x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.edge_id.cmp(&b.edge_id))
    });
    edge_infos
}

fn assign_edge_tracks(edge_infos: &[EdgeInfo]) -> BTreeMap<String, usize> {
    let mut channel_next_track: BTreeMap<usize, usize> = BTreeMap::new();
    let mut edge_track: BTreeMap<String, usize> = BTreeMap::new();

    for ei in edge_infos {
        if ei.src_rank == ei.tgt_rank {
            let ch = ei.src_rank;
            let track = *channel_next_track.entry(ch).or_insert(0);
            let next = (track + 1).min(MAX_TRACKS - 1);
            *channel_next_track.entry(ch).or_insert(0) = next;
            edge_track.insert(ei.edge_id.clone(), track);
        } else {
            let (min_r, max_r) = rank_span(ei);
            let mut track = 0usize;
            for ch in min_r..max_r {
                let t = *channel_next_track.get(&ch).unwrap_or(&0);
                track = track.max(t);
            }
            for ch in min_r..max_r {
                let next = (track + 1).min(MAX_TRACKS - 1);
                let e = channel_next_track.entry(ch).or_insert(0);
                if next > *e {
                    *e = next;
                }
            }
            edge_track.insert(ei.edge_id.clone(), track);
        }
    }

    edge_track
}

fn compute_channel_max_tracks(
    edge_infos: &[EdgeInfo],
    edge_track: &BTreeMap<String, usize>,
) -> BTreeMap<usize, usize> {
    let mut max_track_by_channel: BTreeMap<usize, usize> = BTreeMap::new();
    for ei in edge_infos {
        let track = *edge_track.get(&ei.edge_id).unwrap_or(&0);
        if ei.src_rank == ei.tgt_rank {
            update_channel_max(&mut max_track_by_channel, ei.src_rank, track);
        } else {
            let (min_r, max_r) = rank_span(ei);
            for ch in min_r..max_r {
                update_channel_max(&mut max_track_by_channel, ch, track);
            }
        }
    }
    max_track_by_channel
}

fn update_channel_max(channel_max_track: &mut BTreeMap<usize, usize>, ch: usize, track: usize) {
    let e = channel_max_track.entry(ch).or_insert(0);
    if track > *e {
        *e = track;
    }
}

fn route_edge(ei: &EdgeInfo, ctx: &RoutingContext<'_>) -> Option<Vec<(f64, f64)>> {
    let src_id = ei.src_id.as_str();
    let tgt_id = ei.tgt_id.as_str();

    let src = node_frame(src_id, ctx)?;
    let tgt = node_frame(tgt_id, ctx)?;
    let track = *ctx.edge_track.get(&ei.edge_id).unwrap_or(&0);

    if ei.src_rank == ei.tgt_rank {
        Some(route_same_rank_edge(ei, ctx, src, tgt, track))
    } else {
        Some(route_cross_rank_edge(ei, ctx, src, tgt, track))
    }
}

fn node_frame(id: &str, ctx: &RoutingContext<'_>) -> Option<NodeFrame> {
    let &(x, y) = ctx.positions.get(id)?;
    let (w, h) = ctx
        .node_by_id
        .get(id)
        .map(|n| (n.width, n.height))
        .unwrap_or((200.0, 80.0));
    Some(NodeFrame { x, y, w, h })
}

fn route_same_rank_edge(
    ei: &EdgeInfo,
    ctx: &RoutingContext<'_>,
    src: NodeFrame,
    tgt: NodeFrame,
    track: usize,
) -> Vec<(f64, f64)> {
    let src_bottom_x = src.x + src.w / 2.0;
    let src_bottom_y = src.y + src.h;
    let tgt_bottom_x = tgt.x + tgt.w / 2.0;
    let tgt_bottom_y = tgt.y + tgt.h;
    let ch_y = channel_mid_y(ei.src_rank, ctx) + symmetric_offset(ei.src_rank, track, ctx);
    vec![
        (src_bottom_x, src_bottom_y),
        (src_bottom_x, ch_y),
        (tgt_bottom_x, ch_y),
        (tgt_bottom_x, tgt_bottom_y),
    ]
}

fn route_cross_rank_edge(
    ei: &EdgeInfo,
    ctx: &RoutingContext<'_>,
    src: NodeFrame,
    tgt: NodeFrame,
    track: usize,
) -> Vec<(f64, f64)> {
    let goes_down = ei.src_rank < ei.tgt_rank;
    let (src_port_x, src_port_y) = if goes_down {
        (src.x + src.w / 2.0, src.y + src.h)
    } else {
        (src.x + src.w / 2.0, src.y)
    };
    let (tgt_port_x, tgt_port_y) = if goes_down {
        (tgt.x + tgt.w / 2.0, tgt.y)
    } else {
        (tgt.x + tgt.w / 2.0, tgt.y + tgt.h)
    };

    let (min_r, _max_r) = rank_span(ei);
    let mut pts: Vec<(f64, f64)> = Vec::new();
    pts.push((src_port_x, src_port_y));

    let column_aligned = (src_port_x - tgt_port_x).abs() <= 4.0;
    if column_aligned {
        let mid_y = (src_port_y + tgt_port_y) / 2.0;
        pts.push((src_port_x, mid_y));
    } else {
        let raw_ch_y = channel_mid_y(min_r, ctx) + symmetric_offset(min_r, track, ctx);
        let ch_y = soft_clamp_channel_y(min_r, raw_ch_y, ctx);
        pts.push((src_port_x, ch_y));
        pts.push((tgt_port_x, ch_y));
    }

    pts.push((tgt_port_x, tgt_port_y));
    pts.dedup();
    pts
}

fn rank_span(ei: &EdgeInfo) -> (usize, usize) {
    if ei.src_rank < ei.tgt_rank {
        (ei.src_rank, ei.tgt_rank)
    } else {
        (ei.tgt_rank, ei.src_rank)
    }
}

fn channel_mid_y(upper_rank: usize, ctx: &RoutingContext<'_>) -> f64 {
    let bot = ctx.rank_bottom_y.get(&upper_rank).copied().unwrap_or(0.0);
    let next_top = ctx
        .rank_top_y
        .get(&(upper_rank + 1))
        .copied()
        .unwrap_or(bot + 80.0);
    (bot + next_top) / 2.0
}

fn symmetric_offset(ch: usize, track: usize, ctx: &RoutingContext<'_>) -> f64 {
    let n_tracks_idx = *ctx.channel_max_track.get(&ch).unwrap_or(&0);
    let n_tracks = n_tracks_idx as f64;
    let bot = ctx.rank_bottom_y.get(&ch).copied().unwrap_or(0.0);
    let next_top = ctx.rank_top_y.get(&(ch + 1)).copied().unwrap_or(bot + 80.0);
    let gap = next_top - bot;
    let max_half = if gap >= 16.0 {
        (gap - 8.0) / 2.0
    } else {
        gap / 2.0
    };
    let effective_spacing = if n_tracks_idx >= 2 {
        (max_half * 2.0 / (n_tracks + 1.0)).max(TRACK_SPACING)
    } else {
        TRACK_SPACING
    };
    let raw = (track as f64 - n_tracks / 2.0) * effective_spacing;
    raw.clamp(-max_half, max_half)
}

fn soft_clamp_channel_y(ch: usize, raw: f64, ctx: &RoutingContext<'_>) -> f64 {
    let bot = ctx.rank_bottom_y.get(&ch).copied().unwrap_or(0.0);
    let next_top = ctx.rank_top_y.get(&(ch + 1)).copied().unwrap_or(bot + 80.0);
    let gap = next_top - bot;
    let clamped = if gap < 16.0 {
        (bot + next_top) / 2.0
    } else {
        raw.clamp(bot + 4.0, next_top - 4.0)
    };

    let mut result = clamped;
    for &(_, gy, _, _) in ctx.group_bounds.values() {
        if gy < bot || gy > next_top {
            continue;
        }
        let header_bottom = gy + PKG_HEADER_HEIGHT;
        if result < header_bottom {
            let pushed = (header_bottom + 4.0).min(next_top - 4.0);
            result = result.max(pushed);
        }
    }
    result
}
