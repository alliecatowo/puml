//! Regression tests for #1401 — inline sprite definition.
//!
//! PlantUML LRG §23.2 allows defining a monochrome sprite inline using either:
//!   `sprite $name [WxH/N] { ... }`  — explicit depth N (4, 8, or 16 gray levels)
//!   `sprite $name [WxH] { ... }`    — no depth: defaults to 16 gray levels
//!
//! The second form (no depth suffix) was not previously handled. This suite
//! tests both forms and verifies that a class with a sprite stereotype
//! `<<$myicon>>` renders the actual sprite pixel icon in its header.

// ── unit: parse_sprite_header_spec ───────────────────────────────────────────

/// `[WxH]` form (no depth) → (W, H, 16, false)
#[test]
fn parse_sprite_header_spec_no_depth_monochrome() {
    let result = puml::sprites::parse_sprite_header_spec("[16x16]");
    assert_eq!(
        result,
        Some((16, 16, 16, false)),
        "#1401: [WxH] form should default to gray_levels=16, compressed=false"
    );
}

/// `[WxH/N]` form (explicit depth) → (W, H, N, false)
#[test]
fn parse_sprite_header_spec_with_depth() {
    let result = puml::sprites::parse_sprite_header_spec("[16x16/16]");
    assert_eq!(
        result,
        Some((16, 16, 16, false)),
        "#1401: [WxH/N] form should parse correctly"
    );
}

/// `[WxH/Nz]` form (compressed) → (W, H, N, true)
#[test]
fn parse_sprite_header_spec_compressed() {
    let result = puml::sprites::parse_sprite_header_spec("[16x16/16z]");
    assert_eq!(
        result,
        Some((16, 16, 16, true)),
        "#1401: [WxH/Nz] form should set compressed=true"
    );
}

/// Various dimension sizes without depth suffix are accepted
#[test]
fn parse_sprite_header_spec_various_sizes() {
    assert_eq!(
        puml::sprites::parse_sprite_header_spec("[4x4]"),
        Some((4, 4, 16, false))
    );
    assert_eq!(
        puml::sprites::parse_sprite_header_spec("[8x8]"),
        Some((8, 8, 16, false))
    );
    assert_eq!(
        puml::sprites::parse_sprite_header_spec("[32x24]"),
        Some((32, 24, 16, false))
    );
}

/// Zero dimensions are rejected
#[test]
fn parse_sprite_header_spec_zero_rejected() {
    assert_eq!(puml::sprites::parse_sprite_header_spec("[0x8]"), None);
    assert_eq!(puml::sprites::parse_sprite_header_spec("[8x0]"), None);
    assert_eq!(puml::sprites::parse_sprite_header_spec("[0x0]"), None);
}

// ── render: inline sprite in class stereotype ─────────────────────────────────

fn svg_for(src: &str) -> String {
    puml::render_source_to_svg(src).expect("svg should render")
}

/// A class with `<<$myicon>>` stereotype should produce a `data-sprite="myicon"`
/// element in the SVG output — confirming the inline-sprite pixel icon is drawn.
#[test]
fn class_sprite_stereotype_renders_sprite_element() {
    // 4x4 monochrome sprite — a simple diamond pattern using hex digits 0–F.
    // Row values: each row is 4 hex chars long (matching the 4-wide spec).
    let src = r#"
@startuml
sprite $myicon [4x4] {
0FF0
F00F
F00F
0FF0
}
class MyService <<$myicon>>
@enduml
"#;
    let svg = svg_for(src);

    // The sprite group element must appear with the correct data-sprite attribute.
    assert!(
        svg.contains("data-sprite=\"myicon\""),
        "#1401: data-sprite=\"myicon\" not found — sprite icon not rendered in class header"
    );
}

/// The `data-creole-sprites` wrapper group must be present when a sprite stereotype
/// is used — confirming the creole_text path was taken, not the plain-text path.
#[test]
fn class_sprite_stereotype_uses_creole_path() {
    let src = r#"
@startuml
sprite $arrow [4x4] {
0F00
FF00
0F00
0000
}
class Router <<$arrow>>
@enduml
"#;
    let svg = svg_for(src);

    assert!(
        svg.contains("data-creole-sprites=\"true\""),
        "#1401: data-creole-sprites wrapper missing — sprite stereotype not routed through creole_text"
    );
}

/// A class with a plain stereotype (no `$` prefix) must NOT trigger the sprite
/// rendering path — guillemet text is expected instead.
#[test]
fn plain_stereotype_not_treated_as_sprite() {
    let src = r#"
@startuml
class Service <<service>>
@enduml
"#;
    let svg = svg_for(src);

    // Should contain guillemet text, not a sprite group.
    assert!(
        svg.contains("\u{ab}service\u{bb}")
            || svg.contains("&laquo;service&raquo;")
            || svg.contains("«service»"),
        "#1401: plain stereotype should render as guillemet text"
    );
    assert!(
        !svg.contains("data-creole-sprites=\"true\""),
        "#1401: plain stereotype should not trigger creole sprite path"
    );
}

/// A class diagram where inline sprite is defined with the explicit-depth `[WxH/N]`
/// form also renders the sprite icon correctly.
#[test]
fn class_sprite_stereotype_explicit_depth_renders() {
    let src = r#"
@startuml
sprite $box [4x4/16] {
FFFF
F00F
F00F
FFFF
}
class Box <<$box>>
@enduml
"#;
    let svg = svg_for(src);

    assert!(
        svg.contains("data-sprite=\"box\""),
        "#1401: explicit-depth [WxH/N] sprite not rendered; expected data-sprite=\"box\""
    );
}
