use super::{Point, Rect, Segment};

#[derive(Debug, Clone, PartialEq)]
pub enum GeometryIssue {
    NodeOutsideViewport {
        node_id: String,
        bounds: Rect,
        viewport: Rect,
    },
    GroupOutsideViewport {
        group_id: String,
        bounds: Rect,
        viewport: Rect,
    },
    LaneOutsideViewport {
        lane_id: String,
        bounds: Rect,
        viewport: Rect,
    },
    LabelOutsideViewport {
        label_id: String,
        bounds: Rect,
        viewport: Rect,
    },
    EdgeMissingRoute {
        edge_id: String,
    },
    EdgeEndpointDetached {
        edge_id: String,
        anchor_id: String,
        expected: Point,
        actual: Point,
    },
    EdgeCrossesNode {
        edge_id: String,
        node_id: String,
        segment: Segment,
        node_bounds: Rect,
    },
    EdgeCrossesGroupHeader {
        edge_id: String,
        group_id: String,
        segment: Segment,
        header_bounds: Rect,
    },
    EdgeReferencesMissingRouteChannel {
        edge_id: String,
        channel_id: String,
    },
    EdgeReferencesUnownedRouteChannel {
        edge_id: String,
        channel_id: String,
    },
    EdgeRouteOutsideChannel {
        edge_id: String,
        segment: Segment,
    },
    EdgeLabelDetached {
        edge_id: String,
        label_id: String,
        bounds: Rect,
        min_distance: f64,
        max_distance: f64,
    },
    EdgeAnchorOwnerMismatch {
        edge_id: String,
        anchor_id: String,
        expected_node_id: String,
        actual_owner_id: String,
    },
    EdgeEndpointMissingDeclaredPort {
        edge_id: String,
        anchor_id: String,
        node_id: String,
        position: Point,
    },
    EdgeAnchorPortMismatch {
        edge_id: String,
        anchor_id: String,
        port_id: String,
        expected: Point,
        actual: Point,
    },
    GroupChildOutsideFrame {
        group_id: String,
        node_id: String,
        node_bounds: Rect,
        group_bounds: Rect,
    },
    GroupChildOverlapsHeader {
        group_id: String,
        node_id: String,
        node_bounds: Rect,
        header_bounds: Rect,
    },
    LaneChildOutsideFrame {
        lane_id: String,
        node_id: String,
        node_bounds: Rect,
        lane_bounds: Rect,
    },
    LaneChildOverlapsHeader {
        lane_id: String,
        node_id: String,
        node_bounds: Rect,
        header_bounds: Rect,
    },
}
