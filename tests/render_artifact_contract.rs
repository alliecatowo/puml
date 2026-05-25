use puml::render::RenderValidationState;
use puml::render_core::SceneAvailability;

fn render_family_artifact(source: &str) -> puml::render::RenderArtifact {
    let doc = puml::parse(source).expect("parse should succeed");
    match puml::normalize_family(doc).expect("normalize should succeed") {
        puml::NormalizedDocument::Family(family) => puml::render_family_document_artifact(&family),
        other => panic!("expected family document, got {other:?}"),
    }
}

#[test]
fn migrated_component_renderer_requires_typed_scene_before_svg_backstop() {
    let artifact = render_family_artifact(
        r#"
@startuml
component API
component Worker
API --> Worker : jobs
@enduml
"#,
    );

    assert_eq!(artifact.scene_availability(), SceneAvailability::TypedScene);
    assert!(artifact.require_typed_scene().is_ok());
    assert_eq!(
        artifact.validation_state(),
        RenderValidationState::TypedScene
    );
    assert!(
        artifact.invariant_report.is_some(),
        "family render path should record SVG fallback plus typed validation report"
    );
    assert!(
        artifact
            .diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.message.contains("E_RENDER_SCENE_REQUIRED")),
        "migrated component path must not silently drop the typed scene contract: {:?}",
        artifact.diagnostics
    );
}
