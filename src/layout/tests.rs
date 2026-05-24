use super::*;
use crate::layout::autonumber::{format_autonumber, parse_autonumber_command, AutonumberCounter};
use crate::layout::text::{ellipsize, wrap_line};
use crate::model::{
    Participant, ParticipantRole, SequenceDocument, SequenceEvent, SequenceEventKind,
    VirtualEndpoint, VirtualEndpointSide,
};
use crate::render::text_metrics::chunk_text;
use crate::source::Span;

#[test]
fn return_event_with_ids_is_laid_out_with_default_centers_for_unknown_participants() {
    let doc = SequenceDocument {
        participants: vec![Participant {
            id: "A".to_string(),
            display: "A".to_string(),
            role: ParticipantRole::Participant,
            explicit: true,
        }],
        events: vec![SequenceEvent {
            span: Span { start: 0, end: 0 },
            kind: SequenceEventKind::Return {
                label: Some("back".to_string()),
                from: Some("missing-from".to_string()),
                to: Some("missing-to".to_string()),
            },
        }],
        ..SequenceDocument::default()
    };
    let scene = layout(&doc, LayoutOptions::default());
    assert_eq!(scene.messages.len(), 1);
    assert_eq!(scene.messages[0].arrow, "-->>");
}

#[test]
fn text_helpers_cover_empty_whitespace_and_extreme_limits() {
    assert_eq!(wrap_line("", 8), vec![String::new()]);
    assert_eq!(wrap_line("   ", 8), vec![String::new()]);
    assert_eq!(
        wrap_line("seed abcdefghijklmnop", 4),
        vec!["seed", "abcd", "efgh", "ijkl", "mnop"]
    );
    assert_eq!(chunk_text("abc", 0), vec!["abc".to_string()]);
    assert_eq!(chunk_text("", 3), vec![String::new()]);
    assert_eq!(ellipsize("abc", 8), "abc");
    assert_eq!(ellipsize("abc", 0), "");
    assert_eq!(ellipsize("abc", 1), "…");
}

#[test]
fn geometry_and_autonumber_edge_branches_are_deterministic() {
    let options = LayoutOptions::default();
    let mut centers = BTreeMap::new();
    let mut bounds = BTreeMap::new();
    let center = options.margin + options.participant_width / 2;
    bounds.insert(
        "A".to_string(),
        (options.margin, options.margin + options.participant_width),
    );
    centers.insert("A".to_string(), center);

    let (x, _) = note_horizontal_bounds("right", None, &centers, &bounds, 300, 120, &options);
    assert_eq!(x, options.margin);
    let (x_mid, _) =
        note_horizontal_bounds("over", Some("A"), &centers, &bounds, 300, 120, &options);
    assert_eq!(x_mid, center - 60);

    let (gx, gw) = group_horizontal_bounds("group", Some("over   "), &bounds, &options);
    assert_eq!(gx, options.margin);
    assert!(gw >= options.participant_width);
    assert_eq!(group_content_min_size("group", None), (0, 0));

    assert_eq!(row_units_for_height(40, 0), 1);
    assert_eq!(
        message_x_bounds(
            "x",
            "y",
            Some(VirtualEndpoint {
                side: VirtualEndpointSide::Right,
                kind: crate::model::VirtualEndpointKind::Filled,
            }),
            Some(VirtualEndpoint {
                side: VirtualEndpointSide::Left,
                kind: crate::model::VirtualEndpointKind::Filled,
            }),
            &centers,
            &options,
        ),
        (center + 56, center - 56)
    );

    let parsed = parse_autonumber_command("resume");
    assert!(parsed.resume_only);
    let parsed_fmt = parse_autonumber_command("resume fmt");
    assert_eq!(parsed_fmt.format.as_deref(), Some("fmt"));
    let parsed_dotted = parse_autonumber_command("1.02.003 4");
    assert_eq!(
        parsed_dotted.start.as_ref().map(AutonumberCounter::render),
        Some("1.02.003".to_string())
    );
    assert_eq!(parsed_dotted.step, Some(4));
    let mut auton = AutonumberState::default();
    auton.update(None);
    assert_eq!(auton.apply(Some(String::new())).as_deref(), Some("1"));
    let counter = AutonumberCounter::from_number(7);
    assert_eq!(format_autonumber(&counter, Some("")), "7");
    assert_eq!(format_autonumber(&counter, Some("item")), "item7");
    assert_eq!(format_autonumber(&counter, Some("n=#")), "n=7");
    assert_eq!(format_autonumber(&counter, Some("n=###")), "n=007");
}

