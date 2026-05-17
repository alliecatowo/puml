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

#[test]
fn timing_parity_handles_labels_controls_and_standalone_time_blocks() {
    let src = r#"@startuml
title Timing labels and controls
concise "Request phase" as REQ
robust "Bus state" as BUS
binary FLAG
clock CLK with period 8 pulse 3
@0
REQ is "Queued"
BUS is IDLE
FLAG is off
@4 handshake
@8
REQ is "Running"
BUS is RUN
FLAG is on
@16 done
FLAG is low
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("timing render should succeed");

    assert!(svg.contains("Request phase"));
    assert!(svg.contains("Bus state"));
    assert!(svg.contains("Queued"));
    assert!(svg.contains("Running"));
    assert!(svg.contains("handshake"));
    assert!(svg.contains("done"));
    assert!(svg.contains("FLAG"));
    assert!(svg.contains("CLK"));
    assert!(svg.contains("<polyline"));
}

#[test]
fn timing_relative_times_ranges_highlights_and_braced_states_render() {
    let src = r#"@startuml
title Timing range semantics
concise PHASE
binary FLAG
@0
PHASE is {Queued}
FLAG is off
@+5
PHASE is {Running}
FLAG is on
@5 <-> @12 : active window
highlight 12 to 18 : cooldown
@18 FLAG is low
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("timing render should succeed");

    assert!(svg.contains("Queued"));
    assert!(svg.contains("Running"));
    assert!(svg.contains("@5"));
    assert!(svg.contains("class=\"timing-range\""));
    assert!(svg.contains("active window"));
    assert!(svg.contains("cooldown"));
}

#[test]
fn class_nested_package_frames_render_from_scoped_blocks() {
    let src = r#"@startuml
title Nested class packages
package Domain {
namespace Core {
class User
class Account
}
}
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("class render should succeed");

    assert!(svg.contains("class=\"uml-group-frame\" data-uml-group=\"Domain\""));
    assert!(svg.contains("class=\"uml-group-frame\" data-uml-group=\"Domain::Core\""));
    assert!(svg.contains(">package Domain<"));
    assert!(svg.contains(">package Core<"));
    assert!(svg.contains("class Domain::Core::User"));
}

#[test]
fn component_nested_package_frames_render_from_scoped_blocks() {
    let src = r#"@startuml
title Nested component packages
package Edge {
node Rack {
component "API Gateway" as api
interface Gateway
}
}
api --> Gateway : exposes
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("component render should succeed");

    assert!(svg.contains("component diagram"));
    assert!(svg.contains("class=\"uml-group-frame\" data-uml-group=\"Edge\""));
    assert!(svg.contains("class=\"uml-group-frame\" data-uml-group=\"Edge::Rack\""));
    assert!(svg.contains(">package Edge<"));
    assert!(svg.contains(">package Rack<"));
    assert!(svg.contains("API Gateway"));
    assert!(svg.contains("exposes"));
}

#[test]
fn activity_beta_loop_branch_labels_render_is_and_not_clauses() {
    let src = include_str!("fixtures/families/valid_activity_loop_branch_labels.puml");
    let svg = puml::render_source_to_svg(src).expect("activity loop labels should render");

    assert!(svg.contains("healthy? / yes"));
    assert!(svg.contains("more work? / yes / no"));
    assert!(svg.contains("#008080"));
    assert!(svg.contains("repeat while"));
}
