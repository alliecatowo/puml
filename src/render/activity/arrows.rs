/// Compute the center X for a fork branch column.
///
/// Branches are laid out symmetrically around `fork_cx`.
/// With N branches and column width `col_w`:
///   total span = (N-1) * col_w
///   leftmost branch center = fork_cx - (N-1)*col_w/2
///   branch k center = leftmost + k * col_w
pub(super) fn fork_branch_cx(
    fork_cx: i32,
    branch_idx: usize,
    n_branches: usize,
    col_w: i32,
) -> i32 {
    if n_branches <= 1 {
        return fork_cx;
    }
    let total_span = (n_branches as i32 - 1) * col_w;
    let leftmost = fork_cx - total_span / 2;
    leftmost + branch_idx as i32 * col_w
}

/// Node bounding box used for obstacle-avoidance in L-bend arrow routing (#734).
/// All coordinates are SVG pixels: (left, top, right, bottom).
#[derive(Clone, Copy)]
pub(super) struct NodeBbox {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ActivityArrowStyle {
    pub color: Option<String>,
    pub label: Option<String>,
    pub dashed: bool,
    pub hidden: bool,
    pub bold: bool,
    pub no_head: bool,
}

/// Return true when a horizontal line at `y` would pass through `bbox` in the
/// x corridor `[x_min, x_max]`.  A small margin prevents treating edge-touching
/// as a collision.
fn bbox_blocks_horiz(bbox: &NodeBbox, x_min: i32, x_max: i32, y: i32) -> bool {
    let margin = 3;
    let x_lo = x_min.min(x_max) + margin;
    let x_hi = x_min.max(x_max) - margin;
    if bbox.right <= x_lo || bbox.left >= x_hi {
        return false;
    }
    y > bbox.top + margin && y < bbox.bottom - margin
}

/// Return true when a vertical line at `x` would pass through `bbox` in the
/// y corridor `[y_min, y_max]`.
fn bbox_blocks_vert(bbox: &NodeBbox, x: i32, y_min: i32, y_max: i32) -> bool {
    let margin = 3;
    // x must be inside the bbox horizontally (with margin)
    if x <= bbox.left + margin || x >= bbox.right - margin {
        return false;
    }
    // The vertical segment's y-range must overlap the bbox's interior
    let seg_lo = y_min.min(y_max);
    let seg_hi = y_min.max(y_max);
    seg_hi > bbox.top + margin && seg_lo < bbox.bottom - margin
}

/// For a vertical arrow at x=`x` from `y1` to `y2`, find an x-offset that
/// avoids all blocking bboxes.  Returns `None` when the arrow is already clear.
/// The side x is placed to the right of all obstacles, with a small gap.
/// Check if any node bbox blocks a vertical segment at `x` between `y1` and
/// `y2`, excluding bboxes that the arrow STARTS from (i.e., y1 is inside the
/// bbox — that is the legitimate exit point of the source node).
fn choose_vert_bypass_x(x: i32, y1: i32, y2: i32, bboxes: &[NodeBbox]) -> Option<i32> {
    let y_lo = y1.min(y2);
    let y_hi = y1.max(y2);
    let blocking: Vec<&NodeBbox> = bboxes
        .iter()
        .filter(|b| {
            if !bbox_blocks_vert(b, x, y_lo, y_hi) {
                return false;
            }
            // Exclude the source node: if y1 is inside this bbox, the arrow
            // legitimately exits from it — not an obstacle.
            let y_start_inside = y1 >= b.top && y1 <= b.bottom;
            // Exclude the destination node: if y2 is inside this bbox, the
            // arrow legitimately arrives at it.
            let y_end_inside = y2 >= b.top && y2 <= b.bottom;
            !y_start_inside && !y_end_inside
        })
        .collect();
    if blocking.is_empty() {
        return None;
    }
    // Route to the right of all blocking bboxes.
    let side_x = blocking.iter().map(|b| b.right).max().unwrap() + 12;
    Some(side_x)
}

/// Choose an obstacle-free `mid_y` for the horizontal segment of an L-bend
/// arrow from (x1,y1) to (x2,y2).
///
/// Strategy (in order):
///   1. Try the naive midpoint `(y1 + y2) / 2`.
///   2. Try just above each conflicting box (`box.top - 4`) and just below
///      each conflicting box (`box.bottom + 4`), ranked by distance from the
///      naive midpoint.
///   3. Fall back to the naive midpoint if no clear slot is found.
fn choose_mid_y(x1: i32, y1: i32, x2: i32, y2: i32, bboxes: &[NodeBbox]) -> i32 {
    let x_lo = x1.min(x2);
    let x_hi = x1.max(x2);

    // Bboxes whose x range overlaps the corridor between x1 and x2.
    let obstacles: Vec<&NodeBbox> = bboxes
        .iter()
        .filter(|b| !(b.right <= x_lo || b.left >= x_hi))
        .collect();

    let is_clear = |y: i32| -> bool {
        obstacles
            .iter()
            .all(|b| !bbox_blocks_horiz(b, x_lo, x_hi, y))
    };

    // 1. Naive midpoint.
    let naive = y1 + (y2 - y1) / 2;
    if obstacles.is_empty() || is_clear(naive) {
        return naive;
    }

    // 2. Candidates: just above/below every obstacle, restricted to [lo, hi].
    let lo = y1.min(y2);
    let hi = y1.max(y2);
    let mut candidates: Vec<i32> = obstacles
        .iter()
        .flat_map(|b| [b.top - 4, b.bottom + 4])
        .filter(|&y| y >= lo && y <= hi)
        .collect();
    candidates.sort_unstable_by_key(|&y| (y - naive).abs());
    candidates.dedup();
    for y in candidates {
        if is_clear(y) {
            return y;
        }
    }

    // 3. Fall back to naive.
    naive
}

/// Emit an orthogonal (L-shaped / elbow) arrow from (x1,y1) to (x2,y2).
///
/// When x1 == x2 the arrow is drawn as a straight vertical line.
/// Otherwise an L-bend is used:
///
///   1. Vertical:    x1, y1    -> x1, mid_y
///   2. Horizontal:  x1, mid_y -> x2, mid_y
///   3. Vertical:    x2, mid_y -> x2, y2
///
/// `mid_y` is chosen by `choose_mid_y` to avoid crossing any node bbox that
/// lies in the x corridor between x1 and x2, fixing through-node routing (#734).
pub(crate) fn emit_activity_arrow(
    out: &mut String,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: &str,
    bboxes: &[NodeBbox],
) {
    emit_activity_arrow_with_style(
        out,
        x1,
        y1,
        x2,
        y2,
        color,
        &ActivityArrowStyle::default(),
        bboxes,
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn emit_activity_arrow_with_style(
    out: &mut String,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    default_color: &str,
    style: &ActivityArrowStyle,
    bboxes: &[NodeBbox],
) {
    if style.hidden {
        return;
    }
    let color = style.color.as_deref().unwrap_or(default_color);
    let stroke_width = if style.bold { "2.5" } else { "1.5" };
    let dash = if style.dashed {
        " stroke-dasharray=\"6 4\""
    } else {
        ""
    };
    let label_pos: (i32, i32);
    if x1 == x2 {
        // Vertical arrow.  Check whether it passes through any node bbox;
        // if so, route as a 5-segment bypass: out → up/down → back (#734).
        if let Some(side_x) = choose_vert_bypass_x(x1, y1, y2, bboxes) {
            // 5-segment path: (x1,y1) → (side_x,y1) → (side_x,y2) → (x2,y2)
            // implemented as 3 line segments with the arrowhead at (x2,y2).
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                x1, y1, side_x, y1, color, stroke_width, dash
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                side_x, y1, side_x, y2, color, stroke_width, dash
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                side_x, y2, x2, y2, color, stroke_width, dash
            ));
            label_pos = (side_x + 6, y1 + (y2 - y1) / 2);
        } else {
            // Straight vertical arrow -- no routing needed.
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                x1, y1, x2, y2, color, stroke_width, dash
            ));
            label_pos = (x1 + 6, y1 + (y2 - y1) / 2);
        }
        // Arrowhead pointing downward (or upward for back-edges).
        if !style.no_head {
            let uy = if y2 >= y1 { 1.0f64 } else { -1.0f64 };
            let base_y = y2 as f64 - uy * 8.0;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
                x2,
                y2,
                x2 - 4,
                base_y.round() as i32,
                x2 + 4,
                base_y.round() as i32,
                color
            ));
        }
    } else {
        // L-shaped orthogonal routing: down -> across -> down.
        // mid_y is chosen to avoid obstacle node bboxes in the x corridor.
        let mid_y = choose_mid_y(x1, y1, x2, y2, bboxes);

        // Check if the first vertical segment (x1, y1 → x1, mid_y) passes
        // through any node body.  If so, reroute as a 5-segment "bypass"
        // path: go right past all obstacles at y1, then vertical, then back
        // left to x1, then continue with the horizontal and final leg.
        if let Some(bypass_x) = choose_vert_bypass_x(x1, y1, mid_y, bboxes) {
            // 5-segment: (x1,y1)→(bypass,y1)→(bypass,mid_y)→(x2,mid_y)→(x2,y2)
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                x1, y1, bypass_x, y1, color, stroke_width, dash
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                bypass_x, y1, bypass_x, mid_y, color, stroke_width, dash
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                bypass_x, mid_y, x2, mid_y, color, stroke_width, dash
            ));
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                x2, mid_y, x2, y2, color, stroke_width, dash
            ));
        } else {
            // Normal 3-segment L-bend.
            // Segment 1: x1, y1 -> x1, mid_y
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                x1, y1, x1, mid_y, color, stroke_width, dash
            ));
            // Segment 2: x1, mid_y -> x2, mid_y
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                x1, mid_y, x2, mid_y, color, stroke_width, dash
            ));
            // Segment 3: x2, mid_y -> x2, y2
            out.push_str(&format!(
                "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"{}\"{}/>",
                x2, mid_y, x2, y2, color, stroke_width, dash
            ));
        }
        label_pos = (x1 + (x2 - x1) / 2 + 4, mid_y - 4);
        // Arrowhead at (x2, y2) pointing vertically (downward or upward).
        if !style.no_head {
            let dy = y2 - mid_y;
            let uy = if dy >= 0 { 1.0f64 } else { -1.0f64 };
            let base_y = y2 as f64 - uy * 8.0;
            out.push_str(&format!(
                "<polygon points=\"{},{} {},{} {},{}\" fill=\"{}\"/>",
                x2,
                y2,
                x2 - 4,
                base_y.round() as i32,
                x2 + 4,
                base_y.round() as i32,
                color
            ));
        }
    }
    if let Some(label) = &style.label {
        let (label_x, label_y) = label_pos;
        out.push_str(&format!(
            "<text x=\"{}\" y=\"{}\" font-family=\"monospace\" font-size=\"10\" fill=\"{}\">{}</text>",
            label_x,
            label_y,
            color,
            crate::render::svg::escape_text(label)
        ));
    }
}

/// Emit extra arrows stored as (x1, y1, x2, y2) tuples.
///
/// Only those arrows whose destination matches `(dst_cx, dst_y)` are emitted.
pub(super) fn emit_extra_arrows(
    out: &mut String,
    extra_arrows: &[super::layout::ActivityRoute],
    dst_cx: i32,
    dst_y: i32,
    color: &str,
    bboxes: &[NodeBbox],
) {
    for route in extra_arrows
        .iter()
        .filter(|route| route.x2 == dst_cx && route.y2 == dst_y)
    {
        emit_activity_arrow_with_style(
            out,
            route.x1,
            route.y1,
            route.x2,
            route.y2,
            color,
            &route.style,
            bboxes,
        );
    }
}

/// Emit direct arrows (fork-bar→branch, branch→join-bar).
pub(super) fn emit_direct_arrows(
    out: &mut String,
    direct_arrows: &[super::layout::ActivityRoute],
    color: &str,
    bboxes: &[NodeBbox],
) {
    for route in direct_arrows {
        emit_activity_arrow_with_style(
            out,
            route.x1,
            route.y1,
            route.x2,
            route.y2,
            color,
            &route.style,
            bboxes,
        );
    }
}
