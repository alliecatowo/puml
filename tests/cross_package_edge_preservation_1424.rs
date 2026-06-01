//! Regression test for #1424: cross-package edges must not be hidden by
//! the package header band repaint.
//!
//! Root cause: `render_box_grid_package_header_text` was repainting the full
//! dark header band (fill="#1e293b" opaque rects) AFTER edge polylines were
//! emitted, causing any edge segment that passes through the header zone to
//! be visually covered by the repainted band. The fix paints ONLY the white
//! label text at the end, leaving the dark band from the first pass underneath
//! the edges.

fn render_svg(src: &str) -> String {
    puml::render_source_to_svg(src).expect("component diagram should render without error")
}

/// All 10 declared relations in the cloud-DB-queue fixture must appear as
/// SVG `uml-relation` elements. Before #1424 the dark header repaint covered
/// edges whose routing channels passed through the header zone, making them
/// visually absent in the rendered PNG.
#[test]
fn test_all_cross_package_edges_rendered() {
    let puml = r#"@startuml
package "CDN Layer" {
  component "Edge Cache" as EC
}
package "API Cluster" {
  component "Load Balancer" as LB
  component "Service A" as SA
  component "Service B" as SB
}
package "Storage Layer" {
  component "Primary DB" as DB1
  component "Read Replica" as DB2
  component "Object Store" as S3
}
package "Event Bus" {
  component "Kafka" as KF
}
package "Analytics Platform" {
  component "Stream Processor" as SP
  component "Data Warehouse" as DW
}

EC --> LB : origin pull
LB --> SA : route /v1
LB --> SB : route /v2
SA --> DB1 : read/write
SA --> DB2 : read-only
SB --> S3 : upload
SA --> KF : publish events
SB --> KF : publish events
KF --> SP : consume
SP --> DW : aggregate
@enduml"#;

    let svg = render_svg(puml);

    let relation_count = svg.matches("class=\"uml-relation\"").count();
    assert_eq!(
        relation_count, 10,
        "Expected exactly 10 uml-relation elements (one per declared relation), \
         found {}. Cross-package edges must not be dropped or hidden (#1424).",
        relation_count
    );

    // Verify each specific cross-package edge is present.
    let cross_package_edges = [
        ("EC", "LB"),
        ("SA", "KF"),
        ("SB", "KF"),
        ("SB", "S3"),
        ("KF", "SP"),
    ];
    for (from, to) in &cross_package_edges {
        let needle = format!("data-uml-from=\"{from}\" data-uml-to=\"{to}\"");
        assert!(
            svg.contains(&needle),
            "Cross-package edge {from}→{to} must be present in SVG output (#1424). \
             It was not found.",
        );
    }
}

/// The dark header band repaint must NOT add opaque dark rectangles AFTER the
/// edge elements. Only the white label text should be re-emitted after edges
/// to keep header text readable over edge-label backgrounds. (#1424 / #1374)
#[test]
fn test_header_band_not_repainted_after_edges() {
    let puml = r#"@startuml
package "Left Package" {
  component "Node A" as A
}
package "Right Package" {
  component "Node B" as B
}
A --> B : cross-package
@enduml"#;

    let svg = render_svg(puml);

    // Find the position of the last uml-relation element.
    let last_relation_pos = svg
        .rfind("class=\"uml-relation\"")
        .expect("uml-relation must appear in SVG");

    // Count dark-fill rect elements that appear AFTER the last relation.
    // These would be the header-band repaint rects; there should be none.
    let after_last_rel = &svg[last_relation_pos..];
    let dark_rects_after: usize = {
        let mut count = 0;
        let mut search = after_last_rel;
        while let Some(pos) = search.find("<rect") {
            let rest = &search[pos..];
            if let Some(end) = rest.find("/>") {
                let elem = &rest[..end];
                if elem.contains("fill=\"#1e293b\"") {
                    count += 1;
                }
            }
            search = &search[pos + 1..];
        }
        count
    };

    assert_eq!(
        dark_rects_after, 0,
        "Expected 0 dark (#1e293b) rect elements after the last uml-relation, \
         found {}. The header band must not be repainted after edges to avoid \
         covering cross-package edge lines (#1424).",
        dark_rects_after
    );
}
