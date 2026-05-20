use super::support::*;
use puml::scene::LayoutOptions;
use puml::{layout, render};
use std::collections::HashSet;

#[test]
fn render_sequence_decorated_arrows_and_teoz_boundary_stay_deterministic() {
    let src = "@startuml\n!pragma teoz true\nparticipant A\nparticipant B\nA -[#red,dashed]> B : styled\nB ->[#blue,dashed]> A : open styled\nB -[hidden]-> A : hidden\n@enduml\n";
    let svg = puml::render_source_to_svg(src).expect("decorated sequence render");

    assert!(svg.contains("styled"));
    assert!(svg.contains("open styled"));
    assert!(svg.contains("hidden"));
    assert!(svg.contains("<polygon points=\""));
    assert!(svg.contains("<polyline points=\""));
    assert!(svg.contains("stroke=\"#ff0000\""));
    assert!(svg.contains("stroke=\"#0000ff\""));
    assert!(svg.contains("stroke-dasharray=\"6 4\""));
    assert!(svg.contains("class=\"sequence-message-line sequence-message-line-colored sequence-message-line-dashed\""));
    assert!(svg.contains("data-sequence-message-style=\"color dashed\""));
    assert!(svg.contains("visibility=\"hidden\""));
}

#[test]
fn render_sequence_plantuml_line_style_arrow_payloads() {
    let src = r##"@startuml
participant A
participant B
A -[#DodgerBlue;line.dotted;line.thickness=4]>> B : semicolon style
B -[line.dashed;line.hidden]-> A : hidden dashed
@enduml
"##;
    let svg = puml::render_source_to_svg(src).expect("line-style sequence arrows render");

    assert!(svg.contains("semicolon style"));
    assert!(svg.contains("stroke=\"#1e90ff\""));
    assert!(svg.contains("stroke-width=\"4\""));
    assert!(svg.contains("stroke-dasharray=\"2 4\""));
    assert!(svg.contains(
        "class=\"sequence-message-line sequence-message-line-colored sequence-message-line-dotted sequence-message-line-thick\""
    ));
    assert!(svg.contains("data-sequence-message-style=\"color dotted thickness\""));
    assert!(svg.contains("hidden dashed"));
    assert!(svg.contains(
        "class=\"sequence-message-line sequence-message-line-dashed sequence-message-line-hidden\""
    ));
    assert!(svg.contains("data-sequence-message-style=\"dashed hidden\""));
    assert!(svg.contains("visibility=\"hidden\""));
}

#[test]
fn render_sequence_rare_arrow_styles_and_note_positions() {
    let src = fixture("arrows/valid_rare_arrow_styles.puml");
    let svg = puml::render_source_to_svg(&src).expect("rare arrow styles render");

    assert!(svg.contains("stroke-width=\"3\""));
    assert!(svg.contains("stroke-width=\"5\""));
    assert!(svg.contains("stroke-dasharray=\"2 4\""));
    assert!(svg.contains("<polyline points=\""));
    assert!(svg.contains("top note"));
    assert!(svg.contains("bottom note"));
    assert_snapshot!("render_sequence_rare_arrow_styles_and_note_positions", svg);
}

#[test]
fn render_sequence_slanted_arrow_heads_are_distinct() {
    let src = fixture("arrows/valid_arrow_variant_tokenization.puml");
    let svg = puml::render_source_to_svg(&src).expect("slanted arrow styles render");

    assert!(svg.contains("sequence-arrow-head-slash"));
    assert!(svg.contains("sequence-arrow-head-backslash"));
    assert!(
        !svg.contains("<polygon points=\""),
        "slanted half-head arrows should not fall back to filled triangle heads"
    );
}

#[test]
fn render_sequence_bidirectional_arrows_stay_single_row_with_double_heads() {
    let src = "@startuml\nparticipant A\nparticipant B\nA <-> B : sync bidi\nA <--> B : dashed bidi\n@enduml\n";
    let svg = puml::render_source_to_svg(src).expect("bidirectional sequence arrows render");

    assert_eq!(svg.matches(">sync bidi</text>").count(), 1);
    assert_eq!(svg.matches(">dashed bidi</text>").count(), 1);
    assert_eq!(svg.matches("<polygon points=\"").count(), 4);
    assert!(
        svg.contains("stroke-dasharray=\"6 4\""),
        "dashed bidirectional arrow should preserve its dashed stroke"
    );
}

#[test]
fn render_sequence_dotted_parallel_edges_share_teoz_row_deterministically() {
    let src = fixture("arrows/valid_dotted_parallel_sequence_edges.puml");
    let ast = puml::parse(&src).expect("parse");
    let doc = puml::normalize(ast).expect("normalize");
    let scene = layout::layout(&doc, LayoutOptions::default());

    assert_eq!(scene.messages.len(), 3);
    assert_eq!(
        scene.messages[0].y, scene.messages[1].y,
        "PlantUML `&` parallel message should share the previous row"
    );
    assert!(
        scene.messages.iter().all(|message| message.style.dotted),
        "dot-arrow syntax and dotted style should both reach layout"
    );

    let svg = render::render_svg(&scene);
    assert!(svg.contains("P-01 dotted"));
    assert!(svg.contains("compatibility"));
    assert!(svg.contains("P-02 parallel"));
    assert!(svg.contains("dotted styled"));
    assert!(svg.contains("stroke-dasharray=\"2 4\""));
    assert_snapshot!(
        "render_sequence_dotted_parallel_edges_share_teoz_row_deterministically",
        svg
    );
}

#[test]
fn render_sequence_teoz_overlapping_parallel_routes_get_distinct_lanes() {
    let src = fixture("arrows/valid_teoz_overlapping_routes.puml");
    let ast = puml::parse(&src).expect("parse");
    let doc = puml::normalize(ast).expect("normalize");
    assert!(doc.teoz);
    let scene = layout::layout(&doc, LayoutOptions::default());

    assert_eq!(scene.messages.len(), 5);
    let shared_row_y = scene.messages[0].y;
    assert!(
        scene.messages[..4]
            .iter()
            .all(|message| message.y == shared_row_y),
        "Teoz `&` messages stay attached to the initiating row"
    );
    let route_lanes = scene.messages[..4]
        .iter()
        .map(|message| message.route_y)
        .collect::<HashSet<_>>();
    assert_eq!(
        route_lanes.len(),
        4,
        "overlapping Teoz messages should not collapse onto one rendered route"
    );
    assert!(
        scene.messages[4].y > *route_lanes.iter().max().unwrap(),
        "the following row should clear the routed parallel lanes"
    );

    let svg = render::render_svg(&scene);
    assert!(svg.contains("duplicate route"));
    assert!(svg.contains("self audit"));
    assert!(svg.contains("route labels stay readable"));
    assert_snapshot!(
        "render_sequence_teoz_overlapping_parallel_routes_get_distinct_lanes",
        svg
    );
}

#[test]
fn render_sequence_teoz_response_below_arrow_keeps_reply_label_under_dashed_route() {
    let src = fixture("arrows/valid_teoz_response_below_arrow.puml");
    let ast = puml::parse(&src).expect("parse");
    let doc = puml::normalize(ast).expect("normalize");
    assert!(doc.teoz);
    assert!(doc.style.response_message_below_arrow);
    assert_eq!(doc.style.message_align, puml::theme::MessageAlign::Center);

    let scene = layout::layout(&doc, LayoutOptions::default());
    assert_eq!(scene.messages.len(), 4);
    assert_eq!(
        scene.messages[0].y, scene.messages[1].y,
        "teoz `&` message should share the initiating row"
    );
    assert_eq!(
        scene.messages[1].y, scene.messages[2].y,
        "multiple teoz `&` messages should stay on the initiating row"
    );
    assert!(
        scene.messages[3].arrow.contains("--"),
        "final message should remain a dashed response arrow"
    );

    let note = scene
        .notes
        .iter()
        .find(|note| note.text.contains("shared routing context"))
        .expect("note across should reach layout");
    let first = scene.participants.first().expect("first participant");
    let last = scene.participants.last().expect("last participant");
    assert_eq!(note.x, first.x);
    assert!(
        note.width >= (last.x + last.width) - first.x,
        "note across should span the participant range"
    );

    let svg = render::render_svg(&scene);
    assert!(svg.contains("stroke=\"#ff0000\""));
    assert!(svg.contains("stroke-dasharray=\"6 4\""));
    assert!(svg.contains("shared routing context"));
    let texts = parse_svg_texts(&svg);
    let reply_label = texts
        .iter()
        .find(|text| text.text == "crossing result")
        .expect("response label should render");
    assert!(
        reply_label.y > scene.messages[3].route_y,
        "ResponseMessageBelowArrow should put dashed response labels below the arrow"
    );
}

#[test]
fn render_sequence_self_call_keeps_visible_arrowhead_after_groups_and_dividers() {
    for fixture_name in [
        "docs/examples/sequence/17_all_groups.puml",
        "docs/examples/sequence/23_dividers.puml",
    ] {
        let src =
            std::fs::read_to_string(format!("{}/{}", env!("CARGO_MANIFEST_DIR"), fixture_name))
                .expect("fixture");
        let svg = puml::render_source_to_svg(&src).expect("self-call render");

        assert!(
            svg.contains("<path d=\"M "),
            "self-call loop path should render in {fixture_name}"
        );
        assert!(
            svg.contains("<polygon points=\"244,300 252,295 252,305\"")
                || svg.contains("<polygon points=\"84,300 92,295 92,305\"")
                || svg.contains("<polygon points=\"84,980 92,975 92,985\"")
                || svg.contains("<polygon points=\"84,460 92,455 92,465\""),
            "self-call arrowhead should remain visible in {fixture_name}"
        );
    }
}

/// Regression test for #765: divider mid-diagram must not cause footboxes to
/// overlap the self-loop U-shape drawn below the cleanup self-message.
/// The footbox y must be strictly greater than the bottom of the self-loop
/// (self-loop y + 32 = the rendered loop drop).
#[test]
fn regression_765_divider_mid_sequence_footbox_clears_self_loop() {
    let src = std::fs::read_to_string(format!(
        "{}/docs/examples/sequence/23_dividers.puml",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture");
    let ast = puml::parse(&src).expect("parse");
    let doc = puml::normalize(ast).expect("normalize");
    let scene = layout::layout(&doc, LayoutOptions::default());

    let self_loop = scene
        .messages
        .iter()
        .find(|m| m.from_id == m.to_id)
        .expect("23_dividers.puml must contain a self-loop (Alice -> Alice: cleanup)");
    let self_loop_bottom = self_loop.y + 32; // must match SELF_LOOP_DROP in layout.rs

    for footbox in &scene.footboxes {
        assert!(
            footbox.y > self_loop_bottom,
            "footbox '{}' y={} must be strictly below self-loop bottom={} (issue #765: participant header duplicated by divider)",
            footbox.id,
            footbox.y,
            self_loop_bottom,
        );
    }
}

#[test]
fn render_sequence_theme_sunlust_else_separator_clears_self_loop_and_keeps_arrowheads() {
    let src = std::fs::read_to_string(format!(
        "{}/docs/examples/themes/theme_sunlust.puml",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture");
    let ast = puml::parse(&src).expect("parse");
    let doc = puml::normalize(ast).expect("normalize");
    let scene = layout::layout(&doc, LayoutOptions::default());

    let success = scene
        .messages
        .iter()
        .find(|message| message.label.as_deref() == Some("log success"))
        .expect("success self-call");
    let error = scene
        .messages
        .iter()
        .find(|message| message.label.as_deref() == Some("log error"))
        .expect("error self-call");
    let alt = scene
        .groups
        .iter()
        .find(|group| group.kind.eq_ignore_ascii_case("alt"))
        .expect("alt group");
    let separator = alt
        .separators
        .iter()
        .find(|separator| separator.label.as_deref() == Some("else error"))
        .expect("else separator");

    assert!(
        separator.y >= success.route_y + 32 + 14,
        "else separator should reserve enough clearance above the divider label after a self-loop"
    );

    let svg = render::render_svg(&scene);
    for message in [success, error] {
        let tip_y = message.route_y + 32;
        let head = format!(
            "<polygon points=\"{},{} {},{} {},{}\"",
            message.x1,
            tip_y,
            message.x1 + 8,
            tip_y - 5,
            message.x1 + 8,
            tip_y + 5
        );
        assert!(
            svg.contains(&head),
            "self-call arrowhead should remain visible for {}",
            message.label.as_deref().unwrap_or("self-call")
        );
    }
    assert!(svg.contains(">else error</text>"));
}

#[test]
fn render_sequence_ref_over_keeps_followup_response_label_below_box() {
    let src = docs_example("sequence/22_ref_over.puml");
    let ast = puml::parse(&src).expect("parse");
    let doc = puml::normalize(ast).expect("normalize");
    let scene = layout::layout(&doc, LayoutOptions::default());

    let ref_box = scene
        .groups
        .iter()
        .find(|group| group.kind.eq_ignore_ascii_case("ref"))
        .expect("ref group");
    let response = scene
        .messages
        .iter()
        .find(|message| message.label.as_deref() == Some("response"))
        .expect("response message");
    let response_label_top = response.route_y - 8;

    assert!(
        response_label_top > ref_box.y + ref_box.height,
        "response label should clear the bottom of the ref box"
    );

    let svg = render::render_svg(&scene);
    assert!(svg.contains(">response</text>"));
    assert!(svg.contains(">ref</text>"));
}

#[test]
fn render_sequence_ref_fragment_uses_header_row_without_participant_text() {
    let src = docs_example("sequence/08_ref.puml");
    let ast = puml::parse(&src).expect("parse");
    let doc = puml::normalize(ast).expect("normalize");
    let scene = layout::layout(&doc, LayoutOptions::default());

    let svg = render::render_svg(&scene);
    assert!(svg.contains("<polygon points=\"24,120 56,120 56,134 50,140 24,140\""));
    assert!(svg.contains(">ref</text>"));
    assert!(svg.contains(">over Alice, Bob</text>"));
    assert!(svg.contains(">Authentication Flow</text>"));
    assert!(svg.contains(">See diagram AUTH-01</text>"));
}

#[test]
fn render_sequence_box_groups_wrap_declared_participants() {
    let src = docs_example("sequence/09_box.puml");
    let ast = puml::parse(&src).expect("parse");
    let doc = puml::normalize(ast).expect("normalize");
    let scene = layout::layout(&doc, LayoutOptions::default());

    let frontend = scene
        .groups
        .iter()
        .find(|group| group.kind == "box" && group.label.as_deref() == Some("Frontend"))
        .expect("frontend group");
    let backend = scene
        .groups
        .iter()
        .find(|group| group.kind == "box" && group.label.as_deref() == Some("Backend"))
        .expect("backend group");
    let browser = scene
        .participants
        .iter()
        .find(|participant| participant.id == "Browser")
        .expect("Browser");
    let react_app = scene
        .participants
        .iter()
        .find(|participant| participant.id == "ReactApp")
        .expect("ReactApp");
    let api = scene
        .participants
        .iter()
        .find(|participant| participant.id == "API")
        .expect("API");
    let db = scene
        .participants
        .iter()
        .find(|participant| participant.id == "DB")
        .expect("DB");

    assert!(frontend.x <= browser.x);
    assert!(frontend.x + frontend.width >= react_app.x + react_app.width);
    assert!(backend.x <= api.x);
    assert!(backend.x + backend.width >= db.x + db.width);
    assert_eq!(frontend.color.as_deref(), Some("#e0f2fe"));
    assert_eq!(backend.color.as_deref(), Some("#fde68a"));

    let svg = render::render_svg(&scene);
    assert!(svg.contains("class=\"sequence-participant-group\""));
    assert!(svg.contains("fill=\"#e0f2fe\""));
    assert!(svg.contains("fill=\"#fde68a\""));
    assert!(svg.contains(">Frontend</text>"));
    assert!(svg.contains(">Backend</text>"));
}

#[test]
fn render_sequence_parity_slice_places_rich_parallel_and_multitarget_notes() {
    let src = fixture("e2e/sequence_parity_vertical_slice.puml");
    let ast = puml::parse(&src).expect("parse");
    let doc = puml::normalize(ast).expect("normalize");
    let scene = layout::layout(&doc, LayoutOptions::default());

    assert_eq!(scene.messages.len(), 4);
    assert_eq!(
        scene.messages[0].y, scene.messages[1].y,
        "teoz parallel message should share the initiating row"
    );
    assert_eq!(
        scene.messages[1].y, scene.messages[2].y,
        "multiple parallel messages should remain on the same teoz row"
    );
    assert!(
        scene.messages[3].y > scene.messages[0].y + LayoutOptions::default().message_row_height,
        "parallel labels should reserve deterministic space before the next row"
    );

    let note = scene
        .notes
        .iter()
        .find(|note| note.text.contains("span **multi** target"))
        .expect("multi-target note");
    let a = scene
        .participants
        .iter()
        .find(|participant| participant.id == "A")
        .expect("participant A");
    let c = scene
        .participants
        .iter()
        .find(|participant| participant.id == "C")
        .expect("participant C");
    assert_eq!(note.x, a.x);
    assert!(
        note.width >= (c.x + c.width) - a.x,
        "note over A,C should span the participant range"
    );

    let svg = render::render_svg(&scene);
    assert!(svg.contains("REQ-007"));
    assert!(svg.contains("REQ-009"));
    assert!(svg.contains("REQ-011"));
    assert!(svg.contains("font-weight=\"bold\""));
    assert!(svg.contains("font-style=\"italic\""));
    assert!(svg.contains("fill=\"#008800\""));
    assert!(svg.contains("xlink:href=\"https://example.com\""));
    assert!(svg.contains("sequence-arrow-head-slash"));
    assert!(svg.contains("sequence-arrow-head-backslash"));
    assert!(svg.contains("<circle"));
    assert!(svg.contains("span "));
}

#[test]
fn render_sequence_notes_fixture_keeps_leftmost_over_note_centered_with_canvas_padding() {
    let src = std::fs::read_to_string(format!(
        "{}/docs/examples/sequence/07_notes.puml",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("fixture");
    let ast = puml::parse(&src).expect("parse");
    let doc = puml::normalize(ast).expect("normalize");
    let options = LayoutOptions::default();
    let scene = layout::layout(&doc, options);

    let alice = scene
        .participants
        .iter()
        .find(|participant| participant.id == "Alice")
        .expect("participant Alice");
    let note = scene
        .notes
        .iter()
        .find(|note| note.text.contains("client received"))
        .expect("leftmost over note");

    assert_eq!(note.x, options.margin);
    assert_eq!(
        note.x + (note.width / 2),
        alice.x + (alice.width / 2),
        "note over Alice should stay centered on Alice after preserving left canvas padding"
    );
    assert!(
        alice.x > options.margin,
        "leftmost participant should shift right when needed to preserve note centering"
    );

    let svg = render::render_svg(&scene);
    assert!(svg.contains(">client received</text>"));
}

#[test]
fn render_sequence_advanced_wave_autonumber_spacing_and_rare_heads() {
    let src = fixture("e2e/sequence_advanced_wave_autonumber_spacing.puml");
    let ast = puml::parse(&src).expect("parse");
    let doc = puml::normalize(ast).expect("normalize");
    assert!(doc.style.sequence_message_span);
    let scene = layout::layout(&doc, LayoutOptions::default());

    assert_eq!(scene.messages.len(), 5);
    assert_eq!(
        scene.messages[0].y, scene.messages[1].y,
        "ampersand teoz-ish parallel messages should share a row"
    );
    assert!(
        scene.messages[3].y - scene.messages[2].y
            >= LayoutOptions::default().message_row_height * 3,
        "explicit |||80||| spacer should reserve multiple deterministic rows"
    );
    assert!(
        scene.groups.iter().any(|group| group.kind == "ref"
            && group.width >= LayoutOptions::default().participant_spacing * 2),
        "ref over A,C should span the participant range"
    );

    let svg = render::render_svg(&scene);
    assert!(svg.contains("1.02:003 long label"));
    assert!(svg.contains("3.02:007 increment-first"));
    assert!(svg.contains("data-sequence-arrow-end=\"circle\""));
    assert!(svg.contains("data-sequence-arrow-end=\"cross\""));
    assert!(svg.contains("sequence-arrow-head-slash"));
    assert!(svg.contains("sequence-arrow-head-backslash"));
    assert!(svg.contains("stroke=\"#1e90ff\""));
    assert!(svg.contains("stroke-width=\"4\""));
    assert!(svg.ends_with("</svg>"));
}

#[test]
fn render_sequence_ref_over_implicit_participant_extends_span() {
    let src = "@startuml\n\
title Ref Over Multiple Participants\n\
Alice -> Bob: request\n\
ref over Alice, Bob, Charlie\n\
  See authentication flow\n\
  in diagram AUTH-001\n\
end ref\n\
Bob --> Alice: response\n\
@enduml\n";
    let ast = puml::parse(src).expect("parse");
    let doc = puml::normalize(ast).expect("normalize");
    let scene = layout::layout(&doc, LayoutOptions::default());

    let participant_ids = scene
        .participants
        .iter()
        .map(|participant| participant.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(participant_ids, vec!["Alice", "Bob", "Charlie"]);

    let alice = scene
        .participants
        .iter()
        .find(|participant| participant.id == "Alice")
        .expect("Alice participant");
    let charlie = scene
        .participants
        .iter()
        .find(|participant| participant.id == "Charlie")
        .expect("Charlie participant");
    let reference = scene
        .groups
        .iter()
        .find(|group| group.kind == "ref")
        .expect("ref group");

    assert_eq!(reference.x, alice.x);
    assert_eq!(reference.x + reference.width, charlie.x + charlie.width);

    let svg = render::render_svg(&scene);
    assert!(
        svg.matches(">Charlie</text>").count() >= 2,
        "Charlie should render in both the top and bottom participant rows"
    );
}

#[test]
fn render_sequence_lifecycle_shortcuts_have_visible_markers() {
    let src = fixture("lifecycle/valid_shortcuts_expansion.puml");
    let svg = puml::render_source_to_svg(&src).expect("lifecycle shortcut render");

    assert!(
        svg.contains("class=\"sequence-activation\""),
        "activation bars should render for ++ shortcut"
    );
    assert!(
        svg.contains("class=\"sequence-create\""),
        "create markers should render for create/**"
    );
    assert!(
        svg.contains("class=\"sequence-destroy\""),
        "destroy markers should render for !!"
    );
}

#[test]
fn render_sequence_participant_order_controls_lifeline_placement() {
    let src = "@startuml\nparticipant Last order 30\nparticipant Middle order 20\nparticipant First order 10\nFirst -> Last : ordered\n@enduml\n";
    let doc = puml::parse(src).expect("parse");
    let model = puml::normalize(doc).expect("normalize");
    let scene = layout::layout(&model, LayoutOptions::default());

    let ids = scene
        .participants
        .iter()
        .map(|p| p.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["First", "Middle", "Last"]);

    let first_x = scene
        .participants
        .iter()
        .find(|p| p.id == "First")
        .expect("First participant")
        .x;
    let last_x = scene
        .participants
        .iter()
        .find(|p| p.id == "Last")
        .expect("Last participant")
        .x;
    assert!(
        first_x < last_x,
        "participant order should affect x placement"
    );

    let svg = render::render_svg(&scene);
    assert!(svg.contains("ordered"));
    assert_snapshot!(
        "render_sequence_participant_order_controls_lifeline_placement",
        svg
    );
}

#[test]
fn render_sequence_explicit_lifecycle_has_activation_and_destroy_marker() {
    let src = fixture("lifecycle/valid_create_activate_destroy.puml");
    let svg = puml::render_source_to_svg(&src).expect("explicit lifecycle render");

    assert!(svg.contains("data-participant=\"Worker\""));
    assert!(svg.contains("class=\"sequence-activation\""));
    assert!(svg.contains("class=\"sequence-create\""));
    assert!(svg.contains("class=\"sequence-destroy\""));
}
