//! Renderer-neutral geometry and pre-SVG scene contracts.
//!
//! This module is the first typed geometry slice for the renderer refactor. SVG
//! validation remains as a compatibility backstop while graph-family layout code
//! starts exposing inspectable geometry before serialization.

use std::collections::BTreeMap;

pub mod validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendFormat {
    Svg,
    Html,
    Png,
    Jpg,
    Webp,
    Pdf,
}

impl BackendFormat {
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Html => "html",
            Self::Png => "png",
            Self::Jpg => "jpg",
            Self::Webp => "webp",
            Self::Pdf => "pdf",
        }
    }

    pub const fn media_type(self) -> &'static str {
        match self {
            Self::Svg => "image/svg+xml",
            Self::Html => "text/html",
            Self::Png => "image/png",
            Self::Jpg => "image/jpeg",
            Self::Webp => "image/webp",
            Self::Pdf => "application/pdf",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneAvailability {
    TypedScene,
    NotMigrated,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendCapability {
    VectorOutput,
    HtmlExport,
    RasterExport,
    PdfExport,
    Metadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendDescriptor {
    pub id: &'static str,
    pub display_name: &'static str,
    pub primary_format: BackendFormat,
    pub export_formats: &'static [BackendFormat],
    pub capabilities: &'static [BackendCapability],
}

impl BackendDescriptor {
    pub fn supports_format(self, format: BackendFormat) -> bool {
        self.primary_format == format || self.export_formats.contains(&format)
    }

    pub fn has_capability(self, capability: BackendCapability) -> bool {
        self.capabilities.contains(&capability)
    }
}

pub trait RenderBackend {
    fn descriptor(&self) -> &'static BackendDescriptor;

    fn supports_format(&self, format: BackendFormat) -> bool {
        self.descriptor().supports_format(format)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SvgBackend;

pub static SVG_BACKEND_DESCRIPTOR: BackendDescriptor = BackendDescriptor {
    id: "svg",
    display_name: "SVG backend",
    primary_format: BackendFormat::Svg,
    export_formats: &[
        BackendFormat::Html,
        BackendFormat::Png,
        BackendFormat::Jpg,
        BackendFormat::Webp,
        BackendFormat::Pdf,
    ],
    capabilities: &[
        BackendCapability::VectorOutput,
        BackendCapability::HtmlExport,
        BackendCapability::RasterExport,
        BackendCapability::PdfExport,
        BackendCapability::Metadata,
    ],
};

impl RenderBackend for SvgBackend {
    fn descriptor(&self) -> &'static BackendDescriptor {
        &SVG_BACKEND_DESCRIPTOR
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn distance_to(self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

impl Size {
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    pub fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    pub fn min_x(self) -> f64 {
        self.origin.x
    }

    pub fn min_y(self) -> f64 {
        self.origin.y
    }

    pub fn max_x(self) -> f64 {
        self.origin.x + self.size.width
    }

    pub fn max_y(self) -> f64 {
        self.origin.y + self.size.height
    }

    pub fn center(self) -> Point {
        Point::new(
            self.min_x() + self.size.width / 2.0,
            self.min_y() + self.size.height / 2.0,
        )
    }

    pub fn contains_point(self, point: Point) -> bool {
        point.x >= self.min_x()
            && point.x <= self.max_x()
            && point.y >= self.min_y()
            && point.y <= self.max_y()
    }

    pub fn contains_rect(self, rect: Self) -> bool {
        self.contains_point(rect.origin)
            && self.contains_point(Point::new(rect.max_x(), rect.max_y()))
    }

    pub fn intersects(self, other: Self) -> bool {
        self.min_x() < other.max_x()
            && other.min_x() < self.max_x()
            && self.min_y() < other.max_y()
            && other.min_y() < self.max_y()
    }

    pub fn inset(self, insets: Insets) -> Self {
        Self::new(
            self.min_x() + insets.left,
            self.min_y() + insets.top,
            self.size.width - insets.left - insets.right,
            self.size.height - insets.top - insets.bottom,
        )
    }

    pub fn union(self, other: Self) -> Self {
        let min_x = self.min_x().min(other.min_x());
        let min_y = self.min_y().min(other.min_y());
        let max_x = self.max_x().max(other.max_x());
        let max_y = self.max_y().max(other.max_y());
        Self::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Insets {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

impl Insets {
    pub const fn new(top: f64, right: f64, bottom: f64, left: f64) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub const fn all(value: f64) -> Self {
        Self::new(value, value, value, value)
    }

    pub const fn symmetric(horizontal: f64, vertical: f64) -> Self {
        Self::new(vertical, horizontal, vertical, horizontal)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub start: Point,
    pub end: Point,
}

impl Segment {
    pub const fn new(start: Point, end: Point) -> Self {
        Self { start, end }
    }

    pub fn is_horizontal(self) -> bool {
        (self.start.y - self.end.y).abs() < f64::EPSILON
    }

    pub fn is_vertical(self) -> bool {
        (self.start.x - self.end.x).abs() < f64::EPSILON
    }

    pub fn length(self) -> f64 {
        self.start.distance_to(self.end)
    }

    pub fn bounds(self) -> Rect {
        let min_x = self.start.x.min(self.end.x);
        let min_y = self.start.y.min(self.end.y);
        let max_x = self.start.x.max(self.end.x);
        let max_y = self.start.y.max(self.end.y);
        Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Polyline {
    pub points: Vec<Point>,
}

impl Polyline {
    pub fn new(points: Vec<Point>) -> Self {
        Self { points }
    }

    pub fn from_tuples(points: &[(f64, f64)]) -> Self {
        Self::new(points.iter().map(|(x, y)| Point::new(*x, *y)).collect())
    }

    pub fn first(&self) -> Option<Point> {
        self.points.first().copied()
    }

    pub fn last(&self) -> Option<Point> {
        self.points.last().copied()
    }

    pub fn segments(&self) -> Vec<Segment> {
        self.points
            .windows(2)
            .map(|pair| Segment::new(pair[0], pair[1]))
            .collect()
    }

    pub fn bounds(&self) -> Option<Rect> {
        let mut points = self.points.iter();
        let first = points.next()?;
        let mut bounds = Rect::new(first.x, first.y, 0.0, 0.0);
        for point in points {
            bounds = bounds.union(Rect::new(point.x, point.y, 0.0, 0.0));
        }
        Some(bounds)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortSide {
    Top,
    Right,
    Bottom,
    Left,
    Center,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Port {
    pub id: String,
    pub node_id: String,
    pub side: PortSide,
    pub position: Point,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    pub id: String,
    pub owner_id: String,
    pub position: Point,
    pub port: Option<Port>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelRole {
    Node,
    Edge,
    Group,
    Lane,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LabelBox {
    pub id: String,
    pub text: String,
    pub bounds: Rect,
    pub owner_id: Option<String>,
    pub role: LabelRole,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeBox {
    pub id: String,
    pub bounds: Rect,
    pub ports: Vec<Port>,
    pub labels: Vec<LabelBox>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupFrame {
    pub id: String,
    pub bounds: Rect,
    pub header: Option<Rect>,
    pub child_node_ids: Vec<String>,
    pub labels: Vec<LabelBox>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LaneFrame {
    pub id: String,
    pub bounds: Rect,
    pub header: Option<Rect>,
    pub child_node_ids: Vec<String>,
    pub labels: Vec<LabelBox>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RouteChannel {
    pub id: String,
    pub bounds: Rect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneNode {
    pub id: String,
    pub node_box: NodeBox,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub route: Polyline,
    pub source_anchor: Anchor,
    pub target_anchor: Anchor,
    pub labels: Vec<LabelBox>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneGroup {
    pub id: String,
    pub frame: GroupFrame,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneLabel {
    pub id: String,
    pub label_box: LabelBox,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderScene {
    pub viewport: Rect,
    pub nodes: BTreeMap<String, SceneNode>,
    pub edges: BTreeMap<String, SceneEdge>,
    pub groups: BTreeMap<String, SceneGroup>,
    pub lanes: BTreeMap<String, LaneFrame>,
    pub labels: BTreeMap<String, SceneLabel>,
    pub route_channels: BTreeMap<String, RouteChannel>,
}

impl RenderScene {
    pub fn new(viewport: Rect) -> Self {
        Self {
            viewport,
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            groups: BTreeMap::new(),
            lanes: BTreeMap::new(),
            labels: BTreeMap::new(),
            route_channels: BTreeMap::new(),
        }
    }

    pub fn add_node(&mut self, node: SceneNode) {
        for label in &node.node_box.labels {
            self.add_label_box(label.clone());
        }
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn add_edge(&mut self, edge: SceneEdge) {
        for label in &edge.labels {
            self.add_label_box(label.clone());
        }
        self.edges.insert(edge.id.clone(), edge);
    }

    pub fn add_group(&mut self, group: SceneGroup) {
        for label in &group.frame.labels {
            self.add_label_box(label.clone());
        }
        self.groups.insert(group.id.clone(), group);
    }

    pub fn add_lane(&mut self, lane: LaneFrame) {
        for label in &lane.labels {
            self.add_label_box(label.clone());
        }
        self.lanes.insert(lane.id.clone(), lane);
    }

    pub fn add_label_box(&mut self, label_box: LabelBox) {
        let label = SceneLabel {
            id: label_box.id.clone(),
            label_box,
        };
        self.labels.insert(label.id.clone(), label);
    }

    pub fn visible_bounds(&self) -> Rect {
        let mut bounds = self.viewport;
        for node in self.nodes.values() {
            bounds = bounds.union(node.node_box.bounds);
        }
        for edge in self.edges.values() {
            if let Some(route_bounds) = edge.route.bounds() {
                bounds = bounds.union(route_bounds);
            }
        }
        for group in self.groups.values() {
            bounds = bounds.union(group.frame.bounds);
            if let Some(header) = group.frame.header {
                bounds = bounds.union(header);
            }
        }
        for lane in self.lanes.values() {
            bounds = bounds.union(lane.bounds);
            if let Some(header) = lane.header {
                bounds = bounds.union(header);
            }
        }
        for label in self.labels.values() {
            bounds = bounds.union(label.label_box.bounds);
        }
        for channel in self.route_channels.values() {
            bounds = bounds.union(channel.bounds);
        }
        bounds
    }

    pub fn fit_viewport_to_visible_bounds(&mut self) {
        self.viewport = self.visible_bounds();
    }

    pub fn validate_scene(&self) -> validate::SceneValidationReport {
        validate::validate_scene(self)
    }

    pub fn validate_geometry(&self) -> Vec<GeometryIssue> {
        self.validate_scene().issues
    }
}

impl Default for RenderScene {
    fn default() -> Self {
        Self::new(Rect::new(0.0, 0.0, 0.0, 0.0))
    }
}

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
}
