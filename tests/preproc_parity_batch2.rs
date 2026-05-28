/// Preprocessor parity batch 2 — `%mod`, `%is_light`, `%hsl_color`,
/// `%reverse_hsluv_color`, `%version`.
///
/// Each test exercises a newly-implemented PlantUML preprocessor builtin
/// through the public `parse` API, verifying the expanded result via
/// AST message labels.
use puml::{ast::StatementKind, parse};

// ── helper ────────────────────────────────────────────────────────────────────

fn msg_labels(src: &str) -> Vec<String> {
    let doc = parse(src).expect("parse failed");
    doc.statements
        .iter()
        .filter_map(|s| match &s.kind {
            StatementKind::Message(m) => m.label.clone(),
            _ => None,
        })
        .collect()
}

fn preprocess_ok(src: &str) {
    parse(src).expect("parse should succeed");
}

// ── %mod ──────────────────────────────────────────────────────────────────────

/// Basic modulo: 10 mod 3 = 1.
#[test]
fn mod_basic_positive_remainder() {
    let src = "@startuml
!$r = %mod(10, 3)
A -> B : $r
@enduml";
    assert_eq!(msg_labels(src), vec!["1"]);
}

/// Modulo with exact divisibility returns 0.
#[test]
fn mod_exact_divisor_returns_zero() {
    let src = "@startuml
!$r = %mod(9, 3)
A -> B : $r
@enduml";
    assert_eq!(msg_labels(src), vec!["0"]);
}

/// %mod is useful in loops for alternating patterns (even/odd index).
#[test]
fn mod_alternating_pattern_in_loop() {
    // Build a string counting how many loop iterations have an even index
    // by checking %mod(i, 2) == 0.  Indexes 0, 2, 4 → 3 even iterations.
    let src = "@startuml
!$i = 0
!$evens = 0
!while $i < 5
!if %mod($i, 2) == 0
!$evens = %eval($evens + 1)
!endif
!$i = %eval($i + 1)
!endwhile
A -> B : $evens
@enduml";
    assert_eq!(msg_labels(src), vec!["3"]);
}

/// Division by zero returns 0 (no panic).
#[test]
fn mod_division_by_zero_returns_zero() {
    let src = "@startuml
!$r = %mod(7, 0)
A -> B : $r
@enduml";
    assert_eq!(msg_labels(src), vec!["0"]);
}

/// Negative dividend with positive divisor — Euclidean semantics mean the
/// result is non-negative: (-1) rem_euclid(3) == 2.
#[test]
fn mod_negative_dividend_euclidean() {
    let src = "@startuml
!$r = %mod(-1, 3)
A -> B : $r
@enduml";
    assert_eq!(msg_labels(src), vec!["2"]);
}

// ── %is_light ─────────────────────────────────────────────────────────────────

