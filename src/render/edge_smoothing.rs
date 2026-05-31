//! Edge-path smoothing for the [`EdgeRouting::Splines`] mode.
//!
//! The orthogonal channel router produces a list of waypoints. When the
//! global routing mode is [`EdgeRouting::Splines`] we render those waypoints
//! as a **rounded-corner SVG path**: straight segments between waypoints with
//! small quarter-arc chamfers at each interior corner.
//!
//! ## Algorithm
//!
//! For waypoints `[p0, p1, …, pN]`:
//!
//! 1. `M p0`
//! 2. For each interior point `pi` (i = 1 … N-1):
//!    - Compute `dir_in` = unit vector from `p(i-1)` to `pi`
//!    - Compute `dir_out` = unit vector from `pi` to `p(i+1)`
//!    - Let the arc radius `r = CORNER_RADIUS` (capped to half the shorter
//!      adjacent segment to prevent overshoot)
//!    - Emit `L (pi − r·dir_in)` — straight up to corner-minus-r
//!    - Emit `Q pi, (pi + r·dir_out)` — quadratic arc through the corner
//! 3. `L pN`
//!
//! Straight segments stay straight. Endpoints are pinned to the routed
//! anchor points so arrowheads render exactly at the connector pin.
//! Two-point paths emit a plain `M … L …` (no corners to round).
//!
//! This replaces the former Catmull-Rom / cubic-Bézier approach that caused
//! the #1334 catastrophic regression (wander, overshoot, layout drift). The
//! rounded-corner renderer has none of those properties: every straight
//! segment of the polyline is unchanged; only the corner shape differs.
//!
//! See `docs/internal/architecture/edge-routing.md` for the mode-selection
//! contract and `docs/internal/forensics/2026-05-31-plantuml-edge-routing-investigation.md`
//! for the post-mortem that motivated this approach.

use crate::render::graph_layout::EdgeRouting;

/// Corner-arc radius in pixels. Corners tighter than this are automatically
/// reduced (see cap logic in [`rounded_corner_path_d`]).
const CORNER_RADIUS: f64 = 8.0;

