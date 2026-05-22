use std::fs;

fn fixture(name: &str) -> String {
    format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

fn svg_attr_i32(svg: &str, attr: &str) -> i32 {
    let key = format!("{attr}=\"");
    let start = svg.find(&key).expect("attribute start") + key.len();
    let rest = &svg[start..];
    let end = rest.find('"').expect("attribute end");
    rest[..end].parse::<i32>().expect("integer SVG attribute")
}

#[test]
fn salt_open_droplist_inside_table_cell_renders_expanded_list() {
    let src = "@startsalt\n{\n{#\n= Setting | = Value\nTheme | ^Workspace^^System default^^Light^^Dark^\n}\n}\n@endsalt\n";
    let svg = puml::render_source_to_svg(src).expect("salt droplist should render");

    assert!(svg.contains("data-salt-widget=\"open-combo\""));
    assert!(svg.contains("data-salt-widget=\"open-combo-list\""));
    assert!(svg.contains("System default"));
    assert!(svg.contains("Light"));
    assert!(svg.contains("Dark"));
}

#[test]
fn salt_creole_cell_layout_grows_for_visual_lines_not_raw_markup() {
    let src = "@startsalt\n{\n| = Notes |\n| <color:blue>**Primary**\\n//Secondary//</color> |\n}\n@endsalt\n";
    let svg = puml::render_source_to_svg(src).expect("salt creole cell should render");

    assert!(svg.contains("data-salt-creole=\"true\""));
    assert!(svg.contains("dy=\"1.2em\""));
    assert!(svg.contains("fill=\"blue\"") || svg.contains("fill=\"#0000ff\""));
    assert!(
        svg_attr_i32(&svg, "height") >= 54,
        "multi-line creole row should increase overall SVG height: {svg}"
    );
}

#[test]
fn salt_settings_dialog_showcase_composes_menu_tabs_tree_table_and_creole() {
    let src = fs::read_to_string(fixture("families/valid_salt_settings_dialog_showcase.puml"))
        .expect("fixture should load");
    let svg = puml::render_source_to_svg(&src).expect("settings dialog fixture should render");

    assert!(svg.contains("data-salt-widget=\"menu\""));
    assert!(svg.contains("data-salt-widget=\"tab\""));
    assert!(svg.contains("data-salt-widget=\"tree\""));
    assert!(svg.contains("data-salt-widget=\"header\""));
    assert!(svg.contains("data-salt-widget=\"open-combo-list\""));
    assert!(svg.contains("data-salt-creole=\"true\""));
    assert!(svg.contains("data-salt-icons=\"person\""));
    assert!(svg.contains("Workspace"));
    assert!(svg.contains("Security"));
    assert!(svg.contains("System default"));
}
