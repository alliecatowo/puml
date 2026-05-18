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

fn attr_value_in_next_tag_after(haystack: &str, marker: &str, tag_prefix: &str, attr: &str) -> i32 {
    let marker_idx = haystack.find(marker).expect("marker should exist");
    let tag_start = haystack[marker_idx..]
        .find(tag_prefix)
        .map(|idx| marker_idx + idx)
        .expect("next tag should exist");
    let tag_end = haystack[tag_start..]
        .find('>')
        .map(|idx| tag_start + idx)
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
fn object_relation_labels_stay_centered_on_vertical_relations() {
    let svg = puml::render_source_to_svg(
        "@startuml\nobject Order\nobject Customer\nOrder --> Customer : hasSession\n@enduml\n",
    )
    .expect("object svg should render");

    let label_x = attr_value_in_tag(&svg, ">hasSession</text>", "x");
    let target_center_x = attr_value_in_tag(&svg, ">Customer</text>", "x");

    assert!(
        label_x == target_center_x,
        "expected object relation label to stay centered on the vertical relation: label_x={label_x}, target_center_x={target_center_x}"
    );
}

#[test]
fn deployment_svg_keeps_rightmost_node_inside_viewbox_with_gutter() {
    let svg = puml::render_source_to_svg(
        "@startuml\nnode WebServer\nnode AppServer\nnode DBServer\nWebServer --> AppServer : HTTP\nAppServer --> DBServer : readsRequests\n@enduml\n",
    )
    .expect("deployment svg should render");

    let viewbox_width = attr_value_in_tag(&svg, "<svg ", "width");
    let node_x = attr_value_in_next_tag_after(
        &svg,
        "data-uml-id=\"DBServer\"",
        "<rect class=\"uml-node uml-deployment-shape\"",
        "x",
    );
    let node_w = attr_value_in_next_tag_after(
        &svg,
        "data-uml-id=\"DBServer\"",
        "<rect class=\"uml-node uml-deployment-shape\"",
        "width",
    );
    let label_x = attr_value_in_tag(&svg, ">readsRequests</text>", "x");

    assert!(
        viewbox_width - (node_x + node_w) >= 40,
        "expected rightmost deployment node to keep at least 40px gutter: width={viewbox_width}, node_right={}",
        node_x + node_w
    );
    assert!(
        viewbox_width - label_x >= 80,
        "expected deployment relation label to stay clear of right canvas edge: width={viewbox_width}, label_x={label_x}"
    );
}
