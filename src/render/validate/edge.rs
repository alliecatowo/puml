use std::cmp::Reverse;

use super::semantic::{extract_node_bboxes, NodeBbox};
use super::svg::{extract_attr_str, parse_attr_i32, svg_element_tags, tag_has_class};
use super::text::{
    extract_text_elements, TextAnchor, CHAR_WIDTH_PX, TEXT_ASCENT_PX, TEXT_DESCENT_PX,
};
use super::{AutoCorrect, EndpointSide, InvariantKind, InvariantViolation};

/// A polyline segment for intersection testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Segment {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

/// Returns `true` if the axis-aligned segment intersects the rectangle
/// `(bx, by, bw, bh)` (exclusive of the boundary pixels to avoid false
/// positives at port attachment points).
pub(crate) fn segment_crosses_rect(seg: Segment, bx: i32, by: i32, bw: i32, bh: i32) -> bool {
    // Clamp both endpoints to see if they enter the box interior.
    let rx1 = bx + 2;
    let ry1 = by + 2;
    let rx2 = bx + bw - 2;
    let ry2 = by + bh - 2;
    if rx1 >= rx2 || ry1 >= ry2 {
        return false;
    }

    let (sx1, sy1, sx2, sy2) = (seg.x1, seg.y1, seg.x2, seg.y2);

    // Horizontal segment: check if y is inside rect's y-range.
    if sy1 == sy2 {
        let y = sy1;
        if y <= ry1 || y >= ry2 {
            return false;
        }
        let min_x = sx1.min(sx2);
        let max_x = sx1.max(sx2);
        return min_x < rx2 && max_x > rx1;
    }

    // Vertical segment.
    if sx1 == sx2 {
        let x = sx1;
        if x <= rx1 || x >= rx2 {
            return false;
        }
        let min_y = sy1.min(sy2);
        let max_y = sy1.max(sy2);
        return min_y < ry2 && max_y > ry1;
    }

    // Diagonal: check bounding-box overlap as a conservative estimate.
    let seg_min_x = sx1.min(sx2);
    let seg_max_x = sx1.max(sx2);
    let seg_min_y = sy1.min(sy2);
    let seg_max_y = sy1.max(sy2);
    seg_min_x < rx2 && seg_max_x > rx1 && seg_min_y < ry2 && seg_max_y > ry1
}

/// Extract polyline/line endpoints from SVG edge hooks.
pub(crate) fn extract_relation_segments(svg: &str) -> Vec<(String, String, Vec<Segment>)> {
    let mut result = Vec::new();

    for tag in svg_element_tags(svg) {
        if !tag_has_class(tag, "puml-edge") && !tag_has_class(tag, "uml-relation") {
            continue;
        }

        let segs = segments_from_edge_tag(tag);
        if segs.is_empty() {
            continue;
        }

        let from = extract_attr_str(tag, "data-puml-from")
            .or_else(|| extract_attr_str(tag, "data-uml-from"))
            .unwrap_or_default();
        let to = extract_attr_str(tag, "data-puml-to")
            .or_else(|| extract_attr_str(tag, "data-uml-to"))
            .unwrap_or_default();
        result.push((from, to, segs));
    }

    result
}

pub(crate) fn extract_relation_segments_with_class(
    svg: &str,
    class_name: &str,
    from_attr: &str,
    to_attr: &str,
) -> Vec<(String, String, Vec<Segment>)> {
    let mut result = Vec::new();

    for tag in svg_element_tags(svg) {
        if !tag_has_class(tag, class_name) {
            continue;
        }

        let segs = segments_from_edge_tag(tag);

        if segs.is_empty() {
            continue;
        }

        let from = extract_attr_str(tag, from_attr).unwrap_or_default();
        let to = extract_attr_str(tag, to_attr).unwrap_or_default();
        result.push((from, to, segs));
    }

    result
}

