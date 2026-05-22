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
    assert!(svg.contains("App Edge [203.0.113.10, 2001:db8::10]"));
    assert!(svg.contains("data-nwdiag-addresses=\"203.0.113.10, 2001:db8::10\""));

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

fn fixture(name: &str) -> String {
    format!(
        "{}/tests/fixtures/non_sequence/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

#[derive(Debug, PartialEq, Eq)]
struct SvgRectGeom {
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
        y: svg_attr_i32(tag, "y")?,
    })
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
