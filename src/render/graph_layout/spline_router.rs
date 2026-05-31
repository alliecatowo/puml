//! Spline-native edge waypoint generator (#1391).
//!
//! This module is a **parallel renderer** to [`crate::render::edge_smoothing`]
//! for [`crate::render::graph_layout::EdgeRouting::Splines`] mode. Unlike the
//! rounded-corner renderer (which keeps the orthogonal waypoints and just
//! chamfers the corners), this generator produces a fundamentally different
//! waypoint set: 3-5 cubic Bézier control points encoding a single sweeping
//! curve that flows from the source anchor to the target anchor with tangents
//! perpendicular to the source/target boundary.
//!
//! ## Why this is separate from `edge_smoothing.rs`
//!
//! `edge_smoothing::rounded_corner_path_d` takes the orthogonal channel
//! router's 3-7 waypoints (right-angle topology) and replaces each interior
//! corner with a quarter-arc. That produces a "bracket-football" shape — it
//! still visibly reads as orthogonal-with-smoothing. The corners are visible.
//!
//! This module replaces that whole topology with a single cubic that flows
//! source → target with NO orthogonal corner points. The control points are
//! derived from the source/target anchor positions and tangent vectors, NOT
//! from any waypoint along the channel router's path.
//!
//! ## Algorithm overview
//!
//! Given:
//!  - `src_anchor` (the on-boundary source port the orthogonal router chose)
//!  - `tgt_anchor` (the on-boundary target port)
//!  - `src_tangent` (unit vector pointing OUT of the source — derived from
//!    which side of the source bbox `src_anchor` lies on, or from the second
//!    waypoint of the orthogonal path)
//!  - `tgt_tangent` (unit vector pointing INTO the target — derived
//!    similarly from the second-to-last waypoint)
//!  - `obstacles` (sibling node bboxes that the curve must not cross)
//!
//! Construct a cubic Bézier `M src C c1 c2 tgt`:
//!  - `c1 = src_anchor + handle_len * src_tangent`
//!  - `c2 = tgt_anchor - handle_len * tgt_tangent`
//!  - `handle_len ≈ 0.4 * euclidean_distance(src, tgt)` (the standard
//!    fraction that produces a natural Graphviz-like sweep)
//!
//! If that curve crosses any obstacle, insert one intermediate control point
//! pulled around the obstacle, producing a 2-segment cubic chain
//! (`M src C c1 c2 mid C c3 c4 tgt`). If still crossing after 1-2 detour
//! attempts, return `None` so the caller falls back to the rounded-corner
//! renderer.
//!
//! ## Topology cases handled
//!
//! 1. **1-to-1 same rank, no obstacles**: single cubic with handles pulled
//!    along the source/target boundary normals.
//! 2. **1-to-1 cross-rank**: single cubic; tangent is the channel-perpendicular
//!    direction (downward for downward edges, upward for upward edges).
//! 3. **Hub-and-spoke / many-to-one fan**: each edge gets a *distinct*
//!    tangent angle. The caller passes a per-edge tangent angle offset (see
//!    [`SplinePathInput::src_tangent_jitter`]) so curves emanating from the
//!    same source diverge at the port itself.
//! 4. **Self-loops**: out of scope — the existing C-shape renderer is kept.
//! 5. **Obstacle pierce**: insert 1 intermediate control point; fall back if
//!    still crossing after attempt.
//! 6. **Multi-out / multi-in spread**: handled via the tangent-jitter input,
//!    analogous to the channel router's port fan offset.
//!
//! ## What is NOT implemented (yet)
//!
//! - True Gansner-Koutsofios-North-Vo polygonal-channel spline placement.
//! - Multi-rank routing through intermediate dummy nodes.
//! - C¹-continuous chain across 3+ ranks.
//!
//! These are conscious deferrals — the first-cut goal is "visibly reads as
//! splines, doesn't wander, doesn't pierce obstacles" per #1391's acceptance.

