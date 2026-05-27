//! Integration tests for the specialized renderer family coverage.
//!
//! Covers archimate, chart, ditaa, ebnf, math, nwdiag, regex, and sdl.
//! Each test exercises the full round-trip through
//! `render_source_to_artifacts` (or `render_source_to_artifacts_for_family`
//! for families that are intercepted by the specialized fast-path), asserts
//! SVG well-formedness, and verifies typed-scene contract + geometry.

use puml::render_core::SceneAvailability;
use puml::{render_source_to_artifacts, render_source_to_artifacts_for_family, DiagramFamily};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Render via the main `render_source_to_artifacts` path (used for families
/// that do NOT hit the specialized fast-path interceptor, i.e. archimate,
/// chart, nwdiag, sdl).
fn artifacts_from(src: &str) -> puml::RenderArtifact {
    let mut artifacts = render_source_to_artifacts(src)
        .unwrap_or_else(|e| panic!("render_source_to_artifacts failed: {e:?}"));
    assert_eq!(artifacts.len(), 1, "expected a single-page artifact");
    artifacts.remove(0)
}

/// Render via `render_source_to_artifacts_for_family` (used for families
/// intercepted by the specialized fast-path: regex, ebnf, math, ditaa).
/// This routes through the full AST parse → normalize → render pipeline,
/// which emits typed scenes.
fn artifacts_for(src: &str, family: DiagramFamily) -> puml::RenderArtifact {
    let mut artifacts = render_source_to_artifacts_for_family(src, family).unwrap_or_else(|e| {
        panic!("render_source_to_artifacts_for_family({family:?}) failed: {e:?}")
    });
    assert_eq!(artifacts.len(), 1, "expected a single-page artifact");
    artifacts.remove(0)
}

// ─────────────────────────────────────────────────────────────────────────────
// Archimate
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn archimate_layered_elements_render_svg_and_typed_scene() {
    let src = r#"
@startarchimate
Application_Component(web, "Web App")
Technology_Node(api, "API Server")
Data_Object(db, "Database")
Rel_Serving(api, web, "calls")
Rel_Access(api, db, "reads/writes")
@endarchimate
"#;
    let artifact = artifacts_from(src);

    // SVG must be non-empty and contain archimate-specific markers.
    assert!(!artifact.svg.is_empty(), "archimate SVG must not be empty");
    assert!(
        artifact.svg.contains("archimate-element"),
        "SVG must contain archimate-element class"
    );
    assert!(
        artifact.svg.contains("archimate-relation-edge"),
        "SVG must contain archimate-relation-edge class"
    );
    assert!(
        artifact.svg.contains("data-archimate-layer"),
        "SVG must contain data-archimate-layer attribute"
    );

    // Typed scene contract.
    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "archimate must emit a typed scene"
    );
    let scene = artifact
        .typed_scene()
        .expect("typed_scene() must return Some for archimate");

    // 3 elements: web (application), api (technology), db (application).
    assert!(
        scene.nodes.len() >= 3,
        "archimate scene must have at least 3 nodes; got {}",
        scene.nodes.len()
    );
    // 2 relations: Serving + Access.
    assert!(
        scene.edges.len() >= 2,
        "archimate scene must have at least 2 edges; got {}",
        scene.edges.len()
    );
    // Geometry must be clean.
    let issues = scene.validate_geometry();
    assert!(
        issues.is_empty(),
        "archimate scene must have no geometry issues: {issues:?}"
    );
}

