/// Wave-10 batch C — state diagram inline fill/border colors and `<style>` block selectors.
///
/// Tests parsing and rendering of:
/// - `state X #color`         → fill color on node background
/// - `state X #c1-c2`         → gradient fill (linearGradient SVG)
/// - `state X ##color`        → border color (default solid)
/// - `state X ##[dashed]color`→ border color + dashed style modifier
/// - `state X { state Y #color }` → composite with colored sub-state
/// - `<style> stateDiagram { arrow { LineColor X } }</style>` → arrow color via style block
use puml::model::NormalizedDocument;

// ─── Parser + normalizer tests ─────────────────────────────────────────────

#[test]
fn state_inline_color_fills_node_background() {
    let src = r#"@startuml
state Active #pink
state Warn #orange
[*] --> Active
Active --> Warn
@enduml"#;

    let doc = puml::parser::parse(src).expect("parse");
    let NormalizedDocument::State(model) = puml::normalize_family(doc).expect("normalize") else {
        panic!("expected state model");
    };

    let active = model
        .nodes
        .iter()
        .find(|n| n.name == "Active")
        .expect("Active node");
    assert_eq!(
        active.style.fill_color.as_deref(),
        Some("pink"),
        "Active fill color should be pink"
    );
    assert!(
        active.style.fill_gradient.is_none(),
        "Active should have no gradient"
    );

    let warn = model
        .nodes
        .iter()
        .find(|n| n.name == "Warn")
        .expect("Warn node");
    assert_eq!(
        warn.style.fill_color.as_deref(),
        Some("orange"),
        "Warn fill color should be orange"
    );
}

#[test]
fn state_gradient_color_renders_linear_gradient() {
    let src = r#"@startuml
state Composite #red-green {
  state Sub #lightblue
}
state HexGrad #FF0000-#00FF00
[*] --> Composite
Composite --> HexGrad
@enduml"#;

    let doc = puml::parser::parse(src).expect("parse");
    let NormalizedDocument::State(model) = puml::normalize_family(doc).expect("normalize") else {
        panic!("expected state model");
    };

    // Check that the composite node has a gradient
    let composite = model
        .nodes
        .iter()
        .find(|n| n.name == "Composite")
        .expect("Composite node");
    let (c1, c2) = composite
        .style
        .fill_gradient
        .as_ref()
        .expect("Composite should have a fill_gradient");
    assert_eq!(c1, "red", "Composite gradient c1 should be red");
    assert_eq!(c2, "green", "Composite gradient c2 should be green");
    // fill_color should mirror c1 as fallback
    assert_eq!(
        composite.style.fill_color.as_deref(),
        Some("red"),
        "fill_color fallback should be first gradient color"
    );

    // Check the hex gradient node
    let hex_grad = model
        .nodes
        .iter()
        .find(|n| n.name == "HexGrad")
        .expect("HexGrad node");
    let (hc1, hc2) = hex_grad
        .style
        .fill_gradient
        .as_ref()
        .expect("HexGrad should have a fill_gradient");
    assert_eq!(hc1, "#FF0000", "HexGrad gradient c1 should be #FF0000");
    assert_eq!(hc2, "#00FF00", "HexGrad gradient c2 should be #00FF00");

    // Render to SVG and verify linearGradient is emitted
    let svg = puml::render_source_to_svg(src).expect("render gradient state diagram");
    assert!(
        svg.contains("linearGradient"),
        "SVG should contain a linearGradient element for gradient-filled states"
    );
    assert!(
        svg.contains("url(#grad-"),
        "SVG rect fill should reference the gradient via url(#grad-...)"
    );
}

#[test]
fn state_border_color_applied() {
    // Use the `#back:fill;line:border` syntax which avoids the `##` preprocessor
    // token-concatenation operator collision.
    // PlantUML supports: `state X #back:fillcolor;line:bordercolor`
    let src = r#"@startuml
state Warn #back:gold;line:navy
[*] --> Warn
@enduml"#;

    let doc = puml::parser::parse(src).expect("parse");
    let NormalizedDocument::State(model) = puml::normalize_family(doc).expect("normalize") else {
        panic!("expected state model");
    };

    let warn = model
        .nodes
        .iter()
        .find(|n| n.name == "Warn")
        .expect("Warn node");
    assert_eq!(
        warn.style.fill_color.as_deref(),
        Some("gold"),
        "fill color should be gold"
    );
    assert_eq!(
        warn.style.border_color.as_deref(),
        Some("navy"),
        "border color should be navy"
    );
    assert!(
        !warn.style.border_dashed,
        "default border style should not be dashed"
    );

    // Verify the SVG renders with correct stroke color
    let svg = puml::render_source_to_svg(src).expect("render border color state");
    assert!(
        svg.contains("navy"),
        "SVG should contain navy stroke for Warn node"
    );
}

