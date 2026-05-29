//! Canonical text-width estimation for the renderer layer.
//!
//! All diagram families must use these helpers instead of inline
//! `chars().count() * 7` literals.  Having a single source of truth means
//! that font or heuristic changes require exactly one code edit, and the
//! layout engine and the validator always agree on estimated glyph widths.
//!
//! # Heuristic
//!
//! The underlying model is a monospace font at a 14 px baseline:
//!
//!   - 1 glyph ≈ 7 px at 14 px font size
//!   - Width scales linearly with font size: `chars × 7 × (font_px / 14)`
//!   - Bold text is approximately 1.1× wider than regular
//!   - Multi-byte characters are counted as one glyph (Unicode codepoint
//!     counting via `.chars().count()`, not byte-length)
//!
//! These constants match the `DEFAULT_MONOSPACE_CHAR_WIDTH = 7` in
//! `crate::render::text_metrics` and the `CHAR_WIDTH_PX = 7` in the
//! validate layer, keeping all three in sync.

/// Baseline font size for the `7 px/char` heuristic (in pixels).
pub const BASE_FONT_SIZE_PX: u32 = 14;

/// Character width at the baseline font size (in pixels).
pub const BASE_CHAR_WIDTH_PX: u32 = 7;

/// Estimated pixel width of `text` rendered at `font_size_px` in a
/// monospace font.
///
/// Uses `.chars().count()` for Unicode-correct glyph counting.
///
/// # Panics
///
/// Never panics.
#[inline]
pub fn estimate_text_width(text: &str, font_size_px: u32) -> u32 {
    let chars = text.chars().count() as u32;
    chars * BASE_CHAR_WIDTH_PX * font_size_px / BASE_FONT_SIZE_PX
}

/// Like [`estimate_text_width`] but returns `f64` for renderers that work
/// in floating-point geometry.
#[inline]
pub fn estimate_text_width_f64(text: &str, font_size_px: f64) -> f64 {
    text.chars().count() as f64 * BASE_CHAR_WIDTH_PX as f64 * font_size_px
        / BASE_FONT_SIZE_PX as f64
}

/// Estimated pixel width at the default (14 px) font size, returned as `i32`
/// for renderers that use integer geometry.
///
/// Equivalent to `text.chars().count() as i32 * 7`.
#[inline]
pub fn estimate_text_width_default(text: &str) -> i32 {
    text.chars().count() as i32 * BASE_CHAR_WIDTH_PX as i32
}

/// Bold text is ~1.1× wider than regular.  Returns `f64`.
#[inline]
pub fn estimate_bold_text_width_f64(text: &str, font_size_px: f64) -> f64 {
    estimate_text_width_f64(text, font_size_px) * 1.1
}
