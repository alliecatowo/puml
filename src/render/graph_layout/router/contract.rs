use crate::render::graph_layout::{EdgeSpec, NodeSize};
use crate::render::layout_constants::{MAX_TRACKS, PKG_HEADER_ROUTING_CLEARANCE, TRACK_SPACING};
use crate::render_core::RouteChannel;
use std::collections::{BTreeMap, BTreeSet};

/// Global edge-routing mode, selected by `skinparam linetype <value>`.
///
/// All three modes share the same waypoints produced by the orthogonal channel
/// router. The mode only affects SVG emission, not node positions.
///
/// ## PlantUML compatibility note
///
/// The PlantUML 1.2025 Language Reference Guide documents **only `ortho`** as a
/// valid `skinparam linetype` value (§20.3, IE/chen crow's-feet workaround).
/// `splines` and `polyline` are recognized by the upstream Java parser but are
/// **NOT documented** to users. We support them for compatibility with diagrams
/// written against the Java implementation.
///
/// PUML's default is [`EdgeRouting::Polyline`]. PlantUML's default is `splines=true`
/// (Graphviz B-splines). These are intentionally different — see §5 of
/// `docs/internal/architecture/edge-routing.md` for the rationale.
///
/// ## Variants
///
/// - [`EdgeRouting::Polyline`] (PUML default) — `<polyline points="…"/>` straight
///   orthogonal segments through the channel-router waypoints. No smoothing. Maps
///   to upstream `splines=polyline`.
/// - [`EdgeRouting::Splines`] — `<path d="M … L … Q …"/>` rounded-corner path:
///   same waypoints as Polyline, with each interior corner replaced by an 8 px
///   quarter-arc (quadratic Bézier) chamfer. This is **NOT** a Graphviz B-spline;
///   it is a rounded-chamfer post-processor. Opt-in via `skinparam linetype splines`.
///   The former Catmull-Rom implementation that caused the #1334 regression has been
///   replaced (see `src/render/edge_smoothing.rs`).
/// - [`EdgeRouting::Ortho`] — identical to `Polyline` for most families. For the
///   chen-ie (ER/IE notation) family, switches angled crow's-feet to orthogonal
///   right-angle elbows (the only documented use case per PlantUML §20.3).
///
/// See `docs/internal/architecture/edge-routing.md` for the full guide,
/// per-family routing matrix, and non-goals.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum EdgeRouting {
    /// Rounded-corner path — same waypoints as Polyline, interior corners replaced
    /// by 8 px quarter-arc (quadratic Bézier) chamfers. Opt-in via
    /// `skinparam linetype splines`. NOT a Graphviz B-spline; see module doc.
    Splines,
    /// Straight orthogonal segments through channel-router waypoints — PUML default.
    /// Emits `<polyline points="…"/>`. Maps to upstream `splines=polyline`.
    #[default]
    Polyline,
    /// Orthogonal right-angle elbows. Identical to Polyline for most families;
    /// for chen-ie switches angled crow's-feet to right-angle elbows (PlantUML §20.3).
    /// The only `linetype` value documented in the PlantUML 1.2025 spec.
    Ortho,
}

impl EdgeRouting {
    /// Parse a `skinparam linetype` value. Accepts case-insensitive tokens;
    /// returns `None` (silent no-op) for any unrecognized token, matching
    /// upstream PlantUML's fallback behavior.
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
