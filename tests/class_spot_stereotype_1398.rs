//! Regression tests for #1398 — class spot stereotype <<(L,#color) Label>> badge support.
//!
//! PlantUML supports `class Foo <<(S,#FF7700) Service>>` to render a coloured circle
//! with a single letter inside the class header, used pervasively by stdlib themes
//! (AWS/Azure/GCP) as visual type indicators.
//!
//! Tests cover:
//!   - spot letter only: `<<(S,#FF7700)>>`
//!   - spot letter + color + label: `<<(C,#0066CC) Controller>>`
//!   - spot with long color name: `<<(R,#009900) Repository>>`
//!   - spot badge overrides kind-default C badge
//!   - spot label is displayed as «Label» in header
//!   - non-spot user stereotypes are unaffected
//!   - spot member text `<<spot:…>>` does NOT appear as raw text

fn svg_for(src: &str) -> String {
    puml::render_source_to_svg(src).expect("svg should render")
}

// ── Parser round-trip: spot member encoding ───────────────────────────────────

/// A spot stereotype `<<(S,#FF7700) Service>>` must be encoded internally as
/// `<<spot:S:#FF7700:Service>>` and that raw encoding must NEVER appear in the SVG.
#[test]
fn spot_encoding_not_visible_in_svg() {
    let svg = svg_for(
        r#"@startuml
class MyService <<(S,#FF7700) Service>>
@enduml"#,
    );
    assert!(
        !svg.contains("spot:S"),
        "raw spot encoding must not appear in SVG; SVG:\n{svg}"
    );
    assert!(
        !svg.contains("(S,#FF7700)"),
        "raw spot syntax must not appear in SVG; SVG:\n{svg}"
    );
}

// ── Rendered badge: circle + letter ──────────────────────────────────────────

/// A spot stereotype must emit a `<circle>` with the spot colour as fill.
#[test]
fn spot_badge_circle_emitted() {
    let svg = svg_for(
        r#"@startuml
class MyService <<(S,#FF7700) Service>>
@enduml"#,
    );
    // Spot badge circle must use the spot color.
    assert!(
        svg.contains("uml-spot-badge"),
        "SVG must contain a spot badge circle; SVG:\n{svg}"
    );
    assert!(
        svg.contains("#FF7700"),
        "SVG must contain the spot colour #FF7700; SVG:\n{svg}"
    );
}

/// The spot badge must display a white letter inside the circle.
#[test]
fn spot_badge_letter_is_white() {
    let svg = svg_for(
        r#"@startuml
class MyService <<(S,#FF7700) Service>>
@enduml"#,
    );
    // White letter text on a coloured circle.
    assert!(
        svg.contains("uml-spot-badge-letter"),
        "SVG must contain a spot badge letter element; SVG:\n{svg}"
    );
    assert!(
        svg.contains("fill=\"#ffffff\""),
        "spot badge letter must be white; SVG:\n{svg}"
    );
}

// ── Spot label text in header ─────────────────────────────────────────────────

/// A spot with a label `<<(S,#FF7700) Service>>` must show `«Service»` in the header.
#[test]
fn spot_label_shown_in_header() {
    let svg = svg_for(
        r#"@startuml
class MyService <<(S,#FF7700) Service>>
@enduml"#,
    );
    // «Service» guillemet label in the header.
    assert!(
        svg.contains("\u{ab}Service\u{bb}"),
        "spot label must appear as guillemet text in header; SVG:\n{svg}"
    );
}

/// A spot without a label `<<(R,#009900)>>` must NOT emit an empty guillemet row.
#[test]
fn spot_without_label_no_empty_guillemet() {
    let svg = svg_for(
        r#"@startuml
class MyRepo <<(R,#009900)>>
@enduml"#,
    );
    // No «» empty guillemet row.
    assert!(
        !svg.contains("\u{ab}\u{bb}"),
        "spot without label must not emit empty guillemet text; SVG:\n{svg}"
    );
    // The circle must still be present with the spot colour.
    assert!(
        svg.contains("#009900"),
        "spot color #009900 must appear in SVG; SVG:\n{svg}"
    );
}

// ── Spot overrides kind-default badge ────────────────────────────────────────

/// A plain `class Foo` gets a green C badge.  A spot-stereotyped `class Foo <<(S,#FF7700)>>`
/// must replace that green C with the spot letter and colour (no green C).
#[test]
fn spot_overrides_kind_default_badge() {
    // Plain class gets the default C badge fill colour (#A2D5A2).
    let plain_svg = svg_for(
        r#"@startuml
class PlainClass
@enduml"#,
    );
    assert!(
        plain_svg.contains("#A2D5A2"),
        "plain class must have green-ish C badge; SVG:\n{plain_svg}"
    );

    // Spot class must NOT contain the default C badge colour.
    let spot_svg = svg_for(
        r#"@startuml
class SpotClass <<(S,#FF7700) Service>>
@enduml"#,
    );
    assert!(
        !spot_svg.contains("#A2D5A2"),
        "spot class must not contain default C badge fill; SVG:\n{spot_svg}"
    );
    assert!(
        spot_svg.contains("#FF7700"),
        "spot class must use spot colour #FF7700; SVG:\n{spot_svg}"
    );
}

// ── Non-spot stereotypes unaffected ──────────────────────────────────────────

/// A plain user stereotype `<<Service>>` (no spot syntax) must still render
/// as a guillemet label and must NOT be confused with a spot stereotype.
#[test]
fn plain_user_stereotype_unaffected() {
    let svg = svg_for(
        r#"@startuml
class PlainService <<Service>>
@enduml"#,
    );
    // Plain user stereotype shows as «Service».
    assert!(
        svg.contains("\u{ab}Service\u{bb}"),
        "plain <<Service>> must appear as guillemet; SVG:\n{svg}"
    );
    // It must not produce a spot badge (no uml-spot-badge class).
    assert!(
        !svg.contains("uml-spot-badge"),
        "plain <<Service>> must not produce a spot badge; SVG:\n{svg}"
    );
}

// ── Multiple spot stereotypes / combined with other stereotypes ───────────────

/// Multiple stereotypes including a spot: `<<(C,#0066CC) Controller>> <<extra>>`.
/// The spot badge must render and `«extra»` label must appear.
#[test]
fn spot_combined_with_extra_stereotype() {
    let svg = svg_for(
        r#"@startuml
class Ctrl <<(C,#0066CC) Controller>> <<api>>
@enduml"#,
    );
    assert!(
        svg.contains("uml-spot-badge"),
        "combined spot+extra must emit spot badge; SVG:\n{svg}"
    );
    assert!(
        svg.contains("#0066CC"),
        "spot colour #0066CC must be present; SVG:\n{svg}"
    );
    assert!(
        svg.contains("\u{ab}Controller\u{bb}"),
        "spot label «Controller» must be present; SVG:\n{svg}"
    );
    assert!(
        svg.contains("\u{ab}api\u{bb}"),
        "extra stereotype «api» must also be present; SVG:\n{svg}"
    );
}
