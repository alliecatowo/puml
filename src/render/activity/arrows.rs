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

/// Emit an orthogonal (L-shaped / elbow) arrow from (x1,y1) to (x2,y2).
///
/// When the source and destination share the same X coordinate the arrow is a
/// straight vertical line (the common case for sequential nodes).  When they
/// differ the path is routed as an L-bend:
///
///   1. Straight down from (x1, y1) to (x1, mid_y)    — vertical segment
///   2. Straight across from (x1, mid_y) to (x2, mid_y) — horizontal segment
///   3. Straight down from (x2, mid_y) to (x2, y2)    — vertical segment
///
/// `mid_y` is placed half-way between y1 and y2, giving a symmetric elbow.
/// This eliminates the diagonal arrows that would otherwise cross through
/// node bodies on multi-branch flows (#778).
pub(crate) fn emit_activity_arrow(
    out: &mut String,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: &str,
) {
    if x1 == x2 {
        // Straight vertical arrow — no routing needed.
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            x1, y1, x2, y2, color
        ));
        // Arrowhead pointing downward (or upward for back-edges).
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
    } else {
        // L-shaped orthogonal routing: down → across → down.
        // mid_y is half-way between y1 and y2 on both sides.
        let mid_y = y1 + (y2 - y1) / 2;
        // Segment 1: x1, y1 → x1, mid_y
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            x1, y1, x1, mid_y, color
        ));
        // Segment 2: x1, mid_y → x2, mid_y
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            x1, mid_y, x2, mid_y, color
        ));
        // Segment 3: x2, mid_y → x2, y2
        out.push_str(&format!(
            "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
            x2, mid_y, x2, y2, color
        ));
        // Arrowhead at (x2, y2) pointing vertically (downward or upward).
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

/// Emit all queued direct arrows (fork-bar→branch, branch→join-bar).
pub(super) fn emit_direct_arrows(
    out: &mut String,
    direct_arrows: &[(i32, i32, i32, i32)],
    color: &str,
) {
    for (x1, y1, x2, y2) in direct_arrows {
        emit_activity_arrow(out, *x1, *y1, *x2, *y2, color);
    }
}

/// Emit all redirected extra arrows (if-branch merge arrows).
pub(super) fn emit_extra_arrows(
    out: &mut String,
    extra_arrows: &[(i32, i32, i32, i32)],
    target_cx: i32,
    target_y: i32,
    color: &str,
) {
    for (x1, y1, x2, y2) in extra_arrows
        .iter()
        .filter(|a| a.2 == target_cx && a.3 == target_y)
    {
        emit_activity_arrow(out, *x1, *y1, *x2, *y2, color);
    }
}
