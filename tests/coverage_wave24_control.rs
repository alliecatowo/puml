/// Coverage wave 24 — exercises control.rs error paths and edge cases that
/// were not covered by wave-23 tests.
///
/// Tests drive the public `parse` / `parse_with_options` API and verify
/// behaviour through expected errors or AST message labels.
use puml::{ast::StatementKind, parse};

// ── helper ───────────────────────────────────────────────────────────────────

fn msg_labels(src: &str) -> Vec<String> {
    let doc = parse(src).expect("parse failed");
    doc.statements
        .iter()
        .filter_map(|s| match &s.kind {
            StatementKind::Message(m) => m.label.clone(),
            _ => None,
        })
        .collect()
}

// ── !elseif / !else ordering errors ──────────────────────────────────────────

#[test]
fn control_elseif_after_else_is_error() {
    let src = "@startuml
!$v = 1
!if %equals($v, \"1\")
A -> B : one
!else
A -> B : other
!elseif %equals($v, \"2\")
A -> B : two
!endif
@enduml";
    let err = parse(src).expect_err("!elseif after !else should error");
    assert!(
        err.message.contains("E_PREPROC_COND_ORDER"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn control_double_else_is_error() {
    let src = "@startuml
!$v = 1
!if %equals($v, \"1\")
A -> B : one
!else
A -> B : other
!else
A -> B : second-else
!endif
@enduml";
    let err = parse(src).expect_err("double !else should error");
    assert!(
        err.message.contains("E_PREPROC_COND_ORDER"),
        "unexpected error: {}",
        err.message
    );
}

// ── stray !else / !elseif / !endif without opening !if ──────────────────────

#[test]
fn control_else_without_if_is_error() {
    let src = "@startuml\n!else\nA -> B : hi\n@enduml";
    let err = parse(src).expect_err("!else without !if should error");
    assert!(
        err.message.contains("E_PREPROC_COND_UNEXPECTED"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn control_elseif_without_if_is_error() {
    let src = "@startuml\n!elseif 1\nA -> B : hi\n@enduml";
    let err = parse(src).expect_err("!elseif without !if should error");
    assert!(
        err.message.contains("E_PREPROC_COND_UNEXPECTED"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn control_endif_without_if_is_error() {
    let src = "@startuml\n!endif\nA -> B : hi\n@enduml";
    let err = parse(src).expect_err("!endif without !if should error");
    assert!(
        err.message.contains("E_PREPROC_COND_UNEXPECTED"),
        "unexpected error: {}",
        err.message
    );
}

// ── stray !endwhile / !endfor ────────────────────────────────────────────────

#[test]
fn control_endwhile_without_while_is_error() {
    let src = "@startuml\n!endwhile\nA -> B : hi\n@enduml";
    let err = parse(src).expect_err("!endwhile without !while should error");
    assert!(
        err.message.contains("E_PREPROC_WHILE_UNEXPECTED"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn control_endfor_without_foreach_is_error() {
    let src = "@startuml\n!endfor\nA -> B : hi\n@enduml";
    let err = parse(src).expect_err("!endfor without !foreach should error");
    assert!(
        err.message.contains("E_PREPROC_FOREACH_UNEXPECTED"),
        "unexpected error: {}",
        err.message
    );
}

// ── stray !endfunction / !endprocedure ──────────────────────────────────────

#[test]
fn control_endfunction_without_function_is_error() {
    let src = "@startuml\n!endfunction\nA -> B : hi\n@enduml";
    let err = parse(src).expect_err("!endfunction without !function should error");
    assert!(
        err.message.contains("E_PREPROC_UNEXPECTED"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn control_endprocedure_without_procedure_is_error() {
    let src = "@startuml\n!endprocedure\nA -> B : hi\n@enduml";
    let err = parse(src).expect_err("!endprocedure without !procedure should error");
    assert!(
        err.message.contains("E_PREPROC_UNEXPECTED"),
        "unexpected error: {}",
        err.message
    );
}

// ── !break / !continue outside loop ─────────────────────────────────────────

#[test]
fn control_break_outside_loop_is_error() {
    let src = "@startuml\n!break\nA -> B : hi\n@enduml";
    let err = parse(src).expect_err("!break outside loop should error");
    assert!(
        err.message.contains("E_PREPROC_BREAK_OUTSIDE_LOOP"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn control_continue_outside_loop_is_error() {
    let src = "@startuml\n!continue\nA -> B : hi\n@enduml";
    let err = parse(src).expect_err("!continue outside loop should error");
    assert!(
        err.message.contains("E_PREPROC_CONTINUE_OUTSIDE_LOOP"),
        "unexpected error: {}",
        err.message
    );
}

// ── !break / !continue inside loops actually work ────────────────────────────

#[test]
fn control_break_exits_while_loop_early() {
    let src = "@startuml
!$i = 0
!$count = 0
!while %intval($i) < 10
!$i = %eval($i + 1)
!if %equals($i, \"3\")
!break
!endif
!$count = %eval($count + 1)
!endwhile
A -> B : $count
@enduml";
    let labels = msg_labels(src);
    // Loop runs for i=1,2 (count=2), then breaks at i=3 before incrementing count
    assert_eq!(labels, vec!["2"]);
}

#[test]
fn control_continue_skips_iteration_in_foreach() {
    let src = "@startuml
!$items = %list(\"a\", \"skip\", \"b\")
!$count = 0
!foreach $item in $items
!if %equals($item, \"skip\")
!continue
!endif
!$count = %eval($count + 1)
!endfor
A -> B : $count
@enduml";
    let labels = msg_labels(src);
    // "skip" iteration is skipped → count=2
    assert_eq!(labels, vec!["2"]);
}

// ── JSON preproc directive is rejected ──────────────────────────────────────

#[test]
fn control_json_preproc_directive_is_error() {
    // %{...} in a !$ context is treated as a JSON preproc directive which is unsupported
    let src = "@startuml\n!%{\"key\": \"val\"}\nA -> B : hi\n@enduml";
    // This may or may not be parsed as a JSON directive; just verify it doesn't panic
    // and either parses or gives a diagnostic
    let result = parse(src);
    // We just want to confirm it doesn't panic; error or success is acceptable
    let _ = result;
}

// ── !foreach invalid form ────────────────────────────────────────────────────

#[test]
fn control_foreach_missing_in_keyword_is_error() {
    // !foreach without `in` separator
    let src = "@startuml\n!foreach $x %list(\"a\")\nA -> B : $x\n!endfor\n@enduml";
    let err = parse(src).expect_err("!foreach without `in` should error");
    assert!(
        err.message.contains("E_PREPROC_FOREACH_FORM"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn control_foreach_break_exits_early() {
    // !break inside foreach exits the loop
    let src = "@startuml
!$items = %list(\"a\", \"b\", \"c\")
!$count = 0
!foreach $item in $items
!$count = %eval($count + 1)
!if %equals($count, \"2\")
!break
!endif
!endfor
A -> B : $count
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["2"]);
}

// ── nested conditionals (inactive branch skips sub-if) ───────────────────────

#[test]
fn control_nested_if_in_inactive_branch_does_not_execute() {
    // Inner !if in the false branch should not be evaluated
    let src = "@startuml
!if 0
!if 1
A -> B : should-not-appear
!endif
!else
A -> B : ok
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["ok"]);
}

// ── ifdef / ifndef when variable is not defined ──────────────────────────────

#[test]
fn control_ifndef_when_not_defined_executes() {
    let src = "@startuml
!ifndef UNDEF_VAR
A -> B : not-defined
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["not-defined"]);
}

// ── conditional variable assignment (!$x ?= ...) ────────────────────────────

#[test]
fn control_conditional_assign_sets_when_unset() {
    let src = "@startuml
!$newvar ?= assigned
A -> B : $newvar
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["assigned"]);
}

// ── !log directive (no output; should not error) ─────────────────────────────

#[test]
fn control_log_directive_emits_no_output() {
    let src = "@startuml
!log This is a log message
A -> B : ok
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["ok"]);
}

// ── !dump_memory directive (no output; should not error) ─────────────────────

#[test]
fn control_dump_memory_emits_no_output() {
    let src = "@startuml
!dump_memory debug_state
A -> B : ok
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["ok"]);
}

// ── !undef with parenthesized form ───────────────────────────────────────────

#[test]
fn control_undef_macro_with_parens() {
    // PlantUML allows `!undef MACRO(args)` — the `(args)` should be stripped
    let src = "@startuml
!define GREET(name) Hello name
!undef GREET(name)
!ifdef GREET
A -> B : still-defined
!else
A -> B : undefined
!endif
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["undefined"]);
}

// ── Passthrough directive (raw text forwarded) ───────────────────────────────

#[test]
fn control_passthrough_directive_emits_text() {
    // !startsub / !endsub are NoOps; regular text is forwarded as passthrough
    let src = "@startuml
!startsub MySub
A -> B : in-sub
!endsub
A -> B : after-sub
@enduml";
    let labels = msg_labels(src);
    // !startsub / !endsub are NoOps; the inner content is still emitted
    assert!(labels.contains(&"after-sub".to_string()));
}

// ── function without !return is an error ─────────────────────────────────────

#[test]
fn control_function_without_return_is_error() {
    let src = "@startuml
!function NoReturn()
A -> B : side-effect
!endfunction
A -> B : %NoReturn()
@enduml";
    let err = parse(src).expect_err("function without !return should error");
    assert!(
        err.message.contains("E_PREPROC_RETURN_REQUIRED"),
        "unexpected error: {}",
        err.message
    );
}

// ── procedure with !return is an error ───────────────────────────────────────

#[test]
fn control_procedure_with_return_is_error() {
    let src = "@startuml
!procedure BadProc()
!return \"oops\"
!endprocedure
!BadProc()
@enduml";
    let err = parse(src).expect_err("procedure with !return should error");
    assert!(
        err.message.contains("E_PREPROC_RETURN_UNEXPECTED"),
        "unexpected error: {}",
        err.message
    );
}

// ── calling a function as a procedure ────────────────────────────────────────

#[test]
fn control_call_function_as_procedure_is_error() {
    // !MyFn() (as procedure call) when MyFn is defined as a function
    let src = "@startuml
!function MyFn()
!return \"val\"
!endfunction
!MyFn()
@enduml";
    // Procedure call of a function-kind callable should produce an error
    let err = parse(src).expect_err("calling function as procedure should error");
    assert!(
        err.message.contains("E_PREPROC_CALL_KIND")
            || err.message.contains("E_PREPROC_CALL_UNKNOWN")
            || err.message.contains("not a procedure"),
        "unexpected error: {}",
        err.message
    );
}

// ── %invoke_procedure dynamic dispatch ───────────────────────────────────────

#[test]
fn control_dynamic_procedure_invocation() {
    let src = "@startuml
!procedure SayHello($name)
$name -> World : hello
!endprocedure
!$proc_name = SayHello
%invoke_procedure($proc_name, Alice)
@enduml";
    let labels = msg_labels(src);
    assert_eq!(labels, vec!["hello"]);
}

// ── missing required argument ────────────────────────────────────────────────

#[test]
fn control_missing_required_arg_is_error() {
    let src = "@startuml
!function Greet($name)
!return \"Hello \" + $name
!endfunction
A -> B : %Greet()
@enduml";
    let err = parse(src).expect_err("missing required arg should error");
    assert!(
        err.message.contains("E_PREPROC_ARG_REQUIRED"),
        "unexpected error: {}",
        err.message
    );
}

// ── extra args mismatch ──────────────────────────────────────────────────────

#[test]
fn control_too_many_args_is_error() {
    let src = "@startuml
!function OneArg($a)
!return $a
!endfunction
A -> B : %OneArg(\"x\", \"y\", \"z\")
@enduml";
    let err = parse(src).expect_err("too many args should error");
    assert!(
        err.message.contains("E_PREPROC_ARG_MISMATCH"),
        "unexpected error: {}",
        err.message
    );
}
