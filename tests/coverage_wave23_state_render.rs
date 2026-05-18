use puml::render_source_to_svg;

#[test]
fn composite_state_scopes_internal_start_and_end_pseudostates() {
    let src = r#"@startuml
state Parent {
  [*] --> Child
  Child --> [*]
}
[*] --> Parent
@enduml"#;

    let svg = render_source_to_svg(src).expect("state svg should render");

    assert!(
        svg.contains("data-state-from=\"[*]__in__Parent\" data-state-to=\"Child\""),
        "internal start pseudo-state should be scoped to composite"
    );
    assert!(
        svg.contains("data-state-from=\"Child\" data-state-to=\"[*]__end__Parent\""),
        "internal end pseudo-state should be scoped to composite"
    );
    assert!(
        svg.contains("data-state-from=\"[*]\" data-state-to=\"Parent\""),
        "outer transition should keep global pseudo-state"
    );
}
