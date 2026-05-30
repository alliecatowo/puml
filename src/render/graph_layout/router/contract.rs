use crate::render::graph_layout::{EdgeSpec, NodeSize};
use crate::render::layout_constants::{MAX_TRACKS, PKG_HEADER_ROUTING_CLEARANCE, TRACK_SPACING};
use crate::render_core::RouteChannel;
use std::collections::{BTreeMap, BTreeSet};

/// Global edge-routing mode, selected by `skinparam linetype <value>`.
///
/// PlantUML exposes exactly three routing modes (mapped 1-to-1 onto Graphviz's
/// `splines=` attribute):
///
/// - [`EdgeRouting::Splines`] — smooth B-spline curves, PlantUML's upstream
///   default; opt-in in PUML via `skinparam linetype splines`. Long sweeping
///   arcs for distant endpoints, gentle near-straights for short ones.
///   Source: `splines=true` (no directive emitted by upstream `DotStringFactory`).
/// - [`EdgeRouting::Polyline`] — straight line segments through the routed
///   waypoints, no smoothing. Source: `splines=polyline`.
/// - [`EdgeRouting::Ortho`] — pure orthogonal right-angle elbows. Source:
///   `splines=ortho`. The only mode we shipped pre-Stage-2.
///
/// See `docs/internal/architecture/edge-routing.md` for the user-facing guide
/// and `docs/internal/architecture/edge-curve-research-2026-05-29.md` for the
/// upstream Java references.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum EdgeRouting {
    /// Smooth B-spline curves — opt-in via `skinparam linetype splines`.
    Splines,
    /// Straight line segments through waypoints — PUML default.
    #[default]
    Polyline,
    /// Orthogonal right-angle elbows.
    Ortho,
}

impl EdgeRouting {
    /// Parse a `skinparam linetype` value. Accepts case-insensitive
    /// `splines`, `polyline`, and `ortho`. Returns `None` for any other token.
    pub fn parse_linetype(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "splines" | "spline" | "curve" | "curved" => Some(Self::Splines),
            "polyline" | "poly" | "straight" => Some(Self::Polyline),
            "ortho" | "orthogonal" => Some(Self::Ortho),
            _ => None,
        }
    }

    /// Return the value as it would appear in upstream PlantUML source.
    pub const fn as_skinparam_value(self) -> &'static str {
        match self {
            Self::Splines => "splines",
            Self::Polyline => "polyline",
            Self::Ortho => "ortho",
        }
    }
}

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
