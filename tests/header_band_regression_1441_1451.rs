//! Structural regression tests for #1441 and #1451:
//!
//! **#1441** — `architecture-overview`: stray `uml-edge-label-bg` white rects were
//! generated for package *header* text labels, mottling the dark header bands with
//! white stripes.  Root cause: `check_label_edge_clearance` was processing all text
//! elements (including header labels) when the SVG contained no marked edge labels,
//! and `header_height: 30` was smaller than the actual 48 px header strip, so the
//! "push below header" guard produced a rect that still overlapped the band.
//!
//! **#1451** — `component/08_stereotypes`: the "origin pull" edge-label background
//! rect was placed inside the API Cluster header band (y=160..208) because
//! `header_height: 30` under-counted the 40 px dark rect + 8 px separator.  The rect
//! at y=191 was still inside the band.  After the fix `header_height: 48` the push
//! correctly lands at y=209 (one pixel below the 48 px-tall header strip).

fn render_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("diagram should render without error")
}

/// Parse an integer attribute from an SVG tag fragment.
fn parse_i32(tag: &str, attr: &str) -> Option<i32> {
    let needle = format!("{attr}=\"");
    let pos = tag.find(&needle)?;
    let value_start = pos + needle.len();
    let value_end = value_start + tag[value_start..].find('"')?;
    tag[value_start..value_end].parse().ok()
}

/// Collect all `uml-group-frame` header bands: (x, y, width, header_height=48) from SVG.
fn collect_header_bands(svg: &str) -> Vec<(i32, i32, i32, i32)> {
    let mut bands = Vec::new();
    let mut pos = 0;
    while let Some(rel) = svg[pos..].find("class=\"uml-group-frame") {
        let abs = pos + rel;
        let tag_start = svg[..abs].rfind('<').unwrap_or(abs);
        let close = svg[tag_start..].find('>').unwrap_or(0);
        let tag = &svg[tag_start..tag_start + close];
        if let (Some(x), Some(y), Some(w)) = (
            parse_i32(tag, "x"),
            parse_i32(tag, "y"),
            parse_i32(tag, "width"),
        ) {
            bands.push((x, y, w, 48));
        }
        pos = abs + 1;
    }
    bands
}

/// Collect all `uml-edge-label-bg` rects: (x, y, width, height).
fn collect_bg_rects(svg: &str) -> Vec<(i32, i32, i32, i32)> {
    let mut rects = Vec::new();
    let mut pos = 0;
    while let Some(rel) = svg[pos..].find("class=\"uml-edge-label-bg\"") {
        let abs = pos + rel;
        let tag_start = svg[..abs].rfind('<').unwrap_or(abs);
        let close = svg[tag_start..].find('>').unwrap_or(0);
        let tag = &svg[tag_start..tag_start + close];
        if let (Some(x), Some(y), Some(w), Some(h)) = (
            parse_i32(tag, "x"),
            parse_i32(tag, "y"),
            parse_i32(tag, "width"),
            parse_i32(tag, "height"),
        ) {
            rects.push((x, y, w, h));
        }
        pos = abs + 1;
    }
    rects
}

/// Returns true if rect (rx, ry, rw, rh) overlaps band (bx, by, bw, bh).
#[allow(clippy::too_many_arguments)] // 8 args are 2 paired (x,y,w,h) tuples — a struct would add no clarity
fn overlaps_band(rx: i32, ry: i32, rw: i32, rh: i32, bx: i32, by: i32, bw: i32, bh: i32) -> bool {
    let overlaps_x = rx < bx + bw && rx + rw > bx;
    let overlaps_y = ry < by + bh && ry + rh > by;
    overlaps_x && overlaps_y
}

/// #1441 — architecture-overview: no `uml-edge-label-bg` rect should overlap any
/// package header band.  Before the fix, header labels received white bg rects
/// that mottled the dark navy headers.
#[test]
fn test_arch_overview_no_bg_rect_on_header_bands() {
    let src = include_str!("../docs/diagrams/architecture-overview.puml");
    let svg = render_svg(src);

    let bands = collect_header_bands(&svg);
    let bg_rects = collect_bg_rects(&svg);

    assert!(
        !bands.is_empty(),
        "architecture-overview must contain package frame elements"
    );

    for (rx, ry, rw, rh) in &bg_rects {
        for (bx, by, bw, bh) in &bands {
            assert!(
                !overlaps_band(*rx, *ry, *rw, *rh, *bx, *by, *bw, *bh),
                "uml-edge-label-bg rect at ({rx},{ry} w={rw} h={rh}) overlaps \
                 package header band at ({bx},{by} w={bw} h={bh}). \
                 White bg rects must not mottle dark package headers (#1441)."
            );
        }
    }
}

/// #1451 — component/08_stereotypes: no `uml-edge-label-bg` rect should overlap any
/// package header band.  Before the fix the "origin pull" label bg rect landed at
/// y=191 which was inside the API Cluster header band (y=160..208).
#[test]
fn test_comp08_no_bg_rect_on_header_bands() {
    let src = include_str!("../docs/examples/component/08_cloud_db_queue_stereotypes.puml");
    let svg = render_svg(src);

    let bands = collect_header_bands(&svg);
    let bg_rects = collect_bg_rects(&svg);

    assert!(
        !bands.is_empty(),
        "component/08 must contain package frame elements"
    );

    for (rx, ry, rw, rh) in &bg_rects {
        for (bx, by, bw, bh) in &bands {
            assert!(
                !overlaps_band(*rx, *ry, *rw, *rh, *bx, *by, *bw, *bh),
                "uml-edge-label-bg rect at ({rx},{ry} w={rw} h={rh}) overlaps \
                 package header band at ({bx},{by} w={bw} h={bh}). \
                 Edge-label bg rects must not overlap dark package headers (#1451)."
            );
        }
    }
}
