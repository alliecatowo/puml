//! Structural regression tests for the five router bugs fixed in wave-14:
//! #1323 (side-port exits), #1324 (multi-out spread), #1325 (group-frame
//! obstacles), #1326 (package header clearance), #1327 (package-framed
//! target anchor).
//!
//! Tests render PUML component/deployment diagrams and inspect the generated
//! SVG for structural correctness in edge geometry.

/// Count how many `uml-relation` elements are in the SVG output.
fn count_relations(svg: &str) -> usize {
    svg.matches("class=\"uml-relation\"").count()
}

/// Render a PUML source string to SVG, panicking on any error.
fn svg(puml: &str) -> String {
    puml::render_source_to_svg(puml).expect("render must succeed")
}

// ── #1323 ─────────────────────────────────────────────────────────────────────

/// #1323: A component diagram where A and B are at the same horizontal level
/// (both point to a common sink C) should produce the A→B edge.  The router
/// must not crash or drop edges when a side-port preference is applied.
#[test]
fn side_exit_routing_1323_does_not_drop_peer_edges() {
    let s = svg(r#"
@startuml
component [A] as A
component [B] as B
component [C] as C
A --> C
B --> C
A --> B
@enduml
"#);
    assert!(
        count_relations(&s) >= 3,
        "all 3 relations should be rendered (including peer A→B), got:\n{s}"
    );
}

// ── #1324 ─────────────────────────────────────────────────────────────────────

/// #1324: A hub node with 3 outgoing edges must produce 3 distinct `uml-relation`
/// elements.  Previously they stacked on the same port; the fix spreads them.
#[test]
fn multi_out_spread_1324_all_edges_rendered() {
    let s = svg(r#"
@startuml
component [Hub] as Hub
component [A] as A
component [B] as B
component [C] as C
Hub --> A
Hub --> B
Hub --> C
@enduml
"#);
    assert_eq!(
        count_relations(&s),
        3,
        "all 3 hub outgoing edges should be rendered, got:\n{s}"
    );
}

// ── #1325 ─────────────────────────────────────────────────────────────────────

/// #1325: An edge spanning two ranks that passes through a sibling package frame
/// must be rendered without crashing, and all 3 edges must be present.
#[test]
fn group_frame_obstacle_1325_edge_does_not_crash() {
    let s = svg(r#"
@startuml
package "Core" {
  component [B] as B
}
component [A] as A
component [C] as C
A --> B
B --> C
A --> C
@enduml
"#);
    assert!(s.contains("<svg"), "SVG output must be present, got:\n{s}");
    assert_eq!(
        count_relations(&s),
        3,
        "all 3 relations should be rendered with group-frame obstacle check, got:\n{s}"
    );
}

// ── #1326 ─────────────────────────────────────────────────────────────────────

/// #1326: An edge entering a package must be rendered even when the package
/// header band intersects the inter-rank routing channel.
#[test]
fn package_header_clearance_1326_edge_rendered() {
    let s = svg(r#"
@startuml
package "Services" {
  component [Parser] as Parser
}
component [CLI] as CLI
CLI --> Parser
@enduml
"#);
    assert_eq!(
        count_relations(&s),
        1,
        "the CLI→Parser edge should be rendered through package header clearance, got:\n{s}"
    );
}

// ── #1327 ─────────────────────────────────────────────────────────────────────

/// #1327: An edge from an unpackaged node to a node inside a package should be
/// rendered and the path must contain non-degenerate coordinates (not 0,0).
#[test]
fn package_framed_target_anchor_1327_edge_rendered() {
    let s = svg(r#"
@startuml
package "Pipeline Core" {
  component [Parser] as Parser
}
package "Frontends" {
  component [Adapters] as Adapters
}
Adapters --> Parser
@enduml
"#);
    assert_eq!(
        count_relations(&s),
        1,
        "Adapters→Parser edge across package boundary should be rendered, got:\n{s}"
    );
    // Path must have non-degenerate coordinates.
    let has_good_coords = s
        .lines()
        .filter(|line| line.contains("uml-relation"))
        .any(|line| !line.contains("0,0 0,0") && !line.contains("M 0 0 L 0 0"));
    assert!(
        has_good_coords,
        "Adapters→Parser path should have non-zero coordinates, got:\n{s}"
    );
}
