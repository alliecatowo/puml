//! Shared renderer-to-SVG scene contract.
//!
//! Renderers can still keep specialized layout engines, but their primary
//! visual output should be describable with these primitives and mirrored in
//! SVG through canonical `puml-*` hooks.  The validator and tests use the same
//! geometry vocabulary instead of learning each renderer's private classes.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
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

    pub fn from_center(center: Point, size: Size) -> Self {
        Self {
            x: center.x - size.w / 2.0,
            y: center.y - size.h / 2.0,
            w: size.w,
            h: size.h,
        }
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

    pub fn inflated(self, amount: f64) -> Self {
        Self {
            x: self.x - amount,
            y: self.y - amount,
            w: self.w + amount * 2.0,
            h: self.h + amount * 2.0,
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

    pub fn overlaps(self, other: Rect, clearance: f64) -> bool {
        self.x < other.right() + clearance
            && self.right() + clearance > other.x
            && self.y < other.bottom() + clearance
            && self.bottom() + clearance > other.y
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

#[derive(Debug, Clone, PartialEq)]
pub struct VisualLabel {
    pub id: String,
    pub owner_id: String,
    pub kind: String,
    pub text: String,
    pub anchor: Point,
    pub estimated_bbox: Rect,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualContainer {
    pub id: String,
    pub kind: String,
    pub frame_bbox: Rect,
    pub header_bbox: Option<Rect>,
    pub member_ids: Vec<String>,
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
}

pub fn estimate_text_bbox(x: f64, y: f64, text: &str, font_size: f64, middle_anchor: bool) -> Rect {
    let width = text.chars().count() as f64 * font_size * 0.62;
    let height = font_size + 4.0;
    let left = if middle_anchor { x - width / 2.0 } else { x };
    Rect::new(left, y - height * 0.72, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_overlap_respects_clearance() {
        let a = Rect::new(10.0, 10.0, 20.0, 20.0);
        let b = Rect::new(32.0, 10.0, 20.0, 20.0);
        assert!(!a.overlaps(b, 1.0));
        assert!(a.overlaps(b, 3.0));
    }

    #[test]
    fn text_bbox_is_centered_for_middle_anchor() {
        let bbox = estimate_text_bbox(100.0, 50.0, "abcd", 10.0, true);
        assert!(bbox.x < 100.0);
        assert!(bbox.right() > 100.0);
        assert!(bbox.y < 50.0);
    }
}
