use super::support::*;
use super::*;

#[test]
fn archimate_family_check_mode_passes_for_valid_input() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("non_sequence/valid_archimate.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn sequence_box_grouping_and_hide_unlinked_fixture_validates_cleanly() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("structure/valid_box_grouping_and_hide_unlinked.puml"),
        ])
        .assert()
        .success();
}

#[test]
fn format_png_flag_is_accepted_in_check_mode() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--format", "png", "--check", "-"])
        .write_stdin("@startuml\nA -> B\n@enduml\n")
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn charset_flag_accepts_utf8_and_rejects_others() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--charset", "UTF-8", "--check", "-"])
        .write_stdin("@startuml\nA -> B\n@enduml\n")
        .assert()
        .success();

    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--charset", "latin-1", "--check", "-"])
        .write_stdin("@startuml\nA -> B\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_CHARSET_UNSUPPORTED"));
}

#[test]
fn overwrite_flag_is_accepted_as_noop() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--overwrite", "--check", "-"])
        .write_stdin("@startuml\nA -> B\n@enduml\n")
        .assert()
        .success();
}

#[test]
fn duration_flag_emits_elapsed_to_stderr() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--duration", "--check", "-"])
        .write_stdin("@startuml\nA -> B\n@enduml\n")
        .assert()
        .success()
        .stderr(predicate::str::contains("elapsed:"));
}

#[test]
fn verbose_flag_emits_stage_timings_to_stderr() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--verbose", "--check", "-"])
        .write_stdin("@startuml\nA -> B\n@enduml\n")
        .assert()
        .success()
        .stderr(predicate::str::contains("[verbose] parse"));
}

#[test]
fn quiet_flag_suppresses_warnings_on_stderr() {
    // Unsupported skinparams still warn normally, and --quiet suppresses that output.
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--quiet", "--check", "-"])
        .write_stdin("@startuml\nskinparam unknownKey foo\nA -> B\n@enduml\n")
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn fail_on_warn_flag_exits_one_when_warnings_emitted() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--fail-on-warn", "--check", "-"])
        .write_stdin("@startuml\nskinparam UnknownXyzKey value\nA -> B\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_WARNINGS_PRESENT"));
}

