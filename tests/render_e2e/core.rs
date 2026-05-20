use super::support::*;
use std::collections::HashSet;

#[test]
fn render_svg_is_deterministic_for_same_input() {
    let src = fixture("e2e/deterministic_sequence.puml");
    let first = puml::render_source_to_svg(&src).expect("first render should succeed");
    let second = puml::render_source_to_svg(&src).expect("second render should succeed");

    assert_eq!(first, second, "render output should be deterministic");
    assert_snapshot!("render_svg_is_deterministic_for_same_input", first);
}

#[test]
fn render_svg_pragma_teoz_boundary_keeps_sequence_render_output_stable() {
    let base = "@startuml\nparticipant A\nparticipant B\nA -> B: hello\n@enduml\n";
    let with_pragma =
        "@startuml\n!pragma teoz true\nparticipant A\nparticipant B\nA -> B: hello\n@enduml\n";

    let base_svg = puml::render_source_to_svg(base).expect("base render");
    let pragma_svg = puml::render_source_to_svg(with_pragma).expect("pragma render");

    assert_eq!(base_svg, pragma_svg);
}

#[test]
fn render_core_uml_broad_partials_surface_expected_labels() {
    let cases = [
        (
            "class",
            "@startuml\ninterface Gateway\nabstract class Shape\nGateway -[#blue,dashed]-> Shape : adapts\n@enduml\n",
            // Wave 3 renderer now uses Unicode guillemets («»), not HTML-escaped angle pairs
            vec!["Gateway", "\u{00ab}interface\u{00bb}", "Shape", "adapts"],
        ),
        (
            "object",
            "@startuml\nmap Settings {\n  theme => light\n}\nobject Runtime\nSettings --> Runtime : configures\n@enduml\n",
            vec![
                "Settings",
                // Fix #551: <<map>> marker now renders as «map» guillemet in the class header
                "\u{ab}map\u{bb}",
                "theme =&gt; light",
                "configures",
            ],
        ),
        (
            "usecase",
            "@startuml\nactor Customer as C\nusecase (Login) as UC1\nC ..> UC1 : <<include>>\n@enduml\n",
            vec!["Customer", "Login", "&lt;&lt;include&gt;&gt;"],
        ),
        (
            "activity",
            "@startuml\nstart\nswitch (kind?)\ncase (A)\n:Do A;\nendswitch\nsplit\n:one;\nsplit again\n:two;\nend split\nlabel retry\ngoto retry\nbackward: retry path;\nkill\n@enduml\n",
            // Wave 3-D (#533) suppresses "(else) A" and "branch 2" canvas literals;
            // verify the content and control-flow nodes that are still rendered
            vec![
                "switch kind?",
                "Do A",
                "goto retry",
                "backward retry path",
            ],
        ),
        (
            "state",
            "@startuml\nstate Waiting as W\nstate Choice <<choice>>\nstate Done <<end>>\n[*] --> W : begin\nW --> Choice : choose\nChoice --> Done : ok\n@enduml\n",
            vec!["Waiting", "begin", "choose", "ok"],
        ),
    ];

    for (name, src, expected) in cases {
        let svg = puml::render_source_to_svg(src).unwrap_or_else(|err| {
            panic!("{name} broad partial should render, got {}", err.message)
        });
        for needle in expected {
            assert!(
                svg.contains(needle),
                "{name} render should contain `{needle}`"
            );
        }
    }
}

#[test]
fn render_core_uml_nested_scopes_lollipops_and_relation_annotations() {
    let class_src = "@startuml\nskinparam ArrowColor #225588\nset namespaceSeparator .\npackage Domain {\n  namespace Core {\n    class Api\n    class Repo\n    Api \"1\" -[#green,dashed,thickness=3]-> \"0..*\" Repo : owns:cache\n  }\n}\n@enduml\n";
    let class_svg = puml::render_source_to_svg(class_src).expect("class scope render");
    assert!(class_svg.contains("Domain.Core.Api"));
    assert!(class_svg.contains("Domain.Core.Repo"));
    assert!(class_svg.contains("owns:cache"));
    assert!(class_svg.contains("0..*"));
    assert!(class_svg.contains("#008000"));
    assert!(class_svg.contains("stroke-dasharray"));

    let component_src = "@startuml\nskinparam ComponentArrowColor #884400\nnamespace Edge {\n  component API\n  interface Orders\n  API --() Orders : provides\n}\n@enduml\n";
    let component_svg = puml::render_source_to_svg(component_src).expect("component scope render");
    assert!(component_svg.contains("Edge::API"));
    assert!(component_svg.contains("Edge::Orders"));
    assert!(component_svg.contains("provides"));
    assert!(component_svg.contains("uml-lollipop"));
    assert!(component_svg.contains("#884400"));
}

