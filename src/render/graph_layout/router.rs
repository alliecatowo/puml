use super::{EdgeSpec, NodeSize};
use crate::render::layout_constants::{
    COMPONENT_BOX_HEIGHT, COMPONENT_BOX_WIDTH, EDGE_PORT_FAN_MAX_SHIFT, EDGE_PORT_FAN_SPACING,
    MAX_TRACKS, PKG_HEADER_ROUTING_CLEARANCE, TRACK_SPACING,
};
use crate::render_core::{Point, Rect, RouteChannel, Segment};
use std::collections::{BTreeMap, BTreeSet};

//
// Algorithm:
//   a. Compute inter-rank channels: horizontal routing bands between adjacent
//      ranks, each 24px tall. Channels are indexed by (upper_rank, lower_rank).
//   b. Assign each edge a track within each channel it passes through.
//      Track assignment is greedy (sorted by source x), enforcing no two edges
//      share (channel, track).
//   c. Generate orthogonal polyline: bottom-port → vertical → horizontal in
//      channel → vertical → top-port. Multi-rank edges zigzag through each
//      intermediate channel.
//   d. Same-rank edges use a U-shape: down into channel BELOW the rank,
//      horizontal, then back up.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RouteOptions {
    pub track_spacing: f64,
    pub max_tracks: usize,
    pub package_header_clearance: f64,
}

impl Default for RouteOptions {
    fn default() -> Self {
        Self {
            track_spacing: TRACK_SPACING,
            max_tracks: MAX_TRACKS,
            package_header_clearance: PKG_HEADER_ROUTING_CLEARANCE,
        }
    }
}

pub(super) struct RouteRequest<'a> {
    pub nodes: &'a [NodeSize],
    pub edges: &'a [EdgeSpec],
    pub positions: &'a BTreeMap<String, (f64, f64)>,
    pub reversed_edges: &'a BTreeSet<String>,
    pub group_bounds: &'a BTreeMap<String, (f64, f64, f64, f64)>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct RoutingResult {
    pub edge_paths: BTreeMap<String, Vec<(f64, f64)>>,
    pub route_channels: BTreeMap<String, RouteChannel>,
}

