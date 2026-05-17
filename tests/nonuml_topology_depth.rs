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
fn nwdiag_node_shape_and_style_attributes_render() {
    let src = r##"@startnwdiag
nwdiag {
  network edge {
    color = "#e0f2fe"
    lb [address = "10.0.0.2", description = "Load Balancer", color = "#ffffff", shape = "roundedbox", style = "dashed"];
  }
}
@endnwdiag
"##;
    let svg = puml::render_source_to_svg(src).expect("nwdiag render");

    assert!(svg.contains("Load Balancer [10.0.0.2]"));
    assert!(svg.contains("data-nwdiag-shape=\"roundedbox\""));
    assert!(svg.contains("data-nwdiag-style=\"dashed\""));
    assert!(svg.contains("stroke-dasharray=\"5 3\""));
}

#[test]
fn regex_and_ebnf_render_token_style_classes_for_advanced_constructs() {
    let regex = r#"@startregex
^(foo|bar)[0-9]{2,4}$
@endregex
"#;
    let regex_svg = puml::render_source_to_svg(regex).expect("regex render");
    assert!(regex_svg.contains("class=\"regex-token regex-alt\""));
    assert!(regex_svg.contains("class=\"regex-token regex-repeat\""));
    assert!(regex_svg.contains("class=\"regex-token regex-anchor\""));

    let ebnf = r#"@startebnf
expr = term, { ("+" | "-"), term }, [ "end" ];
@endebnf
"#;
    let ebnf_svg = puml::render_source_to_svg(ebnf).expect("ebnf render");
    assert!(ebnf_svg.contains("class=\"ebnf-token ebnf-repetition\""));
    assert!(ebnf_svg.contains("class=\"ebnf-token ebnf-optional\""));
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
