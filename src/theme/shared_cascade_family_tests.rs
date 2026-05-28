//! Unit tests for the per-family shared cascade functions wired in #1184
//! (activity, state, timing, sequence, mindmap).
//!
//! Kept in a separate file to stay within the 600-line file-size guardrail.

use super::families::*;
use super::simple_cascade;
use crate::theme::values::StyleSource as Src;

// ── simple_cascade tests ──────────────────────────────────────────────────────

/// Default-only (Src::Default) — returns the diagram value sourced as Default.
#[test]
fn simple_cascade_default_only() {
    let result = simple_cascade("#ecfdf5", Src::Default, None, None);
    assert_eq!(result.as_str(), "#ecfdf5");
    assert_eq!(result.source(), Src::Default);
}

/// Theme-sourced diagram color wins over Default.
#[test]
fn simple_cascade_theme_beats_default() {
    let result = simple_cascade("#aabbcc", Src::ThemePreset, None, None);
    assert_eq!(result.as_str(), "#aabbcc");
    assert_eq!(result.source(), Src::ThemePreset);
}

/// Skinparam-sourced diagram color wins over Default.
#[test]
fn simple_cascade_skinparam_beats_default() {
    let result = simple_cascade("#112233", Src::SkinParam, None, None);
    assert_eq!(result.as_str(), "#112233");
    assert_eq!(result.source(), Src::SkinParam);
}

/// Stereotype wins over skinparam-sourced value.
#[test]
fn simple_cascade_stereotype_beats_skinparam() {
    let result = simple_cascade("#112233", Src::SkinParam, Some("#334455"), None);
    assert_eq!(result.as_str(), "#334455");
    assert_eq!(result.source(), Src::Stereotype);
}

/// Inline wins over all other tiers.
#[test]
fn simple_cascade_inline_beats_all() {
    let result = simple_cascade("#112233", Src::SkinParam, Some("#334455"), Some("#ff0000"));
    assert_eq!(result.as_str(), "#ff0000");
    assert_eq!(result.source(), Src::Inline);
}

// ── activity_node_effective_style tests ───────────────────────────────────────

/// Default activity style — all fields resolve to their built-in defaults.
#[test]
fn activity_effective_default() {
    use crate::theme::skinparam::ActivityStyle;
    let style = ActivityStyle::default();
    let result = activity_node_effective_style(&style, Src::Default, None, None, None);
    assert_eq!(result.fill.as_str(), "#ecfdf5");
    assert_eq!(result.fill.source(), Src::Default);
    assert_eq!(result.stroke.as_str(), "#047857");
    assert_eq!(result.stroke.source(), Src::Default);
    assert_eq!(result.font_color.as_str(), "#0f172a");
    assert_eq!(result.font_color.source(), Src::Default);
}

/// Theme-sourced values flow through the cascade correctly.
#[test]
fn activity_effective_theme_sourced() {
    use crate::theme::skinparam::ActivityStyle;
    let style = ActivityStyle {
        background_color: "#aabbcc".to_string(),
        ..ActivityStyle::default()
    };
    let result = activity_node_effective_style(&style, Src::ThemePreset, None, None, None);
    assert_eq!(result.fill.as_str(), "#aabbcc");
    assert_eq!(result.fill.source(), Src::ThemePreset);
}

/// Inline fill overrides a skinparam-sourced diagram value.
#[test]
fn activity_effective_inline_beats_skinparam() {
    use crate::theme::skinparam::ActivityStyle;
    let style = ActivityStyle {
        background_color: "#112233".to_string(),
        ..ActivityStyle::default()
    };
    let result = activity_node_effective_style(&style, Src::SkinParam, Some("#ff0000"), None, None);
    assert_eq!(result.fill.as_str(), "#ff0000");
    assert_eq!(result.fill.source(), Src::Inline);
}

// ── state_node_effective_style tests ─────────────────────────────────────────

