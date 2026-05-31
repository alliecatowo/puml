//! Structural regression tests for #1374:
//! - Parallel edges that share the same label must not be collapsed (Bug 1)
//! - Package frame header text must not be obscured by edge label backgrounds (Bug 2)

fn render_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("component diagram should render without error")
}

/// Bug 1: Two `publish events` edges (SA→KF and SB→KF) must both appear as
/// distinct SVG relation elements. Before #1374 they were collapsed into one
/// because both arrived at the same (x2, y2) port coordinate on Kafka.
#[test]
fn test_parallel_edges_both_rendered() {
    let puml = r#"@startuml
package "Event Bus" {
  component "Kafka" as KF
}
package "API Cluster" {
  component "Service A" as SA
  component "Service B" as SB
}
SA --> KF : publish events
SB --> KF : publish events
@enduml"#;

    let svg = render_svg(puml);

    // Both edges must be present as uml-relation elements targeting KF.
    let edges_to_kf = svg.matches("data-uml-to=\"KF\"").count();
    assert!(
        edges_to_kf >= 2,
        "Expected ≥2 edges targeting KF (SA→KF and SB→KF), found {}. \
         Parallel edges with the same label must not be collapsed (#1374).",
        edges_to_kf
    );

    // The two edges must arrive at DIFFERENT terminal coordinates (fan applied).
    // Fan direction depends on port orientation: left/right ports fan along y;
    // top/bottom ports fan along x. Either coordinate must differ.
    let terminals: Vec<(i64, i64)> = {
        let mut result = Vec::new();
        let mut search = svg.as_str();
        while let Some(rel_pos) = search.find("data-uml-to=\"KF\"") {
            let after = &search[rel_pos..];
            if let Some(pts_start) = after.find("points=\"") {
                let pts_str = &after[pts_start + 8..];
                if let Some(pts_end) = pts_str.find('"') {
                    let pts = &pts_str[..pts_end];
                    if let Some(last_pair) = pts.split_whitespace().last() {
                        let mut coords = last_pair.split(',');
                        if let (Some(xs), Some(ys)) = (coords.next(), coords.next()) {
                            if let (Ok(x), Ok(y)) = (xs.parse::<i64>(), ys.parse::<i64>()) {
                                result.push((x, y));
                            }
                        }
                    }
                }
            }
            search = &search[rel_pos + 1..];
        }
        result
    };
    assert_eq!(
        terminals.len(),
        2,
        "Expected 2 parsed KF terminal coordinates (one per parallel edge), got {:?}",
        terminals
    );
    assert!(
        terminals[0] != terminals[1],
        "SA→KF and SB→KF must arrive at distinct terminal (x,y) positions (port fan). \
         Both arrived at {:?} (#1374).",
        terminals[0]
    );
}

/// Bug 2: The package header text re-draw (dark band + white text) must appear
/// AFTER the last edge-label-bg rect in SVG document order, ensuring the dark
/// header band paints on top of any white edge-label backgrounds that route
/// through it. Before #1374 the header text was drawn first and then hidden.
#[test]
fn test_header_text_rendered_after_edge_label_backgrounds() {
    let puml = r#"@startuml
package "Pipeline Core" {
  component [Parser] as Parser
  component [Renderer] as Renderer
}
package "Shared Services" {
  component [Preprocessor] as Preproc
}
Preproc --> Parser
Parser --> Renderer
@enduml"#;

    let svg = render_svg(puml);

    // Find the last occurrence of the "package Pipeline Core" header text.
    let last_header_pos = svg
        .rfind("package Pipeline Core")
        .expect("'package Pipeline Core' must appear in SVG");

    // If there are edge label backgrounds, the last one must come BEFORE
    // the last header text occurrence.
    if let Some(last_bg_pos) = svg.rfind("uml-edge-label-bg") {
        assert!(
            last_header_pos > last_bg_pos,
            "Header text 'package Pipeline Core' (byte {}) must appear after \
             the last edge-label-bg rect (byte {}) so the dark band is repainted \
             on top and the text stays visible (#1374).",
            last_header_pos,
            last_bg_pos
        );
    }
}
