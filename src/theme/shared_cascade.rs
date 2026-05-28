//! Shared style cascade resolver — issue #1184.
//!
//! # Precedence (lowest → highest)
//!
//! | Tier | Source | `StyleSource` variant |
//! |------|--------|----------------------|
//! | 1 | Built-in defaults (hard-coded in the style struct) | `Default` |
//! | 2 | Active `!theme` preset | `ThemePreset` |
//! | 3 | `skinparam` directives | `SkinParam` |
//! | 4 | Matching stereotype style (`skinparam Foo<<Bar>>`) | `Stereotype` |
//! | 5 | `<style>` block property | `StyleBlock` |
//! | 6 | Inline token on the element (`#color`) | `Inline` |
//!
//! Higher tiers always win over lower tiers for the same property.
//!
//! # Usage
//!
//! Build a [`CascadeInput`] for a concrete element (node, relation…), call
//! [`resolve_color`] for each colour property, and you get back an
//! [`EffectiveStyleValue<StyleColor>`] that carries both the resolved value and
//! the tier that sourced it (useful for diagnostics / future per-element
//! inheritance logic).
//!
//! # Migration status — TODO(#1184)
//!
//! - [x] Class family  — `effective_class_node_style` wired through this module
//! - [ ] Component family  — still uses `effective_component_node_style` directly
//! - [ ] Activity family   — `ActivityStyle` not yet wired
//! - [ ] State family      — `StateStyle` not yet wired
//! - [ ] Timing family     — `TimingStyle` not yet wired
//! - [ ] Sequence family   — `SequenceStyle` not yet wired
//! - [ ] MindMap family    — `MindMapStyle` not yet wired

use super::values::{EffectiveStyleValue, StyleColor, StyleSource};

/// A single colour tier: an optional colour value paired with the source that
/// produced it.  `None` means "this tier has nothing to say about this
/// property".
#[derive(Debug, Clone)]
pub struct CascadeTier {
    /// The colour value at this tier, if set.
    pub color: Option<String>,
    /// Source label for this tier.
    pub source: StyleSource,
}

impl CascadeTier {
    /// A tier that carries a concrete value.
    pub fn value(color: impl Into<String>, source: StyleSource) -> Self {
        Self {
            color: Some(color.into()),
            source,
        }
    }

    /// A tier that is absent (this source did not set the property).
    pub fn absent(source: StyleSource) -> Self {
        Self {
            color: None,
            source,
        }
    }
}

/// All inputs needed to resolve one colour property for one element.
///
/// Tiers are ordered lowest-to-highest precedence:
/// `default → theme → skinparam → stereotype → style_block → inline`.
///
/// Callers fill in only the tiers that are relevant; leave unused tiers as
/// [`CascadeTier::absent`].
#[derive(Debug, Clone)]
pub struct CascadeInput {
    /// Tier 1 — built-in default.
    pub default: CascadeTier,
    /// Tier 2 — active `!theme` preset.
    pub theme: CascadeTier,
    /// Tier 3 — `skinparam` directive.
    pub skinparam: CascadeTier,
    /// Tier 4 — matching stereotype style.
    pub stereotype: CascadeTier,
    /// Tier 5 — `<style>` block rule.
    pub style_block: CascadeTier,
    /// Tier 6 — inline token on the element.
    pub inline: CascadeTier,
}

impl CascadeInput {
    /// Construct a minimal input with only a default tier.  All other tiers
    /// start absent.
    pub fn with_default(color: impl Into<String>) -> Self {
        Self {
            default: CascadeTier::value(color, StyleSource::Default),
            theme: CascadeTier::absent(StyleSource::ThemePreset),
            skinparam: CascadeTier::absent(StyleSource::SkinParam),
            stereotype: CascadeTier::absent(StyleSource::Stereotype),
            style_block: CascadeTier::absent(StyleSource::StyleBlock),
            inline: CascadeTier::absent(StyleSource::Inline),
        }
    }
}

