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

// ── style-block tier (Phase B) exercises ────────────────────────────────

/// style_block beats skinparam-sourced diagram color.
#[test]
fn class_cascade_style_block_beats_skinparam() {
    let result = class_node_cascade("#112233", Src::SkinParam, None, Some("#778899"), None);
    assert_eq!(result.as_str(), "#778899");
    assert_eq!(result.source(), Src::StyleBlock);
}

/// style_block beats stereotype when stereotype is lower precedence.
#[test]
fn class_cascade_style_block_beats_stereotype() {
    let result = class_node_cascade(
        "#112233",
        Src::SkinParam,
        Some("#334455"),
        Some("#778899"),
        None,
    );
    assert_eq!(result.as_str(), "#778899");
    assert_eq!(result.source(), Src::StyleBlock);
}

/// Inline beats style_block (inline is highest precedence).
#[test]
fn class_cascade_inline_beats_style_block() {
    let result = class_node_cascade(
        "#112233",
        Src::SkinParam,
        Some("#334455"),
        Some("#778899"),
        Some("#ff0000"),
    );
    assert_eq!(result.as_str(), "#ff0000");
    assert_eq!(result.source(), Src::Inline);
}

/// component cascade: style_block tier beats skinparam.
#[test]
fn component_cascade_style_block_beats_skinparam() {
    let result =
        component_node_cascade("#f0f4f8", Src::SkinParam, None, None, Some("#aabbcc"), None);
    assert_eq!(result.as_str(), "#aabbcc");
    assert_eq!(result.source(), Src::StyleBlock);
}

/// component cascade: style_block tier beats stereotype.
#[test]
fn component_cascade_style_block_beats_stereotype() {
    let result = component_node_cascade(
        "#f0f4f8",
        Src::SkinParam,
        None,
        Some("#334455"),
        Some("#aabbcc"),
        None,
    );
    assert_eq!(result.as_str(), "#aabbcc");
    assert_eq!(result.source(), Src::StyleBlock);
}

/// component cascade: inline beats style_block.
#[test]
fn component_cascade_inline_beats_style_block() {
    let result = component_node_cascade(
        "#f0f4f8",
        Src::SkinParam,
        None,
        None,
        Some("#aabbcc"),
        Some("#ff0000"),
    );
    assert_eq!(result.as_str(), "#ff0000");
    assert_eq!(result.source(), Src::Inline);
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

// ── Phase D (#1416): class_node_effective_style with style-builder properties ──

/// Phase D: `LineThickness` in a style-builder sets stroke_width.
#[test]
fn class_effective_phase_d_line_thickness_from_style_builder() {
    use crate::ast::style::{
        PName, SName, SelectorChain, SelectorSegment, StyleRule, StyleScheme, StyleValue,
    };
    use crate::theme::skinparam::ClassStyle;
    use crate::theme::style_builder::StyleBuilder;
    use std::collections::BTreeMap;

    let mut builder = StyleBuilder::new();
    let rule = StyleRule {
        selector_path: vec![
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::ClassDiagram)],
            },
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::Class_)],
            },
        ],
        properties: [(PName::LineThickness, StyleValue::Number(3.0))]
            .iter()
            .cloned()
            .collect(),
        unknown_properties: BTreeMap::new(),
        source_order: 1,
        scheme: StyleScheme::Regular,
    };
    builder.push(rule);

    let class_style = ClassStyle { style_builder: Some(Box::new(builder)), ..ClassStyle::default() };
    let inline = FamilyNodeInlineStyle::default();
    let result = class_node_effective_style(&class_style, None, &inline, None, None);
    assert!(
        (result.stroke_width - 3.0).abs() < 0.01,
        "LineThickness 3 must set stroke_width=3.0, got {}",
        result.stroke_width
    );
}

/// Phase D: `RoundCorner` in a style-builder sets style_round_corner.
#[test]
fn class_effective_phase_d_round_corner_from_style_builder() {
    use crate::ast::style::{
        PName, SName, SelectorChain, SelectorSegment, StyleRule, StyleScheme, StyleValue,
    };
    use crate::theme::skinparam::ClassStyle;
    use crate::theme::style_builder::StyleBuilder;
    use std::collections::BTreeMap;

    let mut builder = StyleBuilder::new();
    let rule = StyleRule {
        selector_path: vec![
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::ClassDiagram)],
            },
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::Class_)],
            },
        ],
        properties: [(PName::RoundCorner, StyleValue::Number(20.0))]
            .iter()
            .cloned()
            .collect(),
        unknown_properties: BTreeMap::new(),
        source_order: 1,
        scheme: StyleScheme::Regular,
    };
    builder.push(rule);

    let class_style = ClassStyle { style_builder: Some(Box::new(builder)), ..ClassStyle::default() };
    let inline = FamilyNodeInlineStyle::default();
    let result = class_node_effective_style(&class_style, None, &inline, None, None);
    assert_eq!(
        result.style_round_corner,
        Some(20),
        "RoundCorner 20 must set style_round_corner=Some(20)"
    );
}

