use insta::assert_snapshot;
use puml::model::{
    Participant, ParticipantRole, SequenceDocument, SequenceEvent, SequenceEventKind,
};
use puml::scene::LayoutOptions;
use puml::theme::SequenceStyle;
use puml::{
    extract_markdown_diagrams, layout, parse_with_pipeline_options, render, FrontendSelection,
    ParsePipelineOptions,
};
use std::collections::HashSet;

const MESSAGE_LABEL_LINE_GAP: i32 = 16;

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

#[test]
fn render_svg_is_deterministic_for_same_input() {
    let src = fixture("e2e/deterministic_sequence.puml");
    let first = puml::render_source_to_svg(&src).expect("first render should succeed");
    let second = puml::render_source_to_svg(&src).expect("second render should succeed");

    assert_eq!(first, second, "render output should be deterministic");
    assert_snapshot!("render_svg_is_deterministic_for_same_input", first);
}

#[test]
fn render_svg_pragma_teoz_boundary_keeps_sequence_render_output_stable() {
    let base = "@startuml\nparticipant A\nparticipant B\nA -> B: hello\n@enduml\n";
    let with_pragma =
        "@startuml\n!pragma teoz true\nparticipant A\nparticipant B\nA -> B: hello\n@enduml\n";

    let base_svg = puml::render_source_to_svg(base).expect("base render");
    let pragma_svg = puml::render_source_to_svg(with_pragma).expect("pragma render");

    assert_eq!(base_svg, pragma_svg);
}

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

    assert!(svg.contains("ref over A, B"));
    assert!(svg.contains("fallback"));
    assert_snapshot!("render_svg_handles_ref_else_and_multi_target_notes", svg);
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
    assert_count(
        "fill=\"#fff0f0\" stroke=\"#8a3030\" stroke-width=\"1\"",
        2,
        "queue",
    );
    assert_count(
        "x1=\"1152\" y1=\"32\" x2=\"1256\" y2=\"32\"",
        1,
        "queue top stripe",
    );
    assert_count(
        "x1=\"1152\" y1=\"352\" x2=\"1256\" y2=\"352\"",
        1,
        "queue footbox stripe",
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
        "fill=\"#e9f5ff\" stroke=\"#1b5e8a\" stroke-width=\"1\"",
        6,
        "database cylinder parts",
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
    assert_count(
        "<circle cx=\"196\" cy=\"34\" r=\"4\" fill=\"none\" stroke=\"#8a5a00\" stroke-width=\"1\"/>",
        1,
        "actor head",
    );
    assert_count(
        "x1=\"196\" y1=\"366\" x2=\"200\" y2=\"372\"",
        1,
        "actor footbox leg",
    );

    assert_snapshot!("render_svg_renders_distinct_participant_kinds", svg);
}

