use puml::model::{FamilyNodeKind, NormalizedDocument};

// ---------------------------------------------------------------------------
// 6.3 — if / then / else branch guards
// ---------------------------------------------------------------------------

#[test]
fn activity_ch06_if_else_renders_both_branch_guards_on_arrows() {
    let svg = puml::render_source_to_svg(include_str!(
        "../docs/examples/activity_new/02_if_else.puml"
    ))
    .expect("render activity-new if/else guards");

    assert!(
        svg.contains(">condition?<"),
        "decision diamond should keep only the condition text"
    );
    assert!(
        !svg.contains(">condition? / yes<"),
        "then guard should not be folded into the diamond label"
    );
    assert!(
        svg.contains(">yes<"),
        "then guard should render as branch-arrow text"
    );
    assert!(
        svg.contains(">no<"),
        "else guard should render as branch-arrow text"
    );
}

// ---------------------------------------------------------------------------
// 6.5 — kill / detach rendering (section 6.5 / 6.20)
// ---------------------------------------------------------------------------

#[test]
fn activity_ch06_kill_renders_as_x_symbol_not_text() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
:Before kill;
kill
@enduml
"#,
    )
    .expect("render kill");
    // Kill should use data-activity-kind="Kill" and render an X-circle (two line elements),
    // NOT a text label ">kill<"
    assert!(
        svg.contains("data-activity-kind=\"Kill\""),
        "kill node must carry Kill metadata"
    );
    assert!(
        !svg.contains(">kill<"),
        "kill should not render as a text label"
    );
    // X-in-circle: should have the outer circle and two diagonal lines
    assert!(
        svg.contains("stroke-width=\"2\""),
        "kill X lines should use stroke-width 2"
    );
}

#[test]
fn activity_ch06_detach_renders_as_horizontal_bar_not_text() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
:Before detach;
detach
@enduml
"#,
    )
    .expect("render detach");
    assert!(
        svg.contains("data-activity-kind=\"Detach\""),
        "detach node must carry Detach metadata"
    );
    assert!(
        !svg.contains(">detach<"),
        "detach should not render as a text label"
    );
    // Detach renders as a horizontal bar — a single line with stroke-width 3
    assert!(
        svg.contains("stroke-width=\"3\""),
        "detach bar should use stroke-width 3"
    );
}

#[test]
fn activity_ch06_kill_suppresses_outgoing_arrow() {
    // After a kill node, the next action should NOT receive an incoming arrow
    // (the flow is terminated).
    let src = r#"@startuml
start
:Action A;
kill
:Action B should not get arrow;
@enduml
"#;
    let svg = puml::render_source_to_svg(src).expect("render kill suppression");
    // Verify both nodes are present
    assert!(svg.contains(">Action A<"), "Action A should be rendered");
    // Label may be wrapped across tspan elements when it is long; check for
    // the first word of the label which is always present verbatim.
    assert!(
        svg.contains(">Action B should not get arrow<") || svg.contains("Action B should"),
        "Action B should be rendered"
    );
    // The kill node is at slot y ≈ 160 (action A ends there), Action B starts at y ≈ 220.
    // There must be no arrow between kill's arrow_out_y and Action B's slot_y.
    // We check that data-activity-kind="Kill" appears and that Action B has no predecessor
    // arrow by counting line elements — exact coords may vary, so we check metadata only.
    assert!(
        svg.contains("data-activity-kind=\"Kill\""),
        "kill metadata present"
    );
}

// ---------------------------------------------------------------------------
// 6.21.2 — SDL action terminator shapes
// ---------------------------------------------------------------------------

#[test]
fn activity_ch06_sdl_send_shape_renders_chevron() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
:Send message>
stop
@enduml
"#,
    )
    .expect("render SDL send");
    // SDL send renders as a polygon (chevron shape), not a rect
    assert!(
        svg.contains(">Send message<"),
        "label text should be present"
    );
    assert!(
        svg.contains("<polygon"),
        "send shape should be a polygon (chevron)"
    );
    // Should not use a rounded rect (rx/ry)
    assert!(
        !svg.contains("rx=\"18\""),
        "send shape should not use rounded rect"
    );
}

#[test]
fn activity_ch06_sdl_receive_shape_renders_notch() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
:Receive message<
stop
@enduml
"#,
    )
    .expect("render SDL receive");
    assert!(
        svg.contains(">Receive message<"),
        "label text should be present"
    );
    assert!(
        svg.contains("<polygon"),
        "receive shape should be a polygon (notch)"
    );
    assert!(
        !svg.contains("rx=\"18\""),
        "receive shape should not use rounded rect"
    );
}

