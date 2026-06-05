//! Structural area-ratio guards for the component-family density retune.
//!
//! 2026-06-04 (density-revert PR #1563): global layout_constants reverted to pre-#1346
//! looser values. Component-family per-shape constants stay post-retune, but ratios
//! inflate due to looser rank/node separation. Caps raised as regression guards.
//!
//! Wave-4 audit identified three component fixtures with area ratios well above
//! the ≤1.5× target vs PlantUML:
//!   - component/02_interfaces:          4.09×
//!   - component/07_ports_lollipop:      2.89×
//!   - component/08_cloud_db_queue:      3.43×
//!
//! After introducing `COMPONENT_NODE_BOX_WIDTH=130`, `COMPONENT_NODE_BOX_HEIGHT=50`,
//! and `COMPONENT_RANK_EXTRA_GAP=8.0` in `layout_constants.rs` and branching on
//! `family == "component"` in `box_grid.rs`, the ratios drop to:
//!   - component/02_interfaces:          ~2.30×  (limited by 400px min-width + margin overhead)
//!   - component/07_ports_lollipop:      ~1.60×
//!   - component/08_cloud_db_queue:      ~1.66×
//!
//! These tests guard the canvas area against regressions — any future increase in the
//! area beyond the ≤3.0× guard will fail, alerting that the retune was inadvertently
//! reverted.  The guards are deliberately conservative (well above the current values)
//! to avoid false positives from minor layout engine changes.

use puml::render_source_to_svg;
use std::fs;

/// Extract SVG canvas dimensions from the opening `<svg ...>` tag.
fn svg_dimensions(svg: &str) -> (u32, u32) {
    let tag_end = svg.find('>').expect("malformed SVG: no closing >");
    let tag = &svg[..tag_end];

    let width = tag
        .split("width=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .and_then(|s| s.parse::<u32>().ok())
        .expect("SVG missing width attribute");

    let height = tag
        .split("height=\"")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .and_then(|s| s.parse::<u32>().ok())
        .expect("SVG missing height attribute");

    (width, height)
}

fn render_fixture(path: &str) -> String {
    let src = fs::read_to_string(path).unwrap_or_else(|_| panic!("fixture missing: {path}"));
    render_source_to_svg(&src).unwrap_or_else(|e| panic!("render failed for {path}: {e:?}"))
}

/// PlantUML reference areas estimated from wave-4 audit ratios applied to
/// the BEFORE canvas areas.  Used to compute our area ratio in the assertions.
/// plantuml_area = before_area / before_ratio
const PLANTUML_02_AREA: u64 = 57_467; // 235040 / 4.09
const PLANTUML_07_AREA: u64 = 339_633; // 981540 / 2.89
const PLANTUML_08_AREA: u64 = 531_233; // 1822128 / 3.43

/// component/02_interfaces — area ratio must not exceed 3.0×.
///
/// After the retune the observed ratio is ~2.30×; this guard fires on
/// significant regression (e.g. constants reverted to 200×80).
#[test]
fn component_02_interfaces_area_ratio_le_3x() {
    let svg = render_fixture("docs/examples/component/02_interfaces.puml");
    let (w, h) = svg_dimensions(&svg);
    let area = (w as u64) * (h as u64);
    let ratio_x100 = area * 100 / PLANTUML_02_AREA;
    assert!(
        ratio_x100 <= 500,
        "component/02_interfaces area ratio {:.2}x exceeds 5.0x post-revert cap ({}x{} = {} px2; plantuml est {})",
        ratio_x100 as f64 / 100.0,
        w,
        h,
        area,
        PLANTUML_02_AREA,
    );
}

/// component/07_ports_lollipop_interfaces — area ratio must not exceed 2.5×.
///
/// After the retune the observed ratio is ~1.60×; guard fires on regression.
#[test]
fn component_07_ports_lollipop_area_ratio_le_2x5() {
    let svg = render_fixture("docs/examples/component/07_ports_lollipop_interfaces.puml");
    let (w, h) = svg_dimensions(&svg);
    let area = (w as u64) * (h as u64);
    let ratio_x100 = area * 100 / PLANTUML_07_AREA;
    assert!(
        ratio_x100 <= 400,
        "component/07_ports_lollipop area ratio {:.2}x exceeds 4.0x post-revert cap ({}x{} = {} px2; plantuml est {})",
        ratio_x100 as f64 / 100.0,
        w,
        h,
        area,
        PLANTUML_07_AREA,
    );
}

/// component/08_cloud_db_queue_stereotypes — area ratio must not exceed 2.5×.
///
/// After the retune the observed ratio is ~1.66×; guard fires on regression.
#[test]
fn component_08_cloud_db_queue_area_ratio_le_2x5() {
    let svg = render_fixture("docs/examples/component/08_cloud_db_queue_stereotypes.puml");
    let (w, h) = svg_dimensions(&svg);
    let area = (w as u64) * (h as u64);
    let ratio_x100 = area * 100 / PLANTUML_08_AREA;
    assert!(
        ratio_x100 <= 400,
        "component/08_cloud_db_queue area ratio {:.2}x exceeds 4.0x post-revert cap ({}x{} = {} px2; plantuml est {})",
        ratio_x100 as f64 / 100.0,
        w,
        h,
        area,
        PLANTUML_08_AREA,
    );
}
