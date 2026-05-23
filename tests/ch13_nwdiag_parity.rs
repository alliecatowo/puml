use std::fs;

#[test]
fn nwdiag_peer_links_width_full_and_group_overlay_render() {
    let src = fs::read_to_string(fixture("valid_nwdiag_peer_links_width_full.puml")).unwrap();
    let svg = puml::render_source_to_svg(&src).expect("render nwdiag parity fixture");

    assert_eq!(svg.matches("class=\"nwdiag-peer-link\"").count(), 3);
    assert!(svg.contains("data-nwdiag-peer-a=\"internet\" data-nwdiag-peer-b=\"router\""));
    assert!(svg.contains("data-nwdiag-peer-a=\"router\" data-nwdiag-peer-b=\"switch\""));
    assert!(svg.contains("data-nwdiag-peer-a=\"switch\" data-nwdiag-peer-b=\"printer\""));
    assert!(svg.contains("class=\"nwdiag-node nwdiag-toplevel\""));
    assert!(svg.contains("data-nwdiag-shape=\"cloud\""));
    assert!(svg.contains("group dmz"));
    assert!(svg.contains("App Edge"));
    assert!(svg.contains("class=\"nwdiag-jump-line\" data-nwdiag-node=\"app\""));
    assert!(svg.contains("data-nwdiag-addresses=\"203.0.113.10, 2001:db8::10, 10.0.0.10\""));
    assert_eq!(
        svg_node_rect_count(&svg, "app"),
        1,
        "shared nwdiag node should render as one node box plus jump line"
    );
    assert_eq!(
        svg.matches("data-nwdiag-node=\"app\"").count(),
        3,
        "shared app should keep both network connectors plus the jump line"
    );

    let edge_width = svg_network_width(&svg, "network edge (203.0.113.0/24)").expect("edge width");
    let core_width = svg_network_width(&svg, "network core (10.0.0.0/24)").expect("core width");
    let devices_width = svg_network_width(&svg, "network devices").expect("devices width");
    assert_eq!(edge_width, devices_width);
    assert!(
        edge_width > core_width,
        "width=full should extend selected busbars beyond the default network span"
    );

    let devices_y = svg_network_y(&svg, "network devices").expect("devices y");
    let internet = svg_node_rect(&svg, "internet").expect("internet rect");
    let printer = svg_node_rect(&svg, "printer").expect("printer rect");
    assert!(internet.y > devices_y);
    assert_eq!(internet.y, printer.y);
}

#[test]
fn nwdiag_peer_link_stub_rows_expand_canvas_height() {
    let src = r#"@startnwdiag
nwdiag {
  network lan {
    a;
  }
  a -- b;
}
@endnwdiag
"#;
    let svg = puml::render_source_to_svg(src).expect("render nwdiag peer-link stub");
    let height = svg_root_attr_i32(&svg, "height").expect("svg height");
    let stub = svg_node_rect(&svg, "b").expect("peer-link stub rect");

    assert!(svg.contains("data-nwdiag-peer-a=\"a\" data-nwdiag-peer-b=\"b\""));
    assert!(
        stub.y + 28 < height,
        "peer-link-only stub should stay within the SVG canvas"
    );
}

#[test]
fn nwdiag_shared_node_renders_once_with_jump_line_across_networks() {
    let src = r#"@startnwdiag
nwdiag {
  network public {
    lb;
    web;
  }
  network private {
    lb;
    app;
  }
  network ops {
    lb;
    metrics;
  }
}
@endnwdiag
"#;
    let svg = puml::render_source_to_svg(src).expect("render nwdiag shared node");

    assert_eq!(
        svg_node_rect_count(&svg, "lb"),
        1,
        "shared node should not duplicate a node box per network row"
    );
    assert!(svg.contains("class=\"nwdiag-jump-line\" data-nwdiag-node=\"lb\""));
    assert_eq!(
        svg.matches("data-nwdiag-node=\"lb\"").count(),
        4,
        "shared lb should keep three network connectors plus one jump line"
    );

    let lb = svg_node_rect(&svg, "lb").expect("lb rect");
    let app = svg_node_rect(&svg, "app").expect("app rect");
    let metrics = svg_node_rect(&svg, "metrics").expect("metrics rect");
    assert!(app.x > lb.x);
    assert!(metrics.x > lb.x);
}

#[test]
fn nwdiag_dotted_style_attributes_render_as_dotted_strokes() {
    let src = r##"@startnwdiag
nwdiag {
  internet [shape = cloud, style = "dotted"];
  group edge {
    style = "dotted"
    web;
  }
  network public {
    style = "dotted"
    web [style = "dotted"];
  }
}
@endnwdiag
"##;
    let svg = puml::render_source_to_svg(src).expect("render nwdiag dotted styles");

    assert!(svg.contains("data-nwdiag-style=\"dotted\""));
    assert!(
        svg.matches("stroke-dasharray=\"1 3\"").count() >= 4,
        "dotted style should affect group, network, connector, and node strokes"
    );
    assert!(
        svg.contains("class=\"nwdiag-node nwdiag-toplevel\"")
            && svg.contains("data-nwdiag-name=\"internet\"")
    );
}

fn fixture(name: &str) -> String {
    format!(
        "{}/tests/fixtures/non_sequence/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

#[derive(Debug, PartialEq, Eq)]
struct SvgRectGeom {
    x: i32,
    y: i32,
}

fn svg_network_width(svg: &str, label: &str) -> Option<i32> {
    let text_ix = svg.find(label)?;
    let before_text = &svg[..text_ix];
    let rect_ix = before_text.rfind("<rect class=\"nwdiag-network\"")?;
    let tag = before_text[rect_ix..].split_once('>')?.0;
    svg_attr_i32(tag, "width")
}

fn svg_network_y(svg: &str, label: &str) -> Option<i32> {
    let text_ix = svg.find(label)?;
    let before_text = &svg[..text_ix];
    let rect_ix = before_text.rfind("<rect class=\"nwdiag-network\"")?;
    let tag = before_text[rect_ix..].split_once('>')?.0;
    svg_attr_i32(tag, "y")
}

fn svg_node_rect(svg: &str, name: &str) -> Option<SvgRectGeom> {
    let needle = format!("data-nwdiag-name=\"{name}\"");
    let tag_start = svg.find(&needle)?;
    let rect_start = svg[..tag_start].rfind("<rect ")?;
    let tag = svg[rect_start..].split_once('>')?.0;
    Some(SvgRectGeom {
        x: svg_attr_i32(tag, "x")?,
        y: svg_attr_i32(tag, "y")?,
    })
}

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

fn svg_root_attr_i32(svg: &str, attr: &str) -> Option<i32> {
    let tag = svg.split_once('>')?.0;
    svg_attr_i32(tag, attr)
}

fn svg_attr_i32(tag: &str, attr: &str) -> Option<i32> {
    let needle = format!("{attr}=\"");
    let rest = tag.split_once(&needle)?.1;
    let value = rest.split_once('"')?.0;
    value.parse().ok()
}