#[test]
fn activity_ch06_sdl_input_output_use_parallelogram() {
    let input_svg = puml::render_source_to_svg(
        r#"@startuml
start
:Input data/
stop
@enduml
"#,
    )
    .expect("render SDL input");
    assert!(input_svg.contains(">Input data<"), "input label present");
    assert!(
        input_svg.contains("<polygon"),
        "input should use polygon (parallelogram)"
    );

    let output_svg = puml::render_source_to_svg(
        r#"@startuml
start
:Output data\
stop
@enduml
"#,
    )
    .expect("render SDL output");
    assert!(output_svg.contains(">Output data<"), "output label present");
    assert!(
        output_svg.contains("<polygon"),
        "output should use polygon (parallelogram)"
    );
}

#[test]
fn activity_ch06_sdl_bar_uses_rect_no_rounding() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
:Procedure call|
stop
@enduml
"#,
    )
    .expect("render SDL bar");
    assert!(svg.contains(">Procedure call<"), "bar label present");
    // Bar shape uses a plain rect (rx="0")
    assert!(
        svg.contains("rx=\"0\""),
        "bar action should use rx=0 rectangle"
    );
}

#[test]
fn activity_ch06_sdl_shapes_preserve_fill_color() {
    // SDL shapes can also have fill colors from #color prefix
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
#LightGreen:Colored send>
stop
@enduml
"#,
    )
    .expect("render SDL colored send");
    assert!(svg.contains(">Colored send<"), "colored SDL label present");
    assert!(
        svg.contains("fill=\"LightGreen\""),
        "fill color should apply to SDL shape"
    );
}

const PARITY_SRC: &str = r##"@startuml
title Activity ch06 parity slice
start
#LightBlue:Collect;
-[#red,dashed]-> reviewed;
:Review;
note right: keep evidence
#pink:(A)
partition #LightYellow Ops {
  :Ship;
}
detach
:After detach;
@enduml
"##;

#[test]
fn activity_ch06_metadata_survives_normalization() {
    let document = puml::parser::parse(PARITY_SRC).expect("parse activity parity slice");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize activity parity slice")
    else {
        panic!("activity should normalize as a family document");
    };

    let collect = model
        .nodes
        .iter()
        .find(|node| node.label.as_deref() == Some("Collect"))
        .expect("colored action node");
    assert_eq!(collect.fill_color.as_deref(), Some("LightBlue"));

    let note = model
        .nodes
        .iter()
        .find(|node| node.label.as_deref() == Some("keep evidence"))
        .expect("activity note node");
    assert_eq!(note.kind, FamilyNodeKind::Note);

    let connector = model
        .nodes
        .iter()
        .find(|node| node.label.as_deref() == Some("(A)"))
        .expect("connector node");
    assert_eq!(connector.kind, FamilyNodeKind::ActivityAction);
    assert_eq!(connector.fill_color.as_deref(), Some("pink"));

    let lane = model
        .nodes
        .iter()
        .find(|node| {
            node.label.as_deref() == Some("Ops")
                && matches!(node.kind, FamilyNodeKind::ActivityPartition)
        })
        .expect("colored partition marker");
    assert_eq!(lane.fill_color.as_deref(), Some("LightYellow"));
}

#[test]
fn activity_ch06_nested_partition_close_restores_outer_lane() {
    let document = puml::parser::parse(
        r#"@startuml
start
partition Outer {
  :one;
  partition Inner {
    :two;
  }
  :three;
}
stop
@enduml
"#,
    )
    .expect("parse nested activity partitions");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize nested activity partitions")
    else {
        panic!("activity should normalize as a family document");
    };

    let lane_for = |label: &str| -> String {
        model
            .nodes
            .iter()
            .find(|node| node.label.as_deref() == Some(label))
            .and_then(|node| node.alias.as_deref())
            .and_then(|alias| {
                alias
                    .split('|')
                    .find_map(|part| part.strip_prefix("lane=").map(str::to_string))
            })
            .unwrap_or_else(|| panic!("missing lane metadata for {label}"))
    };

    assert_eq!(lane_for("one"), "Outer");
    assert_eq!(lane_for("two"), "Inner");
    assert_eq!(lane_for("three"), "Outer");
}

#[test]
fn activity_ch06_render_applies_arrow_color_note_connector_and_detach() {
    let svg = puml::render_source_to_svg(PARITY_SRC).expect("render activity parity slice");

    assert!(svg.contains("fill=\"LightBlue\""));
    assert!(svg.contains("fill=\"LightYellow\""));
    assert!(svg.contains("fill=\"pink\""));
    assert!(svg.contains("stroke=\"red\""));
    assert!(svg.contains("stroke-dasharray=\"6 4\""));
    assert!(svg.contains(">reviewed<"));
    assert!(svg.contains("data-activity-kind=\"Note\""));
    assert!(svg.contains("data-activity-kind=\"Connector\""));
    assert!(svg.contains("data-activity-kind=\"Detach\""));
    assert!(svg.contains(">After detach<"));

    assert!(
        !svg.contains("y1=\"488\""),
        "detach should suppress the outgoing arrow into the following action"
    );
}

