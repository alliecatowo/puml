use crate::render::graph_layout::NodeSize;
use crate::render::layout_constants::COMPONENT_BOX_WIDTH;
use crate::render_core::{Rect, RouteChannel};
use std::collections::BTreeMap;

pub(super) struct RouteChannelBuildInput<'a, ChannelMidY, SymmetricOffset>
where
    ChannelMidY: Fn(usize) -> f64,
    SymmetricOffset: Fn(usize, usize) -> f64,
{
    pub(super) channel_max_track: &'a BTreeMap<usize, usize>,
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
