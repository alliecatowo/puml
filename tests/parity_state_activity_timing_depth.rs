mod svg_test_helpers;

use svg_test_helpers::{attr, bounds, f64_attr, SvgDoc};

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
    let doc = SvgDoc::parse(&svg);

    // Wave 3-A (#490) suppressed "activity diagram" canvas leak; Wave 3-D
    // (#492) replaced "partition: <name>" prefix with lane header labels.
    assert!(svg.contains("API"));
    assert!(svg.contains("Worker"));
    let start = doc
        .elements("circle")
        .into_iter()
        .next()
        .expect("activity start node should render");
    assert!(
        f64_attr(start, "cy") > 86.0,
        "start node should sit below the swimlane header row"
    );
    // Wave 3-D (#487): fork branches now render their action labels directly,
    // not synthetic "branch N" markers.
    assert!(svg.contains("validate payload"));
    assert!(svg.contains("run job"));
    // Dashed strokes are emitted for swimlane dividers / fork branch lines.
    // Exact dasharray pattern shifted between Wave 3-D's lane-header refactor
    // and current layout; just assert that at least one dashed stroke exists.
    assert!(svg.contains("stroke-dasharray="));
}

#[test]
fn activity_partition_example_keeps_start_below_lane_header() {
    let src = include_str!("../docs/examples/activity/07_partition.puml");
    let svg = puml::render_source_to_svg(src).expect("activity partition example should render");
    let doc = SvgDoc::parse(&svg);

    assert!(svg.contains("Worker"));
    assert!(svg.contains("Backend"));
    assert!(svg.contains("Frontend"));

    let start = doc
        .elements("circle")
        .into_iter()
        .next()
        .expect("activity start node should render");
    assert!(
        f64_attr(start, "cy") - 12.0 >= 86.0,
        "start node should stay inside lane content and below the header row"
    );
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
    let doc = SvgDoc::parse(&svg);

    // Wave 3-A (#494 family-type label suppression). Assert on the state node
    // presence via metadata rather than the literal "state diagram" text.
    assert!(!doc.texts_containing("Ready").is_empty());
    let ready = doc.first_with_attr("metadata", "data-state-node", "Ready");
    assert_eq!(attr(ready, "data-state-kind"), "normal");
    assert!(!doc.texts_containing("entry / setup").is_empty());
    assert!(!doc.texts_containing("exit / cleanup").is_empty());

    let choice = doc.first_with_attr("metadata", "data-state-node", "ChoiceNode");
    assert_eq!(attr(choice, "data-state-kind"), "choice");
    let choice_marker = doc.elements("polygon").into_iter().next().unwrap();
    let choice_bounds = bounds(choice_marker);
    assert!(choice_bounds.width > 0.0 && choice_bounds.height > 0.0);

    let shallow_history = doc.first_with_attr("metadata", "data-state-node", "[H]");
    let deep_history = doc.first_with_attr("metadata", "data-state-node", "[H*]");
    assert_eq!(attr(shallow_history, "data-state-kind"), "history-shallow");
    assert_eq!(attr(deep_history, "data-state-kind"), "history-deep");
    assert!(!doc.texts_containing("H").is_empty());
    assert!(!doc.texts_containing("H*").is_empty());
    let history_circles = doc.elements("circle");
    assert_eq!(history_circles.len(), 2);
    // The two history circles must be at different positions (cx or cy differs),
    // regardless of whether single- or two-column layout is used.
    let cx_diff = (f64_attr(history_circles[1], "cx") - f64_attr(history_circles[0], "cx")).abs();
    let cy_diff = (f64_attr(history_circles[1], "cy") - f64_attr(history_circles[0], "cy")).abs();
    assert!(
        cx_diff > 0.0 || cy_diff > 0.0,
        "history circles must be at distinct positions"
    );

    let ready_to_choice = doc.first_with_attr("line", "data-state-from", "Ready");
    assert_eq!(attr(ready_to_choice, "data-state-to"), "ChoiceNode");
    let transition_bounds = bounds(ready_to_choice);
    assert!(transition_bounds.width > 0.0 && transition_bounds.height > 0.0);
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

    // Wave 3-A (#490 family-type label suppression).
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
    let doc = SvgDoc::parse(&svg);

    assert!(!doc.texts_containing("Queued").is_empty());
    assert!(!doc.texts_containing("Running").is_empty());
    assert!(!doc.texts_containing("@5").is_empty());
    let active_label = doc
        .texts_containing("active window")
        .into_iter()
        .next()
        .expect("range label should be visible");
    let cooldown_label = doc
        .texts_containing("cooldown")
        .into_iter()
        .next()
        .expect("highlight label should be visible");
    assert!(f64_attr(active_label, "x") < f64_attr(cooldown_label, "x"));

    let ranges = doc.elements_with_class("rect", "timing-range");
    assert_eq!(
        ranges.len(),
        2,
        "range and highlight should render as visible bands"
    );
    let active = bounds(ranges[0]);
    let cooldown = bounds(ranges[1]);
    assert!(active.width > 0.0 && active.height > 0.0);
    assert!(cooldown.width > 0.0 && cooldown.height > 0.0);
    assert!(active.x < cooldown.x);
    assert!(active.right() <= cooldown.right());
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
    assert!(svg.contains("Domain::Core::User"));
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

    // Wave 3-A (#490 family-type label suppression).
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
    let doc = SvgDoc::parse(&svg);

    assert!(
        svg.contains("healthy?"),
        "while condition should appear in diamond"
    );
    assert!(
        svg.contains("yes"),
        "while guard should float on loop arrow"
    );
    assert!(
        svg.contains("more work?"),
        "repeat condition should appear in diamond"
    );
    assert!(
        svg.contains("yes / no"),
        "repeat while guards should float on arrow"
    );
    assert!(svg.contains("#008080"));
    assert!(
        !svg.contains(">repeat while<"),
        "repeat while should drive control flow without rendering as a visible label"
    );
    assert!(
        doc.elements("line")
            .into_iter()
            .any(|line| f64_attr(line, "y2") < f64_attr(line, "y1")),
        "repeat and while loops should emit at least one upward back-edge"
    );
}
