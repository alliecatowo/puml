/// Wave-13 parity tests for nwdiag network diagrams.
///
/// Validates the acceptance criteria:
///   - Horizontal network buses with CIDR address labels at left edge
///   - Host boxes rendered below the bus header (in SVG top-down coords)
///   - Vertical drop-lines from host to each bus it belongs to
///   - IP address labels at bus intersections
///   - Multi-homed hosts (same name on multiple networks) render as ONE box
///     with drop-lines to each network
///
/// Syntax: `@startnwdiag` / `@endnwdiag` (the nwdiag-specific start/end tags).

// ─────────────────────────────────────────────────────────────────────────────
// SVG helper utilities
// ─────────────────────────────────────────────────────────────────────────────

fn svg_attr_i32(tag: &str, attr: &str) -> Option<i32> {
    let needle = format!("{attr}=\"");
    let rest = tag.split_once(&needle)?.1;
    rest.split_once('"')?.0.parse().ok()
}

/// Find the x coordinate of the first `<rect class="nwdiag-network"` element
/// whose following label text contains `label`.
fn svg_network_x(svg: &str, label: &str) -> Option<i32> {
    let text_ix = svg.find(label)?;
    let before = &svg[..text_ix];
    let rect_ix = before.rfind("<rect class=\"nwdiag-network\"")?;
    let tag = before[rect_ix..].split_once('>')?.0;
    svg_attr_i32(tag, "x")
}

/// Find the y coordinate of the first network rect whose label matches.
fn svg_network_y(svg: &str, label: &str) -> Option<i32> {
    let text_ix = svg.find(label)?;
    let before = &svg[..text_ix];
    let rect_ix = before.rfind("<rect class=\"nwdiag-network\"")?;
    let tag = before[rect_ix..].split_once('>')?.0;
    svg_attr_i32(tag, "y")
}

#[allow(dead_code)]
struct SvgNodeRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

/// Find the first physical `<rect class="nwdiag-node` with the given name.
fn svg_node_rect(svg: &str, name: &str) -> Option<SvgNodeRect> {
    let needle = format!("data-nwdiag-name=\"{name}\"");
    let tag_start = svg.find(&needle)?;
    let rect_start = svg[..tag_start].rfind("<rect ")?;
    let tag = svg[rect_start..].split_once('>')?.0;
    Some(SvgNodeRect {
        x: svg_attr_i32(tag, "x")?,
        y: svg_attr_i32(tag, "y")?,
        w: svg_attr_i32(tag, "width")?,
        h: svg_attr_i32(tag, "height")?,
    })
}

/// Count all `<rect class="nwdiag-node` elements whose `data-nwdiag-name` matches.
fn svg_node_rect_count(svg: &str, name: &str) -> usize {
    let needle = format!("data-nwdiag-name=\"{name}\"");
    svg.match_indices("<rect class=\"nwdiag-node")
        .filter(|(ix, _)| {
            svg[*ix..]
                .split_once('>')
                .map(|(tag, _)| tag.contains(&needle))
                .unwrap_or(false)
        })
        .count()
}

/// Count `<line class="nwdiag-connector"` elements attributed to a given network.
fn svg_connector_count_for_network(svg: &str, network: &str) -> usize {
    let needle = format!("data-nwdiag-network=\"{network}\"");
    svg.match_indices("<line class=\"nwdiag-connector\"")
        .filter(|(ix, _)| {
            svg[*ix..]
                .split_once('>')
                .map(|(tag, _)| tag.contains(&needle))
                .unwrap_or(false)
        })
        .count()
}