fn segments_from_edge_tag(tag: &str) -> Vec<Segment> {
    if tag.starts_with("<polyline ") {
        parse_polyline_segments(tag)
    } else if tag.starts_with("<line ") {
        let x1 = parse_attr_i32(tag, "x1").unwrap_or(0);
        let y1 = parse_attr_i32(tag, "y1").unwrap_or(0);
        let x2 = parse_attr_i32(tag, "x2").unwrap_or(0);
        let y2 = parse_attr_i32(tag, "y2").unwrap_or(0);
        vec![Segment { x1, y1, x2, y2 }]
    } else {
        Vec::new()
    }
}

/// Parse `points="x1,y1 x2,y2 …"` into a list of `Segment` values.
pub(crate) fn parse_polyline_segments(tag: &str) -> Vec<Segment> {
    let Some(start) = tag.find("points=\"") else {
        return Vec::new();
    };
    let inner_start = start + "points=\"".len();
    let Some(rel_end) = tag[inner_start..].find('"') else {
        return Vec::new();
    };
    let pts_str = &tag[inner_start..inner_start + rel_end];

    let pts: Vec<(i32, i32)> = pts_str
        .split_whitespace()
        .filter_map(|pair| {
            let mut it = pair.splitn(2, ',');
            let x = it.next()?.parse::<f64>().ok()?.round() as i32;
            let y = it.next()?.parse::<f64>().ok()?.round() as i32;
            Some((x, y))
        })
        .collect();

    pts.windows(2)
        .map(|w| Segment {
            x1: w[0].0,
            y1: w[0].1,
            x2: w[1].0,
            y2: w[1].1,
        })
        .collect()
}

/// Check invariant #1: edge segments must not cross non-endpoint node bounding boxes.
///
/// When `mode == AutoCorrect::Apply` this function does NOT rewrite the SVG
/// polyline paths (that requires deeper routing logic wired into `graph_layout`);
/// instead it records violations with `corrected: false` so callers know which
/// edges need re-routing.  The full auto-correct for this invariant is handled
/// upstream at layout time — this pass is the structural assertion that catches
/// any regressions.
pub fn check_edge_node_clearance(svg: &str) -> Vec<InvariantViolation> {
    let nodes = extract_node_bboxes(svg);
    let relations = extract_relation_segments(svg);
    let mut violations = Vec::new();

    for (from, to, segs) in &relations {
        for seg in segs {
            for node in &nodes {
                // Skip the source and target nodes — edges are allowed to touch
                // their own endpoint boxes.
                if node.id == *from || node.id == *to {
                    continue;
                }
                if segment_crosses_rect(*seg, node.x, node.y, node.w, node.h) {
                    violations.push(InvariantViolation {
                        kind: InvariantKind::EdgeCrossesNode {
                            from: from.clone(),
                            to: to.clone(),
                            node_id: node.id.clone(),
                        },
                        corrected: false,
                        message: format!(
                            "[INV-1] edge {from:?}→{to:?} segment ({},{})→({},{}) crosses node {:?} bbox ({},{},{},{})",
                            seg.x1, seg.y1, seg.x2, seg.y2,
                            node.id, node.x, node.y, node.w, node.h
                        ),
                    });
                    // Report only the first crossing per edge-node pair.
                    break;
                }
            }
        }
    }

    violations
}

/// Minimum clearance in pixels between label bbox and edge stroke.
const MIN_LABEL_CLEARANCE_PX: i32 = 4;

