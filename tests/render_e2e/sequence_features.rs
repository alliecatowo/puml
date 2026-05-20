use super::support::*;
use puml::model::{
    Participant, ParticipantRole, SequenceDocument, SequenceEvent, SequenceEventKind,
};
use puml::scene::LayoutOptions;
use puml::{
    extract_markdown_diagrams, layout, parse_with_pipeline_options, render, FrontendSelection,
    ParsePipelineOptions,
};

#[test]
fn render_svg_contains_expected_structure() {
    let src = fixture("e2e/deterministic_sequence.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");

    assert!(svg.starts_with("<svg "));
    assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(svg.contains("<rect width=\"100%\" height=\"100%\" fill=\"white\"/>"));
    assert!(svg.contains("stroke-dasharray=\"6 4\""));
    assert!(svg.contains("<polygon points=\""));
    assert!(svg.ends_with("</svg>"));
}

#[test]
fn render_svg_applies_autonumber_restart_step_and_format_subset() {
    let src = fixture("structure/valid_autonumber_restart_step_format.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");

    for expected in [
        "[010] first",
        "[015] second",
        "unnumbered",
        "R-20 resumed",
        "S-03 restarted",
    ] {
        assert!(
            svg.contains(expected),
            "expected autonumber label not found: {expected}"
        );
    }
    assert!(!svg.contains("20 unnumbered"));
}

#[test]
fn render_svg_applies_autonumber_off_and_resume_edges() {
    let src = fixture("structure/valid_autonumber_off_resume_edges.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");

    for expected in [
        "ID-07 first",
        "gap",
        "R-10",
        "resumed-default-step",
        "R-13",
        "resumed-new-step",
    ] {
        assert!(
            svg.contains(expected),
            "expected autonumber label not found: {expected}"
        );
    }
    assert!(!svg.contains("ID-10 gap"));
}

#[test]
fn render_svg_applies_dotted_autonumber_and_hash_padding() {
    let src = fixture("structure/valid_autonumber_dotted_and_hash_padding.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");

    for expected in [
        "1.02.003 dotted-start",
        "1.02.004 dotted-next",
        "ID-007 hash-padded",
        "ID-009 hash-next",
        "plain-gap",
        "R-011 resume-hash-step",
    ] {
        assert!(
            svg.contains(expected),
            "expected autonumber label not found: {expected}"
        );
    }
    assert!(!svg.contains("ID-011 plain-gap"));
}

#[test]
fn render_svg_rejects_invalid_source() {
    let src = fixture("errors/invalid_plain.txt");
    let err = puml::render_source_to_svg(&src);
    assert!(err.is_err(), "invalid source should fail render");
}

#[test]
fn render_svg_output_avoids_active_content_patterns() {
    let src = fixture("e2e/deterministic_sequence.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");
    let lowered = svg.to_ascii_lowercase();

    for forbidden in [
        "<script",
        "foreignobject",
        "onload=",
        "onerror=",
        "javascript:",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "svg should not contain forbidden pattern: {forbidden}"
        );
    }
}

#[test]
fn render_source_to_svgs_supports_newpage_with_title_override() {
    let src = "@startuml\nTitle Base\nA -> B : one\nnewpage Page Two\nB -> A : two\n@enduml\n";
    let pages = puml::render_source_to_svgs(src).expect("render should succeed");

    assert_eq!(pages.len(), 2);
    assert!(pages[0].contains(">Base<"));
    assert!(pages[1].contains(">Page Two<"));
}

#[test]
fn render_svg_sequence_header_footer_and_caption_have_visible_lifecycle() {
    let src = "@startuml\nheader Trace Header\ncaption\nAudit trail\npage 1\nend caption\nfooter Rendered Footer\nA -> B : hello\n@enduml\n";
    let ast = puml::parse(src).expect("parse should succeed");
    let doc = puml::normalize(ast).expect("normalize should succeed");
    assert_eq!(doc.header.as_deref(), Some("Trace Header"));
    assert_eq!(doc.caption.as_deref(), Some("Audit trail\npage 1"));
    assert_eq!(doc.footer.as_deref(), Some("Rendered Footer"));

    let scene = layout::layout(&doc, LayoutOptions::default());
    assert!(scene.header.is_some(), "header should reach the scene");
    assert!(scene.caption.is_some(), "caption should reach the scene");
    assert!(scene.footer.is_some(), "footer should reach the scene");
    assert!(
        scene.header.as_ref().expect("header label").y < scene.participants[0].y,
        "header should reserve vertical space before participants"
    );
    assert!(
        scene.caption.as_ref().expect("caption label").y > scene.footboxes[0].y,
        "caption should render after the sequence body"
    );

    let svg = render::render_svg(&scene);
    assert!(svg.contains("class=\"sequence-header\""));
    assert!(svg.contains("class=\"sequence-caption\""));
    assert!(svg.contains("class=\"sequence-footer\""));
    assert!(svg.contains(">Trace Header<"));
    assert!(svg.contains(">Audit trail<"));
    assert!(svg.contains(">page 1<"));
    assert!(svg.contains(">Rendered Footer<"));
}

