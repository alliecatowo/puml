use super::*;

#[test]
fn preprocessor_if_elseif_else_emits_only_selected_branch() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_if_elseif_else.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let labels = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["primary"]);
}

#[test]
fn preprocessor_while_executes_until_condition_is_false() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_while_define_counter.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let labels = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["loop 2", "loop 1"]);
}

#[test]
fn preprocessor_variable_assignment_and_reference_semantics_are_applied() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_variable_assignment_reference.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let participants = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Participant"]["name"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(participants, vec!["Alice", "Bob"]);
}

#[test]
fn preprocessor_function_and_procedure_args_expand_deterministically() {
    let fn_out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_function_call_args_defaults_keywords.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let fn_json: Value = serde_json::from_slice(&fn_out).unwrap();
    let fn_labels = fn_json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
        .collect::<Vec<_>>();
    // `+` is the string concatenation operator in PlantUML preprocessor (#582).
    // `!return $lhs + "->" + $rhs` should evaluate to the joined string.
    assert_eq!(fn_labels, vec!["A->B", "C->D"]);

    let proc_out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_procedure_call_args.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let proc_json: Value = serde_json::from_slice(&proc_out).unwrap();
    let proc_labels = proc_json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(proc_labels, vec!["\"ok\"", "go"]);
}

#[test]
fn preprocessor_function_return_with_leading_indentation_is_honored() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_function_return_indented.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json: Value = serde_json::from_slice(&out).unwrap();
    let labels = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|stmt| stmt["kind"]["Message"]["label"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["\"A\""]);
}

#[test]
fn preprocessor_function_procedure_assert_log_and_dump_are_minimally_compatible() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("preprocessor/valid_function_procedure_assert_log_dump.puml"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("preprocessor/valid_log_and_dump_with_payload.puml"),
        ])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}

#[test]
fn preprocessor_assert_false_reports_diagnostic_snapshot() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("preprocessor/invalid_assert_false.puml"),
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();
    let stderr = String::from_utf8(out.stderr).expect("utf8 stderr");
    assert!(stderr.contains("E_PREPROC_ASSERT"));
    assert_snapshot!("preprocessor_assert_false_reports_diagnostic", stderr);
}

#[test]
fn preprocessor_unclosed_function_reports_diagnostic_snapshot() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("preprocessor/invalid_unclosed_function.puml"),
        ])
        .assert()
        .code(1)
        .get_output()
        .clone();
    let stderr = String::from_utf8(out.stderr).expect("utf8 stderr");
    assert!(stderr.contains("E_FUNCTION_UNCLOSED"));
    assert_snapshot!("preprocessor_unclosed_function_reports_diagnostic", stderr);
}

#[test]
fn preprocessor_conditional_and_while_balance_errors_are_deterministic() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_preproc_conditional_order.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_COND_ORDER"));

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_preproc_unclosed_if.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_COND_UNCLOSED"));

    Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--check",
            &fixture("errors/invalid_preproc_endwhile_without_while.puml"),
        ])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_WHILE_UNEXPECTED"));
}

#[test]
fn preprocessor_expression_validation_errors_are_deterministic() {
    let cases = [
        (
            "errors/invalid_preproc_expr_missing.puml",
            "E_PREPROC_EXPR_REQUIRED",
        ),
        (
            "errors/invalid_preproc_unexpected_endfunction.puml",
            "E_PREPROC_UNEXPECTED",
        ),
        (
            "errors/invalid_preproc_procedure_unsupported.puml",
            "E_PREPROC_CALL_KIND",
        ),
        (
            "errors/invalid_preproc_while_iteration_limit.puml",
            "E_PREPROC_WHILE_LIMIT",
        ),
        (
            "errors/invalid_preproc_assert_missing_expr.puml",
            "E_PREPROC_ASSERT_EXPR_REQUIRED",
        ),
        (
            "errors/invalid_preproc_builtin_in_assert.puml",
            "E_PREPROC_BUILTIN_UNSUPPORTED",
        ),
        (
            "errors/invalid_preproc_builtin_in_log.puml",
            "E_PREPROC_BUILTIN_UNSUPPORTED",
        ),
        (
            "errors/invalid_preproc_dynamic_invoke.puml",
            "E_PREPROC_DYNAMIC_UNSUPPORTED",
        ),
        (
            "errors/invalid_preproc_json_assignment.puml",
            "E_PREPROC_JSON_UNSUPPORTED",
        ),
        (
            "errors/invalid_preproc_function_missing_arg.puml",
            "E_PREPROC_ARG_REQUIRED",
        ),
        (
            "errors/invalid_preproc_procedure_return.puml",
            "E_PREPROC_RETURN_UNEXPECTED",
        ),
        (
            "errors/invalid_import_empty_path.puml",
            "E_IMPORT_PATH_REQUIRED",
        ),
        // URL imports are covered separately by import_url_disabled_produces_deterministic_error.
        // Keep this list focused on local include/import path-shape diagnostics.
        (
            "errors/invalid_import_absolute_path.puml",
            "E_IMPORT_ABSOLUTE_PATH",
        ),
        (
            "errors/invalid_import_tag_form.puml",
            "E_IMPORT_INVALID_FORM",
        ),
        ("errors/invalid_import_escape_path.puml", "E_IMPORT_ESCAPE"),
        (
            "errors/invalid_import_missing_module.puml",
            "E_IMPORT_STDLIB_NOT_FOUND",
        ),
    ];

    for (path, code) in cases {
        Command::cargo_bin("puml")
            .expect("binary")
            .args(["--check", &fixture(path)])
            .assert()
            .code(1)
            .stderr(predicate::str::contains(code));
    }
}