/// Check invariant #3: every edge label must have ≥4px clearance from the
/// edge stroke, or a background rect will be inserted behind it.
///
/// When `mode == AutoCorrect::Apply`, a white background rect is spliced into
/// the SVG immediately before each offending `<text>` element.
pub fn check_label_edge_clearance(svg: &mut String, mode: AutoCorrect) -> Vec<InvariantViolation> {
    let relations = extract_relation_segments(svg);
    // Find all text elements with text-anchor="middle" (edge labels).
    let texts = extract_text_elements(svg);
    let mut violations = Vec::new();
    let mut inserts: Vec<(usize, String)> = Vec::new(); // (char-pos-in-svg, rect-svg)

    for (tx, ty, anchor, snippet) in &texts {
        let text_len: i32 = snippet.chars().count() as i32;
        let half_w = text_len * CHAR_WIDTH_PX / 2;
        let (label_x1, label_x2) = match anchor {
            TextAnchor::Middle => (tx - half_w, tx + half_w),
            TextAnchor::End => (tx - text_len * CHAR_WIDTH_PX, *tx),
            TextAnchor::Start => (*tx, tx + text_len * CHAR_WIDTH_PX),
        };
        let label_y1 = ty - TEXT_ASCENT_PX;
        let label_y2 = ty + TEXT_DESCENT_PX;

        for (_from, _to, segs) in &relations {
            for seg in segs {
                let clearance = segment_to_rect_clearance(
                    *seg,
                    label_x1,
                    label_y1,
                    label_x2 - label_x1,
                    label_y2 - label_y1,
                );
                if clearance < MIN_LABEL_CLEARANCE_PX {
                    violations.push(InvariantViolation {
                        kind: InvariantKind::LabelEdgeClearance {
                            from: _from.clone(),
                            to: _to.clone(),
                            clearance_px: clearance,
                        },
                        corrected: matches!(mode, AutoCorrect::Apply),
                        message: format!(
                            "[INV-3] label {:?} has only {clearance}px clearance from edge stroke (min {}px)",
                            &snippet[..snippet.len().min(20)],
                            MIN_LABEL_CLEARANCE_PX
                        ),
                    });

                    if matches!(mode, AutoCorrect::Apply) {
                        // Queue a white background rect to be inserted before
                        // the text element in the SVG.
                        let rect = format!(
                            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"white\" opacity=\"0.85\"/>",
                            label_x1 - 2,
                            label_y1 - 1,
                            (label_x2 - label_x1) + 4,
                            (label_y2 - label_y1) + 2
                        );
                        // Find the position of this text in the SVG to insert before it.
                        if let Some(pos) = find_text_element_pos(svg, *tx, *ty) {
                            inserts.push((pos, rect));
                        }
                    }
                    break; // one violation per label
                }
            }
        }
    }

    // Apply inserts in reverse order to preserve byte positions.
    if !inserts.is_empty() {
        inserts.sort_by_key(|b| Reverse(b.0));
        for (pos, rect) in inserts {
            svg.insert_str(pos, &rect);
        }
    }

    violations
}

/// Approximate minimum distance from a segment to a rectangle's boundary.
///
/// Returns 0 if the segment passes through the rect.  Otherwise returns the
/// minimum of:
///   • perpendicular distance from each endpoint to the rect boundary
///   • the closest-approach distance for the segment's interior to the rect
///
/// Uses axis-aligned geometry: only horizontal/vertical segments are treated
/// as passing close to the rect when they share the same y/x range.
/// Diagonal segments use the endpoint-based estimate as a conservative bound.
fn segment_to_rect_clearance(seg: Segment, rx: i32, ry: i32, rw: i32, rh: i32) -> i32 {
    if segment_crosses_rect(seg, rx, ry, rw, rh) {
        return 0;
    }

    // For horizontal segments, check the actual segment y against the rect's y-range.
    if seg.y1 == seg.y2 {
        let sy = seg.y1;
        let seg_x_min = seg.x1.min(seg.x2);
        let seg_x_max = seg.x1.max(seg.x2);
        let rect_x_max = rx + rw;
        // Only consider proximity if the segment's x-range overlaps the rect's x-range.
        if seg_x_min < rect_x_max && seg_x_max > rx {
            // Compute y-distance from segment to rect boundary.
            let dy = if sy < ry {
                ry - sy
            } else if sy > ry + rh {
                sy - (ry + rh)
            } else {
                0
            };
            return dy;
        }
    }

    // For vertical segments, check the actual segment x against the rect's x-range.
    if seg.x1 == seg.x2 {
        let sx = seg.x1;
        let seg_y_min = seg.y1.min(seg.y2);
        let seg_y_max = seg.y1.max(seg.y2);
        let rect_y_max = ry + rh;
        if seg_y_min < rect_y_max && seg_y_max > ry {
            let dx = if sx < rx {
                rx - sx
            } else if sx > rx + rw {
                sx - (rx + rw)
            } else {
                0
            };
            return dx;
        }
    }

    // Fallback: minimum Manhattan-distance from each endpoint to the nearest rect edge.
    // Only applies when the segment is diagonal or doesn't overlap the rect's range.
    let pts = [(seg.x1, seg.y1), (seg.x2, seg.y2)];
    let mut min_dist = i32::MAX;
    for (px, py) in pts {
        let dx = if px < rx {
            rx - px
        } else if px > rx + rw {
            px - (rx + rw)
        } else {
            0
        };
        let dy = if py < ry {
            ry - py
        } else if py > ry + rh {
            py - (ry + rh)
        } else {
            0
        };
        // Manhattan distance: use max of dx,dy as a conservative bound for
        // "how far is the endpoint from entering the rect in either axis".
        let d = dx + dy;
        min_dist = min_dist.min(d);
    }
    min_dist
}

