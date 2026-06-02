//! Cross-family density sweep — wave-7 pass-2 area-ratio guards.
//!
//! After prior per-family density retunes (waves 4-6), 8 fixtures remained in
//! the 1.55–2.45× range.  This wave-7 pass-2 addresses the structural causes:
//!
//! | Root cause                         | Fix applied                                  |
//! |------------------------------------|----------------------------------------------|
//! | `max(400)` svg_width floor         | Lowered to `max(120)` in box_grid_canvas.rs  |
//! | `group_top_overhead` always 52px   | Conditional on `!doc.groups.is_empty()`     |
//! | `2×PKG_PADDING` in rank_sep always | Conditional on groups present                |
//! | `DEPLOYMENT_RANK_EXTRA_GAP` = 30   | Reduced to 16 (flat diagrams ~80px rank sep) |
//! | `CLASS_MARGIN_X` = 16              | Reduced to 8 (matches PlantUML ~4–8px gutter)|
//! | `CLASS_ROW_GAP` = 40               | Reduced to 30 (PlantUML ~25–30px inter-rank) |
//! | `CLASS_BOX_MIN_WIDTH` = 130        | Reduced to 120 (PlantUML ~110–125px boxes)   |
//!
//! Measured before/after ratios (PlantUML reference from wave-4/5 forensics):
//!
//! | Fixture            | Before | After  | PlantUML ref    |
//! |--------------------|--------|--------|-----------------|
//! | class/01_basic     | 1.82×  | ~1.59× | 134×276=36984   |
//! | class/03_comp      | 1.58×  | ~1.41× | 148×384=56832   |
//! | class/05_vis       | 1.61×  | ~1.53× | 259×198=51282   |
//! | class/11_generics  | 1.63×  | ~1.53× | 361×316=114076  |
//! | component/02       | 2.30×  | ~1.15× | 280×205=57400   |
//! | component/08       | 1.67×  | ~1.67× | 660×803=529980  |
//! | deployment/02      | 2.43×  | ~1.33× | 254×322=81788   |
//! | deployment/03      | 1.95×  | ~0.95× | 344×199=68456   |
//!
//! Guards are set at 1.75× for class fixtures, 1.5× for component/02 (which
//! has grouped-diagram neighbours that must remain unaffected), and 1.5× for
//! deployment/02.  deployment/03 can legitimately go below 1.0× (PUML is
//! denser than PlantUML for that text-heavy fixture) so its lower-bound guard
//! ensures it doesn't shrink below 0.70× (text-overflow risk).
//!
//! Isolated guard: deployment/06 (grouped K8s fixture) must NOT shrink below
//! the pre-wave-7 baseline (1.16× → must remain ≥ 0.80× and ≤ 1.8×) to verify
//! the conditional group_top_overhead does not strip overhead for grouped
//! diagrams.

fn render(src: &str) -> String {
    puml::render_source_to_svg(src).expect("render should succeed")
}

fn svg_area(svg: &str) -> u64 {
    let tag_end = svg.find('>').unwrap_or(svg.len());
    let tag = &svg[..tag_end];
    let w = attr_u64(tag, "width");
    let h = attr_u64(tag, "height");
    w * h
}

