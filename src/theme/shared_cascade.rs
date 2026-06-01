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
//! - [x] Class family      — `effective_class_node_style` wired through this module
//! - [x] Component family  — `component_node_effective_style` wired through this module
//! - [x] Deployment family — shares `ComponentStyle`; covered by the component wiring above
//! - [x] Activity family   — `activity_node_effective_style` wired through this module
//! - [x] State family      — `state_node_effective_style` wired through this module
//! - [x] Timing family     — `timing_lane_effective_style` wired through this module
//! - [x] Sequence family   — `sequence_participant_effective_style` wired through this module
//! - [x] MindMap family    — `mindmap_node_effective_style` wired through this module

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
/// `style_block_color` — the colour resolved from `<style>` block rules (Phase B).
/// `inline_color` — the colour from the inline `#color` token on the element,
/// if any.
fn class_node_cascade(
    diagram_color: &str,
    diagram_source: Src,
    stereotype_color: Option<&str>,
    style_block_color: Option<&str>,
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
        // Phase B (#1404): populate the style_block tier from `<style>` rules.
        style_block: CascadeTier {
            color: style_block_color.map(str::to_string),
            source: Src::StyleBlock,
        },
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
/// * `class_style`        — Diagram-level style (captures theme + skinparam results).
/// * `scoped_style`       — Stereotype-scoped overrides for this node, if any.
/// * `inline_style`       — Inline token overrides extracted from the node members.
/// * `fill_inline`        — Node-level fill colour from the `#color` shorthand token
///   (`FamilyNode::fill_color`). Separate from `inline_style`
///   because the parser stores it in a dedicated field.
/// * `element_stereotype` — Lower-cased stereotype key for this node (e.g. `"service"`),
///   used to build the `StyleQuery` for Phase B `<style>` block lookup.
///
/// # Precedence per property (lowest → highest)
/// default < theme < skinparam < stereotype < `<style>` < inline
pub fn class_node_effective_style(
    class_style: &ClassStyle,
    scoped_style: Option<&super::skinparam::ClassStereotypeStyle>,
    inline_style: &FamilyNodeInlineStyle,
    fill_inline: Option<&str>,
    element_stereotype: Option<&str>,
) -> EffectiveClassNodeStyle {
    use super::style_builder::StyleQuery;
    use crate::ast::style::{PName, SName};

    let title_font_size = class_style.font_size.unwrap_or(13);

    // ── Phase B: resolve `<style>` block colours ──────────────────────────────
    // Build a query for this class element and look up any style-block rules.
    let style_block_resolved = class_style.style_builder.as_deref().map(|builder| {
        let mut query = StyleQuery::tags([SName::ClassDiagram, SName::Class_]);
        if let Some(stereo) = element_stereotype {
            query = query.with_stereotype(stereo);
        }
        builder.resolve(&query)
    });

    let sb_fill = style_block_resolved
        .as_ref()
        .and_then(|es| es.color(PName::BackgroundColor));
    let sb_stroke = style_block_resolved
        .as_ref()
        .and_then(|es| es.color(PName::LineColor));
    let sb_font = style_block_resolved
        .as_ref()
        .and_then(|es| es.color(PName::FontColor));
    let sb_header = style_block_resolved
        .as_ref()
        .and_then(|es| es.color(PName::HeadColor));

    // ── fill (background) ─────────────────────────────────────────────────────
    // fill_inline maps to `FamilyNode::fill_color` (#color token on declaration).
    let fill = class_node_cascade(
        &class_style.background_color,
        class_style.sources.background_color,
        scoped_style.and_then(|s| s.background_color.as_deref()),
        sb_fill,
        fill_inline,
    );

    // ── stroke (border) ───────────────────────────────────────────────────────
    let stroke = class_node_cascade(
        &class_style.border_color,
        class_style.sources.border_color,
        scoped_style.and_then(|s| s.border_color.as_deref()),
        sb_stroke,
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
        sb_font,
        inline_style.text_color.as_deref(),
    );

    // ── member_color ──────────────────────────────────────────────────────────
    let member_color = class_node_cascade(
        &class_style.member_color,
        class_style.sources.member_color,
        scoped_font_color,
        sb_font,
        inline_style.text_color.as_deref(),
    );

    // ── header_color ──────────────────────────────────────────────────────────
    let header_color = class_node_cascade(
        &class_style.header_color,
        class_style.sources.header_color,
        scoped_style.and_then(|s| s.header_color.as_deref()),
        sb_header,
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

// ─── Component/Deployment-family integration ─────────────────────────────────

use super::effective::EffectiveComponentNodeStyle;
use super::skinparam::{ComponentNodeStyle, ComponentStyle};

/// Build a [`CascadeInput`] for a single colour property of a component or
/// deployment node.
///
/// The component family has one extra layer compared to class: target-specific
/// skinparam directives (e.g. `skinparam NodeBackgroundColor`) are more
/// specific than the diagram-level `skinparam BackgroundColor` and therefore
/// win over it.  We model this by placing the target-specific value at the
/// `skinparam` tier and falling through to the diagram-level color (with its
/// recorded source tier) only when no target override exists.
///
/// Precedence enforced (lowest → highest):
///   default < theme < skinparam/target-skinparam < stereotype < inline
///
/// Arguments:
/// * `diagram_color`    — The colour in the diagram-level `ComponentStyle`
///   field (may have been sourced from default, theme, or skinparam).
/// * `diagram_source`   — The [`StyleSource`] that produced `diagram_color`.
/// * `target_color`     — Per-element-kind skinparam override (e.g.
///   `skinparam NodeBackgroundColor`), if set for this node kind.
/// * `target_source`    — Source tier for the target override.
/// * `stereotype_color` — Stereotype-scoped override from
///   `skinparam Foo<<Bar>>`, if any.
/// * `inline_color`     — Inline `#color` token on the element, if any.
fn component_node_cascade(
    diagram_color: &str,
    diagram_source: Src,
    target_color: Option<(&str, Src)>,
    stereotype_color: Option<&str>,
    style_block_color: Option<&str>,
    inline_color: Option<&str>,
) -> EffectiveStyleValue<StyleColor> {
    // Split the diagram-level source into theme / skinparam tiers.
    // If a target-specific skinparam override is present it takes the skinparam
    // slot (it is more specific than the diagram-level generic skinparam).
    let (theme_color, skinparam_color) = match target_color {
        Some((tc, _)) => {
            // Target override exists: use it at the skinparam tier regardless
            // of what the diagram-level source was.
            (None, Some(tc.to_string()))
        }
        None => match diagram_source {
            Src::ThemePreset => (Some(diagram_color.to_string()), None),
            Src::SkinParam | Src::StyleBlock => (None, Some(diagram_color.to_string())),
            Src::Default | Src::Stereotype | Src::Inline => (None, None),
        },
    };

    // The default tier carries the raw diagram default so there is always a
    // fallback, even when all other tiers are absent.
    let effective_default =
        if matches!(diagram_source, Src::Default | Src::Stereotype | Src::Inline) {
            diagram_color.to_string()
        } else {
            // diagram_color came from a higher tier; still store it as a safety net.
            diagram_color.to_string()
        };

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
        // Phase B (#1404): populate the style_block tier from `<style>` rules.
        style_block: CascadeTier {
            color: style_block_color.map(str::to_string),
            source: Src::StyleBlock,
        },
        inline: CascadeTier {
            color: inline_color.map(str::to_string),
            source: Src::Inline,
        },
    };
    resolve_color(&input)
}

/// Compute the fully-resolved per-node style for a component or deployment
/// diagram element via the shared precedence cascade.
///
/// This replaces the hand-rolled chain in `effective.rs` and produces
/// output-equivalent results while routing all lookups through the uniform
/// cascade so precedence is tested in one place.
///
/// Precedence per property (lowest → highest):
///   default < theme < skinparam < target-specific-skinparam < stereotype < `<style>` < inline
///
/// # Arguments
///
/// * `component_style`    — Diagram-level style aggregating theme + skinparam.
/// * `target_style`       — Per-element-kind skinparam overrides (e.g.
///   `skinparam NodeBackgroundColor`), derived from
///   [`ComponentStyle::target_styles`].
/// * `stereotype_style`   — Stereotype-scoped overrides for this element.
/// * `inline_style`       — Member-encoded inline overrides.
/// * `fill_inline`        — Node-level fill from the `#color` shorthand token
///   (`FamilyNode::fill_color`).
/// * `is_interface_or_port` — When true, use `interface_color` as the
///   fallback fill rather than `background_color`.
/// * `element_stereotype` — Lower-cased stereotype key for this node,
///   used to build the `StyleQuery` for Phase B `<style>` block lookup.
pub fn component_node_effective_style(
    component_style: &ComponentStyle,
    target_style: Option<&ComponentNodeStyle>,
    stereotype_style: Option<&ComponentNodeStyle>,
    inline_style: &FamilyNodeInlineStyle,
    fill_inline: Option<&str>,
    is_interface_or_port: bool,
    element_stereotype: Option<&str>,
) -> EffectiveComponentNodeStyle {
    use super::style_builder::StyleQuery;
    use crate::ast::style::{PName, SName};

    // ── Phase B: resolve `<style>` block colours ──────────────────────────────
    let style_block_resolved = component_style.style_builder.as_deref().map(|builder| {
        let mut query = StyleQuery::tags([SName::ComponentDiagram, SName::Component]);
        if let Some(stereo) = element_stereotype {
            query = query.with_stereotype(stereo);
        }
        builder.resolve(&query)
    });

    let sb_fill = style_block_resolved
        .as_ref()
        .and_then(|es| es.color(PName::BackgroundColor));
    let sb_stroke = style_block_resolved
        .as_ref()
        .and_then(|es| es.color(PName::LineColor));
    let sb_font = style_block_resolved
        .as_ref()
        .and_then(|es| es.color(PName::FontColor));

    // ── fill (background) ─────────────────────────────────────────────────────
    let (base_fill, base_fill_source) = if is_interface_or_port {
        (
            component_style.interface_color.as_str(),
            component_style.sources.interface_color,
        )
    } else {
        (
            component_style.background_color.as_str(),
            component_style.sources.background_color,
        )
    };
    let target_fill = target_style
        .and_then(|s| s.background_color.as_deref())
        .map(|c| (c, Src::SkinParam));
    let fill = component_node_cascade(
        base_fill,
        base_fill_source,
        target_fill,
        stereotype_style.and_then(|s| s.background_color.as_deref()),
        sb_fill,
        fill_inline,
    );

    // ── stroke (border) ───────────────────────────────────────────────────────
    let target_stroke = target_style
        .and_then(|s| s.border_color.as_deref())
        .map(|c| (c, Src::SkinParam));
    let stroke = component_node_cascade(
        component_style.border_color.as_str(),
        component_style.sources.border_color,
        target_stroke,
        stereotype_style.and_then(|s| s.border_color.as_deref()),
        sb_stroke,
        inline_style.border_color.as_deref(),
    );

    // ── font_color ────────────────────────────────────────────────────────────
    let target_font = target_style
        .and_then(|s| s.font_color.as_deref())
        .map(|c| (c, Src::SkinParam));
    let font_color = component_node_cascade(
        component_style.font_color.as_str(),
        component_style.sources.font_color,
        target_font,
        stereotype_style.and_then(|s| s.font_color.as_deref()),
        sb_font,
        inline_style.text_color.as_deref(),
    );

    EffectiveComponentNodeStyle {
        fill,
        stroke,
        font_color,
        border_dashed: inline_style.border_dashed,
        stroke_width: inline_style.border_thickness.unwrap_or(1.5),
    }
}

// ─── Simple-family cascade helper ────────────────────────────────────────────
//
// Activity, State, Timing, Sequence, and MindMap style structs do not carry
// per-field source tracking (unlike ClassStyle / ComponentStyle).  Callers
// supply `diagram_source` explicitly — they know whether the current value
// originated from a theme preset, a skinparam directive, or the built-in
// default.
pub(crate) fn simple_cascade(
    diagram_color: &str,
    diagram_source: Src,
    stereotype_color: Option<&str>,
    inline_color: Option<&str>,
) -> EffectiveStyleValue<StyleColor> {
    let (theme_color, skinparam_color) = match diagram_source {
        Src::ThemePreset => (Some(diagram_color.to_string()), None),
        Src::SkinParam | Src::StyleBlock => (None, Some(diagram_color.to_string())),
        Src::Default | Src::Stereotype | Src::Inline => (None, None),
    };
    let input = CascadeInput {
        default: CascadeTier {
            color: Some(diagram_color.to_string()),
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

// ─── Additional-family cascade functions ─────────────────────────────────────
// Activity, State, Timing, Sequence, and MindMap families are in a separate
// module to stay within the 600-line file-size guardrail.
#[path = "shared_cascade_families.rs"]
pub mod families;
pub use families::{
    activity_node_effective_style, mindmap_node_effective_style,
    sequence_participant_effective_style, sequence_participant_effective_style_with_stereotype,
    state_node_effective_style, timing_lane_effective_style, EffectiveActivityNodeStyle,
    EffectiveMindMapNodeStyle, EffectiveSequenceParticipantStyle, EffectiveStateNodeStyle,
    EffectiveTimingLaneStyle,
};

// ─── Unit tests ──────────────────────────────────────────────────────────────
// Tests are in separate files to stay within the 600-line file-size guardrail.

#[cfg(test)]
#[path = "shared_cascade_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "shared_cascade_family_tests.rs"]
mod family_tests;
