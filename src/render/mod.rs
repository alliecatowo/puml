pub(crate) use crate::ast::DiagramKind;
pub(crate) use crate::model::{
    ArchimateDocument, ChartDocument, ChartLabelMode, ChartSubtype, DitaaDocument, EbnfDocument,
    EbnfToken, FamilyDocument, FamilyNode, FamilyNodeKind, FamilyOrientation, JsonDocument,
    LegendHAlign, LegendVAlign, MathDocument, MindMapSide, NwdiagDocument, RegexDocument,
    RegexToken, RepeatKind, SdlDocument, SdlStateKind, StateDocument, StateNode, StateNodeKind,
    TimelineChronologyEvent, TimelineDocument, TimelineMilestone, TimelineTask, WbsCheckbox,
    YamlDocument,
};
pub(crate) use crate::theme::css3_color_to_hex;
pub(crate) use std::collections::BTreeMap;

mod activity;
mod chen;
pub mod contract;
mod data;
mod family;
mod geometry;
pub(crate) mod graph_layout;
mod mindmap;
mod relation;
mod salt;
pub mod scene_graph;
mod semantic_svg;
mod sequence;
mod specialized;
mod state;
mod svg;
mod text;
mod timeline;
mod timing;
pub mod validate;

pub use activity::render_activity_svg;
pub use chen::render_chen_svg;
pub use contract::{validate_svg, RawSvg, RenderProfile, SvgContractError, ValidatedSvg};
pub use data::{render_json_svg, render_yaml_svg};
pub use family::{
    render_class_svg, render_component_svg, render_deployment_svg, render_family_stub_svg,
    render_family_tree_svg,
};
pub use mindmap::{render_mindmap_svg, render_wbs_svg};
pub use salt::render_salt_svg;
pub use sequence::render_svg;
pub use specialized::{
    render_archimate_svg, render_chart_svg, render_ditaa_svg, render_ebnf_svg, render_math_svg,
    render_nwdiag_svg, render_regex_svg, render_sdl_svg,
};
pub use state::render_state_svg;
pub use text::{render_text_pages, TextOutputMode};
pub use timeline::{render_timeline_stub_svg, render_timeline_svg};
pub use timing::render_timing_svg;

pub(crate) use family::family_node_label;
pub(crate) use geometry::compute_edge_anchors_for_direction;
pub(crate) use relation::render_relation_marker_defs;
pub(crate) use semantic_svg::{
    container_attrs as puml_container_attrs, edge_attrs as puml_edge_attrs,
    label_attrs as puml_label_attrs, node_attrs as puml_node_attrs,
};
pub(crate) use svg::{creole_text, escape_text};
