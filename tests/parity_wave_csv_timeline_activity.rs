use puml::parser::{parse_with_options, ParseOptions};
use puml::{DiagramFamily, NormalizedDocument};
use std::fs;

fn timeline_fixture(name: &str) -> String {
    format!(
        "{}/tests/fixtures/timeline/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn svg_attr(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let start = tag.find(&pat)? + pat.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn svg_viewbox_size(svg: &str) -> Option<(i32, i32)> {
    let svg_tag = svg.split("<svg ").nth(1)?.split('>').next()?;
    let viewbox = svg_attr(svg_tag, "viewBox")?;
    let mut parts = viewbox.split_whitespace();
    let _min_x = parts.next()?;
    let _min_y = parts.next()?;
    let width = parts.next()?.parse::<i32>().ok()?;
    let height = parts.next()?.parse::<i32>().ok()?;
    Some((width, height))
}

fn gantt_task_widths(svg: &str) -> Vec<i32> {
    svg.split("<rect class=\"gantt-task")
        .skip(1)
        .filter_map(|chunk| svg_attr(chunk, "width")?.parse::<i32>().ok())
        .collect()
}

fn svg_chunks_by_prefix<'a>(svg: &'a str, prefix: &str) -> Vec<&'a str> {
    svg.split(prefix).skip(1).collect()
}

fn svg_first_number_attr(tag: &str, key: &str) -> i32 {
    svg_attr(tag, key)
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or_else(|| panic!("expected numeric SVG attribute {key} in {tag}"))
}

