//! Shared style cascade — per-family effective-style functions for the
//! remaining families wired in #1184 (activity, state, timing, sequence,
//! mindmap).
//!
//! This file is included from `shared_cascade.rs` via `#[path]` so that the
//! cascade implementation stays under the 600-line file-size guardrail.
//!
//! All functions here delegate to [`super::simple_cascade`] and
//! [`super::resolve_color`] — the single canonical resolver — to enforce the
//! same precedence for every family:
//!
//! `default < theme < skinparam < stereotype < <style> < inline`

use super::super::values::StyleSource as Src;
use super::super::values::{EffectiveStyleValue, StyleColor};
use super::{resolve_color, simple_cascade, CascadeInput, CascadeTier};

// ─── Activity-family integration ─────────────────────────────────────────────

use super::super::skinparam::ActivityStyle;

/// Resolved per-node colours for an activity diagram element.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveActivityNodeStyle {
    pub fill: EffectiveStyleValue<StyleColor>,
    pub stroke: EffectiveStyleValue<StyleColor>,
    pub font_color: EffectiveStyleValue<StyleColor>,
}

/// Compute the fully-resolved style for an activity diagram element via the
/// shared precedence cascade.
///
/// `diagram_source` must reflect how the values in `activity_style` were set
/// (Default / ThemePreset / SkinParam).  The caller tracks this.
/// `inline_fill` — `#color` shorthand on the node declaration, if any.
/// `inline_stroke` / `inline_font` — per-member style tokens, if any.
pub fn activity_node_effective_style(
    activity_style: &ActivityStyle,
    diagram_source: Src,
    inline_fill: Option<&str>,
    inline_stroke: Option<&str>,
    inline_font: Option<&str>,
) -> EffectiveActivityNodeStyle {
    let fill = simple_cascade(
        &activity_style.background_color,
        diagram_source,
        None,
        inline_fill,
    );
    let stroke = simple_cascade(
        &activity_style.border_color,
        diagram_source,
        None,
        inline_stroke,
    );
    let font_color = simple_cascade(
        &activity_style.font_color,
        diagram_source,
        None,
        inline_font,
    );
    EffectiveActivityNodeStyle {
        fill,
        stroke,
        font_color,
    }
}

// ─── State-family integration ─────────────────────────────────────────────────

use super::super::skinparam::StateStyle;

/// Resolved per-node colours for a state diagram element.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveStateNodeStyle {
    pub fill: EffectiveStyleValue<StyleColor>,
    pub stroke: EffectiveStyleValue<StyleColor>,
    pub font_color: EffectiveStyleValue<StyleColor>,
}

/// Compute the fully-resolved style for a state diagram node via the shared
/// precedence cascade.
///
/// `inline_fill` — `StateNodeStyle::fill_color`, if set.
/// `inline_stroke` — `StateNodeStyle::border_color`, if set.
/// `inline_font` — `StateNodeStyle::text_color`, if set.
pub fn state_node_effective_style(
    state_style: &StateStyle,
    diagram_source: Src,
    inline_fill: Option<&str>,
    inline_stroke: Option<&str>,
    inline_font: Option<&str>,
) -> EffectiveStateNodeStyle {
    let fill = simple_cascade(
        &state_style.background_color,
        diagram_source,
        None,
        inline_fill,
    );
    let stroke = simple_cascade(
        &state_style.border_color,
        diagram_source,
        None,
        inline_stroke,
    );
    let font_color = simple_cascade(&state_style.font_color, diagram_source, None, inline_font);
    EffectiveStateNodeStyle {
        fill,
        stroke,
        font_color,
    }
}

// ─── Timing-family integration ────────────────────────────────────────────────

use super::super::skinparam::TimingStyle;

/// Resolved lane/signal colours for a timing diagram.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveTimingLaneStyle {
    pub signal_fill: EffectiveStyleValue<StyleColor>,
    pub signal_stroke: EffectiveStyleValue<StyleColor>,
    pub font_color: EffectiveStyleValue<StyleColor>,
    pub arrow_color: EffectiveStyleValue<StyleColor>,
}