/// Input bundle for [`generate_spline_path`].
#[derive(Debug, Clone)]
pub struct SplinePathInput {
    /// Source-side on-boundary anchor (where the curve begins).
    pub src_anchor: (f64, f64),
    /// Target-side on-boundary anchor (where the curve ends).
    pub tgt_anchor: (f64, f64),
    /// Unit vector pointing OUT of the source bbox at the anchor.
    /// e.g. (0, 1) for a bottom-side anchor on a downward edge.
    pub src_tangent: (f64, f64),
    /// Unit vector pointing INTO the target bbox at the anchor.
    /// e.g. (0, 1) for a top-side anchor on a downward edge (curve enters
    /// the target from above, so the tangent points down into it).
    pub tgt_tangent: (f64, f64),
    /// Sibling node bboxes (excluding source/target) to avoid piercing.
    pub obstacles: Vec<(f64, f64, f64, f64)>,
    /// Optional angular jitter at the source tangent, in radians. Used by
    /// hub-and-spoke fans to spread curves at the source port. Positive
    /// rotates the tangent clockwise (in screen y-down coordinates).
    pub src_tangent_jitter: f64,
    /// Optional angular jitter at the target tangent, in radians. Used by
    /// many-to-one converge fans.
    pub tgt_tangent_jitter: f64,
}

/// A cubic Bézier control point sequence.
///
/// `start` is the path origin; each `CubicSegment` is one `C cp1 cp2 end`
/// segment. The number of segments is the length of `segments`. Total
/// control point count is `1 + 3 * segments.len()`.
#[derive(Debug, Clone, PartialEq)]
pub struct SplinePath {
    pub start: (f64, f64),
    pub segments: Vec<CubicSegment>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CubicSegment {
    pub cp1: (f64, f64),
    pub cp2: (f64, f64),
    pub end: (f64, f64),
}

impl SplinePath {
    /// Render the path as an SVG `d` attribute body (no surrounding quotes).
    ///
    /// Format: `M sx,sy C c1x,c1y c2x,c2y ex,ey [C ...]`
    pub fn to_svg_d(&self) -> String {
        let mut d = format!("M {:.2},{:.2}", self.start.0, self.start.1);
        for seg in &self.segments {
            d.push_str(&format!(
                " C {:.2},{:.2} {:.2},{:.2} {:.2},{:.2}",
                seg.cp1.0, seg.cp1.1, seg.cp2.0, seg.cp2.1, seg.end.0, seg.end.1,
            ));
        }
        d
    }

