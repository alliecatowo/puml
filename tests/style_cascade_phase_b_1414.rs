//! Integration tests for Phase B of epic #1404 — cascade resolver +
//! `<style>` block integration with class and component families.
//!
//! Each test exercises a complete parse → normalize → render pipeline using
//! minimal PUML fixtures so the `<style>` block rules flow all the way through
//! to the rendered SVG output.
//!
//! # Coverage
//!
//! 1. **Simple selector** — `class { backgroundColor #foo }` applied to a
//!    class node produces the correct fill colour in the SVG.
//! 2. **Specificity** — `.entity { color #blue }` beats `class { color #red }`
//!    on a node that carries the `<<entity>>` stereotype.
//! 3. **Cascade order** — `skinparam classBackgroundColor #grey` (tier 3) loses
//!    to `style class { BackgroundColor #blue }` (tier 5).
//! 4. **Backward compat** — the existing `skinparam`-only path still produces
//!    correct output when no `<style>` block is present (the `StyleParam`
//!    compat shim was removed in Phase E #1417; skinparam continues to work).

/// Parse, normalize, and render a PUML source string to SVG; panics on errors.
fn render_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("render must succeed")
}

// ---------------------------------------------------------------------------
// 1. Simple selector — `class { BackgroundColor #dbeafe }` applies to nodes
// ---------------------------------------------------------------------------
/// A `<style>` block with a bare `class { BackgroundColor ... }` rule must
/// propagate the colour to class node fill in the rendered SVG.
#[test]
fn simple_class_selector_applies_background() {
    let src = r#"
@startuml
<style>
classDiagram {
  class {
    BackgroundColor #dbeafe
  }
}
</style>
class Foo
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("#dbeafe"),
        "SVG must contain the style-block background colour #dbeafe; got:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

// ---------------------------------------------------------------------------
// 2. Specificity — stereotyped rule beats plain rule
// ---------------------------------------------------------------------------
/// A `class<<service>>` stereotype selector (`+100 + +1000` specificity) must
/// beat the plain `class { ... }` rule (`+100` specificity) when the node
/// carries the `<<service>>` stereotype.  The plain node uses the `class`
/// colour; the stereotyped node uses the higher-specificity colour.
#[test]
fn stereotype_specificity_beats_plain_class_rule() {
    let src = r#"
@startuml
<style>
classDiagram {
  class {
    BackgroundColor #ff0000
  }
  class<<service>> {
    BackgroundColor #0000ff
  }
}
</style>
class Foo
class Bar <<service>>
@enduml
"#;
    let svg = render_svg(src);
    // Both colours must appear: plain for Foo, stereotype for Bar.
    assert!(
        svg.contains("#0000ff"),
        "SVG must contain the stereotype fill #0000ff for <<service>> node; svg snippet:\n{}",
        &svg[..svg.len().min(2000)]
    );
    assert!(
        svg.contains("#ff0000"),
        "SVG must contain the plain fill #ff0000 for non-stereotyped node; svg snippet:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

// ---------------------------------------------------------------------------
// 3. Cascade tier 5 beats tier 3 (style block > skinparam)
// ---------------------------------------------------------------------------
/// `skinparam classBackgroundColor #888888` is tier 3.  A `<style>` block
/// setting `BackgroundColor #2563eb` is tier 5 and must win.
#[test]
fn style_block_beats_skinparam() {
    let src = r#"
@startuml
skinparam classBackgroundColor #888888
<style>
classDiagram {
  class {
    BackgroundColor #2563eb
  }
}
</style>
class Node
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("#2563eb"),
        "style block colour #2563eb must override skinparam #888888; svg snippet:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

// ---------------------------------------------------------------------------
// 4. Compat shim — skinparam-only path still works without a <style> block
// ---------------------------------------------------------------------------
/// When there is no `<style>` block, the legacy skinparam cascade must still
/// produce the correct fill colour (backward compat: the builder is `None` and
/// the cascade falls back to the skinparam tier).
#[test]
fn compat_shim_skinparam_only_path_works() {
    let src = r#"
@startuml
skinparam classBackgroundColor #f0fdf4
class OnlyNode
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("#f0fdf4"),
        "skinparam-only path must still produce correct fill; svg snippet:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

// ---------------------------------------------------------------------------
// 5. Component family — style block applies to component nodes
// ---------------------------------------------------------------------------
/// A `<style>` block targeting `componentDiagram { component { ... } }` must
/// flow through to component node fills in the rendered SVG.
#[test]
fn component_family_style_block_applies() {
    let src = r#"
@startuml
<style>
componentDiagram {
  component {
    BackgroundColor #f0abfc
  }
}
</style>
component "StyledComp" as c
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("#f0abfc"),
        "style block colour #f0abfc must appear in component SVG; svg snippet:\n{}",
        &svg[..svg.len().min(2000)]
    );
}