/// White (#ffffff) is a light colour.
#[test]
fn is_light_white_returns_true() {
    let src = "@startuml
!$r = %is_light(\"#ffffff\")
A -> B : $r
@enduml";
    assert_eq!(msg_labels(src), vec!["true"]);
}

/// Black (#000000) is not a light colour.
#[test]
fn is_light_black_returns_false() {
    let src = "@startuml
!$r = %is_light(\"#000000\")
A -> B : $r
@enduml";
    assert_eq!(msg_labels(src), vec!["false"]);
}

/// %is_light and %is_dark are complementary for the same colour.
#[test]
fn is_light_is_complement_of_is_dark() {
    let src = "@startuml
!$color = \"#3399ff\"
!$dark = %is_dark($color)
!$light = %is_light($color)
!if $dark == \"true\"
!assert $light == \"false\" : is_light should be false when is_dark is true
!else
!assert $light == \"true\" : is_light should be true when is_dark is false
!endif
A -> B : ok
@enduml";
    assert_eq!(msg_labels(src), vec!["ok"]);
}

/// Mid-grey (#808080) — luminance is exactly 128 → is_light (≥ 128).
#[test]
fn is_light_mid_grey_boundary() {
    let src = "@startuml
!$r = %is_light(\"#808080\")
A -> B : $r
@enduml";
    // luminance = (128*299 + 128*587 + 128*114) / 1000 = 128 → is_light
    assert_eq!(msg_labels(src), vec!["true"]);
}

// ── %hsl_color ────────────────────────────────────────────────────────────────

/// Pure red in HSL is H=0, S=100, L=50 → #ff0000.
#[test]
fn hsl_color_pure_red() {
    let src = "@startuml
!$c = %hsl_color(0, 100, 50)
A -> B : $c
@enduml";
    assert_eq!(msg_labels(src), vec!["#ff0000"]);
}

/// Pure green: H=120, S=100, L=50 → #00ff00.
#[test]
fn hsl_color_pure_green() {
    let src = "@startuml
!$c = %hsl_color(120, 100, 50)
A -> B : $c
@enduml";
    assert_eq!(msg_labels(src), vec!["#00ff00"]);
}

/// Pure blue: H=240, S=100, L=50 → #0000ff.
#[test]
fn hsl_color_pure_blue() {
    let src = "@startuml
!$c = %hsl_color(240, 100, 50)
A -> B : $c
@enduml";
    assert_eq!(msg_labels(src), vec!["#0000ff"]);
}

/// Black: any hue, S=0, L=0 → #000000.
#[test]
fn hsl_color_black() {
    let src = "@startuml
!$c = %hsl_color(0, 0, 0)
A -> B : $c
@enduml";
    assert_eq!(msg_labels(src), vec!["#000000"]);
}

/// White: any hue, S=0, L=100 → #ffffff.
#[test]
fn hsl_color_white() {
    let src = "@startuml
!$c = %hsl_color(0, 0, 100)
A -> B : $c
@enduml";
    assert_eq!(msg_labels(src), vec!["#ffffff"]);
}

/// Four-argument form with alpha=128 produces an #rrggbbaa result.
#[test]
fn hsl_color_with_alpha_produces_8_digit_hex() {
    let src = "@startuml
!$c = %hsl_color(0, 100, 50, 128)
A -> B : $c
@enduml";
    let labels = msg_labels(src);
    // Should start with #ff0000 and end with the alpha byte for 128 (0x80)
    assert_eq!(labels.len(), 1);
    let c = &labels[0];
    assert!(c.starts_with("#ff0000"), "expected red prefix, got: {c}");
    assert_eq!(c.len(), 9, "expected 9-char hex (# + 8 hex digits): {c}");
}

/// Alpha=255 (fully opaque) produces the 3-byte form without alpha suffix.
#[test]
fn hsl_color_fully_opaque_alpha_omits_alpha_byte() {
    let src = "@startuml
!$c = %hsl_color(0, 100, 50, 255)
A -> B : $c
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["#ff0000"]);
}

/// %hsl_color result is a valid hex colour that %is_dark can consume.
#[test]
fn hsl_color_result_usable_by_is_dark() {
    let src = "@startuml
!$dark_blue = %hsl_color(240, 100, 20)
!$r = %is_dark($dark_blue)
A -> B : $r
@enduml";
    // H=240 S=100 L=20 → dark blue → is_dark should be true
    assert_eq!(msg_labels(src), vec!["true"]);
}

// ── %reverse_hsluv_color ──────────────────────────────────────────────────────

/// Reversing white (#ffffff) should give a dark colour.
#[test]
fn reverse_hsluv_color_white_gives_dark() {
    let src = "@startuml
!$r = %reverse_hsluv_color(\"#ffffff\")
!$dark = %is_dark($r)
A -> B : $dark
@enduml";
    assert_eq!(msg_labels(src), vec!["true"]);
}

/// Reversing black (#000000) should give a light colour.
#[test]
fn reverse_hsluv_color_black_gives_light() {
    let src = "@startuml
!$r = %reverse_hsluv_color(\"#000000\")
!$light = %is_light($r)
A -> B : $light
@enduml";
    assert_eq!(msg_labels(src), vec!["true"]);
}

/// Reversing an invalid colour returns an empty string (no panic).
#[test]
fn reverse_hsluv_color_invalid_input_no_panic() {
    let src = "@startuml
!$r = %reverse_hsluv_color(\"notacolor\")
A -> B : empty
@enduml";
    preprocess_ok(src);
}

/// Reversing a mid-hue saturated colour produces a valid hex string.
#[test]
fn reverse_hsluv_color_produces_hex_string() {
    let src = "@startuml
!$r = %reverse_hsluv_color(\"#3366cc\")
A -> B : $r
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels.len(), 1);
    let c = &labels[0];
    assert!(
        c.starts_with('#') && c.len() == 7,
        "expected 7-char hex colour, got: {c}"
    );
}

// ── %version ──────────────────────────────────────────────────────────────────

/// %version() returns a non-empty version string.
#[test]
fn version_returns_non_empty_string() {
    let src = "@startuml
!$v = %version()
A -> B : $v
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels.len(), 1);
    assert!(!labels[0].is_empty(), "expected non-empty version string");
}

/// %version() is deterministic: calling it twice gives the same value.
#[test]
fn version_is_deterministic() {
    let src = "@startuml
!$v1 = %version()
!$v2 = %version()
!assert $v1 == $v2 : version must be deterministic
A -> B : ok
@enduml";
    assert_eq!(msg_labels(src), vec!["ok"]);
}
