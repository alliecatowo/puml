use crate::render::graph_layout::{EdgeSpec, NodeSize};
use crate::render::layout_constants::{MAX_TRACKS, PKG_HEADER_ROUTING_CLEARANCE, TRACK_SPACING};
use crate::render_core::RouteChannel;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq)]
pub(in crate::render::graph_layout) struct RouteOptions {
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

pub(in crate::render::graph_layout) struct RouteRequest<'a> {
    pub nodes: &'a [NodeSize],
    pub edges: &'a [EdgeSpec],
    pub positions: &'a BTreeMap<String, (f64, f64)>,
    pub reversed_edges: &'a BTreeSet<String>,
    pub group_bounds: &'a BTreeMap<String, (f64, f64, f64, f64)>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(in crate::render::graph_layout) struct RoutingResult {
    pub edge_paths: BTreeMap<String, Vec<(f64, f64)>>,
    pub route_channels: BTreeMap<String, RouteChannel>,
}

pub(in crate::render::graph_layout) trait Router {
    fn route(&self, request: RouteRequest<'_>) -> RoutingResult;
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(in crate::render::graph_layout) struct ChannelRouter {
    pub(in crate::render::graph_layout) options: RouteOptions,
}

impl ChannelRouter {
    #[cfg(test)]
    pub(in crate::render::graph_layout) fn new(options: RouteOptions) -> Self {
        Self { options }
    }
}

pub(in crate::render::graph_layout) fn route_edges(
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