fn milestone_center_x(tag: &str) -> i32 {
    let points = svg_attr(tag, "points").expect("milestone polygon should have points");
    points
        .split_whitespace()
        .next()
        .and_then(|pair| pair.split_once(','))
        .and_then(|(x, _)| x.parse::<i32>().ok())
        .expect("milestone first point should expose center x")
}

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
fn gantt_without_project_start_uses_absolute_milestone_as_epoch_anchor() {
    let src = fs::read_to_string(timeline_fixture("valid_gantt_render.puml")).unwrap();
    let svg = puml::render_source_to_svg(&src).expect("gantt render");

    assert!(
        !svg.contains("1970-"),
        "undated tasks should not force an epoch-to-absolute-date axis"
    );
    assert!(
        svg.contains("data-gantt-start=\"2026-05-01\""),
        "bare tasks should be anchored to the earliest explicit absolute date"
    );
    // With the default duration of 14 days (Wave 3-E #481), tasks span weeks so the
    // tick grid is weekly; check that the first tick falls within the task span (May–June 2026).
    assert!(
        svg.contains("data-gantt-tick-day=\"2026-05-08\""),
        "date range should stay near the resolved task and milestone span"
    );

    let (viewbox_w, viewbox_h) = svg_viewbox_size(&svg).expect("svg should include a viewBox");
    // viewBox width grew with the longer 14-day default duration (#481)
    assert!(
        viewbox_w >= 800,
        "expected canvas at least 800px wide, got {viewbox_w}"
    );
    assert!(
        viewbox_h <= 300,
        "regression fixture should render as a compact Gantt, got viewBox height {viewbox_h}"
    );

    let widths = gantt_task_widths(&svg);
    assert_eq!(widths.len(), 3, "fixture should render three task bars");
    assert!(
        widths.iter().all(|width| *width >= 120),
        "task bars should not collapse to tiny epoch-spanning widths: {widths:?}"
    );
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
fn gantt_legend_fixture_spans_phase_bars_and_keeps_launch_inside_grid() {
    let src = fs::read_to_string(format!(
        "{}/docs/examples/gantt/06_with_legend.puml",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("legend fixture");
    let svg = puml::render_source_to_svg(&src).expect("gantt render");

    assert!(
        svg.contains("data-gantt-tick-day="2026-09-01""),
        "date axis should include the milestone day at the grid boundary"
    );

    let task_tags = svg_chunks_by_prefix(&svg, "<rect class="gantt-task");
    let widths: Vec<i32> = task_tags
        .iter()
        .map(|tag| svg_first_number_attr(tag, "width"))
        .collect();
    assert_eq!(widths.len(), 3, "fixture should render three phase bars");
    assert!(
        widths.iter().all(|width| *width >= 120),
        "implicit phase bars should span the next dated anchor instead of collapsing: {widths:?}"
    );

    let chart_left = 204;
    let chart_width = 564;
    let grid_right = chart_left + chart_width;
    let milestone = svg_chunks_by_prefix(&svg, "<polygon class="gantt-milestone")
        .into_iter()
        .next()
        .expect("expected launch milestone");
    let launch_x = milestone_center_x(milestone);
    assert!(
        launch_x <= grid_right,
        "launch milestone should remain inside the grid: x={launch_x}, grid_right={grid_right}"
    );
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
printscale daily
saturdays are closed
sundays are closed
2026-05-04 to 2026-05-05 are closed
2026-05-02 is reopened
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
    assert!(svg.contains("Calendar: closed Saturday, Sunday"));
    assert!(svg.contains("open 2026-05-02"));
    assert!(svg.contains("2026-05-04 to 2026-05-05"));
    assert!(svg.contains("data-gantt-workload=\"2\""));
    assert!(svg.contains("data-gantt-load=\"Alice:50%\""));
    assert!(svg.contains("class=\"gantt-baseline\""));
    assert!(svg.contains("gantt-critical"));
    assert!(svg.contains("class=\"gantt-scale-tick\""));
    assert!(svg.contains(">2026-05-01<"));
    assert!(svg.contains("data-gantt-from=\"Design\" data-gantt-to=\"Build\""));

    let closed_days: Vec<String> =
        svg_chunks_by_prefix(&svg, "<rect class=\"gantt-closed-weekday\"")
            .into_iter()
            .filter_map(|chunk| svg_attr(chunk, "data-gantt-day"))
            .collect();
    assert!(
        !closed_days.iter().any(|day| day == "2026-05-02"),
        "explicitly reopened Saturday should not render as a closed weekday band: {closed_days:?}"
    );
    assert!(
        closed_days.iter().any(|day| day == "2026-05-03"),
        "unreopened Sunday should still render as a closed weekday band: {closed_days:?}"
    );

    let task_rects = svg_chunks_by_prefix(&svg, "<rect class=\"gantt-task");
    let build_rect = task_rects
        .iter()
        .find(|chunk| svg_attr(chunk, "data-gantt-load").as_deref() == Some("Bob:75%, Cara"))
        .expect("build task rect should include resource-load metadata");
    let build_x = svg_first_number_attr(build_rect, "x");
    let milestone = svg_chunks_by_prefix(&svg, "<polygon class=\"gantt-milestone")
        .into_iter()
        .next()
        .expect("release milestone should render as a polygon");
    assert_eq!(
        milestone_center_x(milestone),
        build_x,
        "milestone on [Build]'s start should share the build bar start x"
    );

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
    assert_eq!(model.closed_ranges.len(), 1);
    assert_eq!(design.workload_days, 2);
    assert_eq!(design.duration_days, 7);
    assert_eq!(design.resource_allocations[0].name, "Alice");
    assert_eq!(design.resource_allocations[0].load_percent, Some(50));
    assert_eq!(build.start_day, design.start_day + design.duration_days);
    assert_eq!(build.duration_days, 5);
    assert!(build.is_critical);
    assert!(design.baseline_start_day.is_some());
}

#[test]
fn gantt_separator_relative_constraints_resource_metadata_and_month_scale_render() {
    let src = r#"@startgantt
Project starts 2026-05-01
Project ends 2026-07-01
printscale monthly
saturdays are closed
sundays are closed
2026-05-04 to 2026-05-05 are closed
2026-05-09 is reopened
[Design] on {Alice:50%} requires 2 days
-- Build phase --
[Build] on {Bob:75%, Cara} requires 1 month
[Build] starts 2 days after [Design]'s end
Separator just 1 day before [Build]'s start
[Launch] happens on [Build]'s end
@endgantt
"#;
    let svg = puml::render_source_to_svg(src).expect("gantt render");
    assert!(svg.contains("data-gantt-scale=\"monthly\""));
    assert!(svg.contains("data-gantt-resource-count=\"3\""));
    assert!(svg.contains("data-gantt-separator-count=\"2\""));
    assert!(svg.contains("class=\"gantt-separator\""));
    assert!(svg.contains("Build phase"));
    assert!(svg.contains("data-gantt-load=\"Bob:75%, Cara\""));
    assert!(svg.contains("class=\"gantt-open-range\""));
    assert!(svg.contains("class=\"gantt-closed-range\""));
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
    assert_eq!(model.scale.as_deref(), Some("monthly"));
    assert_eq!(model.separators.len(), 2);
    assert_eq!(build.workload_days, 30);
    assert_eq!(build.start_day, design.start_day + design.duration_days + 2);
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
    // Bug #584: (endwhile) is a layout-only marker — it must NOT appear as a
    // visible process node. Same suppression as (else)/(endif) from Wave 3-D #533.
    assert!(
        !svg.contains("(endwhile)"),
        "(endwhile) must not render as a visible text node"
    );
    // Wave 3-D (#533) intentionally suppresses "(else)" and "(endif)" literal canvas text
    assert!(
        svg.contains("done"),
        "then-branch action must appear in SVG"
    );
    assert!(
        svg.contains("retry"),
        "else-branch action must appear in SVG"
    );
}

#[test]
fn activity_if_else_both_branches_present_in_scene_and_svg() {
    // Regression test for issue #239: the else-branch was never rendered.
    // Both then- and else-branch nodes must appear as distinct shapes in the
    // output SVG, and arrows must diverge from the decision diamond.
    let src = r#"@startuml
start
:Check;
if (ok?) then (yes)
  :HandleOk;
else (no)
  :HandleErr;
endif
stop
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("activity if/else render");

    // Both branch labels must be present
    assert!(
        svg.contains("HandleOk"),
        "then-branch action missing from SVG"
    );
    assert!(
        svg.contains("HandleErr"),
        "else-branch action missing from SVG"
    );

    // Wave 3-D (#533) intentionally suppresses "(else)" and "(endif)" literal canvas text;
    // verify branching by checking that condition label is rendered and arrows diverge.

    // There must be at least two distinct x-coordinates in the arrows, proving
    // that the diagram is not purely linear (i.e., branching exists).
    let arrow_xs: std::collections::HashSet<i32> = {
        let mut xs = std::collections::HashSet::new();
        // Match <line x1="..." and x2="..."
        let mut rest = svg.as_str();
        while let Some(pos) = rest.find("<line x1=\"") {
            rest = &rest[pos + 10..];
            if let Some(end) = rest.find('"') {
                if let Ok(v) = rest[..end].parse::<i32>() {
                    xs.insert(v);
                }
            }
        }
        xs
    };
    assert!(
        arrow_xs.len() >= 2,
        "expected arrows at >= 2 distinct x positions (branching), got: {:?}",
        arrow_xs
    );
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
    assert!(svg.contains("class=\"state-transition\""));
    assert!(svg.contains("data-state-from=\"A\" data-state-to=\"A\""));
    // Pseudo-state radius shifted in Wave 4-B refactor (was 14, now 12 outer
    // with inner 8 for the <<end>> marker). Assert on presence of distinct radii.
    assert!(svg.contains("r=\"12\""));
    assert!(svg.contains("r=\"8\""));
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

#[test]
fn timing_participant_oriented_clock_offset_and_alias_states_render() {
    let src = r#"@startuml
clock "Clock" as CLK with period 4 pulse 1 offset 2
binary "Enable" as EN
robust BUS
@EN
0 is down
+4 is up
+4 is off
@BUS
0 is {Idle}
4 is {Run}
@CLK*0
CLK is high
@CLK*2
CLK is low
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("timing render");
    assert!(svg.contains("data-timing-offset=\"2\""));
    assert!(svg.contains("data-timing-period=\"4\""));
    assert!(svg.contains("Enable"));
    assert!(svg.contains("Idle"));
    assert!(svg.contains("Run"));
    assert!(svg.contains(">high<"));
    assert!(svg.contains(">low<"));
}
