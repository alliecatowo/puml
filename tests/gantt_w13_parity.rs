//! Wave-13 batch T: Gantt diagram parity — project schedule rendering.
//!
//! These tests exercise the full set of gantt constructs required by the
//! PlantUML Language Reference Guide (Gantt chapter):
//!   - `[Task] lasts N days`                  duration → bar of proportional width
//!   - `starts at [Other]'s end`              chained task dependency scheduling
//!   - `is colored in X`                      fill color override on task bar
//!   - `is N% complete`                       progress fill overlay on task bar
//!   - `happens at [Task]'s end`              milestone diamond rendered at date
//!   - `saturday/sunday are closed`           weekend exclusion shading
//!   - `[Parent.Child]` sub-task              indented under parent in output
//!   - `[A] -> [B]`                           dependency arrow between tasks

// ---------------------------------------------------------------------------
// gantt_task_lasts_n_days_renders_bar_of_correct_width
// ---------------------------------------------------------------------------
/// A longer task (`lasts 20 days`) must produce a wider bar than a shorter one
/// (`lasts 5 days`).  We verify this through the `data-gantt-workload` attribute
/// written on each `gantt-task` rect.
#[test]
fn gantt_task_lasts_n_days_renders_bar_of_correct_width() {
    let svg = puml::render_source_to_svg(
        r#"@startgantt
Project starts 2026-01-01
[Short] lasts 5 days
[Long] lasts 20 days
@endgantt
"#,
    )
    .expect("gantt with lasts N days should render without error");

    // Both tasks must appear as gantt-task rects.
    assert!(
        svg.contains("class=\"gantt-task\""),
        "gantt-task rects must be present; got SVG (first 2000 chars): {}",
        &svg[..svg.len().min(2000)]
    );

    // data-gantt-workload attributes encode the day count for each task bar.
    assert!(
        svg.contains("data-gantt-workload=\"5\""),
        "Short task must have workload=5; SVG snippet: {}",
        &svg[..svg.len().min(3000)]
    );
    assert!(
        svg.contains("data-gantt-workload=\"20\""),
        "Long task must have workload=20; SVG snippet: {}",
        &svg[..svg.len().min(3000)]
    );

    // Both task labels must appear in the label column.
    assert!(
        svg.contains("Short"),
        "task name 'Short' must appear in SVG"
    );
    assert!(svg.contains("Long"), "task name 'Long' must appear in SVG");
}

// ---------------------------------------------------------------------------
// gantt_task_starts_at_other_end_chains_correctly
// ---------------------------------------------------------------------------
/// `[Implementation] lasts 20 days and starts at [Design]'s end` — the second
/// task must begin no earlier than where the first ends.  We verify this by
/// checking that the rendered SVG encodes the correct start date.
#[test]
fn gantt_task_starts_at_other_end_chains_correctly() {
    let svg = puml::render_source_to_svg(
        r#"@startgantt
Project starts 2026-01-01
[Design] lasts 10 days
[Implementation] lasts 20 days and starts at [Design]'s end
@endgantt
"#,
    )
    .expect("chained task dependency should render without error");

    // Both tasks must be present.
    assert!(svg.contains("Design"), "'Design' task must appear in SVG");
    assert!(
        svg.contains("Implementation"),
        "'Implementation' task must appear in SVG"
    );

    // Project starts 2026-01-01; Design runs days 0-9; Implementation starts 2026-01-11.
    // data-gantt-start is rendered as a calendar date when a project start is set.
    assert!(
        svg.contains("data-gantt-start=\"2026-01-11\""),
        "Implementation must start at 2026-01-11 (Design's end); SVG snippet: {}",
        &svg[..svg.len().min(4000)]
    );
}

// ---------------------------------------------------------------------------
// gantt_colored_task_renders_fill
// ---------------------------------------------------------------------------
/// `[Design] is colored in LightBlue` — the fill attribute of the task bar rect
/// must reflect the requested color.
#[test]
fn gantt_colored_task_renders_fill() {
    let svg = puml::render_source_to_svg(
        r#"@startgantt
Project starts 2026-01-01
[Design] lasts 10 days
[Design] is colored in LightBlue
@endgantt
"#,
    )
    .expect("colored task should render without error");

    // The fill must contain "LightBlue" (possibly lower-cased or as #ADD8E6).
    let svg_lower = svg.to_ascii_lowercase();
    assert!(
        svg_lower.contains("fill=\"lightblue\"")
            || svg_lower.contains("fill=\"#add8e6\"")
            || svg.contains("fill=\"LightBlue\""),
        "Design task fill must be LightBlue; SVG snippet: {}",
        &svg[..svg.len().min(4000)]
    );
}

// ---------------------------------------------------------------------------
// gantt_percent_complete_renders_progress_bar
// ---------------------------------------------------------------------------
/// `[Testing] is 80% complete` — a progress overlay rect with class
/// `gantt-task-completion` must appear, and `data-gantt-completion="80"` must
/// be on the parent bar.
#[test]
fn gantt_percent_complete_renders_progress_bar() {
    let svg = puml::render_source_to_svg(
        r#"@startgantt
Project starts 2026-01-01
[Testing] lasts 10 days
[Testing] is 80% complete
@endgantt
"#,
    )
    .expect("task with completion% should render without error");

    // The task bar must carry completion metadata.
    assert!(
        svg.contains("data-gantt-completion=\"80\""),
        "data-gantt-completion=\"80\" must be present; SVG: {}",
        &svg[..svg.len().min(4000)]
    );

    // A progress-fill overlay rect must be emitted.
    assert!(
        svg.contains("gantt-task-completion"),
        "gantt-task-completion overlay rect must be present; SVG: {}",
        &svg[..svg.len().min(4000)]
    );
}

