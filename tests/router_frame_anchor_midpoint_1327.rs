//! Regression test for issue #1327: edges entering package-framed nodes must
//! anchor on the component bbox top/bottom-edge midpoint, not the package frame
//! top-left corner.
//!
//! Root cause: `box_grid_edges.rs` computed `tgt_keep_routed_x` using
//! `pick_port`'s y2, which is the component's CENTER y for horizontal-dominant
//! edges.  That value never falls within 16 px of the top or bottom edge, so the
//! condition was always false and the snapping replaced the router's correct
//! top-edge midpoint with the left/right-edge port from `pick_port`.
//!
//! Fix: compute `tgt_keep_routed_x` from the router's own last waypoint y so
//! that top/bottom entry is detected reliably even when `pick_port` chose a
//! horizontal port.

use puml::render_source_to_svg;

/// Extract all `points="…"` values for polyline/line edges matching the given
/// `data-uml-from` / `data-uml-to` pair from the SVG string.
fn edge_points(svg: &str, from: &str, to: &str) -> Vec<Vec<(i32, i32)>> {
    let from_attr = format!("data-uml-from=\"{from}\"");
    let to_attr = format!("data-uml-to=\"{to}\"");
    let mut results = Vec::new();
    let mut search_start = 0;
    while let Some(tag_start) = svg[search_start..].find('<').map(|i| i + search_start) {
        let Some(tag_end) = svg[tag_start..].find("/>").map(|i| i + tag_start + 2) else {
            break;
        };
        let tag = &svg[tag_start..tag_end];
        if tag.contains(&from_attr) && tag.contains(&to_attr) {
            if let Some(pts) = extract_points(tag) {
                results.push(pts);
            }
        }
        search_start = tag_end;
    }
    results
}

/// Parse `points="x1,y1 x2,y2 …"` from a tag string.
fn extract_points(tag: &str) -> Option<Vec<(i32, i32)>> {
    let start = tag.find("points=\"")? + "points=\"".len();
    let end = tag[start..].find('"')? + start;
    let raw = &tag[start..end];
    let pts: Vec<(i32, i32)> = raw
        .split_whitespace()
        .filter_map(|pair| {
            let mut it = pair.split(',');
            let x = it.next()?.parse::<i32>().ok()?;
            let y = it.next()?.parse::<i32>().ok()?;
            Some((x, y))
        })
        .collect();
    (!pts.is_empty()).then_some(pts)
}

/// Extract the axis-aligned bounding box of a node by its label text.
/// Returns (x, y, w, h) or None if not found.
fn node_bbox(svg: &str, label: &str) -> Option<(i32, i32, i32, i32)> {
    // The node rect precedes the label text; find the <desc…> that precedes
    // the relevant <rect class="uml-node" …> and read x, y, width, height.
    let label_needle = format!(">{label}</text>");
    let text_pos = svg.find(&label_needle)?;
    // Walk backwards to find the most recent <rect class="uml-node …
    let before = &svg[..text_pos];
    let rect_start = before.rfind("<rect class=\"uml-node")?;
    let rect_end = svg[rect_start..].find("/>").map(|i| i + rect_start + 2)?;
    let rect_tag = &svg[rect_start..rect_end];
    let x = parse_i32_attr(rect_tag, " x=\"")?;
    let y = parse_i32_attr(rect_tag, " y=\"")?;
    let w = parse_i32_attr(rect_tag, "width=\"")?;
    let h = parse_i32_attr(rect_tag, "height=\"")?;
    Some((x, y, w, h))
}

fn parse_i32_attr(tag: &str, needle: &str) -> Option<i32> {
    let start = tag.find(needle)? + needle.len();
    let end = tag[start..].find('"')? + start;
    tag[start..end].parse().ok()
}

const SRC: &str = include_str!("../docs/diagrams/architecture-overview.puml");

/// Verify that edges entering package-framed nodes terminate on the component's
/// top or bottom edge, not on the package frame corner.
///
/// For each edge, the last waypoint x must lie within the target component's
/// x range [tx, tx+tw] and the last waypoint y must be within 4 px of the
/// component's top edge (ty) or bottom edge (ty+th).
#[test]
fn edges_entering_framed_nodes_anchor_to_component_bbox_edge() {
    let svg = render_source_to_svg(SRC)
        .expect("architecture-overview.puml should render without error");

    // Edges that cross into package-framed components.
    // (from_alias, to_alias, target_label_text)
    let cases: &[(&str, &str, &str)] = &[
        ("CLI", "Frontends", "Adapters"),
        ("CLI", "Preproc", "Preprocessor"),
        ("LSP", "LangSvc", "Language Service"),
        ("WASM", "LangSvc", "Language Service"),
        ("Frontends", "Parser", "Parser"),
        ("Preproc", "Parser", "Parser"),
        ("LangSvc", "Parser", "Parser"),
    ];

    for &(from, to, label) in cases {
        let paths = edge_points(&svg, from, to);
        assert!(
            !paths.is_empty(),
            "expected at least one edge from {from} to {to}",
        );

        let (tx, ty, tw, th) = node_bbox(&svg, label)
            .unwrap_or_else(|| panic!("could not find bbox for node '{label}'"));

        for pts in &paths {
            let (lx, ly) = *pts.last().expect("path has at least one point");
            assert!(
                lx >= tx && lx <= tx + tw,
                "edge {from}→{to} last x={lx} is outside target component \
                 x-range [{tx}, {}] (label={label}); expected midpoint on component bbox, \
                 not package frame corner",
                tx + tw,
            );
            let on_top = (ly - ty).abs() <= 4;
            let on_bottom = (ly - (ty + th)).abs() <= 4;
            assert!(
                on_top || on_bottom,
                "edge {from}→{to} last y={ly} is not within 4px of component \
                 top ({ty}) or bottom ({}) (label={label}); expected bbox edge midpoint",
                ty + th,
            );
        }
    }
}