/// Default state style resolves correctly.
#[test]
fn state_effective_default() {
    use crate::theme::skinparam::StateStyle;
    let style = StateStyle::default();
    let result = state_node_effective_style(&style, Src::Default, None, None, None);
    assert_eq!(result.fill.as_str(), "#f6f6f6");
    assert_eq!(result.fill.source(), Src::Default);
    assert_eq!(result.stroke.as_str(), "#1e293b");
    assert_eq!(result.stroke.source(), Src::Default);
}

/// Inline fill_color (StateNodeStyle) beats skinparam-sourced value.
#[test]
fn state_effective_inline_fill_beats_skinparam() {
    use crate::theme::skinparam::StateStyle;
    let style = StateStyle {
        background_color: "#334455".to_string(),
        ..StateStyle::default()
    };
    let result = state_node_effective_style(&style, Src::SkinParam, Some("#ff0000"), None, None);
    assert_eq!(result.fill.as_str(), "#ff0000");
    assert_eq!(result.fill.source(), Src::Inline);
}

/// Theme-sourced state values propagate through cascade.
#[test]
fn state_effective_theme_sourced() {
    use crate::theme::skinparam::StateStyle;
    let style = StateStyle {
        background_color: "#aabbcc".to_string(),
        border_color: "#112233".to_string(),
        ..StateStyle::default()
    };
    let result = state_node_effective_style(&style, Src::ThemePreset, None, None, None);
    assert_eq!(result.fill.source(), Src::ThemePreset);
    assert_eq!(result.fill.as_str(), "#aabbcc");
    assert_eq!(result.stroke.source(), Src::ThemePreset);
    assert_eq!(result.stroke.as_str(), "#112233");
}

// ── timing_lane_effective_style tests ────────────────────────────────────────

/// Default timing style — signal fill/stroke/font resolve from defaults.
#[test]
fn timing_effective_default() {
    use crate::theme::skinparam::TimingStyle;
    let style = TimingStyle::default();
    let result = timing_lane_effective_style(&style, Src::Default, None, None, None);
    assert_eq!(result.signal_fill.as_str(), "#f8fafc");
    assert_eq!(result.signal_fill.source(), Src::Default);
    assert_eq!(result.signal_stroke.as_str(), "#0f172a");
    assert_eq!(result.signal_stroke.source(), Src::Default);
    assert_eq!(result.font_color.as_str(), "#0f172a");
    assert_eq!(result.font_color.source(), Src::Default);
    assert_eq!(result.arrow_color.as_str(), "#0ea5e9");
    assert_eq!(result.arrow_color.source(), Src::Default);
}

/// Skinparam-sourced timing colors flow through.
#[test]
fn timing_effective_skinparam_sourced() {
    use crate::theme::skinparam::TimingStyle;
    let style = TimingStyle {
        signal_background_color: "#334455".to_string(),
        ..TimingStyle::default()
    };
    let result = timing_lane_effective_style(&style, Src::SkinParam, None, None, None);
    assert_eq!(result.signal_fill.as_str(), "#334455");
    assert_eq!(result.signal_fill.source(), Src::SkinParam);
}

/// Inline fill beats skinparam-sourced timing value.
#[test]
fn timing_effective_inline_beats_skinparam() {
    use crate::theme::skinparam::TimingStyle;
    let style = TimingStyle {
        signal_background_color: "#334455".to_string(),
        ..TimingStyle::default()
    };
    let result = timing_lane_effective_style(&style, Src::SkinParam, Some("#ff0000"), None, None);
    assert_eq!(result.signal_fill.as_str(), "#ff0000");
    assert_eq!(result.signal_fill.source(), Src::Inline);
}

// ── sequence_participant_effective_style tests ────────────────────────────────

