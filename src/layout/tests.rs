use std::collections::BTreeMap;

use super::autonumber::{
    format_autonumber, parse_autonumber_command, AutonumberCounter, AutonumberState,
};
use super::groups::{group_content_min_size, group_horizontal_bounds};
use super::messages::message_x_bounds;
use super::metrics::row_units_for_height;
use super::notes::note_horizontal_bounds;
use super::text::{chunk_text, ellipsize, wrap_line};
use super::*;
use crate::model::{
    Participant, ParticipantRole, SequenceDocument, SequenceEvent, SequenceEventKind,
    VirtualEndpoint, VirtualEndpointKind, VirtualEndpointSide,
};
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
    assert!(gw >= options.participant_width + 64);
    assert_eq!(group_content_min_size("group", None), (0, 0));

    assert_eq!(row_units_for_height(40, 0), 1);
    assert_eq!(
        message_x_bounds(
            "x",
            "y",
            Some(VirtualEndpoint {
                side: VirtualEndpointSide::Right,
                kind: VirtualEndpointKind::Filled,
            }),
            Some(VirtualEndpoint {
                side: VirtualEndpointSide::Left,
                kind: VirtualEndpointKind::Filled,
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
    assert!(gw >= LayoutOptions::default().participant_width + 64);
}
