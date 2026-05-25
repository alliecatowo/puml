use std::fs;

use puml::model::{FamilyStyle, NormalizedDocument};
use puml::{normalize_family, parse};

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
    assert!(svg.contains("data-salt-widget=\"menu-dropdown\""));
    assert!(svg.contains("data-salt-creole=\"true\""));
    assert!(svg.contains("data-salt-icons=\"person\""));
    assert!(svg.contains("Workspace"));
    assert!(svg.contains("Security"));
    assert!(svg.contains("System default"));
}

#[test]
fn salt_style_widget_depth_fixture_covers_shared_style_and_open_menu_layout() {
    let src = fs::read_to_string(fixture("families/valid_salt_style_widget_depth.puml"))
        .expect("fixture should load");
    let svg = puml::render_source_to_svg(&src).expect("salt style widget depth should render");

    assert!(svg.contains("data-salt-widget=\"menu\" data-salt-open=\"true\""));
    assert!(svg.contains("data-salt-widget=\"menu-dropdown\""));
    assert!(svg.contains("data-salt-widget=\"tab\""));
    assert!(svg.contains("data-salt-widget=\"open-combo-list\""));
    assert!(svg.contains("data-salt-widget=\"table-span\""));
    assert!(svg.contains("data-salt-widget=\"sprite-ref\""));
    assert!(svg.contains("data-salt-icons=\"person\""));
    assert!(svg.contains("data-salt-icons=\"account-login\""));
    assert!(svg.contains("fill=\"#f8fafc\""));
    assert!(svg.contains("fill=\"#fed7aa\""));
    assert!(svg.contains("fill=\"#ecfeff\""));
    assert!(svg.contains("fill=\"#ede9fe\""));
    assert!(svg.contains("fill=\"#fef3c7\""));
}

#[test]
fn salt_style_block_lowers_into_typed_family_style_before_rendering() {
    let src = "@startsalt\n<style>\nsaltDiagram {\n  BackgroundColor #f8fafc\n  FontColor #0f172a\n  LineColor #334155\n}\nbutton {\n  BackgroundColor #fed7aa\n  FontColor #7c2d12\n}\ninput {\n  BackgroundColor #ecfeff\n}\n</style>\n{\n| Name | \"Ada\" |\n| Action | [Save] |\n}\n@endsalt\n";
    let doc = parse(src).expect("salt style block should parse");
    let normalized = normalize_family(doc).expect("salt style block should normalize");
    let NormalizedDocument::Family(family) = normalized else {
        panic!("expected salt family document");
    };
    let Some(FamilyStyle::Salt(style)) = family.family_style else {
        panic!("expected typed salt style on family document");
    };

    assert_eq!(style.canvas_fill, "#f8fafc");
    assert_eq!(style.text_color, "#0f172a");
    assert_eq!(style.border_color, "#334155");
    assert_eq!(style.button_fill, "#fed7aa");
    assert_eq!(style.button_text_color, "#7c2d12");
    assert_eq!(style.input_fill, "#ecfeff");
}

#[test]
fn salt_theme_preset_maps_to_typed_widget_style() {
    let src =
        "@startsalt\n!theme hacker\n{\n| User | \"Ada\" |\n| Action | [Login] |\n}\n@endsalt\n";
    let doc = parse(src).expect("salt theme should parse");
    let normalized = normalize_family(doc).expect("salt theme should normalize");
    let NormalizedDocument::Family(family) = normalized else {
        panic!("expected salt family document");
    };
    let Some(FamilyStyle::Salt(style)) = family.family_style else {
        panic!("expected typed salt style on family document");
    };

    assert_eq!(style.text_color, "#00ff00");
    assert_eq!(style.canvas_fill, "#050505");
    assert_eq!(style.button_fill, "#0d0d0d");
}

#[test]
fn salt_openiconic_placeholders_render_as_inline_svg_icons() {
    let src = "@startsalt\n{\n| Login <&person> | [Unlock <&key>] |\n}\n@endsalt\n";
    let svg = puml::render_source_to_svg(src).expect("salt icon placeholders should render");

    assert!(svg.contains("data-salt-creole=\"true\""));
    assert!(svg.contains("data-salt-icons=\"person\""));
    assert!(svg.contains("data-salt-icons=\"key\""));
    assert!(svg.contains("data-sprite=\"person\""));
    assert!(svg.contains("data-sprite=\"key\""));
    assert!(svg.contains("puml-sprite-svg"));
    assert!(
        !svg.contains("&lt;&amp;person&gt;") && !svg.contains("&lt;&amp;key&gt;"),
        "raw OpenIconic markup should not leak into Salt text: {svg}"
    );
}
