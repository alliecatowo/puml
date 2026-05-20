use crate::common::*;

#[test]
fn parser_preprocessor_variables_and_callable_invocations_expand_deterministically() {
    let src = "@startuml\n!$name = Alice\n!function F($x,$y=\"B\")\n!return $x + $y\n!endfunction\n!procedure P($from,$to)\n$from -> $to: via-proc\n!endprocedure\n!P($name, Bob)\n$name -> Bob: %F(\"A\")\n@enduml\n";
    let doc = parse(src).expect("parse should succeed");
    let model = normalize::normalize(doc).expect("normalize should succeed");
    let labels = model
        .events
        .iter()
        .filter_map(|e| match &e.kind {
            SequenceEventKind::Message { label, .. } => label.clone(),
            _ => None,
        })
        .collect::<Vec<_>>();
    // `+` is the string concatenation operator in PlantUML preprocessor (#582).
    // `!return $x + $y` with $x="A" and $y="B" (default) should produce "AB".
    assert_eq!(labels, vec!["via-proc", "AB"]);
}

#[test]
fn parser_preprocessor_concat_expands_and_procedure_return_fails_with_stable_code() {
    let concat_src =
        "@startuml\n!function Join($a##$b)\n!return $a ## $b\n!endfunction\nA -> B: %Join(Al, ice)\n@enduml\n";
    let concat_doc = parse(concat_src).expect("expected concat expansion");
    match &concat_doc.statements[0].kind {
        puml::ast::StatementKind::Message(m) => assert_eq!(m.label.as_deref(), Some("Alice")),
        other => panic!("unexpected statement: {other:?}"),
    }

    let proc_return_src =
        "@startuml\n!procedure Bad($x)\n!return $x\n!endprocedure\n!Bad(\"A\")\n@enduml\n";
    let proc_return_err = parse(proc_return_src).expect_err("expected procedure return failure");
    assert!(proc_return_err
        .message
        .contains("E_PREPROC_RETURN_UNEXPECTED"));
}

#[test]
fn preproc_break_outside_loop_reports_stable_code() {
    let err = parse("!break\n@startuml\nAlice -> Bob\n@enduml\n")
        .expect_err("break outside loops should fail");
    assert!(err.message.contains("E_PREPROC_BREAK_OUTSIDE_LOOP"));
}

#[test]
fn preproc_continue_outside_loop_reports_stable_code() {
    let err = parse("!continue\n@startuml\nAlice -> Bob\n@enduml\n")
        .expect_err("continue outside loops should fail");
    assert!(err.message.contains("E_PREPROC_CONTINUE_OUTSIDE_LOOP"));
}

#[test]
fn preproc_endfor_without_foreach_reports_stable_code() {
    let err = parse("!endfor\n@startuml\nAlice -> Bob\n@enduml\n")
        .expect_err("endfor without foreach should fail");
    assert!(err.message.contains("E_PREPROC_FOREACH_UNEXPECTED"));
}

#[test]
fn preproc_endwhile_without_while_reports_stable_code() {
    let err = parse("!endwhile\n@startuml\nAlice -> Bob\n@enduml\n")
        .expect_err("endwhile without while should fail");
    assert!(err.message.contains("E_PREPROC_WHILE_UNEXPECTED"));
}

#[test]
fn preproc_elseif_after_else_reports_order_error() {
    let src = "!if 1\n!else\n!elseif 1\n!endif\n@startuml\nAlice -> Bob\n@enduml\n";
    let err = parse(src).expect_err("elseif after else should fail");
    assert!(err.message.contains("E_PREPROC_COND_ORDER"));
}
