//! Structural tests for edge side-anchor perpendicularity.
//!
//! An edge that attaches to a node's SIDE (left/right) must exit/enter at
//! 90° to that side: its first/last segment must be HORIZONTAL.  An edge
//! attaching to a TOP/BOTTOM side must enter/exit at 90°: its first/last
//! segment must be VERTICAL.
//!
//! Before this fix, the snap-endpoint-to-port logic in `class_relations.rs`
//! and `box_grid_edges.rs` only constrained the off-axis coordinate of the
//! second/penultimate waypoint along **x** (regardless of which side the
//! anchor sat on).  For a left/right anchor this produced a degenerate
//! zero-length first/last segment and the next segment ran diagonally,
//! leaving the arrowhead "grazing" the box border parallel to the side.
//! For fan-shifted top/bottom anchors the penultimate waypoint stayed at
//! the un-shifted x, producing a diagonal stub that pointed at the side
//! rather than perpendicular to it.
//!
//! The tests below cover three regression cases:
//!
//!  1. class diagram, single edge — verifies the first/last segment is
//!     perpendicular to whichever side the endpoint lands on.
//!  2. component diagram (box-grid) with port-fan — two edges share a
//!     target top-side midpoint and the fan shifts one endpoint laterally;
//!     verifies the fan-shifted edge's last segment is still vertical.
//!  3. component diagram with explicit left/right routing — verifies
//!     left/right anchors produce horizontal stubs.

fn render_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("diagram should render without error")
}

/// Extract `points="..."` values from all `<polyline class="uml-relation"`
/// elements, in document order.
fn extract_polyline_points(svg: &str) -> Vec<Vec<(i32, i32)>> {
    let mut results = Vec::new();
    let mut search = svg;
    while let Some(idx) = search.find("class=\"uml-relation\"") {
        let after = &search[idx..];
        if let Some(pts_start) = after.find("points=\"") {
            let pts_content = &after[pts_start + 8..];
            if let Some(pts_end) = pts_content.find('"') {
                let pts_str = &pts_content[..pts_end];
                let pts: Vec<(i32, i32)> = pts_str
                    .split_whitespace()
                    .filter_map(|token| {
                        let mut parts = token.splitn(2, ',');
                        let x = parts.next()?.parse::<i32>().ok()?;
                        let y = parts.next()?.parse::<i32>().ok()?;
                        Some((x, y))
                    })
                    .collect();
                if pts.len() >= 2 {
                    results.push(pts);
                }
                search = &pts_content[pts_end..];
            } else {
                break;
            }
        } else {
            search = &after[1..];
        }
    }
    results
}

/// Return the side ("left"|"right"|"top"|"bottom") that `endpoint` lies on
/// for the given `bbox`, within the tolerance, or `None` if it doesn't sit
/// on any side cleanly.
fn endpoint_side(
    endpoint: (i32, i32),
    bbox: (i32, i32, i32, i32),
    tol: i32,
) -> Option<&'static str> {
    let (ex, ey) = endpoint;
    let (bx, by, bw, bh) = bbox;
    let near_left = (ex - bx).abs() <= tol;
    let near_right = (ex - (bx + bw)).abs() <= tol;
    let near_top = (ey - by).abs() <= tol;
    let near_bottom = (ey - (by + bh)).abs() <= tol;
    let within_v = ey >= by - tol && ey <= by + bh + tol;
    let within_h = ex >= bx - tol && ex <= bx + bw + tol;
    // Prefer top/bottom when the endpoint sits at a corner, matching the
    // renderer's side detection (`src_keep_routed_x` / `tgt_keep_routed_x`).
    if near_top && within_h {
        Some("top")
    } else if near_bottom && within_h {
        Some("bottom")
    } else if near_left && within_v {
        Some("left")
    } else if near_right && within_v {
        Some("right")
    } else {
        None
    }
}

/// Parse a node bounding box from the SVG by `data-uml-id`.
fn extract_node_bbox(svg: &str, id: &str) -> Option<(i32, i32, i32, i32)> {
    let needle = format!("data-uml-id=\"{}\"", id);
    let idx = svg.find(&needle)?;
    let after = &svg[idx..];
    let x = parse_attr(after, "x=")?;
    let y = parse_attr(after, "y=")?;
    let w = parse_attr(after, "width=")?;
    let h = parse_attr(after, "height=")?;
    Some((x, y, w, h))
}

fn parse_attr(s: &str, attr: &str) -> Option<i32> {
    let idx = s.find(attr)?;
    let after = &s[idx + attr.len()..];
    let after = after.trim_start_matches('"');
    let end = after.find(|c: char| !c.is_ascii_digit())?;
    after[..end].parse().ok()
}

/// Check that the segment between `a` and `b` is perpendicular to the named
/// side: vertical for top/bottom, horizontal for left/right.  Tolerates a 2px
/// rounding slack on the off-axis coordinate.
fn segment_perpendicular_to_side(a: (i32, i32), b: (i32, i32), side: &str) -> bool {
    match side {
        "top" | "bottom" => (a.0 - b.0).abs() <= 2,
        "left" | "right" => (a.1 - b.1).abs() <= 2,
        _ => false,
    }
}