/// Phase D: `FontWeight bold` keyword in a style-builder sets font_weight=700.
#[test]
fn class_effective_phase_d_font_weight_bold_from_style_builder() {
    use crate::ast::style::{
        PName, SName, SelectorChain, SelectorSegment, StyleRule, StyleScheme, StyleValue,
    };
    use crate::theme::skinparam::ClassStyle;
    use crate::theme::style_builder::StyleBuilder;
    use std::collections::BTreeMap;

    let mut builder = StyleBuilder::new();
    let rule = StyleRule {
        selector_path: vec![
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::ClassDiagram)],
            },
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::Class_)],
            },
        ],
        properties: [(PName::FontWeight, StyleValue::Keyword("bold".to_string()))]
            .iter()
            .cloned()
            .collect(),
        unknown_properties: BTreeMap::new(),
        source_order: 1,
        scheme: StyleScheme::Regular,
    };
    builder.push(rule);

    let class_style = ClassStyle { style_builder: Some(Box::new(builder)), ..ClassStyle::default() };
    let inline = FamilyNodeInlineStyle::default();
    let result = class_node_effective_style(&class_style, None, &inline, None, None);
    assert_eq!(
        result.font_weight, 700,
        "FontWeight bold must set font_weight=700"
    );
}

/// Phase D: `LineStyle dashed` in a style-builder sets EffectiveLineStyle::Dashed.
#[test]
fn class_effective_phase_d_line_style_dashed_from_style_builder() {
    use crate::ast::style::{
        PName, SName, SelectorChain, SelectorSegment, StyleRule, StyleScheme, StyleValue,
    };
    use crate::theme::effective::EffectiveLineStyle;
    use crate::theme::skinparam::ClassStyle;
    use crate::theme::style_builder::StyleBuilder;
    use std::collections::BTreeMap;

    let mut builder = StyleBuilder::new();
    let rule = StyleRule {
        selector_path: vec![
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::ClassDiagram)],
            },
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::Class_)],
            },
        ],
        properties: [(PName::LineStyle, StyleValue::Keyword("dashed".to_string()))]
            .iter()
            .cloned()
            .collect(),
        unknown_properties: BTreeMap::new(),
        source_order: 1,
        scheme: StyleScheme::Regular,
    };
    builder.push(rule);

    let class_style = ClassStyle { style_builder: Some(Box::new(builder)), ..ClassStyle::default() };
    let inline = FamilyNodeInlineStyle::default();
    let result = class_node_effective_style(&class_style, None, &inline, None, None);
    assert_eq!(result.line_style, EffectiveLineStyle::Dashed);
    assert_eq!(result.line_style.stroke_dasharray(), "8 4");
}

/// Phase D: `HorizontalAlignment right` in a style-builder sets EffectiveHAlign::Right.
#[test]
fn class_effective_phase_d_h_align_right_from_style_builder() {
    use crate::ast::style::{
        PName, SName, SelectorChain, SelectorSegment, StyleRule, StyleScheme, StyleValue,
    };
    use crate::theme::effective::EffectiveHAlign;
    use crate::theme::skinparam::ClassStyle;
    use crate::theme::style_builder::StyleBuilder;
    use std::collections::BTreeMap;

    let mut builder = StyleBuilder::new();
    let rule = StyleRule {
        selector_path: vec![
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::ClassDiagram)],
            },
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::Class_)],
            },
        ],
        properties: [(
            PName::HorizontalAlignment,
            StyleValue::Keyword("right".to_string()),
        )]
        .iter()
        .cloned()
        .collect(),
        unknown_properties: BTreeMap::new(),
        source_order: 1,
        scheme: StyleScheme::Regular,
    };
    builder.push(rule);

    let class_style = ClassStyle { style_builder: Some(Box::new(builder)), ..ClassStyle::default() };
    let inline = FamilyNodeInlineStyle::default();
    let result = class_node_effective_style(&class_style, None, &inline, None, None);
    assert_eq!(result.h_align, EffectiveHAlign::Right);
    assert_eq!(result.h_align.text_anchor(), "end");
}

/// Phase D: `EffectiveLineStyle::Solid` returns empty dasharray (used when no LineStyle is set).
#[test]
fn effective_line_style_solid_returns_empty_dasharray() {
    use crate::theme::effective::EffectiveLineStyle;
    assert_eq!(EffectiveLineStyle::Solid.stroke_dasharray(), "");
    assert_eq!(EffectiveLineStyle::Dashed.stroke_dasharray(), "8 4");
    assert_eq!(EffectiveLineStyle::Dotted.stroke_dasharray(), "2 3");
}

