use super::geometry::{
    extract_package_frames, extract_relation_segments, segment_crosses_rect, PackageFrame, Segment,
};
use super::svg_hooks::{
    extract_text_elements, parse_viewbox, sync_svg_dimensions, TextAnchor, CHAR_WIDTH_PX,
    TEXT_ASCENT_PX, TEXT_DESCENT_PX,
};
use super::types::{AutoCorrect, InvariantKind, InvariantViolation, PseudoStateKind};

/// Check that every `<text>` element's estimated bounding box fits inside the
/// current viewBox.  If it overflows to the right or bottom, expand the viewBox
/// to contain it (auto-correct) and return the number of expansions applied.
///
/// This is invariant #2.
pub fn check_labels_inside_viewbox(svg: &mut String, mode: AutoCorrect) -> Vec<InvariantViolation> {
    let Some((vb_x, vb_y, mut vb_w, mut vb_h)) = parse_viewbox(svg) else {
        return Vec::new();
    };

    let texts = extract_text_elements(svg);
    let mut violations = Vec::new();
    let mut expanded = false;

    for text in &texts {
        let text_len: i32 = text.snippet.chars().count() as i32;
        let half_w = text_len * CHAR_WIDTH_PX / 2;
        // Compute the actual left/right edges depending on text-anchor.
        let (text_left, text_right) = match text.anchor {
            TextAnchor::Middle => (text.x - half_w, text.x + half_w),
            TextAnchor::End => (text.x - text_len * CHAR_WIDTH_PX, text.x),
            TextAnchor::Start => (text.x, text.x + text_len * CHAR_WIDTH_PX),
        };
        let text_bottom = text.y + TEXT_DESCENT_PX;
        let text_top = text.y - TEXT_ASCENT_PX;

        let left_overflow = (vb_x - text_left).max(0);
        let right_overflow = (text_right - (vb_x + vb_w)).max(0);
        let bottom_overflow = (text_bottom - (vb_y + vb_h)).max(0);
        let top_overflow = (vb_y - text_top).max(0);

        if left_overflow > 0 || right_overflow > 0 || bottom_overflow > 0 || top_overflow > 0 {
            let overflow_px = left_overflow
                .max(right_overflow)
                .max(bottom_overflow)
                .max(top_overflow);
            violations.push(InvariantViolation {
                kind: InvariantKind::LabelOutsideViewbox {
                    snippet: text.snippet.clone(),
                    overflow_px,
                },
                corrected: matches!(mode, AutoCorrect::Apply),
                message: format!(
                    "[INV-2] label {:?} overflows viewBox by {}px",
                    &text.snippet[..text.snippet.len().min(20)],
                    overflow_px
                ),
            });

            if matches!(mode, AutoCorrect::Apply) {
                // Expand viewBox to contain the overflow.
                if left_overflow > 0 {
                    let new_x = vb_x - left_overflow - 8;
                    vb_w += vb_x - new_x;
                    // vb_x = new_x; // keep vb_x stable; just expand width
                }
                vb_w = vb_w.max(text_right - vb_x + 8);
                vb_h = vb_h.max(text_bottom - vb_y + 8);
                if top_overflow > 0 {
                    vb_h += top_overflow;
                }
                expanded = true;
            }
        }
    }

    if expanded {
        *svg = sync_svg_dimensions(svg, vb_x, vb_y, vb_w, vb_h);
    }

    violations
}

// ─────────────────────────────────────────────────────────────────────────────
// Invariant #3: Label-vs-edge-stroke clearance
// ─────────────────────────────────────────────────────────────────────────────

/// Minimum clearance in pixels between label bbox and edge stroke.
const MIN_LABEL_CLEARANCE_PX: i32 = 4;