#[test]
fn overflow_scene_text_anchors_stay_within_note_and_group_bounds() {
    let src = fixture("overflow/overflow_notes_refs_groups.puml");
    let ast = puml::parse(&src).expect("parse should succeed");
    let doc = puml::normalize(ast).expect("normalize should succeed");
    let scene = layout::layout(&doc, LayoutOptions::default());

    for note in &scene.notes {
        for (idx, _) in note.text.lines().enumerate() {
            let text_y = note.y + 20 + (idx as i32 * 16);
            assert!(
                text_y > note.y && text_y <= note.y + note.height,
                "note text baseline should stay within note rect bounds"
            );
        }
    }

    for group in &scene.groups {
        if let Some(label) = &group.label {
            let header_y = group.y + 16;
            assert!(
                header_y > group.y && header_y <= group.y + group.height,
                "group header baseline should stay within group rect bounds"
            );
            if group.kind.eq_ignore_ascii_case("ref") {
                for (idx, _) in label.lines().skip(1).enumerate() {
                    let text_y = group.y + 32 + (idx as i32 * 16);
                    assert!(
                        text_y > group.y && text_y <= group.y + group.height,
                        "ref body baseline should stay within ref rect bounds"
                    );
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct SvgRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    fill: String,
}

#[derive(Debug, Clone)]
struct SvgText {
    x: i32,
    y: i32,
    text: String,
}

fn parse_svg_attr(tag: &str, key: &str) -> Option<String> {
    let pat = format!("{key}=\"");
    let start = tag.find(&pat)? + pat.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn parse_svg_rects(svg: &str) -> Vec<SvgRect> {
    let mut rects = Vec::new();
    for chunk in svg.split("<rect ").skip(1) {
        let Some(end) = chunk.find("/>") else {
            continue;
        };
        let tag = &chunk[..end];
        let (Some(x), Some(y), Some(width), Some(height), Some(fill)) = (
            parse_svg_attr(tag, "x").and_then(|v| v.parse::<i32>().ok()),
            parse_svg_attr(tag, "y").and_then(|v| v.parse::<i32>().ok()),
            parse_svg_attr(tag, "width").and_then(|v| v.parse::<i32>().ok()),
            parse_svg_attr(tag, "height").and_then(|v| v.parse::<i32>().ok()),
            parse_svg_attr(tag, "fill"),
        ) else {
            continue;
        };
        rects.push(SvgRect {
            x,
            y,
            width,
            height,
            fill,
        });
    }
    rects
}

fn parse_svg_texts(svg: &str) -> Vec<SvgText> {
    let mut texts = Vec::new();
    for chunk in svg.split("<text ").skip(1) {
        let Some(close) = chunk.find('>') else {
            continue;
        };
        let attrs = &chunk[..close];
        let body = &chunk[close + 1..];
        let Some(end) = body.find("</text>") else {
            continue;
        };
        let content = body[..end].to_string();
        let (Some(x), Some(y)) = (
            parse_svg_attr(attrs, "x").and_then(|v| v.parse::<i32>().ok()),
            parse_svg_attr(attrs, "y").and_then(|v| v.parse::<i32>().ok()),
        ) else {
            continue;
        };
        texts.push(SvgText {
            x,
            y,
            text: content,
        });
    }
    texts
}

fn parse_svg_viewbox_width(svg: &str) -> Option<i32> {
    let svg_tag = svg.split("<svg ").nth(1)?.split('>').next()?;
    let viewbox = parse_svg_attr(svg_tag, "viewBox")?;
    let mut parts = viewbox.split_whitespace();
    let _min_x = parts.next()?;
    let _min_y = parts.next()?;
    let width = parts.next()?.parse::<i32>().ok()?;
    Some(width)
}

#[test]
fn overflow_svg_text_positions_stay_within_associated_rects() {
    let src = fixture("overflow/overflow_notes_refs_groups.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");
    let rects = parse_svg_rects(&svg);
    let texts = parse_svg_texts(&svg);

    let note_rects = rects
        .iter()
        .filter(|r| r.fill == "#fff8c4")
        .collect::<Vec<_>>();
    let group_rects = rects
        .iter()
        .filter(|r| r.fill == "#eef6ff" || r.fill == "#fafafa")
        .collect::<Vec<_>>();
    assert!(!note_rects.is_empty(), "expected at least one note rect");
    assert!(
        !group_rects.is_empty(),
        "expected at least one group/ref rect"
    );

    let tracked = [
        "note_line_one_for_bounds_guardrail",
        "note_line_two_for_bounds_guardrail",
        "note_line_three_for_bounds_guardrail",
        "alt branch_label_for_bounds_guardrail",
        "ref over A, B",
        "ref_line_one_for_bounds_guardrail",
        "ref_line_two_for_bounds_guardrail",
        "ref_line_three_for_bounds_guardrail",
        "ref_line_four_for_bounds_guardrail",
    ];

    let mut seen = HashSet::new();
    for text in texts {
        if !tracked.contains(&text.text.as_str()) {
            continue;
        }
        seen.insert(text.text.clone());
        let owner = note_rects
            .iter()
            .copied()
            .chain(group_rects.iter().copied())
            .find(|r| {
                text.x >= r.x && text.x <= r.x + r.width && text.y > r.y && text.y <= r.y + r.height
            });
        assert!(
            owner.is_some(),
            "tracked text should stay inside associated note/ref/group rect bounds: {}",
            text.text
        );
    }

    for expected in tracked {
        assert!(
            seen.contains(expected),
            "expected tracked overflow guardrail text in svg: {expected}"
        );
    }
}

#[test]
fn render_svg_wraps_long_message_labels_without_viewbox_clipping() {
    let src = fixture("overflow/overflow_message_labels.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");

    assert!(svg.contains("LEFTE"));
    assert!(svg.contains("CENTEROVERFLOWTOKEN"));
    assert!(svg.contains("RIGHT"));
    assert_snapshot!(
        "render_svg_wraps_long_message_labels_without_viewbox_clipping",
        svg
    );
}

#[test]
fn overflow_message_label_positions_stay_within_scene_viewbox() {
    let src = fixture("overflow/overflow_message_labels.puml");
    let ast = puml::parse(&src).expect("parse should succeed");
    let doc = puml::normalize(ast).expect("normalize should succeed");
    let scene = layout::layout(&doc, LayoutOptions::default());

    for message in &scene.messages {
        if message.label_lines.is_empty() {
            continue;
        }
        let tx = ((message.x1 + message.x2) / 2) + 2;
        let start_y =
            message.y - 8 - (((message.label_lines.len() as i32) - 1) * MESSAGE_LABEL_LINE_GAP);
        for (idx, line) in message.label_lines.iter().enumerate() {
            let width = (line.chars().count() as i32) * 7;
            let left = tx - (width / 2);
            let right = tx + (width / 2);
            let y = start_y + (idx as i32 * MESSAGE_LABEL_LINE_GAP);

            assert!(left >= 0, "message label left edge should be in viewBox");
            assert!(
                right <= scene.width,
                "message label right edge should be in viewBox"
            );
            assert!(y >= 0, "message label baseline should be in viewBox");
            assert!(
                y <= scene.height,
                "message label baseline should be in viewBox"
            );
        }
    }
}

#[test]
fn overflow_unbroken_tokens_stay_within_note_and_ref_rects() {
    let src = fixture("overflow/overflow_unbroken_tokens.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");
    let rects = parse_svg_rects(&svg);
    let texts = parse_svg_texts(&svg);

    let note_rects = rects
        .iter()
        .filter(|r| r.fill == "#fff8c4")
        .collect::<Vec<_>>();
    let ref_rects = rects
        .iter()
        .filter(|r| r.fill == "#eef6ff")
        .collect::<Vec<_>>();

    assert!(!note_rects.is_empty(), "expected note rects");
    assert!(!ref_rects.is_empty(), "expected ref rects");

    let tracked = [
        "note_unbroken_token_AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "ref_unbroken_token_BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB",
    ];

    for token in tracked {
        let text = texts
            .iter()
            .find(|t| t.text == token)
            .unwrap_or_else(|| panic!("expected token in svg: {token}"));
        let owner = note_rects
            .iter()
            .copied()
            .chain(ref_rects.iter().copied())
            .find(|r| {
                text.x >= r.x && text.x <= r.x + r.width && text.y > r.y && text.y <= r.y + r.height
            });
        assert!(
            owner.is_some(),
            "unbroken overflow token should stay inside note/ref bounds: {token}"
        );
    }

    assert_snapshot!(
        "overflow_unbroken_tokens_stay_within_note_and_ref_rects",
        svg
    );
}

#[test]
fn overflow_advanced_note_ref_forms_do_not_overlap_and_render_deterministically() {
    let src = fixture("overflow/overflow_note_ref_advanced_forms_nonoverlap.puml");
    let ast = puml::parse(&src).expect("parse should succeed");
    let doc = puml::normalize(ast).expect("normalize should succeed");
    let scene = layout::layout(&doc, LayoutOptions::default());

    let mut blocks = Vec::new();
    for note in &scene.notes {
        blocks.push(("note", note.y, note.y + note.height));
    }
    for group in &scene.groups {
        if group.kind.eq_ignore_ascii_case("ref") {
            blocks.push(("ref", group.y, group.y + group.height));
        }
    }

    blocks.sort_by_key(|(_, y, _)| *y);
    for window in blocks.windows(2) {
        let (first_kind, _first_y, first_bottom) = window[0];
        let (second_kind, second_y, _second_bottom) = window[1];
        assert!(
            second_y >= first_bottom,
            "advanced annotation boxes should not overlap: {first_kind} bottom {} > {second_kind} top {}",
            first_bottom,
            second_y
        );
    }

    let svg = render::render_svg(&scene);
    let rerendered = puml::render_source_to_svg(&src).expect("render should succeed");
    assert_eq!(svg, rerendered, "render output should be deterministic");
    assert_snapshot!(
        "overflow_advanced_note_ref_forms_do_not_overlap_and_render_deterministically",
        svg
    );
}

#[test]
fn overflow_multiline_group_ref_note_combo_stays_within_rects() {
    let src = fixture("overflow/overflow_multiline_group_ref_note_combo.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");
    let rects = parse_svg_rects(&svg);
    let texts = parse_svg_texts(&svg);

    let note_rects = rects
        .iter()
        .filter(|r| r.fill == "#fff8c4")
        .collect::<Vec<_>>();
    let group_rects = rects
        .iter()
        .filter(|r| r.fill == "#eef6ff" || r.fill == "#fafafa")
        .collect::<Vec<_>>();
    let viewbox_width = parse_svg_viewbox_width(&svg).expect("svg should include viewBox width");

    assert!(!note_rects.is_empty(), "expected note rects");
    assert!(!group_rects.is_empty(), "expected group/ref rects");

    let tracked = [
        "combo_note_line_1_with_a_very_long_unbroken_token_CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC",
        "combo_ref_line_1_with_a_very_long_unbroken_token_DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD",
        "fallback_note_line_1_with_long_unbroken_token_EEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEEE",
    ];

    for line in tracked {
        let text = texts
            .iter()
            .find(|t| t.text == line)
            .unwrap_or_else(|| panic!("expected combo overflow text in svg: {line}"));
        let owner = note_rects
            .iter()
            .copied()
            .chain(group_rects.iter().copied())
            .find(|r| {
                text.x >= r.x && text.x <= r.x + r.width && text.y > r.y && text.y <= r.y + r.height
            });
        assert!(
            owner.is_some(),
            "combo overflow text should stay within associated rects: {line}"
        );
        if let Some(note_rect) = note_rects
            .iter()
            .copied()
            .find(|r| text.x >= r.x && text.y > r.y && text.y <= r.y + r.height)
        {
            let conservative_right = text.x + ((text.text.chars().count() as i32) * 7);
            assert!(
                conservative_right <= note_rect.x + note_rect.width,
                "long note text should fit note rect width without right-edge clipping: {line}"
            );
            assert!(
                conservative_right <= viewbox_width,
                "long note text should fit scene viewBox width without right-edge clipping: {line}"
            );
        }
    }

    assert_snapshot!(
        "overflow_multiline_group_ref_note_combo_stays_within_rects",
        svg
    );
}

#[test]
fn overflow_dense_participant_headers_keep_text_inside_header_boxes() {
    let src = fixture("overflow/overflow_dense_participant_headers.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");
    let rects = parse_svg_rects(&svg);
    let texts = parse_svg_texts(&svg);

    let participant_rects = rects
        .iter()
        .filter(|r| r.fill == "#f6f6f6")
        .collect::<Vec<_>>();
    assert!(
        participant_rects.len() >= 6,
        "expected participant header and footbox rects"
    );

    let tracked_prefixes = [
        "ParticipantHeaderAlpha",
        "ParticipantHeaderBeta",
        "ParticipantHeaderGamma",
        "ParticipantHeaderDelta",
        "ParticipantHeaderEpsilon",
        "ParticipantHeaderZeta",
    ];

    for text in texts {
        if !tracked_prefixes.iter().any(|p| text.text.starts_with(p)) {
            continue;
        }
        let owner = participant_rects.iter().copied().find(|r| {
            text.x >= r.x && text.x <= r.x + r.width && text.y > r.y && text.y <= r.y + r.height
        });
        assert!(
            owner.is_some(),
            "dense participant header text should stay inside participant box: {}",
            text.text
        );
    }

    assert_snapshot!(
        "overflow_dense_participant_headers_keep_text_inside_header_boxes",
        svg
    );
}

#[test]
fn lifelines_start_below_wrapped_participant_headers() {
    let src = "@startuml\nparticipant \"Participant Header With Many Wrapped Words For Height Growth\" as P\nP -> P: ping\n@enduml\n";
    let doc = puml::parse(src).expect("parse");
    let model = puml::normalize(doc).expect("normalize");
    let scene = layout::layout(&model, LayoutOptions::default());

    let participant = scene
        .participants
        .iter()
        .find(|p| p.id == "P")
        .expect("participant");
    let lifeline = scene
        .lifelines
        .iter()
        .find(|l| l.participant_id == "P")
        .expect("lifeline");

    assert_eq!(
        lifeline.y1,
        participant.y + participant.height,
        "lifeline should start at participant box bottom"
    );
}