/// Phase D: `EffectiveHAlign::Center` returns "middle" text-anchor.
#[test]
fn effective_h_align_center_returns_middle() {
    use crate::theme::effective::EffectiveHAlign;
    assert_eq!(EffectiveHAlign::Center.text_anchor(), "middle");
    assert_eq!(EffectiveHAlign::Left.text_anchor(), "start");
    assert_eq!(EffectiveHAlign::Right.text_anchor(), "end");
}

/// Phase D: `Shadowing true` in a style-builder sets shadowing=true.
#[test]
fn class_effective_phase_d_shadowing_from_style_builder() {
    use crate::ast::style::{
        PName, SName, SelectorChain, SelectorSegment, StyleRule, StyleScheme, StyleValue,
    };
    use crate::theme::skinparam::ClassStyle;
    use crate::theme::style_builder::StyleBuilder;
    use std::collections::BTreeMap;

    let mut builder = StyleBuilder::new();
    let rule = StyleRule {
        selector_path: vec![
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::ClassDiagram)],
            },
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::Class_)],
            },
        ],
        properties: [(PName::Shadowing, StyleValue::Keyword("true".to_string()))]
            .iter()
            .cloned()
            .collect(),
        unknown_properties: BTreeMap::new(),
        source_order: 1,
        scheme: StyleScheme::Regular,
    };
    builder.push(rule);

    let class_style = ClassStyle {
        shadowing: false, // start with false
        style_builder: Some(Box::new(builder)),
        ..ClassStyle::default()
    };
    let inline = FamilyNodeInlineStyle::default();
    let result = class_node_effective_style(&class_style, None, &inline, None, None);
    assert!(
        result.shadowing,
        "Shadowing true in style-builder must override skinparam false"
    );
}

/// Phase D: `Padding` in a style-builder sets padding field.
#[test]
fn class_effective_phase_d_padding_from_style_builder() {
    use crate::ast::style::{
        PName, SName, SelectorChain, SelectorSegment, StyleRule, StyleScheme, StyleValue,
    };
    use crate::theme::skinparam::ClassStyle;
    use crate::theme::style_builder::StyleBuilder;
    use std::collections::BTreeMap;

    let mut builder = StyleBuilder::new();
    let rule = StyleRule {
        selector_path: vec![
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::ClassDiagram)],
            },
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::Class_)],
            },
        ],
        properties: [(PName::Padding, StyleValue::Number(12.0))]
            .iter()
            .cloned()
            .collect(),
        unknown_properties: BTreeMap::new(),
        source_order: 1,
        scheme: StyleScheme::Regular,
    };
    builder.push(rule);

    let class_style = ClassStyle { style_builder: Some(Box::new(builder)), ..ClassStyle::default() };
    let inline = FamilyNodeInlineStyle::default();
    let result = class_node_effective_style(&class_style, None, &inline, None, None);
    assert_eq!(result.padding, 12, "Padding 12 must set padding=12");
}

/// Phase D: `MaximumWidth` in a style-builder sets max_width.
#[test]
fn class_effective_phase_d_max_width_from_style_builder() {
    use crate::ast::style::{
        PName, SName, SelectorChain, SelectorSegment, StyleRule, StyleScheme, StyleValue,
    };
    use crate::theme::skinparam::ClassStyle;
    use crate::theme::style_builder::StyleBuilder;
    use std::collections::BTreeMap;

    let mut builder = StyleBuilder::new();
    let rule = StyleRule {
        selector_path: vec![
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::ClassDiagram)],
            },
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::Class_)],
            },
        ],
        properties: [(PName::MaximumWidth, StyleValue::Number(200.0))]
            .iter()
            .cloned()
            .collect(),
        unknown_properties: BTreeMap::new(),
        source_order: 1,
        scheme: StyleScheme::Regular,
    };
    builder.push(rule);

    let class_style = ClassStyle { style_builder: Some(Box::new(builder)), ..ClassStyle::default() };
    let inline = FamilyNodeInlineStyle::default();
    let result = class_node_effective_style(&class_style, None, &inline, None, None);
    assert_eq!(
        result.max_width, 200,
        "MaximumWidth 200 must set max_width=200"
    );
}