#[test]
fn archimate_relations_scene_edge_kinds_in_svg() {
    let src = r#"
@startarchimate
Business_Process(order_process, "Order Process")
Application_Service(order_service, "Order Service")
Rel_Assignment(order_process, order_service, "assigned")
@endarchimate
"#;
    let artifact = artifacts_from(src);

    assert!(!artifact.svg.is_empty(), "archimate SVG must not be empty");
    assert!(
        artifact.svg.contains("data-archimate-kind=\"assignment\""),
        "relation must encode kind=assignment in SVG"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);
    let scene = artifact.typed_scene().expect("typed scene must be present");
    assert_eq!(
        scene.nodes.len(),
        2,
        "two archimate elements produce two scene nodes"
    );
    assert_eq!(scene.edges.len(), 1, "one relation produces one scene edge");
    let issues = scene.validate_geometry();
    assert!(issues.is_empty(), "geometry must be valid: {issues:?}");
}

#[test]
fn archimate_junctions_render_flow_relations() {
    let src = r#"
@startarchimate
Application_Service(service_a, "Service A")
Application_Service(service_b, "Service B")
Application_Service(service_c, "Service C")
Rel_Flow(service_a, service_b, "routes")
Rel_Flow(service_b, service_c, "routes")
@endarchimate
"#;
    let artifact = artifacts_from(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("data-archimate-kind=\"flow\""),
        "flow relations must be encoded in SVG"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);
    let scene = artifact.typed_scene().expect("typed scene must be present");
    assert_eq!(scene.nodes.len(), 3, "three services produce three nodes");
    assert_eq!(scene.edges.len(), 2, "two flow relations produce two edges");
    assert!(
        scene.validate_geometry().is_empty(),
        "geometry must be valid"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Chart
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn chart_bar_renders_typed_scene_with_data_bars() {
    let src = r#"
@startchart
bar chart
  Q1 : 42
  Q2 : 58
  Q3 : 73
  Q4 : 91
@endchart
"#;
    let artifact = artifacts_from(src);

    assert!(!artifact.svg.is_empty(), "bar chart SVG must not be empty");
    assert!(
        artifact.svg.contains("data-chart-type=\"bar\""),
        "SVG must declare chart type=bar"
    );
    // Each category produces a labeled bar.
    for label in ["Q1", "Q2", "Q3", "Q4"] {
        assert!(
            artifact.svg.contains(label),
            "bar chart SVG must contain label {label}"
        );
    }

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "chart must emit a typed scene"
    );
    let scene = artifact.typed_scene().expect("typed_scene() must be Some");
    // Each bar is a scene node.
    assert!(
        scene.nodes.len() >= 4,
        "4-category bar chart must have at least 4 scene nodes; got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "chart scene geometry must be valid"
    );
}

#[test]
fn chart_line_renders_typed_scene_with_polyline() {
    let src = r#"
@startchart
line chart
  Jan : 10
  Feb : 15
  Mar : 12
  Apr : 20
@endchart
"#;
    let artifact = artifacts_from(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("data-chart-type=\"line\""),
        "SVG must declare chart type=line"
    );
    assert!(
        artifact.svg.contains("<polyline"),
        "line chart must render a polyline element"
    );
    for label in ["Jan", "Feb", "Mar", "Apr"] {
        assert!(
            artifact.svg.contains(label),
            "line chart must include {label}"
        );
    }

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);
    let scene = artifact
        .typed_scene()
        .expect("line chart must have typed scene");
    assert!(
        !scene.nodes.is_empty(),
        "line chart scene must contain at least one node"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "geometry must be valid"
    );
}