/// Resolve a colour property from a [`CascadeInput`].
///
/// Iterates the tiers from highest to lowest precedence and returns the first
/// tier that carries a non-`None` value.  The returned
/// [`EffectiveStyleValue`] includes both the resolved colour and the
/// [`StyleSource`] that provided it.
///
/// The function never fails: the `default` tier is always present, so there is
/// always at least one tier to fall back to.
pub fn resolve_color(input: &CascadeInput) -> EffectiveStyleValue<StyleColor> {
    // Walk highest-to-lowest precedence.
    let tiers: [&CascadeTier; 6] = [
        &input.inline,
        &input.style_block,
        &input.stereotype,
        &input.skinparam,
        &input.theme,
        &input.default,
    ];
    for tier in tiers {
        if let Some(ref color) = tier.color {
            return EffectiveStyleValue::color(color.clone(), tier.source);
        }
    }
    // Safety: `default` is always present (guaranteed by CascadeInput API).
    unreachable!("CascadeInput::default must carry a value")
}

// ─── Class-family integration ────────────────────────────────────────────────

use super::effective::{EffectiveClassNodeStyle, FamilyNodeInlineStyle};
use super::skinparam::ClassStyle;
use super::values::StyleSource as Src;

/// Build a [`CascadeInput`] for a single colour property of a class node,
/// given the diagram-level [`ClassStyle`] (which captures theme + skinparam
/// resolution), an optional scoped stereotype style override, and the inline
/// token extracted from the node.
///
/// `diagram_color` — the colour currently stored in the ClassStyle field (may
/// have been set by theme or skinparam; the `sources` field tells us which).
/// `diagram_source` — the `StyleSource` recorded in ClassStyle.sources.
/// `stereotype_color` — the colour set by a matching stereotype rule, if any.
/// `inline_color` — the colour from the inline `#color` token on the element,
/// if any.
fn class_node_cascade(
    diagram_color: &str,
    diagram_source: Src,
    stereotype_color: Option<&str>,
    inline_color: Option<&str>,
) -> EffectiveStyleValue<StyleColor> {
    // diagram_source tells us which tier populated the diagram-level field.
    // We split it back into theme vs. skinparam for the full 6-tier input.
    let (theme_color, skinparam_color) = match diagram_source {
        Src::ThemePreset => (Some(diagram_color.to_string()), None),
        Src::SkinParam | Src::StyleBlock => (None, Some(diagram_color.to_string())),
        Src::Default => (None, None),
        // Stereotype / Inline are per-element, not diagram-level — shouldn't occur here.
        Src::Stereotype | Src::Inline => (None, None),
    };

    // The default tier must always carry a value so `resolve_color` never hits
    // the `unreachable!`.  When the diagram_color came from a higher tier, the
    // default field is `None` (the cascade will find the value in its tier) but
    // we keep the raw diagram_color as an unconditional safety net.
    let default_color = if matches!(diagram_source, Src::Default | Src::Stereotype | Src::Inline) {
        Some(diagram_color.to_string())
    } else {
        None
    };
    // Ensure there is always at least one value to fall back to.
    let effective_default = default_color
        .or_else(|| theme_color.clone().or_else(|| skinparam_color.clone()))
        .unwrap_or_else(|| diagram_color.to_string());

    let input = CascadeInput {
        default: CascadeTier {
            color: Some(effective_default),
            source: Src::Default,
        },
        theme: CascadeTier {
            color: theme_color,
            source: Src::ThemePreset,
        },
        skinparam: CascadeTier {
            color: skinparam_color,
            source: Src::SkinParam,
        },
        stereotype: CascadeTier {
            color: stereotype_color.map(str::to_string),
            source: Src::Stereotype,
        },
        style_block: CascadeTier::absent(Src::StyleBlock),
        inline: CascadeTier {
            color: inline_color.map(str::to_string),
            source: Src::Inline,
        },
    };
    resolve_color(&input)
}

