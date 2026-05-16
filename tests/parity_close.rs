/// Parity closure tests for issues #87, #88, #103, #128.
use puml::{
    normalize, normalize_family, parse, parse_picouml, parse_with_pipeline_options,
    FrontendSelection, NormalizedDocument, ParsePipelineOptions,
};

// ─── Issue #87: hide unlinked ────────────────────────────────────────────────

#[test]
fn hide_unlinked_filters_participants_not_in_any_event() {
    let src = "@startuml\nhide unlinked\nparticipant Alice\nparticipant Bob\nparticipant Unlinked\nAlice -> Bob: hello\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    assert!(
        model.hide_unlinked,
        "hide_unlinked flag should be set on model"
    );
    assert_eq!(
        model.participants.len(),
        2,
        "Unlinked participant should be filtered out"
    );
    let ids: Vec<&str> = model.participants.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.contains(&"Alice"), "Alice should remain");
    assert!(ids.contains(&"Bob"), "Bob should remain");
    assert!(!ids.contains(&"Unlinked"), "Unlinked should be removed");
}

#[test]
fn hide_unlinked_emits_w_hide_unlinked_filtered_diagnostic() {
    let src = "@startuml\nhide unlinked\nparticipant Alice\nparticipant Bob\nparticipant Unlinked\nAlice -> Bob: hello\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    let has_warning = model
        .warnings
        .iter()
        .any(|w| w.message.contains("W_HIDE_UNLINKED_FILTERED"));
    assert!(
        has_warning,
        "expected W_HIDE_UNLINKED_FILTERED warning, got: {:?}",
        model.warnings.iter().map(|w| &w.message).collect::<Vec<_>>()
    );
}

#[test]
fn hide_unlinked_keeps_all_participants_when_all_are_referenced() {
    let src = "@startuml\nhide unlinked\nparticipant Alice\nparticipant Bob\nAlice -> Bob: hello\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");

    assert_eq!(
        model.participants.len(),
        2,
        "all referenced participants should remain"
    );
    let has_warning = model
        .warnings
        .iter()
        .any(|w| w.message.contains("W_HIDE_UNLINKED_FILTERED"));
    assert!(
        !has_warning,
        "no W_HIDE_UNLINKED_FILTERED warning expected when all participants are referenced"
    );
}

#[test]
fn hide_unlinked_fixture_parses_and_normalizes() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let src =
        std::fs::read_to_string(format!("{manifest}/tests/fixtures/basic/valid_hide_unlinked.puml"))
            .unwrap();
    let doc = parse(&src).expect("fixture should parse");
    let model = normalize::normalize(doc).expect("fixture should normalize");
    assert!(model.hide_unlinked, "hide_unlinked should be set");
    // Unlinked participant should be filtered
    let unlinked = model.participants.iter().find(|p| p.id == "Unlinked");
    assert!(unlinked.is_none(), "Unlinked participant should be removed");
}

// ─── Issue #103: JSON projection ─────────────────────────────────────────────

#[test]
fn json_projection_parses_in_class_diagram() {
    let src = "@startuml\nclass User {\n  +name: String\n}\njson Payload {\n  \"id\": 1\n}\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let has_json = doc.statements.iter().any(|s| {
        matches!(
            &s.kind,
            puml::ast::StatementKind::JsonProjection { alias, .. } if alias == "Payload"
        )
    });
    assert!(has_json, "JsonProjection statement should be present in AST");
}

#[test]
fn json_projection_normalizes_into_family_document_json_nodes() {
    let src = "@startuml\nclass User {\n  +name: String\n}\njson Payload {\n  \"id\": 1\n}\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let normalized = normalize_family(doc).expect("normalize should succeed");
    match normalized {
        NormalizedDocument::Family(family) => {
            assert_eq!(family.json_nodes.len(), 1, "one JSON node expected");
            assert_eq!(family.json_nodes[0].alias, "Payload");
            assert!(
                family.json_nodes[0].body.contains("\"id\""),
                "body should contain JSON content"
            );
        }
        _ => panic!("expected Family document"),
    }
}

#[test]
fn json_projection_fixture_renders_svg() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let src = std::fs::read_to_string(format!(
        "{manifest}/tests/fixtures/families/valid_class_with_json_projection.puml"
    ))
    .unwrap();
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");
    assert!(svg.contains("json UserPayload"), "SVG should mention json alias");
    assert!(svg.contains("JSON Projections"), "SVG should include JSON Projections section");
}

// ─── Issue #128: PicoUML canonical parser ────────────────────────────────────

#[test]
fn parse_picouml_parses_canonical_markers() {
    let src = "@startpicouml\nparticipant Alice\nAlice -> Alice: self\n@endpicouml\n";
    let doc = parse_picouml(src).expect("parse_picouml should succeed");
    // Should produce a valid sequence document
    let model = normalize::normalize(doc).expect("normalize should succeed");
    assert_eq!(model.participants.len(), 1);
    assert_eq!(model.participants[0].id, "Alice");
}

#[test]
fn parse_picouml_rejects_mixed_markers() {
    let src = "@startpicouml\nparticipant Alice\n@enduml\n";
    let err = parse_picouml(src).expect_err("mixed markers should fail");
    assert!(
        err.message.contains("E_PICOUML_MARKER_MIXED"),
        "expected E_PICOUML_MARKER_MIXED, got: {}",
        err.message
    );
}

#[test]
fn picouml_frontend_selection_routes_to_canonical_parser() {
    let src = "@startpicouml\nparticipant Alice\nparticipant Bob\nAlice -> Bob: ping\n@endpicouml\n";
    let options = ParsePipelineOptions {
        frontend: FrontendSelection::Picouml,
        ..ParsePipelineOptions::default()
    };
    let doc = parse_with_pipeline_options(src, &options).expect("should parse via Picouml frontend");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    assert_eq!(model.participants.len(), 2);
    assert_eq!(model.events.len(), 1);
}

#[test]
fn picouml_fixture_valid_canonical_parses() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let src = std::fs::read_to_string(format!(
        "{manifest}/tests/fixtures/picouml/valid_canonical.picouml"
    ))
    .unwrap();
    let doc = parse_picouml(&src).expect("canonical picouml fixture should parse");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    assert!(!model.participants.is_empty(), "should have participants");
}

#[test]
fn picouml_fixture_messages_parses() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let src = std::fs::read_to_string(format!(
        "{manifest}/tests/fixtures/picouml/valid_picouml_messages.puml"
    ))
    .unwrap();
    let doc = parse_picouml(&src).expect("picouml messages fixture should parse");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    assert_eq!(model.participants.len(), 3);
}

#[test]
fn picouml_fixture_notes_parses() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let src = std::fs::read_to_string(format!(
        "{manifest}/tests/fixtures/picouml/valid_picouml_notes.puml"
    ))
    .unwrap();
    let doc = parse_picouml(&src).expect("picouml notes fixture should parse");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    assert_eq!(model.participants.len(), 2);
}
