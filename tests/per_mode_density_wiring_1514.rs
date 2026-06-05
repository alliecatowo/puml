//! Phase A → Phase B transition tests for per-mode density wiring (#1514 → #1515 + #1516).
//!
//! Phase A (#1514) proved byte-identical output for both modes by keeping the
//! `layout_density()` arms equal.  Phase B (#1515 object+class, #1516
//! component+deployment) diverges the two arms so that:
//!
//! - `StyleMode::Puml`      → pre-#1346 looser values (PUML chrome breathing room)
//! - `StyleMode::Plantuml`  → post-#1346 tight parity values (upstream PlantUML match)
//!
//! The Phase A equality assertions are replaced here with Phase B directional
//! assertions (`Puml field >= Plantuml field`).  The structural test that every
//! field is read through `layout_density()` is preserved in
//! `tests/per_mode_density_phase_b_1515_1516.rs`.

use puml::render::layout_constants::{layout_density, LayoutDensity};
use puml::theme::StyleMode;

/// Phase B invariant: every `LayoutDensity` field returned for `StyleMode::Puml`
/// is greater than or equal to the corresponding field for `StyleMode::Plantuml`.
/// PUML mode carries richer chrome that needs more breathing room; PlantUML
/// mode stays tight for upstream parity.
#[test]
fn phase_b_puml_density_is_everywhere_ge_plantuml() {
    let p: LayoutDensity = layout_density(StyleMode::Puml);
    let l: LayoutDensity = layout_density(StyleMode::Plantuml);
    assert!(
        p.class_box_min_width >= l.class_box_min_width,
        "class_box_min_width: Puml {} < Plantuml {}",
        p.class_box_min_width,
        l.class_box_min_width
    );
    assert!(
        p.class_margin_x >= l.class_margin_x,
        "class_margin_x: Puml {} < Plantuml {}",
        p.class_margin_x,
        l.class_margin_x
    );
    assert!(
        p.class_col_gap >= l.class_col_gap,
        "class_col_gap: Puml {} < Plantuml {}",
        p.class_col_gap,
        l.class_col_gap
    );
    assert!(
        p.class_row_gap >= l.class_row_gap,
        "class_row_gap: Puml {} < Plantuml {}",
        p.class_row_gap,
        l.class_row_gap
    );
    assert!(
        p.object_node_width_max >= l.object_node_width_max,
        "object_node_width_max: Puml {} < Plantuml {}",
        p.object_node_width_max,
        l.object_node_width_max
    );
    assert!(
        p.object_col_gap >= l.object_col_gap,
        "object_col_gap: Puml {} < Plantuml {}",
        p.object_col_gap,
        l.object_col_gap
    );
    assert!(
        p.object_row_gap >= l.object_row_gap,
        "object_row_gap: Puml {} < Plantuml {}",
        p.object_row_gap,
        l.object_row_gap
    );
    assert!(
        p.object_margin_x >= l.object_margin_x,
        "object_margin_x: Puml {} < Plantuml {}",
        p.object_margin_x,
        l.object_margin_x
    );
    assert!(
        p.component_node_box_width >= l.component_node_box_width,
        "component_node_box_width: Puml {} < Plantuml {}",
        p.component_node_box_width,
        l.component_node_box_width
    );
    assert!(
        p.component_node_box_height >= l.component_node_box_height,
        "component_node_box_height: Puml {} < Plantuml {}",
        p.component_node_box_height,
        l.component_node_box_height
    );
    assert!(
        p.component_rank_extra_gap >= l.component_rank_extra_gap,
        "component_rank_extra_gap: Puml {} < Plantuml {}",
        p.component_rank_extra_gap,
        l.component_rank_extra_gap
    );
    assert!(
        p.deployment_box_width >= l.deployment_box_width,
        "deployment_box_width: Puml {} < Plantuml {}",
        p.deployment_box_width,
        l.deployment_box_width
    );
    assert!(
        p.deployment_box_height >= l.deployment_box_height,
        "deployment_box_height: Puml {} < Plantuml {}",
        p.deployment_box_height,
        l.deployment_box_height
    );
    assert!(
        p.deployment_rank_extra_gap >= l.deployment_rank_extra_gap,
        "deployment_rank_extra_gap: Puml {} < Plantuml {}",
        p.deployment_rank_extra_gap,
        l.deployment_rank_extra_gap
    );
    assert!(
        p.pkg_inner_gap >= l.pkg_inner_gap,
        "pkg_inner_gap: Puml {} < Plantuml {}",
        p.pkg_inner_gap,
        l.pkg_inner_gap
    );
    assert!(
        p.pkg_padding >= l.pkg_padding,
        "pkg_padding: Puml {} < Plantuml {}",
        p.pkg_padding,
        l.pkg_padding
    );
}

/// Sanity check: both modes differ on at least one field — verifies the
/// divergence actually happened (i.e., we are not back in Phase A equality).
#[test]
fn phase_b_modes_actually_differ() {
    let p: LayoutDensity = layout_density(StyleMode::Puml);
    let l: LayoutDensity = layout_density(StyleMode::Plantuml);
    assert_ne!(
        p, l,
        "Phase B modes must differ: layout_density(Puml) == layout_density(Plantuml). \
         If both arms are equal, the Phase B constant divergence was not applied."
    );
}
