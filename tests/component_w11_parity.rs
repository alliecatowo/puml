//! Wave-11 batch B: component diagram parity tests for nested packages,
//! interface alias `()` syntax, and named port declarations.
//!
//! Acceptance criteria:
//!  - Nested `package` blocks render the inner frame INSIDE the outer frame.
//!  - `() "Label" as ID` (interface alias) renders as a circle + label, with `ID`
//!    usable in relations.
//!  - `port NAME` inside a `component "X" [...]` block creates a named Port node
//!    scoped as `X::NAME`.
//!  - Relations like `[Component::port_name] --> [Other]` anchor at the port node.

fn render_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("component diagram should render without error")
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: Nested packages render Inner frame inside Outer frame
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn component_nested_packages_render_inner_inside_outer() {
    let src = r#"@startuml
package "Outer" {
  package "Inner" {
    component [Worker]
  }
  component [Bridge]
}
@enduml
"#;
    let svg = render_svg(src);

    // Both frames present with correct scopes
    assert!(
        svg.contains("class=\"uml-group-frame\" data-uml-group=\"Outer\""),
        "Outer frame should be rendered"
    );
    assert!(
        svg.contains("class=\"uml-group-frame\" data-uml-group=\"Outer::Inner\""),
        "Inner frame should be scoped under Outer"
    );

    // Package label text appears for both frames.
    // Kind-tag suppression (pass 2) strips the "package " prefix so PlantUML
    // shows only the user-supplied name, matching upstream behaviour.
    assert!(svg.contains(">Outer<"), "Outer label should appear");
    assert!(svg.contains(">Inner<"), "Inner label should appear");

    // Worker is scoped under Outer::Inner
    assert!(
        svg.contains("data-uml-id=\"Outer::Inner::Worker\""),
        "Worker should be scoped as Outer::Inner::Worker"
    );

    // Inner frame (depth=1) is rendered as a dashed sub-frame nested inside Outer
    assert!(
        svg.contains("stroke-dasharray"),
        "Inner nested frame should use dashed border style"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: `() "Label" as ID` renders as a circle node with a label
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn component_interface_alias_parentheses_renders_circle_and_label() {
    let src = r#"@startuml
() "API Gateway" as ApiGw
@enduml
"#;
    let svg = render_svg(src);

    // Renders as an interface node (circle shape)
    assert!(
        svg.contains("data-uml-kind=\"interface\""),
        "() syntax should render as interface kind"
    );
    assert!(
        svg.contains("<circle class=\"uml-node uml-interface\""),
        "interface node should be a circle element"
    );

    // The label text appears
    assert!(
        svg.contains("API Gateway"),
        "interface alias label should appear"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: Interface alias ID is usable as a relation endpoint
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn component_interface_alias_can_be_target_of_relation() {
    let src = r#"@startuml
() "HTTP API" as HttpApi
component [Service]
[Service] --> HttpApi : exposes
@enduml
"#;
    let svg = render_svg(src);

    // Interface node exists
    assert!(
        svg.contains("data-uml-kind=\"interface\""),
        "interface alias should produce an interface node"
    );

    // Relation from Service to HttpApi (alias)
    assert!(
        svg.contains("data-uml-from=\"Service\""),
        "relation from should be Service"
    );
    assert!(
        svg.contains("data-uml-to=\"HttpApi\""),
        "relation to should resolve to HttpApi alias"
    );

    // Label appears on the edge
    assert!(svg.contains("exposes"), "relation label should be rendered");
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4: `port NAME` inside a component block creates a Port node
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn component_port_declared_in_block_appears_at_component_edge() {
    let src = r#"@startuml
component "Server" [
  port http_port
  port grpc_port
]
@enduml
"#;
    let svg = render_svg(src);

    // The parent component renders correctly
    assert!(
        svg.contains("data-uml-id=\"Server\""),
        "Server component should exist"
    );
    assert!(
        svg.contains("data-uml-kind=\"component\""),
        "Server should render as a component"
    );

    // Each named port creates a Port node scoped under the component
    assert!(
        svg.contains("data-uml-id=\"Server::http_port\""),
        "http_port should be scoped as Server::http_port"
    );
    assert!(
        svg.contains("data-uml-id=\"Server::grpc_port\""),
        "grpc_port should be scoped as Server::grpc_port"
    );

    // Port nodes render with port kind (small square)
    assert!(
        svg.contains("data-uml-kind=\"port\""),
        "named port nodes should render as port kind"
    );
    assert!(
        svg.contains("class=\"uml-node uml-port\""),
        "port should use uml-port CSS class"
    );

    // Port text does NOT appear as part of the parent component's label
    // (the body lines should be stripped from the component label)
    assert!(
        !svg.contains(">port http_port<"),
        "port declaration text should not appear verbatim in the SVG label"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5: Relation anchors at named port position
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn component_port_relation_anchors_at_named_port() {
    let src = r#"@startuml
component "Server" [
  port http_port
]

[Server::http_port] --> [Frontend]
@enduml
"#;
    let svg = render_svg(src);

    // Port node exists
    assert!(
        svg.contains("data-uml-id=\"Server::http_port\""),
        "http_port port node should exist"
    );
    assert!(
        svg.contains("data-uml-kind=\"port\""),
        "http_port should render as a port kind"
    );

    // The relation is anchored at the port node (from endpoint = port)
    assert!(
        svg.contains("data-uml-from=\"Server::http_port\""),
        "relation source should be anchored at Server::http_port"
    );
    assert!(
        svg.contains("data-uml-to=\"Frontend\""),
        "relation destination should be Frontend"
    );

    // A relation edge (polyline OR path, depending on EdgeRouting mode)
    // should be present. Default is `Splines` -> <path>; `Polyline` and
    // `Ortho` modes produce <polyline>.
    assert!(
        svg.contains("<polyline class=\"uml-relation\"")
            || svg.contains("<path class=\"uml-relation\""),
        "a relation edge (polyline or path) should be rendered"
    );
}
