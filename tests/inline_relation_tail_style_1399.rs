//! Integration tests for inline relation tail-style syntax (issue #1399).
//!
//! PlantUML supports `A --> B #color;line.dashed;text:color` after the RHS
//! endpoint. PUML parses this via `pre_strip_inline_relation_style` +
//! `parse_rhs_inline_relation_style` and plumbs the result through the AST,
//! normalize, and SVG renderer.
//!
//! Acceptance criteria (from #1399):
//! - `A --> B #red` colors the arrow stroke red
//! - `A --> B #line:red;line.bold;text:blue` applies all three
//! - Works for class, component, usecase, deployment families
//! - Existing bracket-form `[#red,line.bold]` continues to work unchanged
//! - Tests covering: trailing color, `line:`, `line.`, `text:`, combinations
//!
//! Color names are normalised to CSS hex by the color parser:
//!   red → #ff0000, blue → #0000ff, green → #008000

// ──────────────────────────────────────────────────────────────────
// Helper: render and assert SVG contains a needle
// ──────────────────────────────────────────────────────────────────

fn render_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("render failed")
}

fn assert_contains(svg: &str, needle: &str, context: &str) {
    assert!(
        svg.contains(needle),
        "{context}: expected to find `{needle}` in SVG output"
    );
}

fn assert_not_contains(svg: &str, needle: &str, context: &str) {
    assert!(
        !svg.contains(needle),
        "{context}: expected NOT to find `{needle}` in SVG output"
    );
}

// ──────────────────────────────────────────────────────────────────
// 1. Bare trailing color: `A --> B #red` → stroke="#ff0000"
// ──────────────────────────────────────────────────────────────────

#[test]
fn test_tail_bare_color_applied_to_stroke() {
    let svg = render_svg(
        r#"
@startuml
class A
class B
A --> B #red
@enduml
"#,
    );
    // Color names are normalised to CSS hex by the color parser.
    assert_contains(
        &svg,
        "stroke=\"#ff0000\"",
        "bare trailing color → red stroke",
    );
    // The node "B" must still be rendered (not swallowed as a label).
    assert_contains(&svg, ">B<", "node B present");
}

// ──────────────────────────────────────────────────────────────────
// 2. `line:blue` prefix form → stroke="#0000ff"
// ──────────────────────────────────────────────────────────────────

#[test]
fn test_tail_line_colon_color() {
    let svg = render_svg(
        r#"
@startuml
class A
class B
A --> B #line:blue
@enduml
"#,
    );
    assert_contains(
        &svg,
        "stroke=\"#0000ff\"",
        "line:blue → blue stroke",
    );
    assert_contains(&svg, ">B<", "node B present");
}

// ──────────────────────────────────────────────────────────────────
// 3. `line.dashed` — dashed pattern applied
// ──────────────────────────────────────────────────────────────────

#[test]
fn test_tail_line_dot_dashed() {
    let svg = render_svg(
        r#"
@startuml
class A
class B
A --> B #line.dashed
@enduml
"#,
    );
    assert_contains(&svg, "stroke-dasharray", "line.dashed → stroke-dasharray");
}

// ──────────────────────────────────────────────────────────────────
// 4. `line.bold` — thick stroke applied
// ──────────────────────────────────────────────────────────────────

#[test]
fn test_tail_line_dot_bold() {
    let svg = render_svg(
        r#"
@startuml
class A
class B
A --> B #line.bold
@enduml
"#,
    );
    // Bold sets thickness ≥ 3; default is 2.
    assert_contains(&svg, "stroke-width=\"3\"", "line.bold → stroke-width 3");
}

// ──────────────────────────────────────────────────────────────────
// 5. `text:blue` — label color applied → fill="#0000ff"
// ──────────────────────────────────────────────────────────────────

#[test]
fn test_tail_text_color_applied_to_label() {
    let svg = render_svg(
        r#"
@startuml
class A
class B
A --> B #text:blue : my label
@enduml
"#,
    );
    // The label text element must carry the overridden fill color.
    assert_contains(
        &svg,
        "fill=\"#0000ff\"",
        "text:blue → blue label fill",
    );
    assert_contains(&svg, "my label", "label text rendered");
}

