use puml::parser::{parse_with_options, ParseOptions};
use puml::{DiagramFamily, NormalizedDocument};

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
    assert!(svg.contains("class=\"gantt-milestone"));
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
fn gantt_closed_weekdays_extend_workload_span_on_date_axis() {
    let src = r#"@startgantt
Project starts 2026-05-01
saturday are closed
sundays are closed
[Build] requires 2 days
@endgantt
"#;
    let svg = puml::render_source_to_svg(src).expect("gantt render");
    assert!(svg.contains("Calendar: closed Saturday, Sunday"));
    assert!(svg.contains("2026-05-01"));
    assert!(
        svg.contains("2026-05-05"),
        "two working days from Friday with a closed weekend should span through Tuesday"
    );
}

#[test]
fn gantt_closed_date_ranges_extend_workload_and_render_calendar_band() {
    let src = r#"@startgantt
Project starts 2026-05-01
saturday are closed
sundays are closed
2026-05-04 to 2026-05-05 is closed
[Build] requires 2 days
@endgantt
"#;
    let svg = puml::render_source_to_svg(src).expect("gantt render");
    assert!(svg.contains("2026-05-04 to 2026-05-05"));
    assert!(svg.contains("class=\"gantt-closed-range\""));
    let doc = parse_with_options(src, &ParseOptions::default()).expect("parse gantt");
    let NormalizedDocument::Timeline(model) = puml::normalize_family(doc).expect("normalize gantt")
    else {
        panic!("expected timeline model");
    };
    assert_eq!(model.closed_ranges.len(), 1);
    assert_eq!(model.tasks[0].duration_days, 6);
}

#[test]
fn gantt_task_reference_starts_constraint_places_dependent_task() {
    let src = r#"@startgantt
Project starts 2026-05-01
[Design] requires 3 days
[Build] starts 2026-05-01 and requires 2 days
[Build] starts at [Design]'s end
@endgantt
"#;
    let svg = puml::render_source_to_svg(src).expect("gantt render");
    assert!(svg.contains("marker-end=\"url(#gantt-arrow)\""));
    let doc = parse_with_options(src, &ParseOptions::default()).expect("parse gantt");
    let NormalizedDocument::Timeline(model) = puml::normalize_family(doc).expect("normalize gantt")
    else {
        panic!("expected timeline model");
    };
    let design = model
        .tasks
        .iter()
        .find(|task| task.name == "Design")
        .expect("design task");
    let build = model
        .tasks
        .iter()
        .find(|task| task.name == "Build")
        .expect("build task");
    assert_eq!(build.start_day, design.start_day + design.duration_days);
}

#[test]
fn gantt_resource_allocation_is_visible_on_task_bar() {
    let src = r#"@startgantt
Project starts 2026-05-01
[Build] on {Bob:50%} requires 3 days
@endgantt
"#;
    let svg = puml::render_source_to_svg(src).expect("gantt render");
    assert!(svg.contains("class=\"gantt-resource-pill\""));
    assert!(svg.contains("class=\"gantt-resource\""));
    assert!(svg.contains("Bob:50%"));
}

#[test]
fn gantt_project_end_extends_axis_and_renders_boundary() {
    let src = r#"@startgantt
Project starts 2026-05-01
Project ends 2026-05-20
[Build] starts 2026-05-02 and requires 2 days
@endgantt
"#;
    let svg = puml::render_source_to_svg(src).expect("gantt render");
    assert!(svg.contains("class=\"gantt-project-end\""));
    assert!(svg.contains("Project ends 2026-05-20"));
    assert!(svg.contains("2026-05-20"));
}

#[test]
fn gantt_scale_single_day_calendar_and_multi_resource_semantics_render() {
    let src = r#"@startgantt
Project starts 2026-05-01
printscale weekly
2026-05-04 is closed
[Build] on {Alice, Bob} requires 2 days
@endgantt
"#;
    let svg = puml::render_source_to_svg(src).expect("gantt render");
    assert!(svg.contains("data-gantt-scale=\"weekly\""));
    assert!(svg.contains("class=\"gantt-scale\""));
    assert!(svg.contains("2026-05-04"));
    assert!(svg.contains("Alice, Bob"));
    let doc = parse_with_options(src, &ParseOptions::default()).expect("parse gantt");
    let NormalizedDocument::Timeline(model) = puml::normalize_family(doc).expect("normalize gantt")
    else {
        panic!("expected timeline model");
    };
    assert_eq!(model.scale.as_deref(), Some("weekly"));
    assert_eq!(model.closed_ranges.len(), 1);
    assert_eq!(model.tasks[0].resources, vec!["Alice", "Bob"]);
}

#[test]
fn gantt_closed_weekdays_render_bands_and_task_metadata() {
    let src = r#"@startgantt
Project starts 2026-05-01
saturdays are closed
sundays are closed
[Build] on {Alice, Bob} requires 2 days
@endgantt
"#;
    let svg = puml::render_source_to_svg(src).expect("gantt render");
    assert!(svg.contains("class=\"gantt-closed-weekday\""));
    assert!(svg.contains("data-gantt-day=\"2026-05-02\""));
    assert!(svg.contains("class=\"gantt-task\""));
    assert!(svg.contains("data-gantt-start=\"2026-05-01\""));
    assert!(svg.contains("data-gantt-duration=\"4\""));
    assert!(svg.contains("data-gantt-resources=\"Alice, Bob\""));
}