/// Compute the fully-resolved style for a timing diagram lane via the shared
/// precedence cascade.
///
/// Timing lanes do not carry per-lane inline overrides in the current model,
/// so `inline_*` args are `None` at call sites.  The parameters are present so
/// future callers can thread inline tokens without a signature change.
pub fn timing_lane_effective_style(
    timing_style: &TimingStyle,
    diagram_source: Src,
    inline_fill: Option<&str>,
    inline_stroke: Option<&str>,
    inline_font: Option<&str>,
) -> EffectiveTimingLaneStyle {
    let signal_fill = simple_cascade(
        &timing_style.signal_background_color,
        diagram_source,
        None,
        inline_fill,
    );
    let signal_stroke = simple_cascade(
        &timing_style.signal_border_color,
        diagram_source,
        None,
        inline_stroke,
    );
    let font_color = simple_cascade(&timing_style.font_color, diagram_source, None, inline_font);
    let arrow_color = simple_cascade(&timing_style.arrow_color, diagram_source, None, None);
    EffectiveTimingLaneStyle {
        signal_fill,
        signal_stroke,
        font_color,
        arrow_color,
    }
}

// ─── Sequence-family integration ──────────────────────────────────────────────

use super::super::styles::SequenceStyle;

/// Resolved per-participant colours for a sequence diagram.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveSequenceParticipantStyle {
    pub fill: EffectiveStyleValue<StyleColor>,
    pub stroke: EffectiveStyleValue<StyleColor>,
}

/// Compute the fully-resolved style for a sequence diagram participant via the
/// shared precedence cascade.
///
/// `inline_fill` — the participant-level `#color` shorthand token, if any.
pub fn sequence_participant_effective_style(
    seq_style: &SequenceStyle,
    diagram_source: Src,
    inline_fill: Option<&str>,
) -> EffectiveSequenceParticipantStyle {
    let fill = simple_cascade(
        &seq_style.participant_background_color,
        diagram_source,
        None,
        inline_fill,
    );
    let stroke = simple_cascade(
        &seq_style.participant_border_color,
        diagram_source,
        None,
        None,
    );
    EffectiveSequenceParticipantStyle { fill, stroke }
}

// ─── MindMap-family integration ───────────────────────────────────────────────

use super::super::styles::{MindMapDepthStyle, MindMapStyle};

/// Resolved per-node colours for a MindMap element.
#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveMindMapNodeStyle {
    pub fill: EffectiveStyleValue<StyleColor>,
    pub stroke: EffectiveStyleValue<StyleColor>,
    pub font_color: EffectiveStyleValue<StyleColor>,
}

/// Compute the fully-resolved style for a MindMap node at a given depth via
/// the shared precedence cascade.
///
/// `depth_style` — the per-depth override from `MindMapStyle::depth_styles`,
/// if any.  Treated as the skinparam tier.
/// `default_fill` — the palette fallback colour for this depth.
/// `inline_fill`  — `FamilyNode::fill_color`, if set.
pub fn mindmap_node_effective_style(
    _mindmap_style: &MindMapStyle,
    depth_style: Option<&MindMapDepthStyle>,
    diagram_source: Src,
    default_fill: &str,
    default_stroke: &str,
    default_font: &str,
    inline_fill: Option<&str>,
) -> EffectiveMindMapNodeStyle {
    // depth_style overrides act as a skinparam-tier override (more specific than
    // the diagram-level default but less specific than an inline token).
    let depth_fill = depth_style.and_then(|s| s.background_color.as_deref());
    let depth_stroke = depth_style.and_then(|s| s.border_color.as_deref());
    let depth_font = depth_style.and_then(|s| s.font_color.as_deref());

    let fill = {
        let mut input = CascadeInput::with_default(default_fill);
        if let Some(c) = depth_fill {
            input.skinparam = CascadeTier::value(c, diagram_source);
        }
        if let Some(c) = inline_fill {
            input.inline = CascadeTier::value(c, Src::Inline);
        }
        resolve_color(&input)
    };
    let stroke = {
        let mut input = CascadeInput::with_default(default_stroke);
        if let Some(c) = depth_stroke {
            input.skinparam = CascadeTier::value(c, diagram_source);
        }
        resolve_color(&input)
    };
    let font_color = {
        let mut input = CascadeInput::with_default(default_font);
        if let Some(c) = depth_font {
            input.skinparam = CascadeTier::value(c, diagram_source);
        }
        resolve_color(&input)
    };
    EffectiveMindMapNodeStyle {
        fill,
        stroke,
        font_color,
    }
}
