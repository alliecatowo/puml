#[test]
fn parses_activity_switch_split_goto_and_terminal_controls() {
    let doc = parse_with_options(
            "@startuml\nstart\nswitch (kind?)\ncase (A)\n:Do A;\ncase (B)\ngoto retry\nendswitch\nif (ready?) then (yes)\nelseif (warm?) then (maybe)\nendif\nrepeat\ncontinue\nbreak\nrepeat while (again?)\nend repeat\nsplit\n:one;\nsplit again\n:two;\nend split\nlabel retry\nbackward: retry path;\ndetach\n@enduml\n",
            &ParseOptions::default(),
        )
        .unwrap();
    assert_eq!(doc.kind, DiagramKind::Activity);
    let steps = doc
        .statements
        .iter()
        .filter_map(|stmt| match &stmt.kind {
            StatementKind::ActivityStep(step) => Some(step),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(steps
        .iter()
        .any(|step| step.kind == ActivityStepKind::IfStart
            && step.label.as_deref() == Some("switch kind?")));
    assert!(steps
        .iter()
        .any(|step| step.kind == ActivityStepKind::Else && step.label.as_deref() == Some("A")));
    assert!(steps
        .iter()
        .any(|step| step.kind == ActivityStepKind::Fork && step.label.as_deref() == Some("split")));
    assert!(steps
        .iter()
        .any(|step| step.kind == ActivityStepKind::Action
            && step.label.as_deref() == Some("goto retry")));
    assert!(steps.iter().any(|step| step.kind == ActivityStepKind::Else
        && step.label.as_deref() == Some("elseif warm? / maybe")));
    assert!(steps
        .iter()
        .any(|step| step.kind == ActivityStepKind::Action
            && step.label.as_deref() == Some("continue")));
    assert!(steps.iter().any(
        |step| step.kind == ActivityStepKind::Action && step.label.as_deref() == Some("break")
    ));
    assert!(steps
        .iter()
        .any(|step| step.kind == ActivityStepKind::Action
            && step.label.as_deref() == Some("backward retry path")));
    assert!(
        steps
            .iter()
            .any(|step| step.kind == ActivityStepKind::Stop
                && step.label.as_deref() == Some("detach"))
    );
}

#[test]
fn parses_family_declaration_blocks_with_members() {
    let doc = parse_with_options(
        "class User {\n  +id: UUID\n  +name: String\n}\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.kind, DiagramKind::Class);
    match &doc.statements[0].kind {
        StatementKind::ClassDecl(decl) => {
            assert_eq!(decl.name, "User");
            assert_eq!(decl.members.len(), 2);
            assert_eq!(decl.members[0].text, "+id: UUID");
            assert_eq!(decl.members[0].modifier, None);
            assert_eq!(decl.members[1].text, "+name: String");
            assert_eq!(decl.members[1].modifier, None);
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}

#[test]
fn unclosed_family_declaration_block_reports_deterministic_error() {
    let err = parse_with_options(
        "object Config {\nkey = \"value\"\n",
        &ParseOptions::default(),
    )
    .unwrap_err();
    assert!(err.message.contains("E_FAMILY_DECL_BLOCK_UNCLOSED"));
}

#[test]
fn parses_gantt_baseline_statements() {
    let doc = parse_with_options(
            "@startgantt\n[Build]\n[Milestone] happens on 2026-05-01\n[Build] starts 2026-04-01\n[Build] requires [Design]\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
    assert_eq!(doc.kind, DiagramKind::Gantt);
    assert!(matches!(
        doc.statements[0].kind,
        StatementKind::GanttTaskDecl { .. }
    ));
    assert!(matches!(
        doc.statements[1].kind,
        StatementKind::GanttMilestoneDecl {
            happens_on: Some(_),
            ..
        }
    ));
    assert!(doc
        .statements
        .iter()
        .any(|stmt| matches!(stmt.kind, StatementKind::GanttConstraint { .. })));
}

#[test]
fn parses_gantt_dates_and_duration_baseline_statements() {
    let doc = parse_with_options(
            "@startgantt\nProject starts 2026-05-01\n[Build] lasts 5 days\n[Test] starts 2026-05-06 and lasts 2 weeks\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
    assert_eq!(doc.kind, DiagramKind::Gantt);
    assert!(matches!(
        doc.statements[0].kind,
        StatementKind::GanttConstraint {
            ref subject,
            ref kind,
            ref target
        } if subject == "Project" && kind == "starts" && target == "2026-05-01"
    ));
    assert!(matches!(
        doc.statements[1].kind,
        StatementKind::GanttTaskDecl {
            ref name,
            duration_days: Some(5),
            ..
        } if name == "Build"
    ));
    assert!(matches!(
        doc.statements[2].kind,
        StatementKind::GanttTaskDecl {
            ref name,
            start_date: Some(ref d),
            duration_days: Some(14),
            ..
        } if name == "Test" && d == "2026-05-06"
    ));
}

#[test]
fn parses_gantt_closed_weekday_calendar_statements() {
    let doc = parse_with_options(
            "@startgantt\nProject starts 2026-05-01\nsaturday are closed\nsundays are closed\n[Build] lasts 2 days\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
    assert_eq!(doc.kind, DiagramKind::Gantt);
    assert!(matches!(
        doc.statements[1].kind,
        StatementKind::GanttCalendarClosed { ref day } if day == "saturday"
    ));
    assert!(matches!(
        doc.statements[2].kind,
        StatementKind::GanttCalendarClosed { ref day } if day == "sunday"
    ));
}

#[test]
fn parses_gantt_closed_date_range_calendar_statement() {
    let doc = parse_with_options(
            "@startgantt\nProject starts 2026-05-01\n2026-05-04 to 2026-05-05 is closed\n[Build] lasts 2 days\n@endgantt\n",
            &ParseOptions::default(),
        )
        .unwrap();
    assert_eq!(doc.kind, DiagramKind::Gantt);
    assert!(matches!(
        doc.statements[1].kind,
        StatementKind::GanttCalendarClosedDateRange {
            ref start_date,
            ref end_date
        } if start_date == "2026-05-04" && end_date == "2026-05-05"
    ));
}

#[test]
fn parses_chronology_happens_on_baseline_statement() {
    let doc = parse_with_options(
        "@startchronology\nRelease happens on 2026-05-15\n@endchronology\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.kind, DiagramKind::Chronology);
    assert!(matches!(
        doc.statements[0].kind,
        StatementKind::ChronologyHappensOn { .. }
    ));
}
