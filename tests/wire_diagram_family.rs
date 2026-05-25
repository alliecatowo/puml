use puml::model::{NormalizedDocument, WirePortSide};
use puml::{detect_diagram_family, normalize_family, parse};

#[test]
fn startwire_detects_and_normalizes_components_ports_and_links() {
    let src = include_str!("../docs/examples/wire/01_basic_components.puml");
    assert_eq!(
        detect_diagram_family(src).expect("detect wire"),
        puml::DiagramFamily::Wire
    );
    let document = parse(src).expect("parse wire");
    assert_eq!(document.kind, puml::ast::DiagramKind::Wire);
    let NormalizedDocument::Wire(wire) = normalize_family(document).expect("normalize wire") else {
        panic!("expected wire model");
    };

    assert_eq!(wire.title.as_deref(), Some("Wire harness controller"));
    assert_eq!(wire.components.len(), 2);
    assert_eq!(wire.links.len(), 2);
    assert!(wire.components[0]
        .ports
        .iter()
        .any(|port| port.side == WirePortSide::Right && port.label == "POWER"));
}

#[test]
fn wire_renderer_returns_svg_and_typed_scene() {
    let src = include_str!("../docs/examples/wire/02_columns_spacing.puml");
    let NormalizedDocument::Wire(wire) =
        normalize_family(parse(src).expect("parse wire")).expect("normalize wire")
    else {
        panic!("expected wire model");
    };
    let artifact = puml::render::render_wire_artifact(&wire);

    assert!(artifact.svg.contains("wire-component"));
    assert!(artifact.svg.contains("InputBank"));
    assert!(artifact.svg.contains("debug"));
    let scene = artifact.scene.expect("wire scene");
    assert_eq!(scene.nodes.len(), 3);
    assert_eq!(scene.edges.len(), 2);
    assert!(
        scene.validate_geometry().is_empty(),
        "wire scene should satisfy typed geometry invariants"
    );
}

#[test]
fn wire_document_artifact_dispatch_carries_scene_contract() {
    let src = include_str!("../docs/examples/wire/03_variables_print.puml");
    let NormalizedDocument::Wire(wire) =
        normalize_family(parse(src).expect("parse wire")).expect("normalize wire")
    else {
        panic!("expected wire model");
    };
    let artifact = puml::render::render_wire_artifact(&wire);

    assert!(artifact.svg.contains("Main_Switch"));
    assert!(artifact.scene.is_some());
}
