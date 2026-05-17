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
fn sdl_render_uses_typed_shapes_and_transition_edges() {
    let src = r#"@startsdl
title SDL With Transitions
start Idle
state Processing
state Waiting
stop Done
Idle -> Processing : request
Processing -> Waiting : response
Waiting -> Idle : retry
Waiting -> Done : complete
@endsdl
"#;
    let svg = puml::render_source_to_svg(src).expect("sdl render");

    assert!(svg.contains("SDL With Transitions"));
    assert!(svg.contains("data-sdl-kind=\"start\""));
    assert!(svg.contains("data-sdl-kind=\"state\""));
    assert!(svg.contains("data-sdl-kind=\"stop\""));
    assert!(svg.contains("class=\"sdl-transition\""));
    assert!(svg.contains("marker-end=\"url(#sdl-arrow)\""));
    assert!(svg.contains("data-sdl-from=\"Waiting\" data-sdl-to=\"Done\""));
    assert!(svg.contains(">request</text>"));
    assert!(svg.contains(">Idle</text>"));
    assert!(svg.contains(">Done</text>"));
    assert!(
        !svg.contains("<ellipse"),
        "SDL should not collapse to an empty ellipse placeholder"
    );
}

#[test]
fn sdl_input_output_decision_shapes_render_from_fixture() {
    let src = std::fs::read_to_string(format!(
        "{}/tests/fixtures/families/valid_sdl_shapes.puml",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("sdl fixture");
    let svg = puml::render_source_to_svg(&src).expect("sdl render");

    for kind in ["start", "input", "state", "decision", "output", "stop"] {
        assert!(
            svg.contains(&format!("data-sdl-kind=\"{kind}\"")),
            "missing SDL {kind} node in rendered SVG"
        );
    }
    for label in ["receive", "parse", "check", "yes", "no", "complete"] {
        assert!(
            svg.contains(&format!(">{label}</text>")),
            "missing transition label {label}"
        );
    }
    assert!(svg.matches("class=\"sdl-transition\"").count() >= 6);
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
    assert!(svg.contains("data-archimate-kind=\"flow\""));
    assert!(svg.contains("data-archimate-direction=\"down\""));
    assert!(svg.contains("data-archimate-style=\"#2563eb\""));
    assert!(svg.contains("data-archimate-kind=\"triggering\""));
    assert!(svg.contains("data-archimate-direction=\"right\""));
    assert!(svg.contains("data-archimate-style=\"dashed\""));
    assert!(svg.contains("routes"));
    assert!(svg.contains("branches"));
    assert!(!svg.contains("<text class=\"archimate-relation\""));
}

#[test]
fn mindmap_left_right_side_and_color_metadata_render() {
    let src = r##"@startmindmap
*[#fef3c7] Platform
left side
**[#fecaca] Risks
*** Mitigation
right side
**[#bbf7d0] Delivery
*** Ship
@endmindmap
"##;
    let svg = puml::render_source_to_svg(src).expect("mindmap render");

    assert!(svg.contains("mindmap-root"));
    assert!(svg.contains("data-mindmap-orientation=\"top-to-bottom\""));
    assert!(svg.contains("data-mindmap-side=\"left\""));
    assert!(svg.contains("data-mindmap-side=\"right\""));
    assert!(svg.contains("data-mindmap-depth=\"2\""));
    assert!(svg.contains("mindmap-depth-2"));
    assert!(svg.contains("data-mindmap-fill=\"#fecaca\""));
    assert!(svg.contains("#fef3c7"));
    assert!(svg.contains("#fecaca"));
    assert!(svg.contains("#bbf7d0"));
}

#[test]
fn wbs_checkbox_progress_and_orientation_metadata_render() {
    let src = r##"@startwbs
left to right direction
* Project
** Build [x]
** Verify [%60]
** Release [ ]
@endwbs
"##;
    let svg = puml::render_source_to_svg(src).expect("wbs render");

    assert!(svg.contains("data-wbs-orientation=\"left-to-right\""));
    assert!(svg.contains("class=\"wbs-node wbs-depth-"));
    assert!(svg.contains("data-wbs-checkbox=\"checked\""));
    assert!(svg.contains("data-wbs-checkbox=\"progress\" data-wbs-progress=\"60\""));
    assert!(svg.contains("data-wbs-checkbox=\"unchecked\""));
    assert!(svg.contains("wbs-depth-1"));
    assert!(svg.contains("wbs-checked"));
    assert!(svg.contains("data-wbs-annotation-style=\"checked\""));
    assert!(svg.contains("class=\"wbs-progress-fill\" data-wbs-progress-fill=\"60\""));
}

#[test]
fn mindmap_and_wbs_large_tree_metadata_and_branch_classes_render() {
    let mindmap = r##"@startmindmap
* Platform
** Delivery
*** Build
*** Test
** Operations
*** Observe
*** Respond
left side
** Risks
*** Security
*** Compliance
@endmindmap
"##;
    let mindmap_svg = puml::render_source_to_svg(mindmap).expect("mindmap render");
    assert!(mindmap_svg.contains("data-mindmap-node-count=\"10\""));
    assert!(mindmap_svg.contains("data-mindmap-leaf-count=\"6\""));
    assert!(mindmap_svg.contains("data-mindmap-child-count=\"3\""));
    assert!(mindmap_svg.contains("mindmap-branch"));
    assert!(mindmap_svg.contains("mindmap-leaf"));
    assert!(mindmap_svg.contains("data-mindmap-sibling-index=\"1\""));

    let wbs = r##"@startwbs
top to bottom direction
* Program
** Build [x]
*** Parser
*** Renderer
** Launch [%75]
*** Docs
*** Release
@endwbs
"##;
    let wbs_svg = puml::render_source_to_svg(wbs).expect("wbs render");
    assert!(wbs_svg.contains("data-wbs-node-count=\"7\""));
    assert!(wbs_svg.contains("data-wbs-leaf-count=\"4\""));
    assert!(wbs_svg.contains("data-wbs-child-count=\"2\""));
    assert!(wbs_svg.contains("class=\"wbs-edge\" data-wbs-edge-depth=\"2\""));
    assert!(wbs_svg.contains("wbs-branch"));
    assert!(wbs_svg.contains("wbs-leaf"));
}

#[test]
fn topology_breadth_archimate_nwdiag_and_deployment_metadata_render() {
    let nwdiag = r##"@startnwdiag
nwdiag {
  group dmz {
    description = "DMZ group"
    color = "#fef3c7"
    style = "dashed"
    web01;
  }
  network public {
    address = "203.0.113.0/24"
    description = "Public edge"
    shape = "swimlane"
    style = "dashed"
    web01 [address = "203.0.113.10, 2001:db8::10", description = "Web Edge", shape = "roundedbox", style = "dashed", width = 320];
  }
}
@endnwdiag
"##;
    let nwdiag_svg = puml::render_source_to_svg(nwdiag).expect("nwdiag render");
    assert!(nwdiag_svg.contains("Public edge"));
    assert!(nwdiag_svg.contains("data-nwdiag-style=\"dashed\""));
    assert!(nwdiag_svg.contains("data-nwdiag-name=\"web01\""));
    assert!(nwdiag_svg.contains("data-nwdiag-addresses=\"203.0.113.10, 2001:db8::10\""));
    assert!(nwdiag_svg.contains("width=\"320\""));
    assert!(nwdiag_svg.contains("DMZ group"));

    let archimate = r##"@startarchimate
Business_Actor(customer, "Customer", "#fef3c7")
Application_Component(app, "Order App", "#dbeafe")
Technology_Node(node, "Kubernetes", "#dcfce7")
Junction_And(j_and, "AND")
Rel_Serving_Right(app, customer, "serves", "#2563eb")
Rel_Flow_Down(node, app, "deploys", "dashed")
Rel_Triggering_Left(customer, j_and, "chooses", "bold")
@endarchimate
"##;
    let archimate_svg = puml::render_source_to_svg(archimate).expect("archimate render");
    assert!(archimate_svg.contains("class=\"archimate-relation-edge\""));
    assert!(archimate_svg.contains("data-archimate-kind=\"serving\""));
    assert!(archimate_svg.contains("data-archimate-direction=\"right\""));
    assert!(archimate_svg.contains("data-archimate-style=\"#2563eb\""));
    assert!(archimate_svg.contains("data-archimate-style=\"dashed\""));
    assert!(archimate_svg.contains("data-archimate-style=\"bold\""));
    assert!(archimate_svg.contains("class=\"archimate-junction\""));

    let deployment = r##"@startuml
node "K8s Cluster" as k8s #dcfce7
database "Orders DB" as db #fef3c7
cloud "Public Cloud" as cloud
folder "Manifests" as manifests
artifact "orders.jar-free" as artifact
portin "HTTPS" as https
portout "SQL" as sql
component "API" as api
cloud -[#2563eb;line.dashed;line.thickness=3]-> https : ingress
https --> api
api -down-> sql
sql -[#b45309;line.bold]right-> db : queries
k8s --> manifests
manifests --> artifact
@enduml
"##;
    let deployment_svg = puml::render_source_to_svg(deployment).expect("deployment render");
    assert!(deployment_svg.contains("data-uml-kind=\"database\""));
    assert!(deployment_svg.contains("data-uml-kind=\"cloud\""));
    assert!(deployment_svg.contains("data-uml-kind=\"folder\""));
    assert!(deployment_svg.contains("data-uml-kind=\"artifact\""));
    assert!(deployment_svg.contains("data-uml-port-direction=\"in\""));
    assert!(deployment_svg.contains("data-uml-port-direction=\"out\""));
    assert!(deployment_svg.contains("data-uml-direction=\"right\""));
    assert!(deployment_svg.contains("stroke=\"#2563eb\""));
    assert!(deployment_svg.contains("stroke-width=\"3\""));
}