/// Count `<line class="nwdiag-connector"` elements attributed to a given node.
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
// Test 1 — single-network diagram: horizontal bus bar with CIDR label at left
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn nwdiag_network_renders_horizontal_bus_with_cidr_label() {
    let src = r#"@startnwdiag
nwdiag {
  network dmz {
    address = "10.0.0.0/24"
    web01 [address = "10.0.0.10"]
    web02 [address = "10.0.0.11"]
  }
}
@endnwdiag
"#;
    let svg = puml::render_source_to_svg(src).expect("nwdiag render");

    // The horizontal bus bar rect must appear in the SVG.
    assert!(
        svg.contains("class=\"nwdiag-network\""),
        "expected nwdiag-network class in SVG"
    );

    // The label combines the network name and CIDR.
    let label = "network dmz (10.0.0.0/24)";
    assert!(
        svg.contains(label),
        "expected CIDR label '{label}' in SVG; got:\n{svg}"
    );

    // The network rect should start near the left edge of the canvas.
    let net_x = svg_network_x(&svg, label).expect("network x coordinate");
    assert!(
        net_x < 200,
        "network bus should start near the left edge, got x={net_x}"
    );

    // Two connector (drop-line) elements — one per host.
    let connectors = svg_connector_count_for_network(&svg, "dmz");
    assert_eq!(
        connectors, 2,
        "expected 2 connector lines for dmz, got {connectors}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2 — host box is drawn below the bus header, with a drop-line
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn nwdiag_host_renders_box_above_bus_with_drop_line() {
    let src = r#"@startnwdiag
nwdiag {
  network lan {
    address = "192.168.0.0/24"
    server [address = "192.168.0.1"]
  }
}
@endnwdiag
"#;
    let svg = puml::render_source_to_svg(src).expect("nwdiag render");

    // The host node box must be present.
    let host = svg_node_rect(&svg, "server").expect("server node rect");

    // In nwdiag layout: bus header rect at y, bus bar at y+24, node boxes at
    // y+24+30. So the host y coordinate must be strictly greater than the
    // network header rect y.
    let label = "network lan (192.168.0.0/24)";
    let bus_y = svg_network_y(&svg, label).expect("lan bus y");
    assert!(
        host.y > bus_y,
        "host box y ({}) should be below the network header rect y ({bus_y})",
        host.y
    );

    // One drop-line connector should connect the host to the bus.
    let connectors = svg_connector_count_for_node(&svg, "server");
    assert_eq!(
        connectors, 1,
        "expected 1 drop-line connector for 'server', got {connectors}"
    );

    // The host box must have positive dimensions.
    assert!(host.w > 0, "host box width should be positive");
    assert!(host.h > 0, "host box height should be positive");
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3 — multi-homed host renders as ONE box with a drop-line to each network
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn nwdiag_host_on_multiple_networks_renders_drop_to_each() {
    let src = r#"@startnwdiag
nwdiag {
  network dmz {
    address = "10.0.0.0/24"
    web01 [address = "10.0.0.10"]
    web02 [address = "10.0.0.11"]
    web03 [address = "10.0.0.12"]
  }
  network internal {
    address = "192.168.1.0/24"
    web01  [address = "192.168.1.10"]
    web02  [address = "192.168.1.11"]
    db01   [address = "192.168.1.100"]
  }
  network management {
    address = "172.16.0.0/24"
    db01   [address = "172.16.0.100"]
    backup [address = "172.16.0.101"]
  }
}
@endnwdiag
"#;
    let svg = puml::render_source_to_svg(src).expect("nwdiag render");

    // web01 appears in dmz and internal — must render as exactly ONE node box.
    assert_eq!(
        svg_node_rect_count(&svg, "web01"),
        1,
        "web01 appears in 2 networks but must render as exactly one node box"
    );

    // db01 appears in internal and management — exactly ONE box.
    assert_eq!(
        svg_node_rect_count(&svg, "db01"),
        1,
        "db01 appears in 2 networks but must render as exactly one node box"
    );

    // web01 must have connector lines to both dmz and internal.
    let web01_connectors = svg_connector_count_for_node(&svg, "web01");
    assert!(
        web01_connectors >= 2,
        "web01 should have drop-lines to at least 2 networks, got {web01_connectors}"
    );

    // db01 must have connector lines to both internal and management.
    let db01_connectors = svg_connector_count_for_node(&svg, "db01");
    assert!(
        db01_connectors >= 2,
        "db01 should have drop-lines to at least 2 networks, got {db01_connectors}"
    );

    // A jump-line should exist for multi-homed nodes spanning multiple rows.
    assert!(
        svg.contains("class=\"nwdiag-jump-line\" data-nwdiag-node=\"web01\"")
            || svg.contains("class=\"nwdiag-jump-line\" data-nwdiag-node=\"db01\""),
        "expected nwdiag-jump-line for at least one multi-homed node"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4 — IP address labels appear in the SVG at the bus intersection
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn nwdiag_ip_address_renders_at_bus_intersection() {
    // Use two networks so both hosts are multi-homed (which triggers the
    // nwdiag-address label text elements near connector lines).
    let src = r#"@startnwdiag
nwdiag {
  network public {
    address = "10.1.0.0/24"
    router [address = "10.1.0.1"]
    switch [address = "10.1.0.2"]
  }
  network private {
    address = "10.2.0.0/24"
    router [address = "10.2.0.1"]
    switch [address = "10.2.0.2"]
  }
}
@endnwdiag
"#;
    let svg = puml::render_source_to_svg(src).expect("nwdiag render");

    // Each assigned IP address must appear somewhere in the rendered SVG.
    for ip in ["10.1.0.1", "10.1.0.2", "10.2.0.1", "10.2.0.2"] {
        assert!(svg.contains(ip), "expected address '{ip}' in rendered SVG");
    }

    // For multi-homed nodes, the renderer emits per-connector address labels
    // using class="nwdiag-address".
    assert!(
        svg.contains("class=\"nwdiag-address\""),
        "expected class=\"nwdiag-address\" text elements for IP labels at bus intersections"
    );

    // The data-nwdiag-addresses attribute on node rects must carry the addresses.
    // For multi-homed nodes, the addresses attribute holds the combined list.
    assert!(
        svg.contains("data-nwdiag-addresses=\"10.1.0.1"),
        "expected data-nwdiag-addresses containing router's public IP"
    );
    assert!(
        svg.contains("data-nwdiag-addresses=\"10.1.0.2"),
        "expected data-nwdiag-addresses containing switch's public IP"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5 — full three-network topology with multi-homed hosts
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn nwdiag_three_network_topology_with_multihomed_host() {
    let src = r#"@startnwdiag
nwdiag {
  network dmz {
    address = "10.0.0.0/24"
    web01  [address = "10.0.0.10"]
    web02  [address = "10.0.0.11"]
    web03  [address = "10.0.0.12"]
  }
  network internal {
    address = "192.168.1.0/24"
    web01  [address = "192.168.1.10"]
    web02  [address = "192.168.1.11"]
    db01   [address = "192.168.1.100"]
  }
  network management {
    address = "172.16.0.0/24"
    db01   [address = "172.16.0.100"]
    backup [address = "172.16.0.101"]
  }
}
@endnwdiag
"#;

    // ── Validate the normalized model ─────────────────────────────────────────
    let parsed = puml::parse(src).expect("parse nwdiag three-network fixture");
    let model = puml::normalize_family(parsed).expect("normalize nwdiag three-network fixture");
    let puml::NormalizedDocument::Nwdiag(ref doc) = model else {
        panic!("expected Nwdiag normalized document");
    };

    assert_eq!(doc.networks.len(), 3, "expected 3 networks in model");

    let dmz = doc
        .networks
        .iter()
        .find(|n| n.name == "dmz")
        .expect("dmz network");
    assert_eq!(
        dmz.address.as_deref(),
        Some("10.0.0.0/24"),
        "dmz CIDR must be preserved in the model"
    );
    assert_eq!(dmz.nodes.len(), 3, "dmz should have 3 hosts");

    let internal = doc
        .networks
        .iter()
        .find(|n| n.name == "internal")
        .expect("internal network");
    assert_eq!(
        internal.address.as_deref(),
        Some("192.168.1.0/24"),
        "internal CIDR must be preserved"
    );

    // web01 must appear in both dmz and internal.
    assert!(
        dmz.nodes.iter().any(|n| n.name == "web01"),
        "web01 must be a member of dmz"
    );
    assert!(
        internal.nodes.iter().any(|n| n.name == "web01"),
        "web01 must be a member of internal"
    );

    let management = doc
        .networks
        .iter()
        .find(|n| n.name == "management")
        .expect("management network");

    // db01 must appear in both internal and management.
    assert!(
        internal.nodes.iter().any(|n| n.name == "db01"),
        "db01 must be a member of internal"
    );
    assert!(
        management.nodes.iter().any(|n| n.name == "db01"),
        "db01 must be a member of management"
    );

    // ── Validate the rendered SVG ─────────────────────────────────────────────
    let artifact = puml::render::render_nwdiag_artifact(doc);
    let svg = &artifact.svg;

    // All three CIDR labels must appear.
    assert!(
        svg.contains("network dmz (10.0.0.0/24)"),
        "dmz CIDR label missing from SVG"
    );
    assert!(
        svg.contains("network internal (192.168.1.0/24)"),
        "internal CIDR label missing from SVG"
    );
    assert!(
        svg.contains("network management (172.16.0.0/24)"),
        "management CIDR label missing from SVG"
    );

    // All 5 unique hosts must appear exactly once as physical node boxes.
    for host in ["web01", "web02", "web03", "db01", "backup"] {
        assert_eq!(
            svg_node_rect_count(svg, host),
            1,
            "host '{host}' must appear exactly once as a physical node box"
        );
    }

    // Network vertical ordering in the SVG (dmz → internal → management).
    let dmz_y = svg_network_y(svg, "network dmz (10.0.0.0/24)").expect("dmz y");
    let internal_y = svg_network_y(svg, "network internal (192.168.1.0/24)").expect("internal y");
    let management_y =
        svg_network_y(svg, "network management (172.16.0.0/24)").expect("management y");
    assert!(
        dmz_y < internal_y,
        "dmz (y={dmz_y}) should appear above internal (y={internal_y})"
    );
    assert!(
        internal_y < management_y,
        "internal (y={internal_y}) should appear above management (y={management_y})"
    );

    // Multi-homed nodes must have at least 2 drop-line connectors.
    for host in ["web01", "web02"] {
        let count = svg_connector_count_for_node(svg, host);
        assert!(
            count >= 2,
            "multi-homed host '{host}' must have at least 2 connectors, got {count}"
        );
    }
    let db01_count = svg_connector_count_for_node(svg, "db01");
    assert!(
        db01_count >= 2,
        "multi-homed host 'db01' must have at least 2 connectors, got {db01_count}"
    );

    // All assigned IP addresses must appear in the SVG.
    for ip in [
        "10.0.0.10",
        "10.0.0.11",
        "10.0.0.12",
        "192.168.1.10",
        "192.168.1.11",
        "192.168.1.100",
        "172.16.0.100",
        "172.16.0.101",
    ] {
        assert!(
            svg.contains(ip),
            "expected IP address '{ip}' in rendered SVG"
        );
    }

    // ── Validate the typed scene ──────────────────────────────────────────────
    let scene = artifact.scene.as_ref().expect("typed nwdiag scene");
    assert!(
        scene.lanes.contains_key("nwdiag:network:dmz"),
        "scene must have a lane for dmz"
    );
    assert!(
        scene.lanes.contains_key("nwdiag:network:internal"),
        "scene must have a lane for internal"
    );
    assert!(
        scene.lanes.contains_key("nwdiag:network:management"),
        "scene must have a lane for management"
    );

    // Scene geometry must be clean.
    let issues = scene.validate_geometry();
    assert!(
        issues.is_empty(),
        "nwdiag scene geometry must have no violations: {issues:?}"
    );
}
