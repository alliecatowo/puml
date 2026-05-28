//! Wave-5 PlantUML syntax parity tests.
//!
//! Features implemented in this batch:
//! - 3.7  `{classifier}` as alias for `{static}` member modifier
//! - 3.35 `[plain]` bracket token explicitly resets arrow style
//! - 3.36 Inline relation tail style `A --> B #line:red;line.bold : label`
//! - 3.8  Titled member separators `== Section ==`, `__ sub __`, `.. note ..`
//!
//! Refs #1258

use puml::model::NormalizedDocument;

// ─── helpers ─────────────────────────────────────────────────────────────────

fn render_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("render should succeed")
}

fn parse_and_normalize_family(src: &str) -> puml::model::FamilyDocument {
    let parsed = puml::parser::parse(src).expect("parse should succeed");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(parsed).expect("normalize should succeed")
    else {
        panic!("expected a Family diagram");
    };
    model
}

// ─── 3.7  {classifier} alias for {static} ────────────────────────────────────

/// `{classifier}` is a PlantUML alias for `{static}` (PlantUML ref 3.7).
/// The modifier must be stored as `Static` and the member rendered with underline.
#[test]
fn classifier_modifier_treated_as_static() {
    let src = r#"@startuml
class Foo {
  {classifier} String sharedField
  {static} String alsoStatic
}
@enduml
"#;
    let model = parse_and_normalize_family(src);
    let node = model
        .nodes
        .iter()
        .find(|n| n.name == "Foo")
        .expect("Foo should exist");

    // Both {classifier} and {static} should produce MemberModifier::Static
    use puml::ast::MemberModifier;
    let has_classifier_as_static = node
        .members
        .iter()
        .any(|m| m.text.contains("sharedField") && m.modifier == Some(MemberModifier::Static));
    let has_static_member = node
        .members
        .iter()
        .any(|m| m.text.contains("alsoStatic") && m.modifier == Some(MemberModifier::Static));
    assert!(
        has_classifier_as_static,
        "{{classifier}} member should have Static modifier; members = {:?}",
        node.members
    );
    assert!(
        has_static_member,
        "{{static}} member should have Static modifier; members = {:?}",
        node.members
    );
}

/// The SVG produced for a `{classifier}` member should contain an underline
/// decoration (same as `{static}`).
#[test]
fn classifier_member_renders_underline_in_svg() {
    let src = r#"@startuml
class Account {
  {classifier} int maxRetries
}
@enduml
"#;
    let svg = render_svg(src);
    // The rendered text for a static/classifier member should carry an underline decoration.
    assert!(
        svg.contains("text-decoration=\"underline\""),
        "classifier member should render with underline decoration in SVG"
    );
    assert!(
        svg.contains("maxRetries"),
        "member text should appear in SVG"
    );
}

// ─── 3.35 `[plain]` resets arrow style ───────────────────────────────────────

/// `[plain]` inside a relation bracket should result in a non-dashed, normal-weight line
/// (class diagram context, where bracket tokens are style modifiers not colors).
#[test]
fn plain_bracket_token_produces_non_dashed_relation() {
    let src = r#"@startuml
class A
class B
A -[plain]-> B
@enduml
"#;
    let svg = render_svg(src);
    // A [plain] arrow must NOT produce a dashed line.
    assert!(
        !svg.contains("stroke-dasharray"),
        "plain relation should not be dashed"
    );
    assert!(svg.contains(">A<") || svg.contains("\"A\"") || svg.contains(">A"));
    assert!(svg.contains(">B<") || svg.contains("\"B\"") || svg.contains(">B"));
}

/// `[plain]` should be accepted without parse error or panic in class diagrams.
#[test]
fn plain_bracket_token_is_accepted_in_class_diagram() {
    let src = r#"@startuml
class Foo
class Bar
Foo -[plain]-> Bar : dependency
@enduml
"#;
    let svg = render_svg(src);
    assert!(svg.contains("Foo") && svg.contains("Bar"));
    assert!(svg.contains("dependency"));
    // No dasharray on a plain arrow
    assert!(
        !svg.contains("stroke-dasharray"),
        "plain relation should not produce a dashed line"
    );
}

// ─── 3.36 Inline relation tail style ─────────────────────────────────────────

/// `A --> B #red : label` — bare color token sets line_color on the relation.
#[test]
fn inline_tail_color_sets_relation_line_color() {
    let src = r#"@startuml
class A
class B
A --> B #red : uses
@enduml
"#;
    let svg = render_svg(src);
    // The SVG edge should carry a red stroke.
    assert!(
        svg.contains("stroke=\"#ff0000\"") || svg.contains("stroke=\"red\""),
        "inline tail color should set relation stroke to red"
    );
    assert!(svg.contains("uses"), "label should still appear");
}

