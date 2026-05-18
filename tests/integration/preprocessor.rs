use super::fixture;
use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::Value;

#[test]
fn preprocessor_builtin_strlen_expands_to_character_count() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_builtin_strlen.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let label = json["statements"][0]["kind"]["Message"]["label"]
        .as_str()
        .unwrap();
    assert_eq!(label, "len=8");
}

#[test]
fn preprocessor_builtin_boolval_expands_truthiness_and_not_inverts() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_builtin_boolval.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let labels: Vec<&str> = json["statements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["kind"]["Message"]["label"].as_str())
        .collect();
    assert_eq!(labels, vec!["true", "false", "false"]);
}

#[test]
fn preprocessor_builtin_chain_composes_substr_upper_intval_dec2hex() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_builtin_chain.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let label = json["statements"][0]["kind"]["Message"]["label"]
        .as_str()
        .unwrap();
    assert_eq!(label, "PLANT-12-ff");
}

#[test]
fn preprocessor_builtin_list_map_stringification_assert_and_log_surface_passes() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_builtin_list_map_stringification_assert_log.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let label = json["statements"][0]["kind"]["Message"]["label"]
        .as_str()
        .unwrap();
    assert_eq!(
        label,
        "beta|alpha+beta+gamma|Ada|2|\"name\",\"role\"|Ada,admin|\"admin\""
    );
}

#[test]
fn preprocessor_dynamic_call_user_func_invokes_function_expression() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_dynamic_call_user_func.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let label = json["statements"][0]["kind"]["Message"]["label"]
        .as_str()
        .unwrap();
    assert_eq!(label, "hi Bob");
}

#[test]
fn preprocessor_dynamic_invoke_procedure_dispatches_variable_and_alias_forms() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_dynamic_invoke_procedure.puml"),
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
    assert_eq!(labels, vec!["via-var", "alias"]);
}

#[test]
fn preprocessor_color_math_builtins_expand_deterministically() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_builtin_color_math.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let label = json["statements"][0]["kind"]["Message"]["label"]
        .as_str()
        .unwrap();
    assert_eq!(label, "7 true #ffffff #7f7f7f #000000");
}

#[test]
fn preprocessor_json_dot_and_bracket_access_expands_native_variable_paths() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_json_dot_bracket_access.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let label = json["statements"][0]["kind"]["Message"]["label"]
        .as_str()
        .unwrap();
    assert_eq!(label, "Ada|uml|1");
}

#[test]
fn preprocessor_splitstr_regex_splits_on_lightweight_delimiter_patterns() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_splitstr_regex.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let label = json["statements"][0]["kind"]["Message"]["label"]
        .as_str()
        .unwrap();
    assert_eq!(label, "alpha|beta|gamma");
}

#[test]
fn preprocessor_macro_concat_expands_inside_safe_procedure_body_lines() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_macro_concat_body.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let msg = &json["statements"][0]["kind"]["Message"];
    assert_eq!(msg["from"].as_str().unwrap(), "Alice");
    assert_eq!(msg["label"].as_str().unwrap(), "Alice");
}

#[test]
fn preprocessor_macro_expression_and_collection_depth_expand() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_macro_expr_collection_depth.puml"),
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
        .map(|stmt| {
            stmt["kind"]["Message"]["label"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect::<Vec<_>>();
    assert_eq!(labels, vec!["OK", "14 / blue / green"]);
}

#[test]
fn preprocessor_unsafe_time_env_random_helpers_follow_deterministic_policy() {
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args([
            "--dump",
            "ast",
            &fixture("preprocessor/valid_unsafe_builtin_policy.puml"),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let json: Value = serde_json::from_slice(&out).unwrap();
    let label = json["statements"][0]["kind"]["Message"]["label"]
        .as_str()
        .unwrap();
    assert_eq!(label, "date=[] env=[] rand=[0]");
}

#[test]
fn preprocessor_unsafe_io_helpers_reject_with_stable_policy_code() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("@startuml\nA -> B : %load_json(\"/tmp/state.json\")\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_UNSAFE_BUILTIN"));
}

#[test]
fn preprocessor_next_wave_expression_callable_scope_and_helper_aliases_expand() {
    let src = "@startuml\n!procedure $Emit($from, $to)\n!$label = %map_get(%map(\"name\", \"Ada\"), \"missing\", \"fallback\")\n$from -> $to : $label/%procedure_exists(\"$Emit\")/%is_empty(%list_clear(%list(\"x\")))\n!endprocedure\n!function Pick($base)\n!$items = %list_push(%list($base), \"beta\")\n!return %list_get($items, 1, \"missing\")\n!endfunction\n!$cfg = {\"empty\":\"\",\"none\":null}\n!if %json_key_exists($cfg, \"empty\") and %json_key_exists($cfg, \"none\") and 1 <> 2\n!$Emit(Alice, Bob)\nAlice -> Bob : %Pick(\"alpha\")/%list_get(%list(\"x\"), 9, \"fallback\")/%is_number(%eval_int(\"2 + 3\"))\n!endif\n@enduml\n";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", "--", "-"])
        .write_stdin(src)
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
    assert_eq!(labels, vec!["fallback/true/true", "beta/fallback/true"]);
}

#[test]
fn preprocessor_loop_controls_and_foreach_object_key_iteration_expand() {
    let src = "@startuml\n!$cfg = {\"alpha\":1,\"beta\":2,\"stop\":3}\n!foreach $key in $cfg\n!if $key == \"beta\"\n!continue\n!endif\n!if $key == \"stop\"\n!break\n!endif\nAlice -> Bob : $key\n!endfor\n!$i = 0\n!while $i < 5\n!$i = %eval_int($i + 1)\n!if $i == 2\n!continue\n!endif\n!if $i == 4\n!break\n!endif\nAlice -> Bob : loop-$i\n!endwhile\n@enduml\n";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", "--", "-"])
        .write_stdin(src)
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
    assert_eq!(labels, vec!["alpha", "loop-1", "loop-3"]);
}

#[test]
fn preprocessor_loop_controls_outside_loop_report_stable_codes() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("@startuml\n!break\nAlice -> Bob : unreachable\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_BREAK_OUTSIDE_LOOP"));

    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("@startuml\n!continue\nAlice -> Bob : unreachable\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_CONTINUE_OUTSIDE_LOOP"));
}

