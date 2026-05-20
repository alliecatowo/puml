#[test]
fn activity_nested_error_paths_share_one_rollback_node() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
title Deployment Pipeline
start
:Code Push;
:Run Tests;
if (Tests pass?) then (yes)
  :Build Artifact;
  :Push to Registry;
  :Deploy to Staging;
  if (Staging OK?) then (yes)
    :Promote to Prod;
    :Smoke Tests;
    if (Smoke OK?) then (yes)
      :Done;
    else (no)
      :Rollback;
    endif
  else (no)
    :Rollback;
  endif
else (no)
  :Notify Dev;
endif
stop
@enduml
"#,
    )
    .expect("deployment pipeline activity should render");

    assert_eq!(
        svg.matches(">Rollback<").count(),
        1,
        "nested error branches should share a single rollback node"
    );
    assert_eq!(svg.matches(">Done<").count(), 1);
    assert_eq!(svg.matches(">Notify Dev<").count(), 1);
    assert!(
        !svg.contains("points=\"720,842")
            && !svg.contains("points=\"720,902")
            && !svg.contains("points=\"720,962"),
        "nested endif merges should not leave dangling arrowheads before the final stop"
    );
}

#[test]
fn activity_renderer_emits_canonical_semantic_svg_hooks() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
|Worker|
start
:Review;
if (ok?) then (yes)
  :Ship;
else (no)
  :Fix;
endif
stop
@enduml
"#,
    )
    .expect("activity semantic hook fixture should render");

    for kind in ["start", "action", "decision", "end", "swimlane"] {
        assert!(
            svg.contains(&format!("data-puml-kind=\"{kind}\"")),
            "activity SVG should include puml-node kind {kind:?}"
        );
    }
    assert!(svg.contains("class=\"activity-action puml-node\""));
    assert!(svg.contains("class=\"activity-arrow puml-edge\""));
    assert!(svg.contains("class=\"activity-label puml-label\""));
    assert!(svg.contains("class=\"activity-swimlane puml-node\""));
    assert!(svg.contains("data-puml-family=\"activity\""));
    assert!(svg.contains("data-puml-bbox="));
    assert!(svg.contains("data-puml-from="));
    assert!(svg.contains("data-puml-to="));
    assert!(svg.contains("data-puml-label-kind="));

    // Existing activity-specific metadata remains available for downstream
    // tests and tools while canonical puml-* hooks are layered on top.
    assert!(svg.contains("data-activity-kind=\"Start\""));
    assert!(svg.contains("data-activity-lane=\"Worker\""));
}
