use crate::model::{FamilyNode, FamilyNodeKind};

use super::shared_cascade::families::{
    activity_node_effective_style as shared_activity_effective,
    mindmap_node_effective_style as shared_mindmap_effective,
    sequence_participant_effective_style as shared_sequence_effective,
    state_node_effective_style as shared_state_effective,
    timing_lane_effective_style as shared_timing_effective, EffectiveActivityNodeStyle,
    EffectiveMindMapNodeStyle, EffectiveSequenceParticipantStyle, EffectiveStateNodeStyle,
    EffectiveTimingLaneStyle,
};
use super::shared_cascade::{
    class_node_effective_style as shared_class_effective,
    component_node_effective_style as shared_component_effective,
};
use super::skinparam::{ActivityStyle, StateStyle, TimingStyle};
use super::styles::{MindMapDepthStyle, MindMapStyle, SequenceStyle};
use super::values::StyleSource;
use super::{ClassStyle, ComponentStyle, ComponentStyleTarget, EffectiveStyleValue};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FamilyNodeInlineStyle {
    pub border_color: Option<String>,
    pub text_color: Option<String>,
    pub border_dashed: bool,
    pub border_thickness: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveClassNodeStyle {
    pub fill: EffectiveStyleValue<super::StyleColor>,
    pub stroke: EffectiveStyleValue<super::StyleColor>,
    pub font_color: EffectiveStyleValue<super::StyleColor>,
    pub member_color: EffectiveStyleValue<super::StyleColor>,
    pub header_color: EffectiveStyleValue<super::StyleColor>,
    pub border_dashed: bool,
    pub stroke_width: f32,
    pub font_family: String,
    pub title_font_size: u32,
    pub member_font_size: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EffectiveComponentNodeStyle {
    pub fill: EffectiveStyleValue<super::StyleColor>,
    pub stroke: EffectiveStyleValue<super::StyleColor>,
    pub font_color: EffectiveStyleValue<super::StyleColor>,
    pub border_dashed: bool,
    pub stroke_width: f32,
}

pub fn family_node_inline_style(node: &FamilyNode) -> FamilyNodeInlineStyle {
    let mut style = FamilyNodeInlineStyle::default();
    for member in &node.members {
        let text = member.text.trim();
        if let Some(color) = text.strip_prefix("\x1fstyle:border:") {
            style.border_color = Some(color.trim().to_string());
        } else if let Some(color) = text.strip_prefix("\x1fstyle:text:") {
            style.text_color = Some(color.trim().to_string());
        } else if text == "\x1fstyle:border-dashed" {
            style.border_dashed = true;
        } else if let Some(width) = text.strip_prefix("\x1fstyle:border-thickness:") {
            if let Ok(width) = width.trim().parse::<f32>() {
                style.border_thickness = Some(width.clamp(1.0, 8.0));
            }
        }
    }
    style
}

pub fn family_node_stereotype_key(node: &FamilyNode) -> Option<String> {
    node.members.iter().find_map(|member| {
        let text = member.text.trim();
        is_user_stereotype_marker(text).then(|| {
            text.trim_start_matches("<<")
                .trim_end_matches(">>")
                .trim()
                .to_ascii_lowercase()
        })
    })
}

pub fn component_style_target_for_node(kind: FamilyNodeKind) -> Option<ComponentStyleTarget> {
    match kind {
        FamilyNodeKind::Actor | FamilyNodeKind::Person => Some(ComponentStyleTarget::Actor),
        FamilyNodeKind::Artifact => Some(ComponentStyleTarget::Artifact),
        FamilyNodeKind::Boundary => Some(ComponentStyleTarget::Boundary),
        FamilyNodeKind::Cloud => Some(ComponentStyleTarget::Cloud),
        FamilyNodeKind::Component => Some(ComponentStyleTarget::Component),
        FamilyNodeKind::Control => Some(ComponentStyleTarget::Control),
        FamilyNodeKind::Database => Some(ComponentStyleTarget::Database),
        FamilyNodeKind::Entity => Some(ComponentStyleTarget::Entity),
        FamilyNodeKind::File => Some(ComponentStyleTarget::File),
        FamilyNodeKind::Folder => Some(ComponentStyleTarget::Folder),
        FamilyNodeKind::Frame => Some(ComponentStyleTarget::Frame),
        FamilyNodeKind::Interface => Some(ComponentStyleTarget::Interface),
        FamilyNodeKind::Node => Some(ComponentStyleTarget::Node),
        FamilyNodeKind::Package => Some(ComponentStyleTarget::Package),
        FamilyNodeKind::Port => Some(ComponentStyleTarget::Port),
        FamilyNodeKind::Queue => Some(ComponentStyleTarget::Queue),
        FamilyNodeKind::Storage => Some(ComponentStyleTarget::Storage),
        FamilyNodeKind::UseCaseDeployment => Some(ComponentStyleTarget::UseCase),
        _ => None,
    }
}

/// Compute the fully-resolved per-node style for a component or deployment
/// diagram element.
///
/// Delegates to the shared cascade resolver
/// ([`shared_cascade::component_node_effective_style`]) which enforces the
/// documented precedence:
///   default < theme < skinparam < target-specific-skinparam < stereotype < inline
///
/// The `node.fill_color` field (the `#color` shorthand on the declaration
/// line) is the Inline-tier override for the fill/background property.
pub fn effective_component_node_style(
    component_style: &ComponentStyle,
    node: &FamilyNode,
) -> EffectiveComponentNodeStyle {
    let target_style = component_style_target_for_node(node.kind)
        .and_then(|target| component_style.target_styles.get(&target));
    let stereotype_style = family_node_stereotype_key(node)
        .and_then(|key| component_style.stereotype_styles.get(&key));
    let inline_style = family_node_inline_style(node);
    let is_interface_or_port =
        matches!(node.kind, FamilyNodeKind::Interface | FamilyNodeKind::Port);
    let fill_inline = node.fill_color.as_deref();
    shared_component_effective(
        component_style,
        target_style,
        stereotype_style,
        &inline_style,
        fill_inline,
        is_interface_or_port,
    )
}

/// Compute the fully-resolved per-node style for a class-family element.
///
/// Delegates to the shared cascade resolver ([`shared_cascade::class_node_effective_style`])
/// which enforces the documented precedence:
///   default < theme < skinparam < stereotype < `<style>` < inline
///
/// The `node.fill_color` field (the `#color` shorthand on the declaration line)
/// is the Inline-tier override for the fill/background property; it is threaded
/// into the cascade separately from the member-encoded `inline_style`.
pub fn effective_class_node_style(
    class_style: &ClassStyle,
    node: &FamilyNode,
) -> EffectiveClassNodeStyle {
    let scoped_style =
        family_node_stereotype_key(node).and_then(|key| class_style.stereotype_styles.get(&key));
    let inline_style = family_node_inline_style(node);
    let fill_inline = node.fill_color.as_deref();
    shared_class_effective(class_style, scoped_style, &inline_style, fill_inline)
}

/// Compute the fully-resolved per-node style for an activity diagram element.
///
/// Delegates to the shared cascade resolver, which enforces the documented
/// precedence: `default < theme < skinparam < stereotype < <style> < inline`.
///
/// `diagram_source` reflects how the values in `activity_style` were last
/// written (Default / ThemePreset / SkinParam).  Inline overrides come from
/// `FamilyNode::fill_color` and the member-encoded inline style.
pub fn effective_activity_node_style(
    activity_style: &ActivityStyle,
    diagram_source: StyleSource,
    node: &FamilyNode,
) -> EffectiveActivityNodeStyle {
    let inline_style = family_node_inline_style(node);
    let fill_inline = node.fill_color.as_deref();
    shared_activity_effective(
        activity_style,
        diagram_source,
        fill_inline,
        inline_style.border_color.as_deref(),
        inline_style.text_color.as_deref(),
    )
}

/// Compute the fully-resolved per-node style for a state diagram element.
///
/// Delegates to the shared cascade resolver.  Inline overrides come from the
/// per-node `StateNodeStyle` (which mirrors `FamilyNodeInlineStyle`'s tokens).
pub fn effective_state_node_style(
    state_style: &StateStyle,
    diagram_source: StyleSource,
    inline_fill: Option<&str>,
    inline_stroke: Option<&str>,
    inline_font: Option<&str>,
) -> EffectiveStateNodeStyle {
    shared_state_effective(
        state_style,
        diagram_source,
        inline_fill,
        inline_stroke,
        inline_font,
    )
}

/// Compute the fully-resolved style for a timing diagram lane.
///
/// Delegates to the shared cascade resolver.  Timing lanes do not currently
/// carry per-lane inline tokens in the model, so `inline_*` arguments are
/// usually `None` at call sites; the parameters exist so future overrides can
/// be threaded without a signature change.
pub fn effective_timing_lane_style(
    timing_style: &TimingStyle,
    diagram_source: StyleSource,
    inline_fill: Option<&str>,
    inline_stroke: Option<&str>,
    inline_font: Option<&str>,
) -> EffectiveTimingLaneStyle {
    shared_timing_effective(
        timing_style,
        diagram_source,
        inline_fill,
        inline_stroke,
        inline_font,
    )
}

/// Compute the fully-resolved per-participant style for a sequence diagram.
///
/// Delegates to the shared cascade resolver.  `inline_fill` is the
/// participant-level `#color` shorthand token, when supplied by the parser.
pub fn effective_sequence_participant_style(
    seq_style: &SequenceStyle,
    diagram_source: StyleSource,
    inline_fill: Option<&str>,
) -> EffectiveSequenceParticipantStyle {
    shared_sequence_effective(seq_style, diagram_source, inline_fill)
}

/// Compute the fully-resolved per-node style for a MindMap element.
///
/// Delegates to the shared cascade resolver.  `depth_style` is the per-depth
/// override from `MindMapStyle::depth_styles` (treated as the skinparam tier);
/// `default_*` are the palette fallbacks for this depth; `inline_fill` is the
/// `FamilyNode::fill_color` inline `#color` token.
pub fn effective_mindmap_node_style(
    mindmap_style: &MindMapStyle,
    depth_style: Option<&MindMapDepthStyle>,
    diagram_source: StyleSource,
    default_fill: &str,
    default_stroke: &str,
    default_font: &str,
    inline_fill: Option<&str>,
) -> EffectiveMindMapNodeStyle {
    shared_mindmap_effective(
        mindmap_style,
        depth_style,
        diagram_source,
        default_fill,
        default_stroke,
        default_font,
        inline_fill,
    )
}

fn is_user_stereotype_marker(text: &str) -> bool {
    text.starts_with("<<") && text.ends_with(">>") && !is_builtin_type_stereotype_marker(text)
}

fn is_builtin_type_stereotype_marker(text: &str) -> bool {
    matches!(
        text,
        "<<enum>>"
            | "<<interface>>"
            | "<<abstract>>"
            | "<<abstract class>>"
            | "<<annotation>>"
            | "<<protocol>>"
            | "<<struct>>"
    )
}
