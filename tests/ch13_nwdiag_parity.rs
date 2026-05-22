use puml::{normalize_family, parse, render_source_to_svg};

#[test]
fn nwdiag_peer_links_outside_network_blocks_normalize_and_render() {
    let source = r#"@startnwdiag
nwdiag {
  inet [shape = cloud];
  network dmz {
    address = "10.0.0.0/24"
    router [address = "10.0.0.1"];
  }
  inet -- router;
}
@endnwdiag
"#;

    let parsed = parse(source).expect("nwdiag source should parse");
    let model = normalize_family(parsed).expect("nwdiag model should normalize");
    let puml::model::NormalizedDocument::Nwdiag(doc) = model else {
        panic!("expected nwdiag document");
    };

    assert_eq!(doc.peer_nodes.len(), 1);
    assert_eq!(doc.peer_nodes[0].name, "inet");
    assert_eq!(doc.peer_nodes[0].shape.as_deref(), Some("cloud"));
    assert_eq!(doc.peer_links.len(), 1);
    assert_eq!(doc.peer_links[0].from, "inet");
    assert_eq!(doc.peer_links[0].to, "router");

    let svg = render_source_to_svg(source).expect("nwdiag should render");
    assert!(svg.contains("class=\"nwdiag-peer-link\""));
    assert!(svg.contains("data-nwdiag-from=\"inet\""));
    assert!(svg.contains("data-nwdiag-to=\"router\""));
    assert!(svg.contains("data-nwdiag-name=\"inet\""));
    assert!(svg.contains("data-nwdiag-shape=\"cloud\""));
}

#[test]
fn nwdiag_chained_peer_links_and_groups_render_together() {
    let source = r#"@startnwdiag
nwdiag {
  group edge {
    switch;
    equip;
  }
  switch -- equip -- printer;
}
@endnwdiag
"#;

    let parsed = parse(source).expect("nwdiag source should parse");
    let model = normalize_family(parsed).expect("nwdiag model should normalize");
    let puml::model::NormalizedDocument::Nwdiag(doc) = model else {
        panic!("expected nwdiag document");
    };

    assert_eq!(doc.peer_links.len(), 2);
    assert!(doc.peer_nodes.iter().any(|node| node.name == "switch"));
    assert!(doc.peer_nodes.iter().any(|node| node.name == "equip"));
    assert!(doc.peer_nodes.iter().any(|node| node.name == "printer"));

    let svg = render_source_to_svg(source).expect("nwdiag should render");
    assert_eq!(svg.matches("class=\"nwdiag-peer-link\"").count(), 2);
    assert!(svg.contains("class=\"nwdiag-group\""));
    assert!(svg.contains("group edge"));
}

#[test]
fn nwdiag_network_width_full_extends_busbar_to_shared_span() {
    let source = r#"@startnwdiag
nwdiag {
  network left {
    a;
  }
  network right {
    width = full
    b;
    c;
  }
}
@endnwdiag
"#;

    let parsed = parse(source).expect("nwdiag source should parse");
    let model = normalize_family(parsed).expect("nwdiag model should normalize");
    let puml::model::NormalizedDocument::Nwdiag(doc) = model else {
        panic!("expected nwdiag document");
    };

    assert!(!doc.networks[0].width_full);
    assert!(doc.networks[1].width_full);

    let svg = render_source_to_svg(source).expect("nwdiag should render");
    let auto_width = network_bar_width(&svg, "auto");
    let full_width = network_bar_width(&svg, "full");

    assert!(
        full_width > auto_width,
        "full-width busbar should extend farther"
    );
}

fn network_bar_width(svg: &str, mode: &str) -> i32 {
    svg.lines()
        .find(|line| {
            line.contains("class=\"nwdiag-network\"")
                && line.contains("height=\"12\"")
                && line.contains(&format!("data-nwdiag-width-mode=\"{mode}\""))
        })
        .and_then(|line| extract_numeric_attr(line, "width"))
        .expect("expected nwdiag busbar width")
}

fn extract_numeric_attr(tag: &str, attr: &str) -> Option<i32> {
    let needle = format!("{attr}=\"");
    let start = tag.find(&needle)? + needle.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    rest[..end].parse::<i32>().ok()
}