#[test]
fn render_source_to_svg_rejects_multipage_sources() {
    let src = "@startuml\nA -> B\nnewpage\nB -> A\n@enduml\n";
    let err = puml::render_source_to_svg(src).expect_err("expected multipage error");
    assert!(err.message.contains("multiple pages detected"));
}

#[test]
fn markdown_fence_frontend_hints_route_mixed_fence_rendering_deterministically() {
    let src = fixture("markdown/mixed_fences.md");
    let diagrams = extract_markdown_diagrams(&src);
    assert_eq!(diagrams.len(), 5);
    assert_eq!(diagrams[0].fence_frontend, FrontendSelection::Auto);
    assert_eq!(diagrams[1].fence_frontend, FrontendSelection::Auto);
    assert_eq!(diagrams[2].fence_frontend, FrontendSelection::Picouml);
    assert_eq!(diagrams[3].fence_frontend, FrontendSelection::Auto);
    assert_eq!(diagrams[4].fence_frontend, FrontendSelection::Mermaid);

    let mut labels = Vec::new();
    for diagram in diagrams {
        let options = ParsePipelineOptions {
            frontend: diagram.fence_frontend,
            ..ParsePipelineOptions::default()
        };
        let document = parse_with_pipeline_options(&diagram.source, &options).expect("parse");
        let model = puml::normalize(document).expect("normalize");
        let message = model
            .events
            .iter()
            .find_map(|event| match &event.kind {
                SequenceEventKind::Message { label, .. } => label.clone(),
                _ => None,
            })
            .expect("message label");
        labels.push(message);
    }

    assert_eq!(
        labels,
        vec![
            "puml-one",
            "pumlx-two",
            "picouml-three",
            "plantuml-four",
            "mermaid-five",
        ]
    );
}

#[test]
fn render_svg_handles_self_found_lost_and_modifiers() {
    let doc = SequenceDocument {
        participants: vec![
            Participant {
                id: "A".to_string(),
                display: "A".to_string(),
                role: ParticipantRole::Participant,
                explicit: true,
            },
            Participant {
                id: "B".to_string(),
                display: "B".to_string(),
                role: ParticipantRole::Participant,
                explicit: true,
            },
        ],
        events: vec![
            SequenceEvent {
                span: puml::source::Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "[*]".to_string(),
                    to: "A".to_string(),
                    arrow: "->".to_string(),
                    label: Some("found".to_string()),
                    style: Default::default(),
                    from_virtual: Some(puml::model::VirtualEndpoint {
                        side: puml::model::VirtualEndpointSide::Left,
                        kind: puml::model::VirtualEndpointKind::Filled,
                    }),
                    to_virtual: None,
                },
            },
            SequenceEvent {
                span: puml::source::Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "A".to_string(),
                    to: "A".to_string(),
                    arrow: "->".to_string(),
                    label: Some("self".to_string()),
                    style: Default::default(),
                    from_virtual: None,
                    to_virtual: None,
                },
            },
            SequenceEvent {
                span: puml::source::Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "A".to_string(),
                    to: "[*]".to_string(),
                    arrow: "->".to_string(),
                    label: Some("lost".to_string()),
                    style: Default::default(),
                    from_virtual: None,
                    to_virtual: Some(puml::model::VirtualEndpoint {
                        side: puml::model::VirtualEndpointSide::Right,
                        kind: puml::model::VirtualEndpointKind::Filled,
                    }),
                },
            },
            SequenceEvent {
                span: puml::source::Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "A".to_string(),
                    to: "B".to_string(),
                    arrow: "-->".to_string(),
                    label: Some("modifier-syntax-safe".to_string()),
                    style: Default::default(),
                    from_virtual: None,
                    to_virtual: None,
                },
            },
        ],
        ..puml::model::SequenceDocument::default()
    };
    let scene = layout::layout(&doc, LayoutOptions::default());
    let first = render::render_svg(&scene);
    let second = render::render_svg(&scene);

    assert_eq!(first, second, "render output should be deterministic");
    assert!(first.contains(">A<"));
    assert!(!first.contains(">[*]<"));
    assert_snapshot!("render_svg_handles_self_found_lost_and_modifiers", first);
}

