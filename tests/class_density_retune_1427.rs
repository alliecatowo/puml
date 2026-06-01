//! Structural density-ratio assertions for the class family per-family retune (#1427).
//!
//! These tests guard against regressions that would re-inflate class diagram canvas
//! sizes back toward the 2-4× PlantUML area ratios observed in the wave-4 audit.
//!
//! | Fixture                    | Pre-#1427 ratio | Target | Post-#1427 ratio |
//! |----------------------------|-----------------|--------|-----------------|
//! | class/01_basic             | 3.24×           | ≤2.0×  | 1.97×           |
//! | class/03_composition       | 2.99×           | ≤2.0×  | 1.68×           |
//! | class/05_visibility        | 1.85×           | ≤2.0×  | 1.77×           |
//! | class/11_generics          | 2.50×           | ≤2.0×  | 1.73×           |
//!
//! PlantUML reference dimensions (ground truth from the wave-4 forensic audit):
//!   - class/01_basic:               134×276 px → 36,984 px²
//!   - class/03_composition:         148×384 px → 56,832 px²
//!   - class/05_visibility:          259×198 px → 51,282 px²
//!   - class/11_generics:            361×316 px → 114,076 px²
//!
//! Object/UseCase diagrams share the class renderer but their density-retune
//! constants are gated on `DiagramKind::Class` only — object/02_with_attributes
//! and usecase/02_with_actors must remain unchanged from their wave-4 baselines.

fn render(src: &str) -> String {
    puml::render_source_to_svg(src).expect("render should succeed")
}

/// Extract the SVG canvas area (width * height) from the root `<svg ...>` tag.
fn svg_canvas_area(svg: &str) -> u64 {
    let w = extract_svg_attr(svg, "width");
    let h = extract_svg_attr(svg, "height");
    w * h
}

fn extract_svg_attr(svg: &str, attr: &str) -> u64 {
    let tag_end = svg.find('>').unwrap_or(svg.len());
    let tag = &svg[..tag_end];
    let needle = format!("{attr}=\"");
    let start = tag.find(&needle).unwrap_or_else(|| {
        panic!("attribute '{attr}' not found in <svg> tag: {}", &svg[..200])
    }) + needle.len();
    let end = tag[start..].find('"').unwrap_or_else(|| {
        panic!("closing '\"' not found after attribute '{attr}' value")
    }) + start;
    tag[start..end]
        .parse::<u64>()
        .unwrap_or_else(|_| panic!("attribute '{attr}' value '{}' is not a u64", &tag[start..end]))
}

// ─────────────────────────────────────────────────────────────────────────────
// class/01_basic (was 3.24×, target ≤2.0×)
// PlantUML reference: 134×276 = 36,984 px²
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn class_01_basic_area_ratio_within_target() {
    let src = include_str!("../docs/examples/class/01_basic.puml");
    let svg = render(src);
    let puml_area = svg_canvas_area(&svg);
    let plantuml_area: u64 = 134 * 276; // 36,984 px² (wave-4 reference)
    let ratio_x100 = (puml_area * 100) / plantuml_area;
    assert!(
        ratio_x100 <= 200,
        "class/01_basic area ratio {}.{:02}× exceeds 2.00× target (was 3.24× pre-#1427). \
         PUML area={puml_area}, PlantUML reference=36984 px²",
        ratio_x100 / 100,
        ratio_x100 % 100,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// class/03_composition_aggregation (was 2.99×, target ≤2.0×)
// PlantUML reference: 148×384 = 56,832 px²
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn class_03_composition_area_ratio_within_target() {
    let src = include_str!("../docs/examples/class/03_composition_aggregation.puml");
    let svg = render(src);
    let puml_area = svg_canvas_area(&svg);
    let plantuml_area: u64 = 148 * 384; // 56,832 px² (wave-4 reference)
    let ratio_x100 = (puml_area * 100) / plantuml_area;
    assert!(
        ratio_x100 <= 200,
        "class/03_composition area ratio {}.{:02}× exceeds 2.00× target (was 2.99× pre-#1427). \
         PUML area={puml_area}, PlantUML reference=56832 px²",
        ratio_x100 / 100,
        ratio_x100 % 100,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// class/05_visibility (was 1.85×, target ≤2.0×)
// PlantUML reference: 259×198 = 51,282 px²
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn class_05_visibility_area_ratio_within_target() {
    let src = include_str!("../docs/examples/class/05_visibility.puml");
    let svg = render(src);
    let puml_area = svg_canvas_area(&svg);
    let plantuml_area: u64 = 259 * 198; // 51,282 px² (wave-4 reference)
    let ratio_x100 = (puml_area * 100) / plantuml_area;
    assert!(
        ratio_x100 <= 200,
        "class/05_visibility area ratio {}.{:02}× exceeds 2.00× target (was 1.85× pre-#1427). \
         PUML area={puml_area}, PlantUML reference=51282 px²",
        ratio_x100 / 100,
        ratio_x100 % 100,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// class/11_generics (was 2.50×, target ≤2.0×)
// PlantUML reference: 361×316 = 114,076 px²
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn class_11_generics_area_ratio_within_target() {
    let src = include_str!("../docs/examples/class/11_generics.puml");
    let svg = render(src);
    let puml_area = svg_canvas_area(&svg);
    let plantuml_area: u64 = 361 * 316; // 114,076 px² (wave-4 reference)
    let ratio_x100 = (puml_area * 100) / plantuml_area;
    assert!(
        ratio_x100 <= 200,
        "class/11_generics area ratio {}.{:02}× exceeds 2.00× target (was 2.50× pre-#1427). \
         PUML area={puml_area}, PlantUML reference=114076 px²",
        ratio_x100 / 100,
        ratio_x100 % 100,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Isolation guard: object/02_with_attributes must remain at wave-4 baseline
//
// The class density-retune constants (#1427) are gated on DiagramKind::Class
// only.  This test ensures object diagrams (DiagramKind::Object, which also
// routes through the class renderer) are NOT affected by the retune.
// PlantUML reference: 223×253 = 56,419 px² (wave-4 object/02 baseline)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn object_02_unaffected_by_class_retune() {
    let src = include_str!("../docs/examples/object/02_with_attributes.puml");
    let svg = render(src);
    let puml_area = svg_canvas_area(&svg);
    // Object/02 wave-4 baseline: 327×450 = 147,150 px²
    // Ensure it is NOT smaller than 0.8× of the wave-4 baseline (regression guard
    // that it wasn't accidentally retune'd smaller).
    let baseline_area: u64 = 327 * 450;
    assert!(
        puml_area >= baseline_area * 80 / 100,
        "object/02 area={puml_area} is more than 20% smaller than wave-4 baseline={baseline_area} — \
         class retune may have accidentally shrunk object diagrams"
    );
}
