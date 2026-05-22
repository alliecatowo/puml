use puml::render_source_to_svg;

fn render_svg(src: &str) -> String {
    render_source_to_svg(src).expect("salt render should succeed")
}

#[test]
fn salt_cells_render_creole_color_and_openiconic_markup() {
    let src = "@startsalt\n{\n| **Field** | <color:Blue>Value<&person></color> |\n| [<b>Save</b> <&account-login>] | \"//Ada//\" |\n}\n@endsalt\n";
    let svg = render_svg(src);

    assert!(svg.contains("data-salt-creole=\"true\""));
    assert!(svg.contains("data-salt-icons=\"person\""));
    assert!(svg.contains("data-salt-icons=\"account-login\""));
    assert!(!svg.contains("&lt;color:Blue&gt;"));
    assert!(!svg.contains("**Field**"));
    assert!(!svg.contains("//Ada//"));
    assert!(svg.contains("[person]"));
    assert!(svg.contains("[account-login]"));
}

#[test]
fn salt_open_droplist_renders_popup_items() {
    let src =
        "@startsalt\n{\n^This is an open droplist^^ item 1^^ item 2^ | ^Closed^\n}\n@endsalt\n";
    let svg = render_svg(src);

    assert!(svg.contains("data-salt-widget=\"combo\" data-salt-open=\"true\""));
    assert!(svg.contains("data-salt-widget=\"combo-popup\""));
    assert!(svg.contains("item 1"));
    assert!(svg.contains("item 2"));
}

#[test]
fn salt_open_menu_renders_popup_and_anchor() {
    let src = "@startsalt\n{+\n{* File | Edit | Source | Refactor }\n Refactor | New | Open File | - | Close | Close All\n}\n@endsalt\n";
    let svg = render_svg(src);

    assert!(svg.contains("data-salt-menu-anchor=\"Refactor\""));
    assert!(svg.contains("data-salt-widget=\"menu\" data-salt-open=\"true\""));
    assert!(svg.contains("Open File"));
    assert!(svg.contains("Close All"));
}

#[test]
fn salt_tree_table_variants_emit_expected_grid_markers() {
    let vertical = render_svg("@startsalt\n{T!\n+ Root | One\n++ Child | Two\n}\n@endsalt\n");
    assert!(vertical.contains("data-salt-grid=\"vertical\""));
    assert!(!vertical.contains("data-salt-grid=\"horizontal\""));

    let horizontal = render_svg("@startsalt\n{T-\n+ Root | One\n++ Child | Two\n}\n@endsalt\n");
    assert!(horizontal.contains("data-salt-grid=\"horizontal\""));
    assert!(!horizontal.contains("data-salt-grid=\"vertical\""));

    let full = render_svg("@startsalt\n{T#\n+ Root | One\n++ Child | Two\n}\n@endsalt\n");
    assert!(full.contains("data-salt-grid=\"horizontal\""));
    assert!(full.contains("data-salt-grid=\"vertical\""));
}

#[test]
fn salt_common_commands_render_header_title_caption_legend_and_footer() {
    let src = "@startsalt\nheader Salt Header\ntitle Salt Title\ncaption Salt Caption\nlegend right\nLegend Line\nend legend\nfooter Salt Footer\n{\nA | B\n}\n@endsalt\n";
    let svg = render_svg(src);

    assert!(svg.contains("data-salt-meta=\"header\""));
    assert!(svg.contains("Salt Header"));
    assert!(svg.contains("data-salt-meta=\"title\""));
    assert!(svg.contains("Salt Title"));
    assert!(svg.contains("data-salt-meta=\"caption\""));
    assert!(svg.contains("Salt Caption"));
    assert!(svg.contains("data-salt-meta=\"legend\""));
    assert!(svg.contains("Legend Line"));
    assert!(svg.contains("data-salt-meta=\"footer\""));
    assert!(svg.contains("Salt Footer"));
}
