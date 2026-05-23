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

fn row_text_x(svg: &str, label_attr: &str, text_index: usize) -> i32 {
    let group = row_group(svg, label_attr);
    let text = group
        .split("<text ")
        .nth(text_index)
        .unwrap_or_else(|| panic!("missing text {text_index} for row group: {group}"));
    attr_i32(text, "x")
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
    assert!(svg.contains("class=\"data-table-frame json-table\""));
    assert!(svg.contains("class=\"data-table-separator\""));

    let root_x = row_rect_x(&svg, "data-json-label=\"{...}\"");
    let service_x = row_rect_x(&svg, "data-json-label=\"service: {...}\"");
    let array_item_x = row_rect_x(&svg, "data-json-label=\"[0]: {...}\"");
    let replica_x = row_rect_x(&svg, "data-json-label=\"replicas: 3\"");

    assert_eq!(root_x, 24);
    assert_eq!(service_x, root_x);
    assert_eq!(array_item_x, root_x);
    assert_eq!(replica_x, root_x);
    assert_eq!(
        row_text_x(&svg, "data-json-label=\"service: {...}\"", 1),
        50
    );
    assert_eq!(row_text_x(&svg, "data-json-label=\"[0]: {...}\"", 1), 86);
    assert_eq!(row_text_x(&svg, "data-json-label=\"replicas: 3\"", 1), 104);
    assert_eq!(row_text_x(&svg, "data-json-label=\"replicas: 3\"", 2), 268);
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
    assert!(svg.contains("class=\"data-table-frame yaml-table\""));
    assert!(svg.contains("class=\"data-table-separator\""));

    let service_x = row_rect_x(&svg, "data-yaml-label=\"service: {...}\"");
    let regions_x = row_rect_x(&svg, "data-yaml-label=\"regions: [...]\"");
    let region_item_x = row_rect_x(&svg, "data-yaml-label=\"[0]: {...}\"");
    let replica_x = row_rect_x(&svg, "data-yaml-label=\"replicas: 3\"");

    assert_eq!(service_x, 24);
    assert_eq!(regions_x, service_x);
    assert_eq!(region_item_x, service_x);
    assert_eq!(replica_x, service_x);
    assert_eq!(
        row_text_x(&svg, "data-yaml-label=\"service: {...}\"", 1),
        50
    );
    assert_eq!(
        row_text_x(&svg, "data-yaml-label=\"regions: [...]\"", 1),
        68
    );
    assert_eq!(row_text_x(&svg, "data-yaml-label=\"[0]: {...}\"", 1), 86);
    assert_eq!(row_text_x(&svg, "data-yaml-label=\"replicas: 3\"", 1), 104);
    assert_eq!(row_text_x(&svg, "data-yaml-label=\"replicas: 3\"", 2), 268);
}

#[test]
fn json_highlight_paths_styles_and_creole_scalars_render() {
    let src = include_str!("fixtures/structured/valid_json_highlight_projection.puml");

    let svg = puml::render_source_to_svg(src).expect("json highlight render");

    assert!(svg.contains("data-projection=\"json\""));
    assert!(!svg.contains("#highlight"));
    assert!(!svg.contains("&lt;style&gt;"));
    assert!(svg.contains("data-json-path=\"/phoneNumbers/0/number\""));
    assert!(svg.contains("data-json-highlight=\"true\""));
    assert!(svg.contains("data-json-highlight-class=\"hot\""));
    assert!(svg.contains("fill=\"#dc2626\""));
    assert!(svg.contains("font-style=\"italic\""));
    assert!(svg.contains("data-json-path=\"/empty\""));
    assert!(svg.contains("data-json-label=\"empty: []\""));
    assert!(svg.contains("Smith"));
    assert!(
        svg.contains("font-weight"),
        "Creole-like bold scalar content should render with styled tspans: {svg}"
    );
}

#[test]
fn yaml_highlight_paths_styles_and_creole_scalars_render() {
    let src = include_str!("fixtures/structured/valid_yaml_highlight_projection.puml");

    let svg = puml::render_source_to_svg(src).expect("yaml highlight render");

    assert!(svg.contains("data-projection=\"yaml\""));
    assert!(!svg.contains("#highlight"));
    assert!(!svg.contains("&lt;style&gt;"));
    assert!(svg.contains("data-yaml-path=\"/xmas-fifth-day/partridges\""));
    assert!(svg.contains("data-yaml-highlight=\"true\""));
    assert!(svg.contains("data-yaml-highlight-class=\"h2\""));
    assert!(svg.contains("fill=\"#16a34a\""));
    assert!(svg.contains("font-style=\"italic\""));
    assert!(svg.contains("data-yaml-path=\"/french-hens\""));
    assert!(svg.contains("fill=\"#fde68a\""));
    assert!(svg.contains("pear"));
    assert!(
        svg.contains("font-weight"),
        "Creole-like bold scalar content should render with styled tspans: {svg}"
    );
}

#[test]
fn invalid_json_strips_highlight_and_style_before_fallback() {
    let src = r##"@startjson
#highlight "root" <<bad>>
<style>
.bad {
  BackGroundColor #dc2626
}
</style>
{
  "ok": true,
  "bad":
}
@endjson
"##;

    let text = puml::render_source_to_text(src, puml::TextOutputMode::Txt)
        .expect("invalid JSON should strip controls before fallback rendering");

    assert!(!text.contains("#highlight"));
    assert!(!text.contains("<style>"));
    assert!(text.contains("\"ok\": true"));
    assert!(text.contains("\"bad\":"));
}

#[test]
fn json_and_yaml_root_arrays_keep_index_paths_for_highlight() {
    let json = r##"@startjson
#highlight "1"
["alpha", "beta", "gamma"]
@endjson
"##;
    let yaml = r##"@startyaml
#highlight "1"
- alpha
- beta
- gamma
@endyaml
"##;

    let json_svg = puml::render_source_to_svg(json).expect("json root array render");
    let yaml_svg = puml::render_source_to_svg(yaml).expect("yaml root array render");

    assert!(json_svg.contains("data-json-label=\"[...]\""));
    assert!(json_svg.contains("data-json-path=\"/1\""));
    assert!(json_svg.contains("data-json-label=\"[1]: &quot;beta&quot;\""));
    assert!(json_svg.contains("data-json-highlight=\"true\""));

    assert!(yaml_svg.contains("data-yaml-label=\"[...]\""));
    assert!(yaml_svg.contains("data-yaml-path=\"/1\""));
    assert!(yaml_svg.contains("data-yaml-label=\"[1]: beta\""));
    assert!(yaml_svg.contains("data-yaml-highlight=\"true\""));
}