// ──────────────────────────────────────────────────────────────────
// 6. Full combination: `#line:red;line.bold;text:blue`
// ──────────────────────────────────────────────────────────────────

#[test]
fn test_tail_full_combination() {
    let svg = render_svg(
        r#"
@startuml
class A
class B
A --> B #line:red;line.bold;text:blue : combined
@enduml
"#,
    );
    assert_contains(&svg, "stroke=\"#ff0000\"", "full combo → red stroke");
    assert_contains(&svg, "stroke-width=\"3\"", "full combo → bold stroke");
    assert_contains(&svg, "fill=\"#0000ff\"", "full combo → blue label fill");
    assert_contains(&svg, "combined", "label rendered");
}

// ──────────────────────────────────────────────────────────────────
// 7. Existing bracket-form `[#green,dashed]` still works
// ──────────────────────────────────────────────────────────────────

#[test]
fn test_bracket_form_unchanged() {
    let svg = render_svg(
        r#"
@startuml
class A
class B
A -[#green,dashed]-> B
@enduml
"#,
    );
    assert_contains(&svg, "stroke=\"#008000\"", "bracket form → green stroke");
    assert_contains(&svg, "stroke-dasharray", "bracket form → dashed");
}

// ──────────────────────────────────────────────────────────────────
// 8. Works in component family
// ──────────────────────────────────────────────────────────────────

#[test]
fn test_component_family_tail_style() {
    let svg = render_svg(
        r#"
@startuml
[Comp A] --> [Comp B] #red
@enduml
"#,
    );
    assert_contains(
        &svg,
        "stroke=\"#ff0000\"",
        "component family → red stroke",
    );
}

// ──────────────────────────────────────────────────────────────────
// 9. Works in usecase family
// ──────────────────────────────────────────────────────────────────

#[test]
fn test_usecase_family_tail_style() {
    let svg = render_svg(
        r#"
@startuml
actor User
usecase UC as "Log In"
User --> UC #blue
@enduml
"#,
    );
    assert_contains(
        &svg,
        "stroke=\"#0000ff\"",
        "usecase family → blue stroke",
    );
}

// ──────────────────────────────────────────────────────────────────
// 10. Tail style with label: label is not consumed as a node name
// ──────────────────────────────────────────────────────────────────

#[test]
fn test_tail_style_label_colon_split() {
    let svg = render_svg(
        r#"
@startuml
class A
class B
A --> B #red : hello
@enduml
"#,
    );
    assert_contains(&svg, "stroke=\"#ff0000\"", "tail + label → red stroke");
    assert_contains(&svg, "hello", "tail + label → label rendered");
    // The raw #red token must be stripped and not appear in label text.
    assert_not_contains(&svg, ">#red<", "tail token stripped from label");
}

// ──────────────────────────────────────────────────────────────────
// 11. `#green;line.dashed` combination (bare color + line modifier)
// ──────────────────────────────────────────────────────────────────

#[test]
fn test_tail_color_and_dashed() {
    let svg = render_svg(
        r#"
@startuml
class A
class B
A --> B #green;line.dashed
@enduml
"#,
    );
    assert_contains(&svg, "stroke=\"#008000\"", "color;dashed → green stroke");
    assert_contains(&svg, "stroke-dasharray", "color;dashed → dashed");
}

// ──────────────────────────────────────────────────────────────────
// 12. Fixture file round-trip (all forms in one diagram)
// ──────────────────────────────────────────────────────────────────

#[test]
fn test_fixture_inline_tail_style_round_trip() {
    let src = include_str!("fixtures/families/valid_class_inline_tail_style.puml");
    let svg = render_svg(src);
    // Red and dashed strokes appear from the fixture
    assert_contains(&svg, "stroke=\"#ff0000\"", "fixture: red stroke");
    assert_contains(&svg, "stroke-dasharray", "fixture: dashed stroke");
    assert_contains(&svg, "labeled", "fixture: label rendered");
    // Green stroke from D --> E #green;line.bold
    assert_contains(&svg, "stroke=\"#008000\"", "fixture: green stroke");
    // Blue label fill from C --> F #line:red;line.bold;text:blue
    assert_contains(&svg, "fill=\"#0000ff\"", "fixture: blue label fill");
}
