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
