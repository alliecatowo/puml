use super::*;

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
        StatementKind::GanttCompound {
            ref name,
            ref clauses,
            ..
        } if name == "Test" && clauses == "starts 2026-05-06 and lasts 2 weeks"
    ));
}

#[test]
fn parses_gantt_completion_percentage_forms() {
    let doc = parse_with_options(
        "@startgantt\n[Build] is 40% complete\n[Test] requires 3 days and is 10% completed\n@endgantt\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.kind, DiagramKind::Gantt);
    assert!(matches!(
        doc.statements[0].kind,
        StatementKind::GanttCompound {
            ref name,
            ref clauses,
            ..
        } if name == "Build" && clauses == "is 40% complete"
    ));
    assert!(matches!(
        doc.statements[1].kind,
        StatementKind::GanttCompound {
            ref name,
            ref clauses,
            ..
        } if name == "Test" && clauses == "requires 3 days and is 10% completed"
    ));
}

#[test]
fn parses_gantt_task_hyperlink_forms() {
    let doc = parse_with_options(
        "@startgantt\n[Build] links to [[https://example.com/build]]\n[Test] requires 3 days and links to [[https://example.com/test Test docs]]\n@endgantt\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.kind, DiagramKind::Gantt);
    assert!(matches!(
        doc.statements[0].kind,
        StatementKind::GanttCompound {
            ref name,
            ref clauses,
            ..
        } if name == "Build" && clauses == "links to [[https://example.com/build]]"
    ));
    assert!(matches!(
        doc.statements[1].kind,
        StatementKind::GanttCompound {
            ref name,
            ref clauses,
            ..
        } if name == "Test" && clauses == "requires 3 days and links to [[https://example.com/test Test docs]]"
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

#[test]
fn parses_chronology_ranges_eras_and_brackets() {
    let doc = parse_with_options(
        "@startchronology\n[Exploration] happens from 2026-01-01 to 2026-03-31 is colored in #bfdbfe\nRelease window spans 2026-04-01 to 2026-04-15\nbracket FY26 from 2026-01-01 to 2026-12-31\n@endchronology\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.kind, DiagramKind::Chronology);
    assert!(matches!(
        doc.statements[0].kind,
        StatementKind::ChronologyHappensOn {
            ref subject,
            ref when,
            ref end,
            ref color,
            bracket: false,
        } if subject == "Exploration"
            && when == "2026-01-01"
            && end.as_deref() == Some("2026-03-31")
            && color.as_deref() == Some("#bfdbfe")
    ));
    assert!(matches!(
        doc.statements[2].kind,
        StatementKind::ChronologyHappensOn {
            ref subject,
            bracket: true,
            ..
        } if subject == "FY26"
    ));
}

#[test]
fn parses_chronology_between_years_and_datetime_subjects() {
    let doc = parse_with_options(
        "@startchronology\nbracket 1990s between 1990 and 1999 is colored in #f97316\n[A: 2024-01-15 01:08:12] happens on 2024-01-15 01:08:12\n@endchronology\n",
        &ParseOptions::default(),
    )
    .unwrap();
    assert_eq!(doc.kind, DiagramKind::Chronology);
    assert!(matches!(
        doc.statements[0].kind,
        StatementKind::ChronologyHappensOn {
            ref subject,
            ref when,
            ref end,
            bracket: true,
            ..
        } if subject == "1990s" && when == "1990" && end.as_deref() == Some("1999")
    ));
    assert!(matches!(
        doc.statements[1].kind,
        StatementKind::ChronologyHappensOn {
            ref subject,
            ref when,
            ..
        } if subject == "A: 2024-01-15 01:08:12" && when == "2024-01-15 01:08:12"
    ));
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
fn parses_wbs_cross_tree_alias_relation() {
    let doc = parse_with_options(
        "@startwbs\n* Root\n** (a) Build\n** (b) Launch\na -> b\n@endwbs\n",
        &ParseOptions::default(),
    )
    .expect("wbs alias relation should parse");
    assert_eq!(doc.kind, DiagramKind::Wbs);
    match &doc.statements[3].kind {
        StatementKind::FamilyRelation(rel) => {
            assert_eq!(rel.from, "a");
            assert_eq!(rel.to, "b");
            assert_eq!(rel.arrow, "->");
        }
        other => panic!("unexpected statement: {other:?}"),
    }
}
