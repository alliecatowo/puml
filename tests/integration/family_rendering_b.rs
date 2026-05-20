use super::support::*;
use super::*;

#[test]
fn class_hide_options_suppress_circle_and_stereotype() {
    let src = fs::read_to_string(fixture("families/valid_class_hide_options.puml")).unwrap();
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("families/valid_class_hide_options.puml"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let svg = render_source_to_svg(&src).expect("rendered svg");
    // When hide circle is active, no circle element for class icon
    assert!(
        !svg.contains("<circle"),
        "SVG should not contain class icon circle when hide circle is set"
    );
    // When hide stereotype is active, the 'class' keyword label should not appear before node names
    // The node names themselves should still appear
    assert!(
        svg.contains("Visible"),
        "SVG should contain node name 'Visible'"
    );
}

#[test]
fn class_visibility_markers_render_colored_symbols() {
    let src = fs::read_to_string(fixture("families/valid_class_visibility.puml")).unwrap();
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("families/valid_class_visibility.puml")])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let svg = render_source_to_svg(&src).expect("rendered svg");
    // Visibility symbols should appear as colored text elements
    assert!(svg.contains("+"), "SVG should contain + visibility marker");
    assert!(svg.contains("-"), "SVG should contain - visibility marker");
    assert!(svg.contains("#"), "SVG should contain # visibility marker");
    assert!(svg.contains("~"), "SVG should contain ~ visibility marker");
    // Abstract and static modifiers should produce style attributes
    assert!(
        svg.contains("font-style=\"italic\""),
        "SVG should contain italic style for {{abstract}} modifier"
    );
    assert!(
        svg.contains("text-decoration=\"underline\""),
        "SVG should contain underline style for {{static}} modifier"
    );
}

