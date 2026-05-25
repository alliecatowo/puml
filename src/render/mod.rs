pub(crate) use crate::ast::DiagramKind;
pub(crate) use crate::model::{
    ArchimateDocument, ChartDocument, ChartLabelMode, ChartSubtype, DitaaDocument, EbnfDocument,
    EbnfToken, FamilyDocument, FamilyNode, FamilyNodeKind, FamilyOrientation, JsonDocument,
    LegendHAlign, LegendVAlign, MathDocument, MindMapSide, NwdiagDocument, RegexDocument,
    RegexToken, RepeatKind, SdlDocument, SdlStateKind, StateDocument, StateNode, StateNodeKind,
    TimelineChronologyEvent, TimelineDocument, TimelineMilestone, TimelineResourceOffRange,
    TimelineTask, WbsCheckbox, YamlDocument,
};
pub(crate) use std::collections::BTreeMap;

mod activity;
mod chen;
mod data;
mod family;
mod geometry;
pub(crate) mod graph_layout;
pub(crate) mod layout_constants;
mod mindmap;
mod relation;
mod salt;
mod sequence;
mod specialized;
mod state;
mod stdlib;
mod svg;
mod text;
pub(crate) mod text_metrics;
mod text_output;
mod text_specialized;
mod text_timeline;
mod timeline;
mod timing;
pub mod validate;

use crate::render_core::RenderScene;

#[derive(Debug, Default)]
pub struct RenderArtifact {
    pub svg: String,
    pub scene: Option<RenderScene>,
    pub invariant_report: Option<validate::InvariantReport>,
}

impl RenderArtifact {
    pub fn svg_only(svg: String) -> Self {
        Self {
            svg,
            scene: None,
            invariant_report: None,
        }
    }

    pub fn with_scene(svg: String, scene: RenderScene) -> Self {
        Self {
            svg,
            scene: Some(scene),
            invariant_report: None,
        }
    }
}

pub use activity::render_activity_svg;
pub use chen::render_chen_svg;
pub use data::{render_json_svg, render_yaml_svg};
pub use family::{
    render_class_artifact, render_class_svg, render_component_artifact, render_component_svg,
    render_deployment_artifact, render_deployment_svg, render_family_stub_artifact,
    render_family_stub_svg, render_family_tree_svg,
};
pub use mindmap::{render_mindmap_svg, render_wbs_svg};
pub use salt::render_salt_svg;
pub use sequence::render_svg;
pub use specialized::{
    render_archimate_svg, render_chart_svg, render_ditaa_svg, render_ebnf_svg, render_math_svg,
    render_nwdiag_svg, render_regex_svg, render_sdl_svg,
};
pub use state::render_state_svg;
pub use stdlib::render_stdlib_svg;
pub use text::{render_text_pages, TextOutputMode};
pub use timeline::{render_timeline_stub_svg, render_timeline_svg};
pub use timing::render_timing_svg;

pub(crate) use family::family_node_label;
pub(crate) use geometry::compute_edge_anchors_for_direction;

pub(crate) fn append_mainframe_svg(svg: &mut String, title: &str) {
    let Some(width) = svg_numeric_attr(svg, "width") else {
        return;
    };
    let Some(height) = svg_numeric_attr(svg, "height") else {
        return;
    };
    let Some(insert_at) = svg.rfind("</svg>") else {
        return;
    };
    if width <= 8 || height <= 8 {
        return;
    }

    const INSET: i32 = 4;
    const NOTCH_H: i32 = 20;
    const NOTCH_CUT: i32 = 6;
    let notch_w = ((title.chars().count() as i32 * 7) + 16).clamp(32, width - 2 * INSET);
    let stroke = "#1e293b";
    let fill = "#ffffff";
    let x = INSET;
    let y = INSET;
    let w = width - 2 * INSET;
    let h = height - 2 * INSET;

    let mut frame = format!(
        "<rect class=\"uml-mainframe\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>"
    );
    frame.push_str(&format!(
        "<polygon class=\"uml-mainframe-title\" points=\"{},{} {},{} {},{} {},{} {},{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        x,
        y,
        x + notch_w,
        y,
        x + notch_w,
        y + NOTCH_H - NOTCH_CUT,
        x + notch_w - NOTCH_CUT,
        y + NOTCH_H,
        x,
        y + NOTCH_H,
        fill,
        stroke
    ));
    if !title.is_empty() {
        frame.push_str(&creole_text(
            x + 8,
            y + 14,
            "font-family=\"monospace\" font-size=\"12\" font-weight=\"600\"",
            title,
            stroke,
        ));
    }
    svg.insert_str(insert_at, &frame);
}

fn svg_numeric_attr(svg: &str, attr: &str) -> Option<i32> {
    let needle = format!("{attr}=\"");
    let start = svg.find(&needle)? + needle.len();
    let value = svg[start..].split('"').next()?;
    value.parse::<f64>().ok().map(|v| v.round() as i32)
}
pub(crate) use relation::render_relation_marker_defs;
pub(crate) use svg::{creole_text, escape_text, render_sprite_sheet, with_sprite_registry};
