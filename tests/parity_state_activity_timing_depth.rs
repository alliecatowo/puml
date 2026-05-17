#[test]
fn activity_swimlane_and_fork_depth_renders_lane_boxes_and_branch_markers() {
    let src = r#"@startuml
title Activity swimlane + fork depth
partition API {
start
:receive request;
fork
:validate payload;
fork again
partition Worker {
:run job;
}
end fork
if (ok?) then (yes)
:emit response;
else (no)
:emit error;
endif
stop
}
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("activity render should succeed");

    assert!(svg.contains("activity diagram"));
    assert!(svg.contains("partition: API"));
    assert!(svg.contains("partition: Worker"));
    assert!(svg.contains("branch 2"));
    assert!(svg.contains("stroke-dasharray=\"4 2\""));
}

#[test]
fn state_internals_history_choice_depth_renders_expected_shapes_and_actions() {
    let src = r#"@startuml
title State internals/history/choice depth
state Ready {
  Ready : entry / setup
  Ready : exit / cleanup
}
state ChoiceNode <<choice>>
state ForkNode <<fork>>
state JoinNode <<join>>
[H]
[H*]
Ready --> ChoiceNode : check
ChoiceNode --> ForkNode : yes
ForkNode --> JoinNode
JoinNode --> [H]
[H] --> [H*]
[H*] --> Ready
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("state render should succeed");

    assert!(svg.contains("state diagram"));
    assert!(svg.contains("entry / setup"));
    assert!(svg.contains("exit / cleanup"));
    assert!(svg.contains("<polygon points=\""));
    assert!(svg.contains(">H<"));
    assert!(svg.contains(">H*<"));
}

#[test]
fn timing_waveform_semantics_depth_renders_global_events_and_binary_clock_states() {
    let src = r#"@startuml
title Timing waveform semantics depth
robust BUS
binary FLAG
clock CLK
@0 FLAG is low
@5 FLAG is on
@10 FLAG is off
@0 BUS is IDLE
@8 BUS is RUN
@16 BUS is WAIT
@4 system-ready
@12 checkpoint
@0 CLK is high
@6 CLK is low
@12 CLK is high
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("timing render should succeed");

    assert!(svg.contains("timing diagram"));
    assert!(svg.contains("system-ready"));
    assert!(svg.contains("checkpoint"));
    assert!(svg.contains("FLAG"));
    assert!(svg.contains("BUS"));
    assert!(svg.contains("CLK"));
    assert!(svg.contains("<polyline"));
}