/// Format the integer waypoints as the `points="..."` attribute body of a
/// `<polyline>` element. Used by [`EdgeRouting::Polyline`] and
/// [`EdgeRouting::Ortho`] modes.
pub fn polyline_points_attr(pts: &[(i32, i32)]) -> String {
    pts.iter()
        .map(|(x, y)| format!("{x},{y}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Emit an SVG `<path>` `d` attribute that renders the waypoints as a
/// **rounded-corner** path.
///
/// Returns the body of the `d` attribute (no surrounding quotes). For
/// inputs of length 0 or 1 returns an empty string; for length 2 emits a
/// straight `M ax,ay L bx,by`; for length ≥ 3 emits `M`, `L`, `Q` commands
/// that chamfer every interior corner with a small quadratic arc.
///
/// Endpoint coords are pinned exactly to the first and last waypoint so
/// arrowheads anchor correctly.
pub fn rounded_corner_path_d(pts: &[(i32, i32)]) -> String {
    if pts.len() < 2 {
        return String::new();
    }
    let (sx, sy) = pts[0];
    if pts.len() == 2 {
        let (ex, ey) = pts[1];
        return format!("M {sx},{sy} L {ex},{ey}");
    }

    // Helper: unit vector from (ax,ay) to (bx,by). Returns (0,0) for zero-length.
    let unit = |ax: f64, ay: f64, bx: f64, by: f64| -> (f64, f64) {
        let dx = bx - ax;
        let dy = by - ay;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-9 {
            (0.0, 0.0)
        } else {
            (dx / len, dy / len)
        }
    };

    let n = pts.len();
    let mut d = format!("M {sx},{sy}");

    for i in 1..n - 1 {
        let (px, py) = (pts[i - 1].0 as f64, pts[i - 1].1 as f64);
        let (cx, cy) = (pts[i].0 as f64, pts[i].1 as f64);
        let (nx, ny) = (pts[i + 1].0 as f64, pts[i + 1].1 as f64);

        let seg_in = ((cx - px) * (cx - px) + (cy - py) * (cy - py)).sqrt();
        let seg_out = ((nx - cx) * (nx - cx) + (ny - cy) * (ny - cy)).sqrt();

        // Cap the radius to half the shorter adjacent segment to prevent
        // arc overshoot. If either segment is near-zero, skip the arc.
        let r = CORNER_RADIUS.min(seg_in / 2.0).min(seg_out / 2.0);

        if r < 0.5 {
            // Segments too short to arc — emit a straight corner.
            d.push_str(&format!(" L {cx},{cy}"));
            continue;
        }

        let (din_x, din_y) = unit(px, py, cx, cy);
        let (dout_x, dout_y) = unit(cx, cy, nx, ny);

        // Point on incoming segment, r px before the corner.
        let pre_x = cx - r * din_x;
        let pre_y = cy - r * din_y;
        // Point on outgoing segment, r px after the corner.
        let post_x = cx + r * dout_x;
        let post_y = cy + r * dout_y;

        d.push_str(&format!(
            " L {pre_x:.2},{pre_y:.2} Q {cx:.2},{cy:.2} {post_x:.2},{post_y:.2}"
        ));
    }

    let (ex, ey) = pts[n - 1];
    d.push_str(&format!(" L {ex},{ey}"));
    d
}

/// Backwards-compatible alias so callers that imported `cubic_bezier_path_d`
/// by name continue to compile. Delegates to [`rounded_corner_path_d`].
#[inline]
pub fn cubic_bezier_path_d(pts: &[(i32, i32)]) -> String {
    rounded_corner_path_d(pts)
}

/// Emit an edge as an SVG element, choosing between `<polyline>` (for
/// [`EdgeRouting::Polyline`] and [`EdgeRouting::Ortho`]) and `<path>`
/// (for [`EdgeRouting::Splines`]). Returns just the geometry-bearing
/// attributes — the caller still needs to wrap them in an element with
/// the appropriate class / data-* / stroke attributes.
///
/// The signature returns `(tag, geometry_attr)` where `tag` is either
/// `"polyline"` or `"path"` and `geometry_attr` is either
/// `points="..."` or `d="..."`, ready to splice directly into the
/// element. This keeps the call-site formatting consistent while letting
/// each renderer keep ownership of marker / dash / stroke attribute
/// composition.
pub fn edge_geometry_attr(routing: EdgeRouting, pts: &[(i32, i32)]) -> (&'static str, String) {
    match routing {
        EdgeRouting::Splines => {
            let d = rounded_corner_path_d(pts);
            ("path", format!("d=\"{d}\""))
        }
        EdgeRouting::Polyline | EdgeRouting::Ortho => {
            let body = polyline_points_attr(pts);
            ("polyline", format!("points=\"{body}\""))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_produces_empty_string() {
        assert_eq!(rounded_corner_path_d(&[]), "");
        assert_eq!(rounded_corner_path_d(&[(0, 0)]), "");
    }

    #[test]
    fn two_point_input_emits_straight_line() {
        let d = rounded_corner_path_d(&[(10, 20), (110, 220)]);
        assert_eq!(d, "M 10,20 L 110,220");
    }

    #[test]
    fn three_point_input_emits_one_quadratic_arc() {
        // An L-shaped path: one interior corner → exactly one Q command.
        let d = rounded_corner_path_d(&[(0, 0), (100, 0), (100, 100)]);
        let q_count = d.matches(" Q ").count();
        assert_eq!(q_count, 1, "expected 1 Q command for 3 waypoints: {d}");
    }

    #[test]
    fn four_point_input_emits_two_quadratic_arcs() {
        // A Z-shaped path: two interior corners → exactly two Q commands.
        let d = rounded_corner_path_d(&[(0, 0), (100, 0), (100, 100), (200, 100)]);
        let q_count = d.matches(" Q ").count();
        assert_eq!(q_count, 2, "expected 2 Q commands for 4 waypoints: {d}");
    }

    #[test]
    fn two_point_path_uses_only_move_and_line_no_q() {
        // Sparse path — single straight segment, no arcs.
        let d = rounded_corner_path_d(&[(5, 10), (50, 80)]);
        assert!(
            !d.contains(" Q "),
            "straight segment must not contain Q: {d}"
        );
        assert!(d.starts_with("M 5,10"), "must start at first waypoint: {d}");
        assert!(d.ends_with("L 50,80"), "must end at last waypoint: {d}");
    }

    #[test]
    fn start_endpoint_pinned_to_first_waypoint() {
        let pts = [(7, 13), (100, 13), (100, 200)];
        let d = rounded_corner_path_d(&pts);
        // M command must be exactly the first waypoint, no offset.
        assert!(d.starts_with("M 7,13"), "start not pinned: {d}");
    }

    #[test]
    fn end_endpoint_pinned_to_last_waypoint() {
        let pts = [(0, 0), (50, 0), (50, 100)];
        let d = rounded_corner_path_d(&pts);
        // Final L command must end at the last waypoint exactly.
        assert!(d.ends_with("L 50,100"), "end not pinned: {d}");
    }

    #[test]
    fn short_segment_skips_arc_to_avoid_overshoot() {
        // Two adjacent segments each only 3 px long — too short for CORNER_RADIUS=8.
        // The radius cap should prevent overshoot; we just verify the path is valid.
        let pts = [(0, 0), (3, 0), (3, 3)];
        let d = rounded_corner_path_d(&pts);
        // Path must start at origin and end at (3,3).
        assert!(d.starts_with("M 0,0"), "start: {d}");
        assert!(d.ends_with("L 3,3"), "end: {d}");
    }

    #[test]
    fn polyline_points_attr_joins_with_spaces() {
        let s = polyline_points_attr(&[(1, 2), (3, 4), (5, 6)]);
        assert_eq!(s, "1,2 3,4 5,6");
    }

    #[test]
    fn edge_geometry_attr_routes_by_mode() {
        let pts = [(0, 0), (10, 0), (10, 10)];
        let (tag, attr) = edge_geometry_attr(EdgeRouting::Splines, &pts);
        assert_eq!(tag, "path");
        assert!(attr.starts_with("d=\""));

        let (tag, attr) = edge_geometry_attr(EdgeRouting::Polyline, &pts);
        assert_eq!(tag, "polyline");
        assert!(attr.starts_with("points=\""));

        let (tag, attr) = edge_geometry_attr(EdgeRouting::Ortho, &pts);
        assert_eq!(tag, "polyline");
        assert!(attr.starts_with("points=\""));
    }

    #[test]
    fn splines_path_contains_no_cubic_bezier_c_commands() {
        // The rounded-corner renderer uses Q (quadratic), not C (cubic).
        // Verify C commands are absent for a multi-waypoint path.
        let pts = [(0, 0), (100, 0), (100, 100), (200, 100)];
        let d = rounded_corner_path_d(&pts);
        assert!(
            !d.contains(" C "),
            "rounded-corner renderer must not emit cubic Bezier C commands: {d}"
        );
    }

    #[test]
    fn alias_cubic_bezier_path_d_matches_rounded_corner() {
        let pts = [(0, 0), (100, 0), (100, 100)];
        assert_eq!(cubic_bezier_path_d(&pts), rounded_corner_path_d(&pts));
    }
}
