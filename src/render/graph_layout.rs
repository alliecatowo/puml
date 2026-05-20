//! Hierarchical (Sugiyama-style) graph layout for component, deployment, and C4 diagrams.
//!
//! Stage 3 of the layout engine refactor (#593, child of #590).
//!
//! Pipeline:
//!   1. Rank assignment   - longest-path topological sort; cycle detection via DFS
//!   2. Crossing minim.   - barycenter heuristic plus adjacent transpositions
//!   3. Coord assignment  - nodes left-to-right per rank, ranks top-to-bottom
//!   4. Group bounds      - bounding box over all children + padding
//!   5. Edge routing      - orthogonal channel-based routing

use std::collections::BTreeMap;

mod ordering;
mod placement;
mod rank;
mod routing;

use ordering::minimise_crossings;
use placement::{assign_coordinates, compute_group_bounds};
use rank::assign_ranks;
use routing::route_edges;

#[cfg(test)]
mod tests;

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
}

/// Result of the layout pass.
#[derive(Debug, Clone, Default)]
pub struct GraphLayout {
    /// Map node id -> (x, y) of top-left corner.
    pub node_positions: BTreeMap<String, (f64, f64)>,
    /// Map node id -> assigned rank (0 = top).
    /// Exposed for diagnostics and Stage 3 orthogonal routing.
    #[allow(dead_code)]
    pub node_ranks: BTreeMap<String, usize>,
    /// Map edge id -> orthogonal path points.
    #[allow(dead_code)]
    pub edge_paths: BTreeMap<String, Vec<(f64, f64)>>,
    /// Map group id -> bounding rect (x, y, w, h).
    pub group_bounds: BTreeMap<String, (f64, f64, f64, f64)>,
    /// Total canvas width.
    pub canvas_width: f64,
    /// Total canvas height.
    pub canvas_height: f64,
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
    /// Right-side margin added to the canvas width. Defaults to `canvas_margin`
    /// when `None`. Set explicitly to decouple a large top/left margin (which
    /// absorbs titles and package-label tabs) from the right-side gutter.
    pub canvas_right_margin: Option<f64>,
}

impl Default for LayoutOptions {
    fn default() -> Self {
        Self {
            rank_separation: 80.0,
            node_separation: 60.0,
            group_padding: 28.0,
            direction: Direction::TopDown,
            canvas_margin: 40.0,
            canvas_right_margin: None,
        }
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

    let (ranks, reversed_edges) = assign_ranks(nodes, edges);
    let rank_order = minimise_crossings(nodes, edges, &ranks, 12);
    let (node_positions, canvas_width, canvas_height) =
        assign_coordinates(nodes, &ranks, &rank_order, options);
    let group_bounds = compute_group_bounds(nodes, &node_positions, options);
    let edge_paths = route_edges(
        nodes,
        edges,
        &node_positions,
        &reversed_edges,
        &group_bounds,
    );

    GraphLayout {
        node_positions,
        node_ranks: ranks,
        edge_paths,
        group_bounds,
        canvas_width,
        canvas_height,
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
            .unwrap_or((200, 80));
        out.insert(id.clone(), (x as i32, y as i32, w, h));
    }
    out
}
