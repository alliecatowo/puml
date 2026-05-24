use super::*;

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
        .any(|step| step.kind == ActivityStepKind::Fork
            && step.label.as_deref() == Some("split")));
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
    assert!(steps
        .iter()
        .any(|step| step.kind == ActivityStepKind::Action
            && step.label.as_deref() == Some("break")));
    assert!(steps
        .iter()
        .any(|step| step.kind == ActivityStepKind::Action
            && step.label.as_deref() == Some("backward retry path")));
    assert!(steps
        .iter()
        .any(|step| step.kind == ActivityStepKind::Detach
            && step.label.as_deref() == Some("detach")));
}

#[test]
fn parses_activity_new_metadata_steps() {
    let doc = parse_with_options(
        "@startuml\nstart\n#LightBlue:Collect;\n-[#red,dashed]-> reviewed;\n:Review;\nnote right: keep evidence\n#pink:(A)\ngroup Audit\n:Log;\nend group\npartition #LightYellow Ops {\n:Ship;\n}\nkill\n@enduml\n",
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
    assert!(steps.iter().any(|step| {
        step.kind == ActivityStepKind::Action
            && step.label.as_deref() == Some("\u{1f}style:fill:LightBlue\u{1f}Collect")
    }));
    assert!(steps.iter().any(|step| {
        step.kind == ActivityStepKind::Arrow
            && step
                .label
                .as_deref()
                .is_some_and(|label| label.contains("color:red") && label.contains("dashed:1"))
    }));
    assert!(doc.statements.iter().any(|stmt| matches!(
        &stmt.kind,
        StatementKind::Note(note) if note.text == "keep evidence"
    )));
    assert!(steps
        .iter()
        .any(|step| step.kind == ActivityStepKind::Connector
            && step.label.as_deref() == Some("\u{1f}style:fill:pink\u{1f}(A)")));
    assert!(steps.iter().any(|step| {
        step.kind == ActivityStepKind::PartitionStart
            && step.label.as_deref() == Some("\u{1f}style:fill:LightYellow\u{1f}Ops")
    }));
    assert!(steps.iter().any(|step| {
        step.kind == ActivityStepKind::PartitionStart
            && step.label.as_deref() == Some("Audit")
    }));
    assert!(steps
        .iter()
        .any(|step| step.kind == ActivityStepKind::Kill
            && step.label.as_deref() == Some("kill")));
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
