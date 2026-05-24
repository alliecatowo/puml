/// Box geometry for a single class/node box used by `render_class_svg`.
#[derive(Clone, Copy)]
pub(super) struct ClassNodeBox {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
    pub(super) header_h: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ClassPortSide {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy)]
pub(super) struct ClassEndpointAnchor {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) side: ClassPortSide,
    pub(super) is_row_port: bool,
}

impl ClassEndpointAnchor {
    pub(super) fn point(self) -> (i32, i32) {
        (self.x, self.y)
    }
}

/// Output of `class_compute_canvas` — the canvas dimensions and node extents
/// needed to build the SVG header and position projections/labels.
pub(super) struct ClassCanvasMetrics {
    pub(super) svg_width: i32,
    pub(super) svg_height: i32,
    pub(super) nodes_bottom: i32,
}

pub(super) struct ClassNodeGeometry {
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) w: i32,
    pub(super) h: i32,
    pub(super) header_h: i32,
}
