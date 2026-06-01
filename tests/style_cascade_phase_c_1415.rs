//! Integration tests for Phase C of epic #1404 — `<style>` block cascade
//! wired into the sequence and activity families.
//!
//! Each test exercises a complete parse → normalize → render pipeline using
//! minimal PUML fixtures so `<style>` block rules flow all the way through
//! to the rendered SVG output.
//!
//! # Coverage
//!
//! 1. **Sequence bare selector** — `sequenceDiagram { participant { ... } }`
//!    applies to all participants in a sequence diagram.
//! 2. **Sequence style_block beats skinparam** — `<style>` block (tier 5) wins
//!    over `skinparam` (tier 3) for sequence participants.
//! 3. **Activity bare selector** — `activityDiagram { activity { ... } }`
//!    applies to activity nodes.
//! 4. **Activity style_block beats skinparam** — `<style>` block wins over
//!    `skinparam` for activity nodes.
//! 5. **Activity font color** — FontColor property applied via `<style>` block.
//! 6. **No-style fallback** — diagrams without a `<style>` block use defaults.
//! 7. **Phase B regression** — class/component families still work correctly.

/// Parse, normalize, and render a PUML source string to SVG; panics on errors.
fn render_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("render must succeed")
}

// ---------------------------------------------------------------------------
// 1. Sequence bare selector — `sequenceDiagram { participant { ... } }`
// ---------------------------------------------------------------------------
/// A `<style>` block with a nested `sequenceDiagram { participant { BackgroundColor ... } }`
/// rule must propagate the colour to participant node fill in the rendered SVG.
#[test]
fn sequence_style_block_participant_background() {
    let src = r#"
@startuml
<style>
sequenceDiagram {
  participant {
    BackgroundColor #bbdefb
  }
}
</style>
participant Alice
participant Bob
Alice -> Bob : hi
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("#bbdefb"),
        "SVG must contain the style-block background colour #bbdefb; got:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

// ---------------------------------------------------------------------------
// 2. Sequence style_block beats skinparam
// ---------------------------------------------------------------------------
/// `skinparam participantBackgroundColor #888888` is tier 3.
/// A `<style>` block setting `BackgroundColor #2563eb` is tier 5 and must win.
#[test]
fn sequence_style_block_beats_skinparam() {
    let src = r#"
@startuml
skinparam participantBackgroundColor #888888
<style>
sequenceDiagram {
  participant {
    BackgroundColor #2563eb
  }
}
</style>
participant Node1
participant Node2
Node1 -> Node2 : message
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("#2563eb"),
        "style block colour #2563eb must override skinparam #888888; svg snippet:\n{}",
        &svg[..svg.len().min(2000)]
    );
    assert!(
        !svg.contains("#888888"),
        "skinparam colour #888888 must NOT appear when overridden by style block; svg snippet:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

// ---------------------------------------------------------------------------
// 3. Activity bare selector — `activityDiagram { activity { ... } }`
// ---------------------------------------------------------------------------
/// A `<style>` block with `activityDiagram { activity { BackgroundColor ... } }`
/// must propagate the colour to activity node fills in the rendered SVG.
#[test]
fn activity_style_block_background() {
    let src = r#"
@startuml
<style>
activityDiagram {
  activity {
    BackgroundColor #d1fae5
  }
}
</style>
:Start;
:Process Data;
:End;
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("#d1fae5"),
        "SVG must contain the style-block background colour #d1fae5; got:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

// ---------------------------------------------------------------------------
// 4. Activity style_block beats skinparam
// ---------------------------------------------------------------------------
/// `skinparam activityBackgroundColor #cccccc` is tier 3.
/// A `<style>` block setting `BackgroundColor #7c3aed` is tier 5 and must win.
#[test]
fn activity_style_block_beats_skinparam() {
    let src = r#"
@startuml
skinparam activityBackgroundColor #cccccc
<style>
activityDiagram {
  activity {
    BackgroundColor #7c3aed
  }
}
</style>
:Step One;
:Step Two;
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("#7c3aed"),
        "style block colour #7c3aed must override skinparam #cccccc; svg snippet:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

// ---------------------------------------------------------------------------
// 5. Activity font color via <style> block
// ---------------------------------------------------------------------------
/// `FontColor` in an `activityDiagram { activity { ... } }` block must appear
/// in activity node text elements.
#[test]
fn activity_style_block_font_color() {
    let src = r#"
@startuml
<style>
activityDiagram {
  activity {
    BackgroundColor #ede9fe
    FontColor #4c1d95
  }
}
</style>
:Task;
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("#ede9fe"),
        "SVG must contain the activity background colour #ede9fe; got:\n{}",
        &svg[..svg.len().min(2000)]
    );
    assert!(
        svg.contains("#4c1d95"),
        "SVG must contain the activity font colour #4c1d95; got:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

// ---------------------------------------------------------------------------
// 6. No-style fallback — default colours without a <style> block
// ---------------------------------------------------------------------------
/// When no `<style>` block is present, sequence participants use the default
/// background colour (#f6f6f6).
#[test]
fn sequence_no_style_uses_default() {
    let src = r#"
@startuml
participant Alice
participant Bob
Alice -> Bob : hi
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("#f6f6f6"),
        "No-style diagram must use default #f6f6f6; got:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

/// When no `<style>` block is present, activity nodes use the default
/// background colour (#ecfdf5).
#[test]
fn activity_no_style_uses_default() {
    let src = r#"
@startuml
:Step;
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("#ecfdf5"),
        "No-style activity diagram must use default #ecfdf5; got:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

// ---------------------------------------------------------------------------
// 7. Phase B regression — class/component families still work
// ---------------------------------------------------------------------------
/// Ensure Phase B class family still works correctly after Phase C changes.
#[test]
fn phase_b_class_regression() {
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
        "Phase B class style block must still work; got:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

/// Ensure Phase B component family still works correctly after Phase C changes.
#[test]
fn phase_b_component_regression() {
    let src = r#"
@startuml
<style>
componentDiagram {
  component {
    BackgroundColor #f0abfc
  }
}
</style>
component "Styled" as c
@enduml
"#;
    let svg = render_svg(src);
    assert!(
        svg.contains("#f0abfc"),
        "Phase B component style block must still work; got:\n{}",
        &svg[..svg.len().min(2000)]
    );
}

// ---------------------------------------------------------------------------
// 8. StyleBuilder unit tests for stereotype + wildcard rules (Phase C)
// ---------------------------------------------------------------------------
/// Verify stereotype-scoped rules (+1000 specificity) beat plain tag rules
/// (+100 specificity) in the StyleBuilder.
#[test]
fn style_builder_stereotype_beats_plain() {
    use puml::ast::style::{
        PName, SName, SelectorChain, SelectorSegment, StyleRule, StyleScheme, StyleValue,
    };
    use puml::theme::style_builder::{StyleBuilder, StyleQuery};
    use std::collections::BTreeMap;

    let mut builder = StyleBuilder::new();
    // Plain participant rule: low specificity (+100)
    builder.push(StyleRule {
        selector_path: vec![SelectorChain {
            segments: vec![SelectorSegment::Tag(SName::Participant)],
        }],
        properties: {
            let mut m = BTreeMap::new();
            m.insert(
                PName::BackgroundColor,
                StyleValue::Color("#ff0000".to_string()),
            );
            m
        },
        unknown_properties: BTreeMap::new(),
        source_order: 1,
        scheme: StyleScheme::Regular,
    });
    // Stereotyped rule: high specificity (+100 + +1000)
    builder.push(StyleRule {
        selector_path: vec![
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::Participant)],
            },
            SelectorChain {
                segments: vec![SelectorSegment::Stereotype("apache".to_string())],
            },
        ],
        properties: {
            let mut m = BTreeMap::new();
            m.insert(
                PName::BackgroundColor,
                StyleValue::Color("#0000ff".to_string()),
            );
            m
        },
        unknown_properties: BTreeMap::new(),
        source_order: 2,
        scheme: StyleScheme::Regular,
    });

    // Query without stereotype → plain rule wins
    let q_plain = StyleQuery::tags([SName::SequenceDiagram, SName::Participant]);
    let plain = builder.lookup(&q_plain);
    assert_eq!(
        plain.color(PName::BackgroundColor),
        Some("#ff0000"),
        "plain participant query must get #ff0000"
    );

    // Query with stereotype → stereotype rule wins
    let q_stereo =
        StyleQuery::tags([SName::SequenceDiagram, SName::Participant]).with_stereotype("apache");
    let stereo = builder.lookup(&q_stereo);
    assert_eq!(
        stereo.color(PName::BackgroundColor),
        Some("#0000ff"),
        "stereotyped query must get #0000ff"
    );
}

/// Verify wildcard rule (+1) loses to a plain tag rule (+100).
#[test]
fn style_builder_wildcard_loses_to_plain() {
    use puml::ast::style::{
        PName, SName, SelectorChain, SelectorSegment, StyleRule, StyleScheme, StyleValue,
    };
    use puml::theme::style_builder::{StyleBuilder, StyleQuery};
    use std::collections::BTreeMap;

    let mut builder = StyleBuilder::new();
    // Wildcard rule: very low specificity (+1)
    builder.push(StyleRule {
        selector_path: vec![SelectorChain {
            segments: vec![SelectorSegment::Wildcard],
        }],
        properties: {
            let mut m = BTreeMap::new();
            m.insert(
                PName::BackgroundColor,
                StyleValue::Color("#aaaaaa".to_string()),
            );
            m
        },
        unknown_properties: BTreeMap::new(),
        source_order: 1,
        scheme: StyleScheme::Regular,
    });
    // Plain tag rule: higher specificity (+100)
    builder.push(StyleRule {
        selector_path: vec![SelectorChain {
            segments: vec![SelectorSegment::Tag(SName::Participant)],
        }],
        properties: {
            let mut m = BTreeMap::new();
            m.insert(
                PName::BackgroundColor,
                StyleValue::Color("#bbbbbb".to_string()),
            );
            m
        },
        unknown_properties: BTreeMap::new(),
        source_order: 2,
        scheme: StyleScheme::Regular,
    });

    let q = StyleQuery::tags([SName::SequenceDiagram, SName::Participant]);
    let result = builder.lookup(&q);
    assert_eq!(
        result.color(PName::BackgroundColor),
        Some("#bbbbbb"),
        "plain tag rule must beat wildcard rule"
    );
}