/// Compute the fully-resolved per-node style for a class diagram element.
///
/// This replaces the hand-rolled chain in `effective.rs` and routes all colour
/// lookups through the shared cascade so the precedence is enforced uniformly
/// and tested in one place.
///
/// # Arguments
///
/// * `class_style`   — Diagram-level style (captures theme + skinparam results).
/// * `scoped_style`  — Stereotype-scoped overrides for this node, if any.
/// * `inline_style`  — Inline token overrides extracted from the node members.
/// * `fill_inline`   — Node-level fill colour from the `#color` shorthand token
///   (`FamilyNode::fill_color`). Separate from `inline_style`
///   because the parser stores it in a dedicated field.
///
/// # Precedence per property (lowest → highest)
/// default < theme < skinparam < stereotype < `<style>` < inline
pub fn class_node_effective_style(
    class_style: &ClassStyle,
    scoped_style: Option<&super::skinparam::ClassStereotypeStyle>,
    inline_style: &FamilyNodeInlineStyle,
    fill_inline: Option<&str>,
) -> EffectiveClassNodeStyle {
    let title_font_size = class_style.font_size.unwrap_or(13);

    // ── fill (background) ─────────────────────────────────────────────────────
    // fill_inline maps to `FamilyNode::fill_color` (#color token on declaration).
    let fill = class_node_cascade(
        &class_style.background_color,
        class_style.sources.background_color,
        scoped_style.and_then(|s| s.background_color.as_deref()),
        fill_inline,
    );

    // ── stroke (border) ───────────────────────────────────────────────────────
    let stroke = class_node_cascade(
        &class_style.border_color,
        class_style.sources.border_color,
        scoped_style.and_then(|s| s.border_color.as_deref()),
        inline_style.border_color.as_deref(),
    );

    // ── font_color ────────────────────────────────────────────────────────────
    let scoped_font_color = scoped_style
        .and_then(|s| s.font_color.as_deref())
        .filter(|c| !c.is_empty());
    let font_color = class_node_cascade(
        &class_style.font_color,
        class_style.sources.font_color,
        scoped_font_color,
        inline_style.text_color.as_deref(),
    );

    // ── member_color ──────────────────────────────────────────────────────────
    let member_color = class_node_cascade(
        &class_style.member_color,
        class_style.sources.member_color,
        scoped_font_color,
        inline_style.text_color.as_deref(),
    );

    // ── header_color ──────────────────────────────────────────────────────────
    let header_color = class_node_cascade(
        &class_style.header_color,
        class_style.sources.header_color,
        scoped_style.and_then(|s| s.header_color.as_deref()),
        None, // no inline override for header
    );

    EffectiveClassNodeStyle {
        fill,
        stroke,
        font_color,
        member_color,
        header_color,
        border_dashed: inline_style.border_dashed,
        stroke_width: inline_style.border_thickness.unwrap_or(1.5),
        font_family: class_style
            .font_name
            .clone()
            .unwrap_or_else(|| "monospace".to_string()),
        title_font_size,
        member_font_size: title_font_size.saturating_sub(2).max(9),
    }
}

// ─── Unit tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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
        let result = class_node_cascade("#ffffff", Src::Default, None, None);
        assert_eq!(result.as_str(), "#ffffff");
        assert_eq!(result.source(), Src::Default);
    }

    /// Theme-sourced diagram color wins over default.
    #[test]
    fn class_cascade_theme_sourced() {
        let result = class_node_cascade("#aabbcc", Src::ThemePreset, None, None);
        assert_eq!(result.as_str(), "#aabbcc");
        assert_eq!(result.source(), Src::ThemePreset);
    }

    /// Skinparam-sourced diagram color wins over theme.
    #[test]
    fn class_cascade_skinparam_sourced() {
        let result = class_node_cascade("#112233", Src::SkinParam, None, None);
        assert_eq!(result.as_str(), "#112233");
        assert_eq!(result.source(), Src::SkinParam);
    }

    /// Stereotype overrides skinparam-sourced diagram color.
    #[test]
    fn class_cascade_stereotype_beats_skinparam() {
        let result = class_node_cascade("#112233", Src::SkinParam, Some("#334455"), None);
        assert_eq!(result.as_str(), "#334455");
        assert_eq!(result.source(), Src::Stereotype);
    }

    /// Inline overrides stereotype.
    #[test]
    fn class_cascade_inline_beats_stereotype() {
        let result =
            class_node_cascade("#112233", Src::SkinParam, Some("#334455"), Some("#ff0000"));
        assert_eq!(result.as_str(), "#ff0000");
        assert_eq!(result.source(), Src::Inline);
    }

    /// Inline overrides default with no other tiers set.
    #[test]
    fn class_cascade_inline_beats_default() {
        let result = class_node_cascade("#ffffff", Src::Default, None, Some("#ff0000"));
        assert_eq!(result.as_str(), "#ff0000");
        assert_eq!(result.source(), Src::Inline);
    }
}
