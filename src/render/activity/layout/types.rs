pub(in crate::render::activity) struct NodeLayout {
    pub cx: i32,
    pub slot_y: i32,
    pub arrow_out_y: i32,
    pub next_slot_y: i32,
}

// ---------------------------------------------------------------------------
// Layout pass 1 output bundle
// ---------------------------------------------------------------------------

pub(in crate::render::activity) struct LayoutResult {
    pub node_layouts: Vec<NodeLayout>,
    pub fork_bar_half_widths: std::collections::BTreeMap<usize, i32>,
    /// Extra arrows used for control-flow branch and merge routes.
    pub extra_arrows: Vec<ActivityRoute>,
    /// Direct arrows: fork-bar→branch, branch→join-bar.
    pub direct_arrows: Vec<ActivityRoute>,
    /// Node indices for which the standard prev→cur arrow is suppressed
    pub suppress_prev_arrow: std::collections::BTreeSet<usize>,
}

#[derive(Clone, Debug)]
pub(in crate::render::activity) struct ActivityRoute {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub style: ActivityArrowStyle,
    /// When `true` this arrow is a loop back-edge that is rendered in SVG via a
    /// multi-segment detoured path (bypassing node bboxes). The typed scene
    /// builder skips back-edge routes so that straight-line approximations do
    /// not produce spurious `EdgeCrossesNode` geometry violations.
    pub skip_in_scene: bool,
}

impl ActivityRoute {
    pub(in crate::render::activity) fn new(x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self {
            x1,
            y1,
            x2,
            y2,
            style: ActivityArrowStyle::default(),
            skip_in_scene: false,
        }
    }

    pub(in crate::render::activity) fn with_label(mut self, label: Option<String>) -> Self {
        self.style.label = label.filter(|label| !label.trim().is_empty());
        self
    }

    pub(in crate::render::activity) fn skip_in_scene(mut self) -> Self {
        self.skip_in_scene = true;
        self
    }
}

pub(in crate::render::activity) struct LayoutParams<'a> {
    pub header_h: i32,
    pub lane_header_h: i32,
    pub step_h: i32,
    pub branch_x_offset: i32,
    pub fork_col_w: i32,
    pub lane_w: i32,
    /// Left edge of the lane area within the canvas. Fork bars must not extend
    /// left of this coordinate (used to clamp bar_span_half; #1299).
    pub lane_area_x: i32,
    pub lane_center_x: &'a dyn Fn(&str) -> i32,
    /// Minimum column width per branch: box_w + inter-node gap so adjacent
    /// fork-branch boxes never overlap each other.
    pub min_fork_col_w: i32,
}
use super::super::arrows::ActivityArrowStyle;