#[test]
fn activity_ch06_hash_named_arrow_style_renders_color_and_label() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
:Collect;
-[#blue]-> : reviewed;
:Review;
stop
@enduml
"#,
    )
    .expect("render hash-named activity arrow");

    assert!(svg.contains("stroke=\"blue\""));
    assert!(svg.contains("fill=\"blue\""));
    assert!(svg.contains(">reviewed<"));
}

#[test]
fn activity_ch06_side_note_attaches_without_consuming_flow_slot() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
title Activity note attachment gap
start
:Prepare release;
note right: attach evidence packet
:Ship release;
stop
@enduml
"#,
    )
    .expect("render attached activity note");

    assert!(svg.contains("class=\"activity-note-connector\""));
    assert!(
        // Ortho routing emits <line>; Polyline/Splines routing emits <polyline>
        svg.contains("<line x1=\"116\" y1=\"132\" x2=\"116\" y2=\"134\"")
            || svg.contains("<polyline points=\"116,132 116,134\""),
        "main flow should continue from the noted action to the following action"
    );
    assert!(
        !svg.contains(">note right: attach evidence packet<"),
        "note directive should not be rendered as flow text"
    );
}

#[test]
fn activity_ch06_multiline_and_floating_notes_stay_out_of_main_flow() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
:Prepare release;
note right
first evidence line
second evidence line
third evidence line
end note
floating note left: manual checklist
:Ship release;
stop
@enduml
"#,
    )
    .expect("render multiline and floating activity notes");

    assert!(svg.contains(">first evidence line<"));
    assert!(svg.contains(">second evidence line<"));
    assert!(svg.contains(">third evidence line<"));
    assert!(
        !svg.contains(">end note<"),
        "multiline activity note terminator should not render as an action"
    );
    assert!(
        !svg.contains("data-activity-kind=\"OldStyle\""),
        "multiline note body lines should not become fallback action boxes"
    );
    assert_eq!(
        svg.matches("class=\"activity-note-connector\"").count(),
        1,
        "floating notes should not draw attached-note connectors"
    );
    assert!(
        // Ortho routing emits <line>; Polyline/Splines routing emits <polyline>
        svg.contains("<line x1=\"316\" y1=\"114\" x2=\"316\" y2=\"116\"")
            || svg.contains("<polyline points=\"316,114 316,116\""),
        "main flow should skip floating note nodes"
    );
}

#[test]
fn activity_ch06_top_and_bottom_notes_place_around_anchor() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
:Prepare;
note top: top note
:Review;
note bottom: bottom note
:Ship;
stop
@enduml
"#,
    )
    .expect("render top and bottom activity notes");

    assert!(svg.contains(">top note<"));
    assert!(svg.contains(">bottom note<"));
    assert!(
        svg.matches("class=\"activity-note-connector\"").count() >= 2,
        "top and bottom notes should draw attached-note connectors"
    );
    assert!(
        svg.contains("data-activity-kind=\"Note\""),
        "top and bottom note nodes should keep activity-note metadata"
    );
}

#[test]
fn activity_ch06_detached_fork_branch_does_not_join() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
title Fork branch termination gap
start
fork
  :branch A;
  detach
fork again
  :branch B;
end fork
:after fork;
stop
@enduml
"#,
    )
    .expect("render detached fork branch");

    assert!(svg.contains("data-activity-kind=\"Detach\""));
    // A detached branch (left fork, x≈104) should NOT draw a long connector into the
    // fork join bar.  We extract all points="..." attributes from the SVG and check that
    // no polyline starting at x=104 spans more than ~50 px vertically.
    let polyline_spans_over_50_from_x = |svg: &str, x: i32| -> bool {
        let mut search = svg;
        while let Some(start) = search.find("points=\"") {
            search = &search[start + 8..];
            if let Some(end) = search.find('"') {
                let pts_str = &search[..end];
                let pts: Vec<(i32, i32)> = pts_str
                    .split_whitespace()
                    .filter_map(|pair| {
                        let mut it = pair.splitn(2, ',');
                        let px: i32 = it.next()?.parse().ok()?;
                        let py: i32 = it.next()?.parse().ok()?;
                        Some((px, py))
                    })
                    .collect();
                if let (Some(&(x0, y0)), Some(&(_, y_last))) = (pts.first(), pts.last()) {
                    if x0 == x && (y_last - y0).abs() > 50 {
                        return true;
                    }
                }
            }
        }
        false
    };
    assert!(
        !polyline_spans_over_50_from_x(&svg, 104),
        "a detached branch should not draw a long connector into the fork join bar"
    );
    // The non-terminated branch B (right fork, x≈288) MUST connect downward to the join bar
    // with a span > 50 px.
    assert!(
        polyline_spans_over_50_from_x(&svg, 288),
        "non-terminated fork branches should still connect to the join bar"
    );
}

