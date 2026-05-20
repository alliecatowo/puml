pub(super) use super::geometry::{compute_edge_anchors_for_direction, pick_port};
pub(super) use super::relation::{
    normalize_relation_endpoints, render_relation_marker_defs, usecase_dependency_label,
};
pub(super) use super::scene_graph::{estimate_text_bbox, Rect as SceneRect};
pub(super) use super::svg::{escape_text, render_actor_stick_figure};
pub(super) use crate::ast::{DiagramKind, MemberModifier};
pub(super) use crate::model::{
    FamilyDocument, FamilyGroup, FamilyNode, FamilyNodeKind, FamilyOrientation, FamilyStyle,
};
pub(super) use crate::theme::{ClassStyle, ComponentStyle};

mod box_grid;
mod box_relations;
mod class;
mod class_layout;
mod class_relations;
mod common;
mod members;
mod nodes;
mod shapes;
mod tree;

pub use box_grid::{render_component_svg, render_deployment_svg};
pub use class::{render_class_svg, render_family_stub_svg};
pub use tree::render_family_tree_svg;

pub(crate) use members::family_node_label;
pub(crate) use shapes::render_note_card;

use box_grid::{count_polyline_collisions, segment_intersects_rect};
use box_relations::render_box_grid_relations_and_labels;
use class_layout::*;
use class_relations::*;
use common::*;
use members::*;
use nodes::*;
use shapes::*;
use tree::{
    extract_projection_tree_rows, family_projection_extra_height, render_family_projection_boxes,
};
