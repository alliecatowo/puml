//! Unit tests for the shared style cascade resolver.
//!
//! Kept in a separate file to stay within the 600-line file-size guardrail
//! while keeping coverage close to the implementation.

use super::*;
use crate::theme::values::StyleSource as Src;

// ── resolve_color precedence tests ───────────────────────────────────────

/// Baseline: only default, no overrides — should return the default.
#[test]
fn cascade_default_only() {
    let input = CascadeInput::with_default("#ffffff");
    let result = resolve_color(&input);
    assert_eq!(result.as_str(), "#ffffff");
    assert_eq!(result.source(), Src::Default);
}

/// Theme overrides default.
#[test]
fn cascade_theme_beats_default() {
    let mut input = CascadeInput::with_default("#ffffff");
    input.theme = CascadeTier::value("#aabbcc", Src::ThemePreset);
    let result = resolve_color(&input);
    assert_eq!(result.as_str(), "#aabbcc");
    assert_eq!(result.source(), Src::ThemePreset);
}

/// Skinparam overrides theme and default.
#[test]
fn cascade_skinparam_beats_theme() {
    let mut input = CascadeInput::with_default("#ffffff");
    input.theme = CascadeTier::value("#aabbcc", Src::ThemePreset);
    input.skinparam = CascadeTier::value("#112233", Src::SkinParam);
    let result = resolve_color(&input);
    assert_eq!(result.as_str(), "#112233");
    assert_eq!(result.source(), Src::SkinParam);
}

/// Stereotype overrides skinparam.
#[test]
fn cascade_stereotype_beats_skinparam() {
    let mut input = CascadeInput::with_default("#ffffff");
    input.skinparam = CascadeTier::value("#112233", Src::SkinParam);
    input.stereotype = CascadeTier::value("#334455", Src::Stereotype);
    let result = resolve_color(&input);
    assert_eq!(result.as_str(), "#334455");
    assert_eq!(result.source(), Src::Stereotype);
}

/// `<style>` block overrides stereotype.
#[test]
fn cascade_style_block_beats_stereotype() {
    let mut input = CascadeInput::with_default("#ffffff");
    input.stereotype = CascadeTier::value("#334455", Src::Stereotype);
    input.style_block = CascadeTier::value("#556677", Src::StyleBlock);
    let result = resolve_color(&input);
    assert_eq!(result.as_str(), "#556677");
    assert_eq!(result.source(), Src::StyleBlock);
}

/// Inline overrides everything.
#[test]
fn cascade_inline_beats_all() {
    let input = CascadeInput {
        default: CascadeTier::value("#aaaaaa", Src::Default),
        theme: CascadeTier::value("#bbbbbb", Src::ThemePreset),
        skinparam: CascadeTier::value("#cccccc", Src::SkinParam),
        stereotype: CascadeTier::value("#dddddd", Src::Stereotype),
        style_block: CascadeTier::value("#eeeeee", Src::StyleBlock),
        inline: CascadeTier::value("#ff0000", Src::Inline),
    };
    let result = resolve_color(&input);
    assert_eq!(result.as_str(), "#ff0000");
    assert_eq!(result.source(), Src::Inline);
}

/// Full all-absent-except-default (no inline/style/stereotype/skinparam/theme).
#[test]
fn cascade_all_absent_falls_back_to_default() {
    let input = CascadeInput {
        default: CascadeTier::value("#123456", Src::Default),
        theme: CascadeTier::absent(Src::ThemePreset),
        skinparam: CascadeTier::absent(Src::SkinParam),
        stereotype: CascadeTier::absent(Src::Stereotype),
        style_block: CascadeTier::absent(Src::StyleBlock),
        inline: CascadeTier::absent(Src::Inline),
    };
    let result = resolve_color(&input);
    assert_eq!(result.as_str(), "#123456");
    assert_eq!(result.source(), Src::Default);
}

/// Verify that intermediate absent tiers don't block lower-precedence values.
#[test]
fn cascade_absent_tiers_transparent() {
    let mut input = CascadeInput::with_default("#ffffff");
    input.theme = CascadeTier::value("#001122", Src::ThemePreset);
    // skinparam absent, stereotype absent — theme should win over default.
    let result = resolve_color(&input);
    assert_eq!(result.as_str(), "#001122");
    assert_eq!(result.source(), Src::ThemePreset);
}

/// Inline present but style_block absent — inline should win.
#[test]
fn cascade_inline_wins_without_style_block() {
    let mut input = CascadeInput::with_default("#ffffff");
    input.stereotype = CascadeTier::value("#334455", Src::Stereotype);
    input.inline = CascadeTier::value("#ff0000", Src::Inline);
    let result = resolve_color(&input);
    assert_eq!(result.as_str(), "#ff0000");
    assert_eq!(result.source(), Src::Inline);
}

