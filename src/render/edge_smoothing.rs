//! Edge-path smoothing for the [`EdgeRouting::Splines`] mode.
//!
//! The orthogonal channel router produces a list of waypoints. When the
//! global routing mode is [`EdgeRouting::Splines`] (the default, matching
//! upstream PlantUML's `splines=true` Graphviz directive) we smooth those
//! waypoints into a sequence of cubic Bézier segments using a
//! Catmull-Rom-style tangent construction.
//!
//! The Catmull-Rom variant we use is the standard centripetal scheme
//! adapted for SVG cubic Bézier emission: for each interior segment
//! `P_{i} → P_{i+1}` we build two control points from the chord vectors
//! `P_{i-1} → P_{i+1}` and `P_{i} → P_{i+2}`, scaled by a tension
//! coefficient. Endpoints are mirrored so the curve starts and ends
//! exactly at the routed endpoints (so arrowheads still anchor correctly).
//!
//! For two-point paths we emit a straight line — Catmull-Rom degenerates
//! to a line when there are no interior waypoints to curve around. Empty
//! and single-point inputs produce empty output (caller should fall back
//! to a `<line>` element in that case).
//!
//! See `docs/internal/architecture/edge-routing.md` for the mode-selection
//! contract and `docs/internal/architecture/edge-curve-research-2026-05-29.md`
//! for the upstream Java reference.

use crate::render::graph_layout::EdgeRouting;

/// Smoothing tension. Lower → tighter curves closer to the polyline;
/// higher → smoother arcs. `0.5` matches the visual feel of upstream
/// PlantUML's spline interpolator on the corpus samples Allie shared.
const SMOOTHING_TENSION: f64 = 0.5;

/// Format the integer waypoints as the `points="..."` attribute body of a
/// `<polyline>` element. Used by [`EdgeRouting::Polyline`] and
/// [`EdgeRouting::Ortho`] modes.
pub fn polyline_points_attr(pts: &[(i32, i32)]) -> String {
    pts.iter()
        .map(|(x, y)| format!("{x},{y}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Emit an SVG `<path>` `d` attribute that smoothly interpolates the
/// waypoints using Catmull-Rom-derived cubic Béziers.
///
/// Returns the body of the `d` attribute (no surrounding quotes). For
/// inputs of length 0 or 1 returns an empty string; for length 2 emits a
/// straight `M ax,ay L bx,by`; for length ≥ 3 emits a sequence of
/// `M`, then `C` commands. Each cubic Bézier segment shares its endpoint
/// with the next segment's start, so the curve is C1-continuous
/// everywhere except at corners with a sharp polyline angle, where the
/// smoothing gently rounds them.
pub fn cubic_bezier_path_d(pts: &[(i32, i32)]) -> String {
    if pts.len() < 2 {
        return String::new();
    }
    if pts.len() == 2 {
        let (ax, ay) = pts[0];
        let (bx, by) = pts[1];
        return format!("M {ax},{ay} L {bx},{by}");
    }

    // Build the "phantom" tangent points by mirroring the second/penultimate
    // points across the endpoints. This pins the curve to the routed
    // endpoints and lets us use the same Catmull-Rom formula for every
    // segment (no special-case at the boundaries).
    let mut ctrl: Vec<(f64, f64)> = Vec::with_capacity(pts.len() + 2);
    {
        let (x0, y0) = pts[0];
        let (x1, y1) = pts[1];
        ctrl.push(((2 * x0 - x1) as f64, (2 * y0 - y1) as f64));
    }
    for (x, y) in pts {
        ctrl.push((*x as f64, *y as f64));
    }
    {
        let n = pts.len();
        let (xn1, yn1) = pts[n - 1];
        let (xn2, yn2) = pts[n - 2];
        ctrl.push(((2 * xn1 - xn2) as f64, (2 * yn1 - yn2) as f64));
    }

    let (sx, sy) = pts[0];
    let mut d = format!("M {sx},{sy}");
    // Iterate over segments P_i → P_{i+1} (i ∈ [1, n]), using the phantom
    // points for tangents at the endpoints.
    for i in 1..(ctrl.len() - 2) {
        let p0 = ctrl[i - 1];
        let p1 = ctrl[i];
        let p2 = ctrl[i + 1];
        let p3 = ctrl[i + 2];

        // Standard Catmull-Rom → Bézier control points.
        // c1 = p1 + (p2 - p0) / 6 * tension * 2
        // c2 = p2 - (p3 - p1) / 6 * tension * 2
        let c1x = p1.0 + (p2.0 - p0.0) * SMOOTHING_TENSION / 3.0;
        let c1y = p1.1 + (p2.1 - p0.1) * SMOOTHING_TENSION / 3.0;
        let c2x = p2.0 - (p3.0 - p1.0) * SMOOTHING_TENSION / 3.0;
        let c2y = p2.1 - (p3.1 - p1.1) * SMOOTHING_TENSION / 3.0;
        d.push_str(&format!(
            " C {:.1},{:.1} {:.1},{:.1} {:.1},{:.1}",
            c1x, c1y, c2x, c2y, p2.0, p2.1
        ));
    }
    d
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
            let d = cubic_bezier_path_d(pts);
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
        assert_eq!(cubic_bezier_path_d(&[]), "");
        assert_eq!(cubic_bezier_path_d(&[(0, 0)]), "");
    }

    #[test]
    fn two_point_input_emits_straight_line() {
        let d = cubic_bezier_path_d(&[(10, 20), (110, 220)]);
        assert_eq!(d, "M 10,20 L 110,220");
    }

    #[test]
    fn three_point_input_emits_cubic_bezier() {
        let d = cubic_bezier_path_d(&[(0, 0), (100, 0), (100, 100)]);
        assert!(d.starts_with("M 0,0"));
        assert!(
            d.contains(" C "),
            "expected at least one cubic Bezier segment: {d}"
        );
        // Should end exactly on the last waypoint.
        assert!(d.ends_with("100.0,100.0"), "got: {d}");
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
    fn splines_curve_passes_near_all_waypoints() {
        // A simple L-shape: the smoothed curve should remain close to the
        // corner waypoint (within a few px) so the routed edge is visually
        // identifiable as following the same route.
        let pts = [(0, 0), (50, 0), (50, 50)];
        let d = cubic_bezier_path_d(&pts);
        // The first cubic segment ends at the corner waypoint (50,0):
        // (50,0) appears as the last triplet of the first " C " command.
        assert!(d.contains("50.0,0.0"), "curve must touch corner: {d}");
    }
}
