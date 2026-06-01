pub(crate) use crate::ast::DiagramKind;
pub(crate) use crate::model::{
    ArchimateDocument, BoardCard, BoardDocument, ChartDocument, ChartLabelMode, ChartSubtype,
    DitaaDocument, EbnfDocument, EbnfToken, FamilyDocument, FamilyNode, FamilyNodeKind,
    FamilyOrientation, FileTreeNode, FilesDocument, JsonDocument, LegendHAlign, LegendVAlign,
    MathDocument, MindMapSide, NwdiagDocument, RegexDocument, RegexToken, RepeatKind, SdlDocument,
    SdlStateKind, StateDocument, StateNode, StateNodeKind, TimelineChronologyEvent,
    TimelineDatePrecision, TimelineDocument, TimelineMilestone, TimelineResourceOffRange,
    TimelineTask, WbsCheckbox, YamlDocument,
};
pub(crate) use std::collections::BTreeMap;

mod activity;
mod board_files;
mod chen;
mod data;
pub(crate) mod edge_smoothing;
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
mod text_family_misc;
pub(crate) mod text_metrics;
mod text_output;
mod text_specialized;
mod text_timeline;
mod timeline;
mod timing;
pub mod validate;
mod wire;

pub use crate::output::{RenderArtifact, RenderArtifactDimensions, RenderSceneContract};
use crate::render_core::SceneAvailability;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderValidationState {
    NotRun,
    SvgBackstop,
    TypedScene,
}

impl RenderArtifact {
    pub fn scene_availability(&self) -> SceneAvailability {
        self.scene_availability
    }

    pub fn validation_state(&self) -> RenderValidationState {
        match (&self.invariant_report, self.typed_scene()) {
            (None, _) => RenderValidationState::NotRun,
            (Some(_), Some(_)) => RenderValidationState::TypedScene,
            (Some(_), None) => RenderValidationState::SvgBackstop,
        }
    }

    pub fn validate_svg(&mut self, mode: validate::AutoCorrect) {
        let scene = if matches!(self.scene_availability, SceneAvailability::TypedScene) {
            self.scene.as_ref()
        } else {
            None
        };
        self.invariant_report = Some(validate::run_with_scene(&mut self.svg, scene, mode).into());
        self.refresh_svg_metadata();
    }
}

pub use activity::{render_activity_artifact, render_activity_svg};
pub use board_files::{
    render_board_artifact, render_board_svg, render_files_artifact, render_files_svg,
};
pub use chen::{render_chen_artifact, render_chen_svg};
pub use data::{render_json_artifact, render_json_svg, render_yaml_artifact, render_yaml_svg};
pub use family::{
    render_class_artifact, render_class_svg, render_component_artifact, render_component_svg,
    render_deployment_artifact, render_deployment_svg, render_family_stub_artifact,
    render_family_stub_svg, render_family_tree_artifact, render_family_tree_svg,
};
pub use mindmap::{
    render_mindmap_artifact, render_mindmap_svg, render_wbs_artifact, render_wbs_svg,
};
pub use salt::{render_salt_artifact, render_salt_svg};
pub use sequence::{render_artifact as render_sequence_artifact, render_svg};
pub use specialized::{
    render_archimate_artifact, render_archimate_svg, render_chart_artifact, render_chart_svg,
    render_ditaa_artifact, render_ditaa_svg, render_ebnf_artifact, render_ebnf_svg,
    render_math_artifact, render_math_svg, render_nwdiag_artifact, render_nwdiag_svg,
    render_regex_artifact, render_regex_svg, render_sdl_artifact, render_sdl_svg,
};
pub use state::{render_state_artifact, render_state_svg};
pub use stdlib::{render_stdlib_artifact, render_stdlib_svg};
pub use text::{render_text_pages, TextOutputMode};
pub use timeline::{render_timeline_artifact, render_timeline_stub_svg, render_timeline_svg};
pub use timing::{render_timing_artifact, render_timing_svg};
pub use wire::{render_wire_artifact, render_wire_svg};

pub(crate) use geometry::compute_edge_anchors_for_direction;

pub(crate) use relation::render_relation_marker_defs;
pub(crate) use svg::{creole_text, escape_text, render_sprite_sheet, with_sprite_registry};