/// Find the byte position in `svg` of a `<text` element with the given `x` and `y`.
fn find_text_element_pos(svg: &str, x: i32, y: i32) -> Option<usize> {
    let needle_x = format!("x=\"{x}\"");
    let needle_y = format!("y=\"{y}\"");
    let mut pos = 0;
    while let Some(rel) = svg[pos..].find("<text ") {
        let abs = pos + rel;
        let Some(rel_close) = svg[abs..].find('>') else {
            pos = abs + 1;
            continue;
        };
        let tag = &svg[abs..abs + rel_close];
        if tag.contains(&needle_x) && tag.contains(&needle_y) {
            return Some(abs);
        }
        pos = abs + 1;
    }
    None
}

/// Check invariant #6: every edge's first/last point must be within the bounding
/// box of its declared source/target node.  Returns diagnostic violations.
pub fn check_endpoint_connectivity(svg: &str) -> Vec<InvariantViolation> {
    let nodes = extract_node_bboxes(svg);
    let relations = extract_relation_segments(svg);
    let mut violations = Vec::new();

    for (from, to, segs) in &relations {
        if segs.is_empty() {
            continue;
        }
        let first_pt = (segs[0].x1, segs[0].y1);
        let last_pt = {
            let last = &segs[segs.len() - 1];
            (last.x2, last.y2)
        };

        // Source check
        if let Some(src_box) = nodes.iter().find(|n| n.id == *from) {
            if !point_touches_bbox(first_pt, src_box) {
                violations.push(InvariantViolation {
                    kind: InvariantKind::FloatingEndpoint {
                        from: from.clone(),
                        to: to.clone(),
                        side: EndpointSide::Source,
                    },
                    corrected: false,
                    message: format!(
                        "[INV-6] edge {from:?}→{to:?} first point ({},{}) is not on source node bbox",
                        first_pt.0, first_pt.1
                    ),
                });
            }
        }

        // Target check
        if let Some(tgt_box) = nodes.iter().find(|n| n.id == *to) {
            if !point_touches_bbox(last_pt, tgt_box) {
                violations.push(InvariantViolation {
                    kind: InvariantKind::FloatingEndpoint {
                        from: from.clone(),
                        to: to.clone(),
                        side: EndpointSide::Target,
                    },
                    corrected: false,
                    message: format!(
                        "[INV-6] edge {from:?}→{to:?} last point ({},{}) is not on target node bbox",
                        last_pt.0, last_pt.1
                    ),
                });
            }
        }
    }

    violations
}

/// Returns `true` if point `(px, py)` lies on or just inside the perimeter
/// of the bounding box (within 4px tolerance to handle port attachment snap).
fn point_touches_bbox(pt: (i32, i32), bbox: &NodeBbox) -> bool {
    let tol = 4;
    let (px, py) = pt;
    px >= bbox.x - tol
        && px <= bbox.x + bbox.w + tol
        && py >= bbox.y - tol
        && py <= bbox.y + bbox.h + tol
}
