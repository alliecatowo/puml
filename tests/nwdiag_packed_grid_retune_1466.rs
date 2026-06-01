//! Packed-grid density retune tests for nwdiag (#1466).
//!
//! Validates the acceptance criteria from issue #1466:
//!   - `docs/examples/nwdiag/02_multi_network.puml` area ratio ≤ 1.5× PlantUML
//!   - Canvas height reduced (sub-300px for 3-network fixture)
//!   - Node boxes are compact (≤ 25px tall for single-line labels)
//!   - No content overlap; subnet bars and nodes remain readable
//!   - Multi-homed nodes still render with correct drop-lines and jump-lines

// ─────────────────────────────────────────────────────────────────────────────
// SVG helper utilities (mirrored from nwdiag_w13_parity.rs)
// ─────────────────────────────────────────────────────────────────────────────

fn svg_attr_i32(tag: &str, attr: &str) -> Option<i32> {
    let needle = format!("{attr}=\"");
    let rest = tag.split_once(&needle)?.1;
    rest.split_once('"')?.0.parse().ok()
}

/// Extract the SVG canvas height from the root `<svg` element.
fn svg_canvas_height(svg: &str) -> Option<i32> {
    let tag = svg.split_once('>')?.0;
    svg_attr_i32(tag, "height")
}

/// Extract the SVG canvas width from the root `<svg` element.
fn svg_canvas_width(svg: &str) -> Option<i32> {
    let tag = svg.split_once('>')?.0;
    svg_attr_i32(tag, "width")
}

/// Find the first physical node rect for the given name.
fn svg_node_rect_h(svg: &str, name: &str) -> Option<i32> {
    let needle = format!("data-nwdiag-name=\"{name}\"");
    let tag_start = svg.find(&needle)?;
    let rect_start = svg[..tag_start].rfind("<rect ")?;
    let tag = svg[rect_start..].split_once('>')?.0;
    svg_attr_i32(tag, "height")
}

/// Count connector lines for a given node name.
fn svg_connector_count_for_node(svg: &str, node_name: &str) -> usize {
    let needle = format!("data-nwdiag-node=\"{node_name}\"");
    svg.match_indices("<line class=\"nwdiag-connector\"")
        .filter(|(ix, _)| {
            svg[*ix..]
                .split_once('>')
                .map(|(tag, _)| tag.contains(&needle))
                .unwrap_or(false)
        })
        .count()
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 1 — 02_multi_network area-ratio guard (≤ 1.5×)
// ─────────────────────────────────────────────────────────────────────────────

/// Area ratio guard: PUML canvas area must be ≤ 1.5× the PlantUML reference area.
///
/// PlantUML 1.2026.x renders 02_multi_network at approximately 530 × 203 = 107,590 px².
/// We compare against this reference area with a 1.5× tolerance.
#[test]
fn nwdiag_02_multi_network_area_ratio_le_1_5x() {
    let src = include_str!("../docs/examples/nwdiag/02_multi_network.puml");
    let svg = puml::render_source_to_svg(src).expect("nwdiag render");

    let width = svg_canvas_width(&svg).expect("canvas width") as i64;
    let height = svg_canvas_height(&svg).expect("canvas height") as i64;
    let puml_area = width * height;

    // PlantUML 1.2026.x reference area for this fixture: ~107,590 px²
    let plantuml_reference_area: i64 = 107_590;
    let ratio = puml_area as f64 / plantuml_reference_area as f64;

    assert!(
        ratio <= 1.5,
        "PUML area {puml_area} ({width}×{height}) is {ratio:.2}× the PlantUML reference \
         area ({plantuml_reference_area}); must be ≤ 1.5×"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2 — canvas height sub-300px for a 3-network fixture
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn nwdiag_02_multi_network_canvas_height_compact() {
    let src = include_str!("../docs/examples/nwdiag/02_multi_network.puml");
    let svg = puml::render_source_to_svg(src).expect("nwdiag render");

    let height = svg_canvas_height(&svg).expect("canvas height");
    assert!(
        height < 300,
        "canvas height {height}px should be < 300px for 3-network fixture after retune"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3 — node boxes are compact (single-line label ≤ 25px tall)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn nwdiag_single_line_node_height_compact() {
    let src = r#"@startnwdiag
nwdiag {
  network lan {
    address = "10.0.0.0/24"
    server;
    client;
  }
}
@endnwdiag
"#;
    let svg = puml::render_source_to_svg(src).expect("nwdiag render");

    let server_h = svg_node_rect_h(&svg, "server").expect("server height");
    assert!(
        server_h <= 25,
        "single-line node height {server_h}px should be ≤ 25px after packed-grid retune"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4 — all content visible: labels, addresses, connectors
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn nwdiag_02_all_content_visible() {
    let src = include_str!("../docs/examples/nwdiag/02_multi_network.puml");
    let svg = puml::render_source_to_svg(src).expect("nwdiag render");

    // All network labels must be present.
    assert!(
        svg.contains("public (203.0.113.0/24)"),
        "public CIDR label missing"
    );
    assert!(
        svg.contains("private (10.0.0.0/24)"),
        "private CIDR label missing"
    );
    assert!(
        svg.contains("ops (172.16.0.0/24)"),
        "ops CIDR label missing"
    );

    // All IP addresses must appear.
    for ip in ["203.0.113.10", "10.0.0.10", "172.16.0.10"] {
        assert!(svg.contains(ip), "IP address '{ip}' missing from SVG");
    }

    // Multi-homed 'api' node must have connectors to all 3 networks.
    let api_connectors = svg_connector_count_for_node(&svg, "api");
    assert!(
        api_connectors >= 3,
        "api node must have connectors to 3 networks, got {api_connectors}"
    );

    // Jump-line for multi-homed api must be present.
    assert!(
        svg.contains("class=\"nwdiag-jump-line\" data-nwdiag-node=\"api\""),
        "jump-line for multi-homed api node missing"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5 — canvas width is reduced from 760px floor to 520px floor
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn nwdiag_canvas_min_width_reduced() {
    // A minimal one-node, one-network diagram should not force a 760px canvas.
    let src = r#"@startnwdiag
nwdiag {
  network dmz {
    address = "10.0.0.0/24"
    web;
  }
}
@endnwdiag
"#;
    let svg = puml::render_source_to_svg(src).expect("nwdiag render");

    let width = svg_canvas_width(&svg).expect("canvas width");
    assert!(
        width <= 520,
        "minimal nwdiag canvas width {width}px should be ≤ 520px (packed-grid retune)"
    );
}
