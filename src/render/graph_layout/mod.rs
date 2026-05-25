//! Hierarchical (Sugiyama-style) graph layout for component, deployment, and C4 diagrams.
//!
//! Stage 3 of the layout engine refactor (#593, child of #590).
//!
//! Pipeline:
//!   1. Rank assignment   — longest-path topological sort; cycle detection via DFS
//!   2. Crossing minim.  — barycenter heuristic, up to 12 alternating sweeps
//!   3. Coord assignment  — simple: nodes left-to-right per rank, ranks top-to-bottom
//!   4. Group bounds      — bounding box over all children + padding
//!   5. Edge routing      — orthogonal channel-based routing (Stage 3)

mod coordinates;
mod crossing;
mod groups;
mod rank;
mod router;
mod scene;

#[cfg(test)]
mod tests;

use super::layout_constants::{
    COMPONENT_BOX_HEIGHT, COMPONENT_BOX_WIDTH, DEFAULT_CANVAS_MARGIN, DEFAULT_GROUP_PADDING,
    DEFAULT_NODE_SEPARATION, DEFAULT_RANK_SEPARATION,
};
use crate::render_core::RenderScene;
use crate::render_core::RouteChannel;
use std::collections::BTreeMap;

/// Size specification for one node.
#[derive(Debug, Clone)]
pub struct NodeSize {
    pub id: String,
    pub width: f64,
    pub height: f64,
    /// Optional parent group id; nodes with the same parent are enclosed together.
    pub parent: Option<String>,
}

/// One directed edge in the graph.
#[derive(Debug, Clone)]
pub struct EdgeSpec {
    pub id: String,
    pub from: String,
    pub to: String,
    pub label: Option<String>,
}

/// Result of the layout pass.
#[derive(Debug, Clone, Default)]
pub struct GraphLayout {
    /// Map node id → (x, y) of top-left corner.
    pub node_positions: BTreeMap<String, (f64, f64)>,
    /// Map node id → assigned rank (0 = top).
    /// Exposed for diagnostics and Stage 3 orthogonal routing.
    #[allow(dead_code)]
    pub node_ranks: BTreeMap<String, usize>,
    /// Map edge id → two-point path (straight line anchor points).
    /// Will be used by Stage 3 orthogonal routing.
    #[allow(dead_code)]
    pub edge_paths: BTreeMap<String, Vec<(f64, f64)>>,
    /// Deterministic channel bands used by routed edge tracks.
    #[allow(dead_code)]
    pub route_channels: BTreeMap<String, RouteChannel>,
    /// Map group id → bounding rect (x, y, w, h).
    pub group_bounds: BTreeMap<String, (f64, f64, f64, f64)>,
    /// Total canvas width.
    pub canvas_width: f64,
    /// Total canvas height.
    pub canvas_height: f64,
    /// Typed pre-SVG scene geometry for graph-family layout validation.
    #[allow(dead_code)]
    pub scene: RenderScene,
}

/// Layout direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    TopDown,
    /// Planned for Stage 3 (left-to-right flow for wide diagrams).
    #[allow(dead_code)]
    LeftRight,
}

/// Tuneable layout parameters.
#[derive(Debug, Clone)]
pub struct LayoutOptions {
    /// Vertical gap between ranks (pixels).
    pub rank_separation: f64,
    /// Horizontal gap between nodes in the same rank.
    pub node_separation: f64,
    /// Padding inside group containers.
    pub group_padding: f64,
    /// Flow direction (TopDown or LeftRight).
    #[allow(dead_code)]
    pub direction: Direction,
    /// Left/top margin around the full canvas (also used for node x-origin).
    pub canvas_margin: f64,
    /// Right-side margin added to the canvas width.  Defaults to `canvas_margin`
    /// when `None`.  Set explicitly to decouple a large top/left margin (which
    /// absorbs titles and package-label tabs) from the right-side gutter.
    pub canvas_right_margin: Option<f64>,
    /// Stack vertically staggered group collisions downward instead of shifting
    /// them sideways. Component diagrams use this to keep downstream packages
    /// out of lollipop/interface routing channels without changing deployment
    /// layouts that already have blessed artifacts.
    pub stack_staggered_group_collisions: bool,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            rank_separation: DEFAULT_RANK_SEPARATION,
            node_separation: DEFAULT_NODE_SEPARATION,
            group_padding: DEFAULT_GROUP_PADDING,
            direction: Direction::TopDown,
            canvas_margin: DEFAULT_CANVAS_MARGIN,
            canvas_right_margin: None,
            stack_staggered_group_collisions: false,
        }
    }
}

