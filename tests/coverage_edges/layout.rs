use crate::common::*;

#[test]
fn layout_applies_autonumber_stop_and_restart() {
    let src = "@startuml\nautonumber\nA -> B : one\nautonumber stop\nB -> A : two\nautonumber 5\nA -> B : three\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let scene = layout::layout(&model, LayoutOptions::default());

    let labels = scene
        .messages
        .iter()
        .map(|m| m.label.clone().unwrap_or_default())
        .collect::<Vec<_>>();

    assert_eq!(labels, vec!["1 one", "two", "5 three"]);
}

#[test]
fn layout_handles_return_without_caller() {
    let doc = SequenceDocument {
        participants: vec![Participant {
            id: "A".to_string(),
            display: "A".to_string(),
            role: ParticipantRole::Participant,
            explicit: true,
        }],
        events: vec![SequenceEvent {
            span: puml::source::Span { start: 0, end: 0 },
            kind: SequenceEventKind::Return {
                label: Some("noop".to_string()),
                from: None,
                to: None,
            },
        }],
        ..SequenceDocument::default()
    };

    let scene = layout::layout(&doc, LayoutOptions::default());
    assert!(scene.messages.is_empty());
}

#[test]
fn layout_wraps_participant_labels_and_grows_boxes_by_default() {
    let doc = SequenceDocument {
        participants: vec![Participant {
            id: "A".to_string(),
            display: "A very long participant label that should wrap".to_string(),
            role: ParticipantRole::Participant,
            explicit: true,
        }],
        ..SequenceDocument::default()
    };
    let scene = layout::layout(&doc, LayoutOptions::default());

    let participant = scene
        .participants
        .iter()
        .find(|p| p.id == "A")
        .expect("expected participant A");
    assert!(participant.display_lines.len() > 1);
    assert!(participant.height > LayoutOptions::default().participant_height);

    let svg = render::render_svg(&scene);
    assert!(svg.contains(">A very long<"));
    assert!(svg.contains(">participant<"));
}

#[test]
fn layout_uses_ellipsis_for_single_line_overflow_policy() {
    let doc = SequenceDocument {
        participants: vec![Participant {
            id: "A".to_string(),
            display: "A very long participant label that should truncate".to_string(),
            role: ParticipantRole::Participant,
            explicit: true,
        }],
        ..SequenceDocument::default()
    };
    let options = LayoutOptions {
        text_overflow_policy: TextOverflowPolicy::EllipsisSingleLine,
        ..LayoutOptions::default()
    };
    let scene = layout::layout(&doc, options);

    let participant = scene
        .participants
        .iter()
        .find(|p| p.id == "A")
        .expect("expected participant A");
    assert_eq!(participant.display_lines.len(), 1);
    assert!(participant.display_lines[0].contains('…'));
    assert_eq!(
        participant.height,
        LayoutOptions::default().participant_height
    );
}

#[test]
fn layout_expands_rows_for_wrapped_labels_and_open_group_tail() {
    let model = SequenceDocument {
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
                span: Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "A".to_string(),
                    to: "B".to_string(),
                    arrow: "->".to_string(),
                    label: Some(
                        "one two three four five six seven eight nine ten eleven twelve"
                            .to_string(),
                    ),
                    style: Default::default(),
                    from_virtual: None,
                    to_virtual: None,
                },
            },
            SequenceEvent {
                span: Span { start: 0, end: 0 },
                kind: SequenceEventKind::GroupStart {
                    kind: "group".to_string(),
                    label: Some("over   ".to_string()),
                },
            },
            SequenceEvent {
                span: Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "B".to_string(),
                    to: "A".to_string(),
                    arrow: "->".to_string(),
                    label: Some("short".to_string()),
                    style: Default::default(),
                    from_virtual: None,
                    to_virtual: None,
                },
            },
        ],
        ..SequenceDocument::default()
    };
    let options = LayoutOptions::default();
    let scene = layout::layout(&model, options);

    assert_eq!(scene.messages.len(), 2);
    let first = &scene.messages[0];
    let second = &scene.messages[1];
    assert!(
        first.label_lines.len() > 1,
        "expected wrapped first label to consume multiple rows"
    );
    assert!(
        second.y - first.y > options.message_row_height,
        "wrapped row should push following message downward"
    );
    assert_eq!(
        scene.groups.len(),
        1,
        "open group should be finalized at end"
    );
    assert!(
        scene.groups[0].height >= options.message_row_height,
        "finalized open group should have non-zero height"
    );
}

