// ─────────────────────────────────────────────────────────────────────────────
// Node bounding-box extraction from SVG data attributes
// ─────────────────────────────────────────────────────────────────────────────

use super::svg_hooks::{parse_attr_i32, parse_attr_i32_lossy};

/// A node bounding box scraped from SVG `data-uml-*` attributes.
#[derive(Debug, Clone)]
pub struct NodeBbox {
    pub id: String,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

/// A package/group bounding box with its header strip.
#[derive(Debug, Clone)]
pub struct PackageFrame {
    pub id: String,
    /// Left edge of the package frame.
    pub x: i32,
    /// Top-left y of the entire package frame.
    pub y: i32,
    /// Width of the package frame.
    pub width: i32,
    /// Height of the label/header strip at the top.
    pub header_height: i32,
}

/// Extract node bounding boxes from SVG elements marked `class="uml-node"`.
/// Supports the shape forms currently emitted by graph-family renderers.
pub fn extract_node_bboxes(svg: &str) -> Vec<NodeBbox> {
    let mut result = Vec::new();
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
        let tag_name = tag
            .trim_start_matches('<')
            .split_whitespace()
            .next()
            .unwrap_or("");
        let Some((x, y, w, h)) = node_bbox_from_tag(tag_name, tag) else {
            pos = next_pos;
            continue;
        };
        let id = extract_attr_str(tag, "data-uml-id")
            .or_else(|| extract_attr_str(tag, "id"))
            .or_else(|| nearest_preceding_desc_id(svg, tag_start))
            .unwrap_or_else(|| format!("node@{x},{y}"));

        if w > 0 && h > 0 {
            result.push(NodeBbox { id, x, y, w, h });
        }
        pos = next_pos;
    }
    result
}

fn node_bbox_from_tag(tag_name: &str, tag: &str) -> Option<(i32, i32, i32, i32)> {
    match tag_name {
        "rect" => Some((
            parse_attr_i32_lossy(tag, "x")?,
            parse_attr_i32_lossy(tag, "y")?,
            parse_attr_i32_lossy(tag, "width")?,
            parse_attr_i32_lossy(tag, "height")?,
        )),
        "circle" => {
            let cx = parse_attr_i32_lossy(tag, "cx")?;
            let cy = parse_attr_i32_lossy(tag, "cy")?;
            let r = parse_attr_i32_lossy(tag, "r")?;
            Some((cx - r, cy - r, r * 2, r * 2))
        }
        "ellipse" => {
            let cx = parse_attr_i32_lossy(tag, "cx")?;
            let cy = parse_attr_i32_lossy(tag, "cy")?;
            let rx = parse_attr_i32_lossy(tag, "rx")?;
            let ry = parse_attr_i32_lossy(tag, "ry")?;
            Some((cx - rx, cy - ry, rx * 2, ry * 2))
        }
        "polygon" => parse_points_bbox(tag),
        _ => None,
    }
}

fn parse_points_bbox(tag: &str) -> Option<(i32, i32, i32, i32)> {
    let points = extract_attr_str(tag, "points")?;
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for point in points.split_whitespace() {
        let Some((x, y)) = point.split_once(',') else {
            continue;
        };
        xs.push(x.parse::<f64>().ok()?.round() as i32);
        ys.push(y.parse::<f64>().ok()?.round() as i32);
    }
    let min_x = xs.iter().copied().min()?;
    let max_x = xs.iter().copied().max()?;
    let min_y = ys.iter().copied().min()?;
    let max_y = ys.iter().copied().max()?;
    Some((min_x, min_y, max_x - min_x, max_y - min_y))
}

fn nearest_preceding_desc_id(svg: &str, before_pos: usize) -> Option<String> {
    let prefix = svg.get(..before_pos)?;
    let desc_start = prefix.rfind("<desc ")?;
    let after_desc = &prefix[desc_start..];
    let desc_end = after_desc.find("</desc>")?;
    if desc_start + desc_end + "</desc>".len() != prefix.len() {
        return None;
    }
    extract_attr_str(&after_desc[..desc_end], "data-uml-id")
}

/// Extract a string attribute value from a tag fragment.
///
/// Requires the attribute name to be preceded by a whitespace character to
/// avoid false matches on suffix strings (e.g. `id` inside `data-uml-id`).
pub(super) fn extract_attr_str(tag: &str, attr: &str) -> Option<String> {
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
pub(super) struct Segment {
    pub(super) x1: i32,
    pub(super) y1: i32,
    pub(super) x2: i32,
    pub(super) y2: i32,
}

/// Returns `true` if the axis-aligned segment intersects the rectangle
/// `(bx, by, bw, bh)` (exclusive of the boundary pixels to avoid false
/// positives at port attachment points).
pub(super) fn segment_crosses_rect(seg: Segment, bx: i32, by: i32, bw: i32, bh: i32) -> bool {
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
pub(super) fn extract_relation_segments(svg: &str) -> Vec<(String, String, Vec<Segment>)> {
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

/// Extract package/group header strips from rendered SVG group frames.
///
/// The current SVG backend emits a `rect.uml-group-frame`, a small title tab,
/// and a separator line. Typed group geometry is not available for every
/// renderer yet, so the fallback uses the top 30px as the reserved header
/// strip.
pub fn extract_package_frames(svg: &str) -> Vec<PackageFrame> {
    let mut result = Vec::new();
    let mut pos = 0;
    while let Some(rel) = svg[pos..].find("class=\"uml-group-frame") {
        let abs = pos + rel;
        let tag_start = svg[..abs].rfind('<').unwrap_or(abs);
        let next_pos = abs + "class=\"uml-group-frame".len();
        let Some(rel_close) = svg[tag_start..].find('>') else {
            pos = next_pos;
            continue;
        };
        let tag = &svg[tag_start..tag_start + rel_close];
        let Some(x) = parse_attr_i32_lossy(tag, "x") else {
            pos = next_pos;
            continue;
        };
        let Some(y) = parse_attr_i32_lossy(tag, "y") else {
            pos = next_pos;
            continue;
        };
        let Some(width) = parse_attr_i32_lossy(tag, "width") else {
            pos = next_pos;
            continue;
        };
        let id =
            extract_attr_str(tag, "data-uml-group").unwrap_or_else(|| format!("group@{x},{y}"));
        result.push(PackageFrame {
            id,
            x,
            y,
            width,
            header_height: 30,
        });
        pos = next_pos;
    }
    result
}

/// Parse `points="x1,y1 x2,y2 …"` into a list of `Segment` values.
pub(super) fn parse_polyline_segments(tag: &str) -> Vec<Segment> {
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
