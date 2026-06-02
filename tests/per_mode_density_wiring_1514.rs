//! Phase A invariant tests for the per-mode density wiring (#1514).
//!
//! This wiring PR introduces `LayoutDensity` + `layout_density(StyleMode)` as
//! the single chokepoint through which the box-grid (component/deployment)
//! and class/object renderers read their per-family density constants.  The
//! Phase A guarantee — proven byte-for-byte by the BEFORE/AFTER SVG diff in
//! the PR description — is that **both modes return the same `LayoutDensity`
//! values** so rendered output is identical to pre-PR.
//!
//! Phase B follow-ups (#1515 object+class, #1516 component+deployment) will
//! diverge the `StyleMode::Puml` branch of `layout_density()` to give PUML
//! chrome breathing room.  At that point these per-field equality assertions
//! flip to per-field "Puml >= Plantuml" assertions; the structural test
//! (every `LayoutDensity` field is read through `layout_density()`) stays.

use puml::render::layout_constants::{layout_density, LayoutDensity};
use puml::theme::StyleMode;

/// Phase A invariant: `layout_density()` returns identical values for both
/// style modes.  Any divergence introduced by #1515 or #1516 must remove this
/// test (or flip it to per-field directional assertions).
#[test]
fn phase_a_layout_density_is_mode_invariant() {
    let puml = layout_density(StyleMode::Puml);
    let plantuml = layout_density(StyleMode::Plantuml);
    assert_eq!(
        puml, plantuml,
        "Phase A invariant violated: layout_density(Puml) != layout_density(Plantuml). \
         If this test fails, you are landing #1515 or #1516 — remove this test (or \
         convert to per-field directional assertions) as part of that PR."
    );
}

/// Cross-check every field individually so a future divergence pinpoints
/// which constant changed.  Redundant with the struct-equality assertion
/// above, but the field-level error messages are easier to diff in a PR
/// review.
#[test]
fn phase_a_each_layout_density_field_matches_across_modes() {
    let p: LayoutDensity = layout_density(StyleMode::Puml);
    let l: LayoutDensity = layout_density(StyleMode::Plantuml);
    assert_eq!(p.class_box_min_width, l.class_box_min_width);
    assert_eq!(p.class_margin_x, l.class_margin_x);
    assert_eq!(p.class_col_gap, l.class_col_gap);
    assert_eq!(p.class_row_gap, l.class_row_gap);
    assert_eq!(p.object_node_width_max, l.object_node_width_max);
    assert_eq!(p.object_col_gap, l.object_col_gap);
    assert_eq!(p.object_row_gap, l.object_row_gap);
    assert_eq!(p.object_margin_x, l.object_margin_x);
    assert_eq!(p.component_node_box_width, l.component_node_box_width);
    assert_eq!(p.component_node_box_height, l.component_node_box_height);
    assert_eq!(p.component_rank_extra_gap, l.component_rank_extra_gap);
    assert_eq!(p.deployment_box_width, l.deployment_box_width);
    assert_eq!(p.deployment_box_height, l.deployment_box_height);
    assert_eq!(p.deployment_rank_extra_gap, l.deployment_rank_extra_gap);
    assert_eq!(p.pkg_inner_gap, l.pkg_inner_gap);
    assert_eq!(p.pkg_padding, l.pkg_padding);
}

/// Sanity check that the Phase A wiring still reflects the bare constants it
/// replaced.  This anchors `layout_density()` to the constants module so a
/// future renaming or accidental decoupling fails the test instead of
/// silently shipping different geometry.
#[test]
fn phase_a_layout_density_reflects_module_constants() {
    use puml::render::layout_constants::{
        CLASS_BOX_MIN_WIDTH, CLASS_COL_GAP, CLASS_MARGIN_X, CLASS_ROW_GAP,
        COMPONENT_NODE_BOX_HEIGHT, COMPONENT_NODE_BOX_WIDTH, COMPONENT_RANK_EXTRA_GAP,
        DEPLOYMENT_BOX_HEIGHT, DEPLOYMENT_BOX_WIDTH, DEPLOYMENT_RANK_EXTRA_GAP, OBJECT_COL_GAP,
        OBJECT_MARGIN_X, OBJECT_NODE_WIDTH_MAX, OBJECT_ROW_GAP, PKG_INNER_GAP, PKG_PADDING,
    };
    let d = layout_density(StyleMode::Puml);
    assert_eq!(d.class_box_min_width, CLASS_BOX_MIN_WIDTH);
    assert_eq!(d.class_margin_x, CLASS_MARGIN_X);
    assert_eq!(d.class_col_gap, CLASS_COL_GAP);
    assert_eq!(d.class_row_gap, CLASS_ROW_GAP);
    assert_eq!(d.object_node_width_max, OBJECT_NODE_WIDTH_MAX);
    assert_eq!(d.object_col_gap, OBJECT_COL_GAP);
    assert_eq!(d.object_row_gap, OBJECT_ROW_GAP);
    assert_eq!(d.object_margin_x, OBJECT_MARGIN_X);
    assert_eq!(d.component_node_box_width, COMPONENT_NODE_BOX_WIDTH);
    assert_eq!(d.component_node_box_height, COMPONENT_NODE_BOX_HEIGHT);
    assert_eq!(d.component_rank_extra_gap, COMPONENT_RANK_EXTRA_GAP);
    assert_eq!(d.deployment_box_width, DEPLOYMENT_BOX_WIDTH);
    assert_eq!(d.deployment_box_height, DEPLOYMENT_BOX_HEIGHT);
    assert_eq!(d.deployment_rank_extra_gap, DEPLOYMENT_RANK_EXTRA_GAP);
    assert_eq!(d.pkg_inner_gap, PKG_INNER_GAP);
    assert_eq!(d.pkg_padding, PKG_PADDING);
}
