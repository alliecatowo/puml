//! Integration tests for Phase D of epic #1404 — wiring 10 missing `PName`
//! properties into the class and component family renderers.
//!
//! Each test exercises the full parse → normalize → render pipeline.  The
//! `<style>` block rule sets one property; the assertion checks that the
//! rendered SVG contains the expected attribute value.
//!
//! # Coverage (one test per property)
//!
//! 1. `LineThickness` — stroke-width on class node border
//! 2. `LineStyle` — stroke-dasharray on class node border (dashed)
//! 3. `LineStyle` dotted — stroke-dasharray (dotted variant)
//! 4. `Padding` — property parsed and stored without crash
//! 5. `Margin` — property parsed and stored without crash
//! 6. `RoundCorner` — rx attribute on class node rect
//! 7. `FontWeight` bold — font-weight on class label text
//! 8. `FontWeight` numeric — font-weight 300 thin
//! 9. `Shadowing` — drop-shadow filter reference in class node rect
//! 10. `HorizontalAlignment` left — text-anchor="start" on class label
//! 11. `HorizontalAlignment` right — text-anchor="end"
//! 12. `MaximumWidth` — property parsed and stored without crash
//! 13. `MinimumWidth` — property parsed and stored without crash
//! 14. `LineThickness` via component — stroke-width on component node border
//! 15. `RoundCorner` beats skinparam — `<style>` value wins over skinparam

/// Parse, normalize, and render a PUML source string to SVG; panics on errors.
fn render_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("render must succeed")
}

// ---------------------------------------------------------------------------
// 1. LineThickness — stroke-width on class node border
// ---------------------------------------------------------------------------
/// `LineThickness 4` in a `<style>` block must produce `stroke-width="4"` on
/// the class node outer rect in the rendered SVG.
#[test]
fn line_thickness_sets_stroke_width_on_class() {
    let src = r#"
@startuml
<style>
classDiagram {
  class {
    LineThickness 4
  }
}
</style>
class ThickBorder
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("stroke-width=\"4\""),
        "SVG must contain stroke-width=\"4\" from LineThickness 4; got:\n{}",
        &svg[..svg.len().min(3000)]
    );
}

// ---------------------------------------------------------------------------
// 2. LineStyle dashed — stroke-dasharray on class node border
// ---------------------------------------------------------------------------
/// `LineStyle dashed` in a `<style>` block must produce a `stroke-dasharray`
/// attribute on the class node border in the rendered SVG.
#[test]
fn line_style_dashed_produces_dasharray() {
    let src = r#"
@startuml
<style>
classDiagram {
  class {
    LineStyle dashed
  }
}
</style>
class DashedBox
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("stroke-dasharray="),
        "SVG must contain stroke-dasharray attribute from LineStyle dashed; got:\n{}",
        &svg[..svg.len().min(3000)]
    );
    // Dashed pattern uses "8 4" (distinct from the legacy "5 3" inline dashed).
    assert!(
        svg.contains("8 4"),
        "SVG stroke-dasharray must be \"8 4\" for dashed; got:\n{}",
        &svg[..svg.len().min(3000)]
    );
}

// ---------------------------------------------------------------------------
// 3. LineStyle dotted — stroke-dasharray (dotted variant)
// ---------------------------------------------------------------------------
/// `LineStyle dotted` must produce `stroke-dasharray="2 3"`.
#[test]
fn line_style_dotted_produces_dot_dasharray() {
    let src = r#"
@startuml
<style>
classDiagram {
  class {
    LineStyle dotted
  }
}
</style>
class DottedBox
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("2 3"),
        "SVG stroke-dasharray must contain \"2 3\" for dotted line style; got:\n{}",
        &svg[..svg.len().min(3000)]
    );
}