#[test]
fn class_diagram_with_relations_renders_real_svg() {
    let src = fs::read_to_string(fixture("families/valid_class_with_relations.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("class svg should render");
    // Real SVG primitives must appear.
    assert!(svg.starts_with("<svg"), "svg should start with <svg tag");
    assert!(svg.contains("<rect"), "should contain rect for boxes");
    assert!(
        svg.contains("<line"),
        "should contain lines for relations: {svg}"
    );
    // Class names should be present.
    for name in ["Animal", "Dog", "Cat", "Collar"] {
        assert!(svg.contains(name), "missing class name {name}");
    }
    // Inheritance arrow uses the triangle marker.
    assert!(
        svg.contains("arrow-triangle"),
        "inheritance arrow marker missing"
    );
    // Composition uses the filled-diamond marker.
    assert!(
        svg.contains("arrow-diamond-filled"),
        "composition diamond marker missing"
    );
    // Aggregation uses the open-diamond marker.
    assert!(
        svg.contains("arrow-diamond-open"),
        "aggregation diamond marker missing"
    );
    // Label rendering.
    assert!(svg.contains("has"), "composition label missing");
    assert!(svg.contains("wears"), "aggregation label missing");
}

#[test]
fn class_inheritance_example_renders_fixture_text_and_relations() {
    let src = fs::read_to_string("docs/examples/class/02_inheritance.puml").unwrap();
    let svg = render_source_to_svg(&src).expect("class inheritance svg should render");

    for expected in [
        "Vehicle",
        "+make: String",
        "+model: String",
        "+start()",
        "Car",
        "+doors: Int",
        "+drive()",
        "Truck",
        "+payload: Float",
        "+haul()",
    ] {
        assert!(
            svg.contains(expected),
            "missing class fixture text {expected}"
        );
    }

    for (from, to) in [("Vehicle", "Car"), ("Vehicle", "Truck")] {
        let relation = svg_relation_element(&svg, from, to)
            .unwrap_or_else(|| panic!("missing inheritance relation {from} -> {to}"));
        assert!(
            svg_element_has_class(relation, "uml-relation")
                || svg_element_has_class(relation, "puml-edge"),
            "inheritance relation {from} -> {to} should have a semantic edge class"
        );
    }
    assert!(
        svg.contains("marker-start=\"url(#arrow-triangle)\"")
            || svg.contains("marker-end=\"url(#arrow-triangle)\""),
        "inheritance arrow marker missing"
    );
}

#[test]
fn class_diagram_with_relations_render_is_deterministic() {
    let src = fs::read_to_string(fixture("families/valid_class_with_relations.puml")).unwrap();
    let first = render_source_to_svg(&src).unwrap();
    let second = render_source_to_svg(&src).unwrap();
    assert_eq!(first, second);
}

#[test]
fn family_relations_with_cardinalities_render_endpoint_labels() {
    let src = fs::read_to_string(fixture("families/valid_class_with_cardinalities.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("family svg should render");
    assert!(svg.contains(">1<"), "left cardinality should render");
    assert!(svg.contains(">*<"), "right cardinality should render");
    assert!(
        svg.contains(">0..1<"),
        "right cardinality variant should render"
    );
    assert!(svg.contains("places"), "relation label should render");
    assert!(svg.contains("reads"), "relation label should render");
}

#[test]
fn component_relations_with_cardinalities_render_endpoint_labels() {
    let src =
        "@startuml\ncomponent API\ncomponent DB\nAPI \"1\" --> \"n\" DB : depends-on\n@enduml\n";
    let svg = render_source_to_svg(src).expect("component svg should render");
    assert!(
        svg.contains(">1<"),
        "component left cardinality should render"
    );
    assert!(
        svg.contains(">n<"),
        "component right cardinality should render"
    );
    assert!(
        svg.contains("depends-on"),
        "component relation label should render"
    );
}

#[test]
fn family_relations_render_colon_endpoint_roles_without_stealing_edge_label() {
    let src = "@startuml\nclass Customer\nclass Order\nCustomer \"1\" :buyer --> \"*\" :orders Order : places\n@enduml\n";
    let svg = render_source_to_svg(src).expect("class relation roles should render");
    assert!(svg.contains(">buyer<"), "left colon role should render");
    assert!(svg.contains(">orders<"), "right colon role should render");
    assert!(svg.contains(">1<"), "left cardinality should render");
    assert!(svg.contains(">*<"), "right cardinality should render");
    assert!(svg.contains("places"), "edge label should render");
}

#[test]
fn component_and_deployment_groups_render_labeled_frames_and_nested_members() {
    let component_src = "@startuml\nskinparam ComponentBorderColor #0f766e\npackage \"Core Services\" {\n  component \"Public API\" as API\n  node \"Runtime Zone\" {\n    component Worker\n  }\n}\nAPI --> Worker : dispatches\n@enduml\n";
    let component_svg =
        render_source_to_svg(component_src).expect("component group svg should render");
    assert!(
        component_svg.contains(">package Core Services<"),
        "component package frame label should render"
    );
    assert!(
        component_svg.contains("Public API") || component_svg.contains(">API<"),
        "component group member should render"
    );
    assert!(
        component_svg.contains(">Worker<"),
        "nested component group member should render"
    );
    assert!(
        component_svg.contains("stroke=\"#0f766e\""),
        "component border skinparam should style group frame"
    );
    assert!(
        component_svg.contains(">dispatches<"),
        "relation label between grouped members should render"
    );

    let deployment_src = "@startuml\nnode \"Edge Site\" {\n  artifact App\n  database Cache\n}\nApp --> Cache : warms\n@enduml\n";
    let deployment_svg =
        render_source_to_svg(deployment_src).expect("deployment group svg should render");
    assert!(
        deployment_svg.contains(">node Edge Site<"),
        "deployment node frame label should render"
    );
    assert!(
        deployment_svg.contains(">App<"),
        "artifact member should render"
    );
    assert!(
        deployment_svg.contains(">Cache<"),
        "database member should render"
    );
    assert!(
        deployment_svg.contains(">warms<"),
        "deployment grouped relation should render"
    );
}

#[test]
fn class_relations_with_roles_render_endpoint_role_labels() {
    let src = fs::read_to_string(fixture("families/valid_class_with_relation_roles.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("class svg should render");
    assert!(svg.contains(">buyer<"), "left role label should render");
    assert!(svg.contains(">items<"), "right role label should render");
    assert!(svg.contains(">1<"), "left cardinality should render");
    assert!(svg.contains(">0..*<"), "right cardinality should render");
}

#[test]
fn class_parallel_relations_stagger_labels_for_shared_node_pairs() {
    let svg =
        render_source_to_svg(&fs::read_to_string(example("class/12_all_relations.puml")).unwrap())
            .expect("class all relations example should render");
    let extends_label = svg_text_positions(&svg, "&lt;&lt;extend&gt;&gt;")
        .into_iter()
        .next()
        .expect("extends stereotype position");
    let association_label = svg_text_positions(&svg, "association")
        .into_iter()
        .next()
        .expect("association label position");
    assert!(
        (extends_label.0 - association_label.0).abs() >= 12
            || (extends_label.1 - association_label.1).abs() >= 12,
        "shared-pair labels should not overlap"
    );
}

#[test]
fn class_package_headers_stay_above_nested_members() {
    let svg = render_source_to_svg(
        &fs::read_to_string(example("class/14_nested_packages.puml")).unwrap(),
    )
    .expect("nested class packages should render");
    let package_label = svg_text_positions(&svg, "package repository")
        .into_iter()
        .next()
        .expect("repository package label");
    let user_service = svg_text_positions(&svg, "service::UserService")
        .into_iter()
        .next()
        .expect("user service position");
    let product_service = svg_text_positions(&svg, "service::ProductService")
        .into_iter()
        .next()
        .expect("product service position");
    assert!(
        package_label.1 + 12 < user_service.1.min(product_service.1),
        "nested package label should stay above enclosed service nodes"
    );
}

#[test]
fn component_and_deployment_edges_render_advanced_markers_and_dashes() {
    let component_src = "@startuml\ncomponent API\ninterface Gateway\nport Ingress\nAPI o-- Gateway : exposes\nIngress <|-- API : binds\n@enduml\n";
    let component_svg = render_source_to_svg(component_src).expect("component svg should render");
    assert!(
        component_svg.contains("arrow-diamond-open"),
        "aggregation marker should render for component edges"
    );
    assert!(
        component_svg.contains("arrow-triangle"),
        "triangle marker should render for generalization edges"
    );
    assert!(
        component_svg.contains(">exposes<") && component_svg.contains(">binds<"),
        "component relation labels should render"
    );

    let deployment_src = "@startuml\nnode Web\nartifact App\ndatabase Store\nWeb --> App : hosts\nApp *-- Store : data\n@enduml\n";
    let deployment_svg =
        render_source_to_svg(deployment_src).expect("deployment svg should render");
    assert!(
        deployment_svg.contains("arrow-diamond-filled"),
        "composition marker should render for deployment edges"
    );
}

#[test]
fn deployment_database_edge_labels_stay_clear_of_terminal_database_segment() {
    let src = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/examples/deployment/02_databases.puml"),
    )
    .expect("deployment example should load");
    let svg = render_source_to_svg(&src).expect("deployment example should render");
    // Verify the reads/writes label is present and is positioned in the upper
    // shaft segment of the AppServer→PostgreSQL edge, well above the arrowhead
    // at PostgreSQL (which renders at y≈400+).  We avoid pinning to exact pixel
    // coordinates so the test remains valid after layout spacing adjustments.
    assert!(
        svg.contains(">reads/writes<"),
        "reads/writes label should appear in the SVG"
    );
    // Find the y= attribute of the reads/writes text element.
    // The element looks like: <text x="NNN" y="YYY" ...>reads/writes</text>
    let idx = svg.find(">reads/writes<").expect("label present");
    let tag_start = svg[..idx].rfind("<text ").expect("text tag before label");
    let tag = &svg[tag_start..idx];
    let y_pos = tag
        .split("y=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .and_then(|s| s.parse::<i32>().ok())
        .expect("y attribute should be numeric in reads/writes text element");
    assert!(
        y_pos < 360,
        "reads/writes label y={y_pos} should be in the upper shaft segment (< 360), clear of the PostgreSQL arrowhead"
    );
}

#[test]
fn object_diagram_renders_underlined_header_and_rects() {
    let src = fs::read_to_string(fixture("families/valid_object_members_block.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("object svg should render");
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("<rect"));
    assert!(svg.contains("Session"));
    assert!(svg.contains("UserRef"));
    // Objects use underline text-decoration for their name.
    assert!(
        svg.contains("text-decoration=\"underline\""),
        "object header should be underlined"
    );
}

#[test]
fn uml_declaration_stereotypes_and_component_shorthand_aliases_render() {
    // Fix #551: user-defined stereotypes on class/object nodes now render as
    // guillemet labels («…») in the class header, NOT as member rows.
    let class_src = "@startuml\nclass Order <<Entity>>\n@enduml\n";
    let class_svg = render_source_to_svg(class_src).expect("stereotype svg should render");
    assert!(
        class_svg.contains("\u{ab}Entity\u{bb}"),
        "class stereotype should render as guillemet in header (fix #551)"
    );
    let object_src = "@startuml\nobject cache <<singleton>>\n@enduml\n";
    let object_svg = render_source_to_svg(object_src).expect("object stereotype svg should render");
    assert!(
        object_svg.contains("\u{ab}singleton\u{bb}"),
        "object stereotype should render as guillemet in header (fix #551)"
    );
    let usecase_src =
        "@startuml\nactor Shopper <<primary>> as S\nusecase Checkout <<critical>> as UC\nS --> UC : starts\n@enduml\n";
    let usecase_svg =
        render_source_to_svg(usecase_src).expect("usecase stereotype svg should render");
    assert!(
        usecase_svg.contains("&lt;&lt;primary&gt;&gt;"),
        "actor stereotype should render"
    );
    assert!(
        usecase_svg.contains("&lt;&lt;critical&gt;&gt;"),
        "usecase stereotype should render"
    );

    let component_src =
        "@startuml\n[Public API] as API\n() \"Gateway Port\" as Gateway\nAPI --> Gateway : exposes\n@enduml\n";
    let component_svg =
        render_source_to_svg(component_src).expect("component shorthand svg should render");
    assert!(
        component_svg.contains("Public API"),
        "component shorthand label should render"
    );
    assert!(
        component_svg.contains("Gateway Port"),
        "interface shorthand label should render"
    );
    assert!(
        component_svg.contains(">exposes<"),
        "aliased shorthand relation should render"
    );
}

#[test]
fn creole_note_link_svg_contains_hyperlink() {
    let src = fs::read_to_string(fixture("conformance/valid_creole_note_link.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("render");
    assert!(
        svg.contains("xlink:href=\"https://example.com\""),
        "expected hyperlink href in SVG"
    );
    assert!(
        svg.contains("fill=\"blue\""),
        "expected blue fill on link span"
    );
    assert!(
        svg.contains("text-decoration=\"underline\""),
        "expected underline on link span"
    );
}

#[test]
fn class_together_group_passes_check_and_svg_contains_group_frame() {
    let src = fs::read_to_string(fixture("families/valid_class_together.puml")).unwrap();
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("families/valid_class_together.puml")])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let svg = render_source_to_svg(&src).expect("rendered svg");
    // together group frame should be present
    assert!(
        svg.contains("together"),
        "SVG should contain 'together' group label"
    );
    // member names from the together block
    assert!(svg.contains("User"), "SVG should contain User");
    assert!(svg.contains("Account"), "SVG should contain Account");
}

#[test]
fn class_package_namespace_passes_check_and_svg_contains_scope_labels() {
    let src = fs::read_to_string(fixture("families/valid_class_package_namespace.puml")).unwrap();
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("families/valid_class_package_namespace.puml"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    let svg = render_source_to_svg(&src).expect("rendered svg");
    // package and namespace labels should appear
    assert!(
        svg.contains("package"),
        "SVG should contain 'package' label"
    );
    assert!(
        svg.contains("namespace"),
        "SVG should contain 'namespace' label"
    );
    assert!(
        svg.contains("com.example"),
        "SVG should contain package label"
    );
    assert!(
        svg.contains("net.api"),
        "SVG should contain namespace label"
    );
}
