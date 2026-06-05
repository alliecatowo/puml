//! Phase B structural tests for per-mode density constants (#1515 + #1516).
//!
//! These tests assert the concrete per-mode constant values and verify that
//! `layout_density()` correctly routes to the PUML-mode and PlantUML-mode
//! constant sets introduced in Phase B.
//!
//! #1515 covers: object family + class family
//! #1516 covers: component family + deployment family

use puml::render::layout_constants::{
    layout_density, LayoutDensity, PLANTUML_MODE_CLASS_BOX_MIN_WIDTH, PLANTUML_MODE_CLASS_COL_GAP,
    PLANTUML_MODE_CLASS_MARGIN_X, PLANTUML_MODE_CLASS_ROW_GAP,
    PLANTUML_MODE_COMPONENT_NODE_BOX_HEIGHT, PLANTUML_MODE_COMPONENT_NODE_BOX_WIDTH,
    PLANTUML_MODE_COMPONENT_RANK_EXTRA_GAP, PLANTUML_MODE_DEPLOYMENT_BOX_HEIGHT,
    PLANTUML_MODE_DEPLOYMENT_BOX_WIDTH, PLANTUML_MODE_DEPLOYMENT_RANK_EXTRA_GAP,
    PLANTUML_MODE_OBJECT_COL_GAP, PLANTUML_MODE_OBJECT_MARGIN_X,
    PLANTUML_MODE_OBJECT_NODE_WIDTH_MAX, PLANTUML_MODE_OBJECT_ROW_GAP, PLANTUML_MODE_PKG_INNER_GAP,
    PLANTUML_MODE_PKG_PADDING, PUML_MODE_CLASS_BOX_MIN_WIDTH, PUML_MODE_CLASS_COL_GAP,
    PUML_MODE_CLASS_MARGIN_X, PUML_MODE_CLASS_ROW_GAP, PUML_MODE_COMPONENT_NODE_BOX_HEIGHT,
    PUML_MODE_COMPONENT_NODE_BOX_WIDTH, PUML_MODE_COMPONENT_RANK_EXTRA_GAP,
    PUML_MODE_DEPLOYMENT_BOX_HEIGHT, PUML_MODE_DEPLOYMENT_BOX_WIDTH,
    PUML_MODE_DEPLOYMENT_RANK_EXTRA_GAP, PUML_MODE_OBJECT_COL_GAP, PUML_MODE_OBJECT_MARGIN_X,
    PUML_MODE_OBJECT_NODE_WIDTH_MAX, PUML_MODE_OBJECT_ROW_GAP, PUML_MODE_PKG_INNER_GAP,
    PUML_MODE_PKG_PADDING,
};
use puml::theme::StyleMode;

// ─── PUML-mode routing (#1515 object+class) ───────────────────────────────────

/// `layout_density(Puml)` routes to PUML_MODE_* constants for the class family.
#[test]
fn phase_b_puml_class_density_routes_to_puml_mode_constants() {
    let d: LayoutDensity = layout_density(StyleMode::Puml);
    assert_eq!(d.class_box_min_width, PUML_MODE_CLASS_BOX_MIN_WIDTH);
    assert_eq!(d.class_margin_x, PUML_MODE_CLASS_MARGIN_X);
    assert_eq!(d.class_col_gap, PUML_MODE_CLASS_COL_GAP);
    assert_eq!(d.class_row_gap, PUML_MODE_CLASS_ROW_GAP);
}

/// `layout_density(Puml)` routes to PUML_MODE_* constants for the object family.
#[test]
fn phase_b_puml_object_density_routes_to_puml_mode_constants() {
    let d: LayoutDensity = layout_density(StyleMode::Puml);
    assert_eq!(d.object_node_width_max, PUML_MODE_OBJECT_NODE_WIDTH_MAX);
    assert_eq!(d.object_col_gap, PUML_MODE_OBJECT_COL_GAP);
    assert_eq!(d.object_row_gap, PUML_MODE_OBJECT_ROW_GAP);
    assert_eq!(d.object_margin_x, PUML_MODE_OBJECT_MARGIN_X);
}

// ─── PlantUML-mode routing (#1515 object+class) ───────────────────────────────