#[test]
fn render_svg_applies_supported_sequence_skinparam_colors() {
    let src = fixture("styling/valid_skinparam_sequence_colors_supported.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");

    assert!(
        svg.contains("stroke=\"#ff0000\""),
        "arrow color should be applied"
    );
    assert!(
        svg.contains("stroke=\"#00aa00\""),
        "lifeline border color should be applied"
    );
    assert!(
        svg.contains("fill=\"#f0f0ff\""),
        "participant background should be applied"
    );
    assert!(
        svg.contains("stroke=\"#2222aa\""),
        "participant border should be applied"
    );
    assert!(
        svg.contains("fill=\"#ffffdd\""),
        "note background should be applied"
    );
    assert!(
        svg.contains("stroke=\"#aa8800\""),
        "note border should be applied"
    );
    assert!(
        svg.contains("fill=\"#f5f5f5\""),
        "group background should be applied"
    );
    assert!(
        svg.contains("stroke=\"#444444\""),
        "group border should be applied"
    );
    assert_snapshot!(
        "render_svg_applies_supported_sequence_skinparam_colors",
        svg
    );
}

#[test]
fn render_svg_supports_sequence_arrow_color_alias() {
    let src = "@startuml\nskinparam SequenceArrowColor #ab1010\nA -> B : hello\n@enduml\n";
    let svg = puml::render_source_to_svg(src).expect("render should succeed");
    assert!(svg.contains("stroke=\"#ab1010\""));
    assert_snapshot!("render_svg_supports_sequence_arrow_color_alias", svg);
}

#[test]
fn render_svg_skinparam_color_values_are_canonicalized_and_hardened() {
    let src = "@startuml\nskinparam ArrowColor #AaBbCc\nskinparam NoteBorderColor #112233\"/><script>\nA -> B : hi\nnote over A, B: note\n@enduml\n";
    let svg = puml::render_source_to_svg(src).expect("render should succeed");

    assert!(
        svg.contains("stroke=\"#aabbcc\""),
        "supported color should be lowercased and applied"
    );
    assert!(
        svg.contains("stroke=\"#111\""),
        "invalid note border color should keep deterministic default"
    );
    assert!(
        !svg.to_ascii_lowercase().contains("<script"),
        "unsafe skinparam token must not be emitted"
    );
}

#[test]
fn render_svg_sequence_skinparam_maxmessagesize_is_noop_and_deterministic() {
    let src = fixture("styling/valid_skinparam_maxmessagesize_supported.puml");
    let first = puml::render_source_to_svg(&src).expect("first render should succeed");
    let second = puml::render_source_to_svg(&src).expect("second render should succeed");
    assert_eq!(first, second, "render output should be deterministic");
    assert_snapshot!(
        "render_svg_sequence_skinparam_maxmessagesize_is_noop_and_deterministic",
        first
    );
}

#[test]
fn render_svg_handles_ref_else_and_multi_target_notes() {
    let src = fixture("groups/valid_ref_and_else_rendering.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");

    // "ref" appears in the header row; the body carries only the payload text.
    assert!(svg.contains(">ref<") || svg.contains(">ref "));
    assert!(svg.contains("else fallback"));
    assert_snapshot!("render_svg_handles_ref_else_and_multi_target_notes", svg);
}

