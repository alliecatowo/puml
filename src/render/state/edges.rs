use super::*;

pub(super) struct StateEdgeStyle<'a> {
    pub(super) stroke: &'a str,
    pub(super) sw: u32,
    pub(super) dash: &'a str,
    pub(super) hidden: &'a str,
    pub(super) dir: &'a str,
}

/// Emit an SVG `<path>` element that routes a state transition orthogonally
/// (L-shaped / Z-shaped elbow) rather than as a straight diagonal.
///
/// Routing rules (same logic as the activity renderer):
/// - Same X or same Y: emit a straight line segment.
/// - Otherwise: route via a symmetric mid-point bend
///   `(x1,y1) → (x1,mid_y) → (x2,mid_y) → (x2,y2)`.
///
/// The path carries the same SVG attributes (stroke, stroke-width, dash, hidden,
/// direction, data-* labels, marker-end) as the old `<line>` element.
// Style attrs are already grouped into `StateEdgeStyle`; the remaining args are
// the mandatory out-buffer, two name strings, and four coordinate scalars — there
// is no meaningful grouping that would reduce the count further without obfuscating
// the call sites.
#[allow(clippy::too_many_arguments)]
pub(super) fn emit_state_orthogonal_path(
    out: &mut String,
    from_name: &str,
    to_name: &str,
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    style: &StateEdgeStyle<'_>,
) {
    let d = state_orthogonal_path_data(x1, y1, x2, y2);
    out.push_str(&format!(
        "<path class=\"state-transition\" data-state-from=\"{}\" data-state-to=\"{}\" d=\"{}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"{}{}{} marker-end=\"url(#arrow)\"/>",
        escape_text(from_name),
        escape_text(to_name),
        d,
        style.stroke,
        style.sw,
        style.dash,
        style.hidden,
        style.dir
    ));
}

pub(super) fn state_orthogonal_path_data(x1: i32, y1: i32, x2: i32, y2: i32) -> String {
    if x1 == x2 || y1 == y2 {
        return format!("M {x1} {y1} L {x2} {y2}");
    }

    if y2 < y1 {
        let mid_x = state_upward_elbow_x(x1, x2);
        format!("M {x1} {y1} L {mid_x} {y1} L {mid_x} {y2} L {x2} {y2}")
    } else {
        let mid_y = y1 + (y2 - y1) / 2;
        format!("M {x1} {y1} L {x1} {mid_y} L {x2} {mid_y} L {x2} {y2}")
    }
}

pub(super) fn state_orthogonal_label_segment(
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
) -> (i32, i32, i32, i32) {
    if x1 == x2 || y1 == y2 {
        return (x1, y1, x2, y2);
    }

    if y2 < y1 {
        let mid_x = state_upward_elbow_x(x1, x2);
        longest_state_path_segment(&[(x1, y1), (mid_x, y1), (mid_x, y2), (x2, y2)])
    } else {
        let mid_y = y1 + (y2 - y1) / 2;
        longest_state_path_segment(&[(x1, y1), (x1, mid_y), (x2, mid_y), (x2, y2)])
    }
}

pub(crate) fn state_upward_elbow_x(x1: i32, x2: i32) -> i32 {
    let dx = x2 - x1;
    if dx.abs() <= 24 {
        x1 + dx / 2
    } else {
        x2 - dx.signum() * 12
    }
}

pub(super) fn longest_state_path_segment(points: &[(i32, i32); 4]) -> (i32, i32, i32, i32) {
    points
        .windows(2)
        .map(|segment| {
            let (sx, sy) = segment[0];
            let (ex, ey) = segment[1];
            let len_sq = (ex - sx).pow(2) + (ey - sy).pow(2);
            (len_sq, sx, sy, ex, ey)
        })
        .max_by_key(|(len_sq, _, _, _, _)| *len_sq)
        .map(|(_, sx, sy, ex, ey)| (sx, sy, ex, ey))
        .unwrap_or((points[0].0, points[0].1, points[3].0, points[3].1))
}