/// `layout_density(Plantuml)` routes to PLANTUML_MODE_* constants for the class family.
#[test]
fn phase_b_plantuml_class_density_routes_to_plantuml_mode_constants() {
    let d: LayoutDensity = layout_density(StyleMode::Plantuml);
    assert_eq!(d.class_box_min_width, PLANTUML_MODE_CLASS_BOX_MIN_WIDTH);
    assert_eq!(d.class_margin_x, PLANTUML_MODE_CLASS_MARGIN_X);
    assert_eq!(d.class_col_gap, PLANTUML_MODE_CLASS_COL_GAP);
    assert_eq!(d.class_row_gap, PLANTUML_MODE_CLASS_ROW_GAP);
}

/// `layout_density(Plantuml)` routes to PLANTUML_MODE_* constants for the object family.
#[test]
fn phase_b_plantuml_object_density_routes_to_plantuml_mode_constants() {
    let d: LayoutDensity = layout_density(StyleMode::Plantuml);
    assert_eq!(d.object_node_width_max, PLANTUML_MODE_OBJECT_NODE_WIDTH_MAX);
    assert_eq!(d.object_col_gap, PLANTUML_MODE_OBJECT_COL_GAP);
    assert_eq!(d.object_row_gap, PLANTUML_MODE_OBJECT_ROW_GAP);
    assert_eq!(d.object_margin_x, PLANTUML_MODE_OBJECT_MARGIN_X);
}

// ─── PUML-mode routing (#1516 component+deployment) ──────────────────────────

/// `layout_density(Puml)` routes to PUML_MODE_* constants for the component family.
#[test]
fn phase_b_puml_component_density_routes_to_puml_mode_constants() {
    let d: LayoutDensity = layout_density(StyleMode::Puml);
    assert_eq!(
        d.component_node_box_width,
        PUML_MODE_COMPONENT_NODE_BOX_WIDTH
    );
    assert_eq!(
        d.component_node_box_height,
        PUML_MODE_COMPONENT_NODE_BOX_HEIGHT
    );
    assert_eq!(
        d.component_rank_extra_gap,
        PUML_MODE_COMPONENT_RANK_EXTRA_GAP
    );
    assert_eq!(d.pkg_inner_gap, PUML_MODE_PKG_INNER_GAP);
    assert_eq!(d.pkg_padding, PUML_MODE_PKG_PADDING);
}

/// `layout_density(Puml)` routes to PUML_MODE_* constants for the deployment family.
#[test]
fn phase_b_puml_deployment_density_routes_to_puml_mode_constants() {
    let d: LayoutDensity = layout_density(StyleMode::Puml);
    assert_eq!(d.deployment_box_width, PUML_MODE_DEPLOYMENT_BOX_WIDTH);
    assert_eq!(d.deployment_box_height, PUML_MODE_DEPLOYMENT_BOX_HEIGHT);
    assert_eq!(
        d.deployment_rank_extra_gap,
        PUML_MODE_DEPLOYMENT_RANK_EXTRA_GAP
    );
}

// ─── PlantUML-mode routing (#1516 component+deployment) ──────────────────────

/// `layout_density(Plantuml)` routes to PLANTUML_MODE_* constants for the component family.
#[test]
fn phase_b_plantuml_component_density_routes_to_plantuml_mode_constants() {
    let d: LayoutDensity = layout_density(StyleMode::Plantuml);
    assert_eq!(
        d.component_node_box_width,
        PLANTUML_MODE_COMPONENT_NODE_BOX_WIDTH
    );
    assert_eq!(
        d.component_node_box_height,
        PLANTUML_MODE_COMPONENT_NODE_BOX_HEIGHT
    );
    assert_eq!(
        d.component_rank_extra_gap,
        PLANTUML_MODE_COMPONENT_RANK_EXTRA_GAP
    );
    assert_eq!(d.pkg_inner_gap, PLANTUML_MODE_PKG_INNER_GAP);
    assert_eq!(d.pkg_padding, PLANTUML_MODE_PKG_PADDING);
}

/// `layout_density(Plantuml)` routes to PLANTUML_MODE_* constants for the deployment family.
#[test]
fn phase_b_plantuml_deployment_density_routes_to_plantuml_mode_constants() {
    let d: LayoutDensity = layout_density(StyleMode::Plantuml);
    assert_eq!(d.deployment_box_width, PLANTUML_MODE_DEPLOYMENT_BOX_WIDTH);
    assert_eq!(d.deployment_box_height, PLANTUML_MODE_DEPLOYMENT_BOX_HEIGHT);
    assert_eq!(
        d.deployment_rank_extra_gap,
        PLANTUML_MODE_DEPLOYMENT_RANK_EXTRA_GAP
    );
}