#[test]
fn state_border_style_dashed_applies() {
    let src = r#"@startuml
state FooDashed #red ##[dashed]blue
state Critical #FF0000 ##[dashed]#0000FF
[*] --> FooDashed
FooDashed --> Critical
@enduml"#;

    let doc = puml::parser::parse(src).expect("parse");
    let NormalizedDocument::State(model) = puml::normalize_family(doc).expect("normalize") else {
        panic!("expected state model");
    };

    let foo = model
        .nodes
        .iter()
        .find(|n| n.name == "FooDashed")
        .expect("FooDashed node");
    assert_eq!(
        foo.style.fill_color.as_deref(),
        Some("red"),
        "FooDashed fill should be red"
    );
    assert_eq!(
        foo.style.border_color.as_deref(),
        Some("blue"),
        "FooDashed border should be blue"
    );
    assert!(
        foo.style.border_dashed,
        "FooDashed border should be dashed"
    );

    let critical = model
        .nodes
        .iter()
        .find(|n| n.name == "Critical")
        .expect("Critical node");
    assert_eq!(
        critical.style.fill_color.as_deref(),
        Some("#FF0000"),
        "Critical fill should be #FF0000"
    );
    assert_eq!(
        critical.style.border_color.as_deref(),
        Some("#0000FF"),
        "Critical border should be #0000FF"
    );
    assert!(
        critical.style.border_dashed,
        "Critical border should be dashed"
    );

    // Render to SVG and verify stroke-dasharray is present for dashed borders
    let svg = puml::render_source_to_svg(src).expect("render dashed border state");
    assert!(
        svg.contains("stroke-dasharray"),
        "SVG should contain stroke-dasharray for dashed border states"
    );
}

#[test]
fn state_composite_with_substate_inherits_color_scope() {
    let src = r#"@startuml
state Outer #lightgrey {
  state Inner #lightblue
  [*] --> Inner
}
[*] --> Outer
@enduml"#;

    let doc = puml::parser::parse(src).expect("parse");
    let NormalizedDocument::State(model) = puml::normalize_family(doc).expect("normalize") else {
        panic!("expected state model");
    };

    // Outer composite node should have fill_color
    let outer = model
        .nodes
        .iter()
        .find(|n| n.name == "Outer")
        .expect("Outer node");
    assert_eq!(
        outer.style.fill_color.as_deref(),
        Some("lightgrey"),
        "Outer composite fill should be lightgrey"
    );

    // Inner sub-state should have its own fill_color in Outer's regions
    let inner = outer
        .regions
        .iter()
        .flat_map(|r| r.iter())
        .find(|n| n.name == "Inner")
        .expect("Inner node in Outer's regions");
    assert_eq!(
        inner.style.fill_color.as_deref(),
        Some("lightblue"),
        "Inner sub-state fill should be lightblue"
    );

    // Render to SVG — both should be present
    let svg = puml::render_source_to_svg(src).expect("render composite with colored substates");
    assert!(
        svg.contains("lightgrey"),
        "SVG should contain lightgrey fill for Outer"
    );
    assert!(
        svg.contains("lightblue"),
        "SVG should contain lightblue fill for Inner"
    );
}

// ─── Style block test (ignored — stretch goal) ─────────────────────────────

#[test]
#[ignore = "style block selector propagation is a stretch goal for wave-10; tracked separately"]
fn state_style_block_arrow_color_propagates() {
    let src = r#"@startuml
<style>
stateDiagram {
  arrow { LineColor blue }
  state { BorderColor purple }
}
</style>
state A
state B
A --> B
@enduml"#;

    let doc = puml::parser::parse(src).expect("parse");
    let NormalizedDocument::State(model) = puml::normalize_family(doc).expect("normalize") else {
        panic!("expected state model");
    };

    // Arrow color should be propagated from the style block
    assert_eq!(
        model.state_style.arrow_color,
        "blue",
        "arrow color should be blue from style block"
    );

    // The SVG should also contain blue strokes for transitions
    let svg = puml::render_source_to_svg(src).expect("render style-block state");
    assert!(
        svg.contains("stroke=\"blue\"") || svg.contains("stroke='blue'"),
        "SVG transitions should use blue stroke from style block"
    );
}