#[test]
fn preprocessor_deep_parity_macro_defaults_dynamic_and_collection_helpers_expand() {
    let src = "@startuml\n!define EMIT(from=Alice,to=Bob,label=hi) from -> to : label\n!function Choice($name=\"Ada\", $suffix=\"!\")\n!return $name ## $suffix\n!endfunction\n!$fn = \"Choice\"\n!$items = %list(\"zero\", \"one\", \"two\", \"three\")\nEMIT(label=kw, to=Carol)\n!foreach $outer in %list(\"A\", \"B\")\n!foreach $inner in %list(\"1\", \"skip\", \"2\")\n!if $inner == \"skip\"\n!continue\n!endif\nAlice -> Bob : $outer$inner\n!if $outer == \"B\" and $inner == \"1\"\n!break\n!endif\n!endfor\n!endfor\nAlice -> Bob : %if(%equals_ignore_case(\"Ada\", \"ada\"), %call_user_func($fn, Ada, ?), \"no\")/%join(%list_slice($items, 1, 2), \"|\")/%min(9, 3, 5)/%max(9, 3, 5)/%abs(-7)/%join(%list_pop($items), \"|\")/%join(%list_shift($items), \"|\")/%contains_ignore_case(\"PlantUML\", \"uml\")\n!if true xor false\nAlice -> Bob : xor-ok\n!endif\n@enduml\n";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", "--", "-"])
        .write_stdin(src)
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
    assert_eq!(
        labels,
        vec![
            "kw",
            "A1",
            "A2",
            "B1",
            "Ada?/one|two/3/9/7/zero|one|two/one|two|three/true",
            "xor-ok",
        ]
    );
}

#[test]
fn preprocessor_malformed_builtin_call_reports_syntax_code() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("@startuml\nA -> B : %strlen(\"unterminated)\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_CALL_SYNTAX"));
}

#[test]
fn preprocessor_include_directives_are_deterministic_for_same_input() {
    // Deterministic-bytes contract for the new include surface: rendering
    // the same source twice yields identical AST bytes.
    for case in [
        "include/valid_include_once.puml",
        "include/valid_include_many.puml",
        "include/valid_includesub.puml",
    ] {
        let first = Command::cargo_bin("puml")
            .expect("binary")
            .args(["--dump", "ast", &fixture(case)])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        let second = Command::cargo_bin("puml")
            .expect("binary")
            .args(["--dump", "ast", &fixture(case)])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        assert_eq!(first, second, "non-deterministic output for {case}");
    }
}

#[test]
fn preprocessor_includeurl_directive_rejects_with_deterministic_code_when_flag_set() {
    Command::cargo_bin("puml")
        .expect("binary")
        .args(["--no-url-includes"])
        .write_stdin("@startuml\n!includeurl https://example.com/lib.puml\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_INCLUDE_URL_DISABLED"));
}

#[test]
fn preprocessor_dynamic_call_error_paths_report_stable_codes() {
    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("@startuml\nA -> B : %call_user_func()\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_DYNAMIC_UNSUPPORTED"));

    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("@startuml\nA -> B : %call_user_func(\"\")\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_DYNAMIC_UNSUPPORTED"));

    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin("@startuml\nA -> B : %call_user_func(\"MissingFn\")\n@enduml\n")
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_CALL_UNKNOWN"));

    Command::cargo_bin("puml")
        .expect("binary")
        .write_stdin(
            "@startuml\n!procedure Emit($from,$to)\n$from -> $to : from-proc\n!endprocedure\nA -> B : %call_user_func(\"Emit\", A, B)\n@enduml\n",
        )
        .assert()
        .code(1)
        .stderr(predicate::str::contains("E_PREPROC_DYNAMIC_UNSUPPORTED"));
}

#[test]
fn preprocessor_builtin_edge_paths_and_foreach_pair_bindings_expand_deterministically() {
    let src = "@startuml\n!$k = \"k\"\n!$pairs = %list(\"left,right\", \"up,down\")\n!foreach $idx,$val in %list(\"red\", \"blue\")\nAlice -> Bob : idx-$idx-$val\n!endfor\n!foreach $a,$b in $pairs\nAlice -> Bob : pair-$a-$b\n!endfor\nAlice -> Bob : %false_then_true($k)-%false_then_true($k)-%true_then_false($k)-%true_then_false($k)\nAlice -> Bob : [%chr(-1)]/%hex2dec(\"zz\")/%ord(\"\")/%dirpath(\"/tmp/demo/file.txt\")/%filename(\"/tmp/demo/file.txt\")/%filenameroot(\"/tmp/demo/file.txt\")\n@enduml\n";
    let out = Command::cargo_bin("puml")
        .expect("binary")
        .args(["--dump", "ast", "--", "-"])
        .write_stdin(src)
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
    assert_eq!(
        labels,
        vec![
            "idx-0-red",
            "idx-1-blue",
            "pair-0-left,right",
            "pair-1-up,down",
            "false-true-true-false",
            "[]/0/0//tmp/demo/file.txt/file",
        ]
    );
}
