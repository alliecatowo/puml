//! Chapter 2 use-case diagram parity (issue #938).

use puml::render_source_to_svgs;

#[test]
fn ch02_actor_style_awesome_emits_filled_head() {
    let src = r#"@startuml
(Seed)
skinparam actorStyle awesome
actor User
@enduml
"#;
    let pages = render_source_to_svgs(src).expect("render");
    let svg = &pages[0];
    assert!(
        svg.contains("r=\"6\"") && !svg.contains("r=\"6\" fill=\"none\""),
        "awesome actor head should be filled, not hollow stick figure"
    );
}

#[test]
fn ch02_business_usecase_and_actor_render() {
    let src = r#"@startuml
(usecase)/
actor/ Clerk
:Colon:/
@enduml
"#;
    let pages = render_source_to_svgs(src).expect("render");
    let svg = &pages[0];
    assert!(
        svg.contains("rx=\"12\""),
        "business use case uses rounded rect"
    );
    assert!(
        svg.contains("<polygon"),
        "business actor should include suit/tie marker"
    );
}

#[test]
fn ch02_newpage_splits_usecase_diagram() {
    let src = r#"@startuml
(Usecase1)
:actor1:
actor1 --> (Usecase1)
newpage Page Two
(Usecase2)
:actor2:
actor2 --> (Usecase2)
@enduml
"#;
    let pages = render_source_to_svgs(src).expect("render");
    assert_eq!(pages.len(), 2, "newpage should emit two SVG pages");
    assert!(pages[0].contains("Usecase1") || pages[0].contains("actor1"));
    assert!(pages[1].contains("Usecase2") || pages[1].contains("actor2"));
}

#[test]
fn ch02_example_fixture_has_visible_nodes_on_both_pages() {
    let src = include_str!("../docs/examples/usecase/07_ch02_parity.puml");
    let pages = render_source_to_svgs(src).expect("render");
    assert_eq!(pages.len(), 2, "example should split into two pages");
    for (i, svg) in pages.iter().enumerate() {
        assert!(
            svg.contains("<text") || svg.contains("<circle") || svg.contains("<ellipse"),
            "page {i} should contain visible diagram content, got {} bytes",
            svg.len()
        );
        assert!(svg.len() > 1500, "page {i} svg too small ({})", svg.len());
    }
}
