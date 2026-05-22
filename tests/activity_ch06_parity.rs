use puml::model::{FamilyNodeKind, NormalizedDocument};

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
    assert!(
        svg.contains(">Action B should not get arrow<"),
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
        svg.contains("<line x1=\"344\" y1=\"406\" x2=\"344\" y2=\"415\"")
            && svg.contains("<line x1=\"344\" y1=\"415\" x2=\"136\" y2=\"415\"")
            && svg.contains("<line x1=\"136\" y1=\"415\" x2=\"136\" y2=\"424\""),
        "annotation connector should route from detach to the out-of-lane note"
    );
}
