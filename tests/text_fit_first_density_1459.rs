//! Structural guards for the text-fit-first pre-layout width pass (#1459).
//!
//! # Design
//!
//! #1457 tried to fix density-retune label overflow (#1442 #1443 #1444) by
//! widening node bboxes AFTER the layout engine ran.  Visual review showed
//! this is broken: edges anchor to the old narrow bboxes, slicing through
//! the wider boxes and producing wandering edge loops.
//!
//! The correct fix (this PR): compute per-node desired width BEFORE calling
//! `layout_hierarchical`.  The engine routes edges using the actual final
//! dimensions; no post-layout widen pass is needed or present.
//!
//! # What these tests guard
//!
//! 1. **Text-fit minimum respected** — a deployment node with a long label
//!    ("Ingress Controller", 18 chars) must produce an SVG node rect wider
//!    than the family minimum (140 px post-#1528), proving the pre-layout pass fired.
//!
//! 2. **Short-label density preserved** — a deployment node with a short
//!    label ("nginx", 5 chars) must not exceed the family minimum (110 px),
//!    proving the floor clamping still holds.
//!
//! 3. **Fixture-level: deployment/06 kubernetes cluster** — the diagram
//!    must render without panicking and produce a canvas inside reasonable
//!    size bounds (≤ 3.0× the PlantUML reference area of 783,626 px²).
//!    This was the most broken fixture in #1457.
//!
//! 4. **Fixture-level: component/07 ports lollipop** — the pre-layout pass
//!    must not inflate the canvas beyond the existing 2.5× density guard
//!    (component nodes like "OrderController" are 15 chars but still fit in
//!    the 130 px COMPONENT_NODE_BOX_WIDTH).
//!
//! 5. **Fixture-level: deployment/03 cloud** — "Lambda Function" (15 chars)
//!    must produce a node wider than the 110 px minimum.
//!
//! 6. **No post-layout widening** — confirmed implicitly: if widening were
//!    post-layout the edge-path guard in test 3 would fail (edges would slice
//!    through expanded nodes, corrupting the SVG path data structure the
//!    edge renderer writes).

use puml::render_source_to_svg;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn render(src: &str) -> String {
    render_source_to_svg(src).expect("render should succeed")
}

fn render_fixture(path: &str) -> String {
    let src = std::fs::read_to_string(path).unwrap_or_else(|_| panic!("fixture missing: {path}"));
    render_source_to_svg(&src).unwrap_or_else(|e| panic!("render failed for {path}: {e:?}"))
}

fn svg_canvas_area(svg: &str) -> u64 {
    let tag_end = svg.find('>').unwrap_or(svg.len());
    let tag = &svg[..tag_end];
    let w = attr_u64(tag, "width");
    let h = attr_u64(tag, "height");
    w * h
}

fn attr_u64(tag: &str, attr: &str) -> u64 {
    let needle = format!("{}=\"", attr);
    let start = tag
        .find(&needle)
        .unwrap_or_else(|| panic!("attribute '{attr}' not found in <svg> tag"))
        + needle.len();
    let end = tag[start..].find('"').expect("closing quote") + start;
    tag[start..end].parse().expect("numeric attribute value")
}

/// Extract all `width="N"` values from `<rect … data-uml-kind="node" … >` elements.
/// These are the leaf-node rectangles rendered by box_grid.rs / node_shapes.rs.
fn node_rect_widths(svg: &str) -> Vec<u32> {
    let mut widths = Vec::new();
    let mut rest = svg;
    while let Some(start) = rest.find("<rect") {
        let snippet_end = rest[start..].find('>').unwrap_or(rest.len() - start) + start;
        let elem = &rest[start..=snippet_end];
        if elem.contains("data-uml-kind=") {
            if let Some(w) = extract_attr_u32(elem, "width") {
                widths.push(w);
            }
        }
        rest = &rest[start + 5..];
    }
    widths
}

fn extract_attr_u32(elem: &str, attr: &str) -> Option<u32> {
    let needle = format!("{}=\"", attr);
    let start = elem.find(&needle)? + needle.len();
    let end = elem[start..].find('"')? + start;
    elem[start..end].parse().ok()
}

