//! Shared renderer-to-SVG scene contract.
//!
//! Renderers can still keep specialized layout engines, but their primary
//! visual output should be describable with these primitives and mirrored in
//! SVG through canonical `puml-*` hooks. The validator and tests use the same
//! geometry vocabulary instead of learning each renderer's private classes.

use std::collections::HashSet;

const EPSILON: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    pub fn translated(self, dx: f64, dy: f64) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
        }
    }

    pub fn distance_to(self, other: Point) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    pub fn lerp(self, other: Point, t: f64) -> Self {
        Self {
            x: self.x + (other.x - self.x) * t,
            y: self.y + (other.y - self.y) * t,
        }
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub w: f64,
    pub h: f64,
}

impl Size {
    pub const fn new(w: f64, h: f64) -> Self {
        Self { w, h }
    }

    pub fn is_empty(self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }

    pub fn is_finite(self) -> bool {
        self.w.is_finite() && self.h.is_finite()
    }

    pub fn inflated(self, amount: f64) -> Self {
        Self {
            w: self.w + amount * 2.0,
            h: self.h + amount * 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }

    pub fn from_points(a: Point, b: Point) -> Self {
        let x = a.x.min(b.x);
        let y = a.y.min(b.y);
        Self {
            x,
            y,
            w: a.x.max(b.x) - x,
            h: a.y.max(b.y) - y,
        }
    }

    pub fn from_center(center: Point, size: Size) -> Self {
        Self {
            x: center.x - size.w / 2.0,
            y: center.y - size.h / 2.0,
            w: size.w,
            h: size.h,
        }
    }

    pub fn size(self) -> Size {
        Size::new(self.w, self.h)
    }

    pub fn right(self) -> f64 {
        self.x + self.w
    }

    pub fn bottom(self) -> f64 {
        self.y + self.h
    }

    pub fn center(self) -> Point {
        Point::new(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    pub fn top_left(self) -> Point {
        Point::new(self.x, self.y)
    }

    pub fn top_right(self) -> Point {
        Point::new(self.right(), self.y)
    }

    pub fn bottom_left(self) -> Point {
        Point::new(self.x, self.bottom())
    }

    pub fn bottom_right(self) -> Point {
        Point::new(self.right(), self.bottom())
    }

    pub fn area(self) -> f64 {
        self.w.max(0.0) * self.h.max(0.0)
    }

    pub fn is_empty(self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }

    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.w.is_finite() && self.h.is_finite()
    }

    pub fn translated(self, dx: f64, dy: f64) -> Self {
        Self {
            x: self.x + dx,
            y: self.y + dy,
            ..self
        }
    }

    pub fn inflated(self, amount: f64) -> Self {
        Self {
            x: self.x - amount,
            y: self.y - amount,
            w: self.w + amount * 2.0,
            h: self.h + amount * 2.0,
        }
    }

    pub fn union(self, other: Rect) -> Self {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Self::new(x, y, right - x, bottom - y)
    }

    pub fn intersection(self, other: Rect) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        if right >= x && bottom >= y {
            Some(Self::new(x, y, right - x, bottom - y))
        } else {
            None
        }
    }

    pub fn contains_point(self, point: Point, tolerance: f64) -> bool {
        point.x >= self.x - tolerance
            && point.x <= self.right() + tolerance
            && point.y >= self.y - tolerance
            && point.y <= self.bottom() + tolerance
    }

    pub fn contains_rect(self, other: Rect, tolerance: f64) -> bool {
        other.x >= self.x - tolerance
            && other.right() <= self.right() + tolerance
            && other.y >= self.y - tolerance
            && other.bottom() <= self.bottom() + tolerance
    }

    pub fn intersects(self, other: Rect) -> bool {
        self.x <= other.right()
            && self.right() >= other.x
            && self.y <= other.bottom()
            && self.bottom() >= other.y
    }

    pub fn overlaps(self, other: Rect, clearance: f64) -> bool {
        self.x < other.right() + clearance
            && self.right() + clearance > other.x
            && self.y < other.bottom() + clearance
            && self.bottom() + clearance > other.y
    }

    pub fn clearance_to(self, other: Rect) -> f64 {
        let dx = if self.right() < other.x {
            other.x - self.right()
        } else if other.right() < self.x {
            self.x - other.right()
        } else {
            0.0
        };
        let dy = if self.bottom() < other.y {
            other.y - self.bottom()
        } else if other.bottom() < self.y {
            self.y - other.bottom()
        } else {
            0.0
        };
        dx.hypot(dy)
    }

    pub fn has_clearance(self, other: Rect, clearance: f64) -> bool {
        self.clearance_to(other) >= clearance
    }

    pub fn anchor_towards(self, target: Point) -> Point {
        anchor_on_rect(self, target)
    }

    pub fn as_puml_bbox(self) -> String {
        format!("{:.1} {:.1} {:.1} {:.1}", self.x, self.y, self.w, self.h)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShapeKind {
    Rect,
    RoundRect,
    Ellipse,
    Circle,
    Diamond,
    Polygon,
    Path,
}

impl ShapeKind {
    pub fn bounds_for(&self, bbox: Rect) -> Rect {
        match self {
            Self::Circle => {
                let side = bbox.w.min(bbox.h);
                Rect::from_center(bbox.center(), Size::new(side, side))
            }
            _ => bbox,
        }
    }

    pub fn anchor_towards(&self, bbox: Rect, target: Point) -> Point {
        match self {
            Self::Ellipse => anchor_on_ellipse(bbox, target),
            Self::Circle => anchor_on_ellipse(self.bounds_for(bbox), target),
            Self::Diamond => anchor_on_diamond(bbox, target),
            Self::Rect | Self::RoundRect | Self::Polygon | Self::Path => {
                anchor_on_rect(bbox, target)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualNode {
    pub id: String,
    pub family: String,
    pub kind: String,
    pub shape: ShapeKind,
    pub bbox: Rect,
    pub label_ids: Vec<String>,
    pub parent_id: Option<String>,
}

impl VisualNode {
    pub fn bounds(&self) -> Rect {
        self.shape.bounds_for(self.bbox)
    }

    pub fn center(&self) -> Point {
        self.bounds().center()
    }

    pub fn anchor_towards(&self, target: Point) -> Point {
        self.shape.anchor_towards(self.bbox, target)
    }

    pub fn contains_point(&self, point: Point, tolerance: f64) -> bool {
        self.bounds().contains_point(point, tolerance)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub points: Vec<Point>,
    pub source_port: Option<Point>,
    pub target_port: Option<Point>,
    pub label_ids: Vec<String>,
    pub route_kind: String,
}

impl VisualEdge {
    pub fn route_bbox(&self) -> Option<Rect> {
        bounds_for_points(self.points.iter().copied())
    }

    pub fn endpoints(&self) -> Option<(Point, Point)> {
        Some((*self.points.first()?, *self.points.last()?))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualLabel {
    pub id: String,
    pub owner_id: String,
    pub kind: String,
    pub text: String,
    pub anchor: Point,
    pub estimated_bbox: Rect,
}

impl VisualLabel {
    pub fn contains_point(&self, point: Point, tolerance: f64) -> bool {
        self.estimated_bbox.contains_point(point, tolerance)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualContainer {
    pub id: String,
    pub kind: String,
    pub frame_bbox: Rect,
    pub header_bbox: Option<Rect>,
    pub member_ids: Vec<String>,
}

impl VisualContainer {
    pub fn bounds(&self) -> Rect {
        match self.header_bbox {
            Some(header) => self.frame_bbox.union(header),
            None => self.frame_bbox,
        }
    }

    pub fn contains_member_bbox(&self, bbox: Rect, tolerance: f64) -> bool {
        self.bounds().contains_rect(bbox, tolerance)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObstacleSet {
    pub obstacles: Vec<Rect>,
}

impl ObstacleSet {
    pub fn new() -> Self {
        Self {
            obstacles: Vec::new(),
        }
    }

    pub fn from_rects(obstacles: impl IntoIterator<Item = Rect>) -> Self {
        Self {
            obstacles: obstacles.into_iter().collect(),
        }
    }

    pub fn add(&mut self, obstacle: Rect) {
        self.obstacles.push(obstacle);
    }

    pub fn is_clear(&self, bbox: Rect, clearance: f64) -> bool {
        self.obstacles
            .iter()
            .all(|obstacle| bbox.has_clearance(*obstacle, clearance))
    }

    pub fn first_collision(&self, bbox: Rect, clearance: f64) -> Option<Rect> {
        self.obstacles
            .iter()
            .copied()
            .find(|obstacle| !bbox.has_clearance(*obstacle, clearance))
    }

    pub fn min_clearance(&self, bbox: Rect) -> Option<f64> {
        self.obstacles
            .iter()
            .map(|obstacle| bbox.clearance_to(*obstacle))
            .reduce(f64::min)
    }

    pub fn inflated(&self, amount: f64) -> Self {
        Self::from_rects(
            self.obstacles
                .iter()
                .map(|obstacle| obstacle.inflated(amount)),
        )
    }
}

impl Default for ObstacleSet {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RenderScene {
    pub family: String,
    pub viewbox: Rect,
    pub nodes: Vec<VisualNode>,
    pub edges: Vec<VisualEdge>,
    pub labels: Vec<VisualLabel>,
    pub containers: Vec<VisualContainer>,
    pub diagnostics: Vec<String>,
}

impl RenderScene {
    pub fn new(family: impl Into<String>, viewbox: Rect) -> Self {
        Self {
            family: family.into(),
            viewbox,
            nodes: Vec::new(),
            edges: Vec::new(),
            labels: Vec::new(),
            containers: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn visual_bounds(&self) -> Option<Rect> {
        let node_bounds = self.nodes.iter().map(VisualNode::bounds);
        let edge_bounds = self.edges.iter().filter_map(VisualEdge::route_bbox);
        let label_bounds = self.labels.iter().map(|label| label.estimated_bbox);
        let container_bounds = self.containers.iter().map(VisualContainer::bounds);
        rect_union_all(
            node_bounds
                .chain(edge_bounds)
                .chain(label_bounds)
                .chain(container_bounds),
        )
    }

    pub fn obstacles(&self) -> ObstacleSet {
        ObstacleSet::from_rects(
            self.nodes
                .iter()
                .map(VisualNode::bounds)
                .chain(self.containers.iter().map(VisualContainer::bounds)),
        )
    }

    pub fn validate(&self) -> Vec<SceneValidationIssue> {
        let mut issues = Vec::new();
        let mut node_ids = HashSet::new();
        let mut label_ids = HashSet::new();
        let mut container_ids = HashSet::new();

        validate_rect("scene", &self.family, "viewbox", self.viewbox, &mut issues);

        for node in &self.nodes {
            if !node_ids.insert(node.id.as_str()) {
                issues.push(SceneValidationIssue::new(
                    SceneValidationSeverity::Error,
                    node.id.clone(),
                    SceneValidationKind::DuplicateId,
                    "duplicate node id",
                ));
            }
            validate_rect("node", &node.id, "bbox", node.bbox, &mut issues);
            if !self.viewbox.contains_rect(node.bounds(), 0.0) {
                issues.push(SceneValidationIssue::new(
                    SceneValidationSeverity::Warning,
                    node.id.clone(),
                    SceneValidationKind::OutsideViewbox,
                    "node bounds are outside the viewbox",
                ));
            }
        }

        for edge in &self.edges {
            if edge.points.iter().any(|point| !point.is_finite()) {
                issues.push(SceneValidationIssue::new(
                    SceneValidationSeverity::Error,
                    edge.id.clone(),
                    SceneValidationKind::NonFiniteGeometry,
                    "edge route contains a non-finite point",
                ));
            }
            if edge.points.len() < 2 {
                issues.push(SceneValidationIssue::new(
                    SceneValidationSeverity::Warning,
                    edge.id.clone(),
                    SceneValidationKind::DegenerateGeometry,
                    "edge route has fewer than two points",
                ));
            }
            if !node_ids.contains(edge.from.as_str()) {
                issues.push(SceneValidationIssue::new(
                    SceneValidationSeverity::Error,
                    edge.id.clone(),
                    SceneValidationKind::MissingReference,
                    "edge source does not reference a known node",
                ));
            }
            if !node_ids.contains(edge.to.as_str()) {
                issues.push(SceneValidationIssue::new(
                    SceneValidationSeverity::Error,
                    edge.id.clone(),
                    SceneValidationKind::MissingReference,
                    "edge target does not reference a known node",
                ));
            }
        }

        for label in &self.labels {
            if !label_ids.insert(label.id.as_str()) {
                issues.push(SceneValidationIssue::new(
                    SceneValidationSeverity::Error,
                    label.id.clone(),
                    SceneValidationKind::DuplicateId,
                    "duplicate label id",
                ));
            }
            validate_rect(
                "label",
                &label.id,
                "estimated_bbox",
                label.estimated_bbox,
                &mut issues,
            );
            if !label.anchor.is_finite() {
                issues.push(SceneValidationIssue::new(
                    SceneValidationSeverity::Error,
                    label.id.clone(),
                    SceneValidationKind::NonFiniteGeometry,
                    "label anchor is non-finite",
                ));
            }
        }

        for container in &self.containers {
            if !container_ids.insert(container.id.as_str()) {
                issues.push(SceneValidationIssue::new(
                    SceneValidationSeverity::Error,
                    container.id.clone(),
                    SceneValidationKind::DuplicateId,
                    "duplicate container id",
                ));
            }
            validate_rect(
                "container",
                &container.id,
                "frame_bbox",
                container.frame_bbox,
                &mut issues,
            );
            if let Some(header_bbox) = container.header_bbox {
                validate_rect(
                    "container",
                    &container.id,
                    "header_bbox",
                    header_bbox,
                    &mut issues,
                );
            }
            for member_id in &container.member_ids {
                if !node_ids.contains(member_id.as_str())
                    && !container_ids.contains(member_id.as_str())
                {
                    issues.push(SceneValidationIssue::new(
                        SceneValidationSeverity::Warning,
                        container.id.clone(),
                        SceneValidationKind::MissingReference,
                        format!("container member '{member_id}' is not present in the scene"),
                    ));
                }
            }
        }

        issues
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneValidationSeverity {
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SceneValidationKind {
    DegenerateGeometry,
    DuplicateId,
    MissingReference,
    NonFiniteGeometry,
    OutsideViewbox,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SceneValidationIssue {
    pub severity: SceneValidationSeverity,
    pub subject_id: String,
    pub kind: SceneValidationKind,
    pub message: String,
}

impl SceneValidationIssue {
    pub fn new(
        severity: SceneValidationSeverity,
        subject_id: impl Into<String>,
        kind: SceneValidationKind,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            subject_id: subject_id.into(),
            kind,
            message: message.into(),
        }
    }
}

pub fn estimate_text_bbox(x: f64, y: f64, text: &str, font_size: f64, middle_anchor: bool) -> Rect {
    let width = text.chars().count() as f64 * font_size * 0.62;
    let height = font_size + 4.0;
    let left = if middle_anchor { x - width / 2.0 } else { x };
    Rect::new(left, y - height * 0.72, width, height)
}

pub fn bounds_for_points(points: impl IntoIterator<Item = Point>) -> Option<Rect> {
    let mut points = points.into_iter();
    let first = points.next()?;
    let mut bounds = Rect::new(first.x, first.y, 0.0, 0.0);
    for point in points {
        bounds = bounds.union(Rect::new(point.x, point.y, 0.0, 0.0));
    }
    Some(bounds)
}

pub fn rect_union_all(rects: impl IntoIterator<Item = Rect>) -> Option<Rect> {
    rects.into_iter().reduce(Rect::union)
}

fn validate_rect(
    subject_kind: &str,
    subject_id: &str,
    field: &str,
    rect: Rect,
    issues: &mut Vec<SceneValidationIssue>,
) {
    if !rect.is_finite() {
        issues.push(SceneValidationIssue::new(
            SceneValidationSeverity::Error,
            subject_id,
            SceneValidationKind::NonFiniteGeometry,
            format!("{subject_kind} {field} contains non-finite geometry"),
        ));
    } else if rect.is_empty() {
        issues.push(SceneValidationIssue::new(
            SceneValidationSeverity::Warning,
            subject_id,
            SceneValidationKind::DegenerateGeometry,
            format!("{subject_kind} {field} has non-positive size"),
        ));
    }
}

fn anchor_on_rect(bbox: Rect, target: Point) -> Point {
    let center = bbox.center();
    let dx = target.x - center.x;
    let dy = target.y - center.y;
    if dx.abs() < EPSILON && dy.abs() < EPSILON {
        return center;
    }

    let half_w = bbox.w / 2.0;
    let half_h = bbox.h / 2.0;
    if half_w <= 0.0 || half_h <= 0.0 {
        return center;
    }

    let scale_x = if dx.abs() > EPSILON {
        half_w / dx.abs()
    } else {
        f64::INFINITY
    };
    let scale_y = if dy.abs() > EPSILON {
        half_h / dy.abs()
    } else {
        f64::INFINITY
    };
    let scale = scale_x.min(scale_y);
    Point::new(
        (center.x + dx * scale).clamp(bbox.x, bbox.right()),
        (center.y + dy * scale).clamp(bbox.y, bbox.bottom()),
    )
}

fn anchor_on_ellipse(bbox: Rect, target: Point) -> Point {
    let center = bbox.center();
    let dx = target.x - center.x;
    let dy = target.y - center.y;
    if dx.abs() < EPSILON && dy.abs() < EPSILON {
        return center;
    }

    let rx = bbox.w / 2.0;
    let ry = bbox.h / 2.0;
    if rx <= 0.0 || ry <= 0.0 {
        return center;
    }

    let scale = 1.0 / ((dx / rx).powi(2) + (dy / ry).powi(2)).sqrt();
    Point::new(center.x + dx * scale, center.y + dy * scale)
}

fn anchor_on_diamond(bbox: Rect, target: Point) -> Point {
    let center = bbox.center();
    let dx = target.x - center.x;
    let dy = target.y - center.y;
    if dx.abs() < EPSILON && dy.abs() < EPSILON {
        return center;
    }

    let half_w = bbox.w / 2.0;
    let half_h = bbox.h / 2.0;
    if half_w <= 0.0 || half_h <= 0.0 {
        return center;
    }

    let scale = 1.0 / (dx.abs() / half_w + dy.abs() / half_h);
    Point::new(center.x + dx * scale, center.y + dy * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-6,
            "expected {actual} to be close to {expected}"
        );
    }

    #[test]
    fn rect_overlap_respects_clearance() {
        let a = Rect::new(10.0, 10.0, 20.0, 20.0);
        let b = Rect::new(32.0, 10.0, 20.0, 20.0);
        assert!(!a.overlaps(b, 1.0));
        assert!(a.overlaps(b, 3.0));
    }

    #[test]
    fn rect_translation_inflation_union_and_intersection() {
        let a = Rect::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!(a.translated(5.0, -10.0), Rect::new(15.0, 10.0, 30.0, 40.0));
        assert_eq!(a.inflated(2.0), Rect::new(8.0, 18.0, 34.0, 44.0));

        let b = Rect::new(30.0, 40.0, 20.0, 10.0);
        assert_eq!(a.union(b), Rect::new(10.0, 20.0, 40.0, 40.0));
        assert_eq!(a.intersection(b), Some(Rect::new(30.0, 40.0, 10.0, 10.0)));
    }

    #[test]
    fn rect_clearance_reports_axis_and_diagonal_gaps() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let horizontal = Rect::new(15.0, 2.0, 4.0, 4.0);
        let diagonal = Rect::new(13.0, 14.0, 4.0, 4.0);
        assert_close(a.clearance_to(horizontal), 5.0);
        assert_close(a.clearance_to(diagonal), 5.0);
        assert!(a.has_clearance(horizontal, 5.0));
        assert!(!a.has_clearance(horizontal, 6.0));
    }

    #[test]
    fn shape_anchors_follow_shape_boundaries() {
        let bbox = Rect::new(0.0, 0.0, 100.0, 50.0);
        assert_eq!(
            ShapeKind::Rect.anchor_towards(bbox, Point::new(200.0, 25.0)),
            Point::new(100.0, 25.0)
        );
        assert_eq!(
            ShapeKind::Diamond.anchor_towards(bbox, Point::new(200.0, 25.0)),
            Point::new(100.0, 25.0)
        );

        let ellipse_anchor = ShapeKind::Ellipse.anchor_towards(bbox, Point::new(50.0, 100.0));
        assert_close(ellipse_anchor.x, 50.0);
        assert_close(ellipse_anchor.y, 50.0);

        let diagonal_diamond = ShapeKind::Diamond.anchor_towards(bbox, Point::new(100.0, 50.0));
        assert_close(diagonal_diamond.x, 75.0);
        assert_close(diagonal_diamond.y, 37.5);
    }

    #[test]
    fn circle_bounds_are_centered_inside_non_square_bbox() {
        let bbox = Rect::new(0.0, 0.0, 100.0, 40.0);
        assert_eq!(
            ShapeKind::Circle.bounds_for(bbox),
            Rect::new(30.0, 0.0, 40.0, 40.0)
        );
    }

    #[test]
    fn obstacle_set_finds_collisions_and_clearance() {
        let obstacles = ObstacleSet::from_rects([Rect::new(0.0, 0.0, 10.0, 10.0)]);
        let candidate = Rect::new(15.0, 0.0, 5.0, 5.0);
        assert!(obstacles.is_clear(candidate, 5.0));
        assert!(!obstacles.is_clear(candidate, 6.0));
        assert_eq!(
            obstacles.first_collision(candidate, 6.0),
            Some(Rect::new(0.0, 0.0, 10.0, 10.0))
        );
        assert_eq!(obstacles.min_clearance(candidate), Some(5.0));
    }

    #[test]
    fn render_scene_validation_catches_reference_and_geometry_issues() {
        let scene = RenderScene {
            family: "test".to_string(),
            viewbox: Rect::new(0.0, 0.0, 100.0, 100.0),
            nodes: vec![VisualNode {
                id: "n1".to_string(),
                family: "test".to_string(),
                kind: "node".to_string(),
                shape: ShapeKind::Rect,
                bbox: Rect::new(10.0, 10.0, 20.0, 20.0),
                label_ids: Vec::new(),
                parent_id: None,
            }],
            edges: vec![VisualEdge {
                id: "e1".to_string(),
                from: "n1".to_string(),
                to: "missing".to_string(),
                points: vec![Point::new(20.0, 20.0)],
                source_port: None,
                target_port: None,
                label_ids: Vec::new(),
                route_kind: "direct".to_string(),
            }],
            labels: Vec::new(),
            containers: Vec::new(),
            diagnostics: Vec::new(),
        };

        let issues = scene.validate();
        assert!(issues
            .iter()
            .any(|issue| issue.kind == SceneValidationKind::MissingReference));
        assert!(issues
            .iter()
            .any(|issue| issue.kind == SceneValidationKind::DegenerateGeometry));
    }

    #[test]
    fn render_scene_visual_bounds_unions_all_visuals() {
        let mut scene = RenderScene::new("test", Rect::new(0.0, 0.0, 200.0, 200.0));
        scene.nodes.push(VisualNode {
            id: "n1".to_string(),
            family: "test".to_string(),
            kind: "node".to_string(),
            shape: ShapeKind::Rect,
            bbox: Rect::new(10.0, 10.0, 20.0, 20.0),
            label_ids: Vec::new(),
            parent_id: None,
        });
        scene.edges.push(VisualEdge {
            id: "e1".to_string(),
            from: "n1".to_string(),
            to: "n1".to_string(),
            points: vec![Point::new(150.0, 150.0), Point::new(160.0, 170.0)],
            source_port: None,
            target_port: None,
            label_ids: Vec::new(),
            route_kind: "loop".to_string(),
        });

        assert_eq!(
            scene.visual_bounds(),
            Some(Rect::new(10.0, 10.0, 150.0, 160.0))
        );
    }

    #[test]
    fn text_bbox_is_centered_for_middle_anchor() {
        let bbox = estimate_text_bbox(100.0, 50.0, "abcd", 10.0, true);
        assert!(bbox.x < 100.0);
        assert!(bbox.right() > 100.0);
        assert!(bbox.y < 50.0);
    }
}
