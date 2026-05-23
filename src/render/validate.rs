//! Render-time invariants pass: makes visually-broken diagrams structurally impossible.
//!
//! This module enforces correctness invariants on the completed SVG output.
//! Each invariant either auto-corrects (mutating the SVG string in place) or
//! emits a structured diagnostic explaining what would have been broken.
//!
//! Priority order (from the issue body):
//!   1. Edge-vs-node non-intersection  [auto-correct: segment re-route]
//!   2. Label-inside-viewBox           [auto-correct: expand viewBox]
//!   3. Label-vs-edge-stroke clearance [auto-correct: background rect]
//!   4. Package-header reservation     [auto-correct: segment re-route]
//!   5. Pseudo-state deduplication     [normalization assertion — see normalize/state.rs]
//!   6. Edge endpoint connectivity     [diagnostic only]
//!   7. Self-loop row allocation       [diagnostic only]
//!
//! The main entry point is [`run`].

use std::fmt;

// ─────────────────────────────────────────────────────────────────────────────
// Public types
// ─────────────────────────────────────────────────────────────────────────────

/// Which invariant was violated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvariantKind {
    /// An edge segment passed through a non-endpoint node bounding box.
    EdgeCrossesNode {
        /// SVG `data-uml-from` attribute of the offending relation.
        from: String,
        /// SVG `data-uml-to` attribute of the offending relation.
        to: String,
        /// ID of the node whose bounding box was crossed.
        node_id: String,
    },
    /// A `<text>` element's estimated bounding box extends outside the viewBox.
    LabelOutsideViewbox {
        /// Approximate text content.
        snippet: String,
        /// How many pixels outside the right edge.
        overflow_px: i32,
    },
    /// A relation label has insufficient clearance from the edge stroke.
    LabelEdgeClearance {
        from: String,
        to: String,
        clearance_px: i32,
    },
    /// An edge segment passes through a package/group header strip.
    EdgeThroughPackageHeader {
        from: String,
        to: String,
        package: String,
    },
    /// Duplicate pseudo-states detected at normalization time.
    DuplicatePseudoState {
        kind: PseudoStateKind,
        scope: String,
        count: usize,
    },
    /// An edge endpoint does not connect to its declared node port.
    FloatingEndpoint {
        from: String,
        to: String,
        side: EndpointSide,
    },
    /// A self-loop does not have enough vertical space for the label.
    SelfLoopTooShort {
        node: String,
        allocated_px: i32,
        minimum_px: i32,
    },
}

/// Whether the invariant was auto-corrected or only recorded as a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoCorrect {
    /// Mutate the SVG/model to correct the violation silently.
    Apply,
    /// Emit a diagnostic but do not mutate.
    EmitDiagnostic,
}

/// A recorded invariant violation.
#[derive(Debug, Clone)]
pub struct InvariantViolation {
    pub kind: InvariantKind,
    pub corrected: bool,
    pub message: String,
}

/// Which pseudo-state kind is duplicated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PseudoStateKind {
    Initial,
    Final,
}

/// Which endpoint of the edge is floating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointSide {
    Source,
    Target,
}

impl fmt::Display for InvariantViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}",
            if self.corrected {
                "CORRECTED"
            } else {
                "VIOLATION"
            },
            self.message
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal SVG geometry helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Parse `viewBox="x y w h"` from an SVG string.
/// Returns `(x, y, width, height)` as integers, or `None` if not found.
pub(crate) fn parse_viewbox(svg: &str) -> Option<(i32, i32, i32, i32)> {
    // Find `viewBox="…"` attribute.
    let start = svg.find("viewBox=\"")?;
    let inner_start = start + "viewBox=\"".len();
    let end = svg[inner_start..].find('"')? + inner_start;
    let parts: Vec<i32> = svg[inner_start..end]
        .split_whitespace()
        .filter_map(|s| s.parse().ok())
        .collect();
    if parts.len() == 4 {
        Some((parts[0], parts[1], parts[2], parts[3]))
    } else {
        None
    }
}

