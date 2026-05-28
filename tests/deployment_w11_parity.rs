/// Wave-11 batch E: deployment diagram parity tests
/// Covers: nested node/cloud/frame blocks, artifact stereotypes, deploy-spec
/// stereotypes on nodes, and frame fill colors.

fn render(src: &str) -> String {
    puml::render_source_to_svg(src).expect("render should succeed")
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Nested node blocks appear in the SVG output
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn nested_node_blocks_render() {
    let src = r#"
@startuml
node "Datacenter" {
  node "Rack A" {
    node "Server1"
  }
}
@enduml
"#;
    let svg = render(src);
    assert!(
        svg.contains("Datacenter"),
        "outer node label missing\n{svg}"
    );
    assert!(
        svg.contains("Rack A") || svg.contains("RackA") || svg.contains("Rack"),
        "nested node label missing\n{svg}"
    );
    assert!(
        svg.contains("Server1") || svg.contains("Server"),
        "doubly-nested node label missing\n{svg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Cloud container block renders
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn cloud_container_renders() {
    let src = r#"
@startuml
cloud "AWSCloud" {
  node "EC2"
  node "RDS"
}
@enduml
"#;
    let svg = render(src);
    assert!(
        svg.contains("AWSCloud") || svg.contains("AWS"),
        "cloud label missing\n{svg}"
    );
    assert!(svg.contains("EC2"), "EC2 node missing\n{svg}");
    assert!(svg.contains("RDS"), "RDS node missing\n{svg}");
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. artifact with stereotype renders
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn artifact_with_stereotype_renders() {
    let src = r#"
@startuml
node "AppServer" {
  artifact "appwar" <<artifact>>
  artifact "configxml" <<config>>
}
@enduml
"#;
    let svg = render(src);
    assert!(
        svg.contains("appwar"),
        "artifact name missing\n{svg}"
    );
    assert!(
        svg.contains("configxml"),
        "second artifact missing\n{svg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. <<stereotype>> after a node name is accepted and renders
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn node_stereotype_renders() {
    let src = r#"
@startuml
node "LoadBalancer" <<lb>>
node "AppServer" <<appserver>>
@enduml
"#;
    let svg = render(src);
    assert!(
        svg.contains("LoadBalancer") || svg.contains("lb"),
        "node with stereotype missing\n{svg}"
    );
    assert!(
        svg.contains("AppServer") || svg.contains("appserver"),
        "second node with stereotype missing\n{svg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. <<deploy spec: ...>> multi-word stereotype is accepted
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn deploy_spec_stereotype_accepted() {
    let src = r#"
@startuml
node "WebServer" <<deploy spec: 2 instances>>
node "DBServer" <<deploy spec: master-slave>>
@enduml
"#;
    let svg = render(src);
    assert!(!svg.is_empty(), "SVG should not be empty\n{svg}");
    assert!(
        svg.contains("WebServer") || svg.contains("DBServer"),
        "deploy spec node missing\n{svg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. frame "Name" #Color applies fill color
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn frame_fill_color_applied() {
    let src = r#"
@startuml
frame "Production" #LightYellow {
  node "WebApp"
  node "Database"
}
@enduml
"#;
    let svg = render(src);
    assert!(svg.contains("Production"), "frame label missing\n{svg}");
    // The fill color should be applied: LightYellow = #ffffe0
    assert!(
        svg.contains("#ffffe0") || svg.contains("ffffe0"),
        "LightYellow fill color not applied to frame\n{svg}"
    );
}