#[test]
fn render_component_style_oracle_slice_exposes_relation_dom_semantics() {
    let src = fixture("families/valid_component_style_oracle_slice.puml");
    let svg = puml::render_source_to_svg(&src).expect("component style oracle slice should render");
    let lines = parse_svg_lines(&svg);

    let publishes = lines
        .iter()
        .find(|line| {
            line.from.as_deref() == Some("Edge::api") && line.to.as_deref() == Some("Edge::orders")
        })
        .unwrap_or_else(|| panic!("missing styled api -> orders relation in {lines:#?}"));
    assert_eq!(publishes.stroke, "#dc2626");
    assert_eq!(publishes.stroke_width, 4);
    assert_eq!(publishes.dash.as_deref(), Some("5 3"));
    assert_eq!(publishes.direction.as_deref(), Some("right"));
    assert!(
        publishes.x2 > publishes.x1,
        "right-directed relation should progress left-to-right: {publishes:?}"
    );
    assert_eq!(
        publishes.y1, publishes.y2,
        "same-row right-directed relation should stay horizontal: {publishes:?}"
    );
    assert!(
        publishes
            .relation_style
            .as_deref()
            .is_some_and(|style| style.contains("color:#dc2626")
                && style.contains("dashed")
                && style.contains("thickness:4")),
        "styled relation should publish color/dash/thickness metadata: {publishes:?}"
    );
    let marker_end = publishes
        .marker_end
        .as_deref()
        .expect("styled arrow relation should have an end marker");
    assert!(
        marker_end.starts_with("url(#uml-rel-"),
        "colored relation should use a per-relation marker: {marker_end}"
    );
    let marker_id = marker_end
        .strip_prefix("url(#")
        .and_then(|value| value.strip_suffix(')'))
        .expect("marker url should contain an id");
    assert!(
        svg.contains(&format!("id=\"{marker_id}\""))
            && svg.contains(&format!("stroke=\"{}\"", publishes.stroke)),
        "colored marker def should be emitted with the relation stroke"
    );

    let hidden = lines
        .iter()
        .find(|line| {
            line.from.as_deref() == Some("Edge::orders")
                && line.to.as_deref() == Some("Edge::https")
        })
        .unwrap_or_else(|| panic!("missing hidden orders -> https relation in {lines:#?}"));
    assert_eq!(hidden.visibility.as_deref(), Some("hidden"));
    assert!(
        hidden
            .relation_style
            .as_deref()
            .is_some_and(|style| style.contains("hidden")),
        "hidden relation should publish hidden metadata: {hidden:?}"
    );

    let port = parse_svg_rect_tags(&svg)
        .into_iter()
        .find(|tag| parse_svg_attr(tag, "data-uml-port-direction").as_deref() == Some("in"))
        .expect("portin node should expose port direction metadata");
    assert_eq!(parse_svg_attr(port, "fill").as_deref(), Some("#dbeafe"));

    let lollipops = parse_svg_circles(&svg)
        .into_iter()
        .filter(|circle| circle.class.as_deref() == Some("uml-lollipop"))
        .collect::<Vec<_>>();
    assert!(
        lollipops.iter().any(|circle| circle.stroke == "#0f766e"),
        "lollipop endpoint should inherit styled relation stroke: {lollipops:#?}"
    );
}

#[test]
fn render_activity_if_else_branches_use_distinct_columns() {
    let svg = puml::render_source_to_svg(include_str!(
        "../../docs/examples/activity/02_if_then_else.puml"
    ))
    .expect("activity if/else example should render");
    assert!(
        !svg.contains("(else) no"),
        "else control-flow marker must not render as a literal text node"
    );
    assert!(
        !svg.contains("(endif)"),
        "endif control-flow marker must not render as a literal text node"
    );
    let texts = parse_svg_texts(&svg);
    let text_x = |needle: &str| -> i32 {
        texts
            .iter()
            .find(|text| text.text == needle)
            .unwrap_or_else(|| panic!("missing text `{needle}`"))
            .x
    };

    let then_x = text_x("Return 200");
    let else_x = text_x("Return 401");
    assert_ne!(
        then_x, else_x,
        "then and else actions should occupy distinct branch columns"
    );
    assert!(
        else_x > then_x,
        "else branch should be horizontally offset from the then branch"
    );
}

#[test]
fn render_activity_nested_if_else_reserves_outer_else_column_after_inner_branch() {
    let svg = puml::render_source_to_svg(include_str!(
        "../../docs/examples/activity/03_nested_if.puml"
    ))
    .expect("nested activity if/else example should render");
    let texts = parse_svg_texts(&svg);
    let text_x = |needle: &str| -> i32 {
        texts
            .iter()
            .find(|text| text.text == needle)
            .unwrap_or_else(|| panic!("missing text `{needle}`"))
            .x
    };

    let execute_x = text_x("Execute");
    let inner_else_x = text_x("Return 403");
    let outer_else_x = text_x("Return 400");
    let branch_xs = HashSet::from([execute_x, inner_else_x, outer_else_x]);

    assert!(
        branch_xs.len() >= 3,
        "nested if/else should use separate columns for then, inner else, and outer else: {branch_xs:?}"
    );
    assert!(
        execute_x < inner_else_x && inner_else_x < outer_else_x,
        "outer else should be placed beyond the inner else branch"
    );
}