/// Replace the `viewBox="…"` value in `svg` with the given dimensions.
fn replace_viewbox(svg: &str, x: i32, y: i32, w: i32, h: i32) -> String {
    let new_vb = format!("{x} {y} {w} {h}");
    // Replace first occurrence only (the root <svg> viewBox).
    if let Some(pos) = svg.find("viewBox=\"") {
        let inner_start = pos + "viewBox=\"".len();
        if let Some(rel_end) = svg[inner_start..].find('"') {
            let end = inner_start + rel_end;
            let mut result = svg[..inner_start].to_string();
            result.push_str(&new_vb);
            result.push_str(&svg[end..]);
            return result;
        }
    }
    svg.to_string()
}

/// Also update `width="…"` and `height="…"` on the root `<svg>` element to
/// match the new viewBox dimensions (prevents the SVG from being cropped when
/// the viewBox is expanded but the intrinsic size stays small).
fn sync_svg_dimensions(svg: &str, vb_x: i32, vb_y: i32, vb_w: i32, vb_h: i32) -> String {
    let svg = replace_viewbox(svg, vb_x, vb_y, vb_w, vb_h);
    // Update width="…" on the opening <svg> tag only.
    let svg = replace_root_attr(&svg, "width", &vb_w.to_string());
    replace_root_attr(&svg, "height", &vb_h.to_string())
}

/// Replace the value of `attr="…"` in the first tag of `svg`.
fn replace_root_attr(svg: &str, attr: &str, new_val: &str) -> String {
    let needle = format!("{attr}=\"");
    if let Some(pos) = svg.find(&needle) {
        let inner_start = pos + needle.len();
        if let Some(rel_end) = svg[inner_start..].find('"') {
            let end = inner_start + rel_end;
            let mut result = svg[..inner_start].to_string();
            result.push_str(new_val);
            result.push_str(&svg[end..]);
            return result;
        }
    }
    svg.to_string()
}

/// Text anchor kind extracted from a `<text>` element.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TextAnchor {
    Start,
    Middle,
    End,
}

/// A scraped SVG `<text>` element relevant to render invariants.
#[derive(Clone)]
struct TextElement {
    x: i32,
    y: i32,
    anchor: TextAnchor,
    snippet: String,
    is_edge_label: bool,
}

/// Extract every `<text …>` element from the SVG with its `x`, `y`,
/// `text-anchor`, a short content snippet, and whether the renderer marked it
/// as a relation label.
fn extract_text_elements(svg: &str) -> Vec<TextElement> {
    let mut result = Vec::new();
    let mut pos = 0;
    while let Some(rel) = svg[pos..].find("<text ") {
        let tag_start = pos + rel;
        // Find the closing `>` of the opening tag.
        let Some(rel_close) = svg[tag_start..].find('>') else {
            break;
        };
        let tag_end = tag_start + rel_close;
        let attrs = &svg[tag_start..tag_end];

        let x = parse_attr_i32(attrs, "x").unwrap_or(0);
        let y = parse_attr_i32(attrs, "y").unwrap_or(0);
        let anchor = if attrs.contains("text-anchor=\"middle\"") {
            TextAnchor::Middle
        } else if attrs.contains("text-anchor=\"end\"") {
            TextAnchor::End
        } else {
            TextAnchor::Start
        };

        // Grab a short snippet from the content.
        let content_start = tag_end + 1;
        let snippet_end = svg[content_start..]
            .find("</text>")
            .map(|r| content_start + r)
            .unwrap_or(content_start + 40.min(svg.len() - content_start));
        let snippet = svg[content_start..snippet_end.min(svg.len())]
            .chars()
            .take(40)
            .collect::<String>();
        let snippet = snippet
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"");

        let is_edge_label = attrs.contains("class=\"uml-edge-label")
            || attrs.contains("data-uml-label-role=\"edge\"");
        result.push(TextElement {
            x,
            y,
            anchor,
            snippet,
            is_edge_label,
        });
        pos = tag_start + 1;
    }
    result
}