impl GraphLayout {
    /// Rebuild the typed scene after renderer-local route normalization.
    pub fn rebuild_scene(&mut self, nodes: &[NodeSize], edges: &[EdgeSpec]) {
        self.scene = scene::build_render_scene(scene::SceneBuildInput {
            nodes,
            edges,
            node_positions: &self.node_positions,
            edge_paths: &self.edge_paths,
            route_channels: &self.route_channels,
            group_bounds: &self.group_bounds,
            canvas_width: self.canvas_width,
            canvas_height: self.canvas_height,
        });
    }
}

/// Run the full Sugiyama-style hierarchical layout pipeline.
pub fn layout_hierarchical(
    nodes: &[NodeSize],
    edges: &[EdgeSpec],
    options: &LayoutOptions,
) -> GraphLayout {
    if nodes.is_empty() {
        return GraphLayout::default();
    }

    // Step 1 — Build adjacency + rank assignment
    let (ranks, reversed_edges) = rank::assign_ranks(nodes, edges);

    // Step 2 — Crossing minimisation (barycenter, 12 sweeps)
    let rank_order = crossing::minimise_crossings(nodes, edges, &ranks, 12);

    // Step 3 — Coordinate assignment
    let (node_positions, canvas_width, canvas_height) =
        coordinates::assign_coordinates(nodes, &ranks, &rank_order, options);

    // Step 4 — Group bounding boxes
    let group_bounds = groups::compute_group_bounds(nodes, &node_positions, options);

    // Step 5 — Orthogonal edge routing (Stage 3: channel-based)
    let routing = router::route_edges(
        nodes,
        edges,
        &node_positions,
        &reversed_edges,
        &group_bounds,
    );

    let scene = scene::build_render_scene(scene::SceneBuildInput {
        nodes,
        edges,
        node_positions: &node_positions,
        edge_paths: &routing.edge_paths,
        route_channels: &routing.route_channels,
        group_bounds: &group_bounds,
        canvas_width,
        canvas_height,
    });

    GraphLayout {
        node_positions,
        node_ranks: ranks,
        edge_paths: routing.edge_paths,
        route_channels: routing.route_channels,
        group_bounds,
        canvas_width,
        canvas_height,
        scene,
    }
}

/// Convert a GraphLayout position map from f64 to i32 tuples (x, y, w, h)
/// given the node size list for width/height lookup.
/// Used by Stage 3 (orthogonal routing integration).
#[allow(dead_code)]
pub fn layout_to_i32_positions(
    layout: &GraphLayout,
    nodes: &[NodeSize],
) -> BTreeMap<String, (i32, i32, i32, i32)> {
    let node_by_id: BTreeMap<&str, &NodeSize> = nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut out: BTreeMap<String, (i32, i32, i32, i32)> = BTreeMap::new();
    for (id, &(x, y)) in &layout.node_positions {
        let (w, h) = node_by_id
            .get(id.as_str())
            .map(|n| (n.width as i32, n.height as i32))
            .unwrap_or((COMPONENT_BOX_WIDTH, COMPONENT_BOX_HEIGHT));
        out.insert(id.clone(), (x as i32, y as i32, w, h));
    }
    out
}