#[test]
fn autonumber_resume_and_zero_state_fallbacks_are_covered() {
    let mut state = AutonumberState::default();
    state.update(Some("resume"));
    assert_eq!(state.next.render(), "1");

    let mut state = AutonumberState {
        enabled: true,
        next: AutonumberCounter::default(),
        step: 0,
        format: None,
    };
    assert_eq!(state.apply(None).as_deref(), Some("1"));

    let mut state = AutonumberState {
        enabled: false,
        next: AutonumberCounter::from_number(8),
        step: 0,
        format: None,
    };
    state.update(Some("resume"));
    assert_eq!(state.step, 1);

    let mut state = AutonumberState::default();
    state.update(Some("1.02.003"));
    assert_eq!(
        state.apply(Some("dotted".to_string())).as_deref(),
        Some("1.02.003 dotted")
    );
    assert_eq!(
        state.apply(Some("next".to_string())).as_deref(),
        Some("1.02.004 next")
    );

    let bounds: BTreeMap<String, (i32, i32)> = BTreeMap::new();
    let (_gx, gw) = group_horizontal_bounds("group", None, &bounds, &LayoutOptions::default());
    assert!(gw >= LayoutOptions::default().participant_width);
}

#[test]
fn wrapped_first_message_label_starts_below_participant_header() {
    let doc = SequenceDocument {
        participants: vec![
            Participant {
                id: "Alice".to_string(),
                display: "Alice".to_string(),
                role: ParticipantRole::Participant,
                explicit: true,
            },
            Participant {
                id: "Bob".to_string(),
                display: "Bob".to_string(),
                role: ParticipantRole::Participant,
                explicit: true,
            },
        ],
        events: vec![SequenceEvent {
            span: Span { start: 0, end: 0 },
            kind: SequenceEventKind::Message {
                from: "Alice".to_string(),
                to: "Bob".to_string(),
                arrow: "->".to_string(),
                label: Some(
                    "This is a very long message that demonstrates maxMessageSize wrapping"
                        .to_string(),
                ),
                style: Default::default(),
                from_virtual: None,
                to_virtual: None,
            },
        }],
        ..SequenceDocument::default()
    };
    let scene = layout(&doc, LayoutOptions::default());
    assert_eq!(scene.messages.len(), 1);
    let msg = &scene.messages[0];
    assert!(
        msg.label_lines.len() > 1,
        "fixture should force wrapped message label lines"
    );
    let participant_bottom = scene.participants[0].y + scene.participants[0].height;
    let label_top = msg.y - 8 - (((msg.label_lines.len() as i32) - 1) * TEXT_LINE_HEIGHT);
    assert!(
            label_top >= participant_bottom + 8,
            "wrapped message label top ({label_top}) should be below participant header bottom ({participant_bottom})"
        );
}

#[test]
fn group_horizontal_bounds_default_span_matches_participants() {
    let options = LayoutOptions::default();
    let mut bounds = BTreeMap::new();
    bounds.insert(
        "A".to_string(),
        (options.margin, options.margin + options.participant_width),
    );
    bounds.insert(
        "B".to_string(),
        (
            options.margin + options.participant_spacing,
            options.margin + options.participant_spacing + options.participant_width,
        ),
    );
    bounds.insert(
        "C".to_string(),
        (
            options.margin + (options.participant_spacing * 2),
            options.margin + (options.participant_spacing * 2) + options.participant_width,
        ),
    );

    let (x, width) = group_horizontal_bounds("alt", Some("branch"), &bounds, &options);
    let right = x + width;
    let participant_right =
        options.margin + (options.participant_spacing * 2) + options.participant_width;
    assert_eq!(x, options.margin);
    assert_eq!(
        right, participant_right,
        "default alt/opt/loop-style frame should end at the rightmost participant edge"
    );
}