/// Check invariant #3: every edge label must have ≥4px clearance from the
/// edge stroke, or a background rect will be inserted behind it.
///
/// When `mode == AutoCorrect::Apply`, a white background rect is spliced into
/// the SVG immediately before each offending `<text>` element.
///
/// The inserted rect is pushed below any group-header band it would otherwise
/// paint over (fixes #1344: white bg rects on navy package headers).
pub fn check_label_edge_clearance(svg: &mut String, mode: AutoCorrect) -> Vec<InvariantViolation> {
    let relations = extract_relation_segments(svg);
    let texts = extract_text_elements(svg);
    let has_marked_edge_labels = texts.iter().any(|text| text.is_edge_label);
    // Extract package frames once so we can avoid placing bg rects over headers.
    let package_frames = if matches!(mode, AutoCorrect::Apply) {
        extract_package_frames(svg)
    } else {
        Vec::new()
    };
    let mut violations = Vec::new();
    let mut inserts: Vec<(usize, String)> = Vec::new(); // (char-pos-in-svg, rect-svg)

    for text in &texts {
        if has_marked_edge_labels && !text.is_edge_label {
            continue;
        }
        let text_len: i32 = text.snippet.chars().count() as i32;
        let half_w = text_len * CHAR_WIDTH_PX / 2;
        let (label_x1, label_x2) = match text.anchor {
            TextAnchor::Middle => (text.x - half_w, text.x + half_w),
            TextAnchor::End => (text.x - text_len * CHAR_WIDTH_PX, text.x),
            TextAnchor::Start => (text.x, text.x + text_len * CHAR_WIDTH_PX),
        };
        let label_y1 = text.y - TEXT_ASCENT_PX;
        let label_y2 = text.y + TEXT_DESCENT_PX;

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
                            &text.snippet[..text.snippet.len().min(20)],
                            MIN_LABEL_CLEARANCE_PX
                        ),
                    });

                    if matches!(mode, AutoCorrect::Apply) {
                        // Queue a white background rect to be inserted before
                        // the text element in the SVG.
                        let rx = label_x1 - 2;
                        let mut ry = label_y1 - 1;
                        let rw = (label_x2 - label_x1) + 4;
                        let rh = (label_y2 - label_y1) + 2;
                        // Push rect below any group-header band it would paint over.
                        // Header band occupies [frame.y, frame.y + header_height).
                        // If the rect overlaps the band horizontally AND vertically,
                        // shift ry to just below the header bottom (#1344).
                        for frame in &package_frames {
                            let header_top = frame.y;
                            let header_bot = frame.y + frame.header_height;
                            let rect_bot = ry + rh;
                            let overlaps_x = rx < frame.x + frame.width && rx + rw > frame.x;
                            let overlaps_y = ry < header_bot && rect_bot > header_top;
                            if overlaps_x && overlaps_y {
                                ry = header_bot + 1;
                            }
                        }
                        let rect = format!(
                            "<rect class=\"uml-edge-label-bg\" data-uml-label-role=\"edge-background\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"white\" opacity=\"0.85\"/>",
                            rx, ry, rw, rh
                        );
                        // Find the position of this text in the SVG to insert before it.
                        if let Some(pos) = find_text_element_pos(svg, text.x, text.y) {
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
        inserts.sort_by_key(|b| std::cmp::Reverse(b.0));
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

// ─────────────────────────────────────────────────────────────────────────────
// Invariant #4: Package-header reservation
// ─────────────────────────────────────────────────────────────────────────────

/// Check invariant #4: edge segments must not pass through package header strips.
///
/// Returns violations.  Auto-correction requires re-routing the edge path, which
/// is left to the layout engine — this pass records violations for diagnostics.
pub fn check_package_headers(svg: &str, frames: &[PackageFrame]) -> Vec<InvariantViolation> {
    let relations = extract_relation_segments(svg);
    let mut violations = Vec::new();

    for (from, to, segs) in &relations {
        for frame in frames {
            let header_top = frame.y;
            let header_bot = frame.y + frame.header_height;
            let header_left = frame.x;
            let header_right = frame.x + frame.width;
            for seg in segs {
                let seg_min_x = seg.x1.min(seg.x2);
                let seg_max_x = seg.x1.max(seg.x2);
                let seg_min_y = seg.y1.min(seg.y2);
                let seg_max_y = seg.y1.max(seg.y2);
                let overlaps_header = seg_min_x < header_right
                    && seg_max_x > header_left
                    && seg_min_y < header_bot
                    && seg_max_y > header_top;
                if overlaps_header {
                    violations.push(InvariantViolation {
                        kind: InvariantKind::EdgeThroughPackageHeader {
                            from: from.clone(),
                            to: to.clone(),
                            package: frame.id.clone(),
                        },
                        corrected: false,
                        message: format!(
                            "[INV-4] edge {from:?}→{to:?} passes through package {:?} header strip [y={}, h={}]",
                            frame.id, frame.y, frame.header_height
                        ),
                    });
                    break;
                }
            }
        }
    }

    violations
}

// ─────────────────────────────────────────────────────────────────────────────
// Invariant #5: Pseudo-state deduplication (normalization-time assertion)
// ─────────────────────────────────────────────────────────────────────────────

/// Assert that the flat `nodes` list (post-normalization) contains at most one
/// canonical initial pseudo-state and at most one canonical final pseudo-state
/// at each nesting level.
///
/// Returns violations describing duplicates found.
///
/// Note: this function operates on the already-normalized model (after
/// `normalize/state.rs` has run) — it is an assertion, not a deduplication
/// pass.  The normalization pass is the authoritative place where `[*]` is
/// split into initial + final; this function just verifies the invariant held.
pub fn check_pseudo_state_dedup(
    nodes: &[crate::model::StateNode],
    scope: &str,
) -> Vec<InvariantViolation> {
    use crate::model::StateNodeKind;
    let mut violations = Vec::new();

    // Count StartEnd nodes (initial pseudo-state = has outgoing transitions
    // from [*]; final is canonicalized to End).  At the flat level, only one
    // [*] node should remain.
    let start_count = nodes
        .iter()
        .filter(|n| n.kind == StateNodeKind::StartEnd)
        .count();
    if start_count > 1 {
        violations.push(InvariantViolation {
            kind: InvariantKind::DuplicatePseudoState {
                kind: PseudoStateKind::Initial,
                scope: scope.to_string(),
                count: start_count,
            },
            corrected: false,
            message: format!(
                "[INV-5] scope {scope:?} has {start_count} initial pseudo-states; expected ≤1"
            ),
        });
    }

    let end_count = nodes
        .iter()
        .filter(|n| n.kind == StateNodeKind::End || n.name == "[*]__end")
        .count();
    if end_count > 1 {
        violations.push(InvariantViolation {
            kind: InvariantKind::DuplicatePseudoState {
                kind: PseudoStateKind::Final,
                scope: scope.to_string(),
                count: end_count,
            },
            corrected: false,
            message: format!(
                "[INV-5] scope {scope:?} has {end_count} final pseudo-states; expected ≤1"
            ),
        });
    }

    violations
}