pub(super) trait Router {
    fn route(&self, request: RouteRequest<'_>) -> RoutingResult;
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct ChannelRouter {
    options: RouteOptions,
}

impl ChannelRouter {
    #[cfg(test)]
    pub(super) fn new(options: RouteOptions) -> Self {
        Self { options }
    }
}

pub(super) fn route_edges(
    nodes: &[NodeSize],
    edges: &[EdgeSpec],
    positions: &BTreeMap<String, (f64, f64)>,
    reversed_edges: &BTreeSet<String>,
    group_bounds: &BTreeMap<String, (f64, f64, f64, f64)>,
) -> RoutingResult {
    ChannelRouter::default().route(RouteRequest {
        nodes,
        edges,
        positions,
        reversed_edges,
        group_bounds,
    })
}

impl Router for ChannelRouter {
    fn route(&self, request: RouteRequest<'_>) -> RoutingResult {
        let RouteRequest {
            nodes,
            edges,
            positions,
            reversed_edges,
            group_bounds,
        } = request;
        let max_tracks = self.options.max_tracks.max(1);
        let max_track_index = max_tracks - 1;

        // Build node lookup map.
        let node_by_id: BTreeMap<&str, &NodeSize> =
            nodes.iter().map(|n| (n.id.as_str(), n)).collect();

        // Compute node ranks from positions (y is consistent per rank in our layout).
        // Map unique y values → rank index (sorted ascending).
        let y_to_rank: BTreeMap<i64, usize> = {
            let mut sorted_ys: Vec<i64> = positions
                .values()
                .map(|&(_, y)| y as i64)
                .collect::<std::collections::BTreeSet<i64>>()
                .into_iter()
                .collect();
            sorted_ys.sort_unstable();
            sorted_ys
                .into_iter()
                .enumerate()
                .map(|(rank_idx, y_key)| (y_key, rank_idx))
                .collect()
        };

        let node_rank: BTreeMap<&str, usize> = nodes
            .iter()
            .filter_map(|n| {
                positions.get(n.id.as_str()).map(|&(_, y)| {
                    let rank = *y_to_rank.get(&(y as i64)).unwrap_or(&0);
                    (n.id.as_str(), rank)
                })
            })
            .collect();

        // Per-rank bottom y (max of node bottoms within that rank).
        let rank_bottom_y: BTreeMap<usize, f64> = {
            let mut m: BTreeMap<usize, f64> = BTreeMap::new();
            for n in nodes {
                if let Some(&(_, y)) = positions.get(n.id.as_str()) {
                    let r = *y_to_rank.get(&(y as i64)).unwrap_or(&0);
                    let bot = y + n.height;
                    let e = m.entry(r).or_insert(bot);
                    if bot > *e {
                        *e = bot;
                    }
                }
            }
            m
        };

        // Per-rank top y (min of node tops within that rank).
        let rank_top_y: BTreeMap<usize, f64> = {
            let mut m: BTreeMap<usize, f64> = BTreeMap::new();
            for n in nodes {
                if let Some(&(_, y)) = positions.get(n.id.as_str()) {
                    let r = *y_to_rank.get(&(y as i64)).unwrap_or(&0);
                    let e = m.entry(r).or_insert(y);
                    if y < *e {
                        *e = y;
                    }
                }
            }
            m
        };

        // channel_y(upper_rank): midpoint of the inter-rank gap between rank
        // `upper_rank` and rank `upper_rank + 1`.  With the default rank_separation
        // of 80px the midpoint is 40px below the bottom of the upper rank and 40px
        // above the top of the lower rank — enough headroom for visible orthogonal
        // bends even after symmetric track fanning.
        let channel_mid_y = |upper_rank: usize| -> f64 {
            let bot = rank_bottom_y.get(&upper_rank).copied().unwrap_or(0.0);
            let next_top = rank_top_y
                .get(&(upper_rank + 1))
                .copied()
                .unwrap_or(bot + 80.0);
            (bot + next_top) / 2.0
        };

        // ── Track assignment ───────────────────────────────────────────────────────
        // For each channel (keyed by upper_rank), track which x-ranges are occupied.
        // We use a simple slot bitmap: channel_tracks[upper_rank] = next_free_track_idx.
        // Greedy: for each channel an edge passes through, claim the same track as
        // already claimed for that edge, or the next available.

        // Process edges sorted by (src_rank, src_x) for determinism.
        struct EdgeInfo {
            edge_id: String,
            src_id: String,
            tgt_id: String,
            src_rank: usize,
            tgt_rank: usize,
            src_x: f64,
        }

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

        // ── Endpoint port fan (adjacent-rank only) ───────────────────────────────
        //
        // When multiple edges share the same endpoint port in one inter-rank
        // channel (e.g. K2,2 backend edges), spread their endpoint x positions
        // across a small horizontal fan so each edge leaves/arrives at a distinct
        // lane and port.
        let mut edge_tgt_port_dx: BTreeMap<String, f64> = BTreeMap::new();
        {
            let mut src_groups: BTreeMap<(String, usize), Vec<(&EdgeInfo, f64)>> = BTreeMap::new();
            let mut tgt_groups: BTreeMap<(String, usize), Vec<(&EdgeInfo, f64)>> = BTreeMap::new();
            for ei in &edge_infos {
                if ei.src_rank == ei.tgt_rank {
                    continue;
                }
                let (min_r, max_r) = if ei.src_rank < ei.tgt_rank {
                    (ei.src_rank, ei.tgt_rank)
                } else {
                    (ei.tgt_rank, ei.src_rank)
                };
                // Only adjacent ranks: broader spans keep centered ports to avoid
                // large visual detours.
                if max_r - min_r != 1 {
                    continue;
                }
                let src_counterpart_x = positions
                    .get(ei.tgt_id.as_str())
                    .map(|(x, _)| *x)
                    .unwrap_or(0.0);
                let tgt_counterpart_x = positions
                    .get(ei.src_id.as_str())
                    .map(|(x, _)| *x)
                    .unwrap_or(0.0);
                src_groups
                    .entry((ei.src_id.clone(), min_r))
                    .or_default()
                    .push((ei, src_counterpart_x));
                tgt_groups
                    .entry((ei.tgt_id.clone(), min_r))
                    .or_default()
                    .push((ei, tgt_counterpart_x));
            }

            let mut src_group_size_by_edge: BTreeMap<String, usize> = BTreeMap::new();
            let mut tgt_group_size_by_edge: BTreeMap<String, usize> = BTreeMap::new();
            for group in src_groups.values() {
                let n = group.len();
                for (ei, _) in group {
                    src_group_size_by_edge.insert(ei.edge_id.clone(), n);
                }
            }
            for group in tgt_groups.values() {
                let n = group.len();
                for (ei, _) in group {
                    tgt_group_size_by_edge.insert(ei.edge_id.clone(), n);
                }
            }

            let assign_fan_offsets =
                |groups: BTreeMap<(String, usize), Vec<(&EdgeInfo, f64)>>,
                 out: &mut BTreeMap<String, f64>,
                 opposite_group_size: &BTreeMap<String, usize>| {
                    for (_, mut group) in groups {
                        if group.len() <= 1 {
                            continue;
                        }
                        // Apply fan only when every edge in this shared endpoint
                        // group has a shared opposite endpoint as well. This
                        // targets true K2,2-style ambiguity and avoids partial
                        // fan-outs on one-to-many / many-to-one patterns.
                        let all_bipartite = group.iter().all(|(ei, _)| {
                            opposite_group_size.get(&ei.edge_id).copied().unwrap_or(1) > 1
                        });
                        if !all_bipartite {
                            continue;
                        }
                        group.sort_by(|(ea, xa), (eb, xb)| {
                            xa.partial_cmp(xb)
                                .unwrap_or(std::cmp::Ordering::Equal)
                                .then_with(|| ea.edge_id.cmp(&eb.edge_id))
                        });
                        let n = group.len() as f64;
                        for (idx, (ei, _)) in group.iter().enumerate() {
                            let lane = idx as f64 - (n - 1.0) / 2.0;
                            let dx = (lane * EDGE_PORT_FAN_SPACING)
                                .clamp(-EDGE_PORT_FAN_MAX_SHIFT, EDGE_PORT_FAN_MAX_SHIFT);
                            out.insert(ei.edge_id.clone(), dx);
                        }
                    }
                };
            assign_fan_offsets(tgt_groups, &mut edge_tgt_port_dx, &src_group_size_by_edge);
        }

        // channel_next_track[upper_rank] = next available track index
        let mut channel_next_track: BTreeMap<usize, usize> = BTreeMap::new();
        // edge_track[edge_id] = track index (shared across all channels that edge uses)
        let mut edge_track: BTreeMap<String, usize> = BTreeMap::new();

        for ei in &edge_infos {
            if ei.src_rank == ei.tgt_rank {
                // Same-rank: uses channel BELOW the rank (upper_rank = src_rank).
                let ch = ei.src_rank;
                let track = *channel_next_track.entry(ch).or_insert(0);
                let next = (track + 1).min(max_track_index);
                *channel_next_track.entry(ch).or_insert(0) = next;
                edge_track.insert(ei.edge_id.clone(), track);
            } else {
                // Cross-rank: pick the max next_track across all channels it passes through.
                let (min_r, max_r) = if ei.src_rank < ei.tgt_rank {
                    (ei.src_rank, ei.tgt_rank)
                } else {
                    (ei.tgt_rank, ei.src_rank)
                };
                let mut track = 0usize;
                for ch in min_r..max_r {
                    let t = *channel_next_track.get(&ch).unwrap_or(&0);
                    track = track.max(t);
                }
                for ch in min_r..max_r {
                    let next = (track + 1).min(max_track_index);
                    let e = channel_next_track.entry(ch).or_insert(0);
                    if next > *e {
                        *e = next;
                    }
                }
                edge_track.insert(ei.edge_id.clone(), track);
            }
        }

        // ── Per-channel track count (for symmetric fanning) ──────────────────────
        //
        // After track assignment, count the maximum track index used in each channel
        // so that path generation can fan track offsets symmetrically around the
        // channel midpoint:  offset(i) = (i - n_tracks/2) * TRACK_SPACING.
        //
        // This ensures that with rank_separation≈80px and a midpoint 40px from each
        // adjacent rank, even track 0 sits squarely in the middle of the gap and
        // the horizontal bend segment is clearly visible (≥10px from each port).

        // channel_max_track[ch] = highest track index allocated in that channel
        let channel_max_track: BTreeMap<usize, usize> = {
            let mut m: BTreeMap<usize, usize> = BTreeMap::new();
            for ei in &edge_infos {
                let track = *edge_track.get(&ei.edge_id).unwrap_or(&0);
                if ei.src_rank == ei.tgt_rank {
                    let e = m.entry(ei.src_rank).or_insert(0);
                    if track > *e {
                        *e = track;
                    }
                } else {
                    let (min_r, max_r) = if ei.src_rank < ei.tgt_rank {
                        (ei.src_rank, ei.tgt_rank)
                    } else {
                        (ei.tgt_rank, ei.src_rank)
                    };
                    for ch in min_r..max_r {
                        let e = m.entry(ch).or_insert(0);
                        if track > *e {
                            *e = track;
                        }
                    }
                }
            }
            m
        };

        // Symmetric track offset for a given channel and track index.
        // With n_tracks tracks in channel `ch`, track i is at:
        //   offset = (i as f64 - n_tracks as f64 / 2.0) * effective_spacing
        // so the band is centered on the channel midpoint.
        //
        // For channels with ≤ 2 tracks (≤ 2 edges crossing the gap), TRACK_SPACING
        // (8 px) is used as before — narrow fans are visually fine.  For channels
        // with ≥ 3 tracks (e.g. the four bipartite edges in a deployment web-server →
        // db/cache tier), the fan is spread adaptively to fill ~2/3 of the available
        // channel half-height so that crossing horizontal segments are clearly
        // separated rather than overlapping in a visually tangled X.
        //
        // The band half-width is capped at (inter_rank_gap − 8) / 2 in all cases so
        // that tracks never collide with the adjacent node rows.
        let symmetric_offset = |ch: usize, track: usize| -> f64 {
            let n_tracks_idx = *channel_max_track.get(&ch).unwrap_or(&0); // max track index used
            let n_tracks = n_tracks_idx as f64;
            // Compute the inter-rank gap for this channel to bound the fan width.
            let bot = rank_bottom_y.get(&ch).copied().unwrap_or(0.0);
            let next_top = rank_top_y.get(&(ch + 1)).copied().unwrap_or(bot + 80.0);
            let gap = next_top - bot;
            let max_half = if gap >= 16.0 {
                (gap - 8.0) / 2.0
            } else {
                gap / 2.0
            };
            // Adaptive spacing: only for channels with ≥ 3 tracks (max index ≥ 2).
            let effective_spacing = if n_tracks_idx >= 2 {
                // Spread the fan so adjacent tracks are ~gap/(n+2) apart, capped at
                // max_half and floored at TRACK_SPACING.
                (max_half * 2.0 / (n_tracks + 1.0)).max(self.options.track_spacing)
            } else {
                self.options.track_spacing
            };
            let raw = (track as f64 - n_tracks / 2.0) * effective_spacing;
            raw.clamp(-max_half, max_half)
        };

        // ── Path generation ────────────────────────────────────────────────────────
        //
        // Every cross-rank edge routes through the inter-rank channel midpoint plus
        // a symmetric track offset, producing a path:
        //   [src_port, (src_x, ch_y), (tgt_x, ch_y), tgt_port]
        // for a single-hop downward edge.  The ch_y is the channel midpoint ± offset,
        // which sits ~40px from each node row with the default rank_separation of 80px,
        // making the orthogonal bend clearly visible regardless of horizontal alignment.
        //
        // The near-port clamp (±2px) from Wave 14 is replaced by a softer boundary:
        // only clamp when the inter-rank gap is genuinely < 16px (degenerate layout).
        //
        // After building each path, adjacent duplicate points are removed so the
        // final polyline always has ≥3 distinct waypoints for cross-rank edges.

        let mut paths: BTreeMap<String, Vec<(f64, f64)>> = BTreeMap::new();

        for ei in &edge_infos {
            let src_id = ei.src_id.as_str();
            let tgt_id = ei.tgt_id.as_str();

            let Some(&(sx, sy)) = positions.get(src_id) else {
                continue;
            };
            let Some(&(tx, ty)) = positions.get(tgt_id) else {
                continue;
            };

            let (sw, sh) = node_by_id
                .get(src_id)
                .map(|n| (n.width, n.height))
                .unwrap_or((COMPONENT_BOX_WIDTH as f64, COMPONENT_BOX_HEIGHT as f64));
            let (tw, th) = node_by_id
                .get(tgt_id)
                .map(|n| (n.width, n.height))
                .unwrap_or((COMPONENT_BOX_WIDTH as f64, COMPONENT_BOX_HEIGHT as f64));

            let track = *edge_track.get(&ei.edge_id).unwrap_or(&0);

            let path = if ei.src_rank == ei.tgt_rank {
                // Same-rank U-shape: exit bottom of source, route through channel
                // below rank, enter bottom of target.
                let src_bottom_x = sx + sw / 2.0;
                let src_bottom_y = sy + sh;
                let tgt_bottom_x = tx + tw / 2.0;
                let tgt_bottom_y = ty + th;
                let ch_y = channel_mid_y(ei.src_rank) + symmetric_offset(ei.src_rank, track);
                vec![
                    (src_bottom_x, src_bottom_y),
                    (src_bottom_x, ch_y),
                    (tgt_bottom_x, ch_y),
                    (tgt_bottom_x, tgt_bottom_y),
                ]
            } else {
                // Cross-rank orthogonal path.
                // Determine direction: downward (src_rank < tgt_rank) or upward.
                let goes_down = ei.src_rank < ei.tgt_rank;

                // Source port: bottom if going down, top if going up.
                let (src_port_x, src_port_y) = if goes_down {
                    (sx + sw / 2.0, sy + sh)
                } else {
                    (sx + sw / 2.0, sy)
                };
                // Target port: top if going down, bottom if going up.
                let (tgt_port_x, tgt_port_y) = if goes_down {
                    (tx + tw / 2.0, ty)
                } else {
                    (tx + tw / 2.0, ty + th)
                };

                let (min_r, max_r) = if goes_down {
                    (ei.src_rank, ei.tgt_rank)
                } else {
                    (ei.tgt_rank, ei.src_rank)
                };

                // Soft clamp: only needed for degenerate layouts where the inter-rank
                // gap is < 16px and the symmetric offset could overshoot the node
                // boundary.  Normal layouts (rank_separation ≥ 80px) are unaffected.
                let soft_clamp_ch_y = |ch: usize, raw: f64| -> f64 {
                    let bot = rank_bottom_y.get(&ch).copied().unwrap_or(0.0);
                    let next_top = rank_top_y.get(&(ch + 1)).copied().unwrap_or(bot + 80.0);
                    let gap = next_top - bot;
                    let clamped = if gap < 16.0 {
                        // Degenerate gap: clamp to exact midpoint.
                        (bot + next_top) / 2.0
                    } else {
                        // Normal gap: allow any value strictly within the gap.
                        raw.clamp(bot + 4.0, next_top - 4.0)
                    };
                    // Package-header avoidance: push the channel y below any package
                    // header band whose frame top (gy) lies within this inter-rank
                    // channel [bot, next_top].  When gy is inside the channel, the
                    // vertical entry shaft from ch_y to the first node inside the
                    // package crosses [gy, gy + PKG_HEADER_ROUTING_CLEARANCE], so we must
                    // ensure ch_y >= gy + PKG_HEADER_ROUTING_CLEARANCE + 4 to keep arrow shafts
                    // out of the package label text.
                    let mut result = clamped;
                    for &(_, gy, _, _) in group_bounds.values() {
                        // Only act on packages whose frame top falls in this channel.
                        if gy < bot || gy > next_top {
                            continue;
                        }
                        let header_bottom = gy + self.options.package_header_clearance;
                        if result < header_bottom {
                            // Push below the header band; re-clamp to the inter-rank
                            // gap ceiling so we don't overshoot the target row.
                            let pushed = (header_bottom + 4.0).min(next_top - 4.0);
                            result = result.max(pushed);
                        }
                    }
                    result
                };

                // Build polyline segment by segment through each channel.
                // For a downward edge from rank R0 to rank R1 (R0 < R1):
                //   start at src_port → vertical to channel(R0) midpoint → horizontal →
                //   vertical to channel(R0+1) midpoint ... → tgt_port
                let mut pts: Vec<(f64, f64)> = Vec::new();
                pts.push((src_port_x, src_port_y));

                let tgt_port_x =
                    tgt_port_x + edge_tgt_port_dx.get(&ei.edge_id).copied().unwrap_or(0.0);

                // Column-align shortcut: when source and target ports are within
                // 4 px of each other horizontally, emit a clean straight vertical
                // with NO horizontal segment.  This eliminates the unnecessary
                // right-then-back jog for nodes that are vertically stacked in the
                // same column (e.g. Parser → AST → Normalizer → Renderer).
                let column_aligned = (src_port_x - tgt_port_x).abs() <= 4.0;
                let vertical_route = VerticalRouteCheck {
                    x: src_port_x,
                    y1: src_port_y,
                    y2: tgt_port_y,
                    source_id: src_id,
                    target_id: tgt_id,
                    nodes,
                    positions,
                };
                let blocked_vertical = vertical_route_crosses_node(vertical_route);

                if column_aligned && !blocked_vertical {
                    // Straight vertical: no channel waypoints needed.
                    // The pts vec already has src_port pushed above; just push a
                    // straight point at the midpoint y so the renderer sees ≥3
                    // distinct waypoints (required by the ≥3-point assertion).
                    let mid_y = (src_port_y + tgt_port_y) / 2.0;
                    pts.push((src_port_x, mid_y));
                } else if column_aligned && blocked_vertical && max_r - min_r > 1 {
                    let first_ch = if goes_down {
                        ei.src_rank
                    } else {
                        ei.src_rank - 1
                    };
                    let last_ch = if goes_down {
                        ei.tgt_rank - 1
                    } else {
                        ei.tgt_rank
                    };
                    let first_ch_y = soft_clamp_ch_y(
                        first_ch,
                        channel_mid_y(first_ch) + symmetric_offset(first_ch, track),
                    );
                    let last_ch_y = soft_clamp_ch_y(
                        last_ch,
                        channel_mid_y(last_ch) + symmetric_offset(last_ch, track),
                    );
                    let detour_x = detour_x_for_vertical_route(
                        vertical_route,
                        self.options.track_spacing * 2.0,
                    );
                    pts.push((src_port_x, first_ch_y));
                    pts.push((detour_x, first_ch_y));
                    pts.push((detour_x, last_ch_y));
                    pts.push((tgt_port_x, last_ch_y));
                } else if max_r - min_r == 1 {
                    // Single channel hop: route through the inter-rank channel
                    // midpoint with a symmetric track offset so the horizontal
                    // bend is clearly visible.
                    let raw_ch_y = channel_mid_y(min_r) + symmetric_offset(min_r, track);
                    let ch_y = soft_clamp_ch_y(min_r, raw_ch_y);
                    pts.push((src_port_x, ch_y));
                    pts.push((tgt_port_x, ch_y));
                } else {
                    // Multi-rank: use a single L-bend through the first inter-rank
                    // channel (just below the source rank for downward edges).  This
                    // avoids the staircase zigzag that the old per-hop interpolation
                    // produced for edges spanning 2+ ranks (e.g. actor → far use-case).
                    // The symmetric track offset is applied to the first channel so that
                    // parallel multi-rank edges fan out and don't overlap.
                    let ch = min_r;
                    let raw_ch_y = channel_mid_y(ch) + symmetric_offset(ch, track);
                    let ch_y = soft_clamp_ch_y(ch, raw_ch_y);
                    pts.push((src_port_x, ch_y));
                    pts.push((tgt_port_x, ch_y));
                }

                pts.push((tgt_port_x, tgt_port_y));

                // Remove adjacent duplicate points so the final polyline is compact
                // (≥3 distinct waypoints for a single-hop cross-rank edge).
                pts.dedup();

                pts
            };

            paths.insert(ei.edge_id.clone(), path);
        }

        RoutingResult {
            edge_paths: paths,
            route_channels: build_route_channels(RouteChannelBuildInput {
                channel_max_track: &channel_max_track,
                positions,
                nodes,
                group_bounds,
                rank_bottom_y: &rank_bottom_y,
                rank_top_y: &rank_top_y,
                channel_mid_y: &channel_mid_y,
                symmetric_offset: &symmetric_offset,
                track_spacing: self.options.track_spacing,
            }),
        }
    }
}

struct RouteChannelBuildInput<'a, ChannelMidY, SymmetricOffset>
where
    ChannelMidY: Fn(usize) -> f64,
    SymmetricOffset: Fn(usize, usize) -> f64,
{
    channel_max_track: &'a BTreeMap<usize, usize>,
    positions: &'a BTreeMap<String, (f64, f64)>,
    nodes: &'a [NodeSize],
    group_bounds: &'a BTreeMap<String, (f64, f64, f64, f64)>,
    rank_bottom_y: &'a BTreeMap<usize, f64>,
    rank_top_y: &'a BTreeMap<usize, f64>,
    channel_mid_y: &'a ChannelMidY,
    symmetric_offset: &'a SymmetricOffset,
    track_spacing: f64,
}

fn build_route_channels<ChannelMidY, SymmetricOffset>(
    input: RouteChannelBuildInput<'_, ChannelMidY, SymmetricOffset>,
) -> BTreeMap<String, RouteChannel>
where
    ChannelMidY: Fn(usize) -> f64,
    SymmetricOffset: Fn(usize, usize) -> f64,
{
    let (min_x, max_x) = route_channel_x_bounds(input.positions, input.nodes, input.group_bounds);
    let mut route_channels = BTreeMap::new();
    for (&upper_rank, &max_track) in input.channel_max_track {
        let bot = input.rank_bottom_y.get(&upper_rank).copied().unwrap_or(0.0);
        let next_top = input
            .rank_top_y
            .get(&(upper_rank + 1))
            .copied()
            .unwrap_or(bot + 80.0);
        let gap_height = (next_top - bot).max(input.track_spacing);
        for track in 0..=max_track {
            let id = route_channel_id(upper_rank, track);
            let center_y =
                (input.channel_mid_y)(upper_rank) + (input.symmetric_offset)(upper_rank, track);
            let bounds = Rect::new(
                min_x,
                center_y - input.track_spacing / 2.0,
                max_x - min_x,
                input.track_spacing.min(gap_height),
            );
            route_channels.insert(id.clone(), RouteChannel { id, bounds });
        }
    }
    route_channels
}

fn route_channel_id(upper_rank: usize, track: usize) -> String {
    format!("rank:{upper_rank}:track:{track}")
}

fn route_channel_x_bounds(
    positions: &BTreeMap<String, (f64, f64)>,
    nodes: &[NodeSize],
    group_bounds: &BTreeMap<String, (f64, f64, f64, f64)>,
) -> (f64, f64) {
    let node_by_id: BTreeMap<&str, &NodeSize> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut min_x = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    for (node_id, &(x, _)) in positions {
        let width = node_by_id
            .get(node_id.as_str())
            .map(|node| node.width)
            .unwrap_or(COMPONENT_BOX_WIDTH as f64);
        min_x = min_x.min(x);
        max_x = max_x.max(x + width);
    }
    for &(x, _, width, _) in group_bounds.values() {
        min_x = min_x.min(x);
        max_x = max_x.max(x + width);
    }
    if min_x.is_finite() && max_x.is_finite() && max_x > min_x {
        (min_x, max_x)
    } else {
        (0.0, 0.0)
    }
}

#[derive(Clone, Copy)]
struct VerticalRouteCheck<'a> {
    x: f64,
    y1: f64,
    y2: f64,
    source_id: &'a str,
    target_id: &'a str,
    nodes: &'a [NodeSize],
    positions: &'a BTreeMap<String, (f64, f64)>,
}

fn vertical_route_crosses_node(check: VerticalRouteCheck<'_>) -> bool {
    let route = Segment::new(Point::new(check.x, check.y1), Point::new(check.x, check.y2));
    check.nodes.iter().any(|node| {
        node.id != check.source_id
            && node.id != check.target_id
            && node_rect(node, check.positions)
                .is_some_and(|rect| segment_crosses_rect(route, rect))
    })
}

fn detour_x_for_vertical_route(check: VerticalRouteCheck<'_>, clearance: f64) -> f64 {
    let route = Segment::new(Point::new(check.x, check.y1), Point::new(check.x, check.y2));
    check
        .nodes
        .iter()
        .filter(|node| node.id != check.source_id && node.id != check.target_id)
        .filter_map(|node| node_rect(node, check.positions))
        .filter(|rect| segment_crosses_rect(route, *rect))
        .map(|rect| rect.max_x() + clearance)
        .fold(check.x + clearance, f64::max)
}

fn node_rect(node: &NodeSize, positions: &BTreeMap<String, (f64, f64)>) -> Option<Rect> {
    let &(x, y) = positions.get(&node.id)?;
    Some(Rect::new(x, y, node.width, node.height))
}

fn segment_crosses_rect(segment: Segment, rect: Rect) -> bool {
    if !segment.bounds().intersects(rect) {
        return false;
    }
    let min_x = rect.min_x();
    let max_x = rect.max_x();
    let min_y = rect.min_y();
    let max_y = rect.max_y();
    if segment.is_vertical() {
        return segment.start.x > min_x
            && segment.start.x < max_x
            && ranges_overlap(segment.start.y, segment.end.y, min_y, max_y);
    }
    if segment.is_horizontal() {
        return segment.start.y > min_y
            && segment.start.y < max_y
            && ranges_overlap(segment.start.x, segment.end.x, min_x, max_x);
    }
    rect.contains_point(segment.start) || rect.contains_point(segment.end)
}

fn ranges_overlap(a: f64, b: f64, c: f64, d: f64) -> bool {
    let a_min = a.min(b);
    let a_max = a.max(b);
    let c_min = c.min(d);
    let c_max = c.max(d);
    a_min < c_max && c_min < a_max
}

// ─────────────────────────────────────────────────────────────────────────────
