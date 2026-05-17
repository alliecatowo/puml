#[test]
fn gantt_places_milestone_using_constraint_day_or_task_reference() {
    let src = r#"@startgantt
[Design]
[Build]
[Kickoff] happens on 2026-05-01
[Release] happens on D+5
[Build] requires [Design]
[Release] requires [Build]
@endgantt
"#;
    let svg = puml::render_source_to_svg(src).expect("gantt render");
    assert!(svg.contains("Kickoff"));
    assert!(svg.contains("Release"));
    assert!(svg.contains("marker-end=\"url(#gantt-arrow)\""));
    assert!(svg.contains("<polygon points=\""));
}

#[test]
fn gantt_renders_resource_lanes_project_date_axis_and_workload_duration() {
    let src = r#"@startgantt
Project starts 2026-05-01
saturday are closed
sundays are closed
[Design] on {Alice} requires 2 days
[Build] on {Bob:50%} starts 2026-05-03 and requires 1 week
[Launch] happens on 2026-05-10
@endgantt
"#;
    let svg = puml::render_source_to_svg(src).expect("gantt render");
    assert!(svg.contains("class=\"resource-lane\""));
    assert!(svg.contains("Alice"));
    assert!(svg.contains("Bob:50%"));
    assert!(svg.contains("2026-05-01"));
    assert!(svg.contains("class=\"gantt-calendar\""));
    assert!(svg.contains("Calendar: closed Saturday, Sunday"));
    assert!(svg.contains("Launch"));
    assert!(
        !svg.contains(">D+0<"),
        "project-start diagrams should use date axis labels"
    );
}

#[test]
fn chronology_sorts_iso_dates_and_renders_event_cards() {
    let src = r#"@startchronology
GA happens on 2026-10-01
Discovery happens on 2026-05-01
Beta happens on 2026-08-01
@endchronology
"#;
    let svg = puml::render_source_to_svg(src).expect("chronology render");
    let d = svg.find("Discovery").expect("discovery");
    let b = svg.find("Beta").expect("beta");
    let g = svg.find("GA").expect("ga");
    assert!(d < b && b < g, "events should be sorted by date");
    assert!(svg.contains("stroke=\"#cbd5e1\""));
}

#[test]
fn activity_new_style_decision_loop_and_merge_tags_are_visible() {
    let src = r#"@startuml
start
:load;
while (ready?) is (yes)
  :run;
endwhile
if (ok?) then (yes)
  :done;
else (no)
  :retry;
endif
stop
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("activity render");
    assert!(svg.contains("while"));
    assert!(svg.contains("(endwhile)"));
    assert!(svg.contains("(else)"));
    assert!(svg.contains("(endif)"));
}

#[test]
fn state_self_loop_and_final_pseudostate_render_distinctly() {
    let src = r#"@startuml
state A
state E <<end>>
A --> A : tick
A --> E
[*] --> A
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("state render");
    assert!(svg.contains("<path d=\"M "));
    assert!(svg.contains("r=\"14\""));
}

#[test]
fn timing_semantics_include_global_events_and_on_off_binary_states() {
    let src = r#"@startuml
binary sig
clock clk
@0 sig is off
@5 sig is on
@10 sig is off
@3 checkpoint
@7 deploy
@0 clk is high
@5 clk is low
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("timing render");
    assert!(svg.contains("checkpoint"));
    assert!(svg.contains("deploy"));
    assert!(svg.contains("<polyline"));
    assert!(svg.contains("timing diagram"));
}
