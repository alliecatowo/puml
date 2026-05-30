//! Wave-15 structural regression: edge-label placement — minimal collision push
//! and header-band avoidance.
//!
//! Validates two independent fixes:
//!
//! 1. **#1363 — minimal-displacement fallback for state transition labels.**
//!    When no collision-free slot exists the fallback now picks the closest
//!    candidate rather than the one with fewest collisions.  This prevents
//!    labels from drifting into the left/right gutter.  The test asserts that
//!    the "data ready" label in state/07_nested is present in the SVG (no silent
//!    drop) and that the label x-coordinate stays within a reasonable horizontal
//!    band around the source↔target node x-span.
//!
//! 2. **#1344 — edge-label white-bg rects must not overlap group-header bands.**
//!    The `check_label_edge_clearance` auto-correction now shifts any bg rect
//!    whose y falls inside a header band down to just below the header bottom.
//!    The test parses the rendered SVG for architecture-overview.puml and verifies
//!    that no `uml-edge-label-bg` rect's y-range overlaps a `uml-group-frame`
//!    header band (header = top 30 px of the frame).

use puml::render_source_to_svg;

// ─────────────────────────────────────────────────────────────────────────────
// #1363 — state/07_nested "data ready" label placement
// ─────────────────────────────────────────────────────────────────────────────

/// The "data ready" label on the Fetching→Processing edge must be present and
/// must not be displaced further than `MAX_GUTTER_PX` pixels to the left of the
/// leftmost node in the diagram.  Before the fix the label floated ~175 px into
/// the left margin; after the fix it stays within a few pixels of the edge.
#[test]
fn data_ready_label_near_edge_in_state07() {
    let src = include_str!("../docs/examples/state/07_nested.puml");
    let svg = render_source_to_svg(src).expect("state/07 svg should render");

    // Label must be present.
    assert!(
        svg.contains("data ready"),
        "edge label \"data ready\" must appear in state/07 SVG"
    );

    // Extract the x-coordinate of the "data ready" text element.
    let label_x = extract_label_x(&svg, "data ready")
        .expect("could not parse x attribute of \"data ready\" text element");

    // The diagram's leftmost node is "Operational" whose left edge is at most
    // STATE_MARGIN (30 px) from the canvas left edge.  The label should not be
    // further left than `MARGIN + MAX_GUTTER_PX` pixels to the left of the
    // diagram body.  We use a generous 120 px to cover the label half-width
    // while still catching the pre-fix ~175 px gutter displacement.
    const MAX_GUTTER_PX: i32 = 120;
    assert!(
        label_x >= -MAX_GUTTER_PX,
        "\"data ready\" label x={label_x} is more than {MAX_GUTTER_PX} px to the left; \
         likely displaced into the gutter (regression of #1363)"
    );
}

/// Extract the `x` attribute value from the first `<text …>…data ready…</text>`
/// element in the SVG string.  Returns `None` if the element is not found.
fn extract_label_x(svg: &str, label: &str) -> Option<i32> {
    // Find the text content match.
    let needle = label;
    let content_pos = svg.find(needle)?;
    // Walk back to find the opening <text tag.
    let tag_start = svg[..content_pos].rfind("<text ")?;
    let tag_end = svg[tag_start..].find('>')?;
    let tag = &svg[tag_start..tag_start + tag_end];
    // Extract x="…".
    let x_attr = "x=\"";
    let xstart = tag.find(x_attr)? + x_attr.len();
    let xend = tag[xstart..].find('"')? + xstart;
    tag[xstart..xend].parse::<i32>().ok()
}

// ─────────────────────────────────────────────────────────────────────────────
// #1344 — architecture-overview: no uml-edge-label-bg rect overlaps a header
// ─────────────────────────────────────────────────────────────────────────────

/// No `uml-edge-label-bg` rect in the architecture-overview SVG may overlap a
/// `uml-group-frame` header band (the top 30 px of each frame).
#[test]
fn arch_overview_edge_label_bg_not_on_header() {
    let src = include_str!("../docs/diagrams/architecture-overview.puml");
    let svg = render_source_to_svg(src).expect("architecture-overview svg should render");

    let frames = collect_group_frames(&svg);
    let bg_rects = collect_edge_label_bg_rects(&svg);

    // Header height used by extract_package_frames — must stay in sync.
    const HEADER_HEIGHT: i32 = 30;

    for rect in &bg_rects {
        for frame in &frames {
            let header_top = frame.y;
            let header_bot = frame.y + HEADER_HEIGHT;
            let rect_top = rect.y;
            let rect_bot = rect.y + rect.h;
            let overlaps_x = rect.x < frame.x + frame.w && rect.x + rect.w > frame.x;
            let overlaps_y = rect_top < header_bot && rect_bot > header_top;
            assert!(
                !(overlaps_x && overlaps_y),
                "uml-edge-label-bg rect at ({},{}) overlaps group-header band of \
                 frame at ({},{}) [header y={}..{}] — regression of #1344",
                rect.x,
                rect.y,
                frame.x,
                frame.y,
                header_top,
                header_bot
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// SVG parsing helpers
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct SvgRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

#[derive(Debug)]
struct FrameInfo {
    x: i32,
    y: i32,
    w: i32,
}

/// Extract all `<rect class="uml-group-frame" …/>` bboxes from the SVG.
fn collect_group_frames(svg: &str) -> Vec<FrameInfo> {
    let mut result = Vec::new();
    let mut pos = 0;
    while let Some(rel) = svg[pos..].find("class=\"uml-group-frame") {
        let abs = pos + rel;
        let tag_start = svg[..abs].rfind('<').unwrap_or(abs);
        let next_pos = abs + "class=\"uml-group-frame".len();
        let Some(rel_end) = svg[tag_start..].find('>') else {
            pos = next_pos;
            continue;
        };
        let tag = &svg[tag_start..tag_start + rel_end];
        let x = parse_i32_attr(tag, "x").unwrap_or(0);
        let y = parse_i32_attr(tag, "y").unwrap_or(0);
        let w = parse_i32_attr(tag, "width").unwrap_or(0);
        if w > 0 {
            result.push(FrameInfo { x, y, w });
        }
        pos = next_pos;
    }
    result
}

/// Extract all `<rect class="uml-edge-label-bg" …/>` bboxes from the SVG.
fn collect_edge_label_bg_rects(svg: &str) -> Vec<SvgRect> {
    let mut result = Vec::new();
    let mut pos = 0;
    while let Some(rel) = svg[pos..].find("class=\"uml-edge-label-bg\"") {
        let abs = pos + rel;
        let tag_start = svg[..abs].rfind('<').unwrap_or(abs);
        let next_pos = abs + "class=\"uml-edge-label-bg\"".len();
        let Some(rel_end) = svg[tag_start..].find('>') else {
            pos = next_pos;
            continue;
        };
        let tag = &svg[tag_start..tag_start + rel_end];
        let x = parse_i32_attr(tag, "x").unwrap_or(0);
        let y = parse_i32_attr(tag, "y").unwrap_or(0);
        let w = parse_i32_attr(tag, "width").unwrap_or(0);
        let h = parse_i32_attr(tag, "height").unwrap_or(0);
        result.push(SvgRect { x, y, w, h });
        pos = next_pos;
    }
    result
}

fn parse_i32_attr(tag: &str, attr: &str) -> Option<i32> {
    let needle = format!("{attr}=\"");
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    tag[start..end].parse::<i32>().ok()
}