/// Phase D: `MinimumWidth` in a style-builder sets min_width.
#[test]
fn class_effective_phase_d_min_width_from_style_builder() {
    use crate::ast::style::{
        PName, SName, SelectorChain, SelectorSegment, StyleRule, StyleScheme, StyleValue,
    };
    use crate::theme::skinparam::ClassStyle;
    use crate::theme::style_builder::StyleBuilder;
    use std::collections::BTreeMap;

    let mut builder = StyleBuilder::new();
    let rule = StyleRule {
        selector_path: vec![
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::ClassDiagram)],
            },
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::Class_)],
            },
        ],
        properties: [(PName::MinimumWidth, StyleValue::Number(100.0))]
            .iter()
            .cloned()
            .collect(),
        unknown_properties: BTreeMap::new(),
        source_order: 1,
        scheme: StyleScheme::Regular,
    };
    builder.push(rule);

    let class_style = ClassStyle { style_builder: Some(Box::new(builder)), ..ClassStyle::default() };
    let inline = FamilyNodeInlineStyle::default();
    let result = class_node_effective_style(&class_style, None, &inline, None, None);
    assert_eq!(
        result.min_width, 100,
        "MinimumWidth 100 must set min_width=100"
    );
}

/// Phase D: `LineStyle: dotted` in a style-builder yields `EffectiveLineStyle::Dotted`.
#[test]
fn class_effective_phase_d_line_style_dotted_from_style_builder() {
    use crate::ast::style::{
        PName, SName, SelectorChain, SelectorSegment, StyleRule, StyleScheme, StyleValue,
    };
    use crate::theme::effective::EffectiveLineStyle;
    use crate::theme::skinparam::ClassStyle;
    use crate::theme::style_builder::StyleBuilder;
    use std::collections::BTreeMap;

    let mut builder = StyleBuilder::new();
    let rule = StyleRule {
        selector_path: vec![
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::ClassDiagram)],
            },
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::Class_)],
            },
        ],
        properties: [(PName::LineStyle, StyleValue::Keyword("dotted".into()))]
            .iter()
            .cloned()
            .collect(),
        unknown_properties: BTreeMap::new(),
        source_order: 1,
        scheme: StyleScheme::Regular,
    };
    builder.push(rule);

    let class_style = ClassStyle { style_builder: Some(Box::new(builder)), ..ClassStyle::default() };
    let inline = FamilyNodeInlineStyle::default();
    let result = class_node_effective_style(&class_style, None, &inline, None, None);
    assert_eq!(result.line_style, EffectiveLineStyle::Dotted);
}

/// Phase D: `LineStyle: solid` in a style-builder yields `EffectiveLineStyle::Solid`.
#[test]
fn class_effective_phase_d_line_style_solid_from_style_builder() {
    use crate::ast::style::{
        PName, SName, SelectorChain, SelectorSegment, StyleRule, StyleScheme, StyleValue,
    };
    use crate::theme::effective::EffectiveLineStyle;
    use crate::theme::skinparam::ClassStyle;
    use crate::theme::style_builder::StyleBuilder;
    use std::collections::BTreeMap;

    let mut builder = StyleBuilder::new();
    let rule = StyleRule {
        selector_path: vec![
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::ClassDiagram)],
            },
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::Class_)],
            },
        ],
        properties: [(PName::LineStyle, StyleValue::Keyword("solid".into()))]
            .iter()
            .cloned()
            .collect(),
        unknown_properties: BTreeMap::new(),
        source_order: 1,
        scheme: StyleScheme::Regular,
    };
    builder.push(rule);

    let class_style = ClassStyle { style_builder: Some(Box::new(builder)), ..ClassStyle::default() };
    let inline = FamilyNodeInlineStyle::default();
    let result = class_node_effective_style(&class_style, None, &inline, None, None);
    assert_eq!(result.line_style, EffectiveLineStyle::Solid);
}

/// Phase D: `HorizontalAlignment: left` in a style-builder yields `EffectiveHAlign::Left`.
#[test]
fn class_effective_phase_d_h_align_left_from_style_builder() {
    use crate::ast::style::{
        PName, SName, SelectorChain, SelectorSegment, StyleRule, StyleScheme, StyleValue,
    };
    use crate::theme::effective::EffectiveHAlign;
    use crate::theme::skinparam::ClassStyle;
    use crate::theme::style_builder::StyleBuilder;
    use std::collections::BTreeMap;

    let mut builder = StyleBuilder::new();
    let rule = StyleRule {
        selector_path: vec![
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::ClassDiagram)],
            },
            SelectorChain {
                segments: vec![SelectorSegment::Tag(SName::Class_)],
            },
        ],
        properties: [(
            PName::HorizontalAlignment,
            StyleValue::Keyword("left".into()),
        )]
        .iter()
        .cloned()
        .collect(),
        unknown_properties: BTreeMap::new(),
        source_order: 1,
        scheme: StyleScheme::Regular,
    };
    builder.push(rule);

    let class_style = ClassStyle { style_builder: Some(Box::new(builder)), ..ClassStyle::default() };
    let inline = FamilyNodeInlineStyle::default();
    let result = class_node_effective_style(&class_style, None, &inline, None, None);
    assert_eq!(result.h_align, EffectiveHAlign::Left);
}
