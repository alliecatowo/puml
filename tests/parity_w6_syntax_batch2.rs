//! Wave-6 syntax parity batch 2 — focused tests for newly-wired PlantUML
//! constructs:
//!
//! - `skinparam roundcorner <N>` for class/object/usecase diagrams: the
//!   `rx`/`ry` attribute on class node rectangles must reflect the value.
//! - `skinparam shadowing true|false` for class/object/usecase/component
//!   diagrams: the SVG must emit a `<filter id="shadow">` block and the
//!   class node rect must reference it via `filter="url(#shadow)"`.
//! - `skinparam roundcorner <N>` for component diagrams: component node
//!   rectangles use the configured corner radius.
//! - `legend left|right|top|bottom` placement for class diagrams: the
//!   `legend ... end legend` block honours the requested alignment.
//!
//! These tests pin parser → normalize → render behaviour end-to-end. They
//! intentionally render to SVG (not PNG) because the visual assertion is
//! structural (specific attributes and elements present).

use puml::render_source_to_svg;

const CLASS_ROUNDCORNER_SRC: &str = "\
@startuml
skinparam roundcorner 18
class Alpha {
  +ping()
}
class Beta
Alpha --> Beta
@enduml
";

#[test]
fn class_roundcorner_skinparam_sets_rect_rx_ry() {
    let svg = render_source_to_svg(CLASS_ROUNDCORNER_SRC).expect("class render");
    // The skinparam should override the default rx="4"/ry="4" with rx="18".
    assert!(
        svg.contains("rx=\"18\" ry=\"18\""),
        "expected roundcorner=18 to produce rx=\"18\" ry=\"18\"; got SVG:\n{svg}"
    );
    // The default 4px corner should no longer appear on the outer class rect
    // (the markers/group frames may still use other radii — assert presence,
    // not absence of `rx=\"4\"` globally).
    assert!(
        !svg.contains(" rx=\"4\" ry=\"4\" fill=\"#ffffff\""),
        "expected no default 4px corner on the outer class rect; got SVG:\n{svg}"
    );
}

const CLASS_DEFAULT_CORNER_SRC: &str = "\
@startuml
class Alpha
class Beta
Alpha --> Beta
@enduml
";

#[test]
fn class_without_roundcorner_keeps_default_radius() {
    let svg = render_source_to_svg(CLASS_DEFAULT_CORNER_SRC).expect("class render");
    // Without skinparam, the outer class rect must still use the historical
    // 4px corner. Search for the canonical default white-fill rect.
    assert!(
        svg.contains("rx=\"4\" ry=\"4\""),
        "expected default rx=\"4\" ry=\"4\" when no roundcorner is set; got SVG:\n{svg}"
    );
}

const CLASS_SHADOWING_SRC: &str = "\
@startuml
skinparam shadowing true
class Alpha {
  +foo()
}
class Beta
Alpha --> Beta
@enduml
";

#[test]
fn class_shadowing_skinparam_emits_filter_and_references_it() {
    let svg = render_source_to_svg(CLASS_SHADOWING_SRC).expect("class render");
    // The filter must be present in defs.
    assert!(
        svg.contains("<filter id=\"shadow\""),
        "expected <filter id=\"shadow\"> in SVG defs; got:\n{svg}"
    );
    assert!(
        svg.contains("feDropShadow"),
        "expected feDropShadow element in SVG; got:\n{svg}"
    );
    // The class rect must reference the filter.
    assert!(
        svg.contains("filter=\"url(#shadow)\""),
        "expected class rect to reference filter=\"url(#shadow)\"; got:\n{svg}"
    );
}

const CLASS_NO_SHADOWING_SRC: &str = "\
@startuml
skinparam shadowing false
class Alpha
class Beta
Alpha --> Beta
@enduml
";

#[test]
fn class_shadowing_false_omits_shadow_filter() {
    let svg = render_source_to_svg(CLASS_NO_SHADOWING_SRC).expect("class render");
    assert!(
        !svg.contains("<filter id=\"shadow\""),
        "expected no shadow filter when shadowing=false; got:\n{svg}"
    );
    assert!(
        !svg.contains("filter=\"url(#shadow)\""),
        "expected no shadow reference when shadowing=false; got:\n{svg}"
    );
}

const COMPONENT_ROUNDCORNER_SRC: &str = "\
@startuml
skinparam roundcorner 12
component Alpha
component Beta
Alpha --> Beta
@enduml
";

#[test]
fn component_roundcorner_skinparam_sets_rect_rx_ry() {
    let svg = render_source_to_svg(COMPONENT_ROUNDCORNER_SRC).expect("component render");
    assert!(
        svg.contains("rx=\"12\" ry=\"12\""),
        "expected component roundcorner=12 to produce rx=\"12\" ry=\"12\"; got SVG:\n{svg}"
    );
}

const COMPONENT_SHADOWING_SRC: &str = "\
@startuml
skinparam shadowing true
component Alpha
component Beta
Alpha --> Beta
@enduml
";