/// Default sequence style — participant fill/stroke resolve from defaults.
#[test]
fn sequence_participant_effective_default() {
    use crate::theme::styles::SequenceStyle;
    let style = SequenceStyle::default();
    let result = sequence_participant_effective_style(&style, Src::Default, None);
    assert_eq!(result.fill.as_str(), "#f6f6f6");
    assert_eq!(result.fill.source(), Src::Default);
    assert_eq!(result.stroke.as_str(), "#111");
    assert_eq!(result.stroke.source(), Src::Default);
}

/// Theme-sourced sequence participant fill propagates.
#[test]
fn sequence_participant_effective_theme_sourced() {
    use crate::theme::styles::SequenceStyle;
    let style = SequenceStyle {
        participant_background_color: "#aabbcc".to_string(),
        ..SequenceStyle::default()
    };
    let result = sequence_participant_effective_style(&style, Src::ThemePreset, None);
    assert_eq!(result.fill.as_str(), "#aabbcc");
    assert_eq!(result.fill.source(), Src::ThemePreset);
}

/// Inline fill beats theme-sourced participant value.
#[test]
fn sequence_participant_effective_inline_beats_theme() {
    use crate::theme::styles::SequenceStyle;
    let style = SequenceStyle {
        participant_background_color: "#aabbcc".to_string(),
        ..SequenceStyle::default()
    };
    let result = sequence_participant_effective_style(&style, Src::ThemePreset, Some("#ff0000"));
    assert_eq!(result.fill.as_str(), "#ff0000");
    assert_eq!(result.fill.source(), Src::Inline);
}

// ── mindmap_node_effective_style tests ───────────────────────────────────────

/// Default mindmap node — no depth style, uses palette default.
#[test]
fn mindmap_effective_default() {
    use crate::theme::styles::MindMapStyle;
    let style = MindMapStyle::default();
    let result = mindmap_node_effective_style(
        &style,
        None,
        Src::Default,
        "#fde68a",
        "#666666",
        "#0f172a",
        None,
    );
    assert_eq!(result.fill.as_str(), "#fde68a");
    assert_eq!(result.fill.source(), Src::Default);
    assert_eq!(result.stroke.as_str(), "#666666");
    assert_eq!(result.stroke.source(), Src::Default);
    assert_eq!(result.font_color.as_str(), "#0f172a");
    assert_eq!(result.font_color.source(), Src::Default);
}

/// Depth style (theme-tier) overrides the palette default.
#[test]
fn mindmap_effective_depth_style_beats_default() {
    use crate::theme::styles::{MindMapDepthStyle, MindMapStyle};
    let style = MindMapStyle::default();
    let depth_style = MindMapDepthStyle {
        background_color: Some("#334455".to_string()),
        border_color: Some("#112233".to_string()),
        font_color: Some("#aabbcc".to_string()),
    };
    let result = mindmap_node_effective_style(
        &style,
        Some(&depth_style),
        Src::ThemePreset,
        "#fde68a",
        "#666666",
        "#0f172a",
        None,
    );
    assert_eq!(result.fill.as_str(), "#334455");
    assert_eq!(result.fill.source(), Src::ThemePreset);
    assert_eq!(result.stroke.as_str(), "#112233");
    assert_eq!(result.font_color.as_str(), "#aabbcc");
}

/// Inline fill_color overrides depth style.
#[test]
fn mindmap_effective_inline_beats_depth_style() {
    use crate::theme::styles::{MindMapDepthStyle, MindMapStyle};
    let style = MindMapStyle::default();
    let depth_style = MindMapDepthStyle {
        background_color: Some("#334455".to_string()),
        ..MindMapDepthStyle::default()
    };
    let result = mindmap_node_effective_style(
        &style,
        Some(&depth_style),
        Src::ThemePreset,
        "#fde68a",
        "#666666",
        "#0f172a",
        Some("#ff0000"),
    );
    assert_eq!(result.fill.as_str(), "#ff0000");
    assert_eq!(result.fill.source(), Src::Inline);
    // stroke + font still from depth_style (no inline for them)
    assert_eq!(result.stroke.as_str(), "#666666");
    assert_eq!(result.stroke.source(), Src::Default);
}
