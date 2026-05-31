//! Regression tests for #1383 — class generics inheritance edge dropped.
//!
//! When a class is declared with generic parameters (e.g. `class Container<T>`)
//! and a relation uses the bare name (`Container <|-- Stack`), the inheritance
//! edge was silently dropped because the layout resolver and relation renderer
//! looked up `"Container"` in a map keyed by `"Container<T>"`.

fn svg_for(src: &str) -> String {
    puml::render_source_to_svg(src).expect("svg should render")
}

/// Return true when `svg` contains the attribute `key="value"`.
fn has_attr(svg: &str, key: &str, value: &str) -> bool {
    svg.contains(&format!("{key}=\"{value}\""))
}

// ── core regression: inheritance edge is present ─────────────────────────────

#[test]
fn class_generics_inheritance_edge_present() {
    let src = r#"
@startuml
class Container<T> {
  +add(item: T): void
}
class Stack<E> {
  +push(e: E): void
}
Container <|-- Stack
@enduml
"#;
    let svg = svg_for(src);

    // The edge must carry the expected from/to data attributes.
    assert!(
        has_attr(&svg, "data-uml-from", "Container"),
        "#1383: data-uml-from=\"Container\" missing — inheritance edge not emitted"
    );
    assert!(
        has_attr(&svg, "data-uml-to", "Stack"),
        "#1383: data-uml-to=\"Stack\" missing — inheritance edge not emitted"
    );

    // The edge must use the hollow-triangle (open generalization) marker.
    assert!(
        svg.contains("url(#arrow-triangle)"),
        "#1383: hollow-triangle marker missing — wrong or absent arrowhead"
    );
}

// ── generic params still displayed in class header ───────────────────────────

#[test]
fn class_generics_params_displayed() {
    let src = r#"
@startuml
class Container<T> {
  +add(item: T): void
}
class Stack<E> {
  +push(e: E): void
}
Container <|-- Stack
@enduml
"#;
    let svg = svg_for(src);

    // Generic parameters must still appear in the rendered header text.
    assert!(
        svg.contains("Container&lt;T&gt;") || svg.contains("Container<T>"),
        "generic parameter <T> must appear in class header"
    );
    assert!(
        svg.contains("Stack&lt;E&gt;") || svg.contains("Stack<E>"),
        "generic parameter <E> must appear in class header"
    );
}

// ── full fixture from docs/examples/class/11_generics.puml ───────────────────

#[test]
fn class_generics_fixture_edge_present() {
    let src = include_str!("../docs/examples/class/11_generics.puml");
    let svg = svg_for(src);

    assert!(
        has_attr(&svg, "data-uml-from", "Container"),
        "#1383: fixture — data-uml-from=\"Container\" missing"
    );
    assert!(
        has_attr(&svg, "data-uml-to", "Stack"),
        "#1383: fixture — data-uml-to=\"Stack\" missing"
    );
    assert!(
        svg.contains("url(#arrow-triangle)"),
        "#1383: fixture — hollow-triangle marker missing"
    );
}
