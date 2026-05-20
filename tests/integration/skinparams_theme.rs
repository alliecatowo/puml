use super::*;

#[test]
fn archimate_stdlib_element_and_relation_macros_render() {
    let src = "@startarchimate\n\
Business_Actor(customer, \"Customer\")\n\
Application_Component(service, \"Order Service\")\n\
Technology_Node(runtime, \"Runtime\")\n\
Rel_Assignment(customer, service, \"places order\")\n\
Rel_Access(service, runtime, \"uses\")\n\
@endarchimate\n";
    let svg = render_source_to_svg(src).expect("archimate stdlib macros should render");
    assert!(svg.contains("Customer"));
    assert!(svg.contains("Order Service"));
    assert!(svg.contains("Runtime"));
    assert!(svg.contains("data-archimate-kind=\"assignment\""));
    assert!(svg.contains("data-archimate-kind=\"access\""));
    assert!(svg.contains("marker-start=\"url(#archimate-assignment)\""));
    assert!(svg.contains("stroke-dasharray=\"5 3\""));
    assert!(svg.contains("places order"));
    assert!(svg.contains("uses"));
    assert!(!svg.contains("<text class=\"archimate-relation\""));
}

// ── skinparam classify: class/state/component/activity (#202) ─────────────────

#[test]
fn skinparam_class_keys_accepted_without_warnings() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("styling/valid_skinparam_class.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn skinparam_class_background_color_appears_in_svg() {
    let src = fs::read_to_string(fixture("styling/valid_skinparam_class.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("class skinparam svg should render");
    assert!(svg.starts_with("<svg"), "should be valid svg");
    assert!(
        svg.contains("#e0f2fe"),
        "ClassBackgroundColor #e0f2fe should appear in SVG: {svg}"
    );
    assert!(
        svg.contains("#0369a1"),
        "ClassBorderColor #0369a1 should appear in SVG"
    );
    assert!(
        svg.contains("#bfdbfe"),
        "ClassHeaderBackgroundColor #bfdbfe should appear in SVG"
    );
    assert!(
        svg.contains("#0284c7"),
        "ClassArrowColor #0284c7 should appear in SVG"
    );
}

#[test]
fn skinparam_state_keys_accepted_without_warnings() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("styling/valid_skinparam_state.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn skinparam_state_colors_appear_in_svg() {
    let src = fs::read_to_string(fixture("styling/valid_skinparam_state.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("state skinparam svg should render");
    assert!(svg.starts_with("<svg"), "should be valid svg");
    assert!(
        svg.contains("#fef3c7"),
        "StateBackgroundColor #fef3c7 should appear in SVG"
    );
    assert!(
        svg.contains("#d97706"),
        "StateBorderColor #d97706 should appear in SVG"
    );
    assert!(
        svg.contains("#b45309"),
        "StateArrowColor #b45309 should appear in SVG"
    );
    assert!(
        svg.contains("#1c1917"),
        "StateStartColor #1c1917 should appear in SVG"
    );
}

#[test]
fn skinparam_component_keys_accepted_without_warnings() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("styling/valid_skinparam_component.puml"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn skinparam_component_colors_appear_in_svg() {
    let src = fs::read_to_string(fixture("styling/valid_skinparam_component.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("component skinparam svg should render");
    assert!(svg.starts_with("<svg"), "should be valid svg");
    assert!(
        svg.contains("#f0fdf4"),
        "ComponentBackgroundColor #f0fdf4 should appear in SVG"
    );
    assert!(
        svg.contains("#16a34a"),
        "ComponentBorderColor #16a34a should appear in SVG"
    );
    assert!(
        svg.contains("#15803d"),
        "ComponentArrowColor #15803d should appear in SVG"
    );
}

#[test]
fn family_notes_render_for_core_uml_families() {
    let cases = [
        (
            "@startuml\nclass Order\nnote right of Order: validates totals\n@enduml\n",
            "validates totals",
        ),
        (
            "@startuml\ncomponent API\nnote right of API: public facade\n@enduml\n",
            "public facade",
        ),
        (
            "@startuml\nstart\n:Build;\nnote top: lane detail\nstop\n@enduml\n",
            "lane detail",
        ),
    ];

    for (src, note_text) in cases {
        let svg = render_source_to_svg(src).expect("family note svg should render");
        assert!(svg.contains("#fff8c4"), "note card fill should render");
        assert!(svg.contains(note_text), "note text should render");
    }
}

#[test]
fn skinparam_activity_keys_accepted_without_warnings() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", &fixture("styling/valid_skinparam_activity.puml")])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn skinparam_activity_colors_appear_in_svg() {
    let src = fs::read_to_string(fixture("styling/valid_skinparam_activity.puml")).unwrap();
    let svg = render_source_to_svg(&src).expect("activity skinparam svg should render");
    assert!(svg.starts_with("<svg"), "should be valid svg");
    assert!(
        svg.contains("#fdf4ff"),
        "ActivityBackgroundColor #fdf4ff should appear in SVG"
    );
    assert!(
        svg.contains("#9333ea"),
        "ActivityBorderColor #9333ea should appear in SVG"
    );
    assert!(
        svg.contains("#f3e8ff"),
        "ActivityDiamondBackgroundColor #f3e8ff should appear in SVG"
    );
    assert!(
        svg.contains("#3b0764"),
        "ActivityBarColor #3b0764 should appear in SVG"
    );
    assert!(
        svg.contains("#7e22ce"),
        "ActivityArrowColor #7e22ce should appear in SVG"
    );
}

#[test]
fn family_theme_applies_to_class_state_component_activity_timing_and_chart() {
    let cases = [
        (
            "@startuml\n!theme vibrant\nclass Demo\n@enduml\n",
            ["#ede9fe", "#7c3aed"],
        ),
        (
            "@startuml\n!theme vibrant\n[*] --> Ready\n@enduml\n",
            ["#ede9fe", "#7c3aed"],
        ),
        (
            "@startuml\n!theme vibrant\ncomponent API\n@enduml\n",
            ["#ede9fe", "#7c3aed"],
        ),
        (
            "@startuml\n!theme vibrant\nstart\n:Ship it;\nstop\n@enduml\n",
            ["#ede9fe", "#7c3aed"],
        ),
        (
            "@startuml\n!theme vibrant\nclock clk\n@0\nclk is high\n@enduml\n",
            ["#ede9fe", "#7c3aed"],
        ),
        (
            "@startchart\n!theme vibrant\nbar\n\"A\" 1\n@endchart\n",
            ["#7c3aed", "#6d28d9"],
        ),
    ];

    for (src, expected) in cases {
        let svg = render_source_to_svg(src).expect("themed family should render");
        for color in expected {
            assert!(
                svg.contains(color),
                "expected themed color {color} in SVG: {svg}"
            );
        }
    }
}

#[test]
fn timing_skinparam_colors_are_accepted_and_rendered() {
    let src = "@startuml\nskinparam TimingBackgroundColor #101820\nskinparam TimingAxisColor #f2aa4c\nskinparam TimingGridColor #5f6f89\nskinparam TimingSignalBackgroundColor #dbeafe\nskinparam TimingSignalBorderColor #1d4ed8\nskinparam TimingArrowColor #dc2626\nskinparam TimingFontColor #f8fafc\nclock clk\n@0\nclk is high\n@enduml\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin(src)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    let svg = render_source_to_svg(src).expect("timing skinparam svg should render");
    for color in [
        "#101820", "#f2aa4c", "#5f6f89", "#dbeafe", "#1d4ed8", "#dc2626", "#f8fafc",
    ] {
        assert!(svg.contains(color), "expected timing color {color}");
    }
}

#[test]
fn chart_skinparam_colors_are_accepted_and_rendered() {
    let src = "@startchart\nskinparam ChartBackgroundColor #fff7ed\nskinparam ChartAxisColor #9a3412\nskinparam ChartGridColor #fed7aa\nskinparam ChartSeriesColor #0f766e\nskinparam ChartBarColor #ea580c\nskinparam ChartLineColor #0369a1\nskinparam ChartPieBorderColor #431407\nskinparam ChartFontColor #7c2d12\nbar\n\"A\" 4\n\"B\" 9\n@endchart\n";
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--check", "-"])
        .write_stdin(src)
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());

    let svg = render_source_to_svg(src).expect("chart skinparam svg should render");
    for color in ["#fff7ed", "#9a3412", "#ea580c", "#7c2d12"] {
        assert!(svg.contains(color), "expected chart color {color}");
    }

    let line_svg = render_source_to_svg(
        "@startchart\nskinparam ChartLineColor #0369a1\nline\n\"A\" 1\n\"B\" 2\n@endchart\n",
    )
    .expect("chart line skinparam svg should render");
    assert!(line_svg.contains("#0369a1"));

    let pie_svg = render_source_to_svg(
        "@startchart\nskinparam ChartSeriesColor #0f766e\nskinparam ChartPieBorderColor #431407\npie\n\"A\" 1\n\"B\" 2\n@endchart\n",
    )
    .expect("chart pie skinparam svg should render");
    assert!(pie_svg.contains("#0f766e"));
    assert!(pie_svg.contains("#431407"));
}

#[test]
fn preprocessor_scoped_globals_range_and_safe_aliases_expand() {
    let src = "@startuml\n!$status = outer\n!procedure Update($name)\n!local $status = local\n!global $shared = %map_set(%map(\"name\", $name), \"tags\", %range(1, 3))\nA -> B : $status\n!endprocedure\n!Update(Ada)\nA -> B : $status/%join(%dict_get($shared, \"tags\"), \"-\")/%dict_get($shared, \"name\", \"missing\")/%json_contains_key($shared, \"tags\")/%now()/%random_int()/%uuid()\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let labels = model
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            SequenceEventKind::Message { label, .. } => label.clone(),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        labels,
        vec![
            "local",
            "outer/1-2-3/Ada/true//0/00000000-0000-0000-0000-000000000000",
        ]
    );
}

#[test]
fn preprocessor_recursive_macro_expansion_is_depth_guarded() {
    let src = "@startuml\n!define WHO Alice\n!define TARGET WHO\nTARGET -> Bob : hi\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    assert_eq!(model.participants[0].id, "Alice");

    let err = parse("@startuml\n!define A A A\nA -> B : loop\n@enduml\n")
        .expect_err("recursive macro growth should be bounded");
    assert!(err.message.contains("E_PREPROC_MACRO_DEPTH"));
}

#[test]
fn preprocessor_unsafe_io_aliases_and_malformed_collections_report_stable_codes() {
    let unsafe_err = parse("@startuml\nA -> B : %file_exists(\"secret.txt\")\n@enduml\n")
        .expect_err("filesystem-sensitive builtin should be rejected");
    assert!(unsafe_err.message.contains("E_PREPROC_UNSAFE_BUILTIN"));

    let syntax_err = parse("@startuml\nA -> B : %list_get([\"a\", 0)\n@enduml\n")
        .expect_err("unbalanced collection argument should fail");
    assert!(syntax_err.message.contains("E_PREPROC_CALL_SYNTAX"));
}

// ─── Issue #240: MindMap/WBS layout — radial/tree not DAG grid ────────────────
