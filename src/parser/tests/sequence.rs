use super::*;

#[test]
fn parses_multiline_title_and_legend_blocks() {
    let doc = parse_with_options(
        "title\nLine 1\nLine 2\nend title\nlegend\nAlpha\nBeta\nend legend\nA -> B\n",
        &ParseOptions::default(),
    )
    .unwrap();

    match &doc.statements[0].kind {
        StatementKind::Title(v) => assert_eq!(v, "Line 1\nLine 2"),
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[1].kind {
        StatementKind::Legend(v) => assert_eq!(v, "Alpha\nBeta"),
        other => panic!("unexpected statement: {other:?}"),
    }
    assert!(matches!(doc.statements[2].kind, StatementKind::Message(_)));
}

#[test]
fn parses_multiline_note_block() {
    let doc = parse_with_options(
        "A -> B\nnote right of B\nline 1\nline 2\nend note\n",
        &ParseOptions::default(),
    )
    .unwrap();

    match &doc.statements[1].kind {
        StatementKind::Note(n) => {
            assert_eq!(n.position, "right");
            assert_eq!(n.target.as_deref(), Some("B"));
            assert_eq!(n.text, "line 1\nline 2");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_note_across_without_target() {
    let doc =
        parse_with_options("note across: shared context\n", &ParseOptions::default()).unwrap();

    match &doc.statements[0].kind {
        StatementKind::Note(n) => {
            assert_eq!(n.position, "across");
            assert!(n.target.is_none());
            assert_eq!(n.text, "shared context");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_multiline_note_with_inline_head_text() {
    let doc = parse_with_options(
        "note over A, B: summary\nline 2\nend note\n",
        &ParseOptions::default(),
    )
    .unwrap();

    match &doc.statements[0].kind {
        StatementKind::Note(n) => {
            assert_eq!(n.position, "over");
            assert_eq!(n.target.as_deref(), Some("A, B"));
            assert_eq!(n.text, "summary\nline 2");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_hnote_and_rnote_aliases_as_note() {
    let doc = parse_with_options(
        "hnote over A: alias form\nrnote right of A: rounded alias\n",
        &ParseOptions::default(),
    )
    .unwrap();

    match &doc.statements[0].kind {
        StatementKind::Note(n) => {
            assert_eq!(n.kind, crate::ast::NoteKind::Hexagonal);
            assert_eq!(n.position, "over");
            assert_eq!(n.target.as_deref(), Some("A"));
            assert_eq!(n.text, "alias form");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[1].kind {
        StatementKind::Note(n) => {
            assert_eq!(n.kind, crate::ast::NoteKind::Rectangle);
            assert_eq!(n.position, "right");
            assert_eq!(n.target.as_deref(), Some("A"));
            assert_eq!(n.text, "rounded alias");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_hnote_and_rnote_multiline_terminators() {
    let doc = parse_with_options(
        "hnote over A\nhex body\nendhnote\nrnote over B\nrect body\nendrnote\n",
        &ParseOptions::default(),
    )
    .unwrap();

    match &doc.statements[0].kind {
        StatementKind::Note(n) => {
            assert_eq!(n.kind, crate::ast::NoteKind::Hexagonal);
            assert_eq!(n.text, "hex body");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[1].kind {
        StatementKind::Note(n) => {
            assert_eq!(n.kind, crate::ast::NoteKind::Rectangle);
            assert_eq!(n.text, "rect body");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_multiline_ref_with_inline_head_text() {
    let doc = parse_with_options(
        "ref over A, B: summary\nline 2\nend ref\n",
        &ParseOptions::default(),
    )
    .unwrap();

    match &doc.statements[0].kind {
        StatementKind::Group(g) => {
            assert_eq!(g.kind, "ref");
            assert_eq!(g.label.as_deref(), Some("over A, B\nsummary\nline 2"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn rejects_malformed_arrow_syntax() {
    let err = parse_with_options("A -x B", &ParseOptions::default()).unwrap_err();
    assert!(err.message.contains("E_ARROW_INVALID"));
}

#[test]
fn parses_lifecycle_shortcut_suffixes() {
    let doc = parse_with_options("A -> B++: inc", &ParseOptions::default()).unwrap();
    match &doc.statements[0].kind {
        StatementKind::Message(m) => {
            assert_eq!(m.arrow, "->@R++");
            assert_eq!(m.to, "B");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_expanded_slanted_arrow_tokens() {
    let doc = parse_with_options("A -/-> B\nB -\\\\->> A\n", &ParseOptions::default()).unwrap();
    match &doc.statements[0].kind {
        StatementKind::Message(m) => assert_eq!(m.arrow, "-/->"),
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[1].kind {
        StatementKind::Message(m) => assert_eq!(m.arrow, "-\\-->>"),
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_filled_virtual_endpoint_side_from_message_context() {
    let doc = parse_with_options("[*] -> A\nA -> [*]\n", &ParseOptions::default()).unwrap();
    match &doc.statements[0].kind {
        StatementKind::Message(m) => {
            let from_virtual = m.from_virtual.expect("from virtual");
            assert_eq!(from_virtual.side, crate::ast::VirtualEndpointSide::Left);
            assert_eq!(from_virtual.kind, crate::ast::VirtualEndpointKind::Filled);
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[1].kind {
        StatementKind::Message(m) => {
            let to_virtual = m.to_virtual.expect("to virtual");
            assert_eq!(to_virtual.side, crate::ast::VirtualEndpointSide::Right);
            assert_eq!(to_virtual.kind, crate::ast::VirtualEndpointKind::Filled);
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_queue_participant_and_separator() {
    let doc = parse_with_options(
        "queue Jobs as Q\n== Processing ==\n",
        &ParseOptions::default(),
    )
    .unwrap();

    match &doc.statements[0].kind {
        StatementKind::Participant(p) => {
            assert_eq!(p.name, "Jobs");
            assert_eq!(p.alias.as_deref(), Some("Q"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[1].kind {
        StatementKind::Separator(v) => assert_eq!(v.as_deref(), Some("Processing")),
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_typed_group_end_keyword() {
    let doc =
        parse_with_options("alt branch\nA -> B\nend alt\n", &ParseOptions::default()).unwrap();

    match &doc.statements[2].kind {
        StatementKind::Group(g) => {
            assert_eq!(g.kind, "end");
            assert_eq!(g.label.as_deref(), Some("alt"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}


#[test]
fn parses_sequence_decorated_arrow_styles_as_portable_arrow_core() {
    let doc = parse_with_options(
        "participant A\nparticipant B\nA -[#red,dashed]> B : styled\nB ->[#blue,dashed]> A : open styled\nA -[hidden]-> B : hidden\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.kind, DiagramKind::Sequence);
    match &doc.statements[2].kind {
        StatementKind::Message(m) => {
            assert_eq!(m.arrow, "->");
            assert_eq!(m.style.color.as_deref(), Some("red"));
            assert!(m.style.dashed);
            assert!(!m.style.hidden);
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[3].kind {
        StatementKind::Message(m) => {
            assert_eq!(m.arrow, "->>");
            assert_eq!(m.style.color.as_deref(), Some("blue"));
            assert!(m.style.dashed);
            assert!(!m.style.hidden);
        }
        other => panic!("unexpected statement: {other:?}"),
    }
    match &doc.statements[4].kind {
        StatementKind::Message(m) => {
            assert_eq!(m.arrow, "-->");
            assert!(m.style.hidden);
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn parses_sequence_participants_in_theme_fixture_context() {
    let fixture = fs::read_to_string(format!(
        "{}/docs/examples/themes/07_no_theme_default.puml",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("theme fixture");

    let doc = parse_with_options(&fixture, &ParseOptions::default()).unwrap();

    assert_eq!(doc.kind, DiagramKind::Sequence);
    assert!(matches!(
        doc.statements[1].kind,
        StatementKind::Participant(_)
    ));
    assert!(matches!(
        doc.statements[2].kind,
        StatementKind::Participant(_)
    ));
    assert!(matches!(doc.statements[3].kind, StatementKind::Message(_)));
    assert!(matches!(doc.statements[4].kind, StatementKind::Message(_)));
}


#[test]
fn apostrophe_comments_are_ignored_but_preserved_inside_quotes() {
    let doc = parse_with_options(
        "@startuml\n' full line comment\nA -> B: \"don't split\" ' trailing comment\n@enduml\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.kind, DiagramKind::Sequence);
    assert_eq!(doc.statements.len(), 1);
    match &doc.statements[0].kind {
        StatementKind::Message(m) => {
            assert_eq!(m.label.as_deref(), Some("\"don't split\""));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

/// Regression: `actor` + `alt` combination must not trigger component family
/// misdetection (issue #776).  `actor` is a valid sequence participant role and
/// should not cause the diagram to be classified as a component diagram when
/// sequence-specific keywords (`alt`, `activate`, sequence arrows) appear.
#[test]
fn actor_alt_combination_is_sequence_not_component() {
    let src = "\
@startuml
actor User
participant Browser
participant Server
User -> Browser: click
Browser -> Server: request
activate Server
alt success
  Server --> Browser: 200 OK
else failure
  Server --> Browser: 500
end
deactivate Server
@enduml
";
    let doc = parse_with_options(src, &ParseOptions::default()).unwrap();
    assert_eq!(
        doc.kind,
        DiagramKind::Sequence,
        "actor+alt diagram must be detected as sequence, not component"
    );
    // Verify the actor participant is present
    assert!(
        doc.statements.iter().any(|s| matches!(
            &s.kind,
            StatementKind::Participant(p) if p.name == "User"
        )),
        "actor participant 'User' must be parsed"
    );
}

/// Regression: `par..also..end` must not trigger component family misdetection
/// and `also` must be recognized as a valid parallel-branch continuation keyword
/// for `par` groups (issue #780).
#[test]
fn par_also_end_is_valid_sequence_group() {
    let src = "\
@startuml
participant A
participant B
participant C
A -> B: start
par branch 1
  B -> C: query
  C --> B: result
also branch 2
  B -> C: notify
  C --> B: ack
end
@enduml
";
    let doc = parse_with_options(src, &ParseOptions::default()).unwrap();
    assert_eq!(
        doc.kind,
        DiagramKind::Sequence,
        "par..also..end diagram must be detected as sequence"
    );
    // Verify `also` parsed as a Group statement
    let also_stmt = doc.statements.iter().find(|s| {
        matches!(&s.kind, StatementKind::Group(g) if g.kind == "also")
    });
    assert!(
        also_stmt.is_some(),
        "`also` keyword must produce a Group statement"
    );
}