fn attr_u64(tag: &str, attr: &str) -> u64 {
    let needle = format!("{attr}=\"");
    let start = tag
        .find(&needle)
        .unwrap_or_else(|| panic!("attribute '{attr}' not found in <svg> tag"))
        + needle.len();
    let end = tag[start..]
        .find('"')
        .unwrap_or_else(|| panic!("closing '\"' not found after '{attr}'"))
        + start;
    tag[start..end].parse::<u64>().unwrap_or_else(|_| {
        panic!(
            "attribute '{attr}' value '{}' is not a u64",
            &tag[start..end]
        )
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Class family pass-2 guards (≤1.75× after wave-7)
// ─────────────────────────────────────────────────────────────────────────────

/// class/01_basic: was 1.82×, wave-7 target ≤1.75×.
/// 2026-06-01 emergency visual rescue (#1519): CLASS_BOX_MIN_WIDTH 120→150 for
/// breathing room; threshold relaxed to ≤1.85× to accommodate the intentional
/// visual-integrity improvement (parity metrics intentionally regress).
#[test]
fn w7_class_01_area_ratio_le_1x75() {
    let src = include_str!("../docs/examples/class/01_basic.puml");
    let area = svg_area(&render(src));
    let pl_area: u64 = 134 * 276; // 36,984 px²
    let ratio_x100 = area * 100 / pl_area;
    assert!(
        ratio_x100 <= 185,
        "class/01 area ratio {:.2}× exceeds 1.85× emergency-rescue threshold (area={area}, pl={pl_area})",
        ratio_x100 as f64 / 100.0
    );
}

/// class/03_composition: was 1.58×, target ≤1.75×.
#[test]
fn w7_class_03_area_ratio_le_1x75() {
    let src = include_str!("../docs/examples/class/03_composition_aggregation.puml");
    let area = svg_area(&render(src));
    let pl_area: u64 = 148 * 384; // 56,832 px²
    let ratio_x100 = area * 100 / pl_area;
    assert!(
        ratio_x100 <= 175,
        "class/03 area ratio {:.2}× exceeds 1.75× wave-7 target (area={area}, pl={pl_area})",
        ratio_x100 as f64 / 100.0
    );
}

/// class/05_visibility: was 1.61×, target ≤1.75×.
#[test]
fn w7_class_05_area_ratio_le_1x75() {
    let src = include_str!("../docs/examples/class/05_visibility.puml");
    let area = svg_area(&render(src));
    let pl_area: u64 = 259 * 198; // 51,282 px²
    let ratio_x100 = area * 100 / pl_area;
    assert!(
        ratio_x100 <= 175,
        "class/05 area ratio {:.2}× exceeds 1.75× wave-7 target (area={area}, pl={pl_area})",
        ratio_x100 as f64 / 100.0
    );
}

/// class/11_generics: was 1.63×, target ≤1.75×.
#[test]
fn w7_class_11_area_ratio_le_1x75() {
    let src = include_str!("../docs/examples/class/11_generics.puml");
    let area = svg_area(&render(src));
    let pl_area: u64 = 361 * 316; // 114,076 px²
    let ratio_x100 = area * 100 / pl_area;
    assert!(
        ratio_x100 <= 175,
        "class/11 area ratio {:.2}× exceeds 1.75× wave-7 target (area={area}, pl={pl_area})",
        ratio_x100 as f64 / 100.0
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Component family pass-2 guards
// ─────────────────────────────────────────────────────────────────────────────

/// component/02_interfaces: was 2.30×, target ≤1.5× after removing max(400) floor
/// and conditional group_top_overhead.
#[test]
fn w7_component_02_area_ratio_le_1x5() {
    let src = include_str!("../docs/examples/component/02_interfaces.puml");
    let area = svg_area(&render(src));
    let pl_area: u64 = 280 * 205; // 57,400 px²
    let ratio_x100 = area * 100 / pl_area;
    assert!(
        ratio_x100 <= 150,
        "component/02 area ratio {:.2}× exceeds 1.5× wave-7 target (area={area}, pl={pl_area})",
        ratio_x100 as f64 / 100.0
    );
}

/// component/08 (grouped): was 1.67×, must remain ≤2.0× and unchanged by flat
/// diagram fixes (groups still get full pkg_pad + pkg_tab overhead).
#[test]
fn w7_component_08_grouped_not_regressed() {
    let src = include_str!("../docs/examples/component/08_cloud_db_queue_stereotypes.puml");
    let area = svg_area(&render(src));
    let pl_area: u64 = 660 * 803; // 529,980 px²
    let ratio_x100 = area * 100 / pl_area;
    assert!(
        ratio_x100 <= 200,
        "component/08 area ratio {:.2}× exceeds 2.0× (area={area}, pl={pl_area}) — \
         wave-7 flat fixes may have unexpectedly shrunk grouped component diagrams",
        ratio_x100 as f64 / 100.0
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Deployment family pass-2 guards
// ─────────────────────────────────────────────────────────────────────────────

/// deployment/02_databases: was 2.43×, wave-7 target ≤1.5×.
/// 2026-06-01 emergency visual rescue (#1519): DEPLOYMENT_BOX_WIDTH 110→140 for
/// breathing room; threshold relaxed to ≤1.65× to accommodate the intentional
/// visual-integrity improvement (parity metrics intentionally regress).
#[test]
fn w7_deployment_02_area_ratio_le_1x5() {
    let src = include_str!("../docs/examples/deployment/02_databases.puml");
    let area = svg_area(&render(src));
    let pl_area: u64 = 254 * 322; // 81,788 px²
    let ratio_x100 = area * 100 / pl_area;
    assert!(
        ratio_x100 <= 165,
        "deployment/02 area ratio {:.2}× exceeds 1.65× emergency-rescue threshold (area={area}, pl={pl_area})",
        ratio_x100 as f64 / 100.0
    );
}

/// deployment/03_cloud: ratio may legitimately dip below 1.0× for text-heavy
/// fixtures.  Guard: 0.70× ≤ ratio ≤ 1.5×.
#[test]
fn w7_deployment_03_area_ratio_in_range() {
    let src = include_str!("../docs/examples/deployment/03_cloud.puml");
    let area = svg_area(&render(src));
    let pl_area: u64 = 344 * 199; // 68,456 px²
    let ratio_x100 = area * 100 / pl_area;
    assert!(
        ratio_x100 >= 70,
        "deployment/03 area ratio {:.2}× is below 0.70× lower bound — text may overflow nodes \
         (area={area}, pl={pl_area})",
        ratio_x100 as f64 / 100.0
    );
    assert!(
        ratio_x100 <= 150,
        "deployment/03 area ratio {:.2}× exceeds 1.5× upper bound (area={area}, pl={pl_area})",
        ratio_x100 as f64 / 100.0
    );
}

/// deployment/06_kubernetes (deeply grouped): conditional group_top_overhead
/// must preserve full overhead for grouped diagrams.  Was 1.16×, must stay
/// between 0.80× and 1.8× (pre-wave-7 bounds).
#[test]
fn w7_deployment_06_grouped_not_regressed() {
    let src = include_str!("../docs/examples/deployment/06_kubernetes_pods_containers.puml");
    let area = svg_area(&render(src));
    let pl_area: u64 = 934 * 839; // 783,626 px²
    let ratio_x100 = area * 100 / pl_area;
    assert!(
        ratio_x100 >= 80,
        "deployment/06 area ratio {:.2}× is below 0.80× — grouped diagram may have lost \
         pkg-tab overhead (area={area}, pl={pl_area})",
        ratio_x100 as f64 / 100.0
    );
    assert!(
        ratio_x100 <= 180,
        "deployment/06 area ratio {:.2}× exceeds 1.80× — deployment-06 regressed \
         (area={area}, pl={pl_area})",
        ratio_x100 as f64 / 100.0
    );
}
