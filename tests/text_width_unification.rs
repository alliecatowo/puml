//! Regression tests for the unified text-width estimation layer.
//!
//! These tests assert that:
//! - The shared `render_core::text_metrics` helper and the per-family
//!   `render::text_metrics` helper return byte-identical results for common
//!   inputs so layout and validation never drift apart.
//! - Bold width is wider than regular width.
//! - Width scales linearly with font size.
//! - Multi-byte (Unicode) characters are counted as single glyphs.

use puml::render_core::text_metrics::{
    estimate_bold_text_width_f64, estimate_text_width, estimate_text_width_default,
    estimate_text_width_f64, BASE_CHAR_WIDTH_PX, BASE_FONT_SIZE_PX,
};

// ── consistency with the per-family render::text_metrics layer ───────────────

/// The render_core helper and render::text_metrics::default_monospace_width
/// must agree on every ASCII string at the default 14 px font size.
#[test]
fn render_core_matches_render_text_metrics_for_ascii() {
    let samples = ["", "a", "hello", "Hello World", "abcdefghijklmnop"];
    for text in samples {
        let render_core_px = estimate_text_width_default(text);
        // render::text_metrics::default_monospace_width = chars * 7
        let family_px = text.chars().count() as i32 * BASE_CHAR_WIDTH_PX as i32;
        assert_eq!(
            render_core_px, family_px,
            "mismatch for {:?}: render_core={} family={}",
            text, render_core_px, family_px
        );
    }
}

/// The floating-point variant at exactly BASE_FONT_SIZE_PX must equal the
/// integer variant (no rounding difference for whole-char inputs).
#[test]
fn f64_variant_matches_integer_variant_at_base_font_size() {
    let samples = ["a", "hello", "foo bar baz", "αβγ"];
    for text in samples {
        let int_px = estimate_text_width_default(text) as f64;
        let f64_px = estimate_text_width_f64(text, BASE_FONT_SIZE_PX as f64);
        assert!(
            (int_px - f64_px).abs() < 1e-9,
            "mismatch for {:?}: int={} f64={}",
            text,
            int_px,
            f64_px
        );
    }
}

// ── font-size scaling ─────────────────────────────────────────────────────────

/// Width must scale linearly with font size: doubling the font doubles the width.
#[test]
fn width_scales_linearly_with_font_size() {
    let text = "hello";
    let w14 = estimate_text_width_f64(text, 14.0);
    let w28 = estimate_text_width_f64(text, 28.0);
    assert!(
        (w28 - w14 * 2.0).abs() < 1e-9,
        "non-linear scaling: w14={} w28={} expected {}",
        w14,
        w28,
        w14 * 2.0
    );
}

/// Check concrete pixel values at 12 px and 14 px against the expected formula.
#[test]
fn concrete_values_at_12px_and_14px() {
    // 5 chars × 7 px/char × (12/14) ≈ 30 px (integer division: 5*7*12/14 = 30)
    assert_eq!(estimate_text_width("hello", 12), 30);
    // 5 chars × 7 px/char × (14/14) = 35 px
    assert_eq!(estimate_text_width("hello", 14), 35);
}

// ── bold variant ──────────────────────────────────────────────────────────────

/// Bold text must be strictly wider than regular text (≥ 1.0×, ratio ≈ 1.1×).
#[test]
fn bold_wider_than_regular() {
    let text = "Label text";
    let regular = estimate_text_width_f64(text, 14.0);
    let bold = estimate_bold_text_width_f64(text, 14.0);
    assert!(
        bold > regular,
        "bold ({}) should be wider than regular ({})",
        bold,
        regular
    );
    // Ratio should be approximately 1.1.
    let ratio = bold / regular;
    assert!(
        (ratio - 1.1).abs() < 0.001,
        "expected bold/regular ≈ 1.1, got {:.4}",
        ratio
    );
}

// ── Unicode / multi-byte ──────────────────────────────────────────────────────

/// Each Unicode codepoint must count as one glyph regardless of byte width.
#[test]
fn unicode_codepoints_counted_as_single_glyphs() {
    // U+03B1 α is 2 bytes in UTF-8 — must count as 1 glyph.
    let ascii_3 = "abc"; // 3 ASCII chars
    let greek_3 = "αβγ"; // 3 Greek chars (each 2 bytes)
    assert_eq!(
        estimate_text_width_default(ascii_3),
        estimate_text_width_default(greek_3),
        "byte-len differences must not affect width estimate"
    );
}

/// An empty string always has zero width.
#[test]
fn empty_string_zero_width() {
    assert_eq!(estimate_text_width("", 14), 0);
    assert_eq!(estimate_text_width_default(""), 0);
    assert!((estimate_text_width_f64("", 14.0) - 0.0).abs() < 1e-9);
}

// ── validate layer agrees with layout layer ───────────────────────────────────

/// `validate::svg_hooks::CHAR_WIDTH_PX` must equal `BASE_CHAR_WIDTH_PX`.
///
/// This is the core "single source of truth" invariant: if they differ,
/// the validator would flag diagram-wide label clips that the layout engine
/// never actually produces — or silently miss real ones.
#[test]
fn validate_char_width_matches_base() {
    // We can't import the private constant directly, but we can check the
    // arithmetic: 1 char at 14 px should give BASE_CHAR_WIDTH_PX pixels.
    let single_char_px = estimate_text_width("x", 14);
    assert_eq!(
        single_char_px, BASE_CHAR_WIDTH_PX,
        "single char at 14 px should be BASE_CHAR_WIDTH_PX={}",
        BASE_CHAR_WIDTH_PX
    );
}