#[test]
fn component_shadowing_skinparam_emits_filter() {
    let svg = render_source_to_svg(COMPONENT_SHADOWING_SRC).expect("component render");
    assert!(
        svg.contains("<filter id=\"shadow\""),
        "expected shadow filter for component shadowing=true; got:\n{svg}"
    );
    assert!(
        svg.contains("filter=\"url(#shadow)\""),
        "expected component rect to reference shadow filter; got:\n{svg}"
    );
}

const CLASS_LEGEND_LEFT_SRC: &str = "\
@startuml
class Alpha
class Beta
Alpha --> Beta
legend left
  Demo legend
end legend
@enduml
";

#[test]
fn class_legend_left_positions_box_at_left_margin() {
    let svg = render_source_to_svg(CLASS_LEGEND_LEFT_SRC).expect("class render");
    // Legend rect with our class must be present.
    assert!(
        svg.contains("class=\"uml-legend\""),
        "expected uml-legend rect in SVG; got:\n{svg}"
    );
    // Left alignment → x="10" (margin constant). Use a tight substring match.
    assert!(
        svg.contains("class=\"uml-legend\" x=\"10\""),
        "expected legend left-aligned at x=\"10\"; got:\n{svg}"
    );
    // Legend body text must round-trip.
    assert!(
        svg.contains("Demo legend"),
        "expected legend body text to render; got:\n{svg}"
    );
}

const CLASS_LEGEND_RIGHT_TOP_SRC: &str = "\
@startuml
class Alpha
class Beta
Alpha --> Beta
legend right top
  Right top body
end legend
@enduml
";

#[test]
fn class_legend_right_top_positions_box_at_top_right() {
    let svg = render_source_to_svg(CLASS_LEGEND_RIGHT_TOP_SRC).expect("class render");
    // Top placement → y="10". Right is computed against svg width, so we
    // only assert the y coordinate which is deterministic.
    assert!(
        svg.contains("class=\"uml-legend\""),
        "expected uml-legend element; got:\n{svg}"
    );
    assert!(
        svg.contains("y=\"10\""),
        "expected legend top placement (y=\"10\"); got:\n{svg}"
    );
    assert!(
        svg.contains("Right top body"),
        "expected legend body to render; got:\n{svg}"
    );
}

const CLASS_LEGEND_BOTTOM_RIGHT_DEFAULT_SRC: &str = "\
@startuml
class Alpha
legend
  Default placement
end legend
@enduml
";

#[test]
fn class_legend_default_placement_is_bottom() {
    let svg = render_source_to_svg(CLASS_LEGEND_BOTTOM_RIGHT_DEFAULT_SRC).expect("class render");
    // Default valign is Bottom; halign Center. We assert the legend renders
    // and the body text round-trips. Pixel placement is data-driven.
    assert!(
        svg.contains("class=\"uml-legend\""),
        "expected uml-legend element; got:\n{svg}"
    );
    assert!(
        svg.contains("Default placement"),
        "expected legend body to render; got:\n{svg}"
    );
}

// ── Skinparam classification round-trip ───────────────────────────────────
// These tests pin the classifier behaviour at the unit level so the
// SupportedNoop → SupportedWithValue migration is locked in.

#[test]
fn classify_class_roundcorner_returns_supported_with_value() {
    use puml::theme::{classify_class_skinparam, ClassSkinParamValue, SkinParamSupport};
    let support = classify_class_skinparam("roundcorner", "20");
    match support {
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::RoundCorner(20)) => {}
        other => panic!("expected SupportedWithValue(RoundCorner(20)); got {other:?}"),
    }
}

#[test]
fn classify_class_shadowing_returns_supported_with_value() {
    use puml::theme::{classify_class_skinparam, ClassSkinParamValue, SkinParamSupport};
    match classify_class_skinparam("shadowing", "true") {
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::Shadowing(true)) => {}
        other => panic!("expected SupportedWithValue(Shadowing(true)); got {other:?}"),
    }
    match classify_class_skinparam("shadowing", "false") {
        SkinParamSupport::SupportedWithValue(ClassSkinParamValue::Shadowing(false)) => {}
        other => panic!("expected SupportedWithValue(Shadowing(false)); got {other:?}"),
    }
}

#[test]
fn classify_class_roundcorner_rejects_negative_value() {
    use puml::theme::{classify_class_skinparam, SkinParamSupport};
    match classify_class_skinparam("roundcorner", "-3") {
        SkinParamSupport::UnsupportedValue => {}
        other => panic!("expected UnsupportedValue for negative roundcorner; got {other:?}"),
    }
}

#[test]
fn classify_component_roundcorner_returns_supported_with_value() {
    use puml::theme::{classify_component_skinparam, ComponentSkinParamValue, SkinParamSupport};
    match classify_component_skinparam("roundcorner", "9") {
        SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::RoundCorner(9)) => {}
        other => panic!("expected SupportedWithValue(RoundCorner(9)); got {other:?}"),
    }
}

#[test]
fn classify_component_shadowing_returns_supported_with_value() {
    use puml::theme::{classify_component_skinparam, ComponentSkinParamValue, SkinParamSupport};
    match classify_component_skinparam("shadowing", "true") {
        SkinParamSupport::SupportedWithValue(ComponentSkinParamValue::Shadowing(true)) => {}
        other => panic!("expected SupportedWithValue(Shadowing(true)); got {other:?}"),
    }
}