// ── class_node_cascade integration tests ─────────────────────────────────

/// Default tier only — no overrides.
#[test]
fn class_cascade_default_only() {
    let result = class_node_cascade("#ffffff", Src::Default, None, None, None);
    assert_eq!(result.as_str(), "#ffffff");
    assert_eq!(result.source(), Src::Default);
}

/// Theme-sourced diagram color wins over default.
#[test]
fn class_cascade_theme_sourced() {
    let result = class_node_cascade("#aabbcc", Src::ThemePreset, None, None, None);
    assert_eq!(result.as_str(), "#aabbcc");
    assert_eq!(result.source(), Src::ThemePreset);
}

/// Skinparam-sourced diagram color wins over theme.
#[test]
fn class_cascade_skinparam_sourced() {
    let result = class_node_cascade("#112233", Src::SkinParam, None, None, None);
    assert_eq!(result.as_str(), "#112233");
    assert_eq!(result.source(), Src::SkinParam);
}

/// Stereotype overrides skinparam-sourced diagram color.
#[test]
fn class_cascade_stereotype_beats_skinparam() {
    let result = class_node_cascade("#112233", Src::SkinParam, Some("#334455"), None, None);
    assert_eq!(result.as_str(), "#334455");
    assert_eq!(result.source(), Src::Stereotype);
}

/// Inline overrides stereotype.
#[test]
fn class_cascade_inline_beats_stereotype() {
    let result = class_node_cascade(
        "#112233",
        Src::SkinParam,
        Some("#334455"),
        None,
        Some("#ff0000"),
    );
    assert_eq!(result.as_str(), "#ff0000");
    assert_eq!(result.source(), Src::Inline);
}

/// Inline overrides default with no other tiers set.
#[test]
fn class_cascade_inline_beats_default() {
    let result = class_node_cascade("#ffffff", Src::Default, None, None, Some("#ff0000"));
    assert_eq!(result.as_str(), "#ff0000");
    assert_eq!(result.source(), Src::Inline);
}

// ── component_node_cascade tests ──────────────────────────────────────────

/// Default-only, no target/stereotype/inline overrides — returns the default.
#[test]
fn component_cascade_default_only() {
    let result = component_node_cascade("#f0f4f8", Src::Default, None, None, None, None);
    assert_eq!(result.as_str(), "#f0f4f8");
    assert_eq!(result.source(), Src::Default);
}

/// Theme-sourced diagram color wins over default.
#[test]
fn component_cascade_theme_sourced() {
    let result = component_node_cascade("#aabbcc", Src::ThemePreset, None, None, None, None);
    assert_eq!(result.as_str(), "#aabbcc");
    assert_eq!(result.source(), Src::ThemePreset);
}

/// Skinparam-sourced diagram color wins over theme/default.
#[test]
fn component_cascade_skinparam_sourced() {
    let result = component_node_cascade("#112233", Src::SkinParam, None, None, None, None);
    assert_eq!(result.as_str(), "#112233");
    assert_eq!(result.source(), Src::SkinParam);
}

/// Target-specific skinparam (e.g. `skinparam NodeBackgroundColor`) beats
/// the diagram-level generic skinparam.
#[test]
fn component_cascade_target_beats_generic_skinparam() {
    // diagram-level set by skinparam; target override present
    let result = component_node_cascade(
        "#112233",
        Src::SkinParam,
        Some(("#99aabb", Src::SkinParam)),
        None,
        None,
        None,
    );
    assert_eq!(result.as_str(), "#99aabb");
    assert_eq!(result.source(), Src::SkinParam);
}

/// Stereotype beats both target-specific skinparam and diagram skinparam.
#[test]
fn component_cascade_stereotype_beats_target_skinparam() {
    let result = component_node_cascade(
        "#112233",
        Src::SkinParam,
        Some(("#99aabb", Src::SkinParam)),
        Some("#334455"),
        None,
        None,
    );
    assert_eq!(result.as_str(), "#334455");
    assert_eq!(result.source(), Src::Stereotype);
}

/// Inline beats stereotype.
#[test]
fn component_cascade_inline_beats_stereotype() {
    let result = component_node_cascade(
        "#112233",
        Src::SkinParam,
        None,
        Some("#334455"),
        None,
        Some("#ff0000"),
    );
    assert_eq!(result.as_str(), "#ff0000");
    assert_eq!(result.source(), Src::Inline);
}

/// Inline beats everything (all tiers set).
#[test]
fn component_cascade_inline_beats_all() {
    let result = component_node_cascade(
        "#112233",
        Src::SkinParam,
        Some(("#99aabb", Src::SkinParam)),
        Some("#334455"),
        None,
        Some("#ff0000"),
    );
    assert_eq!(result.as_str(), "#ff0000");
    assert_eq!(result.source(), Src::Inline);
}