pub(super) fn parse_state_note_on_link_direction(direction: Option<&str>) -> Option<(&str, &str)> {
    let direction = direction?;
    let mut parts = direction.splitn(3, '|');
    if parts.next()? != "on-link" {
        return None;
    }
    let position = parts.next().unwrap_or("over");
    let target = parts.next()?;
    Some((position, target))
}

pub(super) fn emit_state_note_connector(
    out: &mut String,
    transition: &StateTransition,
    from_p: &PlacedNode,
    note_p: &PlacedNode,
    placed: &std::collections::BTreeMap<String, PlacedNode>,
    node_kinds: &std::collections::BTreeMap<&str, &StateNodeKind>,
    fallback_stroke: &str,
) {
    let stroke = escape_text(transition.line_color.as_deref().unwrap_or(fallback_stroke));
    let sw = transition.thickness.unwrap_or(1).clamp(1, 8);
    let (x1, y1) = if let Some((_, target)) =
        parse_state_note_on_link_direction(transition.direction.as_deref())
    {
        if let Some(target_p) = placed.get(target) {
            let (ex1, ey1, ex2, ey2) = edge_anchors_for_kinds(
                node_kinds.get(transition.from.as_str()).copied(),
                from_p,
                node_kinds.get(target).copied(),
                target_p,
            );
            ((ex1 + ex2) / 2, (ey1 + ey2) / 2)
        } else {
            (from_p.x + from_p.w / 2, from_p.y + from_p.h / 2)
        }
    } else {
        (from_p.x + from_p.w / 2, from_p.y + from_p.h / 2)
    };
    let (_, _, x2, y2) = edge_anchors(
        &PlacedNode {
            x: x1,
            y: y1,
            w: 1,
            h: 1,
        },
        note_p,
    );
    out.push_str(&format!(
        "<path class=\"state-note-connector\" data-state-from=\"{}\" data-state-to=\"{}\" d=\"M {} {} L {} {}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\" stroke-dasharray=\"5 3\"/>",
        escape_text(&transition.from),
        escape_text(&transition.to),
        x1,
        y1,
        x2,
        y2,
        stroke,
        sw
    ));
}

/// Offset a line segment by `d` pixels perpendicular to its direction (to the right).
/// Used to separate bidirectional parallel edges so both arrows are visible.
pub(super) fn offset_parallel_edge(
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    d: i32,
) -> (i32, i32, i32, i32) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0 {
        return (x1, y1, x2, y2);
    }
    // Perpendicular unit vector (rotated 90° clockwise): (dy, -dx) / |len|
    let len = (len_sq as f64).sqrt();
    let ox = ((dy as f64 / len) * d as f64).round() as i32;
    let oy = ((-dx as f64 / len) * d as f64).round() as i32;
    (x1 + ox, y1 + oy, x2 + ox, y2 + oy)
}

/// Compute the edge anchor points between two placed nodes.
pub(super) fn edge_anchors(from: &PlacedNode, to: &PlacedNode) -> (i32, i32, i32, i32) {
    let fcx = from.x + from.w / 2;
    let fcy = from.y + from.h / 2;
    let tcx = to.x + to.w / 2;
    let tcy = to.y + to.h / 2;

    let dx = tcx - fcx;
    let dy = tcy - fcy;

    // Use half-sizes for boundary computation
    let fhw = from.w / 2;
    let fhh = from.h / 2;
    let thw = to.w / 2;
    let thh = to.h / 2;

    if dx == 0 && dy == 0 {
        return (fcx, fcy, tcx, tcy);
    }

    // Determine exit/entry side based on dominant direction
    if dx.abs() >= dy.abs() {
        if dx >= 0 {
            (fcx + fhw, fcy, tcx - thw, tcy)
        } else {
            (fcx - fhw, fcy, tcx + thw, tcy)
        }
    } else if dy >= 0 {
        (fcx, fcy + fhh, tcx, tcy - thh)
    } else {
        (fcx, fcy - fhh, tcx, tcy + thh)
    }
}