#[test]
fn gantt_reopened_calendar_resource_load_baseline_and_critical_metadata_render() {
    let src = r#"@startgantt
Project starts 2026-05-01
printscale weekly
saturdays are closed
sundays are closed
2026-05-02 is open
[Design] on {Alice:50%} requires 2 days
[Build] on {Bob:75%, Cara} requires 3 days
[Build] requires [Design]
[Design] baseline starts 2026-05-01 and lasts 2 days
[Build] is critical
[Release] happens on [Build]'s start
@endgantt
"#;
    let svg = puml::render_source_to_svg(src).expect("gantt render");
    assert!(svg.contains("class=\"gantt-open-range\""));
    assert!(svg.contains("Calendar: closed Saturday, Sunday; open 2026-05-02"));
    assert!(svg.contains("data-gantt-workload=\"2\""));
    assert!(svg.contains("data-gantt-load=\"Alice:50%\""));
    assert!(svg.contains("class=\"gantt-baseline\""));
    assert!(svg.contains("gantt-critical"));
    assert!(svg.contains("class=\"gantt-scale-tick\""));
    assert!(svg.contains(">Wk 2026-05-01<"));
    assert!(svg.contains("data-gantt-from=\"Design\" data-gantt-to=\"Build\""));
    let doc = parse_with_options(src, &ParseOptions::default()).expect("parse gantt");
    let NormalizedDocument::Timeline(model) = puml::normalize_family(doc).expect("normalize gantt")
    else {
        panic!("expected timeline model");
    };
    let design = model
        .tasks
        .iter()
        .find(|task| task.name == "Design")
        .expect("design task");
    let build = model
        .tasks
        .iter()
        .find(|task| task.name == "Build")
        .expect("build task");
    assert_eq!(model.open_ranges.len(), 1);
    assert_eq!(design.workload_days, 2);
    assert_eq!(design.duration_days, 5);
    assert_eq!(design.resource_allocations[0].name, "Alice");
    assert_eq!(design.resource_allocations[0].load_percent, Some(50));
    assert_eq!(build.start_day, design.start_day + design.duration_days);
    assert!(build.is_critical);
    assert!(design.baseline_start_day.is_some());
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
fn wbs_orientation_directives_affect_svg_layout_metadata() {
    let src = r#"@startwbs
left to right direction
* Launch
** Plan
*** Milestones
** Build
@endwbs
"#;
    let svg = puml::render_source_to_svg(src).expect("wbs render");
    assert!(svg.contains("data-wbs-orientation=\"left-to-right\""));
    assert!(svg.contains(">Launch<"));
    assert!(svg.contains(">Milestones<"));
}

#[test]
fn mindmap_and_wbs_node_color_tags_render_from_model() {
    let mindmap = r##"@startmindmap
*[#Orange] Root
**[#lightgreen] Delivery
left side
**[#fef3c7] Risks
@endmindmap
"##;
    let mindmap_svg = puml::render_source_to_svg(mindmap).expect("mindmap render");
    assert!(mindmap_svg.contains("fill=\"Orange\""));
    assert!(mindmap_svg.contains("fill=\"lightgreen\""));
    assert!(mindmap_svg.contains("fill=\"#fef3c7\""));

    let doc = parse_with_options(mindmap, &ParseOptions::default()).expect("parse mindmap");
    let NormalizedDocument::Family(model) = puml::normalize_family(doc).expect("normalize mindmap")
    else {
        panic!("expected family model");
    };
    assert_eq!(model.nodes[0].fill_color.as_deref(), Some("Orange"));
    assert_eq!(model.nodes[1].fill_color.as_deref(), Some("lightgreen"));
    assert_eq!(model.nodes[2].fill_color.as_deref(), Some("#fef3c7"));

    let wbs = r##"@startwbs
right to left direction
*[#dbeafe] Program
**[#pink] Stream A [%40]
**[#c7f9cc] Stream B [x]
@endwbs
"##;
    let wbs_svg = puml::render_source_to_svg(wbs).expect("wbs render");
    assert!(wbs_svg.contains("data-wbs-orientation=\"right-to-left\""));
    assert!(wbs_svg.contains("fill=\"#dbeafe\""));
    assert!(wbs_svg.contains("fill=\"pink\""));
    assert!(wbs_svg.contains("fill=\"#c7f9cc\""));
    assert!(wbs_svg.contains("[40%]"));
}

#[test]
fn chart_respects_axis_range_and_negative_values() {
    let src = r##"@startchart
bar chart
skinparam chartBackgroundColor #fff7ed
v-axis "Net" -10 --> 10
h-axis "Quarter" ["Q1", "Q2", "Q3"]
bar "Actual" [-6, 4, 9] #dc2626
bar "Plan" [-2, 2, 6] #2563eb
legend top left
@endchart
"##;
    let svg =
        puml::render_source_to_svg_for_family(src, DiagramFamily::Chart).expect("chart render");
    assert!(svg.contains("data-chart-type=\"bar\""));
    assert!(svg.contains("class=\"chart-zero-axis\""));
    assert!(svg.contains(">Net<"));
    assert!(svg.contains(">Quarter<"));
    assert!(svg.contains(">-10<"));
    assert!(svg.contains(">10<"));
    assert!(svg.contains(">-6<"));
    assert!(svg.contains("fill=\"#fff7ed\""));
    assert!(svg.contains("data-chart-legend=\"top-left\""));
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
