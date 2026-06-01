//! Tests for issue #1427: usecase actor-to-ellipse edge fan separation.
//!
//! When an actor has multiple edges to use cases, the orthogonal router
//! assigns them to the same vertical stem (x-coordinate), creating a
//! visually tangled bundle.  The actor fan pass (#1427) spreads each
//! actor→usecase edge by 20 px in the horizontal direction so that no
//! two edges from the same actor share the same 20-pixel vertical band
//! along their entire length.
//!
//! **Fixture**: `usecase/05_actor_generalization_system_boundary` — User
//! actor connects to UC1, UC2, UC3; Registered User actor connects to UC4,
//! UC5, UC7, UC8.  After the fan, every edge from the same actor must have
//! a different starting x-coordinate (spaced 20 px apart).

use puml::{
    normalize_family, parse_with_pipeline_options, render_artifact_pages_from_model,
    ParsePipelineOptions,
};

fn render(src: &str) -> String {
    let opts = ParsePipelineOptions::default();
    let doc = parse_with_pipeline_options(src, &opts).expect("source should parse");
    let model = normalize_family(doc).expect("source should normalize");
    render_artifact_pages_from_model(&model)
        .into_iter()
        .next()
        .map(|a| a.svg)
        .unwrap_or_default()
}

/// Extract polyline start-x values for edges with the given `data-uml-from`
/// attribute.  The SVG is a single long line so we search for element
/// boundaries manually.
fn polyline_start_xs(svg: &str, from_attr: &str) -> Vec<i32> {
    let needle = format!("data-uml-from=\"{from_attr}\"");
    let mut xs = Vec::new();
    let mut pos = 0;
    while let Some(idx) = svg[pos..].find(&needle) {
        let abs = pos + idx;
        // Find the enclosing element (walk backward to '<')
        let elem_start = svg[..abs].rfind('<').unwrap_or(0);
        // Find element end '/>'
        let elem_end = svg[abs..].find("/>").map(|i| abs + i + 2).unwrap_or(abs);
        let elem = &svg[elem_start..elem_end];
        // Only consider polyline elements (those with points="")
        if elem.contains("points=\"") {
            if let Some(pts_idx) = elem.find("points=\"") {
                let rest = &elem[pts_idx + 8..];
                if let Some(comma) = rest.find(',') {
                    if let Ok(x) = rest[..comma].trim().parse::<i32>() {
                        xs.push(x);
                    }
                }
            }
        }
        pos = elem_end;
    }
    xs.sort_unstable();
    xs
}

/// Count `<line>` elements with both given from and to attributes.
fn count_line_elements(svg: &str, from_attr: &str, to_attr: &str) -> usize {
    let from_needle = format!("data-uml-from=\"{from_attr}\"");
    let to_needle = format!("data-uml-to=\"{to_attr}\"");
    let mut count = 0;
    let mut pos = 0;
    while let Some(idx) = svg[pos..].find(&from_needle) {
        let abs = pos + idx;
        let elem_start = svg[..abs].rfind('<').unwrap_or(0);
        let elem_end = svg[abs..].find("/>").map(|i| abs + i + 2).unwrap_or(abs);
        let elem = &svg[elem_start..elem_end];
        if elem.contains(&to_needle) && !elem.contains("points=\"") {
            count += 1;
        }
        pos = elem_end;
    }
    count
}

const FIXTURE: &str = include_str!(
    "../docs/examples/usecase/05_actor_generalization_system_boundary.puml"
);

#[test]
fn user_actor_usecase_edges_fanned_20px_apart() {
    let svg = render(FIXTURE);
    // User (alias U) has three edges to use cases (UC1, UC2, UC3).
    // After fanning, all three starting x values must differ by ≥ 20 px.
    let xs = polyline_start_xs(&svg, "U");
    assert_eq!(
        xs.len(),
        3,
        "expected 3 polyline edges from actor U, got {:?}",
        xs
    );
    for pair in xs.windows(2) {
        let gap = pair[1] - pair[0];
        assert!(
            gap >= 20,
            "actor U edges not fanned: consecutive x gap = {gap}px (need ≥ 20px), xs = {xs:?}"
        );
    }
}

#[test]
fn registered_user_usecase_edges_fanned_20px_apart() {
    let svg = render(FIXTURE);
    // Registered User (alias RU) has four edges to use cases (UC4, UC5, UC7, UC8).
    let xs = polyline_start_xs(&svg, "RU");
    assert_eq!(
        xs.len(),
        4,
        "expected 4 polyline edges from actor RU, got {:?}",
        xs
    );
    for pair in xs.windows(2) {
        let gap = pair[1] - pair[0];
        assert!(
            gap >= 20,
            "actor RU edges not fanned: consecutive x gap = {gap}px (need ≥ 20px), xs = {xs:?}"
        );
    }
}

#[test]
fn actor_generalization_edges_not_suppressed() {
    let svg = render(FIXTURE);
    // U→RU and U→Admin are actor-generalization edges (straight <line>).
    // They must still be present after the fan pass.
    let ru_count = count_line_elements(&svg, "U", "RU");
    let admin_count = count_line_elements(&svg, "U", "Admin");
    assert_eq!(
        ru_count, 1,
        "expected 1 generalization line U→RU, got {ru_count}"
    );
    assert_eq!(
        admin_count, 1,
        "expected 1 generalization line U→Admin, got {admin_count}"
    );
}