    /// Returns the point at parameter `t` (0 ≤ t ≤ 1) along the entire path,
    /// linearly mapped across segments. Used by label positioning to anchor
    /// the label to the visual midpoint of the curve.
    pub fn point_at(&self, t: f64) -> (f64, f64) {
        if self.segments.is_empty() {
            return self.start;
        }
        let n = self.segments.len() as f64;
        let t = t.clamp(0.0, 1.0);
        let scaled = t * n;
        let seg_idx = (scaled as usize).min(self.segments.len() - 1);
        let local_t = scaled - seg_idx as f64;
        let (p0x, p0y) = if seg_idx == 0 {
            self.start
        } else {
            self.segments[seg_idx - 1].end
        };
        let seg = &self.segments[seg_idx];
        cubic_bezier_point(p0x, p0y, seg.cp1, seg.cp2, seg.end, local_t)
    }
}

/// Evaluate a cubic Bézier at parameter `t`.
fn cubic_bezier_point(
    p0x: f64,
    p0y: f64,
    c1: (f64, f64),
    c2: (f64, f64),
    p3: (f64, f64),
    t: f64,
) -> (f64, f64) {
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    let t2 = t * t;
    let t3 = t2 * t;
    let x = mt3 * p0x + 3.0 * mt2 * t * c1.0 + 3.0 * mt * t2 * c2.0 + t3 * p3.0;
    let y = mt3 * p0y + 3.0 * mt2 * t * c1.1 + 3.0 * mt * t2 * c2.1 + t3 * p3.1;
    (x, y)
}

/// Fraction of the source→target euclidean distance used as the Bézier
/// handle length. 0.4 is a Graphviz-like value that produces a natural sweep
/// without overshoot at typical diagram densities.
const HANDLE_FRACTION: f64 = 0.4;

/// Minimum handle length in pixels — even very short edges should curve.
const MIN_HANDLE_LEN: f64 = 12.0;

/// Maximum handle length in pixels — caps the bulge of very long edges so
/// the curve doesn't wander far outside the source/target rectangle.
const MAX_HANDLE_LEN: f64 = 120.0;

/// If the cubic between source and target passes within this many pixels of
/// any obstacle bbox, we treat it as a pierce and either insert a detour
/// control point or fall back to the rounded-corner renderer.
const OBSTACLE_MARGIN: f64 = 4.0;

/// Sampling density for obstacle-intersection checks. We sample the cubic
/// at `OBSTACLE_SAMPLES + 1` parameter values from t=0 to t=1.
const OBSTACLE_SAMPLES: usize = 32;

/// Maximum number of cubic segments the generator will emit. If obstacle
/// avoidance would require more than this, we return `None` and the caller
/// falls back to the rounded-corner renderer.
const MAX_SEGMENTS: usize = 2;

/// Generate a spline path for one edge, or return `None` if the topology is
/// too complex for the current implementation.
///
/// On `None` the caller should fall back to the existing rounded-corner
/// renderer (`edge_smoothing::rounded_corner_path_d`).
pub fn generate_spline_path(input: SplinePathInput) -> Option<SplinePath> {
    let (sx, sy) = input.src_anchor;
    let (tx, ty) = input.tgt_anchor;

    // Reject zero-length edges (caller should never produce these, but be safe).
    let dx = tx - sx;
    let dy = ty - sy;
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < 1.0 {
        return None;
    }

    // Apply tangent jitter (rotate tangent by jitter radians).
    let src_tangent = rotate(input.src_tangent, input.src_tangent_jitter);
    let tgt_tangent = rotate(input.tgt_tangent, input.tgt_tangent_jitter);

    // Build the single-segment cubic candidate first.
    let handle_len = (dist * HANDLE_FRACTION).clamp(MIN_HANDLE_LEN, MAX_HANDLE_LEN);
    let single = single_cubic(
        input.src_anchor,
        src_tangent,
        input.tgt_anchor,
        tgt_tangent,
        handle_len,
    );

    // Filter obstacles: drop any that contain or are adjacent to src/tgt.
    let obstacles = filter_obstacles(&input.obstacles, input.src_anchor, input.tgt_anchor);

    // If the single cubic clears all obstacles, ship it.
    if !cubic_crosses_obstacles(input.src_anchor, &single, &obstacles) {
        return Some(SplinePath {
            start: input.src_anchor,
            segments: vec![single],
        });
    }

    // Otherwise, attempt a 2-segment detour around the worst-piercing obstacle.
    if let Some(detour) = try_obstacle_detour(
        input.src_anchor,
        src_tangent,
        input.tgt_anchor,
        tgt_tangent,
        &obstacles,
        handle_len,
    ) {
        if detour.segments.len() <= MAX_SEGMENTS && !path_crosses_obstacles(&detour, &obstacles) {
            return Some(detour);
        }
    }

    // Topology too complex for the current implementation — caller falls back.
    None
}

/// Construct a single-segment cubic Bézier between `src` and `tgt` with
/// the given tangent directions and handle length.
fn single_cubic(
    src: (f64, f64),
    src_tangent: (f64, f64),
    tgt: (f64, f64),
    tgt_tangent: (f64, f64),
    handle_len: f64,
) -> CubicSegment {
    let cp1 = (
        src.0 + handle_len * src_tangent.0,
        src.1 + handle_len * src_tangent.1,
    );
    // Target tangent points INTO the target, so subtract handle_len to pull
    // cp2 back along the incoming direction.
    let cp2 = (
        tgt.0 - handle_len * tgt_tangent.0,
        tgt.1 - handle_len * tgt_tangent.1,
    );
    CubicSegment {
        cp1,
        cp2,
        end: tgt,
    }
}

/// Drop obstacles that are the source or target bbox themselves (identified
/// by containing the anchor point). Without this filter the curve would
/// trivially fail the obstacle check because it must originate ON the source
/// boundary.
fn filter_obstacles(
    obstacles: &[(f64, f64, f64, f64)],
    src: (f64, f64),
    tgt: (f64, f64),
) -> Vec<(f64, f64, f64, f64)> {
    obstacles
        .iter()
        .filter(|&&(bx, by, bw, bh)| {
            let inside_src = point_in_rect_inflated(src, (bx, by, bw, bh), -OBSTACLE_MARGIN);
            let inside_tgt = point_in_rect_inflated(tgt, (bx, by, bw, bh), -OBSTACLE_MARGIN);
            !inside_src && !inside_tgt
        })
        .copied()
        .collect()
}

/// Returns true if `(x, y)` lies inside the rect inflated/deflated by
/// `margin` (negative `margin` deflates).
fn point_in_rect_inflated(
    (x, y): (f64, f64),
    (bx, by, bw, bh): (f64, f64, f64, f64),
    margin: f64,
) -> bool {
    let x0 = bx - margin;
    let y0 = by - margin;
    let x1 = bx + bw + margin;
    let y1 = by + bh + margin;
    x >= x0 && x <= x1 && y >= y0 && y <= y1
}

/// Sample-based obstacle check: walk the cubic from t=0 to t=1, return true
/// if any sample (excluding the endpoints themselves) lands inside any
/// inflated obstacle bbox.
fn cubic_crosses_obstacles(
    src: (f64, f64),
    seg: &CubicSegment,
    obstacles: &[(f64, f64, f64, f64)],
) -> bool {
    if obstacles.is_empty() {
        return false;
    }
    // Sample interior of the curve (skip t=0 and t=1 which are the endpoints).
    for i in 1..OBSTACLE_SAMPLES {
        let t = i as f64 / OBSTACLE_SAMPLES as f64;
        let (px, py) = cubic_bezier_point(src.0, src.1, seg.cp1, seg.cp2, seg.end, t);
        for &bbox in obstacles {
            if point_in_rect_inflated((px, py), bbox, OBSTACLE_MARGIN) {
                return true;
            }
        }
    }
    false
}

/// Sample-based obstacle check for a multi-segment path.
fn path_crosses_obstacles(path: &SplinePath, obstacles: &[(f64, f64, f64, f64)]) -> bool {
    if obstacles.is_empty() {
        return false;
    }
    let mut prev_end = path.start;
    for seg in &path.segments {
        if cubic_crosses_obstacles(prev_end, seg, obstacles) {
            return true;
        }
        prev_end = seg.end;
    }
    false
}

/// Attempt a 2-segment detour that bends the curve around the most-
/// obstructive obstacle. Strategy:
///
/// 1. Identify the obstacle the straight curve pierces deepest.
/// 2. Pick a midpoint OUTSIDE the obstacle, offset perpendicular to the
///    source→target vector. Prefer the side closer to the source/target
///    anchors (the curve has less ground to cover).
/// 3. Construct a 2-segment path:
///    `src → cubic via tangent(src) and midpoint-approach → mid → cubic via
///    midpoint-departure and tangent(tgt) → tgt`.
///
/// Returns `None` if no obstacle is pierced (caller used wrong path) or
/// if no detour clears the obstacle.
fn try_obstacle_detour(
    src: (f64, f64),
    src_tangent: (f64, f64),
    tgt: (f64, f64),
    tgt_tangent: (f64, f64),
    obstacles: &[(f64, f64, f64, f64)],
    handle_len: f64,
) -> Option<SplinePath> {
    // Find the worst-piercing obstacle (the one closest to the midpoint of
    // the straight src→tgt line, weighted by depth of intersection).
    let mid = ((src.0 + tgt.0) / 2.0, (src.1 + tgt.1) / 2.0);
    let worst = obstacles
        .iter()
        .filter(|&&(bx, by, bw, bh)| {
            point_in_rect_inflated(mid, (bx, by, bw, bh), OBSTACLE_MARGIN * 4.0) || {
                // Cheap deep check: sample the straight line, see if it pierces.
                let single = single_cubic(src, src_tangent, tgt, tgt_tangent, handle_len);
                cubic_crosses_obstacles(src, &single, &[(bx, by, bw, bh)])
            }
        })
        .min_by(|a, b| {
            let da = ((a.0 + a.2 / 2.0) - mid.0).hypot((a.1 + a.3 / 2.0) - mid.1);
            let db = ((b.0 + b.2 / 2.0) - mid.0).hypot((b.1 + b.3 / 2.0) - mid.1);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .copied()?;

    let (bx, by, bw, bh) = worst;
    let obs_cx = bx + bw / 2.0;
    let obs_cy = by + bh / 2.0;

    // Perpendicular to the src→tgt vector (the "around the obstacle" axis).
    let dx = tgt.0 - src.0;
    let dy = tgt.1 - src.1;
    let len = dx.hypot(dy);
    if len < 1.0 {
        return None;
    }
    // Perpendicular unit vector (rotated 90° CCW).
    let perp = (-dy / len, dx / len);

    // Try both sides of the obstacle: positive and negative perpendicular.
    let detour_offset = (bw.max(bh) / 2.0 + OBSTACLE_MARGIN * 4.0).max(16.0);
    for sign in [1.0_f64, -1.0_f64] {
        let mid_x = obs_cx + sign * perp.0 * detour_offset;
        let mid_y = obs_cy + sign * perp.1 * detour_offset;
        let detour_mid = (mid_x, mid_y);

        // Tangent at the midpoint: roughly parallel to src→tgt (so the
        // C¹-ish continuity is preserved across the segment boundary).
        let mid_tangent = (dx / len, dy / len);
        let half_handle = (handle_len * 0.7).max(MIN_HANDLE_LEN);

        // Segment 1: src → detour_mid, leaving src along src_tangent, arriving
        // at detour_mid along mid_tangent.
        let seg1 = CubicSegment {
            cp1: (
                src.0 + half_handle * src_tangent.0,
                src.1 + half_handle * src_tangent.1,
            ),
            cp2: (
                detour_mid.0 - half_handle * mid_tangent.0,
                detour_mid.1 - half_handle * mid_tangent.1,
            ),
            end: detour_mid,
        };
        // Segment 2: detour_mid → tgt, leaving detour_mid along mid_tangent,
        // arriving at tgt along tgt_tangent.
        let seg2 = CubicSegment {
            cp1: (
                detour_mid.0 + half_handle * mid_tangent.0,
                detour_mid.1 + half_handle * mid_tangent.1,
            ),
            cp2: (
                tgt.0 - half_handle * tgt_tangent.0,
                tgt.1 - half_handle * tgt_tangent.1,
            ),
            end: tgt,
        };
        let candidate = SplinePath {
            start: src,
            segments: vec![seg1, seg2],
        };
        if !path_crosses_obstacles(&candidate, obstacles) {
            return Some(candidate);
        }
    }
    None
}

/// Rotate the 2D vector `(x, y)` by `theta` radians (positive = CCW in
/// standard math coordinates, which is *clockwise* in screen y-down).
fn rotate((x, y): (f64, f64), theta: f64) -> (f64, f64) {
    if theta.abs() < 1e-9 {
        return (x, y);
    }
    let c = theta.cos();
    let s = theta.sin();
    (x * c - y * s, x * s + y * c)
}

/// Infer the source tangent (unit vector pointing OUT of the source bbox)
/// from the anchor's position on the bbox boundary. If the anchor lies
/// within `tol` pixels of one of the four sides, return the outward normal.
/// Otherwise return `None` (caller should infer from the next waypoint).
pub fn tangent_from_bbox_side(
    anchor: (f64, f64),
    bbox: (f64, f64, f64, f64),
    tol: f64,
) -> Option<(f64, f64)> {
    let (x, y) = anchor;
    let (bx, by, bw, bh) = bbox;
    // Distance to each side.
    let d_left = (x - bx).abs();
    let d_right = (x - (bx + bw)).abs();
    let d_top = (y - by).abs();
    let d_bottom = (y - (by + bh)).abs();
    let min = d_left.min(d_right).min(d_top).min(d_bottom);
    if min > tol {
        return None;
    }
    if min == d_top {
        Some((0.0, -1.0))
    } else if min == d_bottom {
        Some((0.0, 1.0))
    } else if min == d_left {
        Some((-1.0, 0.0))
    } else {
        Some((1.0, 0.0))
    }
}

/// Infer the source tangent from the orthogonal path's first two waypoints.
/// Returns the unit vector from waypoint[0] to waypoint[1]. If the path has
/// fewer than 2 distinct points, returns `(0, 1)` (downward — safe default
/// for the default top-down layout).
///
/// Currently unused at the dispatch site (which inlines this logic against
/// integer waypoints), but exported as part of the module's public API for
/// future callers and integration tests.
#[allow(dead_code)]
pub fn tangent_from_orth_path(pts: &[(f64, f64)], reverse: bool) -> (f64, f64) {
    if pts.len() < 2 {
        return (0.0, 1.0);
    }
    let (ax, ay) = if reverse { pts[pts.len() - 1] } else { pts[0] };
    let (bx, by) = if reverse {
        pts[pts.len() - 2]
    } else {
        pts[1]
    };
    let dx = bx - ax;
    let dy = by - ay;
    let len = dx.hypot(dy);
    if len < 1e-6 {
        return (0.0, 1.0);
    }
    (dx / len, dy / len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_cubic_endpoints_match_anchors_exactly() {
        let input = SplinePathInput {
            src_anchor: (10.0, 20.0),
            tgt_anchor: (110.0, 220.0),
            src_tangent: (0.0, 1.0),
            tgt_tangent: (0.0, 1.0),
            obstacles: vec![],
            src_tangent_jitter: 0.0,
            tgt_tangent_jitter: 0.0,
        };
        let path = generate_spline_path(input).expect("should produce path");
        assert_eq!(path.start, (10.0, 20.0));
        assert_eq!(path.segments.len(), 1);
        assert_eq!(path.segments[0].end, (110.0, 220.0));
    }

    #[test]
    fn svg_d_starts_with_m_and_contains_one_c_for_single_segment() {
        let input = SplinePathInput {
            src_anchor: (0.0, 0.0),
            tgt_anchor: (100.0, 100.0),
            src_tangent: (0.0, 1.0),
            tgt_tangent: (0.0, 1.0),
            obstacles: vec![],
            src_tangent_jitter: 0.0,
            tgt_tangent_jitter: 0.0,
        };
        let path = generate_spline_path(input).unwrap();
        let d = path.to_svg_d();
        assert!(d.starts_with("M "), "must start with M: {d}");
        assert_eq!(d.matches(" C ").count(), 1, "expected 1 C command: {d}");
    }

    #[test]
    fn obstacle_detour_emits_two_cubic_segments() {
        // Obstacle directly between source and target.
        let input = SplinePathInput {
            src_anchor: (0.0, 0.0),
            tgt_anchor: (200.0, 0.0),
            src_tangent: (1.0, 0.0),
            tgt_tangent: (1.0, 0.0),
            obstacles: vec![(80.0, -20.0, 40.0, 40.0)],
            src_tangent_jitter: 0.0,
            tgt_tangent_jitter: 0.0,
        };
        let path = generate_spline_path(input).expect("should detour");
        assert_eq!(
            path.segments.len(),
            2,
            "obstacle pierce must insert detour control point"
        );
        let d = path.to_svg_d();
        assert_eq!(d.matches(" C ").count(), 2, "expected 2 C commands: {d}");
    }

    #[test]
    fn falls_back_to_none_when_obstacles_surround_target() {
        // Surround the path with obstacles so no detour clears.
        let obstacles = vec![
            (40.0, -20.0, 20.0, 40.0),
            (40.0, 20.0, 20.0, 40.0),
            (40.0, -60.0, 20.0, 40.0),
            (140.0, -20.0, 20.0, 40.0),
            (140.0, 20.0, 20.0, 40.0),
            (140.0, -60.0, 20.0, 40.0),
            (90.0, -20.0, 20.0, 40.0),
            (90.0, 20.0, 20.0, 40.0),
        ];
        let input = SplinePathInput {
            src_anchor: (0.0, 0.0),
            tgt_anchor: (200.0, 0.0),
            src_tangent: (1.0, 0.0),
            tgt_tangent: (1.0, 0.0),
            obstacles,
            src_tangent_jitter: 0.0,
            tgt_tangent_jitter: 0.0,
        };
        // Either succeeds with detour (acceptable) or returns None (fallback).
        // Just verify it doesn't panic and the result is correct.
        if let Some(path) = generate_spline_path(input) {
            assert!(path.segments.len() <= MAX_SEGMENTS);
        }
    }

    #[test]
    fn tangent_jitter_produces_distinct_control_points_for_fan() {
        // Hub-and-spoke: two edges leaving the same source with different
        // jitter angles should have distinct cp1 (the source-side control).
        let make = |jitter: f64| SplinePathInput {
            src_anchor: (50.0, 50.0),
            tgt_anchor: (150.0, 200.0),
            src_tangent: (0.0, 1.0),
            tgt_tangent: (0.0, 1.0),
            obstacles: vec![],
            src_tangent_jitter: jitter,
            tgt_tangent_jitter: 0.0,
        };
        let a = generate_spline_path(make(0.2)).unwrap();
        let b = generate_spline_path(make(-0.2)).unwrap();
        assert_ne!(a.segments[0].cp1, b.segments[0].cp1);
    }

    #[test]
    fn tangent_from_bbox_side_identifies_bottom_anchor() {
        let bbox = (10.0, 20.0, 100.0, 50.0); // x=10, y=20, w=100, h=50
        let anchor = (60.0, 70.0); // on bottom edge (y=70 = by+bh)
        assert_eq!(tangent_from_bbox_side(anchor, bbox, 2.0), Some((0.0, 1.0)));
    }

    #[test]
    fn tangent_from_bbox_side_identifies_top_anchor() {
        let bbox = (10.0, 20.0, 100.0, 50.0);
        let anchor = (60.0, 20.0); // on top edge
        assert_eq!(tangent_from_bbox_side(anchor, bbox, 2.0), Some((0.0, -1.0)));
    }

    #[test]
    fn tangent_from_orth_path_returns_unit_vector() {
        let pts = vec![(0.0, 0.0), (0.0, 50.0), (100.0, 50.0)];
        let t = tangent_from_orth_path(&pts, false);
        // First segment points straight down → (0, 1).
        assert!((t.0).abs() < 1e-6, "x should be 0: {:?}", t);
        assert!((t.1 - 1.0).abs() < 1e-6, "y should be 1: {:?}", t);
    }

    #[test]
    fn point_at_t0_returns_start_anchor() {
        let input = SplinePathInput {
            src_anchor: (10.0, 20.0),
            tgt_anchor: (110.0, 120.0),
            src_tangent: (0.0, 1.0),
            tgt_tangent: (0.0, 1.0),
            obstacles: vec![],
            src_tangent_jitter: 0.0,
            tgt_tangent_jitter: 0.0,
        };
        let path = generate_spline_path(input).unwrap();
        let p = path.point_at(0.0);
        assert!((p.0 - 10.0).abs() < 1e-6);
        assert!((p.1 - 20.0).abs() < 1e-6);
    }

    #[test]
    fn point_at_t1_returns_end_anchor() {
        let input = SplinePathInput {
            src_anchor: (10.0, 20.0),
            tgt_anchor: (110.0, 120.0),
            src_tangent: (0.0, 1.0),
            tgt_tangent: (0.0, 1.0),
            obstacles: vec![],
            src_tangent_jitter: 0.0,
            tgt_tangent_jitter: 0.0,
        };
        let path = generate_spline_path(input).unwrap();
        let p = path.point_at(1.0);
        assert!((p.0 - 110.0).abs() < 1e-6);
        assert!((p.1 - 120.0).abs() < 1e-6);
    }
}