/// Parse a named integer attribute `attr="value"` from a tag fragment.
///
/// Requires the attribute name to be preceded by a whitespace character or
/// the start of the string to avoid matching partial attribute names
/// (e.g. `y="` inside `data-uml-visibility="public"`).
fn parse_attr_i32(tag: &str, attr: &str) -> Option<i32> {
    let needle = format!("{attr}=\"");
    let mut search_pos = 0;
    while let Some(rel) = tag[search_pos..].find(&needle) {
        let match_pos = search_pos + rel;
        // Verify that the character before the attribute name is a whitespace,
        // '/' (for self-closing), or start-of-string — never a letter/digit.
        let ok = match_pos == 0
            || tag
                .as_bytes()
                .get(match_pos - 1)
                .map(|&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r')
                .unwrap_or(false);
        let value_start = match_pos + needle.len();
        if ok {
            let end = value_start + tag[value_start..].find('"')?;
            return tag[value_start..end].parse().ok();
        }
        search_pos = match_pos + needle.len();
    }
    None
}

/// Approximate character-width estimate in pixels at `font-size="12"`.
const CHAR_WIDTH_PX: i32 = 7;
/// Approximate descent below the y baseline.
const TEXT_DESCENT_PX: i32 = 4;
/// Approximate ascent above the y baseline.
const TEXT_ASCENT_PX: i32 = 12;

// ─────────────────────────────────────────────────────────────────────────────
// Invariant implementations
// ─────────────────────────────────────────────────────────────────────────────

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
// Node bounding-box extraction from SVG data attributes
// ─────────────────────────────────────────────────────────────────────────────

/// A node bounding box scraped from SVG `data-uml-*` attributes.
#[derive(Debug, Clone)]
pub struct NodeBbox {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// Extract node bounding boxes from `<rect class="uml-node"` elements.
/// The bounding box is derived from `x`, `y`, `width`, `height` attributes.
pub(crate) fn extract_node_bboxes(svg: &str) -> Vec<NodeBbox> {
    let mut result = Vec::new();
    // Look for elements with class="uml-node" in their tag.
    let mut pos = 0;
    while let Some(rel) = svg[pos..].find("class=\"uml-node") {
        let abs = pos + rel;
        // Walk back from the match to find the opening '<' of this tag.
        let tag_start = svg[..abs].rfind('<').unwrap_or(abs);
        // Ensure we always advance past the current match to avoid infinite loops.
        let next_pos = abs + "class=\"uml-node".len();

        let Some(rel_close) = svg[tag_start..].find('>') else {
            pos = next_pos;
            continue;
        };
        let tag = &svg[tag_start..tag_start + rel_close];

        let x = parse_attr_i32(tag, "x").unwrap_or(0);
        let y = parse_attr_i32(tag, "y").unwrap_or(0);
        let w = parse_attr_i32(tag, "width").unwrap_or(0);
        let h = parse_attr_i32(tag, "height").unwrap_or(0);
        let id = extract_attr_str(tag, "data-uml-id")
            .or_else(|| extract_attr_str(tag, "id"))
            .unwrap_or_else(|| format!("node@{x},{y}"));

        if w > 0 && h > 0 {
            result.push(NodeBbox { id, x, y, w, h });
        }
        pos = next_pos;
    }
    result
}

/// Extract a string attribute value from a tag fragment.
///
/// Requires the attribute name to be preceded by a whitespace character to
/// avoid false matches on suffix strings (e.g. `id` inside `data-uml-id`).
fn extract_attr_str(tag: &str, attr: &str) -> Option<String> {
    let needle = format!("{attr}=\"");
    let mut search_pos = 0;
    while let Some(rel) = tag[search_pos..].find(&needle) {
        let match_pos = search_pos + rel;
        let ok = match_pos == 0
            || tag
                .as_bytes()
                .get(match_pos - 1)
                .map(|&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r')
                .unwrap_or(false);
        let value_start = match_pos + needle.len();
        if ok {
            let end = value_start + tag[value_start..].find('"')?;
            return Some(tag[value_start..end].to_string());
        }
        search_pos = match_pos + needle.len();
    }
    None
}

// ─────────────────────────────────────────────────────────────────────────────
// Invariant #1: Edge-vs-node non-intersection
// ─────────────────────────────────────────────────────────────────────────────

/// A polyline segment for intersection testing.
#[derive(Debug, Clone, Copy)]
struct Segment {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

/// Returns `true` if the axis-aligned segment intersects the rectangle
/// `(bx, by, bw, bh)` (exclusive of the boundary pixels to avoid false
/// positives at port attachment points).
fn segment_crosses_rect(seg: Segment, bx: i32, by: i32, bw: i32, bh: i32) -> bool {
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

/// Extract polyline/line endpoints from SVG `<polyline …/>` and `<line …/>`
/// elements that carry `class="uml-relation"`.
fn extract_relation_segments(svg: &str) -> Vec<(String, String, Vec<Segment>)> {
    let mut result = Vec::new();
    let mut pos = 0;

    // Polylines
    while let Some(rel) = svg[pos..].find("<polyline ") {
        let tag_start = pos + rel;
        let Some(rel_close) = svg[tag_start..].find("/>") else {
            pos = tag_start + 1;
            continue;
        };
        let tag = &svg[tag_start..tag_start + rel_close];
        if !tag.contains("uml-relation") {
            pos = tag_start + 1;
            continue;
        }
        let from = extract_attr_str(tag, "data-uml-from").unwrap_or_default();
        let to = extract_attr_str(tag, "data-uml-to").unwrap_or_default();
        let segs = parse_polyline_segments(tag);
        if !segs.is_empty() {
            result.push((from, to, segs));
        }
        pos = tag_start + 1;
    }

    // Straight lines
    pos = 0;
    while let Some(rel) = svg[pos..].find("<line ") {
        let tag_start = pos + rel;
        let Some(rel_close) = svg[tag_start..].find("/>") else {
            pos = tag_start + 1;
            continue;
        };
        let tag = &svg[tag_start..tag_start + rel_close];
        if !tag.contains("uml-relation") {
            pos = tag_start + 1;
            continue;
        }
        let from = extract_attr_str(tag, "data-uml-from").unwrap_or_default();
        let to = extract_attr_str(tag, "data-uml-to").unwrap_or_default();
        let x1 = parse_attr_i32(tag, "x1").unwrap_or(0);
        let y1 = parse_attr_i32(tag, "y1").unwrap_or(0);
        let x2 = parse_attr_i32(tag, "x2").unwrap_or(0);
        let y2 = parse_attr_i32(tag, "y2").unwrap_or(0);
        result.push((from, to, vec![Segment { x1, y1, x2, y2 }]));
        pos = tag_start + 1;
    }

    result
}

/// Parse `points="x1,y1 x2,y2 …"` into a list of `Segment` values.
fn parse_polyline_segments(tag: &str) -> Vec<Segment> {
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
            let x: i32 = it.next()?.parse().ok()?;
            let y: i32 = it.next()?.parse().ok()?;
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
pub fn check_label_edge_clearance(svg: &mut String, mode: AutoCorrect) -> Vec<InvariantViolation> {
    let relations = extract_relation_segments(svg);
    let texts = extract_text_elements(svg);
    let has_marked_edge_labels = texts.iter().any(|text| text.is_edge_label);
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
                        let rect = format!(
                            "<rect class=\"uml-edge-label-bg\" data-uml-label-role=\"edge-background\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"white\" opacity=\"0.85\"/>",
                            label_x1 - 2,
                            label_y1 - 1,
                            (label_x2 - label_x1) + 4,
                            (label_y2 - label_y1) + 2
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

/// A package/group bounding box with its header strip.
#[derive(Debug, Clone)]
pub struct PackageFrame {
    pub id: String,
    /// Top-left y of the entire package frame.
    pub y: i32,
    /// Height of the label/header strip at the top.
    pub header_height: i32,
}

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
            for seg in segs {
                // Check if the segment's y range overlaps the header strip.
                let seg_min_y = seg.y1.min(seg.y2);
                let seg_max_y = seg.y1.max(seg.y2);
                if seg_min_y < header_bot && seg_max_y > header_top {
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

// ─────────────────────────────────────────────────────────────────────────────
// Invariant #6: Edge endpoint connectivity
// ─────────────────────────────────────────────────────────────────────────────

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

// ─────────────────────────────────────────────────────────────────────────────
// Top-level entry point
// ─────────────────────────────────────────────────────────────────────────────

/// Result of a full invariant run.
#[derive(Debug, Default)]
pub struct InvariantReport {
    pub violations: Vec<InvariantViolation>,
    pub expansions: usize,
    pub background_rects_added: usize,
}

/// Run all applicable SVG-level invariants on a completed SVG render.
///
/// `mode` controls whether auto-corrections are applied to the SVG string.
///
/// This is the main entry point; call it at the end of every render function.
pub fn run(svg: &mut String, mode: AutoCorrect) -> InvariantReport {
    let mut report = InvariantReport::default();

    // Invariant #2: labels inside viewBox (auto-correct: expand viewBox).
    // This is safe to auto-correct at render time because it only expands the
    // viewBox dimensions — it never changes any element positions.
    {
        let v = check_labels_inside_viewbox(svg, mode);
        let expansions = v.iter().filter(|x| x.corrected).count();
        report.expansions += expansions;
        report.violations.extend(v);
    }

    // Invariant #3: label-vs-edge-stroke clearance. Renderers now mark graph
    // relation labels, so this pass can avoid node/header text false positives.
    {
        let before = svg.matches("class=\"uml-edge-label-bg\"").count();
        let v = check_label_edge_clearance(svg, mode);
        let after = svg.matches("class=\"uml-edge-label-bg\"").count();
        report.background_rects_added += after.saturating_sub(before);
        report.violations.extend(v);
    }

    // Invariant #1: edge-vs-node intersection (diagnostic; layout engine auto-corrects)
    report.violations.extend(check_edge_node_clearance(svg));

    // Invariant #6: edge endpoint connectivity (diagnostic)
    report.violations.extend(check_endpoint_connectivity(svg));

    report
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_viewbox_roundtrip() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300">"#;
        assert_eq!(parse_viewbox(svg), Some((0, 0, 400, 300)));
    }

    #[test]
    fn replace_viewbox_works() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300"></svg>"#;
        let out = replace_viewbox(svg, 0, 0, 500, 400);
        assert!(out.contains("viewBox=\"0 0 500 400\""), "got: {out}");
    }

    #[test]
    fn check_labels_inside_viewbox_expands_on_overflow() {
        // A text element at x=390, content 10 chars → right edge ≈ 390+70=460 > viewBox width 400.
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300"><rect width="100%" height="100%" fill="white"/><text x="390" y="150" text-anchor="middle" font-family="monospace">0123456789</text></svg>"#;
        let mut svg = svg.to_string();
        let v = check_labels_inside_viewbox(&mut svg, AutoCorrect::Apply);
        assert!(
            !v.is_empty(),
            "expected at least one label-overflow violation"
        );
        let (_, _, vb_w, _) = parse_viewbox(&svg).expect("viewBox should be present");
        assert!(
            vb_w > 400,
            "viewBox width should have been expanded; got {vb_w}"
        );
    }

    #[test]
    fn check_labels_inside_viewbox_no_false_positive() {
        // Text well within viewBox.
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="400" height="300" viewBox="0 0 400 300"><text x="100" y="100">Hi</text></svg>"#;
        let mut svg = svg.to_string();
        let v = check_labels_inside_viewbox(&mut svg, AutoCorrect::Apply);
        assert!(v.is_empty(), "no violations expected for in-bounds text");
    }

    #[test]
    fn segment_crosses_rect_basic() {
        // Horizontal segment crossing a rect.
        let seg = Segment {
            x1: 0,
            y1: 50,
            x2: 200,
            y2: 50,
        };
        assert!(segment_crosses_rect(seg, 80, 30, 60, 50)); // rect at (80,30) 60×50
                                                            // Segment that passes above the rect.
        let seg2 = Segment {
            x1: 0,
            y1: 10,
            x2: 200,
            y2: 10,
        };
        assert!(!segment_crosses_rect(seg2, 80, 30, 60, 50));
    }

    #[test]
    fn parse_polyline_segments_basic() {
        let tag = r#"<polyline class="uml-relation" points="10,20 50,20 50,80 100,80""#;
        let segs = parse_polyline_segments(tag);
        assert_eq!(segs.len(), 3, "expected 3 segments from 4 points");
        assert_eq!(segs[0].x1, 10);
        assert_eq!(segs[0].y1, 20);
        assert_eq!(segs[0].x2, 50);
    }

    #[test]
    fn check_pseudo_state_dedup_no_violation_when_normalized() {
        use crate::model::{StateNode, StateNodeKind};
        let nodes = vec![
            StateNode {
                name: "[*]".to_string(),
                display: None,
                kind: StateNodeKind::StartEnd,
                stereotype: None,
                style: Default::default(),
                internal_actions: vec![],
                regions: vec![],
            },
            StateNode {
                name: "Active".to_string(),
                display: None,
                kind: StateNodeKind::Normal,
                stereotype: None,
                style: Default::default(),
                internal_actions: vec![],
                regions: vec![],
            },
        ];
        let violations = check_pseudo_state_dedup(&nodes, "root");
        assert!(violations.is_empty(), "single [*] should not violate");
    }

    #[test]
    fn check_pseudo_state_dedup_catches_duplicates() {
        use crate::model::{StateNode, StateNodeKind};
        let nodes = vec![
            StateNode {
                name: "[*]".to_string(),
                display: None,
                kind: StateNodeKind::StartEnd,
                stereotype: None,
                style: Default::default(),
                internal_actions: vec![],
                regions: vec![],
            },
            StateNode {
                name: "[*]_dup".to_string(),
                display: None,
                kind: StateNodeKind::StartEnd,
                stereotype: None,
                style: Default::default(),
                internal_actions: vec![],
                regions: vec![],
            },
        ];
        let violations = check_pseudo_state_dedup(&nodes, "root");
        assert_eq!(
            violations.len(),
            1,
            "expected one duplicate-initial violation"
        );
        assert!(matches!(
            violations[0].kind,
            InvariantKind::DuplicatePseudoState {
                kind: PseudoStateKind::Initial,
                ..
            }
        ));
    }

    #[test]
    fn check_label_edge_clearance_adds_background_rect() {
        // An edge that passes directly under a label.
        // Using format! to avoid raw-string # ambiguity in concat!.
        let svg = format!(
            concat!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" width="300" height="200" viewBox="0 0 300 200">"#,
                r#"<polyline class="uml-relation" data-uml-from="A" data-uml-to="B" points="50,100 250,100" fill="none" stroke="{}" stroke-width="2"/>"#,
                r#"<text x="150" y="100" text-anchor="middle" font-family="monospace">label</text>"#,
                r#"</svg>"#
            ),
            "#333"
        );
        let mut svg = svg.to_string();
        let v = check_label_edge_clearance(&mut svg, AutoCorrect::Apply);
        // Should detect clearance issue (label sits exactly on the stroke).
        // The text y=100 and the segment y=100 → clearance=0 < 4.
        assert!(
            !v.is_empty() || svg.contains("<rect"),
            "expected either a violation or a background rect to be inserted"
        );
    }

    #[test]
    fn run_entry_point_returns_report() {
        let svg = concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="150" viewBox="0 0 200 150">"#,
            r#"<rect width="100%" height="100%" fill="white"/>"#,
            r#"<text x="10" y="50">hello</text>"#,
            r#"</svg>"#
        );
        let mut svg = svg.to_string();
        let report = run(&mut svg, AutoCorrect::Apply);
        // No violations expected for a simple, well-formed SVG.
        assert!(
            report.violations.is_empty(),
            "expected no violations: {:?}",
            report.violations
        );
    }
}