/// `A --> B #line:blue;line.dashed : label` — combined color + dash style.
#[test]
fn inline_tail_line_colon_color_and_dashed_sets_both() {
    let src = r#"@startuml
class Foo
class Bar
Foo --> Bar #line:blue;line.dashed : dep
@enduml
"#;
    let svg = render_svg(src);
    // Stroke should be blue
    assert!(
        svg.contains("stroke=\"#0000ff\"") || svg.contains("stroke=\"blue\""),
        "inline tail line:blue should set stroke to blue"
    );
    // Line should be dashed
    assert!(
        svg.contains("stroke-dasharray"),
        "inline tail line.dashed should produce a dashed line"
    );
    assert!(svg.contains("dep"), "label should still appear");
}

/// `A --> B #line.bold : label` — bold makes the line thicker.
#[test]
fn inline_tail_bold_increases_stroke_width() {
    let src = r#"@startuml
class X
class Y
X --> Y #line.bold : important
@enduml
"#;
    let svg = render_svg(src);
    // Bold means stroke-width > 1 (typically 3)
    let has_thick = svg.contains("stroke-width=\"3\"")
        || svg.contains("stroke-width=\"4\"")
        || svg.contains("stroke-width=\"5\"");
    assert!(has_thick, "bold inline tail should produce a thicker line");
    assert!(svg.contains("important"));
}

/// Inline tail style without label — `A --> B #line:green` (no colon label).
#[test]
fn inline_tail_color_without_label_parses_cleanly() {
    let src = r#"@startuml
class P
class Q
P --> Q #line:green
@enduml
"#;
    let svg = render_svg(src);
    assert!(svg.contains("P") && svg.contains("Q"));
    // Green color (#008000) should appear somewhere in the SVG as the stroke
    assert!(
        svg.contains("stroke=\"#008000\"") || svg.contains("stroke=\"green\""),
        "green line color should appear in SVG as stroke"
    );
}

/// Inline tail style is parsed correctly from the relation — node name is clean.
#[test]
fn inline_tail_color_does_not_pollute_node_name() {
    let src = r#"@startuml
class Alpha
class Beta
Alpha --> Beta #red
@enduml
"#;
    let model = parse_and_normalize_family(src);
    // Beta node should exist with clean name, not "Beta #red"
    let beta = model.nodes.iter().find(|n| n.name == "Beta");
    assert!(
        beta.is_some(),
        "Beta node should exist with clean name; nodes = {:?}",
        model
            .nodes
            .iter()
            .map(|n| n.name.as_str())
            .collect::<Vec<_>>()
    );
    // Alpha node should also be clean
    let alpha = model.nodes.iter().find(|n| n.name == "Alpha");
    assert!(alpha.is_some(), "Alpha node should exist");
}

// ─── 3.8 Titled member separators ────────────────────────────────────────────

/// `-- Section Name --` inside a class body renders as a divider (line) and shows title.
#[test]
fn titled_separator_double_dash_renders_divider_and_title() {
    let src = r#"@startuml
class Vehicle {
  +brand: String
  -- Internal --
  -engineCode: String
}
@enduml
"#;
    let svg = render_svg(src);
    // Must contain a divider line element
    assert!(
        svg.contains("<line"),
        "titled separator should produce a <line> divider"
    );
    // The title text should appear in the SVG
    assert!(
        svg.contains("Internal"),
        "titled separator title should appear in SVG"
    );
    assert!(svg.contains("brand") && svg.contains("engineCode"));
}

/// `== Section ==` renders a divider line with the section title.
#[test]
fn titled_separator_double_equals_renders_divider_and_title() {
    let src = r#"@startuml
class Repository {
  +save(): void
  == Queries ==
  +findById(id: Long): Entity
}
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("<line"),
        "== separator should produce a <line> divider"
    );
    assert!(
        svg.contains("Queries"),
        "== separator title should appear in SVG"
    );
    assert!(svg.contains("save") && svg.contains("findById"));
}

/// `.. note ..` renders a divider with the note title.
#[test]
fn titled_separator_double_dot_renders_divider_and_title() {
    let src = r#"@startuml
class Config {
  +host: String
  .. connection ..
  +port: int
}
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("<line"),
        ".. separator should produce a <line> divider"
    );
    assert!(
        svg.contains("connection"),
        ".. separator title should appear in SVG"
    );
}

/// `__ private section __` renders a divider with title.
#[test]
fn titled_separator_double_underscore_renders_divider_and_title() {
    let src = r#"@startuml
class Session {
  +userId: String
  __ internals __
  -token: String
}
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("<line"),
        "__ separator should produce a <line> divider"
    );
    assert!(
        svg.contains("internals"),
        "__ separator title should appear in SVG"
    );
}

/// Bare `--` separator (no title) still draws a divider line without adding any label text.
#[test]
fn bare_separator_double_dash_renders_only_divider_line() {
    let src = r#"@startuml
class Simple {
  +attr: int
  --
  +method(): void
}
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("<line"),
        "bare -- separator should produce a <line> divider"
    );
    // There should be no spurious `--` text rendered as a member text element
    // (We check that the string ">--<" doesn't appear, i.e. the literal "--" is not a text node child)
    assert!(
        !svg.contains(">--<"),
        "bare -- separator should not appear as text content"
    );
    assert!(svg.contains("attr") && svg.contains("method"));
}