#[test]
fn layout_offsets_virtual_endpoints_for_overlap_cases() {
    let doc = SequenceDocument {
        participants: vec![Participant {
            id: "A".to_string(),
            display: "A".to_string(),
            role: ParticipantRole::Participant,
            explicit: true,
        }],
        events: vec![
            SequenceEvent {
                span: Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "ghost-left".to_string(),
                    to: "ghost-right".to_string(),
                    arrow: "->".to_string(),
                    label: Some("both virtual".to_string()),
                    style: Default::default(),
                    from_virtual: Some(VirtualEndpoint {
                        side: VirtualEndpointSide::Left,
                        kind: VirtualEndpointKind::Filled,
                    }),
                    to_virtual: Some(VirtualEndpoint {
                        side: VirtualEndpointSide::Right,
                        kind: VirtualEndpointKind::Filled,
                    }),
                },
            },
            SequenceEvent {
                span: Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "ghost-right".to_string(),
                    to: "A".to_string(),
                    arrow: "->".to_string(),
                    label: Some("from virtual".to_string()),
                    style: Default::default(),
                    from_virtual: Some(VirtualEndpoint {
                        side: VirtualEndpointSide::Right,
                        kind: VirtualEndpointKind::Circle,
                    }),
                    to_virtual: None,
                },
            },
            SequenceEvent {
                span: Span { start: 0, end: 0 },
                kind: SequenceEventKind::Message {
                    from: "A".to_string(),
                    to: "ghost-left".to_string(),
                    arrow: "->".to_string(),
                    label: Some("to virtual".to_string()),
                    style: Default::default(),
                    from_virtual: None,
                    to_virtual: Some(VirtualEndpoint {
                        side: VirtualEndpointSide::Left,
                        kind: VirtualEndpointKind::Cross,
                    }),
                },
            },
        ],
        ..SequenceDocument::default()
    };
    let scene = layout::layout(&doc, LayoutOptions::default());
    let center = scene.participants[0].x + (scene.participants[0].width / 2);

    assert_eq!(scene.messages[0].x1, center - 56);
    assert_eq!(scene.messages[0].x2, center + 56);
    assert_eq!(scene.messages[1].x1, center + 56);
    assert_eq!(scene.messages[1].x2, center);
    assert_eq!(scene.messages[2].x1, center);
    assert_eq!(scene.messages[2].x2, center - 56);
}

#[test]
fn layout_pages_preserve_page_titles_and_footer_reserve_without_footboxes() {
    let src = "@startuml\ntitle Base Title\nfooter Shared Footer\nhide footbox\nA -> B : one\nnewpage Page Two\nB -> A : two\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let options = LayoutOptions::default();
    let pages = layout::layout_pages(&model, options);

    assert_eq!(pages.len(), 2);
    assert_eq!(
        pages[0]
            .title
            .as_ref()
            .map(|t| t.lines.join(" "))
            .as_deref(),
        Some("Base Title")
    );
    assert_eq!(
        pages[1]
            .title
            .as_ref()
            .map(|t| t.lines.join(" "))
            .as_deref(),
        Some("Page Two")
    );
    for scene in &pages {
        assert!(
            scene.footboxes.is_empty(),
            "hide footbox should remove footboxes"
        );
        for lifeline in &scene.lifelines {
            assert!(
                scene.height - lifeline.y2 >= options.footer_height + options.margin,
                "footer reserve should still remain when footboxes are hidden"
            );
        }
    }
}

