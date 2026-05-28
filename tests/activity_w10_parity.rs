/// Wave-10 batch B: activity diagram — arrow styling + action colors + partition colors.
///
/// These tests exercise:
///   - `-> label;`                           labeled arrow rendered on edge
///   - `:action; #red`                       suffix fill-color on action box
///   - `#LightBlue:action;`                  prefix fill-color on action box (alternate syntax)
///   - `-[#green,dashed]->`                  dashed styled arrow renders
///   - `-[#blue]->`                          inline color on arrow stroke
///   - `partition "Name" #LightGreen { }     partition fill color on swimlane band

// ---------------------------------------------------------------------------
// activity_arrow_label_renders_on_edge
// ---------------------------------------------------------------------------

#[test]
fn activity_arrow_label_renders_on_edge() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
:Step A;
-> go there;
:Step B;
stop
@enduml
"#,
    )
    .expect("labeled arrow activity should render without error");

    // The label text must appear somewhere in the SVG.
    assert!(
        svg.contains(">go there<"),
        "labeled arrow text 'go there' must appear in the SVG output"
    );
}

// ---------------------------------------------------------------------------
// activity_colored_action_applies_fill
// ---------------------------------------------------------------------------

#[test]
fn activity_colored_action_applies_fill() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
:colored step; #red
stop
@enduml
"#,
    )
    .expect("action with suffix fill-color should render without error");

    // The fill color must appear in the rect/shape that renders the action box.
    assert!(
        svg.contains("fill=\"red\"")
            || svg.contains("fill=\"#red\"")
            || svg.contains("fill=\"#ff0000\"")
            || svg.contains("fill=\"red"),
        "action box fill must be red; got SVG: {}",
        &svg[..svg.len().min(2000)]
    );
    // The action label text must still appear.
    assert!(
        svg.contains(">colored step<"),
        "action label 'colored step' must render as text"
    );
}

// ---------------------------------------------------------------------------
// activity_colored_action_prefix_syntax_applies_fill
// ---------------------------------------------------------------------------

#[test]
fn activity_colored_action_prefix_syntax_applies_fill() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
#LightBlue:another action;
stop
@enduml
"#,
    )
    .expect("action with prefix fill-color should render without error");

    // LightBlue fill must appear.
    assert!(
        svg.contains("fill=\"LightBlue\"") || svg.contains("fill=\"lightblue\""),
        "action box fill must be LightBlue (case may vary); SVG excerpt: {}",
        &svg[..svg.len().min(2000)]
    );
    // Label must appear.
    assert!(
        svg.contains(">another action<"),
        "action label 'another action' must render"
    );
}

// ---------------------------------------------------------------------------
// activity_styled_arrow_dashed_renders
// ---------------------------------------------------------------------------

#[test]
fn activity_styled_arrow_dashed_renders() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
:Step X;
-[#green,dashed]-> dashed;
:Step Y;
stop
@enduml
"#,
    )
    .expect("dashed styled arrow activity should render without error");

    // stroke-dasharray must be present for the dashed style.
    assert!(
        svg.contains("stroke-dasharray"),
        "dashed arrow must include stroke-dasharray attribute"
    );
    // The label text must appear.
    assert!(
        svg.contains(">dashed<"),
        "dashed arrow label 'dashed' must appear in SVG"
    );
}

// ---------------------------------------------------------------------------
// activity_styled_arrow_inline_color_applies
// ---------------------------------------------------------------------------

#[test]
fn activity_styled_arrow_inline_color_applies() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
:Step A;
-[#blue]-> labeled blue edge;
:Step B;
stop
@enduml
"#,
    )
    .expect("inline-color arrow activity should render without error");

    // The blue color must appear on a stroke somewhere in the SVG.
    assert!(
        svg.contains("stroke=\"blue\"") || svg.contains("stroke=\"#blue\""),
        "arrow stroke must be blue; got SVG excerpt: {}",
        &svg[..svg.len().min(2000)]
    );
    // The label must appear.
    assert!(
        svg.contains(">labeled blue edge<"),
        "arrow label 'labeled blue edge' must appear"
    );
}

// ---------------------------------------------------------------------------
// activity_partition_color_fills_swimlane_band
// ---------------------------------------------------------------------------

#[test]
fn activity_partition_color_fills_swimlane_band() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
partition "My partition" #LightGreen {
  :step;
  :next;
}
stop
@enduml
"#,
    )
    .expect("partition with fill color should render without error");

    // The partition fill color must appear in the lane background rect.
    assert!(
        svg.contains("fill=\"LightGreen\"") || svg.contains("LightGreen"),
        "partition fill color LightGreen must appear in SVG"
    );
}
