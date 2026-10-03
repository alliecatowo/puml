//! Regression tests for issue #1417 — deterministic diagnostic emission for
//! unrecognised `<style>` block selectors, properties, and malformed values.
//!
//! Acceptance criteria (Phase E of #1404):
//! - `W_STYLE_UNKNOWN_TAG` is emitted for unrecognised selector names.
//! - `W_STYLE_UNKNOWN_PROPERTY` is emitted for unrecognised property names.
//! - `E_STYLE_BAD_VALUE` is emitted for malformed colour values on colour properties.
//! - Valid style blocks produce no spurious warnings.
//!
//! All fixtures use the class family (classDiagram top-level selector) because
//! the StyleBuilder is fully wired for class, and `<style>` blocks with a
//! recognised top-level selector produce no DeferredRaw errors.

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

/// Parse + normalize a class-family source and return all warning messages.
fn family_warnings(src: &str) -> Vec<String> {
    let doc = puml::parse(src).expect("source must parse without error");
    let model = puml::normalize_family(doc).expect("source must normalize without error");
    let puml::NormalizedDocument::Family(family) = model else {
        panic!("expected NormalizedDocument::Family");
    };
    family.warnings.iter().map(|w| w.message.clone()).collect()
}

// ---------------------------------------------------------------------------
// W_STYLE_UNKNOWN_TAG
// ---------------------------------------------------------------------------

/// An unrecognised nested selector (e.g. a typo like `clas` inside `classDiagram {}`)
/// must trigger `W_STYLE_UNKNOWN_TAG` because it maps to `SelectorSegment::Unknown`.
#[test]
fn unknown_selector_tag_emits_warning() {
    // `clas` is a typo for `class` — not in the SName catalogue → Unknown segment.
    let src = "@startuml\n<style>\nclassDiagram {\n  clas {\n    BackgroundColor #FF0000\n  }\n}\n</style>\nclass Foo {}\n@enduml\n";
    let warnings = family_warnings(src);
    assert!(
        warnings.iter().any(|w| w.contains("W_STYLE_UNKNOWN_TAG")),
        "typo selector must emit W_STYLE_UNKNOWN_TAG; got: {warnings:?}"
    );
}

/// A recognised nested selector (e.g. `class` inside `classDiagram {}`)
/// must NOT trigger `W_STYLE_UNKNOWN_TAG`.
#[test]
fn known_selector_tag_no_warning() {
    let src = "@startuml\n<style>\nclassDiagram {\n  class {\n    BackgroundColor #FF0000\n  }\n}\n</style>\nclass Foo {}\n@enduml\n";
    let warnings = family_warnings(src);
    assert!(
        warnings.iter().all(|w| !w.contains("W_STYLE_UNKNOWN_TAG")),
        "known selector must not emit W_STYLE_UNKNOWN_TAG; got: {warnings:?}"
    );
}

// ---------------------------------------------------------------------------
// W_STYLE_UNKNOWN_PROPERTY
// ---------------------------------------------------------------------------

/// An unrecognised property name inside a `classDiagram` style block must
/// trigger `W_STYLE_UNKNOWN_PROPERTY`.
///
/// Note: `BackgroudColor` (missing 'n') IS a recognised alias in `PName::from_name`,
/// so we use a completely fabricated property name instead.
#[test]
fn unknown_property_emits_warning() {
    let src = "@startuml\n<style>\nclassDiagram {\n  class {\n    MyFakeProperty #00FF00\n  }\n}\n</style>\nclass Foo {}\n@enduml\n";
    let warnings = family_warnings(src);
    assert!(
        warnings
            .iter()
            .any(|w| w.contains("W_STYLE_UNKNOWN_PROPERTY")),
        "unknown property must emit W_STYLE_UNKNOWN_PROPERTY; got: {warnings:?}"
    );
}

/// A recognised property name must NOT trigger `W_STYLE_UNKNOWN_PROPERTY`.
#[test]
fn known_property_no_unknown_warning() {
    let src = "@startuml\n<style>\nclassDiagram {\n  class {\n    BackgroundColor #00FF00\n  }\n}\n</style>\nclass Foo {}\n@enduml\n";
    let warnings = family_warnings(src);
    assert!(
        warnings
            .iter()
            .all(|w| !w.contains("W_STYLE_UNKNOWN_PROPERTY")),
        "known property must not emit W_STYLE_UNKNOWN_PROPERTY; got: {warnings:?}"
    );
}

// ---------------------------------------------------------------------------
// E_STYLE_BAD_VALUE
// ---------------------------------------------------------------------------

/// A malformed hex colour on a colour property (wrong digit count) must trigger
/// `E_STYLE_BAD_VALUE`.  Uses a `classDiagram { class { … } }` wrapper so
/// `has_known_style_target` recognises the block and does not emit DeferredRaw.
#[test]
fn bad_color_value_emits_error() {
    // `#12345` — starts with `#` but 5 hex digits (not 3/4/6/8) → invalid colour.
    let src = "@startuml\n<style>\nclassDiagram {\n  class {\n    BackgroundColor #12345\n  }\n}\n</style>\nclass Foo {}\n@enduml\n";
    let warnings = family_warnings(src);
    assert!(
        warnings.iter().any(|w| w.contains("E_STYLE_BAD_VALUE")),
        "malformed colour must emit E_STYLE_BAD_VALUE; got: {warnings:?}"
    );
}

/// A well-formed hex colour on a colour property must NOT trigger `E_STYLE_BAD_VALUE`.
#[test]
fn good_color_value_no_error() {
    let src = "@startuml\n<style>\nclassDiagram {\n  class {\n    BackgroundColor #FF0000\n  }\n}\n</style>\nclass Foo {}\n@enduml\n";
    let warnings = family_warnings(src);
    assert!(
        warnings.iter().all(|w| !w.contains("E_STYLE_BAD_VALUE")),
        "valid colour must not emit E_STYLE_BAD_VALUE; got: {warnings:?}"
    );
}

/// A named colour value (keyword) on a colour property must NOT trigger `E_STYLE_BAD_VALUE`.
#[test]
fn named_color_value_no_error() {
    let src = "@startuml\n<style>\nclassDiagram {\n  class {\n    BackgroundColor red\n  }\n}\n</style>\nclass Foo {}\n@enduml\n";
    let warnings = family_warnings(src);
    assert!(
        warnings.iter().all(|w| !w.contains("E_STYLE_BAD_VALUE")),
        "named colour must not emit E_STYLE_BAD_VALUE; got: {warnings:?}"
    );
}