#[test]
fn layout_group_else_separator_and_ref_min_height_are_deterministic() {
    let src = fs::read_to_string(fixture("groups/valid_ref_and_else_rendering.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let options = LayoutOptions::default();
    let scene = layout::layout(&model, options);

    let alt = scene
        .groups
        .iter()
        .find(|g| g.kind.eq_ignore_ascii_case("alt"))
        .expect("expected alt group");
    assert!(
        !alt.separators.is_empty(),
        "alt group should capture else separator rows"
    );
    assert!(alt.height >= options.message_row_height);

    let reference = scene
        .groups
        .iter()
        .find(|g| g.kind.eq_ignore_ascii_case("ref"))
        .expect("expected ref group");
    assert!(
        reference.height > options.message_row_height,
        "ref group should use computed min content height"
    );
}

#[test]
fn layout_ref_advanced_forms_stay_above_footboxes() {
    let src = fs::read_to_string(fixture("groups/valid_ref_advanced_forms.puml"))
        .expect("fixture should load");
    let doc = parse(&src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let scene = layout::layout(&model, LayoutOptions::default());

    assert!(!scene.footboxes.is_empty(), "expected footboxes");
    let footbox_top = scene
        .footboxes
        .iter()
        .map(|b| b.y)
        .min()
        .expect("expected a footbox");
    let refs = scene
        .groups
        .iter()
        .filter(|g| g.kind.eq_ignore_ascii_case("ref"))
        .collect::<Vec<_>>();
    assert!(!refs.is_empty(), "expected ref groups from fixture");
    for reference in refs {
        assert!(
            reference.y + reference.height <= footbox_top,
            "ref group bottom should stay above footbox top"
        );
    }
}

#[test]
fn layout_overflow_bounds_keep_multi_target_note_and_over_group_in_view() {
    let src = "@startuml\nparticipant AlphaParticipantWithLongName\nparticipant BetaParticipantWithLongName\nparticipant GammaParticipantWithLongName\nnote right of AlphaParticipantWithLongName, BetaParticipantWithLongName: right note with a very long unbroken token AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\ngroup over AlphaParticipantWithLongName, GammaParticipantWithLongName : over Alpha/Gamma\nAlphaParticipantWithLongName -> GammaParticipantWithLongName : ping\nend\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let scene = layout::layout(&model, LayoutOptions::default());
    assert!(!scene.notes.is_empty(), "expected multi-target note");
    assert!(!scene.groups.is_empty(), "expected over group");
    assert!(
        scene
            .notes
            .iter()
            .all(|note| note.x >= 0 && note.x + note.width <= scene.width),
        "notes should stay within scene width"
    );
}

#[test]
fn layout_left_of_first_participant_note_keeps_margin_and_lane_gap() {
    let src = "@startuml\nA -> B : ping\nnote left of A: left guardrail\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let options = LayoutOptions::default();
    let scene = layout::layout(&model, options);
    let participant = scene
        .participants
        .iter()
        .find(|participant| participant.id == "A")
        .expect("participant A");
    let note = scene
        .notes
        .iter()
        .find(|note| note.text.contains("left guardrail"))
        .expect("left note");

    assert_eq!(note.x, options.margin);
    assert_eq!(note.x + note.width + 12, participant.x);
}

#[test]
fn layout_expands_width_for_long_header_and_footer_metadata() {
    let src = "@startuml\nheader HEADER_WITH_A_VERY_LONG_METADATA_LINE_FOR_LAYOUT_GUARDRAIL_ABCDEFGHIJKLMNOPQRSTUVWXYZ\nfooter FOOTER_WITH_A_VERY_LONG_METADATA_LINE_FOR_LAYOUT_GUARDRAIL_ABCDEFGHIJKLMNOPQRSTUVWXYZ\nA -> B : ping\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let scene = layout::layout(&model, LayoutOptions::default());

    let margin = LayoutOptions::default().margin;
    let max_metadata_right = scene
        .header
        .iter()
        .chain(scene.footer.iter())
        .flat_map(|label| {
            label
                .lines
                .iter()
                .map(|line| label.x + (line.chars().count() as i32 * 7) + margin)
        })
        .max()
        .expect("expected metadata lines");
    assert!(
        scene.width >= max_metadata_right,
        "scene width should include full header/footer metadata width"
    );
}

#[test]
fn layout_expands_width_for_long_legend_lines() {
    let src = "@startuml\nlegend right\nLEGEND_WITH_A_VERY_LONG_LINE_THAT_MUST_BE_VISIBLE_WITHOUT_CANVAS_CLIPPING_ABCDEFGHIJKLMNOPQRSTUVWXYZ\nend legend\nA -> B : ping\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let scene = layout::layout(&model, LayoutOptions::default());

    let margin = LayoutOptions::default().margin;
    let legend_line = scene
        .legend_text
        .as_deref()
        .and_then(|text| text.lines().next())
        .expect("expected legend line");
    let max_right = (legend_line.chars().count() as i32 * 7) + 16 + (margin * 2);
    assert!(
        scene.width >= max_right,
        "scene width should include long legend text extent"
    );
}
