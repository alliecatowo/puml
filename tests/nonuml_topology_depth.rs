#[test]
fn nwdiag_groups_attributes_and_multi_node_lines_render() {
    let src = r##"@startnwdiag
nwdiag {
  group frontend {
    color = "#fef3c7"
    web01; web02;
  }
  network dmz {
    address = "10.0.0.x"
    color = "#e0f2fe"
    web01 [address = "10.0.0.10", description = "Web 01", color = "#ffffff"];
    web02;
  }
}
@endnwdiag
"##;
    let svg = puml::render_source_to_svg(src).expect("nwdiag render");

    assert!(svg.contains("class=\"nwdiag-group\""));
    assert!(svg.contains("frontend"));
    assert!(svg.contains("Web 01 [10.0.0.10]"));
    assert!(svg.contains("web02"));
    assert!(svg.contains("#e0f2fe"));
}

#[test]
fn archimate_junction_direction_and_style_breadth_render() {
    let src = r##"@startarchimate
Application_Service(service, "Service", "#dbeafe")
Business_Process(process, "Order Process")
Junction_Or(j1, "Decision")
Rel_Flow_Down(process, service, "routes", "#2563eb")
Rel_Triggering_Right(service, j1, "branches", "dashed")
@endarchimate
"##;
    let svg = puml::render_source_to_svg(src).expect("archimate render");

    assert!(svg.contains("class=\"archimate-element\""));
    assert!(svg.contains("#dbeafe"));
    assert!(svg.contains("class=\"archimate-junction\""));
    assert!(svg.contains("flow direction=down style=#2563eb"));
    assert!(svg.contains("triggering direction=right style=dashed"));
}
