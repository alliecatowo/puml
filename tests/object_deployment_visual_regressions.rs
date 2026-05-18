fn attr_value_in_tag(haystack: &str, marker: &str, attr: &str) -> i32 {
    let marker_idx = haystack.find(marker).expect("marker should exist");
    let tag_start = haystack[..=marker_idx]
        .rfind('<')
        .expect("tag start should exist");
    let tag_end = haystack[marker_idx..]
        .find('>')
        .map(|idx| marker_idx + idx)
        .expect("tag end should exist");
    let tag = &haystack[tag_start..=tag_end];
    let needle = format!("{attr}=\"");
    let attr_start = tag.find(&needle).expect("attribute should exist") + needle.len();
    let rest = &tag[attr_start..];
    let end = rest.find('"').expect("attribute should terminate");
    rest[..end]
        .parse::<i32>()
        .expect("attribute should parse as i32")
}

#[test]
fn object_relation_labels_keep_clear_gap_from_adjacent_target_boxes() {
    let svg = puml::render_source_to_svg(
        "@startuml\nobject Order\nobject Customer\nOrder --> Customer : hasSession\n@enduml\n",
    )
    .expect("object svg should render");

    let label_x = attr_value_in_tag(&svg, ">hasSession</text>", "x");
    let target_x = attr_value_in_tag(&svg, ">Customer</text>", "x") - 80;

    assert!(
        target_x - label_x >= 36,
        "expected object relation label midpoint to stay clear of target box border: label_x={label_x}, target_x={target_x}"
    );
}

#[test]
fn deployment_svg_keeps_rightmost_node_inside_viewbox_with_gutter() {
    let svg = puml::render_source_to_svg(
        "@startuml\nnode WebServer\nnode AppServer\nnode DBServer\nWebServer --> AppServer : HTTP\nAppServer --> DBServer : readsRequests\n@enduml\n",
    )
    .expect("deployment svg should render");

    let viewbox_width = attr_value_in_tag(&svg, "<svg ", "width");
    let node_x = attr_value_in_tag(&svg, "data-uml-alias=\"DBServer\"", "x");
    let node_w = attr_value_in_tag(&svg, "data-uml-alias=\"DBServer\"", "width");
    let label_x = attr_value_in_tag(&svg, ">readsRequests</text>", "x");

    assert!(
        viewbox_width - (node_x + node_w) >= 10,
        "expected rightmost deployment node to keep at least 10px gutter: width={viewbox_width}, node_right={}",
        node_x + node_w
    );
    assert!(
        viewbox_width - label_x >= 40,
        "expected deployment relation label to stay clear of right canvas edge: width={viewbox_width}, label_x={label_x}"
    );
}
