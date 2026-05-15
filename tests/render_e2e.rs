use insta::assert_snapshot;
use puml::model::{
    Participant, ParticipantRole, SequenceDocument, SequenceEvent, SequenceEventKind,
};
use puml::scene::LayoutOptions;
use puml::{layout, render};

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
                },
            },
            SequenceEvent {
                span: puml::source::Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "A".to_string(),
                    to: "A".to_string(),
                    arrow: "->".to_string(),
                    label: Some("self".to_string()),
                },
            },
            SequenceEvent {
                span: puml::source::Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "A".to_string(),
                    to: "[*]".to_string(),
                    arrow: "->".to_string(),
                    label: Some("lost".to_string()),
                },
            },
            SequenceEvent {
                span: puml::source::Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "A".to_string(),
                    to: "B".to_string(),
                    arrow: "-->".to_string(),
                    label: Some("modifier-syntax-safe".to_string()),
                },
            },
        ],
        title: None,
        header: None,
        footer: None,
        caption: None,
        legend: None,
        skinparams: vec![],
        footbox_visible: true,
        warnings: vec![],
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
fn render_svg_handles_ref_else_and_multi_target_notes() {
    let src = fixture("groups/valid_ref_and_else_rendering.puml");
    let svg = puml::render_source_to_svg(&src).expect("render should succeed");

    assert!(svg.contains("ref over A, B"));
    assert!(svg.contains("fallback"));
    assert_snapshot!("render_svg_handles_ref_else_and_multi_target_notes", svg);
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