// ---------------------------------------------------------------------------
// 4. Padding — parsed and stored without crash
// ---------------------------------------------------------------------------
/// `Padding 20` must not crash the renderer.  The property is stored in the
/// effective style and is available for downstream layout use.
#[test]
fn padding_parses_and_renders_without_crash() {
    let src = r#"
@startuml
<style>
classDiagram {
  class {
    Padding 20
  }
}
</style>
class PaddedNode
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("uml-class"),
        "SVG must contain a class node when Padding is set; got:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

// ---------------------------------------------------------------------------
// 5. Margin — parsed and stored without crash
// ---------------------------------------------------------------------------
/// `Margin 15` must not crash the renderer.  The property is stored for
/// downstream layout wiring (Phase D stores it; layout pass is Phase E+).
#[test]
fn margin_parses_and_renders_without_crash() {
    let src = r#"
@startuml
<style>
classDiagram {
  class {
    Margin 15
  }
}
</style>
class MarginNode
class Other
MarginNode --> Other
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("uml-class"),
        "SVG must contain class nodes when Margin is set; got:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

// ---------------------------------------------------------------------------
// 6. RoundCorner — rx attribute on class node rect
// ---------------------------------------------------------------------------
/// `RoundCorner 18` in a `<style>` block must set `rx="18"` on the outer class
/// node rect, overriding the built-in 4px default.
#[test]
fn round_corner_sets_rx_on_class_rect() {
    let src = r#"
@startuml
<style>
classDiagram {
  class {
    RoundCorner 18
  }
}
</style>
class RoundedNode
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("rx=\"18\""),
        "SVG must contain rx=\"18\" from RoundCorner 18; got:\n{}",
        &svg[..svg.len().min(3000)]
    );
}

// ---------------------------------------------------------------------------
// 7. FontWeight bold — font-weight="700" on class label text
// ---------------------------------------------------------------------------
/// `FontWeight bold` must produce `font-weight="700"` on the class header label
/// text element.
#[test]
fn font_weight_bold_sets_700_on_class_label() {
    let src = r#"
@startuml
<style>
classDiagram {
  class {
    FontWeight bold
  }
}
</style>
class BoldLabel
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("font-weight=\"700\""),
        "SVG must contain font-weight=\"700\" from FontWeight bold; got:\n{}",
        &svg[..svg.len().min(3000)]
    );
}

// ---------------------------------------------------------------------------
// 8. FontWeight numeric 300 — font-weight="300"
// ---------------------------------------------------------------------------
/// `FontWeight 300` must produce `font-weight="300"` on the class label text.
#[test]
fn font_weight_numeric_300_sets_300_on_class_label() {
    let src = r#"
@startuml
<style>
classDiagram {
  class {
    FontWeight 300
  }
}
</style>
class ThinLabel
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("font-weight=\"300\""),
        "SVG must contain font-weight=\"300\" from FontWeight 300; got:\n{}",
        &svg[..svg.len().min(3000)]
    );
}

// ---------------------------------------------------------------------------
// 9. Shadowing — drop-shadow filter reference
// ---------------------------------------------------------------------------
/// `Shadowing true` must produce a `filter="url(#shadow)"` attribute on the
/// class node outer rect, and the shadow filter `<defs>` block in the SVG.
#[test]
fn shadowing_true_emits_shadow_filter() {
    let src = r#"
@startuml
<style>
classDiagram {
  class {
    Shadowing true
  }
}
</style>
class ShadowedNode
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("filter=\"url(#shadow)\""),
        "SVG must contain filter=\"url(#shadow)\" when Shadowing true; got:\n{}",
        &svg[..svg.len().min(3000)]
    );
    assert!(
        svg.contains("feDropShadow") || svg.contains("shadow"),
        "SVG must contain shadow filter definition when Shadowing true; got:\n{}",
        &svg[..svg.len().min(3000)]
    );
}