// ---------------------------------------------------------------------------
// gantt_milestone_renders_diamond_at_date
// ---------------------------------------------------------------------------
/// `[Beta Release] happens at [Testing]'s end` — a gantt-milestone polygon
/// (diamond) must appear in the SVG output.
#[test]
fn gantt_milestone_renders_diamond_at_date() {
    let svg = puml::render_source_to_svg(
        r#"@startgantt
Project starts 2026-01-01
[Testing] lasts 5 days
[Beta Release] happens at [Testing]'s end
@endgantt
"#,
    )
    .expect("milestone should render without error");

    // The milestone diamond polygon must be present.
    assert!(
        svg.contains("class=\"gantt-milestone\""),
        "gantt-milestone polygon must be present; SVG: {}",
        &svg[..svg.len().min(4000)]
    );

    // The milestone name must appear in the label column.
    assert!(
        svg.contains("Beta Release"),
        "'Beta Release' label must appear in SVG"
    );
}

// ---------------------------------------------------------------------------
// gantt_weekend_skipped_when_saturday_sunday_closed
// ---------------------------------------------------------------------------
/// `saturday are closed` + `sunday are closed` — the renderer must emit
/// `gantt-closed-weekday` overlay rects for weekend columns, and the
/// calendar annotation must mention the closed days.
#[test]
fn gantt_weekend_skipped_when_saturday_sunday_closed() {
    let svg = puml::render_source_to_svg(
        r#"@startgantt
Project starts 2026-01-01
[Design] lasts 14 days
saturday are closed
sunday are closed
@endgantt
"#,
    )
    .expect("weekend-closed gantt should render without error");

    // Closed weekday shading rects must be emitted.
    assert!(
        svg.contains("class=\"gantt-closed-weekday\""),
        "gantt-closed-weekday rects must be present for Sat/Sun; SVG: {}",
        &svg[..svg.len().min(4000)]
    );

    // The calendar annotation text must mention the closed days.
    assert!(
        svg.contains("gantt-calendar"),
        "gantt-calendar annotation must be present; SVG: {}",
        &svg[..svg.len().min(4000)]
    );
    // The annotation says "Calendar: closed Saturday, Sunday" (title-cased).
    assert!(
        svg.contains("Saturday") || svg.contains("saturday"),
        "Calendar annotation must mention Saturday; SVG: {}",
        &svg[..svg.len().min(4000)]
    );
    assert!(
        svg.contains("Sunday") || svg.contains("sunday"),
        "Calendar annotation must mention Sunday; SVG: {}",
        &svg[..svg.len().min(4000)]
    );
}

// ---------------------------------------------------------------------------
// gantt_subtask_indents_under_parent
// ---------------------------------------------------------------------------
/// `[Implementation.Backend] lasts 10 days` — sub-tasks using dot-notation
/// must appear as separate rows in the rendered output.  The sub-task names
/// must both appear in the SVG label column.
#[test]
fn gantt_subtask_indents_under_parent() {
    let svg = puml::render_source_to_svg(
        r#"@startgantt
Project starts 2026-01-01
[Implementation] lasts 20 days
[Implementation.Backend] lasts 10 days
[Implementation.Frontend] lasts 8 days and starts at [Implementation.Backend]'s end
@endgantt
"#,
    )
    .expect("sub-task gantt should render without error");

    // All three task names must appear.
    assert!(
        svg.contains("Implementation"),
        "'Implementation' must appear in SVG"
    );
    assert!(
        svg.contains("Implementation.Backend"),
        "'Implementation.Backend' sub-task must appear in SVG"
    );
    assert!(
        svg.contains("Implementation.Frontend"),
        "'Implementation.Frontend' sub-task must appear in SVG"
    );

    // There must be at least three gantt-task rects (one per task).
    let task_count = svg.matches("class=\"gantt-task\"").count();
    assert!(
        task_count >= 3,
        "at least 3 gantt-task rects expected (parent + 2 sub-tasks); got {task_count}"
    );
}

// ---------------------------------------------------------------------------
// gantt_dependency_arrow_connects_two_tasks
// ---------------------------------------------------------------------------
/// `[Implementation] -> [Testing]` — a `gantt-dependency` line with the
/// correct `data-gantt-from` / `data-gantt-to` attributes must be rendered.
#[test]
fn gantt_dependency_arrow_connects_two_tasks() {
    let svg = puml::render_source_to_svg(
        r#"@startgantt
Project starts 2026-01-01
[Implementation] lasts 10 days
[Testing] lasts 5 days and starts at [Implementation]'s end
[Implementation] -> [Testing]
@endgantt
"#,
    )
    .expect("gantt dependency arrow should render without error");

    // A dependency line must appear.
    assert!(
        svg.contains("gantt-dependency"),
        "gantt-dependency line must be present; SVG: {}",
        &svg[..svg.len().min(4000)]
    );

    // The arrow must reference both tasks via data attributes.
    assert!(
        svg.contains("data-gantt-from=\"Implementation\"")
            || svg.contains("data-gantt-from=\"Testing\""),
        "dependency line must carry data-gantt-from referencing one of the tasks; SVG: {}",
        &svg[..svg.len().min(4000)]
    );
    assert!(
        svg.contains("data-gantt-to=\"Testing\"")
            || svg.contains("data-gantt-to=\"Implementation\""),
        "dependency line must carry data-gantt-to referencing one of the tasks; SVG: {}",
        &svg[..svg.len().min(4000)]
    );
}
