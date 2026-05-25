use crate::render::graph_layout::NodeSize;
use crate::render::layout_constants::COMPONENT_BOX_WIDTH;
use crate::render_core::{Rect, RouteChannel};
use std::collections::BTreeMap;

pub(super) struct ChannelEdgeOwner<'a> {
    pub(super) edge_id: &'a str,
    pub(super) src_rank: usize,
    pub(super) tgt_rank: usize,
    pub(super) track: usize,
}

pub(super) struct RouteChannelBuildInput<'a, ChannelMidY, SymmetricOffset>
where
    ChannelMidY: Fn(usize) -> f64,
    SymmetricOffset: Fn(usize, usize) -> f64,
{
    pub(super) channel_max_track: &'a BTreeMap<usize, usize>,
    pub(super) channel_edge_owners: &'a BTreeMap<(usize, usize), Vec<String>>,
    pub(super) positions: &'a BTreeMap<String, (f64, f64)>,
    pub(super) nodes: &'a [NodeSize],
    pub(super) group_bounds: &'a BTreeMap<String, (f64, f64, f64, f64)>,
    pub(super) rank_bottom_y: &'a BTreeMap<usize, f64>,
    pub(super) rank_top_y: &'a BTreeMap<usize, f64>,
    pub(super) channel_mid_y: &'a ChannelMidY,
    pub(super) symmetric_offset: &'a SymmetricOffset,
    pub(super) track_spacing: f64,
}

pub(super) fn build_route_channels<ChannelMidY, SymmetricOffset>(
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
            let owner_edge_ids = input
                .channel_edge_owners
                .get(&(upper_rank, track))
                .cloned()
                .unwrap_or_default();
            let boundary_group_ids = channel_boundary_group_ids(bounds, input.group_bounds);
            route_channels.insert(
                id.clone(),
                RouteChannel::new(id, bounds).with_graph_channel_metadata(
                    upper_rank,
                    track,
                    input.track_spacing,
                    owner_edge_ids,
                    boundary_group_ids,
                ),
            );
        }
    }
    route_channels
}

pub(super) fn collect_channel_edge_owners<'a, I>(
    edge_owners: I,
) -> BTreeMap<(usize, usize), Vec<String>>
where
    I: IntoIterator<Item = ChannelEdgeOwner<'a>>,
{
    let mut owners: BTreeMap<(usize, usize), Vec<String>> = BTreeMap::new();
    for owner in edge_owners {
        if owner.src_rank == owner.tgt_rank {
            owners
                .entry((owner.src_rank, owner.track))
                .or_default()
                .push(owner.edge_id.to_string());
        } else {
            let (min_r, max_r) = if owner.src_rank < owner.tgt_rank {
                (owner.src_rank, owner.tgt_rank)
            } else {
                (owner.tgt_rank, owner.src_rank)
            };
            for ch in min_r..max_r {
                owners
                    .entry((ch, owner.track))
                    .or_default()
                    .push(owner.edge_id.to_string());
            }
        }
    }
    for edge_ids in owners.values_mut() {
        edge_ids.sort();
        edge_ids.dedup();
    }
    owners
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

fn channel_boundary_group_ids(
    bounds: Rect,
    group_bounds: &BTreeMap<String, (f64, f64, f64, f64)>,
) -> Vec<String> {
    group_bounds
        .iter()
        .filter_map(|(group_id, &(x, y, width, height))| {
            let group_rect = Rect::new(x, y, width, height);
            bounds.intersects(group_rect).then(|| group_id.clone())
        })
        .collect()
}