// ---------------------------------------------------------------------------
// 10. HorizontalAlignment left — text-anchor="start"
// ---------------------------------------------------------------------------
/// `HorizontalAlignment left` must produce `text-anchor="start"` on the class
/// header label text element.
#[test]
fn horizontal_alignment_left_sets_text_anchor_start() {
    let src = r#"
@startuml
<style>
classDiagram {
  class {
    HorizontalAlignment left
  }
}
</style>
class LeftLabel
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("text-anchor=\"start\""),
        "SVG must contain text-anchor=\"start\" from HorizontalAlignment left; got:\n{}",
        &svg[..svg.len().min(3000)]
    );
}

// ---------------------------------------------------------------------------
// 11. HorizontalAlignment right — text-anchor="end"
// ---------------------------------------------------------------------------
/// `HorizontalAlignment right` must produce `text-anchor="end"` on the class
/// header label text element.
#[test]
fn horizontal_alignment_right_sets_text_anchor_end() {
    let src = r#"
@startuml
<style>
classDiagram {
  class {
    HorizontalAlignment right
  }
}
</style>
class RightLabel
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("text-anchor=\"end\""),
        "SVG must contain text-anchor=\"end\" from HorizontalAlignment right; got:\n{}",
        &svg[..svg.len().min(3000)]
    );
}

// ---------------------------------------------------------------------------
// 12. MaximumWidth — parsed and stored without crash
// ---------------------------------------------------------------------------
/// `MaximumWidth 100` must not crash the renderer.  The value is stored and
/// available for future layout wiring.
#[test]
fn maximum_width_parses_and_renders_without_crash() {
    let src = r#"
@startuml
<style>
classDiagram {
  class {
    MaximumWidth 100
  }
}
</style>
class MaxWidth
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("uml-class"),
        "SVG must contain a class node when MaximumWidth is set; got:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

// ---------------------------------------------------------------------------
// 13. MinimumWidth — parsed and stored without crash
// ---------------------------------------------------------------------------
/// `MinimumWidth 300` must not crash the renderer.  The value is stored and
/// available for future layout wiring.
#[test]
fn minimum_width_parses_and_renders_without_crash() {
    let src = r#"
@startuml
<style>
classDiagram {
  class {
    MinimumWidth 300
  }
}
</style>
class MinWidth
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("uml-class"),
        "SVG must contain a class node when MinimumWidth is set; got:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

// ---------------------------------------------------------------------------
// 14. LineThickness on component family
// ---------------------------------------------------------------------------
/// `LineThickness 3` applied to a component diagram must produce
/// `stroke-width="3"` on the component node in the rendered SVG.
#[test]
fn line_thickness_sets_stroke_width_on_component() {
    let src = r#"
@startuml
<style>
componentDiagram {
  component {
    LineThickness 3
  }
}
</style>
[ThickComponent]
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("stroke-width=\"3\""),
        "SVG must contain stroke-width=\"3\" from LineThickness 3 on component; got:\n{}",
        &svg[..svg.len().min(3000)]
    );
}

// ---------------------------------------------------------------------------
// 15. RoundCorner in <style> beats skinparam roundcorner
// ---------------------------------------------------------------------------
/// When both `skinparam roundcorner 2` and `<style> class { RoundCorner 22 }`
/// are present, the style block value (22) must win (higher cascade tier).
#[test]
fn style_block_round_corner_beats_skinparam() {
    let src = r#"
@startuml
skinparam roundcorner 2
<style>
classDiagram {
  class {
    RoundCorner 22
  }
}
</style>
class StyledCorner
@enduml
"#;
    let svg = render_svg(src);
    // The style-block value must win: rx="22" must be present.
    assert!(
        svg.contains("rx=\"22\""),
        "SVG must contain rx=\"22\" (style-block wins over skinparam 2); got:\n{}",
        &svg[..svg.len().min(3000)]
    );
    // The skinparam value must NOT appear as the rect corner radius.
    assert!(
        !svg.contains("rx=\"2\""),
        "SVG must NOT contain rx=\"2\" (skinparam must lose to style-block); got:\n{}",
        &svg[..svg.len().min(3000)]
    );
}