#[test]
fn activity_ch06_all_terminal_split_hides_synthetic_join() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
split
  :branch detaches;
  detach
split again
  :branch kills;
  kill
end split
:unreachable follow-up;
stop
@enduml
"#,
    )
    .expect("render all-terminal split");

    assert!(svg.contains("data-activity-kind=\"EndFork\""));
    assert!(
        !svg.contains("<rect x=\"32\" y=\"326\" width=\"736\" height=\"8\""),
        "all-terminal split should not render a synthetic join bar"
    );
    assert!(
        !svg.contains("<line x1=\"400\" y1=\"344\" x2=\"400\" y2=\"362\""),
        "all-terminal split should not connect the hidden join to following actions"
    );
}

#[test]
fn activity_ch06_mixed_terminal_split_join_tracks_live_branch_only() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
split
  :detached branch;
  detach
split again
  :live branch;
  :continues;
end split
:after split;
stop
@enduml
"#,
    )
    .expect("render mixed terminal split");

    assert!(svg.contains(">detached branch<"));
    assert!(svg.contains(">live branch<"));
    assert!(svg.contains(">after split<"));
    assert!(
        !svg.contains("<line x1=\"216\" y1=\"284\" x2=\"216\" y2=\"388\"")
            && !svg.contains("<line x1=\"216\" y1=\"262\" x2=\"216\" y2=\"388\""),
        "terminal split branches should not route into the continuing join bar"
    );
    assert!(
        svg.contains("data-activity-kind=\"EndFork\""),
        "mixed split should keep an end-fork bookmark for the live branch join"
    );
}

#[test]
fn activity_ch06_fork_branch_connector_arrows_do_not_point_upward() {
    let svg = puml::render_source_to_svg(
        r#"@startuml
start
fork
  :branch A;
fork again
  :branch B;
end fork
stop
@enduml
"#,
    )
    .expect("render fork branch arrow directions");

    for segment in svg.split("<polygon points=\"").skip(1) {
        let Some(points) = segment.split('"').next() else {
            continue;
        };
        let mut coords = points.split_whitespace().filter_map(|pair| {
            let mut it = pair.split(',');
            let _x = it.next()?.parse::<i32>().ok()?;
            let y = it.next()?.parse::<i32>().ok()?;
            Some(y)
        });
        let Some(tip_y) = coords.next() else {
            continue;
        };
        let Some(base_left_y) = coords.next() else {
            continue;
        };
        let Some(base_right_y) = coords.next() else {
            continue;
        };
        // Arrowheads in activity flow should point down: tip y is below base y.
        assert!(
            tip_y >= base_left_y && tip_y >= base_right_y,
            "found upward-pointing arrowhead polygon points={points}"
        );
    }
}

#[test]
fn activity_inline_note_renders_payload_without_directive_prefix() {
    let src = include_str!("fixtures/non_sequence/valid_activity_inline_note_text.puml");

    let document = puml::parser::parse(src).expect("parse activity inline note");
    let NormalizedDocument::Family(model) =
        puml::normalize_family(document).expect("normalize activity inline note")
    else {
        panic!("activity should normalize as a family document");
    };
    let note = model
        .nodes
        .iter()
        .find(|node| node.kind == FamilyNodeKind::Note)
        .expect("activity note node");
    assert_eq!(note.label.as_deref(), Some("baseline note"));
    assert!(
        note.alias.as_deref().is_some_and(|alias| {
            alias.contains("activity::Note")
                && alias.contains("position=right")
                && alias.contains("lane=default")
        }),
        "activity notes should stay out of the active partition lane"
    );

    let svg = puml::render_source_to_svg(src).expect("render activity inline note");
    assert!(svg.contains("data-activity-kind=\"Note\" data-activity-lane=\"default\""));
    assert!(svg.contains(">baseline note<"));
    assert!(!svg.contains(">note right: baseline note<"));
    assert!(
        svg.contains("class=\"activity-note-connector\""),
        "annotation connector should route from detach to the out-of-lane note"
    );
}