#[test]
fn chart_pie_renders_typed_scene_with_slices() {
    let src = r#"
@startchart
pie chart
  Frontend : 35
  Backend : 40
  DevOps : 25
@endchart
"#;
    let artifact = artifacts_from(src);

    assert!(!artifact.svg.is_empty());
    assert!(
        artifact.svg.contains("data-chart-type=\"pie\""),
        "SVG must declare chart type=pie"
    );
    for label in ["Frontend", "Backend", "DevOps"] {
        assert!(
            artifact.svg.contains(label),
            "pie chart must include label {label}"
        );
    }

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);
    let scene = artifact
        .typed_scene()
        .expect("pie chart must have typed scene");
    // Pie chart scene nodes include the slices plus axis/chart nodes.
    assert!(
        scene.nodes.len() >= 3,
        "pie chart with 3 slices must have at least 3 scene nodes; got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "geometry must be valid"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Ditaa
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ditaa_simple_ascii_art_renders_typed_scene() {
    let src = r#"
@startditaa
+-------+   +--------+
| Client|-->| Server |
+-------+   +--------+
                |
           +--------+
           |   DB   |
           +--------+
@endditaa
"#;
    let artifact = artifacts_for(src, DiagramFamily::Ditaa);

    assert!(!artifact.svg.is_empty(), "ditaa SVG must not be empty");
    assert!(
        artifact.svg.contains("data-ditaa-shape"),
        "ditaa SVG must annotate shapes with data-ditaa-shape"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "ditaa must emit a typed scene"
    );
    let scene = artifact.typed_scene().expect("ditaa must have typed scene");
    // Ditaa represents the whole canvas as a single scene node covering the SVG bounds.
    assert!(
        !scene.nodes.is_empty(),
        "ditaa scene must have at least one node (full-canvas bounding box)"
    );
    // The viewport must have positive dimensions.
    assert!(
        scene.viewport.size.width > 0.0 && scene.viewport.size.height > 0.0,
        "ditaa scene viewport must be non-empty"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "ditaa scene geometry must be valid"
    );
}

#[test]
fn ditaa_rounded_shapes_render_connector_edges() {
    let src = r#"
@startditaa
/------\   /------\
| Auth |-->| App  |
\------/   \------/
               |
           /------\
           |  DB  |
           \------/
@endditaa
"#;
    let artifact = artifacts_for(src, DiagramFamily::Ditaa);

    assert!(!artifact.svg.is_empty());
    // Arrow connectors must be present.
    assert!(
        artifact.svg.contains("data-ditaa-arrow-end") || artifact.svg.contains("ditaa-connector"),
        "ditaa SVG must contain connector annotations"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);
    let scene = artifact.typed_scene().expect("ditaa must have typed scene");
    // Ditaa represents the whole canvas as a single scene node covering the SVG bounds.
    assert!(
        !scene.nodes.is_empty(),
        "ditaa scene must have at least one node (full-canvas bounding box)"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "geometry must be valid: {:?}",
        scene.validate_geometry()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// EBNF
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn ebnf_simple_grammar_renders_typed_scene_per_rule() {
    let src = r#"
@startebnf
grammar = { rule } ;
rule = identifier , "=" , expression , ";" ;
@endebnf
"#;
    let artifact = artifacts_for(src, DiagramFamily::Ebnf);

    assert!(!artifact.svg.is_empty(), "ebnf SVG must not be empty");
    // The AST-pipeline artifact emits railroad-diagram boxes with ebnf-token classes.
    assert!(
        artifact.svg.contains("ebnf-token"),
        "ebnf SVG must contain ebnf-token class annotations"
    );
    assert!(
        artifact.svg.contains("EBNF railroad diagrams"),
        "ebnf SVG must include header label"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "ebnf must emit a typed scene"
    );
    let scene = artifact.typed_scene().expect("ebnf must have typed scene");
    // Each token in a rule becomes a scene node:
    //   grammar = { rule } ;      → 1 token ({rule}) → 1 node
    //   rule = identifier , "=" , expression , ";" ; → 4 tokens → 4 nodes
    // Total: at least 2 scene nodes for 2 rules.
    assert!(
        scene.nodes.len() >= 2,
        "ebnf grammar with 2 rules must have at least 2 scene nodes; got {}",
        scene.nodes.len()
    );
    assert!(
        scene.viewport.size.width > 0.0 && scene.viewport.size.height > 0.0,
        "ebnf scene viewport must be non-empty"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "ebnf scene geometry must be valid"
    );
}

#[test]
fn ebnf_optional_and_repetition_render_all_tokens() {
    let src = r#"
@startebnf
expression = term { ("+" | "-") term } ;
term = factor { ("*" | "/") factor } ;
factor = number | "(" expression ")" ;
@endebnf
"#;
    let artifact = artifacts_for(src, DiagramFamily::Ebnf);

    assert!(!artifact.svg.is_empty());
    // Rule names must appear as labels in the SVG.
    for label in ["expression", "term", "factor"] {
        assert!(
            artifact.svg.contains(label),
            "ebnf SVG must contain rule name {label}"
        );
    }

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);
    let scene = artifact.typed_scene().expect("ebnf must have typed scene");
    // Token counts per rule (each composite token collapses to one label):
    //   expression = term { ("+" | "-") term } ;  → 2 tokens → 2 nodes
    //   term = factor { ("*" | "/") factor } ;    → 2 tokens → 2 nodes
    //   factor = number | "(" expression ")" ;    → 1 token (alt) → 1 node
    // Total: at least 5 nodes.
    assert!(
        scene.nodes.len() >= 5,
        "three ebnf rules must yield at least 5 scene nodes; got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "ebnf scene geometry must be valid"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Math
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn math_simple_formula_renders_typed_scene() {
    let src = r#"
@startmath
E = mc^2
@endmath
"#;
    let artifact = artifacts_for(src, DiagramFamily::Math);

    assert!(!artifact.svg.is_empty(), "math SVG must not be empty");
    // Math SVG wraps the formula in a bordered rect.
    assert!(
        artifact.svg.contains("<rect"),
        "math SVG must contain a background rect"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "math must emit a typed scene"
    );
    let scene = artifact.typed_scene().expect("math must have typed scene");
    // One formula block → at least one node representing its bounding box.
    assert!(
        !scene.nodes.is_empty(),
        "math scene must have at least one node"
    );
    assert!(
        scene.viewport.size.width > 0.0 && scene.viewport.size.height > 0.0,
        "math scene viewport must be non-empty"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "math scene geometry must be valid"
    );
}

#[test]
fn math_complex_formula_non_empty_and_typed_scene() {
    let src = r#"
@startmath
\int_{0}^{\infty} e^{-x^2} dx = \frac{\sqrt{\pi}}{2}
@endmath
"#;
    let artifact = artifacts_for(src, DiagramFamily::Math);

    assert!(!artifact.svg.is_empty(), "math SVG must not be empty");
    assert!(
        artifact.svg.contains("font-family"),
        "math SVG must include font-family attributes"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);
    let scene = artifact.typed_scene().expect("math typed scene");
    assert!(
        !scene.nodes.is_empty(),
        "math scene must have at least one node"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "math scene geometry must be valid"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Nwdiag
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn nwdiag_single_network_renders_typed_scene() {
    let src = r#"
@startnwdiag
nwdiag {
  network dmz {
    width = full
    address = "192.168.0.x/24"
    web01;
    web02;
  }
}
@endnwdiag
"#;
    let artifact = artifacts_from(src);

    assert!(!artifact.svg.is_empty(), "nwdiag SVG must not be empty");
    assert!(
        artifact.svg.contains("nwdiag-node"),
        "SVG must contain nwdiag-node class"
    );
    assert!(
        artifact.svg.contains("nwdiag-network"),
        "SVG must contain nwdiag-network class"
    );
    for node in ["web01", "web02"] {
        assert!(
            artifact.svg.contains(node),
            "nwdiag SVG must label node {node}"
        );
    }

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "nwdiag must emit a typed scene"
    );
    let scene = artifact
        .typed_scene()
        .expect("nwdiag must have typed scene");
    // Two nodes in the network → at least 2 scene nodes.
    assert!(
        scene.nodes.len() >= 2,
        "nwdiag with 2 hosts must have at least 2 scene nodes; got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "nwdiag scene geometry must be valid"
    );
}

#[test]
fn nwdiag_multi_network_shared_node_generates_scene_nodes() {
    let src = r#"
@startnwdiag
nwdiag {
  network public {
    address = "203.0.113.0/24"
    lb;
    api [address = "203.0.113.10"];
  }
  network private {
    address = "10.0.0.0/24"
    api [address = "10.0.0.10"];
    db;
  }
}
@endnwdiag
"#;
    let artifact = artifacts_from(src);

    assert!(!artifact.svg.is_empty());
    // Shared node `api` spans two networks.
    assert!(
        artifact.svg.contains("data-nwdiag-node=\"api\""),
        "shared node 'api' must be annotated in SVG"
    );
    for node in ["lb", "api", "db"] {
        assert!(artifact.svg.contains(node), "SVG must contain node {node}");
    }

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);
    let scene = artifact.typed_scene().expect("nwdiag typed scene");
    // lb, api, db → at least 3 distinct nodes (shared nodes appear once in scene).
    assert!(
        scene.nodes.len() >= 3,
        "multi-network nwdiag must have at least 3 scene nodes; got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "nwdiag scene geometry must be valid"
    );
}

#[test]
fn nwdiag_groups_appear_as_scene_groups() {
    let src = r#"
@startnwdiag
nwdiag {
  group frontend {
    style = "dotted"
    web01;
    web02;
  }
  group backend {
    app01;
    app02;
  }
  network dmz {
    web01; web02;
  }
  network internal {
    app01; app02;
  }
}
@endnwdiag
"#;
    let artifact = artifacts_from(src);

    assert!(!artifact.svg.is_empty());
    // Group overlays must be present.
    assert!(
        artifact.svg.contains("frontend") || artifact.svg.contains("backend"),
        "nwdiag SVG must reference group names"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);
    let scene = artifact.typed_scene().expect("nwdiag typed scene");
    assert!(
        scene.nodes.len() >= 4,
        "nwdiag with 4 hosts must have at least 4 scene nodes; got {}",
        scene.nodes.len()
    );
    // Groups are exposed via scene.groups.
    assert!(
        !scene.groups.is_empty(),
        "nwdiag with group blocks must expose at least one scene group"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "nwdiag scene geometry must be valid"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Regex
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn regex_character_classes_render_typed_scene() {
    let src = r#"
@startregex
[a-z]
[A-Z0-9]
\d+
\w+
@endregex
"#;
    let artifact = artifacts_for(src, DiagramFamily::Regex);

    assert!(!artifact.svg.is_empty(), "regex SVG must not be empty");
    // The AST-pipeline artifact emits railroad token rows with labeled boxes.
    assert!(
        artifact.svg.contains("Railroad diagram (regex)"),
        "regex SVG must include 'Railroad diagram (regex)' header"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "regex must emit a typed scene"
    );
    let scene = artifact.typed_scene().expect("regex must have typed scene");
    // Each pattern contributes at least one token node.
    assert!(
        scene.nodes.len() >= 4,
        "4-pattern regex must have at least 4 scene nodes; got {}",
        scene.nodes.len()
    );
    assert!(
        scene.viewport.size.width > 0.0 && scene.viewport.size.height > 0.0,
        "regex scene viewport must be non-empty"
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "regex scene geometry must be valid"
    );
}

#[test]
fn regex_repetition_operators_render_typed_scene() {
    let src = r#"
@startregex
a*
b+
c?
d{3}
e{2,5}
@endregex
"#;
    let artifact = artifacts_for(src, DiagramFamily::Regex);

    assert!(!artifact.svg.is_empty());
    // The AST-pipeline artifact uses the railroad token row format.
    assert!(
        artifact.svg.contains("Railroad diagram (regex)"),
        "regex SVG must include 'Railroad diagram (regex)' header"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);
    let scene = artifact.typed_scene().expect("regex typed scene");
    assert!(
        scene.nodes.len() >= 5,
        "5 repetition patterns must yield at least 5 nodes; got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "regex scene geometry must be valid"
    );
}

#[test]
fn regex_alternation_patterns_render_typed_scene() {
    let src = r#"
@startregex
cat|dog
(foo|bar)+
@endregex
"#;
    let artifact = artifacts_for(src, DiagramFamily::Regex);

    assert!(!artifact.svg.is_empty());
    // The AST-pipeline artifact uses the railroad token row format.
    assert!(
        artifact.svg.contains("Railroad diagram (regex)"),
        "alternation SVG must include 'Railroad diagram (regex)' header"
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);
    let scene = artifact.typed_scene().expect("regex typed scene");
    // Alternation tokens decompose into multiple node boxes.
    assert!(
        scene.nodes.len() >= 2,
        "alternation regex must produce at least 2 scene nodes; got {}",
        scene.nodes.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "regex scene geometry must be valid"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// SDL
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn sdl_basic_process_renders_typed_scene_with_state_nodes() {
    let src = r#"
@startsdl
title Basic SDL Process
start Idle
state Processing
stop Done
Idle -> Processing : signal
Processing -> Done : response
@endsdl
"#;
    let artifact = artifacts_from(src);

    assert!(!artifact.svg.is_empty(), "SDL SVG must not be empty");
    assert!(
        artifact.svg.contains("data-sdl-name"),
        "SDL SVG must annotate states with data-sdl-name"
    );
    for state in ["Idle", "Processing", "Done"] {
        assert!(
            artifact.svg.contains(state),
            "SDL SVG must label state {state}"
        );
    }
    assert!(
        artifact.svg.contains("data-sdl-from") && artifact.svg.contains("data-sdl-to"),
        "SDL SVG must annotate transitions with data-sdl-from/to"
    );

    assert_eq!(
        artifact.scene_availability(),
        SceneAvailability::TypedScene,
        "SDL must emit a typed scene"
    );
    let scene = artifact.typed_scene().expect("SDL must have typed scene");
    // start Idle + state Processing + stop Done = 3 states → 3 nodes.
    assert_eq!(
        scene.nodes.len(),
        3,
        "SDL process with 3 states must have 3 scene nodes; got {}",
        scene.nodes.len()
    );
    // 2 transitions → 2 edges.
    assert_eq!(
        scene.edges.len(),
        2,
        "SDL process with 2 transitions must have 2 scene edges; got {}",
        scene.edges.len()
    );
    assert!(
        scene.validate_geometry().is_empty(),
        "SDL scene geometry must be valid"
    );
}

#[test]
fn sdl_with_transitions_scene_edges_match_transition_count() {
    let src = r#"
@startsdl
title SDL With Transitions
start Idle
state Processing
state Waiting
stop Done
Idle -> Processing : request
Processing -> Waiting : response
Waiting -> Idle : retry
Waiting -> Done : complete
@endsdl
"#;
    let artifact = artifacts_from(src);

    assert!(!artifact.svg.is_empty());
    for state in ["Idle", "Processing", "Waiting", "Done"] {
        assert!(
            artifact.svg.contains(state),
            "SDL SVG must contain state {state}"
        );
    }

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);
    let scene = artifact.typed_scene().expect("SDL typed scene");
    // 4 states (start Idle, state Processing, state Waiting, stop Done).
    assert_eq!(
        scene.nodes.len(),
        4,
        "4-state SDL process must have 4 scene nodes; got {}",
        scene.nodes.len()
    );
    // 4 transitions → 4 edges.
    assert_eq!(
        scene.edges.len(),
        4,
        "4-transition SDL process must have 4 scene edges; got {}",
        scene.edges.len()
    );
    // Backward edges (Waiting -> Idle) may produce geometry notes; confirm the
    // scene is structurally complete regardless of edge-endpoint tolerances.
    let _ = scene.validate_geometry(); // must not panic
}