#[test]
fn usecase_diagram_renders_ellipse_nodes() {
    let src = fs::read_to_string(fixture("families/valid_usecase_bootstrap.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("usecase svg should render");
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("<ellipse"), "use cases should be ellipses");
    assert!(svg.contains("Authenticate"));
    assert!(svg.contains("Authorize"));
}

#[test]
fn usecase_include_extend_dependencies_render_as_dashed_open_arrows() {
    let src = "@startuml\nusecase Login\nusecase Authorize\nusecase Recover\nLogin ..> Authorize : <<include>>\nRecover .left.> Login : extends\n@enduml\n";
    let svg = render_source_to_svg(src).expect("usecase include/extend svg should render");
    assert!(svg.contains("stroke-dasharray=\"5 3\""));
    assert!(svg.contains("marker-end=\"url(#arrow-open)\""));
    assert!(svg.contains("&lt;&lt;include&gt;&gt;"));
    assert!(svg.contains("&lt;&lt;extend&gt;&gt;"));
}

#[test]
fn component_family_canvas_keeps_rightmost_nodes_inside_viewbox() {
    for src in [
        fs::read_to_string(example("component/01_basic.puml")).unwrap(),
        fs::read_to_string(example("component/02_interfaces.puml")).unwrap(),
        fs::read_to_string(example("component/05_with_notes.puml")).unwrap(),
    ] {
        let svg = render_source_to_svg(&src).expect("component example should render");
        let svg_width = extract_svg_width_attr(&svg).expect("svg width");
        let rightmost_component = svg_elements_with_attr(&svg, "data-uml-kind", "component")
            .iter()
            .map(|element| {
                svg_attr_i32_required(element, "x") + svg_attr_i32_required(element, "width")
            })
            .max()
            .expect("component nodes");
        let rightmost_interface = svg_elements_with_attr(&svg, "data-uml-kind", "interface")
            .into_iter()
            .filter_map(|element| Some(svg_attr_i32(element, "cx")? + svg_attr_i32(element, "r")?))
            .max()
            .unwrap_or(0);
        let rightmost_drawn = rightmost_component.max(rightmost_interface);
        assert!(
            svg_width >= rightmost_drawn + 24,
            "rightmost component/interface should keep a right margin"
        );
    }
}

#[test]
fn component_arrow_labels_fan_apart_and_stay_inside_viewbox() {
    let svg = render_source_to_svg(
        &fs::read_to_string(example("component/06_with_arrows.puml")).unwrap(),
    )
    .expect("component arrow example should render");
    let calls = svg_text_positions(&svg, "calls")
        .into_iter()
        .next()
        .expect("calls label position");
    let uses = svg_text_positions(&svg, "uses")
        .into_iter()
        .next()
        .expect("uses label position");
    let composed = svg_text_positions(&svg, "composed")
        .into_iter()
        .next()
        .expect("composed label position");
    let svg_width = extract_svg_width_attr(&svg).expect("svg width");

    assert!(
        (uses.0 - composed.0).abs() >= 24 || (uses.1 - composed.1).abs() >= 12,
        "uses and composed labels should not overlap in the shared routing lane"
    );
    assert!(
        (calls.0 - uses.0).abs() >= 24 || (calls.1 - uses.1).abs() >= 12,
        "calls and uses labels should remain visually distinct"
    );
    assert!(
        svg_width >= composed.0 + 48,
        "rightmost component label should keep a readable margin inside the viewbox"
    );
}

#[test]
fn usecase_relation_labels_clear_arrowheads_and_each_other() {
    let overlap_svg = render_source_to_svg(
        &fs::read_to_string(example("usecase/03_extends_includes.puml")).unwrap(),
    )
    .expect("usecase overlap example should render");
    let mut dependency_positions = svg_text_positions(&overlap_svg, "&lt;&lt;extend&gt;&gt;");
    dependency_positions.extend(svg_text_positions(&overlap_svg, "&lt;&lt;include&gt;&gt;"));
    assert_eq!(
        dependency_positions.len(),
        3,
        "expected three dependency labels"
    );
    for i in 0..dependency_positions.len() {
        for j in (i + 1)..dependency_positions.len() {
            let dx = (dependency_positions[i].0 - dependency_positions[j].0).abs();
            let dy = (dependency_positions[i].1 - dependency_positions[j].1).abs();
            assert!(dx >= 40 || dy >= 12, "dependency labels should not collide");
        }
    }

    let basic_svg =
        render_source_to_svg(&fs::read_to_string(example("usecase/01_basic.puml")).unwrap())
            .expect("basic usecase example should render");
    let label = svg_text_positions(&basic_svg, "leads to")
        .into_iter()
        .next()
        .expect("relation label position");
    let relation = svg_relation_element(&basic_svg, "Login", "Register").expect("relation element");
    let end = svg_relation_end(relation).expect("relation endpoint");
    assert!(
        (label.0 - end.0).abs() >= 24 || (label.1 - end.1).abs() >= 18,
        "relation label should clear the arrowhead attachment point"
    );
    assert!(
        label.1 < end.1,
        "relation label should float above the arrowhead endpoint"
    );
}

#[test]
fn usecase_relation_label_clears_actor_body_in_with_actors_example() {
    let svg =
        render_source_to_svg(&fs::read_to_string(example("usecase/02_with_actors.puml")).unwrap())
            .expect("usecase actor example should render");
    let label = svg_text_positions(&svg, "leads to")
        .into_iter()
        .next()
        .expect("leads to label position");
    let admin = svg_text_positions(&svg, "Admin")
        .into_iter()
        .next()
        .expect("Admin actor label position");
    let dx = (label.0 - admin.0).abs();
    let dy = label.1 - admin.1;
    assert!(
        dx >= 24 || dy <= -18 || dy >= 10,
        "relation label should stay outside the Admin actor body envelope"
    );
}

#[test]
fn usecase_package_boundaries_render_tab_headers_and_short_names() {
    let svg = render_source_to_svg(
        &fs::read_to_string(example("usecase/04_with_packages.puml")).unwrap(),
    )
    .expect("usecase package example should render");
    let frame = svg_elements_with_attr(&svg, "data-uml-group", "Back Office")
        .into_iter()
        .find(|element| svg_element_has_class(element, "uml-group-frame"))
        .expect("back office frame");
    let frame_y = svg_attr_i32_required(frame, "y");
    let label = svg_text_positions(&svg, "Back Office")
        .into_iter()
        .next()
        .expect("back office label");
    assert!(
        label.1 <= frame_y + 20,
        "boundary label should render in the top tab area"
    );
    assert!(svg.contains(">ManageProducts<"));
    assert!(svg.contains(">ManageOrders<"));
    assert!(svg_text_positions(&svg, "Back Office::MP").is_empty());
    assert!(svg_text_positions(&svg, "Back Office::MO").is_empty());
}

#[test]
fn class_package_headers_clear_inner_class_labels() {
    let svg = render_source_to_svg(
        &fs::read_to_string(example("class/14_nested_packages.puml")).unwrap(),
    )
    .expect("nested class packages example should render");

    for (group, members) in [
        (
            "repository",
            &["repository::UserRepo", "repository::ProductRepo"][..],
        ),
        (
            "service",
            &["service::UserService", "service::ProductService"][..],
        ),
        (
            "domain",
            &["domain::User", "domain::Product", "domain::Order"][..],
        ),
    ] {
        let frame = svg_elements_with_attr(&svg, "data-uml-group", group)
            .into_iter()
            .find(|element| svg_element_has_class(element, "uml-group-frame"))
            .expect("package frame");
        let frame_y = svg_attr_i32_required(frame, "y");
        let min_member_y = members
            .iter()
            .flat_map(|member| svg_text_positions(&svg, member))
            .map(|(_, y)| y)
            .min()
            .expect("inner class label position");

        assert!(
            min_member_y >= frame_y + 72,
            "{group} package header should stay above enclosed class labels"
        );
    }
}

#[test]
fn class_family_accepts_directional_and_dotted_relation_arrows() {
    let src = "@startuml\nclass Base\nclass Impl\nclass Service\nImpl -up-|> Base\nService ..> Impl : depends\n@enduml\n";
    let svg = render_source_to_svg(src).expect("class directional relation svg should render");
    assert!(svg.contains("arrow-triangle"));
    assert!(svg.contains("stroke-dasharray=\"5 3\""));
    assert!(svg.contains("depends"));
}

#[test]
fn component_relations_render_dotted_markers_and_styled_port_shape() {
    let src = "@startuml\ncomponent API\ninterface REST\nport Adapter\nAPI ..> REST : uses\nAdapter -down-> API : plugs\n@enduml\n";
    let svg = render_source_to_svg(src).expect("component relation svg should render");
    assert!(svg.contains("stroke-dasharray=\"5 3\""));
    assert!(svg.contains("marker-end=\"url(#arrow-open)\""));
    assert!(svg.contains("port"));
    assert!(svg.contains("Adapter"));
}

#[test]
fn component_interfaces_attach_relation_endpoints_to_circle_edges() {
    let svg =
        render_source_to_svg(&fs::read_to_string(example("component/02_interfaces.puml")).unwrap())
            .expect("component interface example should render");
    let interface_elements = svg_elements_with_attr(&svg, "data-uml-kind", "interface")
        .into_iter()
        .filter(|element| svg_attr_i32(element, "cx").is_some())
        .collect::<Vec<_>>();
    let graphql_label = svg_text_positions(&svg, "GraphQL")
        .into_iter()
        .next()
        .expect("GraphQL label position");
    let rest_label = svg_text_positions(&svg, "REST")
        .into_iter()
        .next()
        .expect("REST label position");
    let graphql_circle = interface_elements
        .iter()
        .find(|element| svg_attr_i32_required(element, "cx") == graphql_label.0)
        .expect("GraphQL interface circle");
    let rest_circle = interface_elements
        .iter()
        .find(|element| svg_attr_i32_required(element, "cx") == rest_label.0)
        .expect("REST interface circle");
    let endpoint_on_circle = |endpoint: (i32, i32), circle: &str| {
        let cx = svg_attr_i32_required(circle, "cx");
        let cy = svg_attr_i32_required(circle, "cy");
        let r = svg_attr_i32_required(circle, "r");
        let dx = (endpoint.0 - cx).abs();
        let dy = (endpoint.1 - cy).abs();

        (dx == r && dy <= 1) || (dy == r && dx <= 1)
    };

    let graphql_relation =
        svg_relation_element(&svg, "API", "GraphQL").expect("API to GraphQL relation");
    let graphql_end = svg_relation_end(graphql_relation).expect("GraphQL relation endpoint");
    assert!(
        endpoint_on_circle(graphql_end, graphql_circle),
        "GraphQL relation should land on the interface circle edge"
    );

    for relation in [("API", "REST"), ("Client", "REST")] {
        let rest_relation =
            svg_relation_element(&svg, relation.0, relation.1).expect("REST relation");
        let rest_end = svg_relation_end(rest_relation).expect("REST relation endpoint");
        assert!(
            endpoint_on_circle(rest_end, rest_circle),
            "{} to REST should land on the interface circle edge",
            relation.0
        );
    }
}

#[test]
fn state_transitions_accept_short_and_directional_arrows() {
    let src = "@startuml\nstate Idle\nstate Active\nstate Closed\n[*] -> Idle\nIdle -down-> Active : open\nActive --> Closed : done\n@enduml\n";
    let svg = render_source_to_svg(src).expect("state directional transition svg should render");
    assert!(svg.contains("Idle"));
    assert!(svg.contains("Active"));
    assert!(svg.contains("open"));
}

#[test]
fn activity_if_then_branch_label_is_preserved() {
    let src = "@startuml\nstart\nif (in stock?) then (yes)\n:Ship;\nelse (no)\n:Notify;\nendif\nstop\n@enduml\n";
    let svg = render_source_to_svg(src).expect("activity svg should render");
    assert!(
        svg.contains("in stock?"),
        "condition text should appear in diamond"
    );
    assert!(
        svg.contains("yes"),
        "then-guard should appear on outgoing arrow"
    );
    assert!(
        svg.contains("no"),
        "else-guard should appear on outgoing arrow"
    );
}

#[test]
fn gantt_render_emits_horizontal_bars_and_milestone_diamond() {
    let src = fs::read_to_string(fixture("timeline/valid_gantt_render.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("gantt svg should render");
    assert!(svg.starts_with("<svg"));
    // Task labels.
    for name in ["Design", "Build", "Test", "Kickoff"] {
        assert!(svg.contains(name), "missing task/milestone name {name}");
    }
    // Bars are <rect> elements; milestone uses <polygon>.
    assert!(svg.contains("<rect"), "should contain task bars");
    assert!(svg.contains("<polygon"), "milestone diamond missing");
    // Constraint arrow (requires) is rendered as a dashed line + marker.
    assert!(
        svg.contains("gantt-arrow"),
        "constraint arrow marker missing"
    );
    assert!(
        svg.contains("stroke-dasharray"),
        "dashed constraint arrow missing"
    );
}

#[test]
fn gantt_task_decl_split_across_lines_produces_one_bar_per_task() {
    // Regression test for #241: when a task is declared bare on one line and
    // then constrained (e.g. "[Design] starts 2026-01-02") on a subsequent
    // line, the normalizer must merge them into a single task rather than
    // creating a duplicate, which would result in ghost rows with no bars.
    let src = "\
@startgantt\n\
title Project Timeline\n\
[Design]\n\
[Build]\n\
[Test]\n\
[Kickoff] happens on 2026-01-01\n\
[Design] starts 2026-01-02\n\
[Build] starts 2026-01-15\n\
[Build] requires [Design]\n\
[Test] starts 2026-02-01\n\
[Test] requires [Build]\n\
@endgantt\n";
    let svg = render_source_to_svg(src).expect("gantt should render");
    assert!(svg.starts_with("<svg"), "should produce SVG");
    // Count gantt-task rect elements — must equal 3 (one per unique task).
    let bar_count = svg.matches("class=\"gantt-task\"").count();
    assert_eq!(
        bar_count, 3,
        "expected exactly 3 task bars (Design, Build, Test), got {bar_count}; \
         likely duplicate task rows caused by split declarations"
    );
    // Row labels should appear exactly once each.
    for name in ["Design", "Build", "Test"] {
        assert!(svg.contains(name), "task label {name} missing");
    }
    // Milestone row for Kickoff.
    assert!(svg.contains("<polygon"), "milestone diamond missing");
    // Bars must be positioned after the chart's left axis (x > 100).
    let bar_x_vals: Vec<i32> = svg
        .split("class=\"gantt-task\"")
        .skip(1)
        .filter_map(|chunk| {
            let x_part = chunk.split("x=\"").nth(1)?;
            x_part.split('"').next()?.parse().ok()
        })
        .collect();
    assert_eq!(bar_x_vals.len(), 3, "should have 3 bar x coordinates");
    for &x in &bar_x_vals {
        assert!(
            x > 100,
            "bar x={x} is unexpectedly small (left of label column)"
        );
    }
}

#[test]
fn state_concurrent_renders_dashed_divider() {
    let src = fs::read_to_string(fixture("families/valid_state_concurrent.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("should render state concurrent SVG");
    assert!(svg.contains("<svg"), "expected SVG output");
    assert!(
        svg.contains("stroke-dasharray"),
        "expected dashed divider in concurrent state SVG"
    );
}

#[test]
fn chronology_render_emits_vertical_timeline_with_event_bullets() {
    let src = fs::read_to_string(fixture("timeline/valid_chronology_render.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("chronology svg should render");
    assert!(svg.starts_with("<svg"));
    // Events appear as labels.
    for name in ["Discovery", "Alpha", "Beta", "GA"] {
        assert!(svg.contains(name), "missing event {name}");
    }
    // Dates rendered.
    assert!(svg.contains("2026-05-01"));
    // Vertical timeline line + bullet circles.
    assert!(svg.contains("<line"), "timeline line missing");
    assert!(svg.contains("<circle"), "event bullet missing");
}

#[test]
fn timeline_render_is_deterministic_across_runs() {
    let gantt = fs::read_to_string(fixture("timeline/valid_gantt_render.puml")).unwrap();
    let chrono = fs::read_to_string(fixture("timeline/valid_chronology_render.puml")).unwrap();
    assert_eq!(
        render_source_to_svg(&gantt).unwrap(),
        render_source_to_svg(&gantt).unwrap()
    );
    assert_eq!(
        render_source_to_svg(&chrono).unwrap(),
        render_source_to_svg(&chrono).unwrap()
    );
}

#[test]
fn class_hide_empty_members_collapses_empty_compartment() {
    let src =
        "@startuml\nhide empty members\nclass Full {\n  +name: String\n}\nclass Empty\n@enduml\n";
    let svg = render_source_to_svg(src).expect("rendered svg");
    // Full class should show its member; Empty class box should be shorter (no extra member rows)
    assert!(
        svg.contains("name: String"),
        "SVG should contain member text"
    );
    // Both class names should appear
    assert!(svg.contains("Full"), "SVG should contain Full class");
    assert!(svg.contains("Empty"), "SVG should contain Empty class");
}

#[test]
fn class_set_namespace_separator_is_recorded_in_model() {
    use puml::normalize_family;
    use puml::parser::parse;
    use puml::NormalizedDocument;

    let src = "@startuml\nset namespaceSeparator ::\nclass Foo\n@enduml\n";
    let doc = parse(src).expect("parse ok");
    let model = normalize_family(doc).expect("normalize ok");
    let NormalizedDocument::Family(family) = model else {
        panic!("expected Family model");
    };
    assert_eq!(
        family.namespace_separator.as_deref(),
        Some("::"),
        "namespace_separator should be recorded as ::"
    );
}

#[test]
fn archimate_family_renders_deterministic_svg_with_layers() {
    let src = fs::read_to_string(fixture("non_sequence/valid_archimate.puml")).unwrap();
    let a = render_source_to_svg(&src).expect("render archimate");
    let b = render_source_to_svg(&src).expect("render archimate again");
    assert_eq!(a, b, "archimate render must be deterministic");
    assert!(a.contains("Archimate"));
    assert!(a.contains("application"));
    assert!(a.contains("Customer"));
}

#[test]
fn archimate_docs_examples_render_typed_shapes_and_edges() {
    let root = env!("CARGO_MANIFEST_DIR");
    let layered =
        fs::read_to_string(format!("{root}/docs/examples/archimate/01_layered.puml")).unwrap();
    let relations = fs::read_to_string(format!(
        "{root}/docs/examples/archimate/02_with_relations.puml"
    ))
    .unwrap();
    let flows = fs::read_to_string(format!(
        "{root}/docs/examples/archimate/03_with_junctions.puml"
    ))
    .unwrap();

    let layered_svg = render_source_to_svg(&layered).expect("render layered archimate example");
    assert!(layered_svg.contains("data-archimate-kind=\"component\""));
    assert!(layered_svg.contains("data-archimate-kind=\"node\""));
    assert!(layered_svg.contains("data-archimate-kind=\"data-object\""));
    assert!(layered_svg.contains("data-archimate-kind=\"serving\""));
    assert!(layered_svg.contains("data-archimate-kind=\"access\""));
    assert!(layered_svg.contains("class=\"archimate-relation-edge\""));
    assert!(!layered_svg.contains("<text class=\"archimate-relation\""));

    let relations_svg =
        render_source_to_svg(&relations).expect("render relation archimate example");
    assert!(relations_svg.contains("data-archimate-kind=\"process\""));
    assert!(relations_svg.contains("data-archimate-kind=\"service\""));
    assert!(relations_svg.contains("data-archimate-kind=\"assignment\""));
    assert!(relations_svg.contains("marker-start=\"url(#archimate-assignment)\""));
    assert!(relations_svg.contains("data-archimate-role-icon=\"process\""));
    assert!(relations_svg.contains("data-archimate-role-icon=\"service\""));
    assert!(relations_svg.contains("fill=\"#FFFFB0\""));
    assert!(relations_svg.contains("fill=\"#D5E8F0\""));
    assert!(relations_svg.contains("assigned"));

    let flows_svg = render_source_to_svg(&flows).expect("render flow archimate example");
    assert!(flows_svg.contains("data-archimate-kind=\"flow\""));
    assert!(flows_svg.contains("stroke-dasharray=\"5 3\""));
    assert!(flows_svg.contains("routes"));
}

#[test]
fn archimate_layer_palette_matches_spec_hexes() {
    let src = r#"@startarchimate
archimate "Capability" as cap <<strategy>>
archimate "Order Process" as proc <<business>>
archimate "Order Service" as svc <<application>>
archimate "Gateway" as gw <<technology>>
archimate "Goal" as goal <<motivation>>
@endarchimate"#;

    let svg = render_source_to_svg(src).expect("render archimate palette example");
    assert!(svg.contains("fill=\"#F5DEAA\""));
    assert!(svg.contains("fill=\"#FFFFB0\""));
    assert!(svg.contains("fill=\"#D5E8F0\""));
    assert!(svg.contains("fill=\"#D5F5DD\""));
    assert!(svg.contains("fill=\"#E0D5F5\""));
}

// ---- stdlib catalog tests (#173) ----