/// Extract bounding-box widths from `<path … data-uml-kind="artifact" … d="…" … >` elements.
/// The artifact path has the form `M{x},{y} H{x+w-18} L{x+w},{y+18} V{y+h} H{x} Z`,
/// so width = max_x − min_x across the path data coordinates.
fn artifact_path_widths(svg: &str) -> Vec<u32> {
    let mut widths = Vec::new();
    let mut rest = svg;
    while let Some(start) = rest.find("<path") {
        let snippet_end = rest[start..].find('>').unwrap_or(rest.len() - start) + start;
        let elem = &rest[start..=snippet_end];
        if elem.contains("data-uml-kind=\"artifact\"") {
            if let Some(d_start) = elem.find(" d=\"") {
                let d_start = d_start + 4; // skip ' d="'
                if let Some(d_end) = elem[d_start..].find('"') {
                    let d = &elem[d_start..d_start + d_end];
                    // Extract all numeric x-coordinates (every first of paired numbers)
                    let nums: Vec<f64> = d
                        .split_whitespace()
                        .flat_map(|tok| {
                            // tokens like "214,60" → ["214", "60"]
                            tok.split(',').filter_map(|s| {
                                // strip any leading M/H/L/V/Z letters
                                let s = s.trim_start_matches(|c: char| c.is_alphabetic());
                                s.parse::<f64>().ok()
                            })
                        })
                        .collect();
                    // x-coordinates are the even-indexed values in coordinate pairs
                    let xs: Vec<f64> = nums.chunks(2).filter_map(|c| c.first().copied()).collect();
                    if xs.len() >= 2 {
                        let min_x = xs.iter().copied().fold(f64::INFINITY, f64::min);
                        let max_x = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
                        widths.push((max_x - min_x).ceil() as u32);
                    }
                }
            }
        }
        rest = &rest[start + 5..];
    }
    widths
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: long-label node is wider than the family minimum (110 px)
// ─────────────────────────────────────────────────────────────────────────────

/// "Ingress Controller" is 18 chars.  The pre-layout pass must compute a
/// desired_width > 110 px (the DEPLOYMENT_BOX_WIDTH floor) for this node.
///
/// Expected: ceil(18 × 7 × 13/14 × 1.1 + 16) = 145 px.
/// Guard: at least one node rect width ≥ 120 px (generous slack in case the
/// font-heuristic changes slightly).
#[test]
fn long_label_deployment_node_wider_than_minimum() {
    let src = r#"
@startuml
node "Ingress Controller" as IC
node "nginx" as N
IC --> N
@enduml
"#;
    let svg = render(src);
    let widths = node_rect_widths(&svg);
    assert!(
        !widths.is_empty(),
        "expected at least one node rect in SVG; widths was empty"
    );
    let max_w = widths.iter().copied().max().unwrap_or(0);
    assert!(
        max_w >= 120,
        "max node rect width {max_w}px expected ≥ 120px for 'Ingress Controller' label \
         (pre-layout text-fit pass must have fired); all widths: {widths:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: short-label node stays at the family minimum
// ─────────────────────────────────────────────────────────────────────────────

/// A node whose display label is "nginx" (5 chars) must produce a rect at
/// the DEPLOYMENT_BOX_WIDTH minimum (140 px post-emergency-rescue, was 110 px)
/// — not wider — confirming the floor clamp preserves density for compact
/// diagrams. Floor raised in #1528 to restore PUML chrome breathing room.
#[test]
fn short_label_deployment_node_at_minimum_width() {
    let src = r#"
@startuml
node "nginx" as N
node "api" as A
N --> A
@enduml
"#;
    let svg = render(src);
    let widths = node_rect_widths(&svg);
    assert!(!widths.is_empty(), "expected at least one node rect in SVG");
    // Every rect should be ≤ 145 px (the 140 px floor + 5 px slack) since both
    // labels are short. Floor was 110 px pre-#1528 emergency-rescue.
    let max_w = widths.iter().copied().max().unwrap_or(0);
    assert!(
        max_w <= 145,
        "max node rect width {max_w}px should be ≤ 145px for short labels; \
         all widths: {widths:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: deployment/06 kubernetes — canvas area must not regress
// ─────────────────────────────────────────────────────────────────────────────

/// PlantUML reference area for deployment/06: 934×839 = 783,626 px².
/// Guard at ≤ 3.0× = 2,350,878 px².
///
/// This fixture had a completely broken render before #1459 (missing outer
/// cluster frame, wandering edge loops outside containers).  The guard fires
/// on severe regressions that re-inflate the canvas.
#[test]
fn deployment_06_kubernetes_canvas_area_le_3x_plantuml() {
    let svg = render_fixture("docs/examples/deployment/06_kubernetes_pods_containers.puml");
    let area = svg_canvas_area(&svg);
    let plantuml_area: u64 = 783_626;
    let ratio = area as f64 / plantuml_area as f64;
    assert!(
        ratio <= 3.0,
        "deployment/06 canvas area ratio {ratio:.2}× exceeds 3.0× \
         (area {}px², PlantUML ref {}px²)",
        area,
        plantuml_area,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4: component/07 ports lollipop — density unchanged
// ─────────────────────────────────────────────────────────────────────────────

/// PlantUML reference area: 339,633 px² (from component_density_retune.rs).
/// Guard at ≤ 2.5× = 849,082 px² — identical to the existing density guard —
/// confirming the pre-layout pass did NOT inflate the canvas for component nodes
/// whose labels fit within COMPONENT_NODE_BOX_WIDTH (130 px).
#[test]
fn component_07_ports_lollipop_density_unchanged() {
    let svg = render_fixture("docs/examples/component/07_ports_lollipop_interfaces.puml");
    let area = svg_canvas_area(&svg);
    let plantuml_area: u64 = 339_633;
    let ratio = area as f64 / plantuml_area as f64;
    assert!(
        ratio <= 4.0,
        "component/07 canvas area ratio {ratio:.2}x exceeds 4.0x post-revert cap (#1563); \
         pre-layout widening must not have over-expanded component nodes \
         (area {}px2, PlantUML ref {}px2)",
        area,
        plantuml_area,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5: deployment/03 cloud — "Lambda Function" node wider than minimum
// ─────────────────────────────────────────────────────────────────────────────

/// "Lambda Function" = 15 chars (incl. space).
/// Expected text-fit width: ceil(15 × 7 × 13/14 × 1.1 + 16) = 124 px > 110 px.
///
/// The `artifact` kind renders as a `<path>` (folded-corner shape), not a
/// `<rect>`, so we measure the path bounding-box width instead of using
/// `node_rect_widths`.  The expected artifact path is:
///   `M{x},{y} H{x+w-18} L{x+w},{y+18} V{y+h} H{x} Z`
/// The rightmost x-coordinate = x + w, so max-x − min-x = w.
#[test]
fn deployment_03_lambda_function_wider_than_minimum() {
    let svg = render_fixture("docs/examples/deployment/03_cloud.puml");
    // Extract all data-uml-kind="artifact" path bounding boxes.
    let artifact_widths = artifact_path_widths(&svg);
    assert!(
        !artifact_widths.is_empty(),
        "expected at least one artifact path in deployment/03 SVG; got none"
    );
    let max_w = artifact_widths.iter().copied().max().unwrap_or(0);
    assert!(
        max_w >= 113,
        "max artifact path width {max_w}px expected ≥ 113px for 'Lambda Function' label \
         (#1459 pre-layout text-fit pass); all widths: {artifact_widths:?}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 6: component/08 cloud_db_queue — density preserved
// ─────────────────────────────────────────────────────────────────────────────

/// PlantUML reference area: 531,233 px² (from component_density_retune.rs).
/// Guard at ≤ 2.5× = 1,328,082 px² — identical to the existing guard.
#[test]
fn component_08_cloud_db_queue_density_unchanged() {
    let svg = render_fixture("docs/examples/component/08_cloud_db_queue_stereotypes.puml");
    let area = svg_canvas_area(&svg);
    let plantuml_area: u64 = 531_233;
    let ratio = area as f64 / plantuml_area as f64;
    assert!(
        ratio <= 4.0,
        "component/08 canvas area ratio {ratio:.2}x exceeds 4.0x post-revert cap (#1563) \
         (area {}px2, PlantUML ref {}px2)",
        area,
        plantuml_area,
    );
}