/// Target absent, no stereotype, no inline — falls back to diagram default.
#[test]
fn component_cascade_all_absent_falls_back_to_default() {
    let result = component_node_cascade("#f0f4f8", Src::Default, None, None, None, None);
    assert_eq!(result.as_str(), "#f0f4f8");
    assert_eq!(result.source(), Src::Default);
}

// ── component_node_effective_style integration tests ──────────────────────

/// Default diagram style, plain component node — fill/stroke/font resolve to defaults.
#[test]
fn component_effective_default_node() {
    use crate::theme::skinparam::ComponentStyle;
    let comp_style = ComponentStyle::default();
    let inline_style = FamilyNodeInlineStyle::default();
    let result =
        component_node_effective_style(&comp_style, None, None, &inline_style, None, false, None);
    assert_eq!(result.fill.as_str(), "#f0f4f8");
    assert_eq!(result.fill.source(), Src::Default);
    assert_eq!(result.stroke.as_str(), "#1e293b");
    assert_eq!(result.stroke.source(), Src::Default);
    assert_eq!(result.font_color.as_str(), "#0f172a");
    assert_eq!(result.font_color.source(), Src::Default);
    assert!(!result.border_dashed);
    assert!((result.stroke_width - 1.5).abs() < 0.001);
}

/// Interface/port nodes use `interface_color` as the fill base.
#[test]
fn component_effective_interface_uses_interface_color() {
    use crate::theme::skinparam::ComponentStyle;
    let comp_style = ComponentStyle::default();
    let inline_style = FamilyNodeInlineStyle::default();
    let result = component_node_effective_style(
        &comp_style,
        None,
        None,
        &inline_style,
        None,
        true, // is_interface_or_port
        None,
    );
    assert_eq!(result.fill.as_str(), "#e2e8f0");
    assert_eq!(result.fill.source(), Src::Default);
}

/// Inline fill_color wins over diagram default.
#[test]
fn component_effective_inline_fill_beats_default() {
    use crate::theme::skinparam::ComponentStyle;
    let comp_style = ComponentStyle::default();
    let inline_style = FamilyNodeInlineStyle::default();
    let result = component_node_effective_style(
        &comp_style,
        None,
        None,
        &inline_style,
        Some("#ff0000"),
        false,
        None,
    );
    assert_eq!(result.fill.as_str(), "#ff0000");
    assert_eq!(result.fill.source(), Src::Inline);
}

/// Stereotype fill wins over diagram default but loses to inline.
#[test]
fn component_effective_stereotype_fill_precedence() {
    use crate::theme::skinparam::{ComponentNodeStyle, ComponentStyle};
    let comp_style = ComponentStyle::default();
    let stereotype = ComponentNodeStyle {
        background_color: Some("#aabbcc".to_string()),
        ..Default::default()
    };
    let inline_style = FamilyNodeInlineStyle::default();

    // Stereotype wins over default
    let result = component_node_effective_style(
        &comp_style,
        None,
        Some(&stereotype),
        &inline_style,
        None,
        false,
        None,
    );
    assert_eq!(result.fill.as_str(), "#aabbcc");
    assert_eq!(result.fill.source(), Src::Stereotype);

    // Inline beats stereotype
    let result2 = component_node_effective_style(
        &comp_style,
        None,
        Some(&stereotype),
        &inline_style,
        Some("#ff0000"),
        false,
        None,
    );
    assert_eq!(result2.fill.as_str(), "#ff0000");
    assert_eq!(result2.fill.source(), Src::Inline);
}

/// Target-specific skinparam wins over diagram default, loses to stereotype.
#[test]
fn component_effective_target_and_stereotype_precedence() {
    use crate::theme::skinparam::{ComponentNodeStyle, ComponentStyle};
    let comp_style = ComponentStyle::default();
    let target = ComponentNodeStyle {
        background_color: Some("#334455".to_string()),
        ..Default::default()
    };
    let stereotype = ComponentNodeStyle {
        background_color: Some("#aabbcc".to_string()),
        ..Default::default()
    };
    let inline_style = FamilyNodeInlineStyle::default();

    // Target wins over default
    let result = component_node_effective_style(
        &comp_style,
        Some(&target),
        None,
        &inline_style,
        None,
        false,
        None,
    );
    assert_eq!(result.fill.as_str(), "#334455");
    assert_eq!(result.fill.source(), Src::SkinParam);

    // Stereotype wins over target
    let result2 = component_node_effective_style(
        &comp_style,
        Some(&target),
        Some(&stereotype),
        &inline_style,
        None,
        false,
        None,
    );
    assert_eq!(result2.fill.as_str(), "#aabbcc");
    assert_eq!(result2.fill.source(), Src::Stereotype);
}