// ─── Concrete value spot-checks (regression guards) ──────────────────────────

/// PUML-mode class constants are at the looser pre-#1346 values.
#[test]
fn phase_b_puml_class_concrete_values() {
    assert_eq!(PUML_MODE_CLASS_BOX_MIN_WIDTH, 150);
    assert_eq!(PUML_MODE_CLASS_MARGIN_X, 16);
    assert_eq!(PUML_MODE_CLASS_COL_GAP, 60);
    assert_eq!(PUML_MODE_CLASS_ROW_GAP, 44);
}

/// PlantUML-mode class constants are at the tighter post-#1346 parity values.
#[test]
fn phase_b_plantuml_class_concrete_values() {
    assert_eq!(PLANTUML_MODE_CLASS_BOX_MIN_WIDTH, 120);
    assert_eq!(PLANTUML_MODE_CLASS_MARGIN_X, 8);
    assert_eq!(PLANTUML_MODE_CLASS_COL_GAP, 40);
    assert_eq!(PLANTUML_MODE_CLASS_ROW_GAP, 30);
}

/// PUML-mode object constants are at the looser pre-#1346 values.
#[test]
fn phase_b_puml_object_concrete_values() {
    assert_eq!(PUML_MODE_OBJECT_NODE_WIDTH_MAX, 165);
    assert_eq!(PUML_MODE_OBJECT_COL_GAP, 40);
    assert_eq!(PUML_MODE_OBJECT_ROW_GAP, 36);
    assert_eq!(PUML_MODE_OBJECT_MARGIN_X, 16);
}

/// PlantUML-mode object constants are at the tighter post-#1346 parity values.
#[test]
fn phase_b_plantuml_object_concrete_values() {
    assert_eq!(PLANTUML_MODE_OBJECT_NODE_WIDTH_MAX, 130);
    assert_eq!(PLANTUML_MODE_OBJECT_COL_GAP, 20);
    assert_eq!(PLANTUML_MODE_OBJECT_ROW_GAP, 20);
    assert_eq!(PLANTUML_MODE_OBJECT_MARGIN_X, 8);
}

/// PUML-mode component constants are at the looser values.
#[test]
fn phase_b_puml_component_concrete_values() {
    assert_eq!(PUML_MODE_COMPONENT_NODE_BOX_WIDTH, 165);
    assert_eq!(PUML_MODE_COMPONENT_NODE_BOX_HEIGHT, 60);
    assert_eq!(PUML_MODE_COMPONENT_RANK_EXTRA_GAP, 20.0_f64);
    assert_eq!(PUML_MODE_PKG_INNER_GAP, 40);
    assert_eq!(PUML_MODE_PKG_PADDING, 24);
}

/// PlantUML-mode component constants are at the tighter parity values.
#[test]
fn phase_b_plantuml_component_concrete_values() {
    assert_eq!(PLANTUML_MODE_COMPONENT_NODE_BOX_WIDTH, 130);
    assert_eq!(PLANTUML_MODE_COMPONENT_NODE_BOX_HEIGHT, 50);
    assert_eq!(PLANTUML_MODE_COMPONENT_RANK_EXTRA_GAP, 8.0_f64);
    assert_eq!(PLANTUML_MODE_PKG_INNER_GAP, 20);
    assert_eq!(PLANTUML_MODE_PKG_PADDING, 12);
}

/// PUML-mode deployment constants are at the looser values.
#[test]
fn phase_b_puml_deployment_concrete_values() {
    assert_eq!(PUML_MODE_DEPLOYMENT_BOX_WIDTH, 140);
    assert_eq!(PUML_MODE_DEPLOYMENT_BOX_HEIGHT, 56);
    assert_eq!(PUML_MODE_DEPLOYMENT_RANK_EXTRA_GAP, 30.0_f64);
}

/// PlantUML-mode deployment constants are at the tighter parity values.
#[test]
fn phase_b_plantuml_deployment_concrete_values() {
    assert_eq!(PLANTUML_MODE_DEPLOYMENT_BOX_WIDTH, 110);
    assert_eq!(PLANTUML_MODE_DEPLOYMENT_BOX_HEIGHT, 44);
    assert_eq!(PLANTUML_MODE_DEPLOYMENT_RANK_EXTRA_GAP, 16.0_f64);
}
