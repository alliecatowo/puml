//! Visual regression smoke tests.
//!
//! Renders each fixture in `tests/visual_regression/manifest.json` to SVG via
//! the `puml` CLI and asserts (1) no empty `<text>` elements, (2) all
//! `expected_text` substrings appear, (3) at least `min_text_elements`
//! non-empty `<text>` elements are emitted. Fixtures may also opt into
//! generic semantic SVG contracts for classes, data attributes, expected
//! element counts, and geometry profiles.
//!
//! Also provides PNG baseline-diff sweeps and a bless mechanism
//! (`bless_baselines`) for promoting renders to baselines after intentional
//! changes.
//!
//! Catches the family of bugs where the renderer drops text labels.
//! See `tests/visual_regression/README.md`.

#[path = "visual_regression/harness.rs"]
mod harness;
#[path = "visual_regression/manifest.rs"]
mod manifest;
#[path = "visual_regression/png.rs"]
mod png;
#[path = "svg_test_helpers.rs"]
mod svg_test_helpers;
#[path = "visual_regression/text.rs"]
mod text;
