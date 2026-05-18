fn attr_i32(tag: &str, name: &str) -> i32 {
    let needle = format!("{name}=\"");
    let start = tag
        .find(&needle)
        .unwrap_or_else(|| panic!("missing {name} in tag: {tag}"))
        + needle.len();
    let end = tag[start..]
        .find('"')
        .unwrap_or_else(|| panic!("unterminated {name} in tag: {tag}"))
        + start;
    tag[start..end]
        .parse()
        .unwrap_or_else(|_| panic!("{name} should be numeric in tag: {tag}"))
}

fn row_group<'a>(svg: &'a str, label_attr: &str) -> &'a str {
    svg.split("<g ")
        .find(|tag| tag.contains(label_attr))
        .unwrap_or_else(|| panic!("missing row group with {label_attr}"))
}

fn row_rect_x(svg: &str, label_attr: &str) -> i32 {
    let group = row_group(svg, label_attr);
    let rect = group
        .split("<rect ")
        .nth(1)
        .unwrap_or_else(|| panic!("missing rect for row group: {group}"));
    attr_i32(rect, "x")
}

#[test]
fn json_nested_maps_arrays_render_depth_metadata_and_geometry() {
    let src = include_str!("fixtures/structured/valid_json_nested_projection.puml");

    let svg = puml::render_source_to_svg(src).expect("json render");

    assert!(svg.contains("data-projection=\"json\""));
    assert!(svg.contains("data-json-node-count=\"11\""));
    assert!(svg.contains("data-json-max-depth=\"4\""));
    assert!(svg.contains("class=\"data-tree-node json-node json-depth-4\""));
    assert!(svg.contains("data-json-label=\"[0]: {...}\""));
    assert!(svg.contains("data-json-label=\"replicas: 3\""));

    let root_x = row_rect_x(&svg, "data-json-label=\"{...}\"");
    let service_x = row_rect_x(&svg, "data-json-label=\"service: {...}\"");
    let array_item_x = row_rect_x(&svg, "data-json-label=\"[0]: {...}\"");
    let replica_x = row_rect_x(&svg, "data-json-label=\"replicas: 3\"");

    assert_eq!(root_x, 24);
    assert_eq!(service_x - root_x, 18);
    assert_eq!(array_item_x - service_x, 36);
    assert_eq!(replica_x - array_item_x, 18);
}

#[test]
fn yaml_nested_maps_arrays_render_depth_metadata_and_geometry() {
    let src = include_str!("fixtures/structured/valid_yaml_nested_projection.puml");

    let svg = puml::render_source_to_svg(src).expect("yaml render");

    assert!(svg.contains("data-projection=\"yaml\""));
    assert!(svg.contains("data-yaml-node-count=\"12\""));
    assert!(svg.contains("data-yaml-max-depth=\"4\""));
    assert!(svg.contains("class=\"data-tree-node yaml-node yaml-depth-4\""));
    assert!(svg.contains("data-yaml-label=\"[0]: {...}\""));
    assert!(svg.contains("data-yaml-label=\"replicas: 3\""));

    let service_x = row_rect_x(&svg, "data-yaml-label=\"service: {...}\"");
    let regions_x = row_rect_x(&svg, "data-yaml-label=\"regions: [...]\"");
    let region_item_x = row_rect_x(&svg, "data-yaml-label=\"[0]: {...}\"");
    let replica_x = row_rect_x(&svg, "data-yaml-label=\"replicas: 3\"");

    assert_eq!(service_x, 42);
    assert_eq!(regions_x - service_x, 18);
    assert_eq!(region_item_x - regions_x, 18);
    assert_eq!(replica_x - region_item_x, 18);
}