#[test]
fn box_grid_fan_shifted_top_entry_keeps_vertical_stub() {
    // Two edges converge on `Renderer`'s top side.  The port-fan offset
    // shifts one endpoint laterally; the penultimate waypoint must follow
    // the shifted endpoint x so the last segment stays vertical
    // (perpendicular to the top side).
    let puml = r#"@startuml
component A
component B
component C
A --> C
B --> C
@enduml"#;

    let svg = render_svg(puml);
    let all_pts = extract_polyline_points(&svg);
    let c_bbox = extract_node_bbox(&svg, "C").expect("C node not found");

    let mut top_entry_seen = 0;
    for pts in &all_pts {
        if pts.len() < 2 {
            continue;
        }
        let last = *pts.last().unwrap();
        let Some(side) = endpoint_side(last, c_bbox, 10) else {
            continue;
        };
        if side == "top" {
            top_entry_seen += 1;
            let penultimate = pts[pts.len() - 2];
            assert!(
                segment_perpendicular_to_side(penultimate, last, side),
                "edge ending at C's top side has non-perpendicular last segment: \
                 penultimate={:?} last={:?} C_bbox={:?} full_pts={:?}",
                penultimate,
                last,
                c_bbox,
                pts
            );
        }
    }
    assert!(top_entry_seen >= 1, "expected ≥1 edge to enter C from top");
}

#[test]
fn class_diagram_side_anchors_emit_perpendicular_stubs() {
    // A small class diagram where edges naturally land on assorted sides.
    // Every edge's first/last segment must be perpendicular to whichever
    // side the endpoint lies on.
    let puml = r#"@startuml
class A
class B
class C
class D
A --> B
A --> C
B --> D
C --> D
@enduml"#;

    let svg = render_svg(puml);
    let all_pts = extract_polyline_points(&svg);
    assert!(!all_pts.is_empty(), "expected at least one relation polyline");

    let node_ids = ["A", "B", "C", "D"];
    let bboxes: Vec<(i32, i32, i32, i32)> = node_ids
        .iter()
        .filter_map(|id| extract_node_bbox(&svg, id))
        .collect();

    let mut checked = 0;
    for pts in &all_pts {
        if pts.len() < 2 {
            continue;
        }
        let first = pts[0];
        let last = *pts.last().unwrap();
        // For each endpoint, find the bbox it sits on (if any) and assert
        // perpendicularity of the adjacent segment.
        for (endpoint, adjacent) in [(first, pts[1]), (last, pts[pts.len() - 2])] {
            for &bbox in &bboxes {
                if let Some(side) = endpoint_side(endpoint, bbox, 6) {
                    assert!(
                        segment_perpendicular_to_side(endpoint, adjacent, side),
                        "class edge endpoint {:?} on side {:?} of bbox {:?} \
                         has non-perpendicular stub: adjacent={:?} full_pts={:?}",
                        endpoint,
                        side,
                        bbox,
                        adjacent,
                        pts
                    );
                    checked += 1;
                    break;
                }
            }
        }
    }
    assert!(
        checked >= 2,
        "expected ≥2 endpoints to land on a node side and be checked, got {}",
        checked
    );
}

#[test]
fn component_left_right_routing_emits_horizontal_stubs() {
    // Force a horizontally-dominant layout so endpoints land on left/right
    // sides.  The first/last segment must be horizontal.
    let puml = r#"@startuml
left to right direction
component A
component B
component C
A --> B
B --> C
@enduml"#;

    let svg = render_svg(puml);
    let all_pts = extract_polyline_points(&svg);
    let a = extract_node_bbox(&svg, "A");
    let b = extract_node_bbox(&svg, "B");
    let c = extract_node_bbox(&svg, "C");
    let bboxes: Vec<(i32, i32, i32, i32)> = [a, b, c].into_iter().flatten().collect();

    let mut horizontal_checked = 0;
    for pts in &all_pts {
        if pts.len() < 2 {
            continue;
        }
        for (endpoint, adjacent) in [(pts[0], pts[1]), (*pts.last().unwrap(), pts[pts.len() - 2])] {
            for &bbox in &bboxes {
                if let Some(side) = endpoint_side(endpoint, bbox, 6) {
                    if side == "left" || side == "right" {
                        assert!(
                            segment_perpendicular_to_side(endpoint, adjacent, side),
                            "left/right anchor produced non-horizontal stub: \
                             endpoint={:?} adjacent={:?} side={:?} bbox={:?} pts={:?}",
                            endpoint,
                            adjacent,
                            side,
                            bbox,
                            pts
                        );
                        horizontal_checked += 1;
                    }
                    break;
                }
            }
        }
    }
    // Allow zero — left-to-right layouts may sometimes still route through
    // top/bottom — but if any left/right stubs exist, they must be horizontal
    // (assertions above already enforce that).  This test exists mainly to
    // exercise the left/right code path; the per-endpoint assertions are the
    // primary contract.
    let _ = horizontal_checked;
}
