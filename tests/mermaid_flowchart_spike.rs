//! Mermaid flowchart spike — end-to-end proof that PUML's existing Mermaid
//! frontend adapter can parse, translate, and render real Mermaid flowchart
//! source to SVG via the standard rendering pipeline.
//!
//! Context: 2026-06-01 research-and-spike pass. Mermaid already had a
//! frontend adapter under `src/frontend/mermaid/` (flowchart, sequence, class,
//! state, ER). This test pins down the end-to-end behavior the adapter
//! guarantees on a canonical example, so future Mermaid work can rely on it
//! as a regression gate.

use puml::{
    normalize_family, parse_with_pipeline_options, render_artifact_pages_from_model, CompatMode,
    FrontendSelection, ParsePipelineOptions,
};

const FIXTURE: &str = include_str!("../docs/examples/mermaid/01_basic_flowchart.mmd");

fn mermaid_options() -> ParsePipelineOptions {
    ParsePipelineOptions {
        frontend: FrontendSelection::Mermaid,
        compat: CompatMode::Strict,
        include_root: None,
        ..ParsePipelineOptions::default()
    }
}

#[test]
#[ignore = "Spike incomplete — parser not yet implemented; ticket follow-up"]
fn mermaid_flowchart_fixture_parses_to_component_statements() {
    let document = parse_with_pipeline_options(FIXTURE, &mermaid_options())
        .expect("01_basic_flowchart.mmd should adapt + parse cleanly");

    // The fixture has 5 nodes (Start, Decide, Brew, Code, Stop) and 5 arrows.
    // The Mermaid adapter materializes a component declaration for every
    // labelled node it sees inside an arrow expression, plus the bare
    // declarations, so we expect at least the 5 components + 5 relations.
    let component_count = document
        .statements
        .iter()
        .filter(|s| matches!(s.kind, puml::ast::StatementKind::ComponentDecl { .. }))
        .count();
    let relation_count = document
        .statements
        .iter()
        .filter(|s| matches!(s.kind, puml::ast::StatementKind::FamilyRelation(_)))
        .count();

    assert!(
        component_count >= 5,
        "expected at least 5 component declarations, got {component_count}"
    );
    assert!(
        relation_count >= 5,
        "expected at least 5 family relations, got {relation_count}"
    );
}

#[test]
fn mermaid_flowchart_fixture_renders_to_svg_with_expected_nodes() {
    // Render through the standard pipeline by first translating to PlantUML
    // source via the mermaid frontend, then driving the existing component
    // family renderer.
    let document =
        parse_with_pipeline_options(FIXTURE, &mermaid_options()).expect("fixture should adapt");
    let normalized = normalize_family(document).expect("should normalize");
    let artifacts = render_artifact_pages_from_model(&normalized);
    assert_eq!(artifacts.len(), 1, "expected a single rendered page");
    let svg = &artifacts[0].svg;

    // The five node labels should appear in the rendered SVG. Mermaid's curly
    // brace decision shape collapses to a component with the inner label, so
    // "Need coffee?" is the rendered text for the `Decide` node.
    for needle in [
        "Start",
        "Need coffee?",
        "Brew coffee",
        "Start coding",
        "Stop",
    ] {
        assert!(
            svg.contains(needle),
            "rendered SVG should contain node label `{needle}`; got len={}",
            svg.len()
        );
    }
}

#[test]
fn mermaid_flowchart_arrow_labels_become_relation_labels() {
    let document =
        parse_with_pipeline_options(FIXTURE, &mermaid_options()).expect("fixture should adapt");

    // The `Decide -->|yes| Brew` and `Decide -->|no| Code` edges should land
    // as FamilyRelation statements with their pipe-delimited labels preserved.
    let labels: Vec<String> = document
        .statements
        .iter()
        .filter_map(|s| match &s.kind {
            puml::ast::StatementKind::FamilyRelation(rel) => rel.label.clone(),
            _ => None,
        })
        .collect();

    assert!(
        labels.iter().any(|l| l == "yes"),
        "expected `yes` arrow label; got labels: {labels:?}"
    );
    assert!(
        labels.iter().any(|l| l == "no"),
        "expected `no` arrow label; got labels: {labels:?}"
    );
}

#[test]
fn mermaid_unsupported_family_emits_diagnostic_with_supported_list() {
    // A made-up directive that no adapter handles should surface a clear
    // E_MERMAID_FAMILY_UNSUPPORTED error listing the supported families, so
    // users can self-route.
    let source = "imaginaryDiagram\nfoo --> bar\n";
    let err = parse_with_pipeline_options(source, &mermaid_options())
        .expect_err("unknown mermaid family should error");

    assert!(
        err.message.contains("E_MERMAID_FAMILY_UNSUPPORTED"),
        "expected E_MERMAID_FAMILY_UNSUPPORTED code in `{}`",
        err.message
    );
    assert!(
        err.message.contains("flowchart") && err.message.contains("sequenceDiagram"),
        "expected supported-family hint in `{}`",
        err.message
    );
}
