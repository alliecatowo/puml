#[test]
fn parses_usecase_relations_with_alias_and_label() {
    let doc = parse_with_options(
        "usecase Authenticate as Auth\nusecase User\nAuth --> User : validates\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.kind, DiagramKind::UseCase);
    match &doc.statements[2].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "Auth");
            assert_eq!(rel.to, "User");
            assert_eq!(rel.arrow, "-->");
            assert_eq!(rel.label.as_deref(), Some("validates"));
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn malformed_family_relation_is_preserved_as_unknown_statement() {
    let doc = parse_with_options("class User\nUser -->\n", &ParseOptions::default()).unwrap();
    assert_eq!(doc.kind, DiagramKind::Class);
    assert!(matches!(doc.statements[1].kind, StatementKind::Unknown(_)));
}

#[test]
fn state_keyword_is_parsed_as_state_decl() {
    let doc = parse_with_options("state Running\n", &ParseOptions::default()).unwrap();
    assert_eq!(doc.kind, DiagramKind::State);
    assert!(matches!(
        doc.statements[0].kind,
        StatementKind::StateDecl(_)
    ));
}

#[test]
fn mixed_family_input_reports_deterministic_error() {
    let err = parse_with_options("class A\nnewpage\n", &ParseOptions::default()).unwrap_err();
    assert!(err.message.contains("E_FAMILY_MIXED"));
}

#[test]
fn start_enduml_markers_accept_optional_block_suffixes() {
    let doc = parse_with_options(
            "@startuml \"Primary\"\nA -> B: one\n@enduml anything\n@startuml Second\nB -> A: two\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
    assert_eq!(doc.kind, DiagramKind::Sequence);
    let labels = doc
        .statements
        .iter()
        .filter_map(|s| match &s.kind {
            StatementKind::Message(m) => m.label.as_deref(),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["one", "two"]);
}

#[test]
fn start_end_timeline_markers_accept_optional_block_suffixes() {
    let gantt = parse_with_options(
        "@startgantt \"Gantt\"\n[2026-01] : one\n@endgantt anything\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(gantt.kind, DiagramKind::Gantt);

    let chronology = parse_with_options(
        "@startchronology\nEvent\n@endchronology now\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(chronology.kind, DiagramKind::Chronology);
}

#[test]
fn startmindmap_and_startwbs_markers_set_family_kind() {
    let mindmap = parse_with_options(
        "@startmindmap\n* Root\n** Child\n@endmindmap\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(mindmap.kind, DiagramKind::MindMap);

    let wbs =
        parse_with_options("@startwbs\n* Scope\n@endwbs\n", &ParseOptions::default()).unwrap();
    assert_eq!(wbs.kind, DiagramKind::Wbs);

    let gantt = parse_with_options(
        "@startgantt\n[2026-01-01] : Kickoff\n@endgantt\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(gantt.kind, DiagramKind::Gantt);

    let chronology = parse_with_options(
        "@startchronology\n2026-01-01 : Event\n@endchronology\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(chronology.kind, DiagramKind::Chronology);
}

#[test]
fn parses_activity_oldstyle_baseline_statements() {
    let doc = parse_with_options(
            "@startuml\n|Build|\n(*) --> \"Init\"\n#gold:Compile;\n-->[next] right of \"Test\"\n\"Test\" --> (*)\ndetach\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
    assert_eq!(doc.kind, DiagramKind::Activity);
    assert!(!doc.statements.is_empty());
}

#[test]
fn parses_old_activity_edges_as_canonical_steps() {
    let doc = parse_with_options(
        "@startuml\n(*) --> \"Step1\"\n\"Step1\" -->[ok] \"Step2\"\n\"Step2\" --> (*)\n@enduml\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.kind, DiagramKind::Activity);
    let steps: Vec<_> = doc
        .statements
        .iter()
        .filter_map(|stmt| match &stmt.kind {
            StatementKind::ActivityStep(step) => Some((step.kind.clone(), step.label.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        steps,
        vec![
            (ActivityStepKind::Start, None),
            (ActivityStepKind::Action, Some("Step1".to_string())),
            (ActivityStepKind::Action, Some("Step2".to_string())),
            (ActivityStepKind::Stop, None),
        ]
    );
}

#[test]
fn mismatched_start_end_family_markers_report_deterministic_error() {
    let err = parse_with_options("@startmindmap\n* Root\n@endwbs\n", &ParseOptions::default())
        .unwrap_err();
    assert!(err.message.contains("E_BLOCK_MISMATCH"));
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
    let also_stmt = doc
        .statements
        .iter()
        .find(|s| matches!(&s.kind, StatementKind::Group(g) if g.kind == "also"));
    assert!(
        also_stmt.is_some(),
        "`also` keyword must produce a Group statement"
    );
}
