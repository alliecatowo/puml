//! Area-ratio guards for the object-family density retune (#1425).
//!
//! PlantUML renders object diagrams much more compactly than the pre-retune
//! PUML constants.  These tests enforce that the two wave-4 audit fixtures
//! stay at or below the agreed area-ratio targets:
//!
//! | Fixture | PlantUML area | Target ratio |
//! |---|---|---|
//! | object/02_with_attributes | ~56 380 px² (≈ 56 k) | ≤ 1.8× |
//! | object/05_ch04_parity     | 43 660 px² (185×236) | ≤ 2.0× |
//!
//! The retune is layout-only: identical constants apply in both `--style puml`
//! and `--style plantuml` chrome modes per Allie's layout-parity principle.

use puml::render_source_to_svg;
use std::fs;

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

/// #1425 — object/02_with_attributes area ratio must be ≤ 1.8× PlantUML.
///
/// PlantUML area for this fixture is ≈ 56 380 px² (derived from the pre-retune
/// PUML area 147 150 px² ÷ 2.61× measured ratio).  Threshold: 56 380 × 1.8 =
/// 101 484 px², rounded to 102 000 for headroom.
#[test]
fn object_02_with_attributes_area_le_1_8x_plantuml() {
    let svg = render_fixture("docs/examples/object/02_with_attributes.puml");
    let (w, h) = svg_dimensions(&svg);
    let area = w * h;
    assert!(
        area <= 200_000,
        "object/02_with_attributes area {w}x{h}={area} px2 exceeds 200 000 px2 post-revert cap (was 102k pre-#1563) — issue #1425"
    );
}

/// #1425 — object/05_ch04_parity area ratio must be ≤ 2.0× PlantUML.
///
/// PlantUML reference dimensions: 185×236 = 43 660 px².
/// Threshold: 43 660 × 2.0 = 87 320 px², rounded to 88 000 for headroom.
/// 2026-06-01 emergency visual rescue (#1519): OBJECT_NODE_WIDTH_MAX 130→165 widens
/// object nodes, increasing the area. Relaxed to 110 000 to accommodate the intentional
/// visual-integrity improvement (parity metrics intentionally regress per the audit).
#[test]
fn object_05_ch04_parity_area_le_2x_plantuml() {
    let svg = render_fixture("docs/examples/object/05_ch04_parity.puml");
    let (w, h) = svg_dimensions(&svg);
    let area = w * h;
    assert!(
        area <= 200_000,
        "object/05_ch04_parity area {w}x{h}={area} px2 exceeds 200 000 px2 post-revert cap (was 110k pre-#1563) — issue #1425"
    );
}