#[test]
fn render_svg_sequence_alt_opt_loop_fixture_labels_alt_and_else() {
    let src = std::fs::read_to_string(format!(
        "{}/docs/examples/sequence/05_alt_opt_loop.puml",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("sequence alt/opt/loop fixture should exist");
    let svg =
        puml::render_source_to_svg(&src).expect("sequence alt/opt/loop fixture should render");

    assert!(
        svg.contains(">alt credentials valid<"),
        "alt branch header should include the group keyword and label"
    );
    assert!(
        svg.contains(">else invalid<"),
        "else branch separator should include the group keyword and label"
    );
    assert!(
        svg.contains("<polygon points=\"24,120 187,120 187,134 181,140 24,140\""),
        "alt branch header should render as a pentagon notch instead of a plain rectangle label"
    );
    assert!(
        svg.contains("<polygon points=\"24,320 145,320 145,334 139,340 24,340\""),
        "opt branch header should render as a pentagon notch instead of a plain rectangle label"
    );
    assert!(
        svg.contains("<polygon points=\"24,440 124,440 124,454 118,460 24,460\""),
        "loop branch header should render as a pentagon notch instead of a plain rectangle label"
    );
}

#[test]
fn render_svg_sequence_all_group_types_fixture_uses_fragment_notches() {
    let src = std::fs::read_to_string(format!(
        "{}/docs/examples/sequence/17_all_groups.puml",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("sequence all-groups fixture should exist");
    let svg = puml::render_source_to_svg(&src).expect("sequence all-groups fixture should render");

    for (header_text, polygon_prefix) in [
        (
            "alt success",
            "<polygon points=\"24,148 117,148 117,162 111,168 24,168\"",
        ),
        (
            "opt optional step",
            "<polygon points=\"24,348 159,348 159,362 153,368 24,368\"",
        ),
        (
            "loop retry 3 times",
            "<polygon points=\"24,468 166,468 166,482 160,488 24,488\"",
        ),
        (
            "par parallel",
            "<polygon points=\"24,588 124,588 124,602 118,608 24,608\"",
        ),
        (
            "critical critical section",
            "<polygon points=\"24,788 215,788 215,802 209,808 24,808\"",
        ),
        (
            "break on error",
            "<polygon points=\"24,908 138,908 138,922 132,928 24,928\"",
        ),
        (
            "group custom label",
            // y shifted from 1028 to 1068 after #731 fix: self-loop in break now
            // allocates 2 rows instead of 1 to prevent overlap with following messages.
            "<polygon points=\"24,1068 166,1068 166,1082 160,1088 24,1088\"",
        ),
    ] {
        assert!(
            svg.contains(header_text),
            "combined fragment header should keep its label text for {header_text}"
        );
        assert!(
            svg.contains(polygon_prefix),
            "combined fragment header should render a pentagon notch for {header_text}"
        );
    }
}

#[test]
fn render_svg_preserves_virtual_endpoint_fidelity() {
    let src = fixture("arrows/virtual_endpoint_fidelity.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");

    assert!(
        svg.contains("<circle") && svg.contains("fill=\"white\""),
        "circle virtual endpoint should render"
    );
    assert!(
        svg.contains("fill=\"#111\""),
        "filled virtual endpoint should render"
    );
    assert!(
        svg.contains("x1=\"") && svg.contains("stroke=\"#111\" stroke-width=\"1.5\""),
        "line-based virtual endpoint markers should render"
    );
    assert_snapshot!("render_svg_preserves_virtual_endpoint_fidelity", svg);
}

#[test]
fn render_svg_note_across_spans_content_width() {
    let src = fixture("notes/valid_note_across_multi.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");

    assert!(svg.contains("cluster note"));
    assert_snapshot!("render_svg_note_across_spans_content_width", svg);
}

#[test]
fn render_svg_expands_note_ref_and_group_for_long_multiline_text() {
    let src = fixture("groups/valid_overflow_long_blocks.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");

    assert!(svg.contains("very long yellow note line"));
    assert!(svg.contains("External dependency handshake"));
    assert!(svg.contains("group This group label is intentionally verbose"));
    assert_snapshot!(
        "render_svg_expands_note_ref_and_group_for_long_multiline_text",
        svg
    );
}

#[test]
fn render_svg_hides_footbox_and_ends_lifelines_above_footer_area() {
    let src = "@startuml\nhide footbox\nparticipant A\nparticipant B\nA -> B : hello\n@enduml\n";
    let ast = puml::parse(src).expect("parse should succeed");
    let doc = puml::normalize(ast).expect("normalize should succeed");
    let scene = layout::layout(&doc, LayoutOptions::default());
    let svg = render::render_svg(&scene);

    assert!(scene.footboxes.is_empty(), "footboxes should be omitted");
    assert_eq!(scene.lifelines.len(), 2);
    assert!(
        scene.lifelines.iter().all(|l| l.y2 < scene.height - 24),
        "lifelines should end above reserved footer/caption area"
    );
    assert_eq!(
        svg.match_indices("fill=\"#f6f6f6\"").count(),
        2,
        "only top participant boxes should be rendered"
    );
    assert_snapshot!(
        "render_svg_hides_footbox_and_ends_lifelines_above_footer_area",
        svg
    );
}

#[test]
fn render_svg_shows_footbox_and_lifelines_reach_it() {
    let src = "@startuml\nshow footbox\nparticipant A\nparticipant B\nA -> B : hello\n@enduml\n";
    let ast = puml::parse(src).expect("parse should succeed");
    let doc = puml::normalize(ast).expect("normalize should succeed");
    let scene = layout::layout(&doc, LayoutOptions::default());
    let svg = render::render_svg(&scene);

    assert_eq!(
        scene.footboxes.len(),
        2,
        "bottom footboxes should be rendered"
    );
    assert_eq!(scene.lifelines.len(), 2);
    for (lifeline, footbox) in scene.lifelines.iter().zip(scene.footboxes.iter()) {
        assert_eq!(lifeline.participant_id, footbox.id);
        assert_eq!(lifeline.y2, footbox.y);
    }
    assert_eq!(
        svg.match_indices("fill=\"#f6f6f6\"").count(),
        4,
        "top and bottom participant boxes should be rendered"
    );
    assert_snapshot!("render_svg_shows_footbox_and_lifelines_reach_it", svg);
}

#[test]
fn render_svg_renders_separator_delay_divider_and_spacer_distinctly() {
    let src = fixture("structure/valid_separator_delay_divider_spacer.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");

    assert!(svg.contains("== Stage 1 =="));
    assert!(svg.contains("Midpoint"));
    assert!(svg.contains("wait"));
    assert_snapshot!(
        "render_svg_renders_separator_delay_divider_and_spacer_distinctly",
        svg
    );
}

#[test]
fn render_svg_renders_distinct_participant_kinds() {
    let src = fixture("e2e/participant_kinds.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");

    let assert_count = |pattern: &str, expected: usize, label: &str| {
        assert_eq!(
            svg.match_indices(pattern).count(),
            expected,
            "{label} signature count mismatch for pattern: {pattern}"
        );
    };

    // Each role appears twice (header + footbox), so signatures should appear twice.
    // Queue now renders as a horizontal cylinder with neutral blue palette (no pink/red).
    assert_count(
        "fill=\"#e9f5ff\" stroke=\"#1b5e8a\" stroke-width=\"1\"",
        8, // 6 database cylinder parts + 2 queue main rects
        "database and queue blue rects",
    );
    // Queue ellipse end-caps (2 per participant × 2 positions = 4 total)
    assert_count(
        "fill=\"#d0eaff\" stroke=\"#1b5e8a\" stroke-width=\"1\"",
        4,
        "queue ellipse caps",
    );
    assert_count(
        "x=\"992\" y=\"24\" width=\"24\" height=\"8\"",
        1,
        "collections top tab",
    );
    assert_count(
        "x=\"998\" y=\"26\" width=\"24\" height=\"8\"",
        1,
        "collections stacked tab",
    );
    assert_count(
        "fill=\"#edf7ed\" stroke=\"#2d6a2d\" stroke-width=\"1\"",
        2,
        "control polygon",
    );
    assert_count(
        "x1=\"514\" y1=\"40\" x2=\"614\" y2=\"40\"",
        1,
        "control top midline",
    );
    assert_count(
        "x1=\"514\" y1=\"360\" x2=\"614\" y2=\"360\"",
        1,
        "control footbox midline",
    );
    assert_count(
        "fill=\"#f4f0ff\" stroke=\"#4e3a8f\" stroke-width=\"1\"",
        2,
        "entity base box",
    );
    assert_count(
        "x1=\"670\" y1=\"36\" x2=\"778\" y2=\"36\"",
        1,
        "entity top divider",
    );
    assert_count(
        "x1=\"670\" y1=\"356\" x2=\"778\" y2=\"356\"",
        1,
        "entity footbox divider",
    );
    assert_count("stroke-dasharray=\"5 3\"", 2, "boundary dashed box");
    assert_count(
        "x1=\"350\" y1=\"28\" x2=\"350\" y2=\"52\"",
        1,
        "boundary left rail",
    );
    assert_count(
        "x1=\"458\" y1=\"28\" x2=\"458\" y2=\"52\"",
        1,
        "boundary right rail",
    );
    assert_count(
        "fill=\"#fff3e0\" stroke=\"#8a5a00\" stroke-width=\"1\"",
        2,
        "actor box",
    );
    // Canonical actor head: r=6, stroke-width=1.5 (issue #715)
    assert_count(
        "<circle cx=\"196\" cy=\"25\" r=\"6\" fill=\"none\" stroke=\"#8a5a00\" stroke-width=\"1.5\"/>",
        1,
        "actor head",
    );
    // Canonical actor footbox right leg: hip at y=365, foot at y=381, cx±8 spread
    assert_count(
        "x1=\"196\" y1=\"365\" x2=\"204\" y2=\"381\"",
        1,
        "actor footbox leg",
    );

    assert_snapshot!("render_svg_renders_distinct_participant_kinds", svg);
}
